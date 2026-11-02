#!/usr/bin/env python3
"""Admit the existing operator binding before any expensive Docker work."""
from __future__ import annotations

import argparse
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def check() -> dict:
    infrastructure = Path(os.environ.get("WRITFOLD_CI_INFRA", str(ROOT.parent / "ci-infra"))).resolve()
    state = Path(os.environ.get("WRITFOLD_CI_STATE", str(Path.home() / ".local/share/buildbot-ci"))).resolve()
    if not (infrastructure / "ci_infra/storage.py").is_file():
        raise ValueError("Existing ci_infra storage admission is unavailable; set WRITFOLD_CI_INFRA")
    if not (state / "storage.json").is_file():
        raise ValueError("No operator storage binding; refusing an unadmitted Docker store")
    sys.path.insert(0, str(infrastructure))
    storage = importlib.import_module("ci_infra.storage")
    admitted = storage.check(state)
    if admitted is None:
        raise ValueError("Storage admission returned no binding")
    info = subprocess.run(
        ["docker", "info", "--format", '{"cpus":{{.NCPU}},"memory_bytes":{{.MemTotal}}}'],
        capture_output=True, text=True, check=True, timeout=30,
    )
    available = json.loads(info.stdout)
    resources = json.loads((ROOT / "qualification/resources.json").read_text())["allocation"]
    if available["cpus"] < resources["cpus"] or available["memory_bytes"] < resources["memory_bytes"]:
        raise ValueError("Configured qualification allocation exceeds available Docker resources")
    return {"schema_version": 1, "storage": admitted, "resources": resources,
            "available": available, "binding": str(state / "storage.json"),
            "admission_source": str(infrastructure / "ci_infra/storage.py")}


def main() -> int:
    argparse.ArgumentParser(description=__doc__).parse_args()
    try:
        print(json.dumps(check(), indent=2, sort_keys=True))
        return 0
    except (ValueError, OSError, ImportError, subprocess.SubprocessError) as error:
        print(f"Writfold preflight refused: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
