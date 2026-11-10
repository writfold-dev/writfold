#!/usr/bin/env python3
"""Apply the pre-registered selection rule to the calibration grades.

A candidate qualifies if (a) arm A was clean in at most 75% of its
calibration trials (with 2 repeats: 0 or 1 of 2) and (b) its reference
solution (reference.py) is graded clean. If fewer than 6 qualify: stop. If
6-8 qualify: all go to the main stage. If more than 8 qualify, choose 8:
sort the qualifying tasks by (arm-A clean count ascending, task id); take the
first task of each family in that order (family coverage), then fill the
remaining slots from the same sorted list.

Usage: select.py CALIBRATION_GRADES REFERENCE_GRADES > selection.json"""
import json, sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from tasks import TASKS  # noqa: E402

cal = json.load(open(sys.argv[1]))
ref = {g["task"]: g["clean"] for g in json.load(open(sys.argv[2]))}
rows = []
for t in sorted(TASKS):
    xs = [g for g in cal if g["task"] == t and g["arm"] == "A"]
    clean = sum(bool(g["clean"]) for g in xs)
    rows.append({"task": t, "family": TASKS[t]["family"], "trials": len(xs), "clean": clean,
                 "clean_rate": clean / len(xs) if xs else None,
                 "hard_enough": bool(xs) and clean / len(xs) <= 0.75, "reference_clean": ref.get(t, False)})
q = sorted((r for r in rows if r["hard_enough"] and r["reference_clean"]), key=lambda r: (r["clean"], r["task"]))
if len(q) < 6:
    chosen, decision = [], f"stop: only {len(q)} tasks qualified (fewer than 6); no main stage"
elif len(q) <= 8:
    chosen, decision = [r["task"] for r in q], f"{len(q)} qualified; all go to the main stage"
else:
    chosen, fams = [], set()
    for r in q:
        if r["family"] not in fams:
            chosen.append(r["task"])
            fams.add(r["family"])
    for r in q:
        if len(chosen) == 8:
            break
        if r["task"] not in chosen:
            chosen.append(r["task"])
    decision = f"{len(q)} qualified; 8 chosen by the pre-registered rule"
print(json.dumps({"rule": __doc__.split("Usage")[0].strip(), "candidates": rows,
                  "qualified": [r["task"] for r in q], "chosen": sorted(chosen), "decision": decision}, indent=1))
