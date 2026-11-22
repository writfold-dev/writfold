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
DEFINITIONS = ("scripts/gate", "scripts/writfold_gate.py", "scripts/writfold_evidence.py",
               "qualification/gate-receipt.schema.json", "compose.yaml", "scripts/unit_tests.py")


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
            "dirty": bool(git("status", "--porcelain=v1", "--untracked-files=normal", "--", ".")),
            "scope": sorted(files), "files": files,
            "scope_policy": "all Git tracked/nonignored untracked paths"}


def definition() -> str:
    files = {name: sha(source_path(name)) for name in DEFINITIONS}
    # Bind every gate input Git sees, tracked or not yet tracked.
    for name in git("ls-files", "-z", "--cached", "--others", "--exclude-standard", "qualification", "scripts", "docker").decode().split("\0"):
        if name and source_path(name).is_file():
            files[name] = sha(source_path(name))
    return digest(files)


def fixtures(source: dict) -> str:
    files = {name: value for name, value in source["files"].items() if name.startswith("fixtures/")}
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


def verify_file(entry: dict) -> None:
    path = Path(entry["path"])
    if not path.is_absolute() or path.is_symlink() or path.resolve().is_relative_to(ROOT):
        raise ValueError("Evidence must be an absolute, external regular file")
    if not path.is_file() or file_record(path) != entry:
        raise ValueError(f"Evidence bytes changed: {path}")
