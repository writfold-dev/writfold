#!/usr/bin/env python3
"""Grade projection-pilot trials against the hidden expected evidence.

Usage: grade.py EXPECTED_JSON TRIALS_DIR -> JSON list, one object per trial.

Each trial directory is named <task>-<arm>-r<repeat> and holds answer.txt with
lines `<path>:<line>: <text>`. A trial succeeds only if the set of (path, line)
pairs equals the expected set exactly and every quoted text equals the source
line after stripping leading and trailing whitespace. Blank lines are ignored;
any other line that does not parse counts as a wrong line. A missing answer.txt
is a failure."""
import json, pathlib, re, sys

ROW = re.compile(r"^(?:\./)?([^\s:][^:]*):(\d+):\s?(.*)$")


def grade(expected, answer_path):
    want = {(e["path"], e["line"]): e["text"].strip() for e in expected}
    if not answer_path.exists():
        return {"answer_present": False, "success": False, "found": 0,
                "expected": len(want), "missing": sorted(f"{p}:{n}" for p, n in want),
                "extra": [], "wrong_text": [], "malformed": []}
    got, malformed = {}, []
    for raw in answer_path.read_text(errors="replace").splitlines():
        if not raw.strip():
            continue
        m = ROW.match(raw.strip())
        if not m:
            malformed.append(raw[:200])
            continue
        got[(m.group(1), int(m.group(2)))] = m.group(3).strip()
    missing = sorted(f"{p}:{n}" for p, n in set(want) - set(got))
    extra = sorted(f"{p}:{n}" for p, n in set(got) - set(want))
    wrong = sorted(f"{p}:{n}" for k in set(got) & set(want) if got[k] != want[k] for p, n in [k])
    correct = len(set(got) & set(want)) - len(wrong)
    return {"answer_present": True,
            "success": not missing and not extra and not wrong and not malformed,
            "found": correct, "expected": len(want), "missing": missing,
            "extra": extra, "wrong_text": wrong, "malformed": malformed}


def main():
    expected = json.loads(pathlib.Path(sys.argv[1]).read_text())["answers"]
    out = []
    for trial in sorted(p for p in pathlib.Path(sys.argv[2]).iterdir() if p.is_dir()):
        task, arm, rep = trial.name.split("-")
        out.append({"trial": trial.name, "task": task, "arm": arm, "repeat": int(rep[1:]),
                    **grade(expected[task], trial / "answer.txt")})
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
