#!/usr/bin/env python3
"""Every required theorem is in a tauten claim, or on a list that only shrinks (#124).

`proofs/Audit.lean`'s `claims` list names the required theorems (every
theorem a mutant names, and each kernel's headline statements). Each must
appear in some `[[claim]]` of `proofs/claims.toml`, or be listed in
`proofs/unclaimed.txt`, the committed backlog of #102. The check fails on a
required theorem in neither, and on a backlog line that is now claimed or
no longer required, so the backlog can only shrink. It reads three text
files and needs neither Docker nor Lean.

    python3 proofs/claims_coverage.py            check
    python3 proofs/claims_coverage.py --write    rewrite unclaimed.txt (only to shrink it)
"""
import re
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
BACKLOG = HERE / "unclaimed.txt"


def required() -> list[str]:
    text = (HERE / "Audit.lean").read_text()
    start = text.index("def claims : List Name := [")
    end = text.index("\n]", start)
    # The calculus (proofs/Calculus, #205) is about no Rust function, so no
    # tauten claim can hold it; mutants.py requires each of its claims to be
    # named by a core mutant or listed as derived instead.
    found = re.findall(r"``([\w.']+)", text[start:end])
    return [name for name in found if not name.startswith("Calculus.")]


def claimed() -> set[str]:
    plan = tomllib.loads((HERE / "claims.toml").read_text())
    return {t for c in plan.get("claim", []) for t in c.get("theorems", [])}


def backlog() -> list[str]:
    if not BACKLOG.exists():
        return []
    lines = (l.strip() for l in BACKLOG.read_text().splitlines())
    return [l for l in lines if l and not l.startswith("#")]


def main() -> int:
    need, have, listed = required(), claimed(), backlog()
    missing = sorted({t for t in need if t not in have} - set(listed))
    spent = sorted(t for t in listed if t in have)
    gone = sorted(t for t in listed if t not in need)
    if "--write" in sys.argv:
        if missing:
            print("claims_coverage: refusing to grow the backlog:", *missing, sep="\n  ")
            return 1
        keep = [t for t in listed if t not in have and t in need]
        head = "# Required theorems (proofs/Audit.lean) not yet in a tauten claim (#102).\n# This list only shrinks: proofs/claims_coverage.py fails on a new entry or a spent one.\n"
        BACKLOG.write_text(head + "".join(f"{t}\n" for t in keep))
        print(f"claims_coverage: wrote {len(keep)} entries")
        return 0
    faults = [f"required theorem in no claim and not in unclaimed.txt: {t}" for t in missing]
    faults += [f"unclaimed.txt lists a claimed theorem (remove the line): {t}" for t in spent]
    faults += [f"unclaimed.txt lists a theorem Audit.lean no longer requires: {t}" for t in gone]
    for f in faults:
        print(f"claims_coverage: {f}")
    if faults:
        return 1
    print(f"CLAIMS_COVERAGE ok: {len(need)} required, {len(need) - len(listed)} claimed, {len(listed)} in the backlog")
    return 0


if __name__ == "__main__":
    sys.exit(main())
