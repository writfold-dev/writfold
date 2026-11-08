#!/usr/bin/env python3
"""Grade one trial: exact bytes of every fixture file, no extra files.
Usage: grade.py FIXTURE HIDDEN TRIALS_DIR -> JSON per trial."""
import json, pathlib, sys
fixture, hidden, trials = map(pathlib.Path, sys.argv[1:4])
tasks = {t["id"].split("-")[0]: t for t in json.loads((hidden / "tasks.json").read_text())}
def files(root):
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}
base = files(fixture)
out = []
for trial in sorted(p for p in trials.iterdir() if p.is_dir()):
    tid, arm = trial.name.rsplit("-", 1)
    task = tasks[tid]
    want = dict(base)
    exp = hidden / "expected" / tid / task["file"]
    if exp.exists():
        want[task["file"]] = exp.read_bytes()
    got = files(trial / "repo")
    target_ok = got.get(task["file"]) == want[task["file"]]
    out_of_scope = sorted(f for f in base if f != task["file"] and got.get(f) != base[f])
    extra = sorted(set(got) - set(base))
    reply = (trial / "reply.txt").read_text().strip() if (trial / "reply.txt").exists() else ""
    out.append({"trial": trial.name, "task": tid, "arm": arm, "target_correct": target_ok,
                "out_of_scope_changes": out_of_scope, "extra_files": extra,
                "success": target_ok and not out_of_scope and not extra,
                "reply": reply[:300]})
print(json.dumps(out, indent=1))
