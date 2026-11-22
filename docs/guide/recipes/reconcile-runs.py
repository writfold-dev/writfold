#!/usr/bin/env python3
"""Reconcile Writfold's run inventory by ID (docs/guide/validation.md,
"Reconciling run counts"; #97).

    reconcile-runs.py [--commit REV] [--other FILE]

Run it at a KB root, with `writfold` on PATH. It reads; it writes nothing.

1. Lists every run `writfold kb runs --json` gives, following the cursor to
   the end (a first page is not the inventory).
2. Lists, from Git at REV (default HEAD), every directory under `runs/` and
   every `manifest.json` in the tree.
3. Prints what one side has and the other does not, and why. That covers
   the causes a reader of `kb runs` alone is not told about: a run directory
   without `manifest.json`, a manifest outside `runs/<id>/`, and a state
   value other than `open` or `closed`.
4. With `--other FILE` (one `<id> <state>` per line, from the tool being
   replaced), it prints the IDs only one tool knows and the IDs whose
   state differs.

Exit 0 when Writfold's list and Git's tree agree (and match FILE, when
given), else 1.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys


def sh(*args: str) -> str:
    """Run a command; `writfold` exits 2 for a partial answer, still an answer."""
    done = subprocess.run(args, check=False, stdout=subprocess.PIPE, text=True)
    allowed = (0, 2) if args[0] == "writfold" else (0,)
    if done.returncode not in allowed:
        raise SystemExit(f"{' '.join(args)} exited {done.returncode}")
    return done.stdout


def writfold_runs() -> tuple[dict[str, tuple[str, str]], dict, int]:
    """ID to (state, manifest path), following the cursor; the last
    trailer; the number of pages read."""
    runs: dict[str, tuple[str, str]] = {}
    cursor = None
    pages = 0
    while True:
        pages += 1
        extra = ["--cursor", cursor] if cursor else []
        answer = json.loads(sh("writfold", "kb", "runs", "--json", *extra))
        for row in answer["rows"]:
            runs[row["id"]] = (row.get("status") or "", row.get("path") or "")
        trailer = answer["trailer"]
        cursor = trailer.get("cursor")
        if not cursor:
            return runs, trailer, pages


def git_runs(commit: str) -> tuple[set[str], dict[str, str], list[str]]:
    """Directories under runs/, manifests at runs/<id>/ (path to text), and
    manifests anywhere else in the tree."""
    files = sh("git", "ls-tree", "-r", "--name-only", commit).splitlines()
    dirs = {f.split("/")[1] for f in files if f.startswith("runs/") and f.count("/") >= 2}
    at_root, elsewhere = {}, []
    for f in files:
        if not f.endswith("/manifest.json") and f != "manifest.json":
            continue
        parts = f.split("/")
        if len(parts) == 3 and parts[0] == "runs":
            at_root[f] = sh("git", "show", f"{commit}:{f}")
        else:
            elsewhere.append(f)
    return dirs, at_root, sorted(elsewhere)


def other_runs(path: str) -> dict[str, str]:
    out = {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if parts and not parts[0].startswith("#"):
                out[parts[0]] = parts[1] if len(parts) > 1 else ""
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--commit", default="HEAD")
    parser.add_argument("--other")
    args = parser.parse_args()
    listed, trailer, pages = writfold_runs()
    dirs, manifests, elsewhere = git_runs(args.commit)
    head = sh("git", "rev-parse", "--short=12", args.commit).strip()
    findings = 0

    def report(title: str, items: list[str], why: str) -> None:
        nonlocal findings
        if items:
            findings += len(items)
            print(f"{title} ({len(items)})" + (f": {why}" if why else ""))
            for item in items:
                print(f"  {item}")

    print(f"writfold lists {len(listed)} run(s) in {pages} page(s); Git at {head} has {len(dirs)} run "
          f"director(ies) and {len(manifests)} manifest(s) at runs/<id>/")
    print(f"  trailer: kb={trailer['kb']} coverage={trailer['coverage']} "
          f"unreadable={trailer.get('unreadable', 0)} omitted={trailer['omitted']}")
    if not trailer["kb"].startswith(head[:12]) or "+dirty" in trailer["kb"]:
        print(f"  note: writfold read {trailer['kb']}, not {head}: compare at the same commit, clean")
    with_manifest = {m.split("/")[1] for m in manifests}
    report("directories without manifest.json", sorted(dirs - with_manifest),
           "not a run to writfold, and not reported by `kb runs`")
    report("manifest.json outside runs/<id>/", elsewhere,
           "only runs/<id>/manifest.json is a run")
    listed_paths = {path for _, path in listed.values()}
    unlisted = sorted(m for m in manifests if m not in listed_paths)
    report("manifests writfold does not list", unlisted,
           "unreadable or a duplicate run_id: `writfold kb index status` names each reason")
    ids: dict[str, list[str]] = {}
    for path, text in manifests.items():
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            continue
        run_id = value.get("run_id") if isinstance(value, dict) else None
        ids.setdefault(run_id or path.split("/")[1], []).append(path)
    report("duplicate run_id", [f"{i}: {', '.join(p)}" for i, p in sorted(ids.items()) if len(p) > 1],
           "the first path wins; the rest are unreadable")
    odd = sorted(f"{i} [{s}]" for i, (s, _) in listed.items() if s not in ("open", "closed"))
    report("state other than open or closed", odd,
           "shown as written; `kb runs --open` keeps every state but closed")
    if args.other:
        theirs = other_runs(args.other)
        report("only in writfold", sorted(set(listed) - set(theirs)), "")
        report("only in the other tool", sorted(set(theirs) - set(listed)), "")
        report("state differs", sorted(
            f"{i}: writfold {listed[i][0]}, other {theirs[i]}"
            for i in set(listed) & set(theirs) if theirs[i] and theirs[i] != listed[i][0]), "")
    if not findings:
        print("agree: every run directory is a listed run")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
