#!/usr/bin/env python3
"""The unit gate's test run (#93): the workspace tests, built once and run
several test executables at a time.

`cargo test --workspace` builds every test executable and then runs them one
after another; the gate's time went to that serial run (174 executables,
about 1050 of 1115 seconds). This builds the same executables with
`cargo test --no-run`, runs them WRITFOLD_TEST_JOBS at a time (default 4),
each in its package directory with the variables cargo sets, then runs the
doctests as `cargo test --doc` does. Each executable's output is printed
whole, in build order, so the `test result:` lines the gate counts are the
ones `cargo test` prints. Each runs with WRITFOLD_REQUIRE_VALE=1, so the
real-Vale conformance test fails rather than skips when the pinned Vale is
missing. The exit status is 101 when any test failed, as
cargo's is; a failed build exits with cargo's status.

Extra arguments are passed to cargo (the gate passes none).
"""
from __future__ import annotations

import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import sys

CARGO = os.environ.get("CARGO", "cargo")
BASE = ["--workspace", "--locked", "--offline"]


def executables(extra: list[str]) -> list[tuple[str, Path, Path]]:
    """(name, executable, package directory) of every test target, in order."""
    build = subprocess.run(
        [CARGO, "test", *BASE, *extra, "--no-run", "--message-format=json-render-diagnostics"],
        stdout=subprocess.PIPE, text=True, check=False)
    if build.returncode != 0:
        raise SystemExit(build.returncode)
    found = []
    for line in build.stdout.splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") != "compiler-artifact" or not record.get("profile", {}).get("test"):
            continue
        if record.get("executable"):
            target = record["target"]
            name = f"{record['package_id'].split('#')[0].rsplit('/', 1)[-1]}::{target['name']} ({target['kind'][0]})"
            found.append((name, Path(record["executable"]), Path(record["manifest_path"]).parent))
    if not found:
        raise SystemExit("no test executables were built")
    return found


def run(entry: tuple[str, Path, Path]) -> tuple[str, int, str]:
    name, executable, package = entry
    # The tools image carries the pinned Vale, so its conformance test must
    # run rather than skip (#133).
    env = dict(os.environ, CARGO_MANIFEST_DIR=str(package), WRITFOLD_REQUIRE_VALE="1")
    result = subprocess.run([str(executable)], cwd=package, env=env, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, errors="replace", check=False)
    return name, result.returncode, result.stdout


def main() -> int:
    extra = sys.argv[1:]
    found = executables(extra)
    jobs = max(1, int(os.environ.get("WRITFOLD_TEST_JOBS", "4") or "4"))
    failed = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        for name, code, output in pool.map(run, found):
            print(f"     Running {name}", flush=True)
            sys.stdout.write(output)
            sys.stdout.flush()
            if code != 0:
                failed.append(name)
    doc = subprocess.run([CARGO, "test", *BASE, *extra, "--doc"], check=False)
    if doc.returncode != 0:
        failed.append("doctests")
    if failed:
        print(f"error: {len(failed)} test target(s) failed: {', '.join(failed)}", file=sys.stderr)
        return 101
    print(f"unit tests: {len(found)} executables and the doctests passed ({jobs} at a time)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
