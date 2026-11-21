---
rfc: "2026-11-21-strict-handoff-check"
title: "Judge a handoff against an operator's required gates, not the agent's plan"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/57
milestone: "v0.6.0-dev.14"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Judge a handoff against an operator's required gates, not the agent's plan

## Summary

A new read-only verb, `writfold kb handoff check --policy REV --candidate REV
[--run ID] [--repo DIR]`, answers one question for whoever receives an
agent's work: did every gate the operator requires pass, mechanically, on
this candidate? The required set is `.writfold/handoff.toml` and the gate
definitions **as they are at the policy revision**, read from Git, never from
the candidate tree, the run's plan, its trailers, or whatever gates happened
to load. Evidence is the run's gate receipts, judged against the approved
definition's digest and the candidate commit's scoped content. Only a
current `Checked(Satisfied)` result passes: an attestation, a waiver, a skip,
a missing or stale receipt, a definition the candidate changed, an empty
required set or an unreadable inventory each refuse, naming the gate and the
remedy. Nothing is executed. Without a `handoff.toml` at the policy
revision the verb refuses ("no required set"), and every existing verb is
unchanged.

The same listing serves a cold session: what is required, which pinned
instructions (the approved definition's path at the policy revision) to
read, what was and was not run, and the next command.

## Proof obligations

- **Kernel:** new module `writfold_core::handoff`:
  `enum Reason { Empty, Incomplete, Undefined, DefinitionChanged, NoEvidence, Stale, NotPassed, Statement }`,
  `struct Item { defined: bool, approved: bool, standing: Standing }`,
  `enum Verdict { Accepted, Refused(usize, Reason) }`,
  `fn item(i: &Item) -> Option<Reason>`, `fn first_failing(items: &[Item]) -> Option<(usize, Reason)>` and
  `fn strict(complete: bool, items: &[Item]) -> Verdict`.
- **Theorems** (`proofs/Writfold/Handoff.lean`):
  - `strict_accepted_iff`: `Accepted` iff `complete`, `items` non-empty, and
    for every item `defined ∧ approved ∧ standing = Checked Satisfied`.
  - `statement_never_passes`: an item whose standing is `Attested` or
    `Waived` is never accepted, whatever the gate's `accept` (the kernel has
    no `accept` input; the item's standing is the raw standing).
  - `append_monotone`: if `strict c xs = Refused …` then
    `strict c (xs ++ ys) = Refused …` (adding requirements never turns a
    refusal into acceptance).
  - `refused_lowest`: `Refused i r` names the lowest failing index, and
    `item xs[i] = some r` (or `i = 0` with `Empty`/`Incomplete`).
  - `incomplete_refused`, `empty_refused`.
  - No plan, skip or exception input exists: the signature is the proof;
    a wiring test shows a plan that skips a required gate still refuses.
  - Totality.
- **Mutants:** `strict-accepts-attested` (treat `Attested` as passing;
  breaks `strict_accepted_iff`, `statement_never_passes`),
  `strict-empty-ok` (drop the non-empty test; breaks `empty_refused`),
  `strict-skips-last` (loop stops one short; breaks `strict_accepted_iff`),
  `strict-ignores-approved` (breaks `strict_accepted_iff`).
- **Tested and unproved:** reading `handoff.toml` and the gate definitions
  with `git show REV:path` and `git ls-tree REV`, digesting the candidate's
  scope at its commit (the existing `Snapshot` over a tree with no dirty
  paths), loading the run's receipts, rendering, exit codes. The receiver
  test repository is `crates/writfold-kb/tests/handoff.rs`. No new crashable
  protocol (nothing is written), so no TLC model.

### Amendment: evidence read in full (#67)

`writfold_core::handoff::checked(evidence_complete, complete, items)` wraps
`strict`: when the run's receipt, statement or cover directory could not be
listed in full, or a file in it does not read, the newest evidence is not
known and the candidate is refused as `incomplete` before any gate is
judged. Proved in `proofs/Writfold/Handoff.lean`:

- `accepted_implies_evidence_complete`: an accepted candidate had its
  evidence read in full and passes `strict`.
- `incomplete_evidence_never_accepted`: unread evidence is refused as
  `Incomplete` at index 0.
- `complete_evidence_unchanged`: with evidence read in full, `checked` is
  `strict`.
- Mutant `checked-ignores-evidence` fails the first two.

Tested only: the shared lister `gate::discover::evidence`, which every
evidence reader (gate receipts, statements, covers, delivery checks,
observations, stale-checker receipts) now uses, and which turns a partial
listing into one invalid row; `tests/handoff.rs` and
`tests/gate_inventory_guard.rs` replace each root with a file.

## Behaviour

```
$ writfold kb handoff check --policy origin/main --candidate HEAD --run run-…
handoff quality [refused] definition-changed: the candidate's receipt was made under a definition that differs from .writfold/gates/quality.toml at origin/main (3f2a…); next: run the policy's definition: writfold kb gate run quality …
handoff unit [accepted] satisfied at candidate 9c1e… (receipt runs/…/gates/unit-….json)
# note: judged candidate 9c1e04… against policy origin/main = 77ab10…; required: quality, unit (.writfold/handoff.toml)
# coverage=complete … refused=1
```

Exit 0 only when accepted; 2 when refused (an answer: the work is not yet
acceptable); 1 on a usage or read error. JSON as for every verb. A
`handoff.toml` (`schema = "writfold-handoff/1"`, `required = ["quality",
"unit"]`) that does not parse is refused by the check with the reason;
`kb lint` support is a follow-up.

What counts as evidence: a receipt of the run (default: the current run)
whose definition digest equals the approved definition's and whose content
digest equals the candidate commit's scoped content. Because receipts bind
digests, not commits, a receipts-trailer-only commit after the gated commit
keeps the evidence current while its scope is unchanged; the output names
both the candidate commit and the commit each receipt was taken at.

Trust: a receipt is a file the candidate's author could write. The check
is as independent as the run it reads: a receiver that wants independence
runs `gate run` itself, with the policy revision's definitions, into its own
run, then calls `handoff check --run <its run>`. The check never runs a
command, so it never replays candidate-supplied argv. Deployment recipes
(pre-receive, required status checks, bypass roles) and attested receipts
are #64.

changes.md: new verb only; nothing existing changes.

## Design

1. Resolve `--policy` and `--candidate` to commits in the subject
   repository (`git rev-parse --verify REV^{commit}`).
2. `git show POLICY:.writfold/handoff.toml` → required IDs; missing → refuse
   `no-required-set`; empty list → kernel `Empty`.
3. `git ls-tree POLICY .writfold/gates/` and `git show` each required
   `<id>.toml` → approved `Definition` and its digest; a Git error listing
   the directory → `complete = false` (#56's rule); a required ID with no
   file → `Undefined`.
4. Candidate snapshot: `ls-tree CANDIDATE`, no dirty paths; scoped digest
   per approved definition's patterns.
5. Run receipts for each ID → `writfold_core::gate::judge(receipts,
   approved_digest, candidate_digest)` → `State`; statements → `Standing`
   (raw, without `accept`). `approved` is "some receipt was made under the
   approved digest".
6. `handoff::strict(complete, items)` → verdict; render each item.

## Compatibility and reversibility

A new verb and an optional new file. An older binary ignores
`handoff.toml`. Removing the verb removes nothing else.

## Decision log

- 2026-11-21: implemented for v0.6.0-dev.14. Kernel `writfold_core::handoff`
  proved in `proofs/Writfold/Handoff.lean` (`strict_accepted_iff`,
  `statement_never_passes`, `strict_refused_names_lowest`,
  `failing_member_refused`; `append_monotone` is stated as
  `failing_member_refused`, since appending to an *empty* set can of course
  turn `Empty` into acceptance); mutants `item-ignores-approved`,
  `first-failing-skips-last`, `strict-empty-ok` calibrated. Verb in
  `crates/writfold-kb/src/handoff/`; receiver tests in
  `crates/writfold-kb/tests/handoff.rs` (accepted, receipts-only commit,
  no evidence, waiver, stale, weakened definition, omitted ID, empty set,
  no required set, skipped plan step). Only subject-repository gates can be
  required (a gate in the KB root has no policy revision of the subject).
- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner,
  21 November: proceed, review and amend later), after the plan review
  posted on #57; deployment recipes and attested receipts split to #64.
- 2026-11-21: amended for #67 (evidence read in full); implemented for
  v0.6.0-dev.15.
