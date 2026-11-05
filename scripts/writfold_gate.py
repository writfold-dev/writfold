"""Explicit local commands; a scoped success is never a complete mandatory gate."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
import time

import writfold_evidence as evidence

ROOT = evidence.ROOT
GATES = {"baseline": "WF00", "bootstrap": "WF01", "build": "WF01", "unit": "WF02",
         "quality": "WF02", "lean": "WF04", "verus": "WF05", "model": "WF06",
         "tlaps": "WF07", "conformance": "WF04", "sources": "WF08", "omnigraph": "WF09",
         "reference": "WF09", "native-smoke": "WF09", "publication": "WF11",
         "storage-foundation": "WF24", "storage-profile": "WF03", "legacy-import": "WF14",
         "views": "WF22", "obsidian": "WF23",
         "workflows": "WF12", "transport": "WF13", "migration": "WF14",
         "evaluate": "WF15", "runtime": "WF16", "package": "WF16", "site": "WF17",
         "scribes": "WF18", "portfolio": "WF19", "design": "WF20", "companion": "WF21",
         "engineering-ledger": "WF25", "skills": "WF26", "effects": "WF27",
         "cross-repo": "WF28", "learning": "WF29", "engineering-cuts": "WF30",
         "engineering-calculus": "WF31", "engineering-lean": "WF31",
         "citation-lean": "WF08", "citation-verus": "WF08",
         "release-required": "release-required"}
SERVICES = {"unit": ("unit", "Cargo.lock"), "build": ("build", "Cargo.lock"),
            "quality": ("quality", "scripts/writfold_quality.py"),
            "verus": ("verus", "scripts/writfold_verus.py"),
            "engineering-lean": ("engineering-lean", "scripts/writfold_engineering_lean.py"),
            "citation-lean": ("citation-lean", "scripts/writfold_citation_lean.py"),
            "citation-verus": ("citation-verus", "scripts/writfold_verus.py"),
            "conformance": ("conformance", "scripts/writfold_conformance.py"),
            "reference": ("reference", "scripts/dolt_smoke.py"),
            "native-smoke": ("reference", "scripts/dolt_smoke.py")}

EXPORT_BUILD = """import json, pathlib, shutil, sys
base = pathlib.Path('/cache/target/rust-1.98.0')
destination = pathlib.Path('/evidence/build')
count = 0
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    record = json.loads(line)
    if record.get('reason') != 'compiler-artifact':
        continue
    for name in record['filenames']:
        source = pathlib.Path(name)
        relative = source.resolve().relative_to(base)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            with source.open('rb') as original, target.open('xb') as retained:
                shutil.copyfileobj(original, retained)
            count += 1
if not count:
    raise ValueError('No retained compiler artifacts')
