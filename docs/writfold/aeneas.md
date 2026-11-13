# Proofs over the shipped Rust (Aeneas)

Owner decision, 9 November 2026: Writfold, Weft and Splicery prove their pure
decision kernels **over the shipped Rust**. Charon and Aeneas translate the
kernel crate to Lean 4, the translation is committed, and the theorems are
stated about the translated functions. There is no hand-written Lean copy of
the code: production calls exactly the code that was translated.

This complements, and does not replace, the existing layers:

| Layer | Role | Where |
|---|---|---|
| Lean calculus (E01–E12, T01–T22) | The specification: system theorems about the abstract model | `formal/` (Lean v4.21.0) |
| **Aeneas proofs** | **The shipped kernel meets its spec, for every input** | `crates/writfold-core`, `proofs/` (Lean v4.35.0) |
| Verus | Proved bodies inside the domain crate (identities, quantities, conversions) | `qualification/verus-profile.json` |
| Lean-exported cases | Cross-check of the shipped kernel against the calculus itself | `crates/writfold-wire/tests/*_lean.rs` |
| TLA+ | Operational interleavings | `spec/` |

## What is proved today

`crates/writfold-core` (no dependencies, `no_std` + `alloc`) holds the KB's
run-graph and gate kernels (below, since 13 November 2026) and the ledger
and grant-authority kernels that `writfold-kernel::ledger` and
`writfold-kernel::authority` now delegate every decision to. Over its
translation (`proofs/Extracted/WritfoldCore.lean`), `proofs/Writfold/` proves,
as total-correctness Hoare triples (terminates, never fails, result as stated):

- `same_spec`, `same_option_spec`, `contains_digest_spec`: byte identities are
  exact equality and digest lookup is membership.
- `entry_ok_spec`, `history_check_spec`: `history_check` holds exactly when
  every entry is on the stream, numbered from 1 without gaps, linked to its
  predecessor and valid against exactly the entries before it (Lean
  `historyCheck`), for any terminating validator.
- `head_matches_spec`, `predecessor_ok_spec`, `material_ok_spec`,
  `retains_record_spec`, `find_spec`: each `AppendReady` conjunct.
- `ready_against_spec`: the result is exactly `readyOutcome`, i.e. Lean
  `AppendReady` conjunct by conjunct **in the calculus's refusal order**;
  `ready_ok_iff`: an append is admitted exactly when every conjunct holds.
- `Store.ready_spec`: `Store::ready` is that decision over the stream's current
  selection; `Store.append_refused`: when `ready` refuses, `append` returns that
  refusal and **the store is unchanged**.

Grant authority (E12), mirroring the calculus's `installGrant`, `currentGrant`
and the grant conjunct of `intentReady`:

- `install_check_spec`: installation is exactly `installOutcome`, the guards
  in the calculus's order (realm, binding realm, policy, retained envelope,
  fresh id including revoked ids, not yet valid, expired);
  `Authority.install_refused`: a refused installation changes nothing.
- `live_spec`, `current_spec`: the current grant is the newest grant for
  exactly this scope that is unrevoked and inside its window, and a newer grant
  shadows older ones even when it covers less.
- `covers_spec`: coverage holds exactly when that current grant carries the
  capability and every requested resource.
- `Authority.revoke_spec`: revocation marks exactly the grants with that id
  revoked and changes nothing else (no grant added, removed, reordered or
  un-revoked; realm, policy and clock untouched); an unknown id changes
  nothing. It is never refused.
- `Authority.advance_spec`: the clock never moves backwards.

The gate (`docker compose -f compose.proofs.yaml run --build --rm proofs`)
regenerates the translation and refuses drift from the committed copy, builds
the proofs, audits every theorem's axioms (only `propext`,
`Classical.choice`, `Quot.sound`; the fifty-four headline claims must exist) and
calibrates forty-five semantic mutants of the Rust source, each of which must break
exactly its named theorem. Warm run: about twenty minutes (13 November 2026, on a shared host), almost all of it mutant calibration.

Trial edit flow (`writfold_core::trial`, adapter `writfold-kernel::trial`):

- `changes_spec`: a candidate changes its source exactly when the bytes
  differ; `prepare` refuses one that does not.
- `recovery_spec`, `recoveryOutcome_unambiguous`: an interrupted apply is
  reconciled from the observed bytes alone. The outcome is `Applied` exactly on
  the candidate's bytes and `Abandoned` exactly on the source's (unambiguous
  because the two differ); anything else is `Unknown`.
- `required_entailed_spec`, `required_ok_iff`: an approval's required Weft
  checks pass exactly when each is present and every check of that name is
  `entailed`; otherwise the first unsatisfied requirement is named, as
  missing or not entailed. A later entailed check never hides an earlier
  refutation.

