"""Generate/verify public Dolt approval vectors with the admitted offline image.

Only the public synthetic seed 00..1f is used. No native database, operational
credential or writer is opened. Existing differing bytes are never overwritten.
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
INPUTS = ROOT / "crates/writfold-wire/fixtures/signatures"
DEFAULT = ROOT / "crates/writfold-wire/fixtures/dolt-operation"
IMAGE = "writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6"
PLAN_DOMAIN = b"writfold-dolt-plan-v1\0"
APPROVAL_DOMAIN = b"writfold-dolt-operation-v1\0"


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False, allow_nan=False).encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def vectors():
    inputs = {name: (INPUTS / name).read_bytes() for name in
              ("v3-admission.operation.json", "v3-selection.operation.json", "v3-admission.policy.json")}
    operations = {name: json.loads(data) for name, data in inputs.items() if ".operation." in name}
    target = dict(backend="dolt-integration", realm="test-realm", kb="test-kb",
                  database="test_database", epoch="epoch-1", schema_digest="a" * 64,
                  query_digest="b" * 64, storage_profile="dolt-corpus-read-v1", producer="c" * 64)
    pin = {key: value for key, value in target.items() if key != "backend"}
    pin.update(namespace="writfold-dolt-read", schema=1, kind="pin",
               commit="0123456789abcdefghijklmnopqrstuv")
    key = Ed25519PrivateKey.from_private_bytes(bytes(range(32)))
    outputs = {}
    for name, inner_name, base_pin, destination, mapping in (
        ("bootstrap", "v3-admission.operation.json", None, target, None),
        ("selection", "v3-selection.operation.json", pin, target, None),
        ("migration", "v3-selection.operation.json", pin, {**target, "database": "next_database"}, "d" * 64),
    ):
        operation = operations[inner_name]
        base = None if base_pin is None else dict(release=operation["base"], pin=base_pin)
        body = dict(namespace="writfold-dolt-operation", schema=1, operation=operation,
                    base=base, target=destination, mapping=mapping)
        identity = sha(PLAN_DOMAIN + canonical(body))
        body["id"] = identity
        message = APPROVAL_DOMAIN + canonical(body)
        signature = key.sign(message)
        key.public_key().verify(signature, message)
        approval = dict(namespace="writfold-dolt-approval", schema=1,
                        plan=identity, key=operation["plan"]["actor"], signature=signature.hex())
        outputs[name + ".operation.json"] = canonical(body)
        outputs[name + ".approval.json"] = canonical(approval)
        outputs[name + ".message.bin"] = message
    manifest = dict(schema_version=1, purpose="public synthetic codec/crypto vectors, not writer qualification",
                    required_tool_image=IMAGE, python_version=platform.python_version(),
                    cryptography_version=cryptography.__version__, public_test_seed_hex=bytes(range(32)).hex(),
                    generator_sha256=sha(Path(__file__).read_bytes()),
                    input_sha256={name: sha(data) for name, data in sorted(inputs.items())},
                    outputs={name: sha(data) for name, data in sorted(outputs.items())},
                    limits="Synthetic realm/database/schema/tool identities; no live policy, release, retained closure or native write admission.")
    return outputs, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    outputs, manifest = vectors()
    payloads = {**outputs, "manifest.json": canonical(manifest)}
    if args.verify:
        present = {path.name for path in args.output.iterdir() if path.is_file()}
        if present != set(payloads):
            raise ValueError("Dolt fixture inventory differs")
        for name, expected in payloads.items():
            if (args.output / name).is_symlink() or (args.output / name).read_bytes() != expected:
                raise ValueError("Dolt fixture/provenance drift: " + name)
    else:
        args.output.mkdir(parents=True, exist_ok=True)
        for name, data in payloads.items():
            path = args.output / name
            if path.exists() or path.is_symlink():
                if path.is_symlink() or path.read_bytes() != data:
                    raise ValueError("Existing fixture differs; select a new output: " + name)
            else:
                with path.open("xb") as output:
                    output.write(data)
    print(json.dumps(dict(result="verified" if args.verify else "generated", vectors=3,
                          payloads=len(outputs), manifest_sha256=sha(payloads["manifest.json"]),
                          scope="Independent canonical bytes and Ed25519 equations only")))


if __name__ == "__main__":
    main()
