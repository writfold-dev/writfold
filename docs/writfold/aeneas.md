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

`crates/writfold-core` (no dependencies, `no_std` + `alloc`) holds the ledger
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
`Classical.choice`, `Quot.sound`; the twenty-two headline claims must exist) and
calibrates twenty-six semantic mutants of the Rust source, each of which must break
exactly its named theorem. Warm run: about three minutes.

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

## Trust base

The Charon/Aeneas translation of the Rust (pinned: Charon `8cae8fd`, Aeneas
`nightly-2026.11.07-c5eb4a2`), the Aeneas library's models of the Rust
operations used, the Lean kernel and the three standard axioms. The domain
adapters in `writfold-kernel::ledger`, `writfold-kernel::authority` and
`writfold-kernel::trial` (nominal types to bytes, validator adaptation, canonical scope bytes) are
tested, not proved; the Lean-exported ledger and grant cases still pass
through them and the core.

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
- Next kernel: procedure admission.

## Writing kernels for translation

Owned data and no nested borrows; no derives in translated modules (equality a
theorem relies on is an explicit `same`); no integer casts; explicit `while`
loops; one loop per function when it returns early (Aeneas does not translate a
`return` inside nested loops); conveniences in `convenience.rs`, outside
`[package.metadata.charon] start_from`.
