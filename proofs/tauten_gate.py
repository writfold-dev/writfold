#!/usr/bin/env python3
"""tauten adapter for the `proofs` gate (docs/writfold/tauten-adoption.md).

Runs inside the gate image (proofs/tauten-gate.sh starts it). It runs the
unchanged gate, `sh proofs/gate.sh`, passes its output through to stderr and
prints what it established as `TAUTEN-PROOFS <json>` lines on stdout, the
result lines `cargo tauten proofs` reads (tauten's docs/proofs/README.md, P5):

  extract  ok once gate.sh starts its drift check
  drift    ok once gate.sh starts `lake build`
  build    ok once gate.sh starts the axiom audit
  audit    ok once gate.sh starts the mutants and Audit.lean printed
           AXIOM_AUDIT ok; one theorem line per `AXIOMS` line it printed
  mutants  ok once gate.sh printed `proofs gate: OK`; one mutant line per
           `calibration:` (killed, with the theorems that broke) or `FAIL`
           line of proofs/mutants.py

After a passing build it also runs proofs/Total.lean, which lists the
theorems of tauten's total shape (`∀ args, f args ⦃ _ ⦄`, no hypotheses), as
`total` lines. Nothing here decides an outcome, and a stage that gate.sh did
not reach is reported `fail`, never `ok`. The exit status is gate.sh's.
"""
from pathlib import Path
import json
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
ALLOWED = {"propext", "Classical.choice", "Quot.sound"}
AXIOMS = re.compile(r"AXIOMS (?P<name>\S+): \[(?P<axioms>[^\]]*)\]", re.S)
TOTAL = re.compile(r"TOTAL (?P<theorem>\S+) (?P<function>\S+)")
KILLED = re.compile(r"^calibration: (?P<name>\S+) rejected by (?P<broken>.+)$", re.M)
FAILED = re.compile(r"^FAIL (?P<name>[\w-]+): (?P<detail>.+)$", re.M)
# The stamp gate.sh prints when a stage starts, which means the one before passed.
REACHED = {
    "extract": "drift check against proofs/Extracted",
    "drift": "] lake build",
    "build": "] axiom audit",
    "audit": "] mutant calibration",
    "mutants": "] proofs gate: OK",
}


def emit(**fields) -> None:
    print("TAUTEN-PROOFS " + json.dumps(fields, sort_keys=True), flush=True)


def main() -> int:
    missing = [tool for tool in ("charon", "aeneas", "lake", "python3") if shutil.which(tool) is None]
    if missing:
        emit(stage="tools", status="missing", detail=f"not on PATH: {', '.join(missing)}")
        return 3
    emit(stage="tools", status="ok")

    # Arguments (`--only a,b` from `[gate] try`) select the mutants (#91).
    gate = subprocess.Popen(["sh", "proofs/gate.sh", *sys.argv[1:]], cwd=ROOT, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    lines = []
    assert gate.stdout is not None
    for line in gate.stdout:
        sys.stderr.write(line)
        sys.stderr.flush()
        lines.append(line)
    status = gate.wait()
    output = "".join(lines)

    for stage, stamp in REACHED.items():
        reached = stamp in output
        if stage == "audit":
            reached = reached and "AXIOM_AUDIT ok" in output
        if stage == "mutants":
            reached = reached and status == 0
        if stage == "build" and reached:
            # the audit's per-theorem results, printed before AXIOM_AUDIT
            # Lake replays a module's messages (`info: Audit.lean:…: AXIOMS …`)
            # beside the audit's own lines: each theorem is reported once,
            # failing if any of its lines names another axiom.
            audited: dict[str, bool] = {}
            for match in AXIOMS.finditer(output):
                axioms = {a.strip() for a in match.group("axioms").split(",") if a.strip()}
                name = match.group("name")
                audited[name] = audited.get(name, True) and axioms <= ALLOWED
            for name, ok in audited.items():
                emit(theorem=name, status="ok" if ok else "fail")
        if stage == "build" and reached:
            total = subprocess.run(["lake", "env", "lean", "Total.lean"], cwd=ROOT / "proofs",
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            sys.stderr.write(total.stdout)
            if total.returncode == 0:
                for match in TOTAL.finditer(total.stdout):
                    emit(total=match.group("theorem"), function=match.group("function"))
        if stage == "audit" and reached:
            for match in KILLED.finditer(output):
                broken = sorted(t.strip() for t in match.group("broken").split(","))
                emit(mutant=match.group("name"), status="killed", broken=broken)
            for match in FAILED.finditer(output):
                emit(mutant=match.group("name"), status="survived", detail=match.group("detail"))
        emit(stage=stage, status="ok" if reached else "fail")
        if not reached:
            break
    return status


if __name__ == "__main__":
    sys.exit(main())
