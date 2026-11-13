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
    # historyCheck: every entry must also be valid, not just linked
    Mutant("history-skips-validator", "ledger.rs",
           "    linked && validator.validate(entries, position, entry)",
           "    linked",
           ("Writfold.Core.entry_ok_spec",)),
    # historyCheck: entries are numbered from 1 without gaps
    Mutant("history-ignores-sequence", "ledger.rs",
           "    if !same(&entry.stream, stream) || entry.sequence != position + 1 {",
           "    if !same(&entry.stream, stream) {",
           ("Writfold.Core.entry_ok_spec",)),
    # historyCheck: the whole prefix, not only its first entry
    Mutant("history-checks-first-only", "ledger.rs",
           "        if !entry_ok(validator, stream, entries, position) {\n            return false;\n        }\n        position += 1;",
           "        if !entry_ok(validator, stream, entries, position) {\n            return false;\n        }\n        position = entries.len();",
           ("Writfold.Core.history_check_loop_spec",)),
    # AppendReady: the expected head must be the current head
    Mutant("ready-ignores-stale-head", "ledger.rs",
           "    if !head_matches(entries, expected) {\n        return Err(Refusal::StaleHead);",
           "    if false {\n        return Err(Refusal::StaleHead);",
           ("Writfold.Core.ready_against_spec",)),
    # AppendReady: record ids are unique across every stream
    Mutant("ready-admits-duplicate", "ledger.rs",
           "    if retains_record(selections, &record.id) {",
           "    if false {",
           ("Writfold.Core.ready_against_spec",)),
    # AppendReady: closure material must be available or retained
    Mutant("ready-ignores-material", "ledger.rs",
           "    if !material_ok(&record.closure, available, material) {",
           "    if false {",
           ("Writfold.Core.ready_against_spec",)),
    # AppendReady refusal order: sequence is checked before linkage
    Mutant("ready-reorders-refusals", "ledger.rs",
           "        return Err(Refusal::Predecessor);",
           "        return Err(Refusal::Sequence);",
           ("Writfold.Core.ready_against_spec",)),
    # the duplicate search covers every selection, not only the first
    Mutant("retains-first-selection-only", "ledger.rs",
           "        if entries_contain(&selections[i].entries, id) {\n            return true;\n        }\n        i += 1;",
           "        if entries_contain(&selections[i].entries, id) {\n            return true;\n        }\n        i = selections.len();",
           ("Writfold.Core.retains_record_loop_spec",)),
    # material: available OR already retained, not both
    Mutant("material-requires-both", "ledger.rs",
           "        if !contains_digest(available, &closure[i]) && !contains_digest(material, &closure[i]) {",
           "        if !contains_digest(available, &closure[i]) || !contains_digest(material, &closure[i]) {",
           ("Writfold.Core.material_ok_loop_spec",)),
    # installGrant: the envelope must name the current policy
    Mutant("install-ignores-policy", "authority.rs",
           "    if !same(&authorization.policy, policy) {",
           "    if false {",
           ("Writfold.Core.install_check_spec",)),
    # installGrant: a revoked or installed id can never be installed again
    Mutant("install-reuses-grant-id", "authority.rs",
           "    if has_id(grants, &grant.id) {",
           "    if false {",
           ("Writfold.Core.install_check_spec",)),
    # installGrant: validity is half-open, expiry is exclusive
    Mutant("install-expiry-inclusive", "authority.rs",
           "    if grant.expires <= now {",
           "    if grant.expires < now {",
           ("Writfold.Core.install_check_spec",)),
    # currentGrant: a revoked grant is never current
    Mutant("current-ignores-revocation", "authority.rs",
           "    same(&g.grant.scope, scope) && !g.revoked && g.grant.valid_from <= now && now < g.grant.expires",
           "    same(&g.grant.scope, scope) && g.grant.valid_from <= now && now < g.grant.expires",
           ("Writfold.Core.live_spec",)),
    # intentReady: the current grant must carry every requested resource
    Mutant("covers-ignores-resources", "authority.rs",
           "            has_capability(&grants[i].grant.capabilities, capability)\n                && resources_within(&grants[i].grant.resources, resources)",
           "            has_capability(&grants[i].grant.capabilities, capability)",
           ("Writfold.Core.covers_spec",)),
    # revokeGrant: every grant with the id, including the newest
    Mutant("revoke-skips-newest", "authority.rs",
           "        while i < self.grants.len() {\n            if same(&self.grants[i].grant.id, id) {",
           "        while i + 1 < self.grants.len() {\n            if same(&self.grants[i].grant.id, id) {",
           ("Writfold.Core.revoke_loop_spec",)),
    # a prepared candidate must change its source
    Mutant("changes-always", "trial.rs",
           "    !same(source, candidate)",
           "    true",
           ("Writfold.Core.changes_spec",)),
    # recovery: only the candidate's exact bytes mean the install happened
    Mutant("recovery-trusts-source", "trial.rs",
           "    if same(observed, candidate) {",
           "    if same(observed, candidate) || same(observed, source) {",
           ("Writfold.Core.recovery_spec",)),
    # a later entailed check never hides an earlier refutation
    Mutant("status-forgets-refutation", "trial.rs",
           "    if !matched || status == 2 {",
           "    if !matched {",
           ("Writfold.Core.fold_status_spec",)),
    # a required check absent from the report refuses
    Mutant("required-ignores-missing", "trial.rs",
           "        if status == 0 {\n            return Err(Refusal::Missing(i));",
           "        if false {\n            return Err(Refusal::Missing(i));",
           ("Writfold.Core.required_entailed_loop_spec",)),
    # edit flow: a retained object is its digest AND its length
    Mutant("blob-ignores-length", "flow.rs",
           "    same(&left.digest, &right.digest) && left.bytes == right.bytes",
           "    same(&left.digest, &right.digest)",
           ("Writfold.Core.Flow.same_blob_spec",)),
    # edit flow: every blob a record names is in its closure, the report too
    Mutant("carried-skips-report", "flow.rs",
           "                && contains_digest(closure, &candidate.digest)\n                && contains_digest(closure, &report.digest)",
           "                && contains_digest(closure, &candidate.digest)",
           ("Writfold.Core.Flow.carried_spec",)),
    # edit flow: the record must carry its material, not only follow its prefix
    Mutant("validate-skips-material", "flow.rs",
           "    carried(event, closure) && follows(prefix, event)",
           "    follows(prefix, event)",
           ("Writfold.Core.Flow.validate_spec",)),
    # edit flow: the claim names exactly the prepared candidate
    Mutant("claim-ignores-candidate", "flow.rs",
           "            prefix.len() == 2 && is_approved(&prefix[1]) && prepared(&prefix[0], candidate)",
           "            prefix.len() == 2 && is_approved(&prefix[1])",
           ("Writfold.Core.Flow.follows_spec",)),
    # edit flow: one outcome only; Applied never follows an outcome
    Mutant("applied-after-outcome", "flow.rs",
           "            prefix.len() == 3 && is_claimed(&prefix[2]) && prepared(&prefix[0], candidate)",
           "            prefix.len() >= 3 && is_claimed(&prefix[2]) && prepared(&prefix[0], candidate)",
           ("Writfold.Core.Flow.follows_spec",)),
    # edit flow: an abandonment says why
    Mutant("abandon-without-reason", "flow.rs",
           "            prefix.len() == 3 && is_claimed(&prefix[2]) && !reason.is_empty()",
           "            prefix.len() == 3 && is_claimed(&prefix[2])",
           ("Writfold.Core.Flow.follows_spec",)),
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
