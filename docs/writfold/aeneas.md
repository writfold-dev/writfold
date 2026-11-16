# Proofs over the shipped Rust (Aeneas)

Owner decision, 9 November 2026: Writfold, Weft and Splicery prove their pure
decision kernels **over the shipped Rust**. Charon and Aeneas translate the
kernel crate to Lean 4, the translation is committed, and the theorems are
stated about the translated functions. There is no hand-written Lean copy of
the code: production calls exactly the code that was translated.

This complements, and does not replace, the existing layers:

| Layer | Role | Where |
|---|---|---|
| **Aeneas proofs** | **The shipped kernel meets its spec, for every input** | `crates/writfold-core`, `proofs/` (Lean v4.35.0) |
| TLA+ | Operational interleavings | `spec/KbWrite.tla`, `spec/RunGraph.tla` |
| Lean calculus (E01–E12, T01–T22) | The abstract model of the earlier edit flow | archived (16 November 2026) |
| Verus, Lean-exported cases | Proved domain bodies; the earlier kernels cross-checked against the calculus | archived (16 November 2026) |

## What is proved today

`crates/writfold-core` (no dependencies, `no_std` + `alloc`) holds the KB's
run-graph and gate kernels (below, since 13 November 2026) and the byte
identities they share. Over its translation
(`proofs/Extracted/WritfoldCore.lean`), `proofs/Writfold/` proves, as
total-correctness Hoare triples (terminates, never fails, result as stated):

- `same_spec`, `same_option_spec`, `contains_digest_spec`: byte identities are
  exact equality and digest lookup is membership.
- The run graph, gate obligations and the assessed gate's basis (below).

The gate (`docker compose -f compose.proofs.yaml run --build --rm proofs`)
regenerates the translation and refuses drift from the committed copy, builds
the proofs, audits every theorem's axioms (only `propext`,
`Classical.choice`, `Quot.sound`; the fifty-four headline claims must exist)
and calibrates thirty-three semantic mutants of the Rust source, each of which
must break exactly its named theorem. The earlier twenty-mutant warm run took
about four and a half minutes (16 November 2026). The semantic spike's
component results and translator limitation are recorded below.

The ledger, grant-authority, trial and edit-flow kernels of the earlier edit
trial were proved here too until 16 November 2026; they moved to the private
archive with the trial flow.

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

Nineteen mutants calibrate them (the twentieth drops the last byte from `same`): `closed` ignoring `skipped`; `advance_check`
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

### Semantic retrieval decisions (spike, 16 November 2026)

The off-by-default spike is not adopted; see
[semantic-search-spike.md](semantic-search-spike.md). Its new kernels were
extracted and their Lean build and axiom audit passed **before** the KB
adapters were switched to them. Numeric embedding and cosine scanning remain
tested-only.

`semantic_order` (`proofs/Writfold/SemanticOrder.lean`) supplies stable fusion
keys, not an unsafe mixed score/ordinal comparator:

- `semantic_wins_spec`: exact cross-multiplied RRF (`k=60`) comparison for
  ranks at most 50; `invalid_rank_refused` excludes overflow/insertion for
  oversized inputs; `rrf_tie_lexical` keeps equal scores lexical-first.
- `semantic_slot_spec`: the insertion slot is after the last lexical score
  blocker or exact match; `lexical_first` appends. `before_spec` and
  `key_transitive` establish the strict key order used by the adapter.
- `lexical_order_preserved`, `exact_protected` and `semantic_ties_stable`
  preserve lexical ordinals, keep semantic-only hits after exact hits, and
  retain semantic rank/ID order within a slot.
- `leading_exact_first` makes an exact hit first **when it leads the lexical
  input**. Existing `--signatures` priority is preserved; this is not a false
  unconditional claim that exact outranks a preceding signature.
- `rrf_insertion_enabled` is a positive control: stable RRF can interleave a
  semantic hit where lexical-first would append it.

`semantic_evidence` (`proofs/Writfold/SemanticEvidence.lean`) distinguishes a
raw retrieval pointer from an independently checked source:

- `semantic_never_grounds`, `lexical_never_grounds`, `checked_preserved`,
  `accepted_iff` and `grounded_iff`: only a checked source retains its actual
  verdict; retrieval cannot promote even a proposed `Grounded` result.
- `semantic_assumption_ungrounded` and `semantic_premise_never_meets` compose
  the existing assumption, basis and assessed-gate kernels.

The KB's typed `EvidenceInput` acceptance boundary invokes that decision
before a raw pointer can be parsed or checked. Independently checked source
citations keep their existing meaning, even when the source was discovered
semantically. The adapter regression checks a fresh citation and a passing
gate: either grounds as a checked reference, neither as a retrieval pointer;
neither pointer supports a checklist item or meets an assessed gate.
Source classification, parsing, IO and Rust sorting are tested boundaries,
not additional Lean claims.

Observed proof checks: extraction drift passed; `lake build` passed; the
axiom audit passed for 119 theorems and 54 required claims, with only the
standard three axioms. All 33 current mutants are calibrated across two
runs. The full `proofs/gate.sh` invocation rejected 32 and failed one because
Aeneas could not translate `slot = index` in the loop. That failed run remains
failed. The equivalent off-by-one mutation `slot = next - 1` translated and
was rejected by `semantic_slot_loop_spec` in a targeted rerun. No translation
error was counted as a proof rejection, and no fresh all-in-one gate pass is
claimed. Logs are named in `target/semantic-spike-report.md`.

The thirteen added mutants cover retrieval admission, checked-negative
bypass, RRF equality/weight, exact protection, insertion position,
lexical-first behavior, key ordering, collapsed lexical/semantic keys and
invalid-rank admission. Their expected failing theorems are registered in
`proofs/mutants.py`; the audit requires the headline claims.

## Trust base

The Charon/Aeneas translation of the Rust (pinned: Charon `8cae8fd`, Aeneas
`nightly-2026.11.07-c5eb4a2`), the Aeneas library's models of the Rust
operations used, the Lean kernel and the three standard axioms.
`writfold-kb`'s plan fold, gate judging and receipt reading, which gather the
facts the run-graph and gate kernels decide on, are tested, not proved.

## Found by the proofs

In the archived ledger kernel, the sequence check could overflow for a
selection of `usize::MAX` entries, and the inventory epoch increment could
overflow; both were rewritten to refuse instead.

## Not yet proved

- `writfold_core::scope` (glob matching), `verdict` and `walk`: closures,
  iterators and generic maps that the translation does not take; tested.
- The later kernels (`investigation`, `checklist`, `guard`, `decision`,
  `observe`, `delivery`, `command`): tested exhaustively, not proved.

## Writing kernels for translation

Owned data and no nested borrows; no derives in translated modules (equality a
theorem relies on is an explicit `same`); no integer casts; explicit `while`
loops; one loop per function when it returns early (Aeneas does not translate a
`return` inside nested loops); conveniences outside
`[package.metadata.charon] start_from`. A module that must keep derives for
its callers (`gate`, `assess`) is named function by function in `start_from`,
so the derives are never reached; inside a translated function, compare
enums with `matches!`, not a derived `==`, and keep a loop's early-return
match flat (Aeneas refuses to expand a nested enum pattern there).