print(json.dumps({'retained_compiler_artifacts': count}))
"""


def compose() -> list[str]:
    return ["docker", "compose", "--env-file", "/dev/null", "--project-directory", str(ROOT),
            "-f", str(ROOT / "compose.yaml")]


def environment(attempt: evidence.Attempt, image: str) -> dict:
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("COMPOSE_", "WRITFOLD_BUILD_"))}
    allocation = evidence.document(ROOT / "qualification/resources.json")["allocation"]
    env.update(WRITFOLD_TOOL_IMAGE=image, WRITFOLD_ATTEMPT=str(attempt.directory),
               WRITFOLD_BUILD_CPUS=str(allocation["cpus"]),
               WRITFOLD_BUILD_MEMORY=str(allocation["memory_bytes"]),
               WRITFOLD_BUILD_JOBS=str(allocation["cargo_jobs"]), PYTHONDONTWRITEBYTECODE="1")
    return env


def container(attempt: evidence.Attempt, service: str, image: str,
              command: list[str] | None = None) -> Path:
    name = f"writfold-gate-{os.getpid()}-{time.monotonic_ns()}"
    argv = compose() + ["run", "--rm", "--no-deps", "-T", "--name", name, service] + (command or [])
    try:
        return attempt.run(argv, environment(attempt, image))
    except (OSError, ValueError, KeyboardInterrupt):
        # A killed Docker client does not reap its container. Retain capacity until removal.
        attempt.run(["docker", "rm", "--force", name])
        raise


def admit_image(args, attempt: evidence.Attempt, receipt: dict) -> str:
    lock_path = ROOT / "toolchains.lock.json"
    lock = None
    if args.tool_image:
        if args.command not in ("bootstrap", "reference", "native-smoke"):
            raise ValueError("--tool-image is only for bootstrap observation/reference native smoke")
        image = args.tool_image
        receipt["tool_admission"] = "explicit-unadmitted"
    else:
        if not lock_path.is_file():
            raise ValueError("Missing toolchains.lock.json; no mutable bootstrap tag fallback")
        lock = evidence.document(lock_path)
        if lock.get("schema_version") != 1 or lock.get("admitted") is not True:
            raise ValueError("Tool lock is not admitted")
        image = lock["image"]["reference"]
    if not re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", image):
        raise ValueError("Tool image must be name@sha256:<64 lowercase hex>")
    receipt["tool_image"] = image
    inspected = evidence.document(attempt.run(["docker", "image", "inspect", image]))
    if len(inspected) != 1 or image not in inspected[0].get("RepoDigests", []):
        raise ValueError("Local image digest does not match selected immutable reference")
    if lock is not None:
        if inspected[0]["Id"] != lock["image"]["id"]:
            raise ValueError("Local image ID differs from admitted tool lock")
        observation = lock["observation"]
        observed_path = Path(observation["path"])
        if not observed_path.is_absolute():
            observed_path = evidence.source_path(observation["path"])
        if evidence.sha(observed_path) != observation["sha256"]:
            raise ValueError("Retained tool observation digest mismatch")
    observed = container(attempt, "reference", image, ["python3", "scripts/writfold_tools.py"])
    actual = evidence.document(observed)
    if not actual.get("tools") or actual.get("target") != evidence.TARGET:
        raise ValueError("Invalid tool observation")
    if lock is not None:
        if actual != evidence.document(observed_path):
            raise ValueError("Fresh executable/tool observation differs from admitted inventory")
        receipt["tool_admission"] = "admitted"
    receipt["artifacts"].append(evidence.file_record(observed))
    return image


def playbook(args, attempt: evidence.Attempt, receipt: dict) -> None:
    path = (args.playbook or Path.home() / ".local/share/writfold/references/playbook-e7d991e").resolve()
    pin = attempt.run(["git", "--no-replace-objects", "-C", str(path), "rev-parse", "HEAD"])
    if pin.read_text().strip() != evidence.PLAYBOOK:
        raise ValueError("Playbook does not resolve to the adopted pin")
    dirty = attempt.run(["git", "--no-replace-objects", "-C", str(path), "status", "--porcelain=v1", "--untracked-files=all"])
    if dirty.read_bytes():
        raise ValueError("Adopted playbook checkout is dirty")
    receipt.update(playbook_commit=evidence.PLAYBOOK, playbook_path=str(path))


def baseline(args, attempt: evidence.Attempt, receipt: dict) -> list[dict]:
    report = attempt.directory / "baseline.json"
    argv = [sys.executable, "scripts/writfold_baseline.py", "--source", str(ROOT),
            "--output", str(report), "--playbook", receipt["playbook_path"]]
    try:
        attempt.run(argv)
    finally:
        if report.is_file():
            receipt["artifacts"].append(evidence.file_record(report))
    checks = evidence.document(report)["checks"] + evidence.frozen_inventories()
    frozen = attempt.directory / "frozen-inventories.json"
    evidence.publish(frozen, {"checks": checks})
    receipt["artifacts"].append(evidence.file_record(frozen))
    return checks


def service_checks(args, attempt: evidence.Attempt, receipt: dict, image: str) -> list[dict]:
    service, backing = SERVICES[args.command]
    if not (ROOT / backing).is_file():
        raise ValueError(f"Backing implementation unavailable: {backing}")
    command = None
    if args.command == "build":
        command = ["cargo", "build", "--workspace", "--release", "--target", evidence.TARGET,
                   "--locked", "--offline", "--message-format=json"]
    log = container(attempt, service, image, command)
    if args.command == "unit":
        stderr = Path(attempt.logs[-1]["path"]).read_text(errors="replace")
        text = log.read_text(errors="replace") + stderr
        rows = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
        if not rows or sum(int(row[0]) for row in rows) == 0:
            raise ValueError("No executed Rust test counts found")
        counts = [sum(int(row[index]) for row in rows) for index in range(3)]
        receipt["skips"].extend([f"{counts[2]} ignored Rust tests"] if counts[2] else [])
        return [{"name": f"rust-test-{index}", "required": True, "status": state}
                for count, state in zip(counts, ("pass", "fail", "skip")) for index in range(count)]
    if args.command in ("reference", "native-smoke"):
        if "DOLT_INTEGRATION_OK:" not in log.read_text(errors="replace"):
            raise ValueError("Native smoke success marker absent")
        return [{"name": "native-reference-smoke-command", "required": True, "status": "pass"}]
    if args.command == "build":
        container(attempt, "dev", image, ["python3", "-c", EXPORT_BUILD, "/evidence/" + log.name])
        return [{"name": "offline-musl-build-command", "required": True, "status": "pass"}]
    report = attempt.directory / "obligations.json"
    result = evidence.document(report)
    receipt["artifacts"].append(evidence.file_record(report))
    receipt["skips"].extend(result["skips"])
    return result["checks"]


def execute(args, attempt: evidence.Attempt, receipt: dict) -> list[dict]:
    playbook(args, attempt, receipt)
    if args.command == "baseline":
        return baseline(args, attempt, receipt)
    if args.command not in SERVICES and args.command not in ("bootstrap", "release-required"):
        raise ValueError(f"Gate producer is not implemented: {args.command}")
    attempt.run([sys.executable, "scripts/writfold_preflight.py"])
    if args.command == "bootstrap" and not args.tool_image:
        # Build is local; execution still requires the exact subsequently verified lock digest.
        attempt.run(compose() + ["--profile", "bootstrap", "build", "tools"],
                    environment(attempt, "unselected-do-not-execute"))
    image = admit_image(args, attempt, receipt)
    if args.command == "bootstrap":
        receipt["scope"] = "tool-observation-only"
        return [{"name": "tool-image-and-executables-observed", "required": True, "status": "pass"}]
    if args.command == "release-required":
        report = evidence.release_required(receipt, receipt["candidate_artifacts"])
        output = attempt.directory / "release-required.json"
        evidence.publish(output, report)
        receipt["artifacts"].append(evidence.file_record(output))
        return report["checks"]
    return service_checks(args, attempt, receipt, image)


def initial_receipt(args, attempt: evidence.Attempt) -> dict:
    return {"schema_version": 2, "gate": GATES[args.command], "status": "blocked",
            # Historical Dolt smoke is not the current Omnigraph WF09 contract.
            "scope": "release-aggregation" if args.command == "release-required" else f"{args.command}-only",
            "source": None, "source_after_digest": None, "command": [sys.executable, *sys.argv],
            "commands": attempt.commands, "toolchain_digest": None, "cargo_lock_digest": None,
            "tool_image": None, "tool_admission": "not-admitted", "fixture_digest": None,
            "gate_definition_digest": None, "target": evidence.TARGET, "features": ["default"],
            "playbook_commit": None, "playbook_path": None, "start_ns": time.time_ns(),
            "end_ns": None, "duration_ns": None, "clock": "time.time_ns / time.monotonic_ns",
            "exit_code": None, "skips": [], "obligations": {"required": 0, "passed": 0, "failed": 0, "skipped": 0},
            "artifacts": [], "candidate_artifacts": [], "logs": attempt.logs,
            "evidence_mode": "executed", "error": None}


def qualify(args, attempt: evidence.Attempt) -> int:
    receipt = initial_receipt(args, attempt)
    if args.command in ("reference", "native-smoke"):
        receipt["scope"] = "historical-dolt-reference-native-smoke-only"
    started = time.monotonic_ns()
    try:
        receipt["source"] = evidence.snapshot()
        receipt["fixture_digest"] = evidence.fixtures(receipt["source"])
        receipt["gate_definition_digest"] = evidence.definition()
        for key, name in (("cargo_lock_digest", "Cargo.lock"), ("toolchain_digest", "toolchains.lock.json")):
            if (ROOT / name).is_file():
                receipt[key] = evidence.sha(ROOT / name)
        for path in args.artifact:
            record = evidence.file_record(path)
            evidence.verify_file(record)
            receipt["candidate_artifacts"].append(record)
        checks = execute(args, attempt, receipt)
        if not checks or any(check.get("required") is not True or check.get("status") not in ("pass", "fail", "skip") for check in checks):
            raise ValueError("Missing/malformed required obligation report")
        counts = receipt["obligations"]
        counts.update(required=len(checks), passed=sum(c["status"] == "pass" for c in checks),
                      failed=sum(c["status"] == "fail" for c in checks), skipped=sum(c["status"] == "skip" for c in checks))
        if counts["failed"] or counts["skipped"] or receipt["skips"]:
            raise ValueError("Required obligations failed or were skipped")
        receipt.update(status="pass", exit_code=0)
    except (Exception, KeyboardInterrupt) as error:
        receipt.update(status="fail" if attempt.commands else "blocked", exit_code=1,
                       error=f"{type(error).__name__}: {error}")
    finally:
        try:
            if receipt["status"] == "pass":
                playbook(args, attempt, receipt)
            after = evidence.snapshot()
            receipt["source_after_digest"] = after["tree_digest"]
            if receipt["source"] != after:
                raise ValueError("Source scope changed during execution")
            for item in receipt["candidate_artifacts"]:
                evidence.verify_file(item)
        except (Exception, KeyboardInterrupt) as error:
            receipt.update(status="fail", exit_code=1, error=str(error))
        # Retain all producer output, including reports emitted before a failed command.
        for path in sorted(attempt.directory.rglob("*")):
            if path.is_symlink():
                receipt.update(status="fail", exit_code=1, error="Symlink in evidence output")
            elif path.is_file():
                if path.stat().st_uid == os.getuid():
                    path.chmod(0o444)
                record = evidence.file_record(path)
                if record not in receipt["logs"] and record not in receipt["artifacts"]:
                    receipt["artifacts"].append(record)
        try:
            for item in receipt["logs"] + receipt["artifacts"]:
                evidence.verify_file(item)
        except (OSError, ValueError) as error:
            receipt.update(status="fail", exit_code=1, error=str(error))
        receipt.update(end_ns=time.time_ns(), duration_ns=time.monotonic_ns() - started)
        evidence.validate(receipt, evidence.document(ROOT / "qualification/gate-receipt.schema.json"))
        evidence.publish(attempt.receipt, receipt)
        attempt.directory.chmod(0o555)
    print(json.dumps({"status": receipt["status"], "scope": receipt["scope"], "receipt": str(attempt.receipt), "error": receipt["error"]}))
    return receipt["exit_code"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=sorted(GATES))
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--playbook", type=Path)
    parser.add_argument("--tool-image")
    parser.add_argument("--artifact", type=Path, action="append", default=[])
    parser.add_argument("--timeout", type=int, default=3600, help="Per-command deadline in seconds")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    try:
        attempt = evidence.Attempt(args.receipt, args.timeout)
        return qualify(args, attempt)
    except (OSError, ValueError) as error:
        print(f"Cannot create/publish qualification attempt: {error}", file=sys.stderr)
        return 2
