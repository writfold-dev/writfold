#!/usr/bin/env python3
"""Build and exercise the pinned Rust Worktrunk boundary with real native tools.

This is component qualification, not owner authorization, recovery or a WF pass.
All repositories, hooks and corrupt objects are fresh disposable evidence fixtures.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import sys
import time

sys.dont_write_bytecode = True
import writfold_evidence as evidence
import writfold_gate as gate
from writfold_preflight import check

WORKTRUNK_SHA256 = '1f3d97a5c263f87345d03a1e439f5d53efa5a468e8bde10e724cbd9e5e79de9b'
WORKTRUNK_BYTES = 29_099_264
CASES = ('normal', 'blob', 'tree', 'commit')
SCOPE = ('Actual static-musl native Worktrunk creation, exact Git closure, isolated '
         'hooks/configuration, preservation and pre-issue corruption refusal only; '
         'not an owner service, interrupted-operation reconciliation or full WF/alpha admission.')


def build(attempt: evidence.Attempt, image: str) -> Path:
    gate.container(attempt, 'build', image, [
        'cargo', 'build', '--release', '--locked', '--offline', '--target', evidence.TARGET,
        '-p', 'writfold-runtime', '--example', 'worktrunk_smoke',
    ])
    gate.container(attempt, 'unit', image, [
        'cp', f'/cache/target/rust-1.98.0/{evidence.TARGET}/release/examples/worktrunk_smoke',
        '/evidence/worktrunk_smoke',
    ])
    headers = gate.container(attempt, 'unit', image, [
        'readelf', '--program-headers', '/evidence/worktrunk_smoke',
    ])
    if 'INTERP' in headers.read_text():
        raise ValueError('native consumer has a dynamic program interpreter')
    return attempt.directory / 'worktrunk_smoke'


def remove_owned_container(attempt: evidence.Attempt, name: str) -> None:
    try:
        attempt.run(['docker', 'rm', '--force', name])
    except ValueError:
        # --rm may already have removed a failed fixture container. Observe
        # absence explicitly; a daemon failure is not proof of released capacity.
        remaining = attempt.run([
            'docker', 'container', 'ls', '--all', '--filter', f'name=^/{name}$',
            '--format', '{{.ID}}',
        ])
        if remaining.read_bytes().strip():
            raise ValueError('owned native fixture container could not be removed')


def exercise(attempt: evidence.Attempt, image: str, inputs: dict, case: str) -> dict:
    directory = attempt.directory / case
    directory.mkdir(mode=0o700)
    for name, source in inputs.items():
        target = directory / name
        shutil.copyfile(source['path'], target)
        target.chmod(0o755)
        if evidence.sha(target) != source['sha256']:
            raise ValueError(f'native input differs from its recorded identity: {name}')
    environment = gate.environment(attempt, image)
    environment['WRITFOLD_ATTEMPT'] = str(directory)
    name = f'writfold-worktrunk-{os.getpid()}-{time.monotonic_ns()}'
    argv = gate.compose() + [
        'run', '--rm', '--no-deps', '-T', '--name', name,
        '--user', f'{os.getuid()}:{os.getgid()}', 'unit', 'python3', '-B',
        '/work/writfold/scripts/writfold_worktrunk_fixture.py',
    ]
    if case != 'normal':
        argv.append(case)
    record = {'case': case, 'status': 'failed'}
    try:
        attempt.run(argv, environment)
        result = evidence.document(directory / 'native-results.json')
        if result.get('status') != 'pass' or result.get('uid') != os.getuid():
            raise ValueError('native fixture did not pass as the operator')
        record['status'] = 'pass'
    except (OSError, ValueError, KeyboardInterrupt) as error:
        record['error'] = str(error)
        # Docker client failure is not container termination or released capacity.
        remove_owned_container(attempt, name)
    finally:
        if (directory / 'native-results.json').is_file():
            record['result'] = evidence.file_record(directory / 'native-results.json')
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--receipt', type=Path, required=True)
    parser.add_argument('--worktrunk', type=Path, required=True,
                        help='retained v0.81.0 Linux-musl wt executable; no automatic download')
    args = parser.parse_args()
    attempt = evidence.Attempt(args.receipt, 900)
    record = {'schema': 'writfold-worktrunk-component/1', 'scope': SCOPE,
              'status': 'failed', 'artifacts': [], 'cases': [], 'skips': [],
              'public_alpha_available': False, 'full_release_gates_passed': []}
    try:
        record['source'] = evidence.snapshot()
        record['preflight'] = check()
        args.command, args.tool_image = 'worktrunk-component', None
        image = gate.admit_image(args, attempt, record)
        worktrunk = args.worktrunk.absolute()
        if worktrunk.is_symlink() or not worktrunk.is_file():
            raise ValueError('Worktrunk input must be a retained regular executable')
        record['worktrunk'] = evidence.file_record(worktrunk)
        if (record['worktrunk']['bytes'] != WORKTRUNK_BYTES or
                record['worktrunk']['sha256'] != WORKTRUNK_SHA256):
            raise ValueError('Worktrunk input differs from the exact admitted development artifact')
        consumer = build(attempt, image)
        record['consumer'] = evidence.file_record(consumer)
        record['fixture'] = evidence.file_record(evidence.ROOT / 'scripts/writfold_worktrunk_fixture.py')
        inputs = {'worktrunk_smoke': record['consumer'], 'wt': record['worktrunk']}
        for case in CASES:
            record['cases'].append(exercise(attempt, image, inputs, case))
        if any(case['status'] != 'pass' for case in record['cases']):
            raise ValueError('native component has failed cases; inspect retained results')
        for source in inputs.values():
            evidence.verify_file(source)
        record['source_after'] = evidence.snapshot()
        if record['source_after'] != record['source']:
            raise ValueError('source changed during native qualification')
        record['status'] = 'pass'
    except (OSError, ValueError, KeyboardInterrupt) as error:
        record['error'] = str(error)
    finally:
        record.update(commands=attempt.commands, logs=attempt.logs)
        evidence.publish(attempt.receipt, record)
    print(json.dumps({'status': record['status'], 'scope': SCOPE,
                      'receipt': str(attempt.receipt), 'error': record.get('error')}))
    return 0 if record['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
