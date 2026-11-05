"""Source-bound, exclusive local qualification evidence (standard library only)."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
PLAYBOOK = "e7d991e990e3dff9c5d70e4327c0e90298e2f67e"
TARGET = "x86_64-unknown-linux-musl"
ACTIVE_HANDOVER = "handovers/revision-5/writfold"
DEFINITIONS = ("scripts/gate", "scripts/writfold_gate.py", "scripts/writfold_evidence.py",
               "qualification/gate-receipt.schema.json", "compose.yaml",
               "qualification/handover-adoption.json",
               "qualification/revision-9-feature-adoption.json",
               "qualification/citation-profile.json", "docs/writfold/revision-9-features.md",
               "qualification/weft-integration.json", "docs/writfold/shared-client-references.md",
               "qualification/splicery-integration.json",
               "handovers/revision-9/writfold/MANIFEST.sha256",
               f"{ACTIVE_HANDOVER}/MANIFEST.sha256",
               f"{ACTIVE_HANDOVER}/registries/gates.json",
               f"{ACTIVE_HANDOVER}/registries/engineering-theorems.json",
               f"{ACTIVE_HANDOVER}/registries/fingerprints.json",
               f"{ACTIVE_HANDOVER}/plan/08-execution-and-acceptance.md")


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def sha(path: Path) -> str:
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def document(path: Path) -> object:
    def pairs(entries):
        result = {}
        for key, value in entries:
            if key in result:
                raise ValueError(f"Duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(path.read_text(), object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def source_path(name: str) -> Path:
    path = ROOT / name
    if Path(name).is_absolute() or ".." in Path(name).parts or not path.resolve().is_relative_to(ROOT):
        raise ValueError(f"Source path escapes checkout: {name}")
    return path


def git(*args: str) -> bytes:
    return subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), *args],
                          capture_output=True, check=True, timeout=60).stdout


def snapshot() -> dict:
    # Git enumerates source paths without walking target, caches, or ignored outputs.
    names = set(git("ls-files", "-z", "--cached", "--others", "--exclude-standard").decode().split("\0")) - {""}
    names.discard("qualification/status.json")  # Mutable index, hashed separately by consumers.
    files = {}
    for name in sorted(names):
        path = source_path(name)
        if not path.exists() and not path.is_symlink():
            files[name] = {"kind": "deleted"}
            continue
        mode = path.lstat().st_mode
        if stat.S_ISLNK(mode):
            raise ValueError(f"Source symlinks are not admitted: {name}")
        if not stat.S_ISREG(mode):
            raise ValueError(f"Source submodule/special file requires explicit admission: {name}")
        files[name] = {"sha256": sha(path), "mode": stat.S_IMODE(mode), "bytes": path.stat().st_size}
    return {"commit": git("rev-parse", "HEAD").decode().strip(),
            "tree_digest": digest(files),
            "dirty": bool(git("status", "--porcelain=v1", "--untracked-files=normal",
                              "--", ".", ":(exclude)qualification/status.json")),
            "scope": sorted(files), "files": files,
            "scope_policy": "all Git tracked/nonignored untracked paths except mutable qualification/status.json"}


def definition() -> str:
    files = {name: sha(source_path(name)) for name in DEFINITIONS}
    # Bind all active contracts and bundled fixtures even before Git tracks the import.
    for path in sorted(source_path(ACTIVE_HANDOVER).rglob("*")):
        if path.is_file():
            name = path.relative_to(ROOT).as_posix()
            files[name] = sha(source_path(name))
    for name in git("ls-files", "-z", "--cached", "--others", "--exclude-standard", "handover/stages", "qualification", "scripts", "docker").decode().split("\0"):
        if name and name != "qualification/status.json" and source_path(name).is_file():
            files[name] = sha(source_path(name))
    return digest(files)


def fixtures(source: dict) -> str:
    files = {name: value for name, value in source["files"].items()
             if name.startswith(("fixtures/", "tests/", "handover/registries/", "handover/reference/"))}
    for directory in ("registries", "reference", "examples", "qualification"):
        for path in sorted(source_path(f"{ACTIVE_HANDOVER}/{directory}").rglob("*")):
            if path.is_file():
                name = path.relative_to(ROOT).as_posix()
                files[name] = {"sha256": sha(source_path(name)), "bytes": path.stat().st_size}
    return digest(files)


def file_record(path: Path) -> dict:
    return {"path": str(path.resolve()), "sha256": sha(path), "bytes": path.stat().st_size}


def publish(path: Path, value: object) -> None:
    fd, temporary = tempfile.mkstemp(prefix=".receipt-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(canonical(value) + b"\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, 0o444)
        os.link(temporary, path)  # Exclusive even against a dangling symlink.
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        os.unlink(temporary)


class Attempt:
    def __init__(self, receipt: Path, timeout: int) -> None:
        self.receipt = receipt.absolute().parent.resolve() / receipt.name
        if self.receipt.is_relative_to(ROOT):
            raise ValueError("--receipt must be outside the source checkout")
        self.receipt.parent.mkdir(parents=True, exist_ok=True)
        if os.path.lexists(self.receipt):
            raise ValueError("Receipt exists; choose a new attempt")
        self.directory = self.receipt.with_name(self.receipt.name + ".attempt")
        self.directory.mkdir(mode=0o700)  # Reservation; failed attempts are never reused.
        self.timeout = timeout
        self.commands: list[dict] = []
        self.logs: list[dict] = []

    def run(self, argv: list[str], env: dict | None = None) -> Path:
        index = len(self.commands)
        output = self.directory / f"{index:03d}.stdout.log"
        errors = self.directory / f"{index:03d}.stderr.log"
        record = {"argv": argv, "cwd": str(ROOT), "start_ns": time.time_ns(),
                  "end_ns": None, "duration_ns": None, "exit_code": None,
                  "timeout_seconds": self.timeout, "timed_out": False}
        self.commands.append(record)
        start = time.monotonic_ns()
        try:
            with output.open("xb") as stdout, errors.open("xb") as stderr:
                process = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=stdout,
                                           stderr=stderr, start_new_session=True)
                try:
                    record["exit_code"] = process.wait(timeout=self.timeout)
                except (subprocess.TimeoutExpired, KeyboardInterrupt):
                    record["timed_out"] = True
                    os.killpg(process.pid, signal.SIGKILL)
                    record["exit_code"] = process.wait()
                    raise ValueError(f"Command interrupted/timed out: {argv}")
            if record["exit_code"]:
                raise ValueError(f"Command exited {record['exit_code']}: {argv}")
            return output
        finally:
            record.update(end_ns=time.time_ns(), duration_ns=time.monotonic_ns() - start)
            for path in (output, errors):
                if path.exists():
                    path.chmod(0o444)
                    self.logs.append(file_record(path))


def validate(value: object, schema: dict, root: dict | None = None) -> None:
    """Validate the deliberately small, closed receipt schema without dependencies."""
    root = root or schema
    if "$ref" in schema:
        return validate(value, root["$defs"][schema["$ref"].rsplit("/", 1)[1]], root)
    for rule in schema.get("allOf", []):
        validate(value, rule, root)
    if "if" in schema:
        try:
            validate(value, schema["if"], root)
        except ValueError:
            validate(value, schema.get("else", {}), root)
        else:
            validate(value, schema.get("then", {}), root)
    types = {"object": dict, "array": list, "string": str, "integer": int, "boolean": bool, "null": type(None)}
    allowed = schema.get("type", list(types))
    allowed = [allowed] if isinstance(allowed, str) else allowed
    if not any(type(value) is types[kind] for kind in allowed):
        raise ValueError("Receipt schema type mismatch")
    if "const" in schema and value != schema["const"]:
        raise ValueError("Unknown receipt schema/constant")
    if "enum" in schema and value not in schema["enum"]:
        raise ValueError("Receipt enum mismatch")
    if isinstance(value, dict):
        properties = schema.get("properties", {})
        if set(schema.get("required", [])) - value.keys():
            raise ValueError("Missing receipt fields")
        for key, item in value.items():
            rule = properties.get(key, schema.get("additionalProperties", True))
            if rule is False:
                raise ValueError(f"Unknown receipt field: {key}")
            if isinstance(rule, dict):
                validate(item, rule, root)
    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            raise ValueError("Empty required receipt array")
        if len(value) > schema.get("maxItems", len(value)):
            raise ValueError("Unexpected/skipped receipt array entries")
        for item in value:
            validate(item, schema.get("items", {}), root)
    if isinstance(value, str) and "pattern" in schema and not re.fullmatch(schema["pattern"], value):
        raise ValueError("Receipt identity format mismatch")
    if type(value) is int and value < schema.get("minimum", value):
        raise ValueError("Negative receipt count")


def frozen_inventories() -> list[dict]:
    checks = []
    for name in ("contracts.json", "evaluation-profile.json"):
        inventory = document(ROOT / "qualification" / name)
        if inventory.get("ancestor") != "1c17b94be37af8f82f222baddf31435f14172e1a" or not inventory.get("source_files"):
            raise ValueError(f"Invalid frozen inventory: {name}")
        for entry in inventory["source_files"]:
            path = source_path(entry["path"])
            passed = sha(path) == entry["sha256"] and path.stat().st_size == entry["bytes"]
            checks.append({"name": f"{name}:{entry['path']}", "required": True,
                           "status": "pass" if passed else "fail"})
    return checks


def verify_file(entry: dict) -> None:
    path = Path(entry["path"])
    if not path.is_absolute() or path.is_symlink() or path.resolve().is_relative_to(ROOT):
        raise ValueError("Evidence must be an absolute, external regular file")
    if not path.is_file() or file_record(path) != entry:
        raise ValueError(f"Evidence bytes changed: {path}")


def release_required(current: dict, candidate: list[dict]) -> dict:
    """Consume status independently; historical/scoped successes cannot qualify release."""
    status_path = ROOT / "qualification/status.json"
    status_digest = sha(status_path)
    status = document(status_path)
    expected = {f"WF{number:02d}" for number in range(32)}
    registry = document(source_path(f"{ACTIVE_HANDOVER}/registries/gates.json"))
    if (len(registry) != 32 or {gate["id"] for gate in registry} != expected
            or any(gate.get("required") is not True for gate in registry)):
        raise ValueError("Active contract must require WF00-WF31 exactly once")
    gates = status.get("gates", [])
    if len(gates) != 32 or {gate["id"] for gate in gates} != expected:
        raise ValueError("Status must inventory WF00-WF31 exactly once")
    if current.get("gate_definition_digest") != definition():
        raise ValueError("Candidate does not bind the active revision-5 gate definitions")
    if not current.get("source") or current.get("fixture_digest") != fixtures(current["source"]):
        raise ValueError("Candidate does not bind the active revision-5 fixtures")
    if not candidate:
        raise ValueError("Release requires --artifact for the exact candidate artifact")
    schema = document(ROOT / "qualification/gate-receipt.schema.json")
    report = {"status_sha256": status_digest, "checks": [], "receipts": []}
    proof_digest = sha(ROOT / "qualification/proofs.json")
    report["checks"].extend(proof_inventory_checks(current, candidate))
    report["proof_inventory_sha256"] = proof_digest
    theorem_path = ROOT / "qualification/engineering-theorems.json"
    theorem_digest = sha(theorem_path)
    theorems = document(theorem_path)
    obligations = theorems.get("obligations", [])
    if (theorems.get("normative_registry") != f"{ACTIVE_HANDOVER}/registries/engineering-theorems.json"
            or len(obligations) != 12 or {row["id"] for row in obligations} != {f"E{n:02d}" for n in range(1, 13)}):
        raise ValueError("Release requires the complete active E01-E12 inventory")
    report["engineering_theorems_sha256"] = theorem_digest
    formal = document(ROOT / "formal/engineering-theorems.json")
    for theorem in obligations:
        try:
            if (theorem.get("status") != "pass" or not theorem.get("theorem_symbols")
                    or not theorem.get("receipts")):
                raise ValueError("Engineering theorem lacks complete source-bound closure")
            mapped = formal["theorems"][theorem["id"]]
            if (mapped.get("full_obligation_status") != "pass"
                    or set(theorem["theorem_symbols"]) != set(mapped["theorem"])):
                raise ValueError("Engineering component is not a fully discharged mapped E obligation")
            for name in theorem["receipts"]:
                path = Path(name)
                record = file_record(path)
                verify_file(record)
                receipt = document(path)
                validate(receipt, schema)
                check_receipt(receipt, "WF31", current, candidate, scope="engineering-lean-only")
                if sha(path) != record["sha256"]:
                    raise ValueError("Engineering receipt changed while reading")
                report["receipts"].append(record)
            report["checks"].append({"name": theorem["id"], "required": True, "status": "pass"})
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["checks"].append({"name": theorem["id"], "required": True,
                                     "status": "fail", "error": str(error)})
    for gate in gates:
        try:
            if gate.get("required") is not True or gate.get("status") != "pass" or not gate.get("receipts"):
                raise ValueError("Required gate missing, blocked, failed, skipped or unrun")
            for name in gate["receipts"]:
                path = Path(name)
                record = file_record(path)
                verify_file(record)
                receipt = document(path)
                validate(receipt, schema)
                check_receipt(receipt, gate["id"], current, candidate)
                if sha(path) != record["sha256"]:
                    raise ValueError("Receipt changed while reading")
                report["receipts"].append(record)
            report["checks"].append({"name": gate["id"], "required": True, "status": "pass"})
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["checks"].append({"name": gate["id"], "required": True,
                                     "status": "fail", "error": str(error)})
    if sha(status_path) != status_digest:
        raise ValueError("Status changed during aggregation")
    if sha(theorem_path) != theorem_digest:
        raise ValueError("Engineering theorem inventory changed during aggregation")
    if sha(ROOT / "qualification/proofs.json") != proof_digest:
        raise ValueError("Proof inventory changed during aggregation")
    return report


def check_receipt(receipt: dict, gate: str, current: dict, candidate: list[dict],
                  *, scope: str = "full-gate") -> None:
    if (receipt["gate"] != gate or receipt["status"] != "pass" or receipt["exit_code"] != 0
            or receipt["skips"] or receipt["scope"] != scope
            or receipt["tool_admission"] != "admitted" or receipt["evidence_mode"] != "executed"):
        raise ValueError("Not freshly executed, complete admitted gate evidence")
    if any(current[key] is None for key in ("source", "cargo_lock_digest", "toolchain_digest",
                                           "fixture_digest", "gate_definition_digest", "playbook_commit")):
        raise ValueError("Candidate qualification identity is incomplete")
    for key in ("source", "cargo_lock_digest", "toolchain_digest", "fixture_digest",
                "gate_definition_digest", "playbook_commit", "target", "features", "tool_image"):
        if receipt[key] != current[key]:
            raise ValueError(f"Stale/mismatched {key}")
    counts = receipt["obligations"]
    if counts["required"] <= 0 or counts["passed"] != counts["required"] or counts["failed"] or counts["skipped"]:
        raise ValueError("Missing/failed/skipped required obligations")
    if not receipt["commands"] or not receipt["logs"] or not receipt["artifacts"]:
        raise ValueError("Missing executed command/log/artifact evidence")
    if any(command["exit_code"] != 0 or command["timed_out"] for command in receipt["commands"]):
        raise ValueError("Failed or timed out command")
    if receipt["source_after_digest"] != receipt["source"]["tree_digest"]:
        raise ValueError("Source changed during qualification")
    for item in receipt["logs"] + receipt["artifacts"] + receipt["candidate_artifacts"]:
        verify_file(item)
    hashes = {item["sha256"] for item in receipt["artifacts"] + receipt["candidate_artifacts"]}
    if not {item["sha256"] for item in candidate} <= hashes:
        raise ValueError("Receipt does not bind candidate artifact")
    if receipt["end_ns"] < receipt["start_ns"] or receipt["duration_ns"] < 0:
        raise ValueError("Invalid receipt clocks")


def proof_inventory_checks(current: dict, candidate: list[dict]) -> list[dict]:
    """Require named, source-bound proof receipts, never bare completion labels."""
    proofs = document(ROOT / "qualification/proofs.json")
    registry = document(source_path(f"{ACTIVE_HANDOVER}/registries/proofs.json"))
    rows = proofs.get("obligations", [])
    expected = {f"V{n:02d}" for n in range(1, 22)} | {f"M{n:02d}" for n in range(1, 17)}
    if (proofs.get("normative_registry") != f"{ACTIVE_HANDOVER}/registries/proofs.json"
            or len(registry) != 37 or {r["id"] for r in registry} != expected
            or len(rows) != 37 or {r["id"] for r in rows} != expected):
        raise ValueError("Release requires the complete active V01-V21/M01-M16 inventory")
    checks = [proof_row_check(row, row["id"],
              {"WF05"} if row["id"].startswith("V") else {"WF06", "WF07"},
              current, candidate) for row in rows]
    bridges = proofs.get("required_model_bridges", [])
    required = {frozenset(group) for group in (
        ("M12", "M13"), ("M12", "M14"), ("M15", "M01", "M02"),
        ("M15", "M03", "M11"), ("M16", "M03", "M10", "M11"),
        ("M13", "M09"), ("M14", "M07", "M08"))}
    actual = [frozenset(row["models"]) for row in bridges]
    if not required <= set(actual) or len(actual) != len(set(actual)):
        raise ValueError("Release requires every revision-5 composition bridge exactly once")
    checks.extend(proof_row_check(row, "bridge:" + "/".join(row["models"]),
                                 {"WF07", "WF31"}, current, candidate) for row in bridges)
    return checks


def proof_row_check(row: dict, name: str, required_gates: set[str],
                    current: dict, candidate: list[dict]) -> dict:
    result = {"name": name, "required": True, "status": "fail"}
    try:
        if row.get("status") != "pass" or not row.get("theorem_symbols") or not row.get("receipts"):
            raise ValueError("Proof requires named symbols and executed receipts, not a status label")
        symbols = {item if isinstance(item, str) else item["symbol"] for item in row["theorem_symbols"]}
        if not all(isinstance(symbol, str) and symbol for symbol in symbols):
            raise ValueError("Invalid proof symbol inventory")
        if name.startswith("V"):
            profile = document(ROOT / "qualification/verus-profile.json")
            declared = {symbol for crate in profile["crates"].values() for symbol in crate["functions"]}
        else:
            declared = {f"{path.stem}!{symbol}" for path in (ROOT / "spec").glob("*.tla")
                        for symbol in re.findall(r"(?m)^THEOREM\s+([A-Za-z_][A-Za-z0-9_]*)",
                                                 path.read_text())}
        if not symbols <= declared:
            raise ValueError("Proof names unregistered production/model declarations")
        gates = set()
        schema = document(ROOT / "qualification/gate-receipt.schema.json")
        for name in row["receipts"]:
            path = Path(name)
            identity = file_record(path)
            verify_file(identity)
            receipt = document(path)
            validate(receipt, schema)
            if receipt["gate"] not in required_gates:
                raise ValueError("Proof receipt names an unrelated gate")
            check_receipt(receipt, receipt["gate"], current, candidate)
            if sha(path) != identity["sha256"]:
                raise ValueError("Proof receipt changed while reading")
            gates.add(receipt["gate"])
        if gates != required_gates:
            raise ValueError("Proof lacks required verification/model/composition receipts")
        result["status"] = "pass"
    except (OSError, ValueError, KeyError, TypeError) as error:
        result["error"] = str(error)
    return result
