#!/usr/bin/env python3
"""Positive control: every expected line appears, with its exact text, among the
rows a reference program writes with --out. Usage: check_reference.py EXPECTED OUT_DIR"""
import json, re, sys
d = json.load(open(sys.argv[1]))["answers"]
ok = True
for t, exp in d.items():
    rows = {}
    for l in open(f"{sys.argv[2]}/{t}.out"):
        m = re.match(r"^([^:]+):(\d+): ?(.*)$", l.rstrip("\n"))
        if m:
            rows[(m[1], int(m[2]))] = m[3].strip()
    miss = [e for e in exp if rows.get((e["path"], e["line"])) != e["text"].strip()]
    ok &= not miss
    print(t, "expected", len(exp), "rows", len(rows), "missing_or_wrong", len(miss))
sys.exit(0 if ok else 1)
