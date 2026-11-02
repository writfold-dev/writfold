#!/usr/bin/env python3
"""Build and execute the actual musl bodies with empty isolated Cargo caches."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-unknown-linux-musl"
PACKAGES = ("writfold-domain", "writfold-kernel", "writfold-wire", "writfold-protocol")


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def manifest(root: Path) -> dict[str, str]:
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"Unadmitted symlink in closure: {path}")
        if path.is_file():
            result[path.relative_to(root).as_posix()] = sha(path)
    return result


def source_identity() -> dict:
    files = {name: sha(ROOT / name) for name in
             ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "toolchains.lock.json",
              "scripts/writfold_offline_spike.py")}
    for package in PACKAGES:
        prefix = f"crates/{package}"
        files.update({f"{prefix}/{name}": value for name, value in manifest(ROOT / prefix).items()})
    return files


def execute(argv: list[str], directory: Path, label: str, env: dict, *, stdin: str | None = None) -> str:
    log = directory / f"{label}.stdout.log"
    errors = directory / f"{label}.stderr.log"
    record = {"argv": argv, "started_ns": time.time_ns(), "cwd": str(ROOT)}
    with log.open("xb") as stdout, errors.open("xb") as stderr:
        process = subprocess.Popen(argv, cwd=ROOT, env=env, text=True, stdin=subprocess.PIPE,
                                   stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            process.communicate(stdin, timeout=600)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
            record["timeout"] = True
        record.update(exit_code=process.returncode, ended_ns=time.time_ns(),
                      stdout_sha256=sha(log), stderr_sha256=sha(errors))
    (directory / f"{label}.command.json").write_text(json.dumps(record, indent=2) + "\n")
    if record.get("timeout") or process.returncode:
        raise ValueError(f"{label} did not complete successfully; retained logs: {directory}")
    return log.read_text()


def inspect_binary(binary: Path, output: Path, name: str, env: dict) -> dict:
    with binary.open("rb") as stream:
        header = stream.read(20)
    if header[:6] != b"\x7fELF\x02\x01" or int.from_bytes(header[18:20], "little") != 62:
        raise ValueError("Expected little-endian Linux x86_64 ELF")
    description = execute(["file", str(binary)], output, f"{name}-file", env)
    headers = execute(["readelf", "-l", str(binary)], output, f"{name}-headers", env)
    dynamic = execute(["readelf", "-d", str(binary)], output, f"{name}-dynamic", env)
    if "INTERP" in headers or "(NEEDED)" in dynamic:
        raise ValueError("Musl artifact has an interpreter or dynamic runtime dependency")
    retained = output / name
    with binary.open("rb") as source, retained.open("xb") as destination:
        shutil.copyfileobj(source, destination)
    retained.chmod(0o555)
    retained_digest = sha(retained)
    if retained_digest != sha(binary):
        raise ValueError("Retained binary differs from compiled bytes")
    return {"path": str(retained), "sha256": retained_digest, "bytes": retained.stat().st_size,
            "description": description.strip(), "interpreter": False, "needed_libraries": []}


def qualify(vendor: Path, output: Path) -> dict:
    if set(path.name for path in Path("/sys/class/net").iterdir()) != {"lo"}:
        raise ValueError("Offline qualification requires an isolated network namespace")
    before = source_identity()
    dependencies = manifest(vendor)
    if not dependencies or not any(name.endswith(".cargo-checksum.json") for name in dependencies):
        raise ValueError("No vendored dependency closure")
    cache = Path(os.environ.get("CARGO_TARGET_DIR", ""))
    if not cache.is_absolute() or not cache.is_relative_to(Path("/cache/target")):
        raise ValueError("An admitted Docker-managed cache is required")
    isolated = cache / "cold-offline" / uuid.uuid4().hex
    home, target = isolated / "cargo-home", isolated / "target"
    home.mkdir(parents=True, mode=0o755)
    target.mkdir(mode=0o755)
    if any(home.iterdir()) or any(target.iterdir()):
        raise ValueError("Cold Cargo home/target are not empty")
    allowed = ("PATH", "HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "CARGO_BUILD_JOBS",
               "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER")
    env = {name: os.environ[name] for name in allowed if name in os.environ}
    env.update(CARGO_HOME=str(home), CARGO_TARGET_DIR=str(target), CARGO_NET_OFFLINE="true")
    common = ["cargo", "build", "--locked", "--offline", "--release", "--target", TARGET,
              "--config", 'source.crates-io.replace-with="vendored-sources"',
              "--config", f"source.vendored-sources.directory={json.dumps(str(vendor))}"]
    execute([*common, "-p", "writfold-protocol", "--example", "generation_smoke"],
            output, "generation-build", env)
    execute([*common, "-p", "writfold-wire", "--bin", "writfold-ref"], output, "reference-build", env)
    binaries = [inspect_binary(target / TARGET / "release/examples/generation_smoke", output,
                               "generation_smoke", env),
                inspect_binary(target / TARGET / "release/writfold-ref", output, "writfold-ref", env)]
    smoke = []
    for index, (arguments, expected) in enumerate([
        (["40", "0", "39"], {"outcome": "allocated", "generation": 41}),
        (["40", "41"], {"outcome": "in_use"}),
        ([str(2**64 - 1), "0"], {"outcome": "exhausted"}),
    ]):
        observed = json.loads(execute([binaries[0]["path"], *arguments], output,
                                     f"generation-execution-{index}", {"PATH": "/nonexistent"}))
        if observed != expected:
            raise ValueError("Runtime generation transition differs from the verified contract")
        smoke.append(observed)
    base = {"schema_version": 2, "semantic_version": "2.0.0", "op": "fresh", "ttl": 1,
            "observed_at": 10**30}
    requests = [{**base, "id": "inclusive", "now": 10**30 + 1},
                {**base, "id": "expired", "now": 10**30 + 2},
                {**base, "id": "future", "now": 10**30 - 1}]
    response = execute([binaries[1]["path"]], output, "semantic-execution", {"PATH": "/nonexistent"},
                       stdin="".join(json.dumps(request) + "\n" for request in requests))
    actual = [json.loads(line) for line in response.splitlines()]
    expected = [{"id": ident, "ok": True, "result": result} for ident, result in
                (("inclusive", "fresh"), ("expired", "stale"), ("future", "unknown"))]
    if actual != expected:
        raise ValueError("Unbounded freshness behavior differs in the cold musl artifact")
    if source_identity() != before or manifest(vendor) != dependencies:
        raise ValueError("Source or offline dependency closure changed during qualification")
    (output / "source-inputs.json").write_text(json.dumps(before, indent=2) + "\n")
    (output / "vendor-inputs.json").write_text(json.dumps(dependencies, indent=2) + "\n")
    return {"status": "pass", "target": TARGET, "cold_cargo_home": str(home),
            "cold_target": str(target), "network_interfaces": ["lo"],
            "vendored_files": len(dependencies), "binaries": binaries, "generation_outcomes": smoke,
            "scope": "cold offline musl compilation and actual production-body/reference smoke; not whole-product installation or proof"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vendor", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("/evidence/cold-offline"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False, mode=0o755)
    try:
        result = qualify(args.vendor.resolve(), args.output)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        result = {"status": "fail", "error": str(error)}
    (args.output / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))
    return 0 if result["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
