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
