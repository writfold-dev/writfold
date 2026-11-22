#!/usr/bin/env python3
"""Plan clone coverage by the evidence needed, not by cloning every cited
repository (docs/guide/validation.md, "Which clones to provision"; #89).

    clone-coverage.py inventory              what is cited, and its state here
    clone-coverage.py shortlist              the order to provision in
    clone-coverage.py provision APPROVED --into DIR [--depth 1]
    clone-coverage.py verdicts [--save FILE] citation verdicts by status and reason
    clone-coverage.py compare BEFORE AFTER   two saved `verdicts`

Run it at a KB root with `writfold` on PATH. `inventory`, `shortlist` and
`verdicts` make no network request and no Git write; a `url=` in KB text is
shown for review and never fetched. Only `provision` clones, and only the
origins in APPROVED (one `<repo> <origin>` per line, written by the
operator), into DIR, which it then names in the KB's local repository map
(`<git dir>/writfold/repos`). It reports the storage the clones take on
disk. It never updates or prunes an existing clone.

The shortlist order: repositories named by gate citations first (a gate
needs that evidence), then by descending token count. A token count ranks
candidates; it does not predict how many citations will read fresh:
cloning turns `repo-not-local` into a real verdict, which may be `changed`.
Stop where the storage or time budget is reached; there is no fixed number.
"""
from __future__ import annotations

import argparse
import collections
import json
import os
from pathlib import Path
import subprocess
import sys


def answer(*args: str) -> tuple[list[dict], dict]:
    """Every row of a `writfold kb … --json` answer, following the cursor,
    and the counts of its last trailer."""
    rows, cursor = [], None
    while True:
        extra = ["--cursor", cursor] if cursor else []
        done = subprocess.run(["writfold", "kb", *args, "--json", *extra],
                              stdout=subprocess.PIPE, text=True, check=False)
        if done.returncode not in (0, 2):
            raise SystemExit(f"writfold kb {' '.join(args)} exited {done.returncode}")
        page = json.loads(done.stdout)
        rows.extend(page["rows"])
        cursor = page["trailer"].get("cursor")
        if not cursor:
            return rows, page["trailer"].get("counts", {})


def writfold(*args: str) -> list[dict]:
    return answer(*args)[0]


def more(row: dict, label: str) -> str:
    return next((m["text"] for m in row.get("more", []) if m["label"] == label), "")


def inventory() -> list[dict]:
    out = []
    for row in writfold("repos"):
        out.append({
            "repo": row["id"],
            "status": row.get("status") or "",
            "tokens": int((row.get("text") or "tokens=0").split("=", 1)[1]),
            "url": more(row, "url"),
            "store": "store" in row.get("flags", []),
            "shallow": more(row, "history") == "shallow",
        })
    return out


def gate_repos() -> set[str]:
    """Repositories named by gate citations (`gate:<repo>@<sha>`)."""
    found = set()
    for row in writfold("stale"):
        if row["id"].startswith("gate:"):
            found.add(row["id"][5:].split("@", 1)[0])
    return found


def cmd_inventory() -> None:
    rows = inventory()
    print(f"{'repo':<16} {'status':<9} {'tokens':>6}  url (shown for review, not fetched)")
    for r in rows:
        kind = " store" if r["store"] else ""
        shallow = " shallow" if r["shallow"] else ""
        print(f"{r['repo']:<16} {r['status']:<9} {r['tokens']:>6}  {r['url'] or '-'}{kind}{shallow}")
    counts = collections.Counter(r["status"] for r in rows)
    print(f"# repos={len(rows)} " + " ".join(f"{k}={v}" for k, v in sorted(counts.items())))
    if counts.get("partial"):
        print("# a partial clone is not `missing`: `kb repos --missing` does not list it")


def cmd_shortlist() -> None:
    gates = gate_repos()
    rows = [r for r in inventory() if r["status"] in ("missing", "partial") and not r["store"]]
    rows.sort(key=lambda r: (r["repo"] not in gates, -r["tokens"], r["repo"]))
    print(f"{'#':>2} {'repo':<16} {'status':<9} {'tokens':>6}  why")
    for n, r in enumerate(rows, 1):
        why = "named by a gate citation" if r["repo"] in gates else "token count"
        print(f"{n:>2} {r['repo']:<16} {r['status']:<9} {r['tokens']:>6}  {why}")
    print("# review this list; write the approved `<repo> <origin>` lines yourself")


