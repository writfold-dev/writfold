#!/usr/bin/env python3
"""Supplemental DINT01: python3 scripts/writfold_dolt_reader.py --receipt ABS_NEW_EXTERNAL.

Retains ABS_NEW_EXTERNAL plus ABS_NEW_EXTERNAL.attempt/{build,native-work}/,
Cargo JSON, static-musl consumer, native command streams and exact refusal report.
This command never promotes a release gate; failures reserve their attempt forever.
"""
from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import time

sys.dont_write_bytecode = True
import writfold_evidence as evidence
import writfold_gate as gate
from writfold_dolt_fixture import require
from writfold_dolt_reader_cases import CHECKS, PROFILE, READ_PROFILE, FIXTURE, interrupted

ROOT = Path(__file__).resolve().parents[1]
BUILD = r'''
import json, os, pathlib, shutil, sys
sys.path.insert(0, '/work/writfold/scripts')
from writfold_stdio_process import Pipes
from writfold_evidence import publish, file_record
root = pathlib.Path('/evidence/build')
root.mkdir(mode=0o755)
(root / 'commands').mkdir()
pipes = Pipes(root / 'commands')
report = dict(status='fail', commands=pipes.records)
try:
    argv = ['cargo','build','-p','writfold-store','--example','dolt_read','--release',
            '--target','x86_64-unknown-linux-musl','--locked','--offline','--message-format=json']
    out, err, code = pipes.run('cargo-build', argv, [('eof',None)], cwd=pathlib.Path('/work/writfold'),
                               env=dict(os.environ), timeout=2400, limit=64*1024*1024)
    (root / 'cargo.jsonl').write_bytes(out)
    if code != 0: raise ValueError('Cargo failed; retained command streams')
    records = [json.loads(line) for line in out.splitlines()]
    if not any(r.get('reason') == 'build-finished' and r.get('success') is True for r in records):
        raise ValueError('Missing Cargo successful build-finished event')
    artifacts = [r for r in records if r.get('reason') == 'compiler-artifact'
                 and r.get('target',{}).get('name') == 'dolt_read'
                 and r.get('target',{}).get('kind') == ['example'] and r.get('executable')]
    if len(artifacts) != 1: raise ValueError('Missing/ambiguous actual example artifact')
    artifact = artifacts[0]
    if pathlib.Path(artifact['manifest_path']).resolve() != pathlib.Path('/work/writfold/crates/writfold-store/Cargo.toml'):
        raise ValueError('Cargo artifact is not the actual store package')
    if pathlib.Path(artifact['target']['src_path']).resolve() != pathlib.Path('/work/writfold/crates/writfold-store/examples/dolt_read.rs'):
        raise ValueError('Cargo artifact is not the actual reader consumer source')
    source = pathlib.Path(artifact['executable'])
    if not source.is_relative_to('/cache/target/rust-1.98.0/x86_64-unknown-linux-musl/release/examples'):
        raise ValueError('Wrong Cargo artifact target path')
    destination = root / 'dolt_read'
    shutil.copyfile(source, destination)
    destination.chmod(0o755)
    out, err, code = pipes.run('static-elf', ['readelf','-h','-l','-d',str(destination)], [('eof',None)],
                               cwd=root, env=dict(os.environ), timeout=30)
    text = out.decode('utf-8')
    if code != 0 or 'ELF64' not in text or 'Advanced Micro Devices X86-64' not in text:
        raise ValueError('Not an x86-64 ELF')
    if 'INTERP' in text or '(NEEDED)' in text:
        raise ValueError('Consumer has a dynamic interpreter/library dependency')
    report.update(status='pass', cargo_artifact=artifact, consumer=file_record(destination),
                  cargo_json=file_record(root/'cargo.jsonl'), static_elf=True)
except BaseException as error:
    report['error'] = type(error).__name__ + ': ' + str(error)
    raise
finally:
    publish(root / 'report.json', report)
'''
HOST_CHECKS = ("operator-image-admission", "actual-static-musl-build", "native-reader-campaign", "source-stability")


def container(attempt, env, receipt, phase, compose, service, command):
    name = f"writfold-reader-{phase}-{os.getpid()}-{time.monotonic_ns()}"
    record = dict(name=name, phase=phase, status="fail", cleanup=dict(status="fail"))
    receipt["containers"].append(record)
    try:
        result = attempt.run(compose + ["run", "--rm", "--no-deps", "-T", "--name", name, service, *command], env)
        record["status"] = "pass"
        return result
    finally:
        timeout = attempt.timeout
        attempt.timeout = 60
        try:
            query = ["docker", "ps", "--all", "--quiet", "--filter", f"name=^/{name}$"]
            remaining = attempt.run(query, env).read_text().strip()
            record["cleanup"]["forced_removal"] = bool(remaining)
            if remaining:
                attempt.run(["docker", "rm", "--force", name], env)
            require(not attempt.run(query, env).read_text().strip(), "Owned container remains: " + name)
            record["cleanup"].update(status="pass", container_absent=True)
        finally:
            attempt.timeout = timeout


