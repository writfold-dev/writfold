"""Run the actual static-musl Dolt approval consumer against public vectors.

Use --receipt ABS_NEW_EXTERNAL. This is a supplied-input codec/crypto component,
not native publication, live policy admission, DINT02 completion or a WF gate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import sys
import time

sys.dont_write_bytecode = True
import writfold_evidence as evidence
import writfold_gate as gate
from writfold_stdio_process import Pipes

ROOT = evidence.ROOT
VECTORS = ROOT / "crates/writfold-wire/fixtures/dolt-operation"
SIGNATURES = ROOT / "crates/writfold-wire/fixtures/signatures"
COPY_CONSUMER = r'''
import json, pathlib, shutil, sys
records = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
if not any(r.get('reason') == 'build-finished' and r.get('success') is True for r in records):
    raise ValueError('No successful completed build')
artifacts = [r for r in records if r.get('reason') == 'compiler-artifact'
             and r.get('target', {}).get('name') == 'dolt_approval'
             and r['target'].get('kind') == ['example'] and r.get('executable')]
if len(artifacts) != 1:
    raise ValueError('Missing or ambiguous consumer artifact')
artifact = artifacts[0]
if artifact['manifest_path'] != '/work/writfold/crates/writfold-wire/Cargo.toml':
    raise ValueError('Wrong consumer crate')
if artifact['target']['src_path'] != '/work/writfold/crates/writfold-wire/examples/dolt_approval.rs':
    raise ValueError('Wrong consumer source')
source = pathlib.Path(artifact['executable'])
if not source.resolve().is_relative_to('/cache/target/rust-1.98.0/x86_64-unknown-linux-musl/release/examples'):
    raise ValueError('Wrong target artifact')
target = pathlib.Path('/evidence/dolt_approval')
with source.open('rb') as original, target.open('xb') as retained:
    shutil.copyfileobj(original, retained)
target.chmod(0o555)
print(json.dumps(artifact))
'''


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()


def reidentify(value):
    value.pop("id", None)
    value["id"] = hashlib.sha256(b"writfold-dolt-plan-v1\0" + canonical(value)).hexdigest()
    return value


class Campaign:
    def __init__(self, work, binary, report):
        self.work, self.binary, self.report = work, binary, report
        self.pipes = Pipes(work)
        report["native_commands"] = self.pipes.records
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(work), "LANG": "C.UTF-8"}
        self.policy = SIGNATURES / "v3-admission.policy.json"
        self.pem = SIGNATURES / "PUBLIC-TEST-ONLY.pem"
        self.toolchain = evidence.document(VECTORS / "bootstrap.operation.json")["operation"]["toolchain"]

    def run(self, name, mode, operation, approval=None, realm="test-realm", toolchain=None, refuse=False):
        check = dict(name=name, status="fail", expected_refusal=refuse)
        self.report["cases"].append(check)
        if mode == "--sign":
            args = [str(operation), str(self.policy), str(self.pem)]
        else:
            args = [str(operation), str(approval), str(self.policy)]
        out, err, code = self.pipes.run(name, [str(self.binary), mode, *args,
            toolchain or self.toolchain, realm], [("eof", None)], cwd=self.work, env=self.env)
        require(code == (1 if refuse else 0), f"Unexpected {name} exit: {code}")
        if refuse:
            require(not out and json.loads(err)["status"] == "refused", f"Unattributed refusal: {name}")
        elif mode == "--sign":
            require(not err and out == approval.read_bytes(), f"Independent signature differs: {name}")
        else:
            value = json.loads(out)
            require(not err and value["status"] == "supplied-inputs-verified" and
                    value["plan"] == evidence.document(operation)["id"] and
                    value["realm"] == realm and value["kb"] == "test-kb" and
                    value["write_authority"] is False, f"Incorrect supplied-input result: {name}")
        check["status"] = "pass"

    def store(self, name, value):
        path = self.work / name
        with path.open("xb") as output:
            output.write(canonical(value))
        return path

    def execute(self):
        for name in ("bootstrap", "selection", "migration"):
            operation = VECTORS / (name + ".operation.json")
            approval = VECTORS / (name + ".approval.json")
            self.run(name + "-sign", "--sign", operation, approval)
            self.run(name + "-verify", "--verify", operation, approval)
        operation, approval = VECTORS / "selection.operation.json", VECTORS / "selection.approval.json"
        self.run("wrong-request-realm", "--verify", operation, approval, realm="other-realm", refuse=True)
        self.run("wrong-toolchain", "--verify", operation, approval, toolchain="f" * 64, refuse=True)
        old = ROOT / "crates/writfold-wire/fixtures/storage-operation/selection.approval.json"
        self.run("omnigraph-approval-namespace", "--verify", operation, old, refuse=True)
        transplanted = evidence.document(approval)
        transplanted["signature"] = evidence.document(old)["signature"]
        self.run("omnigraph-signature-transplant", "--verify", operation,
                 self.store("transplanted-approval.json", transplanted), refuse=True)
        for name, part, field, value in (
            ("retained-commit-replay", "pin", "commit", "u" * 32),
            ("producer-replay", "target", "producer", "e" * 64),
            ("cross-realm-base", "pin", "realm", "other-realm"),
            ("missing-migration-mapping", "target", "database", "next_database"),
        ):
            changed = evidence.document(operation)
            destination = changed["base"]["pin"] if part == "pin" else changed["target"]
            destination[field] = value
            changed = reidentify(changed)
            replay = evidence.document(approval)
            replay["plan"] = changed["id"]
            self.run(name, "--verify", self.store(name + ".operation.json", changed),
                     self.store(name + ".approval.json", replay), refuse=True)


def interrupted(signum, frame):
    raise KeyboardInterrupt(f"signal {signum}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    require(args.receipt.is_absolute(), "Select an absolute new external receipt")
    attempt = evidence.Attempt(args.receipt, 2700)
    report = dict(schema_version=1, status="fail", scope=__doc__.strip(), started_ns=time.time_ns(),
                  commands=attempt.commands, artifacts=[], cases=[], errors=[], release_gates_passed=[],
                  native_publication="not_run", full_dint02="not_run", omnigraph_wf24="blocked_unchanged")
    try:
        report["source_before"] = evidence.snapshot()
        report["preflight"] = evidence.document(attempt.run([sys.executable, "scripts/writfold_preflight.py"]))
        image = gate.admit_image(argparse.Namespace(tool_image=None, command="native-smoke"), attempt, report)
        vectors = gate.container(attempt, "unit", image, ["python3", "scripts/writfold_dolt_fixtures.py", "--verify"])
        report["independent_vectors"] = evidence.document(vectors)
        build = gate.container(attempt, "unit", image, ["cargo", "build", "--offline", "--locked", "--release",
            "--target", evidence.TARGET, "-p", "writfold-wire", "--example", "dolt_approval", "--message-format=json"])
        exported = gate.container(attempt, "unit", image, ["python3", "-c", COPY_CONSUMER, "/evidence/" + build.name])
        report["compiler_artifact"] = evidence.document(exported)
        elf = gate.container(attempt, "unit", image, ["readelf", "-h", "-l", "-d", "/evidence/dolt_approval"])
        text = elf.read_text()
        require("ELF64" in text and "Advanced Micro Devices X86-64" in text and
                "INTERP" not in text and "(NEEDED)" not in text, "Consumer is not static x86_64 ELF")
        binary = attempt.directory / "dolt_approval"
        report["binary"] = evidence.file_record(binary)
        report["static_elf"] = True
        work = attempt.directory / "native"
        work.mkdir(mode=0o700)
        campaign = Campaign(work, binary, report)
        campaign.execute()
        require(evidence.file_record(binary) == report["binary"], "Consumer changed during execution")
    except (Exception, KeyboardInterrupt) as exc:
        report["errors"].append(f"{type(exc).__name__}: {exc}")
    finally:
        try:
            report["source_after"] = evidence.snapshot()
            require(report.get("source_before") == report["source_after"], "Source changed during campaign")
            report["retained_outputs"] = [evidence.file_record(p) for p in sorted(attempt.directory.rglob("*"))
                                          if p.is_file() and not p.is_symlink()]
        except (Exception, KeyboardInterrupt) as exc:
            report["errors"].append(f"{type(exc).__name__}: {exc}")
        report["logs"] = attempt.logs
        report["ended_ns"] = time.time_ns()
        if not report["errors"] and len(report["cases"]) == 14 and all(c["status"] == "pass" for c in report["cases"]):
            report["status"] = "pass"
        evidence.publish(attempt.receipt, report)
    print(json.dumps(dict(status=report["status"], cases=len(report["cases"]), receipt=str(attempt.receipt))))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    raise SystemExit(main())
