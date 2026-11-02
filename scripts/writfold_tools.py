#!/usr/bin/env python3
"""Observe the provisioned tool closure; observation is not product qualification."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command(argv: list[str], *, expected_exit: int = 0) -> str:
    run = subprocess.run(argv, capture_output=True, text=True, check=False, timeout=60)
    output = (run.stdout + run.stderr).strip()
    if run.returncode != expected_exit:
        raise ValueError(f"Tool observation {argv!r} exited {run.returncode}: {output}")
    return output


def observe(name: str, arguments: list[str], *, rustup: bool = False) -> dict:
    if rustup:
        executable = Path(command(["rustup", "which", arguments[0]]))
    else:
        found = shutil.which(arguments[0])
        if found is None:
            raise ValueError(f"Missing required tool: {name}")
        executable = Path(found).resolve()
    return {"name": name, "executable": str(executable), "sha256": sha(executable),
            "argv": arguments, "version_output": command(arguments)}


def main() -> None:
    tools = [
        observe("rustc", ["rustc", "--version", "--verbose"], rustup=True),
        observe("cargo", ["cargo", "--version", "--verbose"], rustup=True),
        observe("rustfmt", ["rustfmt", "--version"], rustup=True),
        observe("clippy", ["clippy-driver", "--version"], rustup=True),
        observe("rustup", ["rustup", "--version"]),
        observe("verus", ["verus", "--version"]),
        observe("z3", [os.environ["VERUS_Z3_PATH"], "--version"]),
        observe("lean", ["lean", "--version"]),
        observe("lake", ["lake", "--version"]),
        observe("tlaps", ["tlapm", "--version"]),
        observe("java", ["java", "-version"]),
        observe("dolt", ["dolt", "version"]),
        observe("git", ["git", "--version"]),
        observe("python", ["python3", "--version"]),
        observe("uv", ["uv", "--version"]),
        observe("musl-gcc", ["musl-gcc", "--version"]),
    ]
    jar = Path("/opt/tla/tla2tools.jar")
    tools.append({"name": "tlc", "artifact": str(jar), "sha256": sha(jar),
                  "version_command_exit": 1,
                  "version_output": command(["java", "-cp", str(jar), "tlc2.TLC", "-help"],
                                            expected_exit=1)})
    packages = Path("/opt/writfold-toolchain/distro-packages.tsv")
    result = {"schema_version": 1, "scope": "observed immutable tool image; no product gate claim",
              "target": "x86_64-unknown-linux-musl", "tools": tools,
              "rust_targets": command(["rustup", "target", "list", "--installed"]).splitlines(),
              "rust_components": command(["rustup", "component", "list", "--installed"]).splitlines(),
              "distro_packages_sha256": sha(packages),
              "distro_packages": packages.read_text().splitlines()}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
