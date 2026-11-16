#!/usr/bin/env python3
"""Explicit developer-only download of pinned, SHA-256 checked semantic assets.

Run from any directory:
  python3 scripts/semantic-model.py --output target/semantic-model

The default destination is target/semantic-model under this script's worktree.
Writfold itself and Cargo builds never invoke this script or download assets.
The pinned model card records the model's MIT license and training provenance.
"""

import argparse
import hashlib
import os
from pathlib import Path
import tempfile
import urllib.request

PIN = "bf8b056651a2c21b8d2565580b8569da283cab23"
BASE = f"https://huggingface.co/minishlab/potion-base-8M/resolve/{PIN}/"
ASSETS = {
    "tokenizer.json": "e67e803f624fb4d67dea1c730d06e1067e1b14d830e2c2202569e3ef0f70bb50",
    "model.safetensors": "f65d0f325faadc1e121c319e2faa41170d3fa07d8c89abd48ca5358d9a223de2",
    "config.json": "2a6ac0e9aaa356a68a5688070db78fc3a464fefe85d2f06a1905ce3718687553",
    "README.md": "de8ec91bf63c5f4c0e20751c227b2d049953e1cab5f8d5d44211c59a44795bdd",
}


def checksum(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def fetch(directory, name, expected):
    destination = directory / name
    if destination.is_file() and checksum(destination) == expected:
        print(f"{expected}  {destination} (already verified)")
        return
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=directory, delete=False) as output:
            temporary = Path(output.name)
            digest = hashlib.sha256()
            with urllib.request.urlopen(BASE + name, timeout=120) as source:
                for block in iter(lambda: source.read(1024 * 1024), b""):
                    output.write(block)
                    digest.update(block)
        actual = digest.hexdigest()
        if actual != expected:
            raise ValueError(f"{name}: expected SHA-256 {expected}, received {actual}")
        os.replace(temporary, destination)
        temporary = None
        print(f"{expected}  {destination}")
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "target/semantic-model",
        help="destination directory (default: this worktree's target/semantic-model)",
    )
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for name, expected in ASSETS.items():
        fetch(args.output, name, expected)


if __name__ == "__main__":
    main()
