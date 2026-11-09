#!/usr/bin/env python3
"""Grade pilot 2. Arm A: the checkout must equal the expected tree.
Arm W: the checkout must be byte-identical to the fixture, the run's worktree
must hold the expected bytes, and `writfold trial show` must validate a history
ending in `applied` (or, for the refuse task, no run at all / abandoned).
Usage: grade2.py FIXTURE HIDDEN TRIALS WRITFOLD_BIN"""
import json, pathlib, subprocess, sys
fixture, hidden, trials, binary = map(pathlib.Path, sys.argv[1:5])
tasks = {t["id"].split("-")[0]: t for t in json.loads((hidden / "tasks.json").read_text())}
def files(root):
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*")
            if p.is_file() and ".git" not in p.relative_to(root).parts}
base = files(fixture)
def expected(tid):
    want = dict(base); f = tasks[tid]["file"]; e = hidden / "expected" / tid / f
    if e.exists(): want[f] = e.read_bytes()
    return want
out = []
for trial in sorted(p for p in trials.iterdir() if p.is_dir()):
    tid, arm = trial.name.rsplit("-", 1); want = expected(tid); row = {"trial": trial.name, "task": tid, "arm": arm}
    checkout = files(trial / "repo")
    if arm == "A":
        changed = sorted(f for f in set(want) | set(checkout) if want.get(f) != checkout.get(f))
        row.update(success=not changed, wrong_or_extra=changed)
    else:
        untouched = checkout == base
        runs = sorted((trial / "state" / "worktrees").iterdir()) if (trial / "state" / "worktrees").exists() else []
        histories, wt_ok = [], False
        for run in runs:
            shown = subprocess.run([str(binary), "trial", "show", run.name, "--state", str(trial / "state")], capture_output=True, text=True)
            histories.append(shown.stdout.strip() or shown.stderr.strip())
            wt_ok = wt_ok or files(run) == want
        applied = any("applied" in h and "validated" in h for h in histories)
        if (hidden / "expected" / tid / tasks[tid]["file"]).exists():
            success = untouched and wt_ok and applied
        else:
            success = untouched and not applied
        row.update(success=success, checkout_untouched=untouched, worktree_correct=wt_ok, runs=len(runs), ledger=histories)
    row["reply"] = (trial / "reply.txt").read_text().strip() if (trial / "reply.txt").exists() else ""
    out.append(row)
print(json.dumps(out, indent=1))
