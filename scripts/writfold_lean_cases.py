#!/usr/bin/env python3
"""Rebuild a Lean-exported case corpus and compare it with its Rust fixture.

Builds `procedurecases` or `ledgercases` from a scratch copy of formal/ inside
the admitted tool image (network disabled), then requires the exported JSON to
equal the committed fixture under crates/writfold-wire/tests/fixtures/ byte for byte.
`--write` replaces the fixture instead. This checks fixture freshness only; it
is not a Lean admission, refinement proof or gate receipt.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CORPORA = {
    "procedure": ("procedurecases", "procedure-lean-cases.json"),
    "ledger": ("ledgercases", "ledger-lean-cases.json"),
}
FIXTURES = ROOT / "crates/writfold-wire/tests/fixtures"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", choices=sorted(CORPORA), default="procedure")
    parser.add_argument("--write", action="store_true", help="Replace the committed fixture")
    args = parser.parse_args()
    executable, name = CORPORA[args.corpus]
    fixture = FIXTURES / name
    image = json.loads((ROOT / "toolchains.lock.json").read_text())["image"]["reference"]
    with tempfile.TemporaryDirectory(prefix="writfold-procedure-cases-") as scratch:
        formal = Path(scratch) / "formal"
        shutil.copytree(ROOT / "formal", formal, ignore=shutil.ignore_patterns(".lake"))
        command = ["docker", "run", "--rm", "--network", "none", "-u", f"{os.getuid()}:{os.getgid()}",
                   "-e", "HOME=/tmp", "-v", f"{formal}:/w", "-w", "/w", image, "sh", "-c",
                   f"lake build {executable} >&2 && ./.lake/build/bin/{executable}"]
        result = subprocess.run(command, capture_output=True, check=False)
        if result.returncode != 0:
            sys.stderr.write(result.stderr.decode(errors="replace"))
            return result.returncode
    exported = result.stdout
    digest = hashlib.sha256(exported).hexdigest()
    if args.write:
        fixture.write_bytes(exported)
        print(json.dumps({"written": str(fixture), "sha256": digest, "image": image}))
        return 0
    fresh = fixture.read_bytes() == exported
    print(json.dumps({"corpus": args.corpus, "fresh": fresh, "sha256": digest, "image": image}))
    return 0 if fresh else 1


if __name__ == "__main__":
    raise SystemExit(main())