def host(args):
    require(args.receipt.is_absolute(), "--receipt must be an absolute new external path")
    require(os.getuid() != 0, "Use an unprivileged operator for native reader cases")
    attempt = evidence.Attempt(args.receipt, 2700)
    receipt = dict(schema_version=1, scope="DINT01 supplemental actual typed Rust exact reader",
                   status="fail", commands=attempt.commands, artifacts=[], containers=[], errors=[],
                   checks=[dict(name=n, required=True, status="fail") for n in HOST_CHECKS],
                   declared_native_checks=list(CHECKS), release_gates_passed=[], agent_e2e="not_run",
                   writer="not_run", omnigraph_wf24="blocked_unchanged", started_ns=time.time_ns())
    checks = {c["name"]: c for c in receipt["checks"]}
    try:
        with (attempt.directory / "exclusive.lock").open("xb") as exclusive:
            fcntl.flock(exclusive, fcntl.LOCK_EX | fcntl.LOCK_NB)
            receipt["source_before"] = evidence.snapshot()
            receipt["inputs"] = [evidence.file_record(path) for path in (
                Path(__file__).resolve(), ROOT / "scripts/writfold_dolt_reader_cases.py",
                ROOT / "compose.dolt-reader.yaml", ROOT / "compose.yaml", PROFILE, READ_PROFILE, FIXTURE,
                ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "toolchains.lock.json")]
            receipt["preflight"] = evidence.document(attempt.run(
                [sys.executable, str(ROOT / "scripts/writfold_preflight.py")]))
            image = gate.admit_image(argparse.Namespace(tool_image=None, command="native-smoke"), attempt, receipt)
            checks["operator-image-admission"]["status"] = "pass"
            env = gate.environment(attempt, image)
            container(attempt, env, receipt, "build", gate.compose(), "unit", ["python3", "-c", BUILD])
            build = evidence.document(attempt.directory / "build/report.json")
            receipt["build"] = build
            require(build["status"] == "pass" and build["static_elf"] is True, "Actual static build did not pass")
            binary = attempt.directory / "build/dolt_read"
            require(evidence.sha(binary) == build["consumer"]["sha256"], "Retained consumer changed")
            checks["actual-static-musl-build"]["status"] = "pass"
            work = attempt.directory / "native-work"
            work.mkdir(mode=0o700)
            shutil.copyfile(binary, work / "dolt_read")
            (work / "dolt_read").chmod(0o500)
            require(evidence.sha(work / "dolt_read") == evidence.sha(binary), "Native consumer transfer differs")
            native_env = {k: v for k, v in os.environ.items()
                          if not k.startswith(("COMPOSE_", "WRITFOLD_TOOL_", "WRITFOLD_DOLT_"))}
            native_env.update(WRITFOLD_TOOL_IMAGE=image, WRITFOLD_DOLT_WORK=str(work),
                              WRITFOLD_DOLT_UID=str(os.getuid()), WRITFOLD_DOLT_GID=str(os.getgid()))
            compose = ["docker", "compose", "--env-file", "/dev/null", "--project-directory", str(ROOT),
                       "-f", str(ROOT / "compose.dolt-reader.yaml")]
            container(attempt, native_env, receipt, "native", compose, "reader",
                      ["/usr/bin/python3", "/work/writfold/scripts/writfold_dolt_reader_cases.py", "--work", "/evidence"])
            report = evidence.document(work / "report.json")
            receipt["native"] = report
            require(report["status"] == "pass" and not report["errors"], "Native report failed")
            actual = report["checks"]
            require(len(actual) == len(CHECKS) and {c["name"] for c in actual} == set(CHECKS)
                    and all(c["status"] == "pass" and c["required"] is True for c in actual),
                    "Missing, duplicate or unexercised native check")
            require(report["consumer"]["sha256"] == evidence.sha(binary), "Executed consumer differs")
            refusals = {r["name"] for r in report["expected_refusals"]}
            for command in report["commands"]:
                require(not command.get("timed_out") and not command.get("error"), "Native command infrastructure failure")
                if command.get("expected_refusal") is True:
                    require(command["name"] in refusals and command["exit_code"] == 1,
                            "Unattributed native refusal")
                else:
                    require(command["exit_code"] == 0, "Unexpected native command failure")
            checks["native-reader-campaign"]["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        receipt["errors"].append(f"{type(error).__name__}: {error}")
    finally:
        try:
            receipt["source_after"] = evidence.snapshot()
            require(receipt.get("source_before") == receipt["source_after"], "Source changed during campaign")
            checks["source-stability"]["status"] = "pass"
        except (Exception, KeyboardInterrupt) as error:
            receipt["errors"].append(f"{type(error).__name__}: {error}")
        # Bind all retained outputs, including partial build/native failures. Native
        # symlink fault fixtures are intentionally not followed or treated as files.
        try:
            receipt["retained_outputs"] = [evidence.file_record(p) for p in sorted(attempt.directory.rglob("*"))
                                           if p.is_file() and not p.is_symlink()]
        except (Exception, KeyboardInterrupt) as error:
            receipt["errors"].append(f"{type(error).__name__}: {error}")
        if not receipt["errors"] and all(c["status"] == "pass" for c in receipt["checks"]):
            receipt["status"] = "pass"
        receipt["ended_ns"] = time.time_ns()
        evidence.publish(attempt.receipt, receipt)
    return 0 if receipt["status"] == "pass" else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupted)
    result = host(args)
    print(json.dumps({"status": "pass" if result == 0 else "fail",
                      "scope": "DINT01 actual typed Rust exact reader", "receipt": str(args.receipt)}))
    raise SystemExit(result)