Trial edit-flow validator (`writfold_core::flow`, adapter
`writfold-kernel::trial::edit_flow`, called by the app's ledger `Flow`):

- `validate_spec`: a record is accepted exactly when it is `Valid`: it
  carries every blob it names in its closure (`carried_spec`) and it
  `Follows` the run so far (`follows_spec`): `Prepared` with a non-empty path
  on an empty run, `Approved` after it, `Claimed` after those two naming
  exactly the prepared candidate (digest and length, `same_blob_spec`), then
  `Applied` naming that same candidate or `Abandoned` with a non-empty reason.
- `follows_closed`: nothing follows a four-record run, so a run has at most
  one outcome and no second claim.

Its six mutants drop the length from blob identity, the report from the
material check, the material check from the rule, the claim's candidate
binding and the abandonment reason, and admit `Applied` after an outcome.
The interleavings around this stream (apply, recover, an external writer,
crashes, the ledger CAS and authority timing) are model-checked separately
in `spec/TrialEditStream.tla` (TLC).

### The run graph and gate kernels (13 November 2026)

The KB's own decisions, as the mission's "The run graph, hooks and order of
work" asked: `writfold_core::plan` (behind `writfold kb plan`), the decisions
of `writfold_core::gate` (behind `writfold kb gate` and the met/owed answer
`plan advance` sees) and the assessed gate's basis in `writfold_core::assess`
(design §23). `gate` and `assess` are named function by function in
`start_from` (`gate::outcome`, `gate::state`, `gate::standing`,
`gate::Standing::met`; `assess::assumption`, `basis`, `met`, `unfinished`,
`found_met`, `outcome`), so their derived trait impls and `name`/`parse` text
conveniences are never translated. Glob matching moved out of `gate` into
`writfold_core::scope` (iterators, closures, recursion over string slices):
it is not translated and not proved.

Run graph (`proofs/Writfold/Plan.lean`, the caller's folded list of steps):

- `current_spec`, `current_least`: `current` is the least index whose step is
  neither done nor skipped; `None` exactly when every step is closed.
- `advance_check_spec`: the result, refusal by refusal in the fixed order:
  `Complete` exactly when there is no current step; then `NotCurrent(now)`
  naming the current step when it is not the one asked; then `Owed(g)` naming
  the first gate not met. `advance_ok_iff`: admitted exactly when the step is
  current and every gate is met.
- `stale_spec`: `stale` is a done step with a gate not met.
  `stale_never_current`: a stale step is never the current step (`current`
  reads only `done`/`skipped`, never a gate), so `!stale` never moves the
  current step back.
- `splice_spec`: `splice(nodes, at, count)` is
  `take at ++ count open steps ++ drop at` (for lists whose new length fits in
  `usize`; `Vec::push` fails past that in the model).
  `splice_keeps_existing`: removing the inserted block gives back the list, so
  every existing step keeps its state and order. `splice_keeps_prefix`: the
  steps before the insertion point are unchanged. `splice_new_open`: the new
  steps are open. `splice_current`: inserted before the current step (or at 0
  into an empty list), the first new step becomes current.
- `splice_check_spec`: the decision in its fixed order (already spliced; then
  the depth bound; then a root splice into an empty list; then before the
  current step, or `Complete`). `splice_check_already_iff`: `Already`
  exactly when the key is among the keys spliced at the current step or its
  ancestors; `splice_idempotent`: so splicing a recorded key again changes
  nothing, whatever else holds. `splice_check_depth`: an admitted insertion
  is never deeper than `MAX_DEPTH` (4). `splice_admitted` composes them: an
  admitted splice of at least one step makes its first step current and keeps
  the done prefix and every existing step.

Gate obligations (`proofs/Writfold/Gate.lean`):

- `outcome_pass_iff`, `outcome_fail_iff`: only exit status 0 on a subject that
  did not move while the command ran is a pass; any other exit status on an
  unmoved subject is a failure; every other ending is unknown.
- `state_spec`: the deciding receipt is the newest about the current
  definition and the current content in scope, with the state its outcome
  gives; with none, the newest receipt decides and the state is `stale`;
  with no receipt, `open`. `state_satisfied_iff`: `satisfied` exactly when
  that newest current receipt passed. `state_other_subject`: receipts about
  another definition or other bytes never satisfy.
- `standing_satisfied_iff`: `Checked(Satisfied)` exactly when the obligation
  is not of the attested class and its checked state is `satisfied`; no
  attestation or waiver ever yields it. `standing_attested_iff`,
  `standing_waived_iff`: each statement stands as itself, and only when
  current. `met_spec`, `standing_met_iff`: met is exactly satisfied, attested
  or waived.
- `satisfied_only_by_current_pass`: whatever statements exist, an obligation
  derived from its receipts stands `Checked(Satisfied)` only when it is a
  checked obligation and the newest receipt about the current subject passed.

Assessed gate (`proofs/Writfold/Assess.lean`):

- `assumption_spec`: an assumption is ungrounded when it cites nothing or a
  citation is ungrounded, else attested when a citation is only attested,
  else grounded. `basis_spec`: a check's basis is grounded exactly when no
  premise is ungrounded. `met_spec`: a check meets the gate exactly when it is
  entailed on a grounded basis.
- `outcome_spec`, `outcome_pass_iff`: unknown when the run was incomplete,
  nothing was required, or Weft could not finish a required check; otherwise
  a pass exactly when every required check is entailed on a grounded basis.
  `hypothetical_never_passes`: a hypothetical entailment is never a pass, so
  an assessed gate's state is never `satisfied` from one.

Nineteen mutants calibrate them: `closed` ignoring `skipped`; `advance_check`
without the current-step check or ignoring gates; `stale` ignoring `done`;
`splice` inserting after the current step, inserting done steps or dropping the
last step; `splice_check` without the key check or with the depth bound off by
one; `outcome` ignoring a moved subject or passing any exit status; `state`
ignoring the scope digest; a waiver or an attestation standing as satisfied;
`met` counting a stale obligation; `basis` ignoring premises; `met` accepting a
hypothetical basis; `outcome` passing an unfinished check; `assumption`
reporting grounded when a citation is only attested.

Two small rewrites made these translatable without changing behaviour (the
kernel's and `writfold-kb`'s tests are unchanged and pass): equality on derived
enums in `standing`, `basis` and `met` is written `matches!`, and
`assess::outcome`'s nested pattern with an early return is two helpers
(`unfinished`, `found_met`) and two flat `if`s. `plan::same` was a copy of
`bytes::same`; `plan::contains` now calls `bytes::same`, so `same_spec`
covers it.

What these proofs do not cover: the fold from event files to the list of
steps, template and definition parsing, digests, Git and file reads, the
write protocol (`spec/KbWrite.tla` and the kill tests), scope globs
(`writfold_core::scope`), citation verdicts (`verdict`) and the bounded walk
(`walk`). Those are tested. The adapters in `writfold-kb` that turn files into
kernel inputs (which receipts are valid, which digests match, which gates a
step names, the ordering by `seq` and file name) are tested, not proved; the
proofs take their output as given.

## Trust base

The Charon/Aeneas translation of the Rust (pinned: Charon `8cae8fd`, Aeneas
`nightly-2026.11.07-c5eb4a2`), the Aeneas library's models of the Rust
operations used, the Lean kernel and the three standard axioms. The domain
adapters in `writfold-kernel::ledger`, `writfold-kernel::authority` and
`writfold-kernel::trial` (nominal types to bytes, validator adaptation, canonical scope bytes) are
tested, not proved; the Lean-exported ledger and grant cases still pass
through them and the core. Likewise `writfold-kb`'s plan fold, gate judging
and receipt reading, which gather the facts the run-graph and gate kernels
decide on.

## Found by the proofs

The sequence check `record.sequence != entries.len() + 1` could overflow for a
selection of `usize::MAX` entries; it is now written so that it cannot. The
inventory epoch increment is checked and refuses instead of overflowing.

## Not yet proved

- The success shape of `append` (the record is selected, material retained):
  it needs capacity preconditions because `Vec::push` can fail at
  `usize::MAX` elements in the model.
- The spec predicates mirror the calculus's `historyCheck`/`AppendReady` but
  live in the proofs project; machine-linking them to `formal/` needs the
  calculus on the same Lean version.
- `writfold_core::scope` (glob matching), `verdict` and `walk`: closures,
  iterators and generic maps that the translation does not take; tested.
- Next kernel: procedure admission.

## Writing kernels for translation

Owned data and no nested borrows; no derives in translated modules (equality a
theorem relies on is an explicit `same`); no integer casts; explicit `while`
loops; one loop per function when it returns early (Aeneas does not translate a
`return` inside nested loops); conveniences in `convenience.rs`, outside
`[package.metadata.charon] start_from`. A module that must keep derives for
its callers (`gate`, `assess`) is named function by function in `start_from`,
so the derives are never reached; inside a translated function, compare
enums with `matches!`, not a derived `==`, and keep a loop's early-return
match flat (Aeneas refuses to expand a nested enum pattern there).
