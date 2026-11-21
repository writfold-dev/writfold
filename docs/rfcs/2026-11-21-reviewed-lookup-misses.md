---
rfc: "2026-11-21-reviewed-lookup-misses"
title: "Turn a reported lookup miss into a reviewable alias"
status: accepted
implementation: not-started
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/22
milestone: "v0.6.0"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Turn a reported lookup miss into a reviewable alias

## Proof obligations first

Extend the [find-next](2026-11-17-find-next.md) `writfold_core::find::propose` design: inputs are the actual ordered first five result IDs (not suggestions), the canonical ID the caller subsequently identified, whether the target is eligible for the aliases key, and whether the phrase already resolves exactly to a record. Return `Offer` or `NoOffer`, not a write operation.

- `proposal_only_after_miss`: an offer implies the named target was absent from the first five actual hits. A mutant checking only the first hit must break it on a rank-five target.
- `no_proposal_when_phrase_names_another`: no offer for a phrase already resolving exactly, including an ambiguous exact match. Dropping the collision check must fail the theorem.
- `offer_iff`: offer exactly for an eligible, identified target absent from the five, with no exact resolution. Mutants accepting unsupported targets or always refusing must fail. A suggestion is not evidence of either a hit or an identified target.
- At acceptance, `find::accept_alias(exact_owner, eligible_now, target_pin_matches) → Apply | AlreadyApplied | Refuse`. `apply_iff`: only an unclaimed phrase, still eligible under `propose`, on the unchanged target can be written; mutants ignoring the pin or recheck must fail. `retry_iff`: only exact ownership by this target alone, with the alias already present, returns `AlreadyApplied`; conflicting or ambiguous ownership always refuses. Mutants treating a collision as a retry must fail. `exact_owner` distinguishes an alias already present on this target from any other exact name. Pin the target bytes at proposal time; enforce the write precondition under the existing lock.

Prove on extracted Rust; drift check, `lake build`, axiom audit and mutants before wiring. Tested, not proved: loading ranked hits with the real lookup options, resolving IDs/aliases, queue codec and permissions, output, and the atomic edits. Use the existing write protocol; no independent authorisation to commit or push.

## Motivation

[#19](https://github.com/writfold-dev/writfold/issues/19) supplied `find --eval` and a control set, not real misses. [#41](https://github.com/writfold-dev/writfold/issues/41) supplies an aliases key, not evidence that a particular alias helps. [#22](https://github.com/writfold-dev/writfold/issues/22) needs a way to connect the phrase someone actually tried to the record they eventually cited, without retaining every search or attaching one session's miss to another session's record. This draft refines the input/retention boundary; it does not reopen the accepted lexical plan or authorise the deferred semantic tier.

## Proposed behaviour

After finding the record, the caller explicitly reports the pair, for example `writfold kb propose alias HBR-101 --phrase 'slow ledger flush'`. Ordinary `find`, hooks, `cite make` and checkpoints neither record phrases nor infer which earlier lookup a citation answered. No transcript import. The command looks up the phrase again: this is a **reported miss, reproduced against the current KB**, not a claim about a previous revision. Reproduce an ordinary unscoped `find` with five actual hits; exclude suggestions. A scoped, context-specific or no-longer-reproducible miss is not an automatic alias candidate.

A qualifying pair enters the local proposal queue, never the record. Report the canonical target, phrase, observed KB pin, and first-five IDs in `proposals show`. A nonqualifying pair explains why and queues nothing (exit 2); an unknown or ambiguous target is refused (exit 3). Preserve existing usage and I/O errors. Repeating the same target/phrase is idempotent. Do not silently publish raw queries via `.writfold/proposals/.tracked`: these proposals are local-only, including when that marker exists; reject `--tracked` for this form. Existing proposal kinds keep their current behaviour.

`proposals accept` rechecks eligibility and collisions against the current KB, and rechecks the target bytes on the normal write path. It edits only the target's existing alias representation from #41, preserving all other fields. Changed eligibility, a changed target or a new conflicting alias refuses without modifying either record or queue. If a retry sees this exact alias already on this target only, report it already applied and clear the pending entry without duplicating the alias. `drop` changes no record. Neither path edits the evaluation set.

## The miss log is a reviewed input, not telemetry

Use the existing `find --eval` JSONL shape: `{"query":"slow ledger flush","expect":["HBR-101"],"note":"real miss: caller identified target"}`. Show a copyable row for a reported pair. Saving a row to a private eval file or committing a redacted row to `.writfold/queries.jsonl` is a deliberate, separate human action; there is no new tracked history format or automatic write. The pending proposal is clone-local and removable; even unaccepted reported misses can be kept in the reviewed file. Old binaries can evaluate these rows. Never equate training on an accepted phrase with an independent ranking improvement: report held-out wording separately. #27 still requires real remaining meaning-only misses and a separate owner decision.

## Alternatives, tests and rollout

Automatically joining the last lookup to the next citation is simpler for the caller but wrong with parallel sessions and creates a private search history. Editing aliases directly loses the refusal/review step. The manual fallback remains valid: keep reviewed eval rows and edit the aliases field in Git.

After #41 and owner acceptance: prove the kernel; exercise rank five vs six, an ambiguous alias, and a suggested/non-hit row through the real query adapter; then exercise two concurrent sessions, a target changed since proposal, an existing tracked-queue marker, retry after write and before queue removal, and loss of the local queue in a second clone. The original query must not appear in tracked files before explicit acceptance or deliberate eval-file editing. Document the new call and exit outcomes when actually exercised. No benchmark pilot or dependency. Mining body acronyms or mention lines is a separate input, not a claimed result of this design.

- 2026-11-21: draft for owner review. Is explicit reporting plus opt-in retention sufficient, instead of passively recording searches? No format or new verb accepted here.

## Decision log

- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner, 21 November: proceed, review and amend later). #4 and #41 have landed on main (v0.6.0-dev.12).
