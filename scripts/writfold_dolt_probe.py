#!/usr/bin/env python3
"""Run DINT00 disposable native Dolt evidence, never a product or release gate."""
from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import time

sys.dont_write_bytecode = True
import writfold_evidence as evidence
from writfold_dolt_fixture import inventory, verify_backup

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "qualification/dolt-integration-profile.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def container(attempt: evidence.Attempt, env: dict, phase: str, receipt: dict,
              command: list[str]) -> Path:
    name = f"writfold-dolt-{phase}-{os.getpid()}-{time.monotonic_ns()}"
    record = {"phase": phase, "name": name, "mount": env["WRITFOLD_DOLT_WORK"],
              "status": "fail", "cleanup": {"status": "not_run"}}
    receipt["containers"].append(record)
    argv = ["docker", "compose", "--env-file", "/dev/null", "--project-directory", str(ROOT),
            "-f", str(ROOT / "compose.dolt-integration.yaml"), "run", "--rm", "--no-deps",
            "-T", "--name", name]
    # The admitted image contains root-readable proof artifacts. Observe them
    # without changing image permissions; native database phases stay unprivileged.
    record["user"] = "0:0" if phase == "observation" else f"{env['WRITFOLD_DOLT_UID']}:{env['WRITFOLD_DOLT_GID']}"
    if phase == "observation":
        argv.extend(["--user", "0:0", "--workdir", "/tmp", "--env", "HOME=/tmp"])
    argv.extend(["probe", *command])
    try:
        output = attempt.run(argv, env)
        record["status"] = "pass"
        return output
    finally:
        # The Docker client exiting does not establish that its container was reaped.
        cleanup = record["cleanup"]
        cleanup.update(status="fail", started_ns=time.time_ns())
        old_timeout = attempt.timeout
        attempt.timeout = 60
        try:
            query = ["docker", "ps", "--all", "--quiet", "--filter", f"name=^/{name}$"]
            remaining = attempt.run(query, env).read_text().strip()
            cleanup["forced_removal"] = bool(remaining)
            if remaining:
                attempt.run(["docker", "rm", "--force", name], env)
            absent = not attempt.run(query, env).read_text().strip()
            require(absent, f"Owned container remains: {name}")
            cleanup.update(status="pass", container_absent=True)
        except (Exception, KeyboardInterrupt) as error:
            cleanup["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            cleanup["ended_ns"] = time.time_ns()
            attempt.timeout = old_timeout


def admit_report(path: Path, mode: str, profile: dict, receipt: dict) -> dict:
    require(path.is_file() and not path.is_symlink(), f"Missing regular {mode} report")
    report = evidence.document(path)
    receipt["reports"][mode] = {"identity": evidence.file_record(path), "report": report}
    require(report.get("schema_version") == 1 and report.get("profile") == profile["profile"]
            and report.get("mode") == mode and report.get("status") == "pass",
            f"{mode} report did not pass the selected profile")
    required = profile[f"required_{mode}_checks"]
    checks = report.get("checks", [])
    require(len(checks) == len(required) and {c["name"] for c in checks} == set(required)
            and all(c.get("required") is True and c.get("status") == "pass" for c in checks),
            f"{mode} report has missing, duplicate, unknown or failed required checks")
    commands = report.get("commands", [])
    require(bool(commands) and not report.get("errors"), f"{mode} lacks successful native evidence")
    refusals = []
    for command in commands:
        require(not command.get("timed_out") and not command.get("error"),
                f"{mode} command timed out or failed")
        code = command.get("exit_code")
        if command.get("expected_refusal") is True:
            require(mode == "prepare" and type(code) is int and code == 1
                    and command.get("name") == "missing-history"
                    and command.get("refusal_kind") == "missing-history",
                    "Invalid native negative-control attribution")
            refusals.append(command)
        else:
            require(type(code) is int and code == 0, f"{mode} native command did not succeed")
    require(len(refusals) == (1 if mode == "prepare" else 0), "Missing/extra native refusal attribution")
    receipt["expected_native_refusals"].extend(refusals)
    return report


def calibrate_refusal(attempt: evidence.Attempt, profile: dict, receipt: dict) -> None:
    """A failed backup cannot borrow the missing-history negative's success."""
    original = attempt.directory / "prepare-work/report.json"
    changed = evidence.document(original)
    controls = [item for item in changed["commands"] if item.get("expected_refusal") is True]
    require(len(controls) == 1, "Calibration requires the actual native refusal")
    controls[0].update(name="native-backup", refusal_kind="backup-failure")
    path = attempt.directory / "misattributed-native-refusal.json"
    evidence.publish(path, changed)
    result = {"status": "fail", "positive": evidence.file_record(original),
              "mutated": evidence.file_record(path)}
    receipt["native_refusal_control"] = result
    try:
        admit_report(path, "prepare", profile, {"reports": {}, "expected_native_refusals": []})
    except ValueError as error:
        require(str(error) == "Invalid native negative-control attribution",
                "Unrelated failure in native-refusal calibration")
        result.update(status="pass", observed_refusal=str(error))
    else:
        raise ValueError("Misattributed native failure was admitted")
    evidence.publish(attempt.directory / "native-refusal-control.json", result)


def transfer_and_calibrate(attempt: evidence.Attempt, receipt: dict) -> Path:
    prepared = attempt.directory / "prepare-work"
    pins_path = prepared / "pins.json"
    require(pins_path.is_file() and not pins_path.is_symlink(), "Pin manifest must be regular")
    pins = evidence.document(pins_path)
    expected = pins["backup_inventory"]
    source = prepared / "prepared/backup"
    verify_backup(source, expected)
    restore = attempt.directory / "restore-work"
    restore.mkdir(mode=0o700)
    incoming = restore / "incoming"
    incoming.mkdir(mode=0o700)
    shutil.copytree(source, incoming / "backup", symlinks=True)
    with pins_path.open("rb") as original, (incoming / "pins.json").open("xb") as copied:
        shutil.copyfileobj(original, copied)
    verify_backup(source, expected)
    verify_backup(incoming / "backup", expected)
    require(evidence.sha(pins_path) == evidence.sha(incoming / "pins.json"), "Transferred pins changed")
    receipt["backup_transfer"] = {"inventory": expected,
                                  "source_pins": evidence.file_record(pins_path),
                                  "incoming_pins": evidence.file_record(incoming / "pins.json"),
                                  "restore_mount": str(restore),
                                  "isolation": "Only copied backup and pins; preparation root is not mounted"}
    tampered = attempt.directory / "tampered-backup"
    shutil.copytree(source, tampered, symlinks=True)
    verify_backup(tampered, expected)
    target = tampered / sorted(expected)[0]
    before = evidence.file_record(target)
    with target.open("r+b") as stream:
        first = stream.read(1)
        stream.seek(0)
        stream.write(bytes([first[0] ^ 1]) if first else b"\x01")
    calibration = {"status": "fail", "before": before, "after": evidence.file_record(target),
                   "expected_refusal": "Backup inventory mismatch", "native_restore_run": False}
    receipt["changed_backup_control"] = calibration
    try:
        verify_backup(tampered, expected)
    except ValueError as error:
        require(str(error) == "Backup inventory mismatch", "Unexpected tamper-control failure")
        calibration.update(status="pass", observed_refusal=str(error))
    else:
        raise ValueError("Changed native backup was admitted")
    verify_backup(source, expected)
    verify_backup(incoming / "backup", expected)
    calibration["tampered_inventory"] = inventory(tampered)
    evidence.publish(attempt.directory / "changed-backup-control.json", calibration)
    return restore


def host(args: argparse.Namespace) -> int:
    require(args.receipt.is_absolute(), "--receipt must be an absolute new external path")
    attempt = evidence.Attempt(args.receipt, 2700)
    receipt = {"schema_version": 1, "scope": "DINT00 integration-only native CLI foundation",
               "status": "fail", "release_gates_passed": [], "omnigraph_wf24": "blocked_unchanged",
               "rust_adapter": "not_run", "agent_e2e": "not_run", "commands": attempt.commands,
               "containers": [], "reports": {}, "expected_native_refusals": [],
               "checks": [], "phases": {p: "not_run" for p in ("observation", "prepare", "restore")},
               "errors": [], "started_ns": time.time_ns()}
    try:
        with (attempt.directory / "exclusive.lock").open("xb") as exclusive:
            fcntl.flock(exclusive, fcntl.LOCK_EX | fcntl.LOCK_NB)
            receipt["source_before"] = evidence.snapshot()
            profile = evidence.document(PROFILE)
            receipt["profile"] = profile
            receipt["checks"] = [{"name": name, "required": True, "status": "fail"}
                                 for name in profile["required_host_checks"]]
            checks = {item["name"]: item for item in receipt["checks"]}
            lock_path = ROOT / "toolchains.lock.json"
            lock = evidence.document(lock_path)
            observed = evidence.source_path(lock["observation"]["path"])
            receipt["inputs"] = [evidence.file_record(path) for path in
                                 (PROFILE, ROOT / profile["fixture"], lock_path, observed)]
            receipt["tool_lock"] = lock
            receipt["preflight"] = evidence.document(attempt.run(
                [sys.executable, str(ROOT / "scripts/writfold_preflight.py")]))
            require(lock.get("admitted") is True and lock.get("schema_version") == 1,
                    "Tool image not admitted")
            image = lock["image"]["reference"]
            require(re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", image) is not None,
                    "Mutable image reference refused")
            require(evidence.sha(observed) == lock["observation"]["sha256"], "Tool observation changed")
            inspection = evidence.document(attempt.run(["docker", "image", "inspect", image]))
            require(len(inspection) == 1 and image in inspection[0].get("RepoDigests", [])
                    and inspection[0]["Id"] == lock["image"]["id"]
                    and lock["image"]["platform"] == "linux/amd64"
                    and inspection[0]["Architecture"] == "amd64" and inspection[0]["Os"] == "linux",
                    "Local image differs from immutable admission")
            receipt["image_inspection"] = inspection
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith(("COMPOSE_", "WRITFOLD_DOLT_", "WRITFOLD_TOOL_"))}
            env.update(WRITFOLD_TOOL_IMAGE=image, WRITFOLD_DOLT_UID=str(os.getuid()),
                       WRITFOLD_DOLT_GID=str(os.getgid()))
            observation_work = attempt.directory / "observation-work"
            observation_work.mkdir(mode=0o700)
            (observation_work / "home").mkdir(mode=0o700)
            receipt["phases"]["observation"] = "fail"
            fresh = container(attempt, dict(env, WRITFOLD_DOLT_WORK=str(observation_work)),
                              "observation", receipt,
                              ["/usr/bin/env", "-u", "DOLT_DISABLE_VERSION_CHECK",
                               "/usr/bin/python3", "/work/writfold/scripts/writfold_tools.py"])
            require(evidence.document(fresh) == evidence.document(observed),
                    "Fresh network-disabled tool observation differs from admission")
            receipt["fresh_tool_observation"] = evidence.file_record(fresh)
            receipt["phases"]["observation"] = "pass"
            checks["operator-storage-and-image-admission"]["status"] = "pass"
            prepare = attempt.directory / "prepare-work"
            prepare.mkdir(mode=0o700)
            receipt["phases"]["prepare"] = "fail"
            container(attempt, dict(env, WRITFOLD_DOLT_WORK=str(prepare)), "prepare", receipt,
                      ["/usr/bin/python3", "/work/writfold/scripts/writfold_dolt_fixture.py",
                       "prepare", "--work", "/evidence"])
            admit_report(prepare / "report.json", "prepare", profile, receipt)
            receipt["phases"]["prepare"] = "pass"
            calibrate_refusal(attempt, profile, receipt)
            checks["native-refusal-attribution"]["status"] = "pass"
            restore = transfer_and_calibrate(attempt, receipt)
            checks["changed-backup-refusal"]["status"] = "pass"
            receipt["phases"]["restore"] = "fail"
            container(attempt, dict(env, WRITFOLD_DOLT_WORK=str(restore)), "restore", receipt,
                      ["/usr/bin/python3", "/work/writfold/scripts/writfold_dolt_fixture.py",
                       "restore", "--work", "/evidence"])
            admit_report(restore / "report.json", "restore", profile, receipt)
            receipt["phases"]["restore"] = "pass"
            checks["independent-restore"]["status"] = "pass"
            receipt["source_after"] = evidence.snapshot()
            require(receipt["source_before"]["tree_digest"] == receipt["source_after"]["tree_digest"],
                    "Checkout changed during attempt; refusing source-bound success")
            checks["source-stability"]["status"] = "pass"
            require(all(check["status"] == "pass" for check in receipt["checks"]), "Host checks incomplete")
            receipt["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        receipt["errors"].append(f"{type(error).__name__}: {error}")
    finally:
        if "source_after" not in receipt:
            try:
                receipt["source_after"] = evidence.snapshot()
            except (Exception, KeyboardInterrupt) as error:
                receipt["errors"].append(f"Final source snapshot: {type(error).__name__}: {error}")
        receipt["ended_ns"] = time.time_ns()
        receipt["phase_counts"] = {status: list(receipt["phases"].values()).count(status)
                                   for status in ("pass", "fail", "not_run")}
        receipt["logs"] = attempt.logs
        receipt["outputs"] = [evidence.file_record(path) for path in sorted(attempt.directory.rglob("*"))
                              if path.is_file() and not path.is_symlink()]
        evidence.publish(attempt.receipt, receipt)
    print(json.dumps({"scope": receipt["scope"], "status": receipt["status"],
                      "receipt": str(attempt.receipt)}, sort_keys=True))
    return 0 if receipt["status"] == "pass" else 1


def interrupted(signum: int, frame: object) -> None:
    raise KeyboardInterrupt(f"Signal {signum}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupted)
    try:
        return host(args)
    except (Exception, KeyboardInterrupt) as error:
        print(f"DINT00 refused: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