def size_kb(path: Path) -> int:
    return int(subprocess.run(["du", "-sk", str(path)], stdout=subprocess.PIPE,
                              text=True, check=True).stdout.split()[0])


def cmd_provision(approved: str, into: str, depth: int | None) -> None:
    target = Path(into).resolve()
    target.mkdir(parents=True, exist_ok=True)
    git_dir = Path(subprocess.run(["git", "rev-parse", "--absolute-git-dir"], stdout=subprocess.PIPE,
                                  text=True, check=True).stdout.strip())
    repo_map = git_dir / "writfold" / "repos"
    mapped = repo_map.read_text() if repo_map.exists() else "# local repository map\n"
    total = 0
    for line in Path(approved).read_text().splitlines():
        parts = line.split()
        if len(parts) != 2 or parts[0].startswith("#"):
            continue
        repo, origin = parts
        dest = target / f"{repo}.git"
        if dest.exists():
            print(f"{repo}: {dest} exists; left as it is")
        else:
            shallow = ["--depth", str(depth)] if depth else []
            subprocess.run(["git", "clone", "-q", "--mirror", *shallow, origin, str(dest)], check=True)
        if not any(l.split()[:1] == [repo] for l in mapped.splitlines()):
            mapped += f"{repo} {dest}\n"
        size = size_kb(dest)
        total += size
        print(f"{repo}: {dest} {size} KiB" + (f" (depth {depth})" if depth else ""))
    repo_map.parent.mkdir(parents=True, exist_ok=True)
    repo_map.write_text(mapped)
    print(f"# provisioned {total} KiB on disk; named in {repo_map}")


def verdicts() -> dict[str, int]:
    """Citations by verdict and, for unverifiable, reason. `kb stale` prints
    no row for a fresh citation: fresh is the checked count less the rows."""
    counts: collections.Counter[str] = collections.Counter()
    rows, trailer = answer("stale")
    for row in rows:
        if row.get("kind") != "cite":
            continue
        status = row.get("status") or ""
        parts = (row.get("text") or "").split(" ", 2)
        reason = parts[2].split(":", 1)[0].split(" ", 1)[0] if status == "unverifiable" and len(parts) > 2 else ""
        counts[f"{status} {reason}".strip()] += 1
    fresh = trailer.get("checked", 0) - sum(counts.values())
    if fresh > 0:
        counts["fresh"] = fresh
    return dict(counts)


def show(counts: dict[str, int]) -> None:
    for key in sorted(counts):
        print(f"{counts[key]:>6}  {key}")


def cmd_compare(before: str, after: str) -> None:
    b, a = json.loads(Path(before).read_text()), json.loads(Path(after).read_text())
    print(f"{'before':>6} {'after':>6}  verdict and reason")
    for key in sorted(set(a) | set(b)):
        print(f"{b.get(key, 0):>6} {a.get(key, 0):>6}  {key}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="step", required=True)
    sub.add_parser("inventory")
    sub.add_parser("shortlist")
    p = sub.add_parser("provision")
    p.add_argument("approved")
    p.add_argument("--into", required=True)
    p.add_argument("--depth", type=int)
    v = sub.add_parser("verdicts")
    v.add_argument("--save")
    c = sub.add_parser("compare")
    c.add_argument("before")
    c.add_argument("after")
    args = parser.parse_args()
    if args.step == "inventory":
        cmd_inventory()
    elif args.step == "shortlist":
        cmd_shortlist()
    elif args.step == "provision":
        cmd_provision(args.approved, args.into, args.depth)
    elif args.step == "verdicts":
        counts = verdicts()
        show(counts)
        if args.save:
            Path(args.save).write_text(json.dumps(counts, sort_keys=True) + "\n")
    else:
        cmd_compare(args.before, args.after)
    return 0


if __name__ == "__main__":
    os.environ.setdefault("GIT_TERMINAL_PROMPT", "0")
    sys.exit(main())
