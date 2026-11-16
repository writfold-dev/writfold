#!/usr/bin/env python3
"""Mutant calibration for the `proofs` gate (docs/writfold/aeneas.md).

Every moved kernel function gets at least one semantic mutation of its Rust
source. For each: copy the crate, apply the mutation, translate it exactly as
the gate does (proofs/extract.sh), build the theorems against the mutated
translation in a scratch copy of the lake project, and require the build to
FAIL inside the named theorem(s) — no other theorem may fail, and the
translation itself must still type-check. A proof that survives its mutant
proves nothing; a failure anywhere else is noise, not calibration.
"""
from dataclasses import dataclass
from pathlib import Path
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PROOFS = ROOT / "proofs"
KERNEL = ROOT / "crates" / "writfold-core"


@dataclass(frozen=True)
class Mutant:
    name: str
    file: str
    before: str
    after: str
    fails: tuple[str, ...]


MUTANTS = (
    # identity is byte-exact: ignoring the last byte is a different identity
    Mutant("same-ignores-last-byte", "bytes.rs",
           "    while i < left.len() {\n        if left[i] != right[i] {",
           "    while i + 1 < left.len() {\n        if left[i] != right[i] {",
           ("Writfold.Core.same_loop_spec",)),
    # run graph: a skipped step is closed, so it is never current
    Mutant("closed-ignores-skip", "plan.rs",
           "    node.done || node.skipped",
           "    node.done",
           ("Writfold.Core.Plan.closed_spec",)),
    # run graph: only the current step may be recorded as done
    Mutant("advance-without-current-check", "plan.rs",
           "    if now != at {",
           "    if false {",
           ("Writfold.Core.Plan.advance_check_spec",)),
    # run graph: every gate of the step must be met
    Mutant("advance-ignores-gates", "plan.rs",
           "        Some(gate) => Err(Refusal::Owed(gate)),",
           "        Some(_) => Ok(()),",
           ("Writfold.Core.Plan.advance_check_spec",)),
    # run graph: a stale step is a done step with an owed gate
    Mutant("stale-ignores-done", "plan.rs",
           "    node.done && first_owed(met).is_some()",
           "    !node.skipped && first_owed(met).is_some()",
           ("Writfold.Core.Plan.stale_spec",)),
    # splice: the new steps go before the current step, not after it
    Mutant("splice-inserts-after-current", "plan.rs",
           "    while index < nodes.len() && index < at {",
           "    while index < nodes.len() && index <= at {",
           ("Writfold.Core.Plan.splice_loop0_spec",)),
    # splice: the new steps are open
    Mutant("splice-new-steps-done", "plan.rs",
           "            done: false,\n            skipped: false,",
           "            done: true,\n            skipped: false,",
           ("Writfold.Core.Plan.splice_loop1_spec",)),
    # splice: every existing step is kept, the last one included
    Mutant("splice-drops-last-step", "plan.rs",
           "    while index < nodes.len() {\n        out.push(Node {",
           "    while index + 1 < nodes.len() {\n        out.push(Node {",
           ("Writfold.Core.Plan.splice_loop2_spec",)),
    # splice: idempotent on the template key
    Mutant("splice-ignores-key", "plan.rs",
           "    if contains(spliced, key) {",
           "    if false {",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # splice: depth is bounded by MAX_DEPTH
    Mutant("splice-depth-off-by-one", "plan.rs",
           "    if depth > MAX_DEPTH {",
           "    if depth > MAX_DEPTH + 1 {",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # gate: a subject that moved while the command ran is never a pass
    Mutant("outcome-ignores-moved", "gate.rs",
           "    if subject_moved {\n        return Outcome::Unknown;",
           "    if false {\n        return Outcome::Unknown;",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: only exit status 0 passes
    Mutant("outcome-any-exit-passes", "gate.rs",
           "        Ending::Exited(0) => Outcome::Pass,",
           "        Ending::Exited(_) => Outcome::Pass,",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: a receipt about other bytes in scope never decides the state
    Mutant("state-ignores-scope", "gate.rs",
           "        if evidence.same_definition && evidence.same_scope {",
           "        if evidence.same_definition {",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: a waiver is never a passing check
    Mutant("waiver-satisfies", "gate.rs",
           "        return Standing::Waived;",
           "        return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Gate.standing_spec",)),
    # gate: an attestation is never a passing check
    Mutant("attestation-satisfies", "gate.rs",
           "            return Standing::Attested;",
           "            return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Gate.standing_spec",)),
    # gate: a stale obligation is owed
    Mutant("met-counts-stale", "gate.rs",
           "Standing::Checked(State::Open | State::Stale | State::Failed | State::Unknown)",
           "Standing::Checked(State::Open | State::Failed | State::Unknown)",
           ("Writfold.Core.Gate.met_spec",)),
    # assessed gate: one ungrounded premise makes the basis hypothetical
    Mutant("basis-ignores-premises", "assess.rs",
           "        if matches!(premises[index], Grounding::Ungrounded) {",
           "        if false {",
           ("Writfold.Core.Assess.basis_loop_spec",)),
    # assessed gate: a hypothetical entailment never meets the gate
    Mutant("met-accepts-hypothetical", "assess.rs",
           "    matches!(verdict, Verdict::Entailed) && matches!(basis, Basis::Grounded)",
           "    matches!(verdict, Verdict::Entailed) || matches!(basis, Basis::Grounded)",
           ("Writfold.Core.Assess.met_spec",)),
    # assessed gate: a check Weft could not finish is unknown, never a pass
    Mutant("outcome-passes-unfinished", "assess.rs",
           "        if unfinished(required[index]) {",
           "        if false {",
           ("Writfold.Core.Assess.outcome_loop_spec",)),
    # assessed gate: an attested citation makes the assumption attested
    Mutant("assumption-ignores-attested", "assess.rs",
           "        true => Grounding::Attested,",
           "        true => Grounding::Grounded,",
           ("Writfold.Core.Assess.assumption_loop_spec",)),
    # Retrieval pointers cannot inherit an independently checked verdict.
    Mutant("semantic-pointer-grounds", "semantic_evidence.rs",
           "        Origin::SemanticRetrieval => Grounding::Ungrounded,",
           "        Origin::SemanticRetrieval => checked,",
           ("Writfold.Core.SemanticEvidence.semantic_never_grounds",)),
    Mutant("lexical-pointer-grounds", "semantic_evidence.rs",
           "        Origin::LexicalRetrieval => Grounding::Ungrounded,",
           "        Origin::LexicalRetrieval => checked,",
           ("Writfold.Core.SemanticEvidence.lexical_never_grounds",)),
    Mutant("checked-source-ignores-negative", "semantic_evidence.rs",
           "        Origin::CheckedSource => checked,",
           "        Origin::CheckedSource => Grounding::Grounded,",
           ("Writfold.Core.SemanticEvidence.checked_preserved",)),
    Mutant("semantic-rrf-equality-overtakes", "semantic_order.rs",
           "        semantic_rank < lexical_rank",
           "        semantic_rank <= lexical_rank",
           ("Writfold.Core.SemanticOrder.semantic_wins_spec",)),
    Mutant("semantic-rrf-wrong-weight", "semantic_order.rs",
           "        let semantic_denominator = 60 + semantic_rank;",
           "        let semantic_denominator = 59 + semantic_rank;",
           ("Writfold.Core.SemanticOrder.semantic_wins_spec",)),
    Mutant("semantic-exact-unprotected", "semantic_order.rs",
           "    lexical.exact || !semantic_wins(lexical.rank, lexical.semantic_rank, semantic_rank)",
           "    !semantic_wins(lexical.rank, lexical.semantic_rank, semantic_rank)",
           ("Writfold.Core.SemanticOrder.blocks_spec", "Writfold.Core.SemanticOrder.exact_blocks")),
    # Equivalent to `slot = index`, whose old-value alias hits an Aeneas
    # loop-join limitation. Explicit subtraction preserves the same fault.
    Mutant("semantic-slot-before-blocker", "semantic_order.rs",
           "            slot = next;", "            slot = next - 1;",
           ("Writfold.Core.SemanticOrder.semantic_slot_loop_spec",)),
    Mutant("semantic-lexical-first-prepends", "semantic_order.rs",
           "    if !rrf {\n        return lexical.len();",
           "    if !rrf {\n        return 0;",
           ("Writfold.Core.SemanticOrder.lexical_first",)),
    Mutant("semantic-position-order-reversed", "semantic_order.rs",
           "        return left.slot < right.slot;",
           "        return left.slot > right.slot;",
           ("Writfold.Core.SemanticOrder.before_spec",)),
    Mutant("semantic-tie-order-reversed", "semantic_order.rs",
           "    left.rank < right.rank", "    left.rank > right.rank",
           ("Writfold.Core.SemanticOrder.before_spec",)),
    Mutant("semantic-lexical-key-collapses", "semantic_order.rs",
           "        slot: index,\n        semantic: false,",
           "        slot: 0,\n        semantic: false,",
           ("Writfold.Core.SemanticOrder.lexical_order_preserved", "Writfold.Core.SemanticOrder.exact_protected")),
    Mutant("semantic-rank-key-collapses", "semantic_order.rs",
           "        semantic: true,\n        rank,",
           "        semantic: true,\n        rank: 0,",
           ("Writfold.Core.SemanticOrder.semantic_ties_stable",)),
    Mutant("semantic-invalid-rank-admitted", "semantic_order.rs",
           "    if lexical_rank > 50 {\n        return false;",
           "    if lexical_rank > 50 {\n        return true;",
           ("Writfold.Core.SemanticOrder.invalid_rank_refused",)),
)

# lake reports `error: <path>.lean:<line>:<col>: <message>`
ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
DECLARATION = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)", re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)


def declarations(path: Path) -> list[tuple[int, str]]:
    """(line, fully qualified name) of every declaration in a theorem file."""
    namespace = ""
    found = []
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if match := NAMESPACE.match(line):
            namespace = match.group("name") + "."
            continue
        if line.startswith("end "):
            namespace = ""
            continue
        if match := DECLARATION.match(line):
            found.append((number, namespace + match.group("name")))
    return found


def attribute(file: Path, line: int) -> str | None:
    enclosing = None
    for start, name in declarations(file):
        if start <= line:
            enclosing = name
    return enclosing


def failing_theorems(output: str, project: Path) -> tuple[set[str], list[str]]:
    failures: set[str] = set()
    unattributed: list[str] = []
    for match in ERROR.finditer(output):
        file = (project / match.group("file")).resolve()
        try:
            relative = file.relative_to(project)
        except ValueError:
            unattributed.append(match.group(0))
            continue
        if relative.parts[0] != "Writfold" or not file.exists():
            unattributed.append(str(relative))
            continue
        name = attribute(file, int(match.group("line")))
        if name is None:
            unattributed.append(str(relative))
        else:
            failures.add(name)
    return failures, unattributed


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, timeout=1800)


def calibrate(mutant: Mutant, scratch: Path) -> None:
    crate = scratch / "crate"
    shutil.copytree(KERNEL, crate, ignore=shutil.ignore_patterns("target"))
    source = crate / "src" / mutant.file
    text = source.read_text()
    if text.count(mutant.before) != 1:
        raise SystemExit(f"FAIL {mutant.name}: mutation site does not match exactly once")
    source.write_text(text.replace(mutant.before, mutant.after))

    extraction = run(["sh", str(PROOFS / "extract.sh"), str(crate), str(scratch / "Extracted")], ROOT)
    if extraction.returncode != 0:
        print(extraction.stdout)
        raise SystemExit(f"FAIL {mutant.name}: the mutated kernel did not translate")

    project = scratch / "proofs"
    project.mkdir()
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Writfold.lean", "Audit.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(PROOFS / "Writfold", project / "Writfold")
    shutil.copytree(scratch / "Extracted", project / "Extracted")
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")

    build = run(["lake", "build"], project)
    if build.returncode == 0:
        raise SystemExit(f"FAIL {mutant.name}: the proofs still build against the mutated kernel")
    failures, unattributed = failing_theorems(build.stdout, project)
    if unattributed:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: errors outside the theorem files: {unattributed}")
    expected = set(mutant.fails)
    if not failures or not expected <= failures:
        print(build.stdout)
        print(f"attributed failures: {sorted(failures)}")
        raise SystemExit(f"FAIL {mutant.name}: expected {sorted(expected)} to fail, got {sorted(failures)}")
    if failures - expected:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: unexpected failing theorems {sorted(failures - expected)}")
    print(f"calibration: {mutant.name} rejected by {', '.join(sorted(failures))}", flush=True)


def main() -> None:
    selected = set(sys.argv[1:])
    mutants = [m for m in MUTANTS if not selected or m.name in selected]
    if selected and len(mutants) != len(selected):
        raise SystemExit(f"unknown mutant(s): {sorted(selected - {m.name for m in mutants})}")
    # Every mutant runs even after a failure, so one gate run reports them all.
    failed: list[str] = []
    for mutant in mutants:
        with tempfile.TemporaryDirectory(prefix=f"mutant-{mutant.name}-") as directory:
            try:
                calibrate(mutant, Path(directory))
            except SystemExit as error:
                print(error, flush=True)
                failed.append(mutant.name)
    if failed:
        raise SystemExit(f"FAIL: {len(failed)} mutant(s) not calibrated: {failed}")
    print(f"PROOF_CALIBRATIONS={len(mutants)}", flush=True)


if __name__ == "__main__":
    main()
