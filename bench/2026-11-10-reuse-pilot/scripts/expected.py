#!/usr/bin/env python3
"""Extract the hidden expected answers from a checkout at the corpus commit:
`session1` is the complete evidence of each session-1 task (used only to check
what session 1 retained and noted), `answers` the session-2 answers graded.
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


def at(path, line):
    return {"path": path, "line": line, "text": source(path)[line - 1]}


def key(r):
    return (r["path"].encode(), r["line"])


s1 = {}
s1["R1"] = [r for r in grep(["-F", "claim_matches_spec(", "--", "crates/"]) if "fn claim_matches_spec" not in r["text"]]
s1["R2"] = grep(["-E", r"pub enum [A-Za-z0-9_]*Error\b", "--", "crates/"])
r3 = []
for r in grep(["-F", "decode_pin(", "--", "crates/"]):
    if re.search(r"\bfn decode_pin\b", r["text"]):
        continue
    rest = r["text"].split("decode_pin(", 1)[1].strip()
    same = bool(rest)
    if not rest:
        rest = source(r["path"])[r["line"]].strip()
    if rest.startswith("&changed("):
        r3.append((r, same))
s1["R3"] = [r for r, _ in r3]
first = {}
for r in grep(["-F", "serde_json::", "--", "crates/"]):
    first.setdefault(r["path"], r)
s1["R4"] = list(first.values())
s1["R5"] = grep(["-F", "#[test]", "--", "crates/writfold-app/src/trial/"])
s1["R6"] = grep(["-F", "MAX_RESPONSE_BYTES", "--", "crates/"])

ans = {}
ans["R1"] = [r for r in s1["R1"] if r["path"].startswith("crates/writfold-kernel/src/checker/")]
ans["R2"] = [at(r["path"], r["line"] + 1) for r in s1["R2"] if r["path"].startswith("crates/writfold-runtime/")]
ans["R3"] = [r if same else at(r["path"], r["line"] + 1) for r, same in r3]
ans["R4"] = [r for r in s1["R4"] if "serde_json::Value" in r["text"]]
ans["R5"] = [at(r["path"], r["line"] + 1) for r in s1["R5"]
             if r["path"].rsplit("/", 1)[1] in ("retain_tests.rs", "recovery_tests.rs")]
ans["R6"] = [r for r in s1["R6"] if "/tests/" not in r["path"] and not r["path"].endswith("_tests.rs")]
print(json.dumps({"commit": COMMIT,
                  "session1": {k: sorted(v, key=key) for k, v in s1.items()},
                  "answers": {k: sorted(v, key=key) for k, v in ans.items()}}, indent=1, ensure_ascii=False))
