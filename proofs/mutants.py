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
    # run graph: the current step is the least open one, not the last step
    Mutant("current-returns-last", "plan.rs",
           "        if !closed(&nodes[index]) {\n            return Some(index);",
           "        if !closed(&nodes[index]) {\n            return Some(nodes.len() - 1);",
           ("Writfold.Core.Plan.current_loop_spec",)),
    # run graph: the first owed gate is the first one not met
    Mutant("first-owed-inverted", "plan.rs",
           "        if !met[index] {",
           "        if met[index] {",
           ("Writfold.Core.Plan.first_owed_loop_spec",)),
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
    # splice: the last key spliced counts too
    Mutant("contains-misses-last", "plan.rs",
           "    while index < keys.len() {",
           "    while index + 1 < keys.len() {",
           ("Writfold.Core.Plan.contains_loop_spec",)),
    # splice: an empty plan takes a root splice at 0
    Mutant("splice-check-refuses-empty", "plan.rs",
           "    if nodes.is_empty() {\n        return Ok(Spliced::Insert(0));",
           "    if nodes.is_empty() {\n        return Err(SpliceRefusal::Complete);",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # splice: a complete plan has no current step to splice under
    Mutant("splice-check-admits-complete", "plan.rs",
           "        None => Err(SpliceRefusal::Complete),\n        Some(at) => Ok(Spliced::Insert(at)),",
           "        None => Ok(Spliced::Insert(0)),\n        Some(at) => Ok(Spliced::Insert(at)),",
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
    # gate: another exit status is a failure, not unknown
    Mutant("outcome-nonzero-unknown", "gate.rs",
           "        Ending::Exited(_) => Outcome::Fail,",
           "        Ending::Exited(_) => Outcome::Unknown,",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: a receipt about other bytes in scope never decides the state
    Mutant("state-ignores-scope", "gate.rs",
           "        if evidence.same_definition && evidence.same_scope {",
           "        if evidence.same_definition {",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: a receipt about another definition never decides the state
    Mutant("state-ignores-definition", "gate.rs",
           "        if evidence.same_definition && evidence.same_scope {",
           "        if evidence.same_scope {",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: the newest receipt about the current subject decides, not the oldest
    Mutant("state-oldest-first", "gate.rs",
           "    let mut index = receipts.len();\n    while index > 0 {\n        index -= 1;\n        let evidence = receipts[index];",
           "    let mut index = 0;\n    while index < receipts.len() {\n        let evidence = receipts[index];\n        index += 1;",
           ("Writfold.Core.Gate.state_loop_spec", "Writfold.Core.Gate.state_spec")),
    # gate: a failed receipt is a failure
    Mutant("state-failed-satisfies", "gate.rs",
           "                Outcome::Fail => State::Failed,",
           "                Outcome::Fail => State::Satisfied,",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: an attested-class obligation has no checked state to stand on
    Mutant("attested-class-stands-on-check", "gate.rs",
           "        (true, false) => Standing::Checked(State::Open),",
           "        (true, false) => Standing::Checked(state),",
           ("Writfold.Core.Gate.standing_spec",)),
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
    # about the present: the recorded definition must be the present one
    Mutant("about-ignores-definition", "gate.rs",
           "    crate::bytes::same(recorded_definition, definition)\n        && crate::bytes::same(recorded_content, content)",
           "    crate::bytes::same(recorded_content, content)",
           ("Writfold.Core.Gate.about_spec",)),
    # a receipt under another definition is not the same definition
    Mutant("evidence-ignores-definition", "gate.rs",
           "        same_definition: crate::bytes::same(&receipt.definition, definition),",
           "        same_definition: true,",
           ("Writfold.Core.Gate.evidence_spec",)),
    # a receipt about other bytes is not about the present content
    Mutant("evidence-ignores-content", "gate.rs",
           "        same_scope: !content.is_empty() && crate::bytes::same(&receipt.content, content),",
           "        same_scope: !content.is_empty(),",
           ("Writfold.Core.Gate.evidence_spec",)),
    # empty present content (unreadable) is never the same content
    Mutant("evidence-accepts-empty-content", "gate.rs",
           "        same_scope: !content.is_empty() && crate::bytes::same(&receipt.content, content),",
           "        same_scope: crate::bytes::same(&receipt.content, content),",
           ("Writfold.Core.Gate.evidence_spec",)),
    # every receipt is judged, the newest included
    Mutant("judge-drops-newest", "gate.rs",
           "    while index < receipts.len() {\n        all.push(",
           "    while index + 1 < receipts.len() {\n        all.push(",
           ("Writfold.Core.Gate.judge_loop_spec",)),
    # an attestation is never a waiver, and a waiver never an attestation
    Mutant("stands-ignores-kind", "gate.rs",
           "    if said.attested != attested {",
           "    if false {",
           ("Writfold.Core.Gate.stands_spec",)),
    # a waiver about other bytes does not stand
    Mutant("statements-counts-moved-waiver", "gate.rs",
           "        if stands(&said[index], false, definition, content) {",
           "        if !said[index].attested {",
           ("Writfold.Core.Gate.statements_loop_spec",)),
    # a gate with no usable definition is never met
    Mutant("advance-undefined-met", "plan.rs",
           "        GateFact::Undefined => false,",
           "        GateFact::Undefined => true,",
           ("Writfold.Core.Plan.fact_met_spec",)),
    # every gate of the step is judged, the last included
    Mutant("met-all-drops-last", "plan.rs",
           "    while index < gates.len() {\n        met.push(",
           "    while index + 1 < gates.len() {\n        met.push(",
           ("Writfold.Core.Plan.met_all_loop_spec",)),
    # a step with no gates owes nothing
    Mutant("advance-refuses-no-gates", "plan.rs",
           "    advance_check(nodes, at, &met_all(gates))",
           "    if gates.is_empty() {\n        return Err(Refusal::Owed(0));\n    }\n    advance_check(nodes, at, &met_all(gates))",
           ("Writfold.Core.Plan.advance_facts_spec", "Writfold.Core.Plan.advance_facts_ok_iff")),
    # checklist: a supported row covers only when it cites something
    Mutant("item-accepts-no-evidence", "checklist.rs",
           "            if row.evidence > 0 && row.grounded {",
           "            if row.grounded {",
           ("Writfold.Core.Checklist.item_spec",)),
    # checklist: a supported row covers only while its evidence is grounded
    Mutant("item-accepts-ungrounded", "checklist.rs",
           "            if row.evidence > 0 && row.grounded {",
           "            if row.evidence > 0 {",
           ("Writfold.Core.Checklist.item_spec",)),
    # checklist: a row about other content or another definition is stale
    Mutant("item-accepts-not-current", "checklist.rs",
           "    if !row.current {\n        return Item::Stale;",
           "    if false {\n        return Item::Stale;",
           ("Writfold.Core.Checklist.item_spec",)),
    # checklist: an unsupported item is not covered
    Mutant("covered-counts-unsupported", "checklist.rs",
           "        Item::Supported | Item::NotApplicable => true,\n        Item::Open | Item::Unsupported | Item::Stale => false,",
           "        Item::Supported | Item::NotApplicable | Item::Unsupported => true,\n        Item::Open | Item::Stale => false,",
           ("Writfold.Core.Checklist.covered_spec",)),
    # checklist: only required items are owed
    Mutant("rollup-ignores-required", "checklist.rs",
           "    entry.required && !covered(entry.item)",
           "    !covered(entry.item)",
           ("Writfold.Core.Checklist.owes_spec",)),
    # checklist: every entry is looked at, the last included
    Mutant("first-owed-skips-last", "checklist.rs",
           "    while index < entries.len() {\n        if owes(",
           "    while index + 1 < entries.len() {\n        if owes(",
           ("Writfold.Core.Checklist.first_owed_loop_spec",)),
    # checklist: an optional stale item does not make the checklist stale
    Mutant("stale-ignores-required", "checklist.rs",
           "    entry.required && matches!(entry.item, Item::Stale)",
           "    matches!(entry.item, Item::Stale)",
           ("Writfold.Core.Checklist.stale_spec",)),
    # checklist: a stale checklist is not attested
    Mutant("checklist-attested-when-stale", "checklist.rs",
           "        attested_current: rollup.covered,",
           "        attested_current: rollup.covered || rollup.stale,",
           ("Writfold.Core.Checklist.standing_spec",)),
    # delivery: an empty set of obligations is not met
    Mutant("delivery-empty-met", "delivery.rs",
           "    if obligations.is_empty() {\n        return Standing::Checked(State::Open);",
           "    if obligations.is_empty() {\n        return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Delivery.standing_spec",)),
    # delivery: a waived obligation is never read as a passing check
    Mutant("delivery-waived-satisfied", "delivery.rs",
           "    if waived {\n        Standing::Waived",
           "    if false {\n        Standing::Waived",
           ("Writfold.Core.Delivery.standing_loop_spec",)),
    # delivery: an unmet obligation stops the delivery
    Mutant("delivery-unmet-ignored", "delivery.rs",
           "    !standing.met()",
           "    false",
           ("Writfold.Core.Delivery.unmet_spec",)),
    # delivery: a bound candidate never decides an open step
    Mutant("deciding-ignores-bound", "delivery.rs",
           "        if !bound[index] {",
           "        if true {",
           ("Writfold.Core.Delivery.deciding_loop_spec", "Writfold.Core.Delivery.deciding_spec")),
    # delivery: the newest unbound candidate decides, not the oldest
    Mutant("deciding-first-unbound", "delivery.rs",
           "    let mut index = bound.len();\n    while index > 0 {\n        index -= 1;\n        if !bound[index] {",
           "    let mut index = 0;\n    while index < bound.len() {\n        index += 1;\n        if !bound[index - 1] {",
           ("Writfold.Core.Delivery.deciding_loop_spec", "Writfold.Core.Delivery.deciding_spec")),
    # delivery: a mode that declares nothing is never ready
    Mutant("ready-accepts-empty", "delivery.rs",
           "    standing(obligations).met()",
           "    obligations.is_empty() || standing(obligations).met()",
           ("Writfold.Core.Delivery.ready_iff",)),
    # delivery: a passed check whose cited evidence moved is stale
    Mutant("rechecked-keeps-satisfied", "delivery.rs",
           "    if matches!(checked, State::Satisfied) && moved > 0 {",
           "    if false {",
           ("Writfold.Core.Delivery.rechecked_spec",)),
    # delivery guard: unknown bytes need at least one candidate
    Mutant("guard-allows-no-candidate", "delivery.rs",
           "        Body::Unknown => candidates > 0 && all_ready,",
           "        Body::Unknown => all_ready,",
           ("Writfold.Core.Delivery.guard_allow_iff",)),
    # delivery guard: known bytes pass only when a ready candidate holds them
    Mutant("guard-known-any-ready", "delivery.rs",
           "        Body::Known => same_ready,",
           "        Body::Known => same_ready || all_ready,",
           ("Writfold.Core.Delivery.guard_allow_iff",)),
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
    # assessed gate: nothing required is unknown, never a pass
    Mutant("assessed-empty-passes", "assess.rs",
           "    if !complete || required.is_empty() {",
           "    if !complete {",
           ("Writfold.Core.Assess.outcome_spec",)),
    # assessed gate: an incomplete run is unknown, never a pass
    Mutant("assessed-incomplete-passes", "assess.rs",
           "    if !complete || required.is_empty() {",
           "    if required.is_empty() {",
           ("Writfold.Core.Assess.outcome_spec",)),
    # assessed gate: an attested citation makes the assumption attested
    Mutant("assumption-ignores-attested", "assess.rs",
           "        true => Grounding::Attested,",
           "        true => Grounding::Grounded,",
           ("Writfold.Core.Assess.assumption_loop_spec",)),
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


def unclaimed() -> list[str]:
    """Theorems a mutant names that the axiom audit does not require: a
    rename would then pass the audit and be caught only here."""
    claims = set(re.findall(r"``([\w.]+)", (PROOFS / "Audit.lean").read_text()))
    return sorted({name for m in MUTANTS for name in m.fails} - claims)


def main() -> None:
    if missing := unclaimed():
        raise SystemExit(f"FAIL: theorems named by mutants are not audited claims: {missing}")
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
