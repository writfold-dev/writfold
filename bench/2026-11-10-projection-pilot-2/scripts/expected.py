#!/usr/bin/env python3
"""Extract the hidden expected answers from a checkout at the pinned commit.
Usage: expected.py REPO > expected.json"""
import json, re, subprocess, sys
repo = sys.argv[1]
COMMIT = "c568505742d00859cad758924c98d8d0a6e98648"
assert subprocess.check_output(["git", "-C", repo, "rev-parse", "HEAD"], text=True).strip() == COMMIT


def grep(args):
    out = subprocess.run(["git", "-C", repo, "grep", "-n", "--no-color", *args],
                         capture_output=True, check=True).stdout.decode()
    rows = []
    for l in out.splitlines():
        p, n, t = l.split(":", 2)
        rows.append({"path": p, "line": int(n), "text": t})
    return rows


def source(path):
    return open(f"{repo}/{path}", encoding="utf-8").read().split("\n")


def key(r):
    return (r["path"].encode(), r["line"])


ans = {}
# P1: every call line of claim_matches_spec( under crates/, not the definition.
ans["P1"] = [r for r in grep(["-F", "claim_matches_spec(", "--", "crates/"]) if "fn claim_matches_spec" not in r["text"]]
# P2: first line per file under crates/ containing serde_json::
first = {}
for r in grep(["-F", "serde_json::", "--", "crates/"]):
    first.setdefault(r["path"], r)
ans["P2"] = list(first.values())
# P3: decode_pin calls whose first argument is &changed(...), reporting the decode_pin( line.
p3 = []
for r in grep(["-F", "decode_pin(", "--", "crates/"]):
    if re.search(r"\bfn decode_pin\b", r["text"]):
        continue
    rest = r["text"].split("decode_pin(", 1)[1].strip()
    if not rest:
        lines = source(r["path"])
        rest = lines[r["line"]].strip()  # next line (0-based index == 1-based line)
    if rest.startswith("&changed("):
        p3.append(r)
ans["P3"] = p3
# P4: the last six .to_owned() lines under crates/ in (path bytes, line) order.
ans["P4"] = sorted(grep(["-F", ".to_owned()", "--", "crates/"]), key=key)[-6:]
# P5: every pub enum under crates/ whose name ends in Error.
ans["P5"] = grep(["-E", r"pub enum [A-Za-z0-9_]*Error\b", "--", "crates/"])
# P6 (control): the definition of PIN_BYTES.
ans["P6"] = grep(["-E", r"const PIN_BYTES\b"])
counts = {
    "P1": len(grep(["-F", "claim_matches_spec(", "--", "crates/"])),
    "P2": len(grep(["-F", "serde_json::", "--", "crates/"])),
    "P3": len(grep(["-F", "decode_pin(", "--", "crates/"])),
    "P4": len(grep(["-F", ".to_owned()", "--", "crates/"])),
    "P5": len(grep(["-E", r"pub enum [A-Za-z0-9_]*Error\b", "--", "crates/"])),
    "P6": len(grep(["-F", "PIN_BYTES"])),
}
print(json.dumps({"commit": COMMIT, "matching_lines_of_the_obvious_search": counts,
                  "answers": {k: sorted(v, key=key) for k, v in ans.items()}}, indent=1, ensure_ascii=False))
