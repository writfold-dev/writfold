#!/usr/bin/env python3
"""Run pinned real-process regression scenarios from JSON-compatible YAML.

A component PASS never means full service E2E, grounding or effect authority.
Use a fresh external --receipt; no build, package manager, inference or native
worktree operation is invoked. Weft's owned control client launches Writfold;
Pipes bounds/captures the outer processes and owns their process groups.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import signal
import sys
import tarfile
import time
import traceback

sys.dont_write_bytecode = True
from writfold_evidence import Attempt, ROOT, digest, document, file_record, publish, sha
from writfold_stdio_process import Pipes
import writfold_toolchain_artifacts
import writfold_toolchain_sessions


def require(condition, message):
    if not condition:
        raise ValueError(message)


def field(value, selector):
    for key in selector.split("."):
        value = value[int(key)] if isinstance(value, list) else value[key]
    return value


def path_from_root(value):
    path = Path(value).expanduser()
    return (ROOT / path).resolve(strict=True)


def hex_digest(value):
    if not re.fullmatch(r"[0-9a-f]{64}", value):
        raise argparse.ArgumentTypeError("Expected lowercase 64-hex SHA-256")
    return value


def checked_cases(scenarios, expected):
    cases = scenarios["cases"]
    require(isinstance(cases, list) and cases, "No scenarios supplied")
    inventory = {}
    for case in cases:
        name = case["id"]
        require(isinstance(name, str) and re.fullmatch(r"[a-z0-9-]+", name), "Unsafe case ID")
        require(name not in inventory, "Duplicate case IDs")
        inventory[name] = digest(case)
        if case["action"] == "blocked":
            require(case["status"] == "BLOCKED" and case["requires"] and case["reason"],
                    "Unexplained blocker")
    require(inventory == expected, "Scenario inventory or assertions differ from the qualified profile")
    require(any(case["action"] != "blocked" for case in cases), "No runnable scenarios supplied")
    return cases


class Smoke:
    def __init__(self, args, attempt):
        self.args, self.attempt = args, attempt
        self.pipes = Pipes(attempt.directory)
        self.inputs = []
        self.paths = {}
        self.binaries = {}
        self.report = dict(schema="writfold-toolchain-smoke-receipt/1",
            invocation=[sys.executable, *sys.argv], start_ns=time.time_ns(),
            component_status="FAIL", full_e2e_status="BLOCKED",
            writfold_grounded=False, effect_authority=False,
            scope="local unresolved symbolic computation and transport control only",
            io_scope="Exact outer argv/stdin/stdout/stderr; Weft owns its real Writfold child. The control client's internal frames are not exposed or invented.",
            cases=[], errors=[], inputs=self.inputs, commands=self.pipes.records,
            provenance={}, binaries={})
        self.work = attempt.directory / "work"
        self.work.mkdir()
        self.env = {"PATH": "/nonexistent", "HOME": str(self.work), "TMPDIR": str(self.work),
                    "LANG": "C", "LC_ALL": "C", "TZ": "UTC"}

    def retain(self, path, name, expected=None):
        path = path.resolve(strict=True)
        before = file_record(path)
        if expected is not None:
            require(before["sha256"] == expected, f"Input digest mismatch: {path}")
        destination = self.attempt.directory / name
        with destination.open("xb") as stream:
            stream.write(path.read_bytes())
        require(sha(destination) == before["sha256"] and file_record(path) == before,
                f"Input changed while retaining: {path}")
        destination.chmod(0o444)
        self.inputs.append(dict(original=before, retained=file_record(destination)))
        return destination

    def provenance(self, profile):
        docs = {}
        for name, pin in profile["provenance"].items():
            path = (self.args.provenance_root / name / Path(pin["path"]).name
                    if self.args.provenance_root else path_from_root(pin["path"]))
            retained = self.retain(path, f"provenance-{name}.json", pin["sha256"])
            docs[name] = document(retained)
            self.report["provenance"][name] = file_record(retained)
        weft = docs["weft_runtime"]
        expected = self.report["binaries"]["weft"]["sha256"]
        require(weft["status"] == "pass" and weft["gate"] == "rust", "Weft runtime receipt failed")
        producer = weft["result"]["producer"]
        require(producer["status"] == "pass" and producer["binary_sha256"] == expected,
                "Weft runtime receipt does not bind executed artifact")
        require(weft["result"]["source_manifest"] and producer["source_manifest"] and
                producer["binary_source_sha256"] == weft["result"]["binary_source_sha256"],
                "Weft receipt has no matching build-source identity")
        package, package_rust = docs["weft_package"], docs["weft_package_rust"]
        require(package["status"] == "pass" and package["binary_sha256"] == expected and
                package["rust_receipt_sha256"] == profile["provenance"]["weft_package_rust"]["sha256"] and
                package_rust["status"] == "pass" and
                package_rust["result"]["producer"]["binary_sha256"] == expected,
                "Weft package provenance chain disagrees")
        reproduction = docs["splicery_reproduction"]
        expected = self.report["binaries"]["splicery"]["sha256"]
        require(reproduction["status"] == "passed_cold_offline_reproduction_component" and
                reproduction["artifact_sha256"] == expected == reproduction["rebuilt_artifact_sha256"],
                "Splicery reproduction does not bind executed artifact")
        manifest = reproduction["source_manifest"]
        require(manifest and manifest == reproduction["source_identity"]["source_manifest"],
                "Splicery reproduction source manifests disagree")
        for name in ("splicery_build", "splicery_build_b"):
            build = docs[name]
            require(build["status"] == "passed_locked_offline_static_musl_build_only" and
                    build["locked"] is True and build["offline"] is True and
                    build["client"]["sha256"] == expected and
                    build["cargo_manifest_sha256"] == manifest["Cargo.toml"] and
                    build["cargo_lock_sha256"] == manifest["Cargo.lock"],
                    f"Splicery final build provenance disagrees: {name}")
        self.report["source_identities"] = dict(
            weft=dict(source_manifest=weft["result"]["source_manifest"],
                      producer_source_manifest=producer["source_manifest"],
                      binary_source_sha256=producer["binary_source_sha256"]),
            splicery=reproduction["source_identity"])

    def prepare(self):
        profile_path = self.retain(self.args.profile, "input-profile.json")
        profile = document(profile_path)
        require(profile["schema"] == "writfold-toolchain-smoke-inputs/1", "Unknown input profile")
        cases_path = self.retain(self.args.cases, "scenarios.yaml")
        self.scenarios = document(cases_path)
        require(self.scenarios["schema"] == "writfold-toolchain-smoke/1", "Unknown scenario schema")
        self.cases = checked_cases(self.scenarios, profile["case_sha256"])
        for name in ("writfold_toolchain_smoke.py", "writfold_toolchain_artifacts.py",
                     "writfold_toolchain_sessions.py", "writfold_evidence.py",
                     "writfold_stdio_process.py", "writfold_conformance_process.py"):
            self.retain(ROOT / "scripts" / name, f"producer-{name}")
        for name in ("writfold", "weft", "splicery"):
            selected = getattr(self.args, name)
            expected = getattr(self.args, f"expect_{name}_sha256")
            if name != "writfold":
                selected = selected or path_from_root(profile[name]["path"])
                expected = expected or profile[name]["sha256"]
                require(expected == profile[name]["sha256"],
                        f"{name}: explicit digest must agree with receipt-bound profile")
            retained = self.retain(selected, name, expected)
            retained.chmod(0o555)
            self.binaries[name] = retained
            self.report["binaries"][name] = file_record(retained)
        self.provenance(profile)
        # The gate's stdio report binds the current Writfold bytes to its actual
        # source manifest; a revision name or a bare executable digest is not enough.
        report_path = self.args.writfold_report or self.args.writfold.resolve().parent / "stdio-report.json"
        retained = self.retain(report_path, "provenance-writfold-stdio.json")
        stdio = document(retained)
        require(stdio["result"] == "pass" and stdio["binary"]["sha256"] == self.args.expect_writfold_sha256,
                "Writfold stdio report does not qualify selected bytes")
        sources = stdio["sources"]
        require(sources["files"] and digest(sources["files"]) == sources["sha256"],
                "Writfold source manifest digest mismatch")
        for name, expected in sources["files"].items():
            path = path_from_root(name)
            require(path.is_relative_to(ROOT) and sha(path) == expected,
                    f"Writfold qualification is stale for current source: {name}")
        self.report["source_identities"]["writfold"] = sources
        self.report["provenance"]["writfold"] = file_record(retained)
        contract = self.args.weft_contract or path_from_root(profile["contract"]["path"])
        self.contract_archive = self.retain(contract, "weft-contract.tar.gz", profile["contract"]["sha256"])
        self.contract_digest = profile["contract"]["contract_sha256"]
        for key, name in self.scenarios["fixtures"].items():
            source = (self.args.cases.resolve().parent / name).resolve(strict=True)
            require(source.is_relative_to(self.args.cases.resolve().parent), "Fixture escapes scenario directory")
            self.paths[key] = self.retain(source, f"fixture-{source.name}")
        original = self.paths["original"].read_bytes()
        edit = self.scenarios["frozen_edit"]
        require(original[edit["lo"]:edit["hi"]] == edit["original"].encode() and
                original[:edit["lo"]] + edit["replacement"].encode() + original[edit["hi"]:] ==
                self.paths["expected_candidate"].read_bytes(), "Frozen edit and expected candidate disagree")
        # Only a disposable copy is presented as Splicery's target; no worktree is modified.
        target = self.work / "original.weft"
        target.write_bytes(original)
        target.chmod(0o444)
        self.paths["original"] = target
        self.original_identity = file_record(target)

    def contract_agreement(self):
        owner = ROOT / "crates/writfold-wire/contracts/agent-stdio/1"
        files = {path.name: path for path in owner.iterdir() if path.is_file()}
        files["LICENSE"] = ROOT / "LICENSE"
        agreements = []
        with tarfile.open(self.contract_archive, "r:gz") as archive:
            members = archive.getmembers()
            require(len(members) == len(files) and {m.name for m in members} == set(files),
                    "Owner/consumer contract inventory differs")
            for member in members:
                require(member.isfile() and member.size <= 1024 * 1024, "Unsafe contract archive member")
                data = archive.extractfile(member).read()
                require(data == files[member.name].read_bytes(), f"Contract bytes differ: {member.name}")
                retained = self.retain(files[member.name], f"contract-{member.name}")
                agreements.append(file_record(retained))
        require(sha(owner / "contract.json") == self.contract_digest, "Shared root contract pin differs")
        contract = document(owner / "contract.json")
        for entry in contract["files"]:
            require(sha(files[entry["path"]]) == entry["sha256"] and
                    files[entry["path"]].stat().st_size == entry["bytes"], "Contract payload binding differs")
        return dict(files=agreements, contract_sha256=self.contract_digest)

    def run_case(self, case):
        action = case["action"]
        if action == "contract-agreement":
            return self.contract_agreement()
        if action in {"artifact-workflow", "preview-refusal"}:
            return writfold_toolchain_artifacts.run(self, case)
        if action == "concurrent-control":
            return writfold_toolchain_sessions.run(self, case)
        weft = str(self.binaries["weft"])
        if action == "control":
            expected = self.args.expect_writfold_sha256
            if case["digest"] == "incorrect":
                expected = ("0" if expected[0] != "0" else "1") + expected[1:]
            else:
                require(case["digest"] == "correct", "Unknown control pin scenario")
            argv = [weft, "control", str(self.binaries["writfold"]), "--expect-sha256", expected, "--json"]
        elif action == "prove":
            argv = [weft, "prove", str(self.paths[case["source"]]), "--json"]
        elif action == "reuse":
            argv = [weft, "reuse", str(self.paths[case["report"]]), str(self.paths[case["source"]]), "--json"]
        elif action == "preview":
            argv = [str(self.binaries["splicery"]), "preview", "--root", str(self.work),
                    str(self.paths[case["recipe"]])]
        else:
            raise ValueError(f"Unknown runnable action: {action}")
        stdout, stderr, code = self.pipes.run(case["id"], argv, [("eof", None)],
            cwd=self.work, env=self.env, timeout=self.args.timeout, limit=2 * 1024 * 1024)
        require(code == case["exit_code"], f"{case['id']}: exit {code}, expected {case['exit_code']}")
        require(not stderr, f"{case['id']}: unexpected stderr (retained)")
        require(file_record(self.paths["original"]) == self.original_identity, "Original source was modified")
        output_path = Path(self.pipes.records[-1]["artifacts"]["stdout"]["path"])
        if action == "preview":
            require(stdout == self.paths[case["expected_bytes"]].read_bytes(), "Preview candidate bytes differ")
            self.paths[case["save"]] = output_path  # The exact captured bytes are the next real CLI input.
            return dict(candidate=file_record(output_path), original_unchanged=True)
        result = document(output_path)
        for selector, expected in case["expect"].items():
            observed = field(result, selector)
            require(type(observed) is type(expected) and observed == expected,
                    f"{case['id']}: {selector}: expected {expected!r}, observed {observed!r}")
        for selector in case.get("absent", []):
            require(selector not in result, f"{case['id']}: unexpected {selector}")
        if action == "control" and case["digest"] == "correct":
            require(result["peer"]["sha256"] == self.args.expect_writfold_sha256 and
                    result["session"]["contract_sha256"] == self.contract_digest and
                    result["session"]["instance_id"] and result["session"]["incarnation"],
                    "Control session did not bind the actual executable/shared contract")
        if action == "prove":
            source = self.paths[case["source"]]
            require(len(result["checks"]) == 1 and result["source"]["sha256"] == sha(source) and
                    result["source"]["content"] == {"encoding": "utf-8", "text": source.read_bytes().decode("utf-8")},
                    "Prove did not evaluate the exact intended source")
        if action == "reuse":
            require(result["prior"]["sha256"] == sha(self.paths[case["report"]]) and
                    result["candidate"]["sha256"] == sha(self.paths[case["source"]]), "Reuse input identity differs")
            if result["decision"] == "refused":
                require(result["differences"] == [dict(field="source.sha256",
                    expected=sha(self.paths["original"]), observed=sha(self.paths[case["source"]]))],
                    "Changed-source reuse was refused for a different reason")
        if "save" in case:
            self.paths[case["save"]] = output_path
        return dict(output=file_record(output_path), observed=result)

    def execute(self):
        started = time.monotonic_ns()
        try:
            self.prepare()
            cases = self.cases
            for case in cases:
                row = dict(id=case["id"], action=case["action"], status="NOT_RUN")
                self.report["cases"].append(row)
                if case["action"] == "blocked":
                    # Blockers are part of the pinned inventory, never successful executions.
                    row.update(status="BLOCKED", requires=case["requires"], reason=case["reason"])
            for case, row in zip(cases, self.report["cases"]):
                if row["status"] == "BLOCKED":
                    continue
                row["status"] = "FAIL"
                row["observations"] = self.run_case(case)
                row["status"] = "PASS"
            self.report["component_status"] = "PASS"
        except (Exception, KeyboardInterrupt) as error:
            self.report["errors"].append(f"{type(error).__name__}: {error}")
            (self.attempt.directory / "failure.log").write_text(traceback.format_exc())
        finally:
            try:
                for name, expected in self.report.get("source_identities", {}).get("writfold", {}).get("files", {}).items():
                    require(sha(path_from_root(name)) == expected,
                            f"Writfold source changed during smoke: {name}")
                for item in self.inputs:
                    require(file_record(Path(item["original"]["path"])) == item["original"], "Original input changed")
                    require(file_record(Path(item["retained"]["path"])) == item["retained"], "Retained input changed")
                for command in self.pipes.records:
                    for item in command["artifacts"].values():
                        require(file_record(Path(item["path"])) == item, "Process evidence changed")
                self.report["binaries_after"] = {name: file_record(path) for name, path in self.binaries.items()}
                require(self.report["binaries_after"] == self.report["binaries"], "Executed artifact changed")
            except (Exception, KeyboardInterrupt) as error:
                self.report["component_status"] = "FAIL"
                self.report["errors"].append(f"Evidence integrity: {error}")
            self.report["counts"] = {status: sum(row["status"] == status for row in self.report["cases"])
                                     for status in ("PASS", "FAIL", "BLOCKED", "NOT_RUN")}
            self.report.update(end_ns=time.time_ns(), duration_ns=time.monotonic_ns() - started)
            artifacts = []
            for path in sorted(self.attempt.directory.rglob("*")):
                if path.is_file():
                    path.chmod(0o555 if path in self.binaries.values() else 0o444)
                    artifacts.append(file_record(path))
            self.report["artifacts"] = artifacts
            publish(self.attempt.receipt, self.report)
            self.attempt.directory.chmod(0o555)
        print(json.dumps(dict(component_status=self.report["component_status"],
            full_e2e_status="BLOCKED", receipt=str(self.attempt.receipt), errors=self.report["errors"])))
        return 0 if self.report["component_status"] == "PASS" else 1

def interrupted(signum, frame):
    raise KeyboardInterrupt(f"signal {signum}")



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--writfold", type=Path, required=True)
    parser.add_argument("--expect-writfold-sha256", type=hex_digest, required=True)
    parser.add_argument("--writfold-report", type=Path, help="Default: stdio-report.json beside the supplied Writfold binary")
    for name in ("weft", "splicery"):
        parser.add_argument(f"--{name}", type=Path)
        parser.add_argument(f"--expect-{name}-sha256", type=hex_digest)
    parser.add_argument("--profile", type=Path, default=ROOT / "qualification/toolchain-smoke-profile.json")
    parser.add_argument("--cases", type=Path, default=ROOT / "fixtures/qualification/toolchain-smoke.yaml")
    parser.add_argument("--weft-contract", type=Path, help="Relocated unchanged vendored contract archive")
    parser.add_argument("--provenance-root", type=Path, help="Relocated receipts: ROOT/PROFILE_KEY/original-basename")
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=45, help="Per-process deadline in seconds")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    try:
        return Smoke(args, Attempt(args.receipt, args.timeout)).execute()
    except (OSError, ValueError) as error:
        print(f"Cannot reserve/publish smoke attempt: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    raise SystemExit(main())
