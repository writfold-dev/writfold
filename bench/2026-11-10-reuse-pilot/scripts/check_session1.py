#!/usr/bin/env python3
"""After session 1, before session 2: how much of each task's complete evidence
session 1 retained (arm B's material) and wrote into notes.txt (arm A's).
Session 1 is not rerun; gaps are recorded and shared by both arms.
Usage: check_session1.py EXPECTED_JSON SCRATCH -> JSON on stdout"""
import json, pathlib, re, subprocess, sys
expected = json.loads(pathlib.Path(sys.argv[1]).read_text())["session1"]
S = pathlib.Path(sys.argv[2])
ROW = re.compile(r"^(?:\./)?([^\s:][^:]*):(\d+):\s?(.*)$")
LISTED = re.compile(r"^[0-9a-f]{12} (.+)@[0-9a-f]{7}:(\d+): ")
out = {}
for task, rows in expected.items():
    d = S / "s1" / task
    want = {(r["path"], r["line"]) for r in rows}
    listed = subprocess.run([str(S / "bin/writfold"), "trial", "evidence", "list", "--state", str(d / "state"),
                             "--limit", "100000"], capture_output=True, text=True, check=True).stdout.splitlines()
    retained = {(m.group(1), int(m.group(2))) for m in map(LISTED.match, listed) if m}
    noted = set()
    notes = d / "notes.txt"
    if notes.exists():
        for line in notes.read_text(errors="replace").splitlines():
            m = ROW.match(line.strip())
            if m:
                noted.add((m.group(1), int(m.group(2))))
    out[task] = {"expected": len(want), "retained_rows": len(retained), "retained_expected": len(want & retained),
                 "notes_present": notes.exists(), "noted_rows": len(noted), "noted_expected": len(want & noted),
                 "trailer": listed[-1] if listed else None}
print(json.dumps(out, indent=1))
