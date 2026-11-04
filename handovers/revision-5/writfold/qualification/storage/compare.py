#!/usr/bin/env python3
"""Bounded local storage smoke comparison. Not a persistence qualification gate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--omnigraph', type=Path, required=True)
    parser.add_argument('--dolt', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    omni, dolt = args.omnigraph.resolve(), args.dolt.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)  # Never overwrite a prior attempt.
    fixture = Path(__file__).resolve().parent
    graph = out / 'graph.omni'
    db = out / 'dolt-db'
    db.mkdir()
    commands, checks = [], []
    env = {**os.environ, 'RUST_BACKTRACE': '0', 'RAYON_NUM_THREADS': '2',
           'TOKIO_WORKER_THREADS': '2'}

    def run(name, binary, argv, cwd=None, allowed=(0,)):
        start = time.time()
        result = subprocess.run([str(binary), *argv], cwd=cwd, env=env,
                                capture_output=True, timeout=60)
        for ext, data in [('stdout', result.stdout), ('stderr', result.stderr)]:
            (out / f'{name}.{ext}').write_bytes(data)
        commands.append({'name': name, 'argv': [str(binary), *argv],
                         'cwd': str(cwd or Path.cwd()), 'exit': result.returncode,
                         'elapsed_seconds': time.time() - start})
        (out / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n')
        if result.returncode not in allowed:
            raise RuntimeError(f'{name}: exit {result.returncode}; inspect logs')
        return result.stdout.decode()

    def check(name, condition):
        checks.append({'name': name, 'pass': bool(condition)})
        (out / 'assertions.json').write_text(json.dumps(checks, indent=2) + '\n')
        if not condition:
            raise AssertionError(name)

    def oq(name, query, snapshot=None):
        argv = ['query', query, '--query', str(fixture / 'queries.gq'),
                '--target', str(graph), '--json']
        if query == 'two':
            argv += ['--params', '{"key":"a"}']
        if snapshot:
            argv += ['--snapshot', snapshot]
        return json.loads(run(name, omni, argv, allowed=(0, 1)))

    def sql(name, query):
        text = run(name, dolt, ['sql', '-r', 'json', '-q', query], db)
        return json.loads(text) if text.strip() else None

    version = json.loads(run('omni-version', omni, ['version', '--json']))
    check('Omnigraph version pin', version['version'] == '0.14.0')
    check('Omnigraph executable digest', digest(omni) ==
          '978e200a412e7b0db46657417d348569f97529c18b47e64687132e565d19dc64')
    dv = run('dolt-version', dolt, ['version'])
    check('Dolt version pin', 'dolt version 2.6.1' in dv)
    run('omni-init', omni, ['init', '--schema', str(fixture / 'schema.pg'), str(graph)])
    seed = json.loads(run('omni-load', omni, ['load', '--data', str(fixture / 'seed.jsonl'),
                      '--mode', 'overwrite', '--target', str(graph), '--json']))
    old = seed['commit']['graph_commit_id']
    check('Omnigraph fixture cardinality', seed['total_entities'] == 9)
    check('GQ shortest two-hop endpoints', oq('omni-two', 'two')['rows'] == [{'t.slug': 'd'}])
    before = oq('omni-old-whole-before', 'whole', old)
    check('Omnigraph pinned whole-node before evolution', before['row_count'] == 4)
    for op in ['plan', 'apply']:
        run('omni-schema-' + op, omni, ['schema', op, '--schema', str(fixture / 'schema-next.pg'),
                                     '--target', str(graph), '--json'])
    after = oq('omni-old-whole-after', 'whole', old)
    check('Omnigraph historical whole-node refusal after addition',
          after.get('code') == 'bad_request' and 'description' in after.get('error', '')
          and 'does not exist at this snapshot' in after.get('error', ''))
    explicit = oq('omni-old-explicit-after', 'all_nodes', old)
    check('Omnigraph historical explicit-field read survives',
          explicit['rows'] == [{'s.slug': x, 's.name': x.upper()} for x in 'abcd'])
    check('Omnigraph current whole-node read survives', oq('omni-new-whole', 'whole')['row_count'] == 4)
    run('dolt-init', dolt, ['init', '--name', 'Writfold Probe', '--email', 'probe@example.invalid'], db)
    run('dolt-seed', dolt, ['sql', '-q', (fixture / 'seed.sql').read_text()], db)
    old_dolt = sql('dolt-pin', "SELECT DOLT_HASHOF('HEAD') AS hash;")['rows'][0]['hash']
    before_dolt = sql('dolt-old-before', f"SELECT * FROM service AS OF '{old_dolt}' ORDER BY slug;")
    got = sql('dolt-traverse', (fixture / 'traversal.sql').read_text())
    check('SQL shortest two-hop endpoints', got['rows'] == [{'slug': 'd'}])
    sql('dolt-alter', 'ALTER TABLE service ADD COLUMN description TEXT NULL;')
    got = sql('dolt-old-after', f"SELECT * FROM service AS OF '{old_dolt}' ORDER BY slug;")
    check('Dolt historical wildcard unchanged', got == before_dolt)
    ddl = sql('dolt-old-schema', f"SHOW CREATE TABLE service AS OF '{old_dolt}';")
    check('Dolt historical schema excludes later property', 'description' not in ddl['rows'][0]['Create Table'])
    record = {'scope': 'Local CLI smoke; not WSL/server/crash/performance/production qualification',
              'host': platform.platform(), 'machine': platform.machine(),
              'python': platform.python_version(), 'thread_limits': {k: env[k] for k in
              ['RAYON_NUM_THREADS', 'TOKIO_WORKER_THREADS']},
              'omni_sha256': digest(omni), 'dolt_sha256': digest(dolt),
              'omni_commit': old, 'dolt_commit': old_dolt, 'assertions': checks,
              'inputs': {p.name: digest(p) for p in fixture.iterdir() if p.is_file()},
              'logs': {p.name: digest(p) for p in out.iterdir() if p.is_file()}}
    (out / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f'PASS: {len(checks)} focused assertions; {len(commands)} commands. See {out}/receipt.json')


if __name__ == '__main__':
    main()
