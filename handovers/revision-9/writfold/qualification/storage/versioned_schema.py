#!/usr/bin/env python3
"""Small native V1/V2 read and historical-branch boundary probe; not WF24."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--omnigraph', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    exe, out = args.omnigraph.resolve(), args.out.resolve()
    fixture = Path(__file__).resolve().parent
    out.mkdir(parents=True, exist_ok=False)
    graph = str(out/'graph.omni')
    env = {**os.environ, 'RUST_BACKTRACE': '0', 'RAYON_NUM_THREADS': '2',
           'TOKIO_WORKER_THREADS': '2'}
    commands, assertions = [], []

    def check(name, result):
        assertions.append({'name': name, 'pass': bool(result)})
        (out/'assertions.json').write_text(json.dumps(assertions, indent=2)+'\n')
        assert result, name

    def run(name, argv, expected=0):
        proc = subprocess.run([str(exe), *argv], capture_output=True, env=env, timeout=60)
        for suffix, data in [('stdout', proc.stdout), ('stderr', proc.stderr)]:
            (out/f'{name}.{suffix}').write_bytes(data)
        commands.append({'name': name, 'argv': [str(exe), *argv],
                         'expected_exit': expected, 'actual_exit': proc.returncode})
        (out/'commands.json').write_text(json.dumps(commands, indent=2)+'\n')
        assert proc.returncode == expected, f'{name}: see retained logs'
        return json.loads(proc.stdout) if '--json' in argv else proc.stdout.decode()

    check('Exact tested CLI bytes', sha(exe) ==
          '978e200a412e7b0db46657417d348569f97529c18b47e64687132e565d19dc64')
    version = run('version', ['version', '--json'])
    check('Tested version', version['version'] == '0.14.0')
    for name in ['schema.pg', 'queries.gq', 'seed.jsonl']:
        (out/name).write_text((fixture/name).read_text().replace('Service', 'ServiceV1'))
    (out/'schema-next.pg').write_text((out/'schema.pg').read_text() +
        '\nnode ServiceV2 {\n slug: String @key\n name: String\n description: String?\n}\n')
    run('init', ['init', '--schema', str(out/'schema.pg'), graph])
    seed = run('load', ['load', '--data', str(out/'seed.jsonl'), '--mode', 'overwrite',
                       '--target', graph, '--json'])
    old = seed['commit']['graph_commit_id']
    query = ['query', 'whole', '--query', str(out/'queries.gq'), '--snapshot', old,
             '--target', graph, '--json']
    before = run('old-before', query)
    check('Old snapshot contains four fixture nodes', before['row_count'] == 4)
    run('schema-apply', ['schema', 'apply', '--schema', str(out/'schema-next.pg'),
                         '--target', graph, '--json'])
    after = run('old-after', query)
    check('Old whole-node rows unchanged', before['rows'] == after['rows'])
    check('Returned commit remains the old pin', after['graph_commit_id'] == old)
    branch = run('old-branch', ['branch', 'create', 'old-release', '--from-commit', old,
                              '--target', graph, '--json'], expected=1)
    check('Cross-schema branching refuses without claiming success',
          branch['code'] == 'branch_start_refusal' and
          branch['branch_start_refusal']['reason'] == 'schema_changed' and
          branch['disposition']['effects'] == 'none')
    receipt = {'scope': 'Native local CLI versioned-type smoke, not full epochs/WSL/server/crash/restore or WF24',
               'date': datetime.now(timezone.utc).date().isoformat(), 'host': platform.platform(), 'machine': platform.machine(),
               'omni_sha256': sha(exe), 'old_commit': old, 'assertions': assertions,
               'thread_limits': {k: env[k] for k in ['RAYON_NUM_THREADS', 'TOKIO_WORKER_THREADS']},
               'inputs': {p.name: sha(p) for p in [Path(__file__), fixture/'schema.pg',
                           fixture/'queries.gq', fixture/'seed.jsonl']},
               'outputs': {p.name: sha(p) for p in out.iterdir() if p.is_file()}}
    (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(f'PASS: {len(assertions)} focused assertions / {len(commands)} commands. Product gates remain not_run.')


if __name__ == '__main__':
    main()
