"""Retained subprocesses and input identities for the three-host producer."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-unknown-linux-musl"


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def record(path: Path) -> dict[str, object]:
    return {"path": str(path), "sha256": sha(path), "bytes": path.stat().st_size}


def manifest(paths: list[Path], base: Path = ROOT) -> dict[str, Any]:
    files = {}
    for path in sorted(set(paths)):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Missing, symlinked or nonregular declared input: {path}")
        files[path.relative_to(base).as_posix()] = sha(path)
    if not files:
        raise ValueError("Empty source manifest")
    return {"files": files, "sha256": hashlib.sha256(canonical(files).encode()).hexdigest()}


def write_json(path: Path, value: object) -> None:
    with path.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n")


class Commands:
    """Every attempted command keeps its exit, timeout, exact argv and raw logs."""

    def __init__(self, directory: Path) -> None:
        self.directory = directory
        self.records: list[dict[str, Any]] = []
        self.env = os.environ.copy()
        self.env.update(PYTHONDONTWRITEBYTECODE="1", CARGO_NET_OFFLINE="true",
                        PYTHONPATH=str(ROOT / "src"))

    def run(self, name: str, argv: list[str], *, cwd: Path = ROOT,
            stdin: Path | None = None, timeout: int = 120,
            environment: dict[str, str] | None = None) -> Path:
        prefix = f"{len(self.records):03d}-{name}"
        stdout = self.directory / f"{prefix}.stdout.log"
        stderr = self.directory / f"{prefix}.stderr.log"
        env = self.env | (environment or {})
        entry: dict[str, Any] = {
            "name": name, "argv": argv, "cwd": str(cwd),
            "environment": {key: value for key, value in sorted(env.items())
                            if key.startswith(("CARGO", "RUST", "LEAN", "LAKE", "PYTHON", "ELAN"))
                            or key in ("PATH", "CC", "CXX", "AR", "LD", "CFLAGS", "LDFLAGS")},
            "stdin": record(stdin) if stdin else None,
            "start_ns": time.time_ns(), "end_ns": None, "duration_ns": None,
            "exit_code": None, "timed_out": False, "timeout_seconds": timeout,
        }
        self.records.append(entry)
        start = time.monotonic_ns()
        try:
            with stdout.open("xb") as out, stderr.open("xb") as err:
                source = stdin.open("rb") if stdin else open(os.devnull, "rb")
                with source:
                    child = subprocess.Popen(argv, cwd=cwd, env=env, stdin=source,
                                             stdout=out, stderr=err, start_new_session=True)
                    try:
                        entry["exit_code"] = child.wait(timeout=timeout)
                    except (subprocess.TimeoutExpired, KeyboardInterrupt) as exc:
                        entry["timed_out"] = isinstance(exc, subprocess.TimeoutExpired)
                        entry["interrupted"] = isinstance(exc, KeyboardInterrupt)
                        try:
                            os.killpg(child.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass  # The process group exited between the deadline and kill.
                        finally:
                            entry["exit_code"] = child.wait()
                        raise
            if entry["exit_code"] != 0:
                raise ValueError(f"{name} exited {entry['exit_code']}; see {stderr.name}")
            return stdout
        except BaseException as exc:
            entry["error"] = f"{type(exc).__name__}: {exc}"
            raise
        finally:
            entry.update(end_ns=time.time_ns(), duration_ns=time.monotonic_ns() - start)
            entry["logs"] = [record(path) for path in (stdout, stderr) if path.is_file()]
            write_json(self.directory / f"{prefix}.command.json", entry)


def observe_tools(commands: Commands) -> dict[str, Any]:
    tools = {}
    declarations = {
        "python": ([os.sys.executable, "--version"], None),
        "lean": (["lean", "--version"], "version 4.21.0"),
        "lake": (["lake", "--version"], "Lean version 4.21.0"),
        "rustc": (["rustc", "--version", "--verbose"], "release: 1.98.0\n"),
        "cargo": (["cargo", "--version", "--verbose"], "cargo 1.98.0 "),
        "musl-gcc": (["musl-gcc", "--version"], None),
    }
    for name, (argv, expected) in declarations.items():
        if name in ("rustc", "cargo"):
            resolved = commands.run(f"resolve-{name}", ["rustup", "which", name], timeout=60)
            executable = Path(resolved.read_text(encoding="utf-8").strip())
        else:
            found = shutil.which(argv[0])
            if found is None:
                raise ValueError(f"Missing required tool: {name}")
            executable = Path(found).resolve(strict=True)
        output = commands.run(f"version-{name}", [str(executable), *argv[1:]], timeout=60)
        version = output.read_text(encoding="utf-8")
        if expected is not None and expected not in version:
            raise ValueError(f"Wrong {name} version: {version!r}")
        tools[name] = {**record(executable), "version": version}
    packages = Path("/opt/writfold-toolchain/distro-packages.tsv")
    tools["distro_packages"] = record(packages)
    write_json(commands.directory / "tools-observed.json", tools)
    lock = json.loads((ROOT / "toolchains.lock.json").read_text(encoding="utf-8"))
    observation_path = ROOT / "qualification/toolchain-observation.json"
    if (lock.get("schema_version") != 1 or lock.get("admitted") is not True
            or lock.get("target") != TARGET
            or lock.get("observation", {}).get("path") != observation_path.relative_to(ROOT).as_posix()
            or lock["observation"].get("sha256") != sha(observation_path)):
        raise ValueError("Missing or inconsistent admitted tool observation")
    observation = json.loads(observation_path.read_text(encoding="utf-8"))
    admitted = {tool["name"]: tool for tool in observation["tools"]}
    for name, tool in tools.items():
        if name == "distro_packages":
            if tool["sha256"] != observation["distro_packages_sha256"]:
                raise ValueError("Distro package inventory differs from admitted image")
        elif (tool["sha256"] != admitted[name]["sha256"]
              or tool["version"].strip() != admitted[name]["version_output"]):
            raise ValueError(f"Executed tool differs from admitted observation: {name}")
    return tools


def source_manifests() -> dict[str, Any]:
    formal = ROOT / "formal"
    lean = list((formal / "Kctx").rglob("*.lean"))
    lean.extend(formal / name for name in (
        "Kctx.lean", "lean-toolchain", "lakefile.toml", "lake-manifest.json"))
    python = list((ROOT / "src" / "kctx").rglob("*.py"))
    python.extend(ROOT / name for name in (
        "scripts/conformance_check.py", "scripts/semantic_cases.py",
        "scripts/semantic_mutations.py", "tests/semantics/examples.py",
        "tests/semantics/__init__.py"))
    rust = [ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
    for pattern in ("*.rs", "Cargo.toml", "*.lock"):
        rust.extend((ROOT / "crates").rglob(pattern))
    rust.extend(path for path in (ROOT / ".cargo").glob("*") if path.is_file())
    rust.extend(path for path in (ROOT / "vendor").rglob("*") if path.is_file())
    producer = list((ROOT / "scripts").glob("writfold_conformance*.py"))
    producer.extend(ROOT / name for name in (
        "formal/Dockerfile", "docker/Dockerfile.tools", "compose.yaml",
        "toolchains.lock.json", "qualification/toolchain-observation.json", "pyproject.toml", "uv.lock"))
    fixtures = sorted((ROOT / "tests" / "conformance").glob("*.jsonl"))
    return {"lean": manifest(lean), "python": manifest(python),
            "rust": manifest(rust), "producer_and_locks": manifest(producer),
            "fixtures": manifest(fixtures)}


def retain_binary(source: Path, destination: Path) -> dict[str, object]:
    if not source.is_file() or not os.access(source, os.X_OK):
        raise ValueError(f"Missing executable build artifact: {source}")
    with source.open("rb") as original, destination.open("xb") as retained:
        shutil.copyfileobj(original, retained)
    destination.chmod(0o555)
    result = record(destination)
    if result["sha256"] != sha(source):
        raise ValueError(f"Retained binary changed during copy: {source}")
    return {**result, "built_path": str(source)}
