"""Generate/verify public storage approval vectors, never product qualification.

Run with the admitted offline Compose image. The only signing seed is the public
bytes 00..1f; no operational credential or backend is opened. Existing differing
vectors/manifests refuse instead of being overwritten.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import cryptography
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ROOT / 'crates/writfold-wire/fixtures/signatures'
DEFAULT = ROOT / 'crates/writfold-wire/fixtures/storage-operation'
IMAGE = 'writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6'
PLAN_DOMAIN = b'writfold-storage-plan-v1\0'
APPROVAL_DOMAIN = b'writfold-storage-operation-v1\0'


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'),
                      ensure_ascii=False, allow_nan=False).encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def vectors():
    inputs = {name: (INPUTS / name).read_bytes() for name in
              ('v3-admission.operation.json', 'v3-selection.operation.json', 'v3-admission.policy.json')}
    operations = {name: json.loads(data) for name, data in inputs.items() if '.operation.' in name}
    policy = json.loads(inputs['v3-admission.policy.json'])
    target = {'backend': 'omnigraph', 'kb': policy['kb_id'], 'graph': 'Graph-01',
              'epoch': 'epoch-1', 'schema_digest': 'a' * 64, 'query_digest': 'b' * 64,
              'storage_profile': 'reader-1', 'native_format': 1, 'producer_generation': 'c' * 64}
    native_pin = {'namespace': 'writfold-storage-pin', 'schema': 1, **target,
                  'commit': '01ARZ3NDEKTSV4RRFFQ69G5FAV'}
    legacy_pin = {'namespace': 'writfold-storage-pin', 'schema': 1, 'backend': 'legacy-dolt',
                  'kb': policy['kb_id'], 'dolt_commit': '0123456789abcdefghijklmnopqrstuv'}
    key = Ed25519PrivateKey.from_private_bytes(bytes(range(32)))
    outputs = {}
    for name, inner_name, pin, mapping in (
        ('bootstrap', 'v3-admission.operation.json', None, None),
        ('selection', 'v3-selection.operation.json', native_pin, None),
        ('legacy-selection', 'v3-selection.operation.json', legacy_pin, 'd' * 64),
    ):
        operation = operations[inner_name]
        base = None if pin is None else {'release': operation['base'], 'pin': pin}
        body = {'namespace': 'writfold-storage-operation', 'schema': 1,
                'operation': operation, 'base': base, 'target': target, 'mapping': mapping}
        identity = sha(PLAN_DOMAIN + canonical(body))
        body['id'] = identity
        message = APPROVAL_DOMAIN + canonical(body)
        signature = key.sign(message)
        key.public_key().verify(signature, message)
        approval = {'namespace': 'writfold-storage-approval', 'schema': 1,
                    'plan': identity, 'key': operation['plan']['actor'], 'signature': signature.hex()}
        outputs[name + '.operation.json'] = canonical(body)
        outputs[name + '.approval.json'] = canonical(approval)
        outputs[name + '.message.bin'] = message
    manifest = {'schema_version': 1, 'purpose': 'public synthetic codec/crypto vectors, not qualification',
                'required_tool_image': IMAGE, 'python_version': platform.python_version(),
                'cryptography_version': cryptography.__version__,
                'public_test_seed_hex': bytes(range(32)).hex(),
                'generator_sha256': sha(Path(__file__).read_bytes()),
                'input_sha256': {name: sha(data) for name, data in sorted(inputs.items())},
                'outputs': {name: sha(data) for name, data in sorted(outputs.items())},
                'limits': 'Native format 1 and graph/profile identities are synthetic, not an admitted engine profile.'}
    return outputs, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=DEFAULT)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    outputs, manifest = vectors()
    payloads = {**outputs, 'manifest.json': canonical(manifest)}
    if args.verify:
        present = {path.name for path in args.output.iterdir() if path.is_file()}
        if present != set(payloads):
            raise ValueError('Storage fixture inventory differs')
        for name, expected in payloads.items():
            if (args.output / name).is_symlink() or (args.output / name).read_bytes() != expected:
                raise ValueError('Storage fixture/provenance drift: ' + name)
    else:
        args.output.mkdir(parents=True, exist_ok=True)
        for name, data in payloads.items():
            path = args.output / name
            if path.exists() or path.is_symlink():
                if path.is_symlink() or path.read_bytes() != data:
                    raise ValueError('Existing fixture differs; select a new output: ' + name)
            else:
                with path.open('xb') as output:
                    output.write(data)
    print(json.dumps({'result': 'verified' if args.verify else 'generated',
                      'vectors': 3, 'payloads': len(outputs), 'manifest_sha256': sha(payloads['manifest.json']),
                      'scope': 'Independent canonical bytes and Ed25519 fixture equations only'}))


if __name__ == '__main__':
    main()
