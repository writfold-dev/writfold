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
decision kernels. Over the translation of the functions named in
`[package.metadata.charon] start_from` (`proofs/Extracted/WritfoldCore.lean`),
`proofs/Writfold/` proves, as total-correctness Hoare triples (terminates,
never fails, result as stated):

- `same_spec`: byte identity is exact equality (`Bytes.lean`). `same` is
  reached in production through `plan::contains` and the digest comparisons
  of `gate::about` and `gate::evidence`. (`same_option` and
  `contains_digest`, left from archived kernels with no caller, were
  deleted on 16 November 2026.)
- The run graph, gate obligations and the assessed gate's basis (13 November
  2026, below).
- The proof backfill's first round (16 November 2026, below): the digest
  comparisons that make a record about the present and the advance on the
  gates as found (`Current.lean`), checklist gates (`Checklist.lean`),
  deliveries and the delivery guard (`Delivery.lean`), guard rules and their
  requirements' facts (`Guard.lean`), and what grounds evidence
  (`Ground.lean`).
- The policy round (16 November 2026, below; built in v0.6.0-dev.3): a
  team's `accept` list, pins and drift, how a step closed and how a plan
  ended (`Policy.lean`); the checklist's `not_applicable` and `none_found`
  (`Checklist.lean`, replaced in place); `plan skip`'s lock and the close
  check (`Plan.lean`); evidence bound to its run and subject in
  `gate::state` (`Gate.lean`, replaced in place).
- The proof backfill's second round (17 November 2026, below; built in
  v0.6.0-dev.4): scope globs (`Wild.lean`, `Scope.lean`), the plan writes'
  re-check under the lock and the closed run (`Recheck.lean`), decision
  states and investigations (`Decision.lean`), citation verdicts and check
  receipts (`Verdict.lean`, `Recorded.lean`), walk's claim, the lint's
  verdict, `find`'s prefix tier and rows, command matching and idle runs
  (`Walk.lean`, `Command.lean`, `Find.lean`, `Idle.lean`), and the order
  receipts are judged in (`Order.lean`).
- The bug-fix round (17 November 2026, below; fixed in v0.6.0-dev.4):
  which repository a gate judges (`Subject.lean`) and `gate lines`
  agreeing with the plan's word (`Policy.lean`, extended in place),
  and when `find`'s rows are full (`Walk.lean`).
- The delivery round (17 November 2026, below; built in v0.6.0-dev.4):
  each delivery obligation met under its own policy, which candidate
  decides once candidates are withdrawn or bound, the `posted:<mode>`
  standing, and the delivery guard's decision over the run's candidates
  (`Delivery.lean`, replaced in place and extended).
- The current-run round (17 November 2026, below; built in v0.6.0-dev.6):
  which of a clone's three current-run pointers decides for a session and
  work tree, and which are written when a run is made current
  (`Pointer.lean`).
- The repin round (17 November 2026, below; built in v0.6.0-dev.6): which
  pin entry is in force and whether a run may adopt a changed policy
  (`Policy.lean`, extended in place).
- The evidence-binding round (18 November 2026, below; kernels only, not
  yet wired): whether a cover row's references answer the item they cover
  (`Bind.lean`) and whether evidence still holds under its pins: what a
  look searched, a catalog lifetime, another repository (`Pin.lean`).
- The tauten claims round (19 November 2026, below): `holds_from` and
  `answered_from` never fail from any start index, and mutants for the
  loops no mutant broke, so `cargo tauten proofs` can claim them.

The gate (`docker compose -f compose.proofs.yaml run --build --rm proofs`,
or the `docker run` form below) regenerates the translation and refuses
drift from the committed copy, builds the proofs, audits every theorem's
axioms (only `propext`, `Classical.choice`, `Quot.sound`; the required
claims in `proofs/Audit.lean` must exist, and every theorem a mutant names
must be one of them) and calibrates the semantic mutants in
`proofs/mutants.py`, each of which must break exactly its named theorem.

### Running the gate without a Compose network

`docker compose run` creates a project network; on a host short of networks,
build the image and run it with `--network none` (the image copies the
sources in at build time, so rebuild after every Rust or Lean edit):

```sh
docker build -f proofs/Dockerfile --target gate -t writfold-proofs:gate .
docker run --rm --network none writfold-proofs:gate            # the whole gate
# refresh proofs/Extracted (what proofs/refresh.sh does through Compose)
docker run --rm --network none writfold-proofs:gate \
  sh -c 'sh proofs/extract.sh crates/writfold-core /tmp/fresh >&2 && tar -C /tmp/fresh -cf - .' > /tmp/fresh.tar
rm -rf proofs/Extracted && mkdir -p proofs/Extracted && tar -C proofs/Extracted -xf /tmp/fresh.tar
# while iterating: one module, or named mutants
docker run --rm --network none writfold-proofs:gate sh -c 'cd proofs && lake build Writfold.Guard'
docker run --rm --network none writfold-proofs:gate python3 proofs/mutants.py guard-known-any-ready
```

To iterate without rebuilding, keep one container from the image and
`docker cp` the changed files into `/work`; do not bind-mount over
`/work/proofs`, which hides the prebuilt `.lake`. Run `cargo fmt` before
refreshing: the translation records line numbers.

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
`writfold_core::scope`; it was reshaped and proved in the second round
(below).

Run graph (`proofs/Writfold/Plan.lean`, the caller's folded list of steps):

- `current_spec`, `current_least`: `current` is the least index whose step is
  neither done nor skipped; `None` exactly when every step is closed.
- `advance_check_spec`: the result, refusal by refusal in the fixed order:
  `Complete` exactly when there is no current step; then `NotCurrent(now)`
  naming the current step when it is not the one asked; then `Owed(g)` naming
  the first gate not met. `advance_ok_iff`: admitted exactly when the step is
  current and every gate is met. As wired, `plan advance` asks about the step
  `current` returns, so the `NotCurrent` branch does not occur there and what
  decides is the `met` vector; since 16 November 2026 that vector is built in
  the kernel from the gates as found (`plan::advance`, below).
- `stale_spec`: `stale` is a done step with a gate not met. That `!stale`
  never moves the current step back follows from `stale_spec` and
  `current_spec` (`current` reads only `done`/`skipped`, never a gate);
  `stale_never_current` states it, and is kept as a lemma, not a required
  claim, because it would hold of any `stale` that implies `done`.
- The splice model. `plan use` does not call `plan::splice`: it places the
  new steps in the fold (`writfold-kb` `plan/fold.rs`), and a test compares
  the fold with `splice` step for step. So the theorems below are about the
  model the fold is tested against, not about the product's placement.
  `splice_spec`: `splice(nodes, at, count)` is
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

The mutants that calibrate them are listed with every other in "The claims
and their mutants" below.

Two small rewrites made these translatable without changing behaviour (the
kernel's and `writfold-kb`'s tests are unchanged and pass): equality on derived
enums in `standing`, `basis` and `met` is written `matches!`, and
`assess::outcome`'s nested pattern with an early return is two helpers
(`unfinished`, `found_met`) and two flat `if`s. `plan::same` was a copy of
`bytes::same`; `plan::contains` now calls `bytes::same`, so `same_spec`
covers it.

### The proof backfill, first round (16 November 2026)

[`proof-backfill-spec.md`](../rfcs/2026-11-16-proof-backfill.md) measured the distance to
"every product decision is proved" and ordered the work by risk; its steps
0 to 5 are built. Each moved a decision out of `writfold-kb` into a kernel
function named in `start_from`, with `writfold-kb` calling it and its tests
and golden files unchanged (one exception, below).

About the current subject (`proofs/Writfold/Current.lean`). Before, the gate
kernels received two booleans (`same_definition`, `same_scope`) computed in
`writfold-kb`; now they receive the recorded and present digests.

- `about_spec`: a record is about the present exactly when its recorded
  definition and content digests equal the present ones.
- `evidence_spec`, `judge_spec`: `gate::judge` is `state` over each receipt
  compared by `evidence`, where the content is the same only when the present
  content is not empty. `judge_satisfied_iff`: `satisfied` exactly when the
  newest receipt about the present definition and non-empty content passed.
  `judge_other_bytes`, `judge_empty_content`: receipts about other bytes, or
  unreadable present content, never satisfy.
- `stands_spec`, `statements_spec`: an attestation or a waiver stands
  exactly when one is about the present; `attested_any` counts every
  attestation.
- `met_only_when_current`: whatever receipts and statements exist, an
  obligation's standing is met only on a passing newest receipt about the
  present digests (checked class), an attestation about them (attested
  class), or a waiver about them.
- `fact_met_spec`, `met_all_spec`, `advance_facts_spec`: `plan::advance` is
  `advance_check` on each gate's `fact_met`, where a gate with no usable
  definition (`GateFact::Undefined`) is never met.
  `advance_facts_ok_iff`: admitted exactly when the step is current and
  every gate it names stands met; `advance_undefined_refused`.
- `done_only_when_met`: the assurance sentence end to end. Given that each
  standing gate's standing is the kernels' (`judge`, `statements`,
  `standing`) on its receipts, statements and present digests, an admitted
  advance means every gate of the step stands on evidence about the present
  digests, as in `met_only_when_current`.

Checklist gates (`proofs/Writfold/Checklist.lean`): `item_spec`,
`item_covered_iff` (covered exactly when the newest row is current and
supported by at least one grounded reference, or not applicable),
`rollup_spec`, `rollup_covered_iff` (covered exactly when every required
item is), `rollup_optional_irrelevant` (two checklists that agree on which
items are required and on those items roll up alike), `standing_spec`,
`standing_attested_iff` (attested exactly when covered),
`standing_never_satisfied`, `standing_met_iff` and `checklist_met_iff` (met
exactly when covered or a current waiver stands). The mapping onto the
gate kernel that `writfold-kb` used to write by hand is now
`checklist::standing`; a cover row's currency is `gate::about`.

Deliveries (`proofs/Writfold/Delivery.lean`): `deciding_spec`,
`standing_spec`, `standing_met_iff`, `standing_satisfied_iff`,
`standing_first_unmet`, `ready_iff`, `empty_never_ready`, `rechecked_spec`
(a passed check whose cited evidence moved is stale) and `guard_allow_iff`.
Their statements were replaced in place on 17 November 2026, when each
obligation gained its policy, a candidate could be withdrawn or bound, and
the guard's scans moved into the kernel; they are given with the delivery
round below.

Guard rules (`proofs/Writfold/Guard.lean`): `all_met_spec`, `decide_spec`,
`decide_pass_iff`, `unknown_never_passes`; `combine_spec`,
`combine_deny_iff`, `combine_pass_iff` with the model lemmas
`combineOf_comm`, `combineOf_assoc`, `combineOf_pass_left`; `effect_spec`,
`effect_audit_passes`, `effect_enforce`; `decide_all_pass_iff` (a call passes
exactly when every enforcing rule's requirements are met) and
`decide_all_deny_iff`; `logged_iff`; `gates_fact_met_iff`,
`gates_fact_never_unknown`; the requirements' facts `run_current_met_iff`,
`step_fact_met_iff`, `step_fact_unknown_iff`, `gate_fact_met_iff`,
`gate_fact_unknown_iff`, `gates_read_met_iff`; `nudge_spec`,
`nudge_with_iff`. `step_fact_met_iff` writes down an asymmetry that is
intended and now stated: `step.met` holds for a run with no plan, while
`gates.met` does not hold for a run that owes no gate.

What grounds evidence (`proofs/Writfold/Ground.lean`): `of_gate_spec`,
`of_gate_grounded_iff`, `of_gate_attested_iff`, `waiver_never_grounds` (a
gate that stands on a waiver grounds nothing), `of_receipt_grounded_iff` and
`of_receipt_never_attested`, `of_citations_grounded_iff`,
`of_token_grounded_iff`, `of_observation_grounded_iff`, `grounds_iff`,
`embeds_iff`, `of_statement_spec`, `moved_iff`. The standing words a
reference records stay in `writfold-kb`.

Reshapes, each with behaviour and tests unchanged: derived `==` and `!=` on
enums in `guard` are `matches!`; `combine`'s tuple match is two flat tests;
loops that update several accumulators call one small function per test
(`gate::stands`, `checklist::owes` and `stale`, `delivery::unmet`,
`is_waived`, `is_attested`, `guard::met`), because Aeneas could not join a
loop body whose nested `if`s update different variables; `checklist::rollup`
takes `&[Entry]` instead of a tuple slice; `delivery::standing` is a `while`
loop; the guard's call-level decisions are `guard::call` and the
requirements' facts `guard::require`.

### The policy round (16 November 2026)

[`policy-spec.md`](../rfcs/2026-11-16-user-defined-policy.md) section 3; built in v0.6.0-dev.3. Every
decision the round adds or changes was translated and proved, and the
proofs gate passed, before any verb called it.

A team's policy (`proofs/Writfold/Policy.lean`, `writfold_core::policy`,
translated whole):

- `met_spec`: a standing meets a gate exactly when it is satisfied,
  attested or waived and the gate's `accept` takes that standing.
  `met_default`: with all three accepted (a definition without `accept`)
  it is `Standing::met`, proved from the code of `policy::met` itself.
  `met_narrows`: a policy only narrows. `waiver_rejected_never_met`: a gate
  whose policy rejects waivers is never met by one, whatever statements
  exist.
- `pin_one_spec`, `pin_one_same_iff`, `pin_one_unpinned_iff`: a pinned
  digest is the same only when something is there now with the same bytes.
  `pin_all_spec`, `pin_all_moved_iff`, `pin_all_same_iff`: one moved pin
  moves the run; `same` needs something pinned and all of it unchanged.
- `close_spec`, `close_passed_iff`, `close_waived_iff`, `close_open_iff`:
  a step reads `passed` only when done with every gate passed; an
  attestation or a waiver is never read as a pass.
- `summary_spec` (the table of the policy spec's 2.6, row by row in
  order), `summary_complete_iff`, `moved_never_complete`,
  `open_never_complete`, `exception_never_clean`, `empty_never_complete`.

Replaced in place, with what changed:

- **Gate** (`Gate.lean`, `Current.lean`): `Evidence`, `Recorded` and `Said`
  carry `bound` (`bound_spec`: this run, this subject, intact). `isCurrent`
  gains the conjunct `bound`; `StateSpec`'s fallback ranges over bound
  receipts (`newest_bound_spec`), and with no bound receipt the state is
  `open`. `state_loop_spec`, `state_spec`, `state_satisfied_iff` keep their
  statements over the new `isCurrent`. `state_other_subject`'s statement
  changed: with no current receipt the state is `stale` when some receipt is
  bound, else `open` (it was `stale` when any receipt existed), because an
  unbound receipt now counts as absent. Added: `current_spec`,
  `state_unbound_never_satisfies`, `state_ignores_unbound` (the state of
  the bound receipts alone). `About` and `AboutS` gain `bound`, so
  `judge_*`, `stands_spec`, `statements_spec` and `met_only_when_current`
  hold with bound records only; `attested_any` counts bound attestations
  (`attests_spec`).
- **Advance** (`Current.lean`): `GateFact::Stands` carries the gate's
  `Accept` and `fact_met` is `policy::met`; `fact_met_spec`,
  `advance_facts_ok_iff` (met under the policy: `PMet`) and
  `done_only_when_met` (whose conclusion is unchanged, by `PMet.met`) are
  restated over it.
- **Checklist** (`Checklist.lean`): the `NoneFound` outcome and item;
  `covered_under_spec` replaces `covered_spec` and `rollup_under_spec`
  replaces `rollup_spec` (each entry covered under its own `na` and `none`);
  `item_covered_iff`, `rollup_covered_iff`, `rollup_optional_irrelevant`
  (two checklists must also agree on the required items' policies) and
  `checklist_met_iff` are restated over them. Added: `item_supported_iff`,
  `item_none_found_iff` (from the code of `item` itself),
  `not_applicable_rejected_never_covers`.
- **Plan** (`Plan.lean`, added, nothing else changed): `skip_check_spec`,
  `skip_ok_iff`, `skipped_spec`, `locked_skip_never_closes`,
  `locked_stays_current`, `close_check_spec`.

Not proved, and tested: the readers of `accept`, `not_applicable`,
`none_found`, `unskippable` and `waivable`; writing and reading pin files
and choosing the newest pin naming a gate; the fold from event and pin
files to `Node`, `locked`, `How`, `Close` and `Facts`; which statuses of an
observation count as a look that found nothing (`no-data`, `not-found`,
read in `writfold-kb`); the counts in the summary line; `kb init`,
`kb policy` and all rendering. `spec/RunGraph.tla` gains the lock:
`PlanSkip` is enabled only for an unlocked step, `LockedNeverSkipped` holds
in the positive configurations, and the mutant `SkipLocked`
(`RunGraph.mutant-SkipLocked.cfg`) violates it.

### The proof backfill, second round (17 November 2026)

[`proof-backfill-spec.md`](../rfcs/2026-11-16-proof-backfill.md) steps 6 to 11; built in
v0.6.0-dev.4. As in the first round, each decision `writfold-kb` made is a
kernel function named in `start_from`, `writfold-kb` calls it, and its tests
and golden files are unchanged; the three behaviour changes are named
below and in "Found by the proofs". The guard conformance cases pass
unchanged.

Scope globs (`Wild.lean`, `Scope.lean`). `scope::covers(patterns, path)`
takes code points split on `/` (`in_scope` and `glob` only decode text
into them). Both matchers are the same two-index loop with one backtrack
point, written as a step function (`char_step` within a segment, with `*`
as the wildcard; `path_step` across segments, with a `**` segment as the
wildcard and `segment` as the one-to-one match), and the loop calls it
until it stops. `Wild.lean` defines `WMatch S L p t` (an inductive
matcher over any wildcard predicate and match) and proves the invariant
once (`inv_star`, `inv_lit`, `inv_back`, `inv_fail`, `inv_exit`): each step
keeps it with a lower measure or stops with `WMatch`'s answer.

- `segment_iff`: one segment pattern matches one path segment exactly when
  `SegMatch` holds: `*` matches any run, none included; `?` one
  character; any other character itself.
- `segments_iff`: a pattern's segments match a path's exactly when
  `PathMatch` holds: a `**` segment matches any number of segments, none
  included; any other segment matches one by `SegMatch`.
- `covers_iff`: a path is covered exactly when some pattern without `!`
  matches it and no `!` pattern does; `exclusion_wins`,
  `no_pattern_no_scope`.

Plan writes (`Recheck.lean`, `writfold_core::plan::recheck`).
`still_stands_spec`, `stands_iff`: a decision stands under the lock
exactly when what it read still holds (no plan set meanwhile; the step
still current; a splice's parent still current when it was, and its
ordinal still the next). `writable_spec`, `sidecar_wins`,
`closed_refuses`: a run takes plan writes unless the state in force (the
sidecar's, else the manifest's) is `closed`. `writfold-kb` compares the
step paths and reads the states; the kernel decides.

Decisions and investigations (`Decision.lean`). `step_spec`: `decision::step`
is exactly the table of `decision.rs` for every pair of states;
`never_written`, `verified_needs_evidence`, `applied_needs_commit`,
`accepted_needs_ready_and_actor`, `rejected_needs_actor`,
`superseded_needs_successor`. `conclude_spec`, `none_spec`,
`conclude_none_iff`: an investigation may be concluded exactly when both
sections have text and no hypothesis is open. Whether what a change needs
was given (the commit on its ref, grounded evidence, a successor) is still
checked in `writfold-kb`, where each is resolved by a read; the spec's
`admit(from, to, given)` was not built, because those reads and their
refusals are interleaved and computing every fact first would change which
refusal a user sees.

Citation verdicts (`Verdict.lean`, `Recorded.lean`).

- `observed_spec` (the table `observedOf`), `observed_fresh_iff` (fresh
  exactly for the eight fresh facts), `moved_span_never_fresh`,
  `unsearched_never_fresh`, `changed_gate_never_fresh`,
  `checker_fresh_iff`; `with_mark_spec`, `mark_keeps_change`,
  `mark_never_fresh`, `decide_spec`.
- `handshake_spec`; `exchange_said_iff` (a reply counts exactly after an
  agreed handshake, a clean exit, a readable version and exactly one
  well-formed reply), `exchange_spec`; `aged_spec`, `aged_expired_iff`;
  `lifetime_spec` (over any type: the generic translated as is).
- `expired_spec`: a timed citation has expired exactly when
  `now - seen > ttl` over the integers.
- `window_spec`, `place_same_iff`, `place_at_iff`, `place_absent_iff`:
  `locate` is split into `window` (whether to search, which window was
  cited) and `place` (from the windows' flags: the cited window first,
  then the first from the top). `locate` keeps its signature and drives
  them with the caller's span comparison, comparing windows from the top
  only while none has matched, as before; that driver is tested, not
  translated.
- `recordable_iff`; `recorded_age_spec` (unusable when dated more than an
  hour ahead, otherwise `now - checked_at` clamped to `0 ..= i64::MAX`);
  `before_spec`, `better_spec`, `best_recorded_spec`, `newest_spec` (the
  pick is a usable statement ahead of every other usable one by age, then
  a receipt before the local statement, then the worse verdict, then the
  first receipt; `none` exactly when none is usable),
  `newest_never_unverifiable`. `newest` is one loop over an explicit
  comparator; the iterator version it replaces is kept as a test reference
  and agrees on every input the tests build, as does the saturating
  `recorded_age`.

The remainder (`Walk.lean`, `Command.lean`, `Find.lean`, `Idle.lean`).
`complete_iff` (`walk::complete`: `coverage=complete` exactly when
nothing is on the frontier and no reason skipped an edge; the traversal
stays compared with a reference search); `blocks_iff` (`diag::blocks`:
a lint fails on an error, or a warning under `--deny-warnings`);
`admit_iff` (`find::admit`: a hit is shown exactly when fewer than the
limit are shown, its record is not shown yet and it passes the filter;
every tier's hits go through it in tier order, so a record is shown once,
under the first tier that found it: the tier order itself is the order of
the calls in `verbs/find.rs`, not a kernel); `is_help_spec`, `named_spec`
(`command::matches` over the words' bytes: a whole-word prefix, not the
prefix and `--help` alone); `query_ok_spec`, `starts_spec`,
`next_from_spec`, `prefixed_spec` and `emb_first` (`find::prefix_match`
over code points: a match exactly when the query may match by prefix and
its tokens begin distinct name tokens in order; taking the first name
token each one begins never misses a match); `newer_spec`, `newest_spec`
(the first greatest stamp), `since_spec`, `idle_spec` (idle exactly when
the newest stamp lies more than `age` before `now` over the integers).

The order receipts are judged in (`Order.lean`, `writfold_core::order`).
`less_spec` (lexicographic bytes), `precedes_spec` (the lower `seq`, then
the file name), and the model lemmas `prec_irrefl`, `prec_trans`,
`prec_total` (a strict total order, tied to the code by `precedes_spec`).
Gate receipts, cover rows, assessed receipts and delivery check receipts
are sorted with `order::cmp`. Assessed receipts and delivery check
receipts gain `seq` (below, "Found by the proofs").

Reshapes, behaviour and tests unchanged unless named: tuple matches,
guards, struct and slice patterns become flat `if`s and single matches;
loops that update several variables call one small function per test
(`scope::include`, `idle::newer`, `verdict::recorded::better`); a match on
an indexed element goes through a function (`verdict::reply`); a
parameter named `verdict` is `reached` (it shadowed the module's
namespace in Lean); scope and `find` take code points (`u32`), `command`
takes bytes. `writfold_core`'s crate-level lint allowances gain
`question_mark`, `manual_filter` and `manual_map` (explicit matches
translate; their combinator forms do not).

### The bug-fix round (17 November 2026)

Fixed in v0.6.0-dev.4. Each decision the round changed was translated and
proved, and its mutants calibrated, before a verb called it.

The subject repository (`proofs/Writfold/Subject.lean`,
`writfold_core::subject::pick`). `pick(here, here_is_kb, recorded,
recorded_is_kb)` chooses the subject when no `--repo` is given;
`writfold-kb` finds the repositories and says what each is (tested, not
proved: whether two repositories share a Git common directory, whether a
work tree holds a KB at a prefix).

- `pick_spec`: the rule, row by row (`Rule`): the enclosing repository when
  it is not the KB's own; else the recorded worktree when it is not a
  checkout of the KB's repository; else the enclosing repository; else the
  KB's own when the recorded worktree was such a checkout; else none.
- `recorded_never_kb_checkout`: the recorded worktree is the subject only
  when it is not a checkout of the KB's repository, so the subject is never
  another checkout of it than the one the command runs in.
- `here_outside_kb_wins`, `kb_only_for_checkout`, `some_subject`.
- `unchanged_unless_checkout`: otherwise the choice is the earlier rule
  (`Earlier`), so the change is confined to that case.

`gate lines` with a plan (`proofs/Writfold/Policy.lean`, extended in place;
`writfold_core::policy::lines`, owner decision). `writfold-kb` gathers which
steps name each gate, how each closed (`policy::close`) and the plan's word
(`policy::summary`); not proved: that gathering, the counts and the header
text.

- `all_skipped_spec`: a gate is listed `skipped` exactly when some step
  names it and every step naming it was skipped.
- `closes_spec`: the word closes the plan exactly for `complete` and
  `complete-with-exceptions`.
- `owed_met_spec`: every listed gate not skipped is met; a skipped gate never
  counts.
- `lines_clean_iff`: the lines exit 0 exactly when the word closes the plan,
  every gate not skipped is met and nothing is invalid.
  `lines_flagged_never_clean`: never for `policy-moved` or `complete-stale`.
- `lines_agree_with_summary`: over `policy::summary`, clean lines imply
  every step closed, every required template present, the policy unmoved
  and no done step stale.

When `find`'s rows are full (`proofs/Writfold/Walk.lean`,
`writfold_core::find::full`). `find` no longer runs its later tiers once
50 rows are shown. `full_spec` (full exactly when `shown ≥ limit`),
`full_admits_nothing` (then `admit` takes no hit, whatever it is) and
`not_full_admits` (before, it takes any hit not yet shown that passes the
filter): skipping the later tiers changes no row. The tiers' order and
their SQL stay tested, not proved.

### The delivery round (17 November 2026)

Built in v0.6.0-dev.4 ([delivery-spec.md](../rfcs/2026-11-17-delivery-modes-second-round.md) section 3). The
delivery kernel's theorems were replaced in place and passed the proofs gate
before any verb was rewired. After the round: 569 theorems (540 before), 298
required claims (280 before), 189 mutants (170 before); every theorem's
axioms are within `propext`, `Classical.choice` and `Quot.sound`.

`writfold_core::delivery` now decides on `Owed { standing, accept }`: each
obligation with the standings its policy takes (a mode's `waivable`, or a
mode gate's own `accept`). `deciding` takes `Place { Open, Bound, Withdrawn }`;
`posted(recorded)` is the `posted:<mode>` standing; the guard's scans over the
run's candidates are in the kernel (`Seen { same, ready, live, routed, spent }`,
`postable`, `guard_any`, `guard_all`, `Unread { AllReady, Deny }`).

Replaced in place (`proofs/Writfold/Delivery.lean`):

- `unmet_spec`: `unmet o` is `!metB` of the standing under its `accept`.
- `standing_loop_spec`, `standing_spec`: `Checked(Open)` for no obligations;
  else the standing of the first obligation not met under its policy; else
  `Waived` if any is waived, else `Attested` if any is attested, else
  `Checked(Satisfied)`.
- `standing_satisfied_iff`: the standing is `Checked(Satisfied)` exactly when
  there are obligations and each is a passing check its policy takes, **or**
  the first obligation not met under its policy is a passing check. The
  second case needs an `accept` without `satisfied`, which the `accept`
  reader never builds (a class's own word is always taken). The
  specification's unconditional form is false for such a policy, so the true
  form is stated.
- `standing_first_unmet`: if `os[i]` is unmet and no earlier one is, the
  standing is `os[i]`'s.
- `standing_met_iff`: a ready delivery's standing is a met word. The converse
  is false (a waiver the policy rejects stands `waived`) and is no longer
  claimed.
- `ready_iff`: ready exactly when there is at least one obligation and every
  one is met under its policy. `empty_never_ready`, restated over `Owed`.
- `deciding_loop_spec`, `deciding_spec`: `deciding ps = some i` exactly when
  `ps[i]` is `Open` and every later entry is `Withdrawn`; `none` otherwise.
- `guard_allow_iff`: known bytes pass exactly when some candidate is
  postable; unknown bytes pass exactly under `AllReady` when some candidate
  is live and every live candidate is ready.

New, each a required claim of the axiom audit:

- `first_unmet_loop_spec`, `first_unmet_spec`: the least unmet index, or
  none.
- `refused_iff`: held by a statement its policy rejects exactly when the
  first unmet obligation stands on a met word.
- `waiver_rejected_never_ready`: a `Waived` obligation whose policy does not
  take a waiver makes the delivery unready, whatever the others are.
- `default_accept_is_before`: with every word taken, readiness and the
  standing are the rule before the round, on the standings alone, so an
  absent `accept` or `waivable` changes nothing. `accept_only_narrows`: ready
  under its policies implies ready with every word taken.
- `fact_met_iff_ready`: the plan's `fact_met` of `fact os` is exactly
  `ready os`, so the proved `done_only_when_met` covers a delivery step.
- `deciding_withdrawn_irrelevant`: inserting a `Withdrawn` entry at any index
  changes the answer only by the shift of its index (removing one is the
  converse). `deciding_none_after_bound`: a `Bound` entry with only
  `Withdrawn` ones after it leaves nothing deciding, so an older draft never
  decides once a newer candidate of its mode is bound.
- `posted_spec`, `posted_never_satisfied`, `posted_met_iff`: `Attested` when a
  record exists, else `Checked(Open)`; never `Checked(Satisfied)`; met
  exactly when recorded.
- `postable_spec`: postable exactly when same, ready, live, routed and not
  spent.
- `guard_any_loop_spec`, `guard_all_loop_spec` (and `guard_any_spec`,
  `guard_all_spec`): the two scans, for every prefix.
- `guard_deny_unread`: unknown bytes never pass under `Deny`.
- `guard_withdrawn_irrelevant`: two candidate lists whose live sublists are
  equal get the same decision.
- `guard_spent_never_passes`: when every candidate holding the bytes is
  spent, known bytes never pass.

Unchanged: `is_waived_spec`, `is_attested_spec`, `rechecked_spec`. Tested,
not proved: reading `waivable`, `audience`, `rendering` and `routes`;
splitting a command line and reading a body flag or a tool field; the
rendering; digests; which withdrawals and post records belong to the
candidate and run (the callers use the proved `gate::about`); drawing
findings; JSON. `deliver withdraw` and `deliver posted` are each one
create-new file under the existing write protocol, so `KbWrite.tla` and
`RunGraph.tla` are unchanged.

### The current-run round (17 November 2026)

Built in v0.6.0-dev.6 ([v060-dev6-spec.md](../rfcs/2026-11-17-v060-dev6-and-attached-stores.md)). Before it,
one local pointer per KB clone named the current run, so every session on
the clone shared it. `writfold_core::pointer` now decides among three
pointers a seat (one session in one work tree) can read: the session's, the
work tree's and the clone's shared one. It was translated and proved, and
its mutants calibrated, before `writfold-kb` called it
(`proofs/Writfold/Pointer.lean`).

A pointer reads `Unset`, or names a run that is `Open`, `Closed` or `Gone`
(not in the KB). `level(session, tree, shared)` is the deciding level,
`open(session, tree, shared)` whether the seat's current run is open, and
`writes(has_session, has_tree)` the pointers a seat writes when it makes a
run current.

- `is_set_spec`, `level_spec`: the deciding level is the most specific
  pointer that is set (session, then work tree, then shared); none exactly
  when none is set.
- `open_spec`: the seat's current run is open exactly when the deciding
  pointer names an open run.
- `session_decides`, `tree_decides`: with its session pointer set, a seat
  reads the same whatever the work tree's and the shared pointers hold;
  with no session pointer and a work-tree pointer, the same whatever the
  shared pointer holds.
- `ended_not_skipped`: a deciding pointer that names a closed or missing
  run is never passed over for a less specific pointer to an open run. A
  session whose run ended is told so; it is not judged against a run that
  may be another session's.
- `shared_alone`: with only the shared pointer the answer is that
  pointer's, which is the rule every earlier release applied.
- `writes_spec`: a seat writes the pointer of each level it has, and the
  shared pointer always.
- `writer_reads_its_run`: after a seat makes an open run current, that seat
  reads an open current run, whatever the three pointers held before and
  whichever levels the seat has. (`after` and `seen`, the pointer after a
  write and a level the seat lacks, are Lean definitions in `Pointer.lean`;
  that a written file reads back as the run is the file system's, tested.)
- `other_session_undisturbed`: a seat whose session pointer is set reads
  the same before and after any other seat makes a run current.

Tested, not proved: reading the pointer files and the state of the run
each names (`writfold-kb` `seat.rs`: a manifest's presence, the sidecar's
or the manifest's `state`); which work tree encloses a directory and
whether it is the KB's own (no work-tree pointer there); the session ID's
form; the `export` line the `SessionStart` hook appends; `resume <run>
--here` (it refuses a closed run through the proved
`guard::require::run_current`). The pointers are local, untracked files,
each written by one atomic rename; a crash between two of them leaves a
combination of pointer states, and every combination is an input the
theorems above cover, so there is no protocol to model: `KbWrite.tla` and
`RunGraph.tla` are unchanged.

### The repin round (17 November 2026)

Built in v0.6.0-dev.6 ([policy-spec.md](../rfcs/2026-11-16-user-defined-policy.md) section 2.5,
`plan repin`). The two decisions the verb adds were translated and proved,
and their mutants calibrated, before the verb was wired. After this round
and the current-run round, merged: 594 theorems, 316 required claims (298
before both), 203 mutants (189 before both).

`writfold_core::policy::repin` (`proofs/Writfold/Policy.lean`, extended in
place):

- `in_force(applies)`: which pin entry is in force. `writfold-kb` lists a
  run's pin entries oldest first (pins by `seq`, each pin's entries in file
  order) and says which apply to what is asked about: a gate for one step,
  or the required file. `in_force_spec` (`InForce`): the answer is an entry
  that applies with no applying entry after it, so the newest pin naming a
  gate wins and a repin's entry for one done step wins over its general
  entry for the gate; `in_force_none_iff`: there is none exactly when no
  entry applies. Before the round "the newest pin naming it" was decided in
  `writfold-kb` for gates, and the required file was always compared with
  the first pin.
- `repin_check(writable, valid, pin, loads, keeps)`: whether a repin may
  be written. `repin_check_spec`: the refusals in their fixed order (the
  run is closed; a pin file does not read; nothing was pinned; nothing
  moved; a gate an open step owes does not load now; the lock list as it
  is now locks a step the run already skipped). `repin_ok_iff`: written
  exactly when the run is open, every pin file reads, the policy against
  the pins (`pin_all`) is `Moved`, every owed gate loads and no skipped
  step would be locked. `repin_unmoved_refused`, `repin_closed_refused`,
  `repin_keeps_skips` (a repin never reopens a step, so it invalidates
  nothing itself).

That a repinned run never reads plain `complete` was already proved
(`summary_complete_iff`, `exception_never_clean`). Tested, not proved:
reading and writing pin files and their digests, gathering the entries and
which apply (comparing gate names and step paths), which digest a repin
records for each step, the definition digests themselves, folding the plan
again under the lock list as it is now to see whether a skipped step would
be locked, the `repin:` line and all other rendering. A repin is one create-new file under the
existing write protocol and the plan re-check (`Recheck.lean`, whose
`Current` case now also stands for "no step was current and none is"), so
`KbWrite.tla` and `RunGraph.tla` are unchanged.

### The attached-stores round (18 November 2026)

Built in v0.6.0-dev.7 ([v060-dev6-spec.md](../rfcs/2026-11-17-v060-dev6-and-attached-stores.md) part B, S1 and
S2; [stores](../guide/stores.md)). `writfold_core::store` was translated and
proved, and its mutants calibrated, before `writfold-kb` called it
(`proofs/Writfold/Store.lean`). After the round: 606 theorems, 328 required
claims (316 before), 210 mutants (203 before).

- `basis(declared, pinned, has_rev)`: which rule judges a citation.
  `basis_pin_iff`: the pin exactly for a declared store whose freshness is
  `pinned` and a token with a revision; `basis_head_default`: every
  undeclared repository, and a store declared `head`, is judged at head as
  before.
- `pinned(commit, at)`: the verdict under the pin, from whether the cited
  commit is absent, present but reached by no ref, or anchored, and what
  `rev:path` holds. `pinned_fresh_iff`: fresh exactly for the cited bytes at
  an anchored commit; `pinned_behind_unverifiable`: a commit the clone lacks
  is never gone and never fresh; `pinned_unanchored_gone`;
  `pinned_other_bytes`. That a reorganised store reads the same is the
  function's signature: nothing about the head is an input.
- `resolve(map, search, hint)`: where a declared store's clone is found.
  `resolve_map_wins` (flagged exactly when the origin differs),
  `resolve_committed_needs_origin`, `resolve_search_before_hint`,
  `resolve_flag_only_map`, `resolve_missing_iff`, `resolve_mismatch_iff`.

An authored mark still wins through the proved `verdict::with_mark`.
Tested, not proved: reading and checking the declaration files, `~` in a
hint, normalising a remote URL and reading a clone's `origin`, choosing
which hint to offer the kernel, the Git reads (one `cat-file --batch-check`
for the commits and paths, one `for-each-ref --contains` per present
commit), the full-revision check, rendering and JSON. Nothing is written, so
`KbWrite.tla` and `RunGraph.tla` are unchanged.

### The run-artifacts round (18 November 2026)

Built in v0.6.0-dev.8 ([v060-dev6-spec.md](../rfcs/2026-11-17-v060-dev6-and-attached-stores.md) part B, S4;
[stores](../guide/stores.md#pointing-a-run-at-its-material)).
`store::artifact` was translated, proved and its mutants calibrated before
`checkpoint --artifact` called it. After the round: 614 theorems, 336
required claims (328 before), 214 mutants (210 before).

- `artifact(declared, found, full_rev, commit, blob)`: whether a run records
  a pointer into a store. `artifact_admit_iff`: exactly when the store is
  declared and found here, the revision is a full commit ID, some ref
  reaches the commit and the path holds a file there.
  `artifact_admit_reads_fresh`: an admitted pointer reads fresh under
  `pinned` when it is written; it is an ordinary citation afterwards, with
  no second validity rule. `artifact_undeclared`, `artifact_not_found`,
  `artifact_short_rev`, `artifact_no_commit`, `artifact_unanchored`,
  `artifact_no_blob`: the refusals, in the order they are checked.

Tested, not proved: parsing `<store>@<rev>:<path>`, the Git reads (one batch
check for the commit, one `for-each-ref --contains`, one blob read), minting
the whole-file token, the artifacts file and the `resume` rows. The pointers
of one checkpoint are one create-new file in the same write plan as the
checkpoint entry, written before it, so `KbWrite.tla` and `RunGraph.tla` are
unchanged.

### The evidence-binding round (18 November 2026)

Kernels and proofs only; no verb calls them yet
([gate-evidence-spec.md](../rfcs/2026-11-18-gate-evidence-binding-and-pins.md), stage 1). After the round:
659 theorems, 366 required claims (336 before), 222 mutants (214 before).

- `bind` (`Bind.lean`): whether the references a cover row cites answer
  the item. `answers_spec`, `answers_any`, `other_never_answers`,
  `unbound_never_answers`, `named_need_iff`: under `Any` every reference
  answers; under `Own` only a look recorded for this item or a gate the
  item names; under `Named` only a named gate. `answered_spec`,
  `answered_iff`: some cited reference is grounded and answers.
  `answeredB_skip`, `answered_unrelated`: a reference that does not answer,
  or does not ground, never changes the answer. `item_under_spec`,
  `item_under_answered`, `item_under_supported_iff`,
  `item_under_none_found_iff`, `item_under_narrows`,
  `item_under_unanswered`: binding only narrows `checklist.item`.
  `bound_supported_iff`: relevance soundness end to end.
- `pin` (`Pin.lean`): evidence holds while every pin holds.
  `holds_spec`, `holds_nil`, `unheld_never_holds`; `timed_spec`,
  `timed_untimed`, `timed_aged_iff`, `timed_never_moved` (a catalog
  lifetime ages by `verdict::expired`); `nothing_found_spec`,
  `nothing_found_refines` (no pins is the status test of today);
  `current_under_spec`, `current_under_about`, `current_under_refines`.

Tested, not proved: everything the wiring stages add (the `evidence` key,
the `searched` field, store resolution, clock reads, rendering). Nothing is
written by these kernels, so `KbWrite.tla` and `RunGraph.tla` are unchanged.

### The backlinks round (19 November 2026)

[v060-stores-next-spec.md](../rfcs/2026-11-18-attached-stores-next.md) S3. `store::claim` was
translated, proved and its mutants calibrated before `context` and `uses`
called it. After the round: 664 theorems, 371 required claims (366
before), 226 mutants (222 before).

- `claim(attached, exact, aliased)`: what one `subject:` entry of a store
  directory's `README.md` amounts to. `claim_listed_iff`: the directory is
  listed under a record exactly when it was read from a declared, readable
  store and names the record by its ID or by an alias no other record has.
  `claim_exact_wins`: a record's own ID is never hidden by an alias others
  share. `claim_ambiguous_iff`, `claim_unknown_iff`: an alias of two or
  more records, or a name of none, lists nothing. `claim_needs_store`:
  nothing is listed from a repository that is not a declared, readable
  store. `Claim` is an input of no grounding function: a `material` row is
  the store's claim, never evidence a cover row can cite.

Tested, not proved: the tree read, front matter, the limits, the cache
table and its key, the two index lookups, row order, text and JSON. Nothing
is written to the KB, so `KbWrite.tla` and `RunGraph.tla` are unchanged.

### The snapshot round (19 November 2026)

[v060-stores-next-spec.md](../rfcs/2026-11-18-attached-stores-next.md) section 2. `store::snapshot` was
translated, proved and its mutants calibrated before `observe --snapshot`
called it. After the round: 670 theorems, 377 required claims (371
before), 233 mutants (226 before).

- `snapshot(facts, commit, at)`: whether `observe --snapshot` records the
  look. `snapshot_record_iff`: exactly when the status embeds a citation,
  the store is declared, pinned and has a work tree here, the bytes match
  their provenance file, `HEAD` is anchored and holds those bytes.
  `snapshot_record_is_fresh`: the citation written reads fresh under
  `pinned`; there is no second validity rule.
  `snapshot_uncommitted_withheld`: an uncommitted snapshot is never cited.
  `snapshot_order`: the reason withheld is the first that applies.
  `snapshot_stage_iff`: `NotCommitted`, the only answer under which files
  are written into the store, exactly when the five facts hold and nothing
  is committed at the path.
- `of_observation_antitone` (`Ground.lean`, no kernel change): adding a
  citation to an observation, or one ceasing to be fresh, never grounds a
  look that did not ground; a snapshot cannot revive an expired look.

Tested, not proved: reading the file, `HEAD`, reachability and
`HEAD:<path>`; the provenance codec and layout; the create-new writes into
the store; the printed commands. The KB write is the existing single create
of `observe`, so `KbWrite.tla` and `RunGraph.tla` are unchanged.

### The tauten claims round (19 November 2026)

For the tauten pilot ([tauten-adoption.md](tauten-adoption.md), "Proof
claims" and "Total helpers"; merged after the backlinks and snapshot
rounds). After the round: 685 theorems, 393 required claims (377 before),
247 mutants (233 before).

- `pin` (`Pin.lean`), `bind` (`Bind.lean`): `holds_from_total`,
  `answered_from_total`: `holds_from` and `answered_from` never fail, from
  any start index (the existing loop specs need the prefix before the start
  to be held or unanswered, so they say nothing for an arbitrary start).
  These are the shape tauten needs to subsume arithmetic and indexing
  findings, `∀ args, f args ⦃ _ ⦄` with no hypotheses; `holds-reads-past-end`
  and `answered-reads-past-end` show they reject an out-of-bounds read.
- Eight skips-last mutants for loops whose theorems no mutant broke
  (`any_stale`, `idle::newest`, `any_unpinned`, `any_waived`,
  `any_attested`, `any_open`, `all_passed`, `best_recorded`), so those
  functions can be tauten claims, which need a killed mutant. Their loop
  specs joined the required claims.

- Total helpers: `idle::newer`, `verdict::recorded::better`,
  `scope::char_step` and `scope::path_step` read through `get` and guard
  the arithmetic their preconditions justified, so they never panic; on
  every input the old preconditions allow they behave as before (the old
  specs are re-proved over the new source and kept). `slice_get_ok`
  (`Get.lean`); `newer_total`, `newer_past_end`, `better_total`,
  `better_past_end`, `char_step_total`, `char_step_outside`,
  `path_step_total`, `path_step_outside`: they never fail, and outside the
  old precondition the result is the conservative one (not newer, keep the
  best so far, no match). Four mutants reintroduce the old failure outside
  the precondition (`newer-reads-past-end`, `better-past-end-underflows`,
  `char-step-stretches-out`, `path-step-stretches-out`).

`proofs/Total.lean` (not part of the gate) lists the theorems of the total
shape; `proofs/tauten_gate.py` runs it after the unchanged gate.

### The find rows round (20 November 2026)

`find::show` takes every tier's hits in tier order (each a record number
and whether it passes the filter) and returns the positions `admit`
takes; `verbs/find.rs` gathers the hits and builds rows only for those
positions, so the tier order is now decided by a proved kernel rather than
by the order of the calls. `find::exact_stops` decides when `find` may stop
after the `exact` tier. `proofs/Writfold/Show.lean` proves `show` equal to
a list model (`show_spec`) and, on the model: at most the limit
(`shown_le_limit`), each record once (`shown_once`), in the hits' order
(`shown_in_order`), every row passing the filter (`shown_passes`) and
under its record's first passing hit (`shown_first`); the rows under a
smaller limit are the first rows under a larger one (`limit_prefix`), and
the rows of the first tiers' hits alone are the first rows of all of them
(`stop_is_prefix`). Four mutants. After the round: 714 theorems, 405
required claims, 251 mutants.

### The decision flags round (20 November 2026)

`status --set` on a decision used to check its flags and facts inline in
`write/decide.rs`. Two kernels now decide them: `decision::flags` (which of
`--by`, `--commit`, `--evidence`, `--run`/`--repo`, `--superseded-by` the
change needs or refuses; the first misfit is named) and `decision::writes`
(every fact the change needs holds: readiness, the commit on its target
ref, grounded evidence, an existing successor). `Decision.lean` proves
`flags_none_iff`, `flags_actor_first` and `writes_spec`, and composes the
latter with the transition table: `verified_written_only_grounded`,
`applied_written_only_on_ref`, `accepted_written_only_ready`,
`superseded_written_only_to_successor`. Five mutants. After the round: 721
theorems, 412 required claims, 256 mutants.

### The checklist snapshot round (20 November 2026)

`checklist::covered_under` gained a fourth fact, `unsnapped`: the
checklist's `snapshot` list names the item and its newest row cites no
look that kept a snapshot. A supported item is then not covered.
`Checklist.lean`'s `coveredB`, `covered_under_spec`, `item_covered_iff`,
`rollup_optional_irrelevant` and `Bind.lean`'s `item_under_narrows` carry
the fact; `snapshot_item_needs_snapshot` and `unlisted_item_unchanged`
(an unlisted item is covered exactly as before) are new. One mutant. After
the round: 723 theorems, 414 required claims, 257 mutants.

### The statement order round (21 November 2026)

Attestations and waivers (a gate's and a delivery's) gain `seq` and are
sorted by `order::cmp`, the receipt order of step 11. Which one stands is
`order::newest`: the position of the maximum, under `Prec`, of the
statements that stand. `Order.lean`'s `newest_loop_spec` and
`newest_is_max` (`none` exactly when none stands; otherwise a standing
statement that no standing statement follows) are new, with two mutants
(`newest-keeps-oldest`, `newest-ignores-stands`).

### The recorded pinned verdicts round (21 November 2026)

`store::recorded(basis, declared_max_age, within_age)` decides whether a
receipt may stand in for a pinned citation, `store::use_recorded` whether
a reader uses it instead of its own verdict (only when that is
`unverifiable`), and `store::pin_recordable` which verdicts `stale
--record` may write. `Store.lean` proves `code_recorded_iff`,
`code_head_never_recorded`, `code_local_wins`, `use_recorded_iff` and
`code_recorded_unverifiable_never`, with four mutants
(`recorded-any-basis`, `recorded-ignores-age`, `receipt-beats-local`,
`pin-records-unverifiable`). Which receipt is newest reuses the proved
`verdict::recorded::newest` ([#4](https://github.com/writfold-dev/writfold/issues/4)).
`Order.lean`'s `newest_total` (from `newest_is_max`) lets tauten subsume
`newest`'s indexing. After the statement order and recorded verdicts
rounds: 731 theorems, 422 required claims, 263 mutants.

### The cleanup (21 November 2026)

The assessed gate was removed with Weft ([#54](https://github.com/writfold-dev/writfold/issues/54)): the `assess` kernel, `Assess.lean`, its nine audited theorems, six mutants and four claims. No remaining theorem depended on it. `Grounding`, which the ground and checklist kernels decide with, moved to `grounding.rs`, outside the translated set; `Ground.lean` names `grounding.Grounding`. After the cleanup: 722 theorems, 413 required claims, 257 mutants. The removed Lean is in the archive's `public-pre-cleanup-2026-11-21` branch.

### The dev.14 round (21 November 2026)

Four kernels, each proved before it was wired ([#56](https://github.com/writfold-dev/writfold/issues/56)–[#59](https://github.com/writfold-dev/writfold/issues/59)):

- `inventory` (`Inventory.lean`): an inventory is complete exactly when no root is unreadable; each unreadable root is a counted problem, so `gates.met` is never met over an incomplete one; planned `gate lines` (`lines_clean_over`) and the unplanned "all met" (`unplanned_clean`, the backfill of the former `Report::all_met() && invalid == 0`) are never clean over one. Mutants `readable-all`, `complete-skips-check`, `unreadable-never-counts`, `over-drops-complete`, `all-true-skips-last`, `unplanned-empty-clean`.
- `handoff` (`Handoff.lean`): `strict` accepts exactly a complete, non-empty set whose every item is defined, made under the approved definition and `Checked Satisfied`; a statement never passes; a refusal names the lowest failing item. Mutants `item-ignores-approved`, `first-failing-skips-last`, `strict-empty-ok`.
- `publish` (`Publish.lean`): a repair replaces only a mismatching index while neither the KB nor the live index moved; a rebuild or refresh publishes unless the KB moved. Mutants `repair-on-drift`, `repair-without-mismatch`, `derived-ignores-move`. Its protocol is the new `spec/KbRepair.tla`.
- `graph` (`Graph.lean`): the refresh plan (skip, delta, whole), and the model theorem `delta_eq_full` (pure Lean, over a per-file edge function) that justifies the delta. Mutants `any-skips-last`, `plan-delta-on-key-change`, `plan-skip-ignores-changes`.

Two mutants first failed for the wrong reason: `plan-skip-ignores-changes` (removing the call to `any` broke an unrelated proof script; it now keeps the call, `any(changed) && false`) and the first draft of `KbRepair` (TLC found that a pin-only re-check lets a repair replace a healthy index a refresh committed during the comparison; the repair now confirms the generation).

### The claims and their mutants

<!-- generated from proofs/mutants.py -->

257 mutants (20 before 16 November 2026, 86 after the backfill's first round, 108 after the policy round, 161 after the backfill's second round, 170 after the bug-fix round, 189 after the delivery round: its nine delivery mutants re-aimed and 19 added, all 28 calibrated; 203 after the current-run and repin rounds; 210 after the attached-stores round; 214 after the run-artifacts round; 222 after the evidence-binding round; 226 after the backlinks round; 233 after the snapshot round; 247 after the tauten claims round; 251 after the find rows round; 256 after the decision flags round; 257 after the checklist snapshot round). Each must break exactly the theorems named.

| Mutant | File | What it breaks | Must break |
|---|---|---|---|
| `same-ignores-last-byte` | `bytes.rs` | identity is byte-exact: ignoring the last byte is a different identity | `same_loop_spec` |
| `closed-ignores-skip` | `plan.rs` | run graph: a skipped step is closed, so it is never current | `Plan.closed_spec` |
| `current-returns-last` | `plan.rs` | run graph: the current step is the least open one, not the last step | `Plan.current_loop_spec` |
| `first-owed-inverted` | `plan.rs` | run graph: the first owed gate is the first one not met | `Plan.first_owed_loop_spec` |
| `advance-without-current-check` | `plan.rs` | run graph: only the current step may be recorded as done | `Plan.advance_check_spec` |
| `advance-ignores-gates` | `plan.rs` | run graph: every gate of the step must be met | `Plan.advance_check_spec` |
| `stale-ignores-done` | `plan.rs` | run graph: a stale step is a done step with an owed gate | `Plan.stale_spec` |
| `splice-inserts-after-current` | `plan.rs` | splice: the new steps go before the current step, not after it | `Plan.splice_loop0_spec` |
| `splice-new-steps-done` | `plan.rs` | splice: the new steps are open | `Plan.splice_loop1_spec` |
| `splice-drops-last-step` | `plan.rs` | splice: every existing step is kept, the last one included | `Plan.splice_loop2_spec` |
| `splice-ignores-key` | `plan.rs` | splice: idempotent on the template key | `Plan.splice_check_spec` |
| `splice-depth-off-by-one` | `plan.rs` | splice: depth is bounded by MAX_DEPTH | `Plan.splice_check_spec` |
| `contains-misses-last` | `plan.rs` | splice: the last key spliced counts too | `Plan.contains_loop_spec` |
| `splice-check-refuses-empty` | `plan.rs` | splice: an empty plan takes a root splice at 0 | `Plan.splice_check_spec` |
| `splice-check-admits-complete` | `plan.rs` | splice: a complete plan has no current step to splice under | `Plan.splice_check_spec` |
| `outcome-ignores-moved` | `gate.rs` | gate: a subject that moved while the command ran is never a pass | `Gate.outcome_spec` |
| `outcome-any-exit-passes` | `gate.rs` | gate: only exit status 0 passes | `Gate.outcome_spec` |
| `outcome-nonzero-unknown` | `gate.rs` | gate: another exit status is a failure, not unknown | `Gate.outcome_spec` |
| `state-ignores-scope` | `gate.rs` | gate: a receipt about other bytes in scope never decides the state | `Gate.current_spec` |
| `state-ignores-definition` | `gate.rs` | gate: a receipt about another definition never decides the state | `Gate.current_spec` |
| `state-ignores-bound` | `gate.rs` | gate: a receipt from another run or subject, or broken, counts as absent | `Gate.state_loop_spec` |
| `current-ignores-bound` | `gate.rs` | gate: a decision needs a bound receipt (the comparison itself) | `Gate.current_spec` |
| `stale-from-unbound` | `gate.rs` | gate: with no current receipt, only a bound one makes the gate stale | `Gate.newest_bound_loop_spec`, `Gate.newest_bound_spec` |
| `bound-ignores-subject` | `gate.rs` | gate: bound needs this run, this subject and an intact digest | `Gate.bound_spec` |
| `stands-ignores-bound` | `gate.rs` | gate: a statement from another run or subject never stands | `Gate.stands_spec` |
| `attests-ignores-bound` | `gate.rs` | gate: an unbound attestation is not "some attestation" | `Gate.attests_spec` |
| `state-oldest-first` | `gate.rs` | gate: the newest receipt about the current subject decides, not the oldest | `Gate.state_loop_spec`, `Gate.state_spec` |
| `state-failed-satisfies` | `gate.rs` | gate: a failed receipt is a failure | `Gate.state_loop_spec` |
| `attested-class-stands-on-check` | `gate.rs` | gate: an attested-class obligation has no checked state to stand on | `Gate.standing_spec` |
| `waiver-satisfies` | `gate.rs` | gate: a waiver is never a passing check | `Gate.standing_spec` |
| `attestation-satisfies` | `gate.rs` | gate: an attestation is never a passing check | `Gate.standing_spec` |
| `met-counts-stale` | `gate.rs` | gate: a stale obligation is owed | `Gate.met_spec` |
| `about-ignores-definition` | `gate.rs` | about the present: the recorded definition must be the present one | `Gate.about_spec` |
| `evidence-ignores-definition` | `gate.rs` | a receipt under another definition is not the same definition | `Gate.evidence_spec` |
| `evidence-ignores-content` | `gate.rs` | a receipt about other bytes is not about the present content | `Gate.evidence_spec` |
| `evidence-accepts-empty-content` | `gate.rs` | empty present content (unreadable) is never the same content | `Gate.evidence_spec` |
| `judge-drops-newest` | `gate.rs` | every receipt is judged, the newest included | `Gate.judge_loop_spec` |
| `stands-ignores-kind` | `gate.rs` | an attestation is never a waiver, and a waiver never an attestation | `Gate.stands_spec` |
| `statements-counts-moved-waiver` | `gate.rs` | a waiver about other bytes does not stand | `Gate.statements_loop_spec` |
| `advance-undefined-met` | `plan.rs` | a gate with no usable definition is never met | `Plan.fact_met_spec` |
| `met-all-drops-last` | `plan.rs` | every gate of the step is judged, the last included | `Plan.met_all_loop_spec` |
| `advance-refuses-no-gates` | `plan.rs` | a step with no gates owes nothing | `Plan.advance_facts_spec`, `Plan.advance_facts_ok_iff` |
| `item-accepts-no-evidence` | `checklist.rs` | checklist: a supported row covers only when it cites something | `Checklist.item_spec` |
| `item-accepts-ungrounded` | `checklist.rs` | checklist: a supported row covers only while its evidence is grounded | `Checklist.item_spec` |
| `item-accepts-not-current` | `checklist.rs` | checklist: a row about other content or another definition is stale | `Checklist.item_spec`, `Checklist.item_none_found_iff` |
| `covered-counts-unsupported` | `checklist.rs` | checklist: an unsupported item is not covered | `Checklist.covered_under_spec` |
| `covered-ignores-na` | `checklist.rs` | checklist: not-applicable covers only where the policy accepts it | `Checklist.covered_under_spec` |
| `covered-none-found-without-policy` | `checklist.rs` | checklist: none-found covers only where the policy accepts it | `Checklist.covered_under_spec` |
| `covered-ignores-snapshot` | `checklist.rs` | checklist: an item the `snapshot` list names needs a look that kept one | `Checklist.covered_under_spec` |
| `item-none-found-no-evidence` | `checklist.rs` | checklist: none-found needs at least one cited look | `Checklist.item_spec`, `Checklist.item_none_found_iff` |
| `rollup-ignores-required` | `checklist.rs` | checklist: only required items are owed | `Checklist.owes_spec` |
| `first-owed-skips-last` | `checklist.rs` | checklist: every entry is looked at, the last included | `Checklist.first_owed_loop_spec` |
| `stale-ignores-required` | `checklist.rs` | checklist: an optional stale item does not make the checklist stale | `Checklist.stale_spec` |
| `checklist-attested-when-stale` | `checklist.rs` | checklist: a stale checklist is not attested | `Checklist.standing_spec` |
| `delivery-empty-met` | `delivery.rs` | delivery: an empty set of obligations is not met | `Delivery.standing_spec` |
| `delivery-waived-satisfied` | `delivery.rs` | delivery: a waived obligation is never read as a passing check | `Delivery.standing_loop_spec` |
| `delivery-unmet-ignored` | `delivery.rs` | delivery: an unmet obligation stops the delivery | `Delivery.unmet_spec` |
| `deciding-ignores-bound` | `delivery.rs` | delivery: a bound candidate ends the scan (continuing past it is the rule before 17 November 2026, under which an abandoned draft met a step) | `Delivery.deciding_loop_spec` |
| `deciding-first-unbound` | `delivery.rs` | delivery: the scan runs from the newest candidate, not the oldest | `Delivery.deciding_loop_spec`, `Delivery.deciding_spec` |
| `ready-accepts-empty` | `delivery.rs` | delivery: a mode that declares nothing is never ready | `Delivery.ready_iff` |
| `rechecked-keeps-satisfied` | `delivery.rs` | delivery: a passed check whose cited evidence moved is stale | `Delivery.rechecked_spec` |
| `guard-allows-no-candidate` | `delivery.rs` | delivery guard: unread bytes under a listed command need a live candidate | `Delivery.guard_all_loop_spec` |
| `guard-known-any-ready` | `delivery.rs` | delivery guard: known bytes pass only when a postable candidate holds them | `Delivery.guard_allow_iff` |
| `all-met-accepts-unknown` | `guard.rs` | guard: an unknown fact is never met | `Guard.met_spec` |
| `all-met-skips-last` | `guard.rs` | guard: every requirement is looked at, the last included | `Guard.all_met_loop_spec` |
| `decide-ignores-facts` | `guard.rs` | guard: a matched rule passes only when every requirement is met | `Guard.decide_spec` |
| `verdict-deny-asks` | `guard.rs` | guard: a deny rule denies | `Guard.verdict_spec` |
| `combine-ask-over-deny` | `guard.rs` | guard: a denial wins over a question | `Guard.combine_spec` |
| `effect-enforces-audit` | `guard.rs` | guard: an audit rule never decides the call | `Guard.effect_spec` |
| `gates-fact-met-on-nothing` | `guard.rs` | guard: gates.met is not met by an empty set | `Guard.gates_fact_met_iff` |
| `gates-fact-ignores-invalid` | `guard.rs` | guard: an invalid receipt or definition keeps gates.met unmet | `Guard.gates_fact_met_iff` |
| `nudge-while-active` | `guard.rs` | guard: the Stop nudge never fires while a stop hook is continuing | `Guard.nudge_spec` |
| `nudge-with-ignores-repeat` | `guard.rs` | guard: a turn rule is not held back by the session flag | `Guard.nudge_with_iff` |
| `decide-all-drops-last` | `guard/call.rs` | guard: every matched rule counts, the last included | `Guard.decide_all_loop_spec` |
| `logged-drops-waiver` | `guard/call.rs` | guard: a pass on a waiver is logged | `Guard.logged_iff` |
| `run-current-closed` | `guard/require.rs` | guard: run.current needs an open run | `Guard.run_current_met_iff` |
| `step-fact-met-unread` | `guard/require.rs` | guard: step.met is never met when the plan could not be read | `Guard.step_fact_met_iff`, `Guard.step_fact_unknown_iff` |
| `gate-fact-undefined-met` | `guard/require.rs` | guard: an undefined gate is never met | `Guard.gate_fact_met_iff` |
| `gates-read-unread-counts` | `guard/require.rs` | guard: gates that could not be judged are unknown | `Guard.gates_read_met_iff` |
| `waiver-grounds` | `ground.rs` | grounding: a waiver is never evidence | `Ground.of_gate_spec` |
| `of-gate-ignores-found` | `ground.rs` | grounding: a reference to no gate grounds nothing | `Ground.of_gate_spec` |
| `receipt-ignores-content` | `ground.rs` | grounding: a receipt about other content does not ground | `Ground.of_receipt_grounded_iff` |
| `receipt-ignores-gate-met` | `ground.rs` | grounding: a receipt grounds only while its gate stands met | `Ground.of_receipt_grounded_iff` |
| `citations-zero-grounds` | `ground.rs` | grounding: a retrieval with no citation grounds nothing | `Ground.of_citations_grounded_iff` |
| `token-many-grounds` | `ground.rs` | grounding: a token grounds only as exactly one citation | `Ground.of_token_grounded_iff` |
| `observation-no-citation-grounds` | `ground.rs` | grounding: an observation with no citation grounds nothing | `Ground.of_observation_grounded_iff` |
| `statement-not-current-attested` | `ground.rs` | grounding: only the current attestation is attested | `Ground.of_statement_spec` |
| `moved-ignores-word` | `ground.rs` | grounding: a changed standing word is a moved reference | `Ground.moved_iff` |
| `observation-partial-grounds` | `observe.rs` | observations: a partial look never grounds | `Ground.grounds_iff` |
| `embeds-drops-partial` | `observe.rs` | observations: a partial look embeds its citation | `Ground.embeds_iff` |
| `skip-without-lock` | `plan.rs` | plan: a step locked by the run's pinned policy cannot be skipped | `Plan.skip_check_spec` |
| `skipped-ignores-lock` | `plan.rs` | plan: a skip record of a locked step never closes it | `Plan.skipped_spec`, `Plan.locked_skip_never_closes` |
| `close-check-ignores-present` | `plan.rs` | plan: the last step closes only with every required template present | `Plan.close_check_spec` |
| `fact-met-ignores-policy` | `plan.rs` | plan: a gate is judged under its own policy | `Plan.fact_met_spec` |
| `met-ignores-waived` | `policy.rs` | policy: a gate that rejects waivers is never met by one | `Policy.met_spec` |
| `met-default-counts-stale` | `policy.rs` | policy: a policy without accept is exactly today's rule | `Policy.met_spec`, `Policy.met_default` |
| `pin-one-missing-same` | `policy.rs` | policy: a pinned digest with nothing there now is moved, never same | `Policy.pin_one_spec`, `Policy.pin_one_same_iff` |
| `pin-all-empty-same` | `policy.rs` | policy: an empty pin is not "same" | `Policy.pin_all_spec` |
| `pin-all-misses-moved` | `policy.rs` | policy: one moved pin moves the run | `Policy.any_moved_loop_spec` |
| `close-waived-passed` | `policy.rs` | policy: a waived gate is never read as passed | `Policy.close_spec` |
| `summary-skips-moved` | `policy.rs` | policy: a moved policy is never read as complete | `Policy.summary_spec` |
| `summary-ignores-closed` | `policy.rs` | policy: a closed run with an open step is closed-incomplete | `Policy.summary_spec` |
| `summary-complete-with-skip` | `policy.rs` | policy: a skipped, waived or attested step is never plain complete | `Policy.summary_spec` |
| `covers-ignores-exclusion` | `scope.rs` | scope: an exclusion that matches keeps the path out | `Scope.covers_loop_spec` |
| `covers-empty-includes` | `scope.rs` | scope: a scope with no pattern that includes covers nothing | `Scope.covers_iff` |
| `include-ignores-hit` | `scope.rs` | scope: an including pattern counts only when it matches | `Scope.include_spec` |
| `segment-no-backtrack` | `scope.rs` | scope: a `*` that led nowhere stretches by one more character | `Scope.char_step_spec` |
| `star-needs-a-char` | `scope.rs` | scope: `*` matches nothing too, at the end of a segment included | `Scope.char_step_spec` |
| `same-char-ignores-any` | `scope.rs` | scope: `?` matches any one character | `Scope.same_char_spec` |
| `globstar-one-star` | `scope.rs` | scope: only `**` is a segment wildcard, not `*` followed by anything | `Scope.globstar_spec` |
| `globstar-needs-a-segment` | `scope.rs` | scope: `**` matches no segment too, at the end of a path included | `Scope.path_step_spec` |
| `segments-accept-trailing` | `scope.rs` | scope: a path with segments left over after the pattern does not match | `Scope.path_step_spec`, `Scope.path_step_outside` |
| `recheck-current-ignored` | `plan/recheck.rs` | plan writes: a step write stands only while the step is still current | `Recheck.still_stands_spec` |
| `recheck-ordinal-ignored` | `plan/recheck.rs` | plan writes: a splice stands only while its ordinal is still the next one | `Recheck.still_stands_spec` |
| `recheck-root-needs-current` | `plan/recheck.rs` | plan writes: a root end template is not under a current step | `Recheck.still_stands_spec` |
| `recheck-ignores-plan` | `plan/recheck.rs` | plan writes: a write on an unplanned run stands only while no plan was set | `Recheck.still_stands_spec` |
| `writable-manifest-wins` | `plan/recheck.rs` | plan writes: the sidecar's state wins over the manifest's | `Recheck.writable_spec`, `Recheck.sidecar_wins`, `Recheck.closed_refuses` |
| `accepted-without-readiness` | `decision.rs` | decisions: accepting needs readiness | `Decision.step_spec` |
| `verified-without-evidence` | `decision.rs` | decisions: verifying needs grounded evidence | `Decision.step_spec` |
| `superseded-twice` | `decision.rs` | decisions: a superseded decision is not superseded again | `Decision.step_spec` |
| `landing-written` | `decision.rs` | decisions: landing is never written | `Decision.step_spec` |
| `conclude-ignores-section` | `investigation.rs` | investigations: a missing section stops the conclusion | `Decision.conclude_spec` |
| `conclude-skips-last` | `investigation.rs` | investigations: every hypothesis is counted, the last included | `Decision.conclude_loop_spec` |
| `none-ignores-open` | `investigation.rs` | investigations: an open hypothesis stops the conclusion | `Decision.none_spec` |
| `moved-span-fresh` | `verdict.rs` | verdict: a span found at other lines is moved, never fresh | `Verdict.observed_spec` |
| `unsearched-fresh` | `verdict.rs` | verdict: a file too long to search is changed, never fresh | `Verdict.observed_spec` |
| `mark-hides-change` | `verdict.rs` | verdict: a mark never hides a change | `Verdict.with_mark_spec` |
| `handshake-kind-as-op` | `verdict.rs` | verdict: the protocol first, then the kind, then the operation | `Verdict.handshake_spec` |
| `exchange-ignores-handshake` | `verdict.rs` | verdict: a reply counts only after an agreed handshake | `Verdict.exchange_said_iff`, `Verdict.exchange_spec` |
| `exchange-first-of-two` | `verdict.rs` | verdict: exactly one reply counts, not the first of two | `Verdict.exchange_said_iff`, `Verdict.exchange_spec` |
| `aged-keeps-unreadable` | `verdict.rs` | verdict: a reply whose age cannot be read is expired | `Verdict.aged_spec` |
| `lifetime-token-as-default` | `verdict.rs` | verdict: the token's own ttl comes before the kind's default | `Verdict.lifetime_spec` |
| `expired-overflow-kept` | `verdict.rs` | verdict: far past the end of i64, the sign alone decides expiry | `Verdict.expired_spec` |
| `window-cited-off-by-one` | `verdict/locate.rs` | verdict: the cited lines must lie inside the file | `Verdict.window_spec` |
| `place-ignores-cited` | `verdict/locate.rs` | verdict: the cited window comes before any other | `Verdict.place_same_iff`, `Verdict.place_at_iff`, `Verdict.place_absent_iff` |
| `recordable-unverifiable` | `verdict/recorded.rs` | check receipts: an unverifiable answer is never shared | `Recorded.recordable_iff` |
| `recorded-age-no-skew` | `verdict/recorded.rs` | check receipts: a date more than an hour ahead is unusable | `Recorded.recorded_age_spec` |
| `before-prefers-older` | `verdict/recorded.rs` | check receipts: the newer statement comes first | `Recorded.before_spec` |
| `better-tie-takes-later` | `verdict/recorded.rs` | check receipts: on a tie the first receipt is kept | `Recorded.better_spec` |
| `newest-local-on-tie` | `verdict/recorded.rs` | check receipts: on a full tie a receipt wins over the local statement | `Recorded.newest_spec` |
| `complete-ignores-skipped` | `walk.rs` | walk: a skipped edge keeps coverage partial | `Walk.complete_iff` |
| `blocks-any-warning` | `diag/mod.rs` | lint: a warning blocks only under --deny-warnings | `Walk.blocks_iff` |
| `admit-shows-twice` | `find.rs` | find: a record is shown once, under the first tier that found it | `Walk.admit_iff` |
| `is-help-wrong-byte` | `command/mod.rs` | command: --help is exactly those bytes | `Command.is_help_spec` |
| `named-skips-last` | `command/mod.rs` | command: every word of a rule's prefix is compared, the last included | `Command.named_loop_spec` |
| `named-help-not-alone` | `command/mod.rs` | command: --help stops a match only when it is the one word after the prefix | `Command.named_loop_spec` |
| `query-ok-three-chars` | `find.rs` | find: a one-token query needs four characters | `Find.query_ok_spec` |
| `starts-skips-last` | `find.rs` | find: every character of a prefix is compared, the last included | `Find.starts_loop_spec` |
| `next-from-reversed` | `find.rs` | find: the query token begins the name token, not the other way round | `Find.next_from_loop_spec` |
| `prefixed-reuses-token` | `find.rs` | find: each query token takes its own name token | `Find.prefixed_loop_spec` |
| `newest-last-of-equal` | `idle.rs` | idle: the first of equal newest stamps is named | `Idle.newer_spec` |
| `idle-saturated-compare` | `idle.rs` | idle: the age is compared over the integers, not after holding it in i64 | `Idle.idle_spec` |
| `since-off-by-one` | `idle.rs` | idle: past the end of i64 the seconds shown are its end | `Idle.since_spec` |
| `less-larger-byte` | `order.rs` | order: at the first differing byte, the smaller sorts first | `Order.less_loop_spec` |
| `less-equal-names` | `order.rs` | order: a name never sorts before itself | `Order.less_loop_spec` |
| `precedes-higher-seq` | `order.rs` | order: the lower seq is judged first | `Order.precedes_spec` |
| `basis-ignores-premises` | `assess.rs` | assessed gate: one ungrounded premise makes the basis hypothetical | `Assess.basis_loop_spec` |
| `met-accepts-hypothetical` | `assess.rs` | assessed gate: a hypothetical entailment never meets the gate | `Assess.met_spec` |
| `outcome-passes-unfinished` | `assess.rs` | assessed gate: a check Weft could not finish is unknown, never a pass | `Assess.outcome_loop_spec` |
| `assessed-empty-passes` | `assess.rs` | assessed gate: nothing required is unknown, never a pass | `Assess.outcome_spec` |
| `assessed-incomplete-passes` | `assess.rs` | assessed gate: an incomplete run is unknown, never a pass | `Assess.outcome_spec` |
| `assumption-ignores-attested` | `assess.rs` | assessed gate: an attested citation makes the assumption attested | `Assess.assumption_loop_spec` |
| `pick-recorded-checkout` | `subject.rs` | subject: another checkout of the KB's repository is never judged | `Subject.pick_spec`, `Subject.recorded_never_kb_checkout` |
| `pick-here-even-kb` | `subject.rs` | subject: the enclosing repository wins only when it is not the KB's | `Subject.pick_spec`, `Subject.unchanged_unless_checkout` |
| `pick-checkout-nothing` | `subject.rs` | subject: a recorded checkout outside any repository is the KB's own | `Subject.pick_spec`, `Subject.some_subject` |
| `skipped-any-step` | `policy/lines.rs` | gate lines: a gate is skipped only when every step naming it was | `Policy.all_skipped_loop_spec`, `Policy.all_skipped_spec` |
| `skipped-unnamed` | `policy/lines.rs` | gate lines: a gate no step names is not skipped | `Policy.all_skipped_spec` |
| `owed-ignores-skip` | `policy/lines.rs` | gate lines: a gate not skipped is owed until met | `Policy.owed_met_loop_spec` |
| `closes-stale` | `policy/lines.rs` | gate lines: a stale plan does not close | `Policy.closes_spec` |
| `clean-ignores-invalid` | `policy/lines.rs` | gate lines: an invalid receipt or definition keeps the lines unclean | `Policy.lines_clean_iff` |
| `full-late` | `find.rs` | find: the later tiers stop only when no hit could be shown | `Walk.full_spec`, `Walk.not_full_admits` |
| `show-ignores-seen` | `find.rs` | find: a record is shown once, under its first hit | `Show.show_loop_spec` |
| `show-records-wrong` | `find.rs` | find: the records shown are the hits' records | `Show.show_loop_spec` |
| `among-skips-last` | `find.rs` | find: every record shown is compared, the last included | `Show.among_loop_spec` |
| `exact-stops-on-many` | `find.rs` | find: stop after the exact tier only on exactly one record | `Show.exact_stops_spec` |
| `flags-actor-inverted` | `decision.rs` | decision: accepting or rejecting needs `--by` | `Decision.flags_none_iff`, `Decision.flags_actor_first` |
| `flags-commit-not-refused` | `decision.rs` | decision: `--commit` goes only with `applied` | `Decision.flags_none_iff` |
| `flags-run-or-repo-ignored` | `decision.rs` | decision: `--run` and `--repo` go only with `verified` | `Decision.flags_none_iff` |
| `writes-ignores-grounded` | `decision.rs` | decision: `verified` needs evidence that grounds | `Decision.writes_spec` |
| `writes-ignores-successor` | `decision.rs` | decision: `superseded` needs an existing successor | `Decision.writes_spec` |
| `delivery-accept-ignored` | `delivery.rs` | delivery: an obligation is met under its own policy, not by any met word | `Delivery.unmet_spec` |
| `ready-from-standing` | `delivery.rs` | delivery: ready is every obligation met under its policy, not a met standing | `Delivery.ready_iff` |
| `refused-any-unmet` | `delivery.rs` | delivery: refused only when the first unmet obligation stands on a met word | `Delivery.refused_iff` |
| `fact-accepts-when-unready` | `delivery.rs` | delivery: a step's deliver gate is met exactly when the candidate is ready | `Delivery.fact_met_iff_ready` |
| `first-unmet-skips-last` | `delivery.rs` | delivery: the last obligation counts too | `Delivery.first_unmet_loop_spec` |
| `deciding-counts-withdrawn` | `delivery.rs` | delivery: a withdrawn candidate never decides | `Delivery.deciding_loop_spec` |
| `deciding-stops-at-withdrawn` | `delivery.rs` | delivery: a withdrawn candidate is passed over, not the end of the scan | `Delivery.deciding_loop_spec` |
| `posted-always` | `delivery.rs` | delivery: posted:<mode> is open until a post record exists | `Delivery.posted_spec` |
| `posted-satisfied` | `delivery.rs` | delivery: a post record is a statement, never a passing check | `Delivery.posted_spec`, `Delivery.posted_never_satisfied` |
| `postable-drops-same` | `delivery.rs` | delivery guard: a postable candidate is same | `Delivery.postable_spec` |
| `postable-drops-ready` | `delivery.rs` | delivery guard: a postable candidate is ready | `Delivery.postable_spec` |
| `postable-drops-live` | `delivery.rs` | delivery guard: a postable candidate is live | `Delivery.postable_spec` |
| `postable-drops-routed` | `delivery.rs` | delivery guard: a postable candidate is routed | `Delivery.postable_spec` |
| `postable-drops-spent` | `delivery.rs` | delivery guard: a posted candidate is spent | `Delivery.postable_spec` |
| `guard-known-all` | `delivery.rs` | delivery guard: known bytes need one postable candidate, not every candidate ready | `Delivery.guard_allow_iff` |
| `guard-unread-deny-allows` | `delivery.rs` | delivery guard: unread bytes under a route never pass | `Delivery.guard_allow_iff`, `Delivery.guard_deny_unread` |
| `guard-all-counts-withdrawn` | `delivery.rs` | delivery guard: a withdrawn candidate need not be ready | `Delivery.guard_all_loop_spec` |
| `guard-all-skips-last` | `delivery.rs` | delivery guard: the last candidate counts for the all-ready rule | `Delivery.guard_all_loop_spec` |
| `guard-any-skips-last` | `delivery.rs` | delivery guard: the last candidate may hold the bytes | `Delivery.guard_any_loop_spec` |
| `level-tree-before-session` | `pointer.rs` | pointer: a session pointer decides before the work tree's | `Pointer.level_spec`, `Pointer.open_spec`, `Pointer.session_decides`, `Pointer.ended_not_skipped` |
| `is-set-skips-gone` | `pointer.rs` | pointer: a pointer to a run the KB lacks is set, and so not skipped | `Pointer.is_set_spec`, `Pointer.level_spec`, `Pointer.open_spec`, `Pointer.session_decides`, `Pointer.tree_decides`, `Pointer.ended_not_skipped` |
| `open-skips-closed-session` | `pointer.rs` | pointer: a closed run at the deciding level is not skipped for an open one below | `Pointer.open_spec`, `Pointer.session_decides`, `Pointer.ended_not_skipped` |
| `open-shared-any-run` | `pointer.rs` | pointer: only an open run is an open current run | `Pointer.open_spec`, `Pointer.shared_alone`, `Pointer.ended_not_skipped` |
| `writes-shared-only-without-session` | `pointer.rs` | pointer: the shared pointer is always written | `Pointer.writes_spec` |
| `writes-no-session` | `pointer.rs` | pointer: a seat with a session writes its session pointer | `Pointer.writes_spec`, `Pointer.writer_reads_its_run` |
| `in-force-inverted` | `policy/repin.rs` | repin: the entry in force is one that applies | `Policy.in_force_loop_spec` |
| `in-force-skips-oldest` | `policy/repin.rs` | repin: the oldest entry counts when no newer one applies | `Policy.in_force_loop_spec` |
| `repin-closed-run` | `policy/repin.rs` | repin: a closed run takes no repin | `Policy.repin_check_spec`, `Policy.repin_ok_iff`, `Policy.repin_closed_refused` |
| `repin-ignores-invalid` | `policy/repin.rs` | repin: a pin file that does not read is not adopted | `Policy.repin_check_spec`, `Policy.repin_ok_iff` |
| `repin-when-same` | `policy/repin.rs` | repin: refused when nothing moved | `Policy.repin_check_spec`, `Policy.repin_ok_iff` |
| `repin-when-unpinned` | `policy/repin.rs` | repin: refused when the run pinned nothing | `Policy.repin_check_spec`, `Policy.repin_ok_iff` |
| `repin-undefined-gate` | `policy/repin.rs` | repin: a gate an open step owes must load to be adopted | `Policy.repin_check_spec`, `Policy.repin_ok_iff` |
| `repin-reopens-skip` | `policy/repin.rs` | repin: adopting a lock list never reopens a skipped step | `Policy.repin_check_spec`, `Policy.repin_ok_iff` |
| `basis-ignores-rev` | `store.rs` | store: the pin judges only a citation that names a revision | `Store.basis_pin_iff` |
| `pinned-absent-gone` | `store.rs` | store: a clone that is behind is not evidence of loss | `Store.pinned_behind_unverifiable` |
| `pinned-unanchored-fresh` | `store.rs` | store: a commit no ref reaches is not fresh | `Store.pinned_fresh_iff`, `Store.pinned_unanchored_gone` |
| `pinned-other-bytes-fresh` | `store.rs` | store: other bytes at the pin are not the cited ones | `Store.pinned_fresh_iff`, `Store.pinned_other_bytes` |
| `resolve-hint-any-origin` | `store.rs` | store: a committed hint needs the declared origin | `Store.resolve_committed_needs_origin`, `Store.resolve_mismatch_iff` |
| `resolve-map-unflagged` | `store.rs` | store: the map's clone is flagged when its origin differs | `Store.resolve_map_wins` |
| `resolve-hint-before-search` | `store.rs` | store: search roots before committed hints | `Store.resolve_search_before_hint` |
| `artifact-short-rev-admitted` | `store.rs` | store: an artifact needs a full commit ID | `Store.artifact_admit_iff`, `Store.artifact_short_rev` |
| `artifact-unanchored-admitted` | `store.rs` | store: an unanchored commit is refused, never admitted | `Store.artifact_admit_iff`, `Store.artifact_admit_reads_fresh`, `Store.artifact_unanchored` |
| `artifact-found-before-declared` | `store.rs` | store: the declaration is checked before the clone | `Store.artifact_undeclared` |
| `artifact-absent-as-unanchored` | `store.rs` | store: a missing commit says fetch, not push | `Store.artifact_no_commit` |
| `claim-two-listed` | `store.rs` | store: an alias of two records lists the directory under neither | `Store.claim_listed_iff`, `Store.claim_ambiguous_iff` |
| `claim-unknown-listed` | `store.rs` | store: a subject that names no record lists nothing | `Store.claim_listed_iff`, `Store.claim_unknown_iff` |
| `claim-alias-hides-id` | `store.rs` | store: a shared alias never hides a record's own ID | `Store.claim_exact_wins`, `Store.claim_listed_iff`, `Store.claim_ambiguous_iff` |
| `claim-any-repository` | `store.rs` | store: nothing is listed from a repository that is not a declared store | `Store.claim_needs_store`, `Store.claim_listed_iff`, `Store.claim_ambiguous_iff`, `Store.claim_unknown_iff` |
| `snapshot-uncommitted-recorded` | `store.rs` | store: an uncommitted snapshot is never cited | `Store.snapshot_record_iff`, `Store.snapshot_record_is_fresh`, `Store.snapshot_uncommitted_withheld`, `Store.snapshot_order`, `Store.snapshot_stage_iff` |
| `snapshot-unanchored-recorded` | `store.rs` | store: a snapshot no ref reaches is not recorded | `Store.snapshot_record_iff`, `Store.snapshot_record_is_fresh`, `Store.snapshot_order` |
| `snapshot-other-bytes-recorded` | `store.rs` | store: a snapshot whose committed bytes differ is not recorded | `Store.snapshot_record_iff`, `Store.snapshot_record_is_fresh`, `Store.snapshot_order` |
| `snapshot-head-store` | `store.rs` | store: a snapshot is kept only in a pinned store | `Store.snapshot_record_iff`, `Store.snapshot_order`, `Store.snapshot_stage_iff` |
| `snapshot-damaged-recorded` | `store.rs` | store: bytes that no longer match their provenance are not recorded | `Store.snapshot_record_iff`, `Store.snapshot_order`, `Store.snapshot_stage_iff` |
| `snapshot-any-status` | `store.rs` | store: a status that embeds nothing takes no snapshot | `Store.snapshot_record_iff`, `Store.snapshot_order`, `Store.snapshot_stage_iff` |
| `snapshot-order-swapped` | `store.rs` | store: a head-judged store is named before a missing clone | `Store.snapshot_order` |
| `own-accepts-other` | `bind.rs` | bind: a look recorded for another item never answers an item that wants its own | `Bind.answers_spec` |
| `named-accepts-this` | `bind.rs` | bind: only a named gate answers an item that names its evidence | `Bind.answers_spec` |
| `answered-ignores-grounded` | `bind.rs` | bind: an ungrounded reference never answers | `Bind.answered_from_loop_spec` |
| `item-under-ignores-answer` | `bind.rs` | bind: an unanswered supported row is stale | `Bind.item_under_spec` |
| `holds-skips-unreadable` | `pin.rs` | pin: an unreadable pin never holds | `Pin.holds_from_loop_spec` |
| `timed-default-never-ages` | `pin.rs` | pin: a catalog default lifetime ages like a token's own | `Pin.timed_spec` |
| `nothing-found-ignores-pins` | `pin.rs` | pin: a negative look's pins decide whether it still found nothing | `Pin.nothing_found_spec` |
| `current-under-ignores-about` | `pin.rs` | pin: pins never make a receipt about other bytes current | `Pin.current_under_spec` |
| `answered-reads-past-end` | `bind.rs` | bind: the total theorem rejects an out-of-bounds read | `Bind.answered_from_loop_spec`, `Bind.answered_from_total` |
| `holds-reads-past-end` | `pin.rs` | pin: the total theorem rejects an out-of-bounds read | `Pin.holds_from_loop_spec`, `Pin.holds_from_total` |
| `newer-reads-past-end` | `idle.rs` | idle: a position past the end has no stamp, so it is never newer | `Idle.newer_total`, `Idle.newer_past_end` |
| `better-past-end-underflows` | `verdict/recorded.rs` | recorded: an index past the end keeps the best so far | `Recorded.better_total`, `Recorded.better_past_end` |
| `char-step-stretches-out` | `scope.rs` | scope: a `*` stretch from a cursor outside the pattern or text is no match | `Scope.char_step_total`, `Scope.char_step_outside` |
| `path-step-stretches-out` | `scope.rs` | scope: a `**` stretch from a cursor outside the pattern or path is no match | `Scope.path_step_total`, `Scope.path_step_outside` |
| `any-stale-skips-last` | `checklist.rs` | checklist: one stale entry is enough, the last one too | `Checklist.any_stale_loop_spec` |
| `newest-skips-last` | `idle.rs` | idle: the newest stamp may be the last one | `Idle.newest_loop_spec` |
| `any-unpinned-skips-last` | `policy.rs` | policy: an unpinned last gate counts | `Policy.any_unpinned_loop_spec` |
| `any-waived-skips-last` | `policy.rs` | policy: a waived last gate counts | `Policy.any_waived_loop_spec` |
| `any-attested-skips-last` | `policy.rs` | policy: an attested last gate counts | `Policy.any_attested_loop_spec` |
| `any-open-skips-last` | `policy.rs` | policy: an open last step counts | `Policy.any_open_loop_spec` |
| `all-passed-skips-last` | `policy.rs` | policy: the last step must have passed too | `Policy.all_passed_loop_spec` |
| `best-recorded-skips-last` | `verdict/recorded.rs` | recorded: the last recorded statement is a candidate | `Recorded.best_recorded_loop_spec` |
<!-- end generated -->

What these proofs do not cover: the fold from event files to the list of
steps (and with it where a splice is placed), template and definition
parsing, digests, Git and file reads, the write protocol
(`spec/KbWrite.tla` and the kill tests), decoding text into code points
for scope globs and `find`, the span comparison `locate` drives, the
command reader, `find`'s name forms and the order of its tiers, `diag`'s
rendering and the bounded walk's traversal (`walk`). Those are tested. The
adapters in `writfold-kb` that read files into kernel inputs (which
receipts and statements are valid and in this run, which gates a step
names, which step path is current, the digests themselves) are tested,
not proved. Statements (`gate attest`, `gate waive`, a delivery's
statements) are ordered by (`seq`, name) since v0.6.0-dev.12, and the one
that stands is `order::newest`.

## Trust base

The Charon/Aeneas translation of the Rust (pinned: Charon `8cae8fd`, Aeneas
`nightly-2026.11.07-c5eb4a2`), the Aeneas library's models of the Rust
operations used, the Lean kernel and the three standard axioms.
`writfold-kb`'s plan fold, receipt, statement and cover-row reading, digests,
citation checks and the guard's lookups, which gather the facts the kernels
decide on, are tested, not proved.

## Found by the proofs

In the archived ledger kernel, the sequence check could overflow for a
selection of `usize::MAX` entries, and the inventory epoch increment could
overflow; both were rewritten to refuse instead.

Found while moving the delivery decisions (16 November 2026): a delivery
mode that declares nothing was "not met" for a plan step but `ready` for
`deliver status` and the delivery guard, which let such a candidate's bytes
through. `ready` is now `delivery::ready`, the same rule as the step
(`ready_iff`, `empty_never_ready`); such a candidate reads `[blocked]` with
the reason and `deliver status` exits 2
(`crates/writfold-kb/tests/deliver_empty_mode.rs`, which failed before the
change).

Found while proving the second round (17 November 2026), each with a test
that failed before the change:

- **A `*` in a path stopped a `*` in the pattern.** The glob matcher
  compared a literal before recording a `*`, so `*a` did not cover the
  segment `*ba`: a file with a literal `*` in its name could fall outside
  its gate's scope digest, and an old receipt kept satisfying. The
  wildcard is now tried first (`scope/tests.rs`,
  `a_star_in_the_path_does_not_stop_a_star_in_the_pattern`).
- **Two clock subtractions could overflow.** A timed citation's expiry
  (`now - seen > ttl`, in `writfold-kb`) and an idle run's age
  (`now - last`) subtracted two `i64` values; the translation demands a
  bound. Both are now compared over the integers (`verdict::expired`,
  used by `idle` too) and the age shown is held inside `i64`
  (`idle.rs`, `far_apart_stamps_do_not_overflow`).
- **Assessed and delivery receipts were ordered by their clock.** A
  receipt written after a clock ran back did not decide. They now carry
  `seq` and are ordered by `(seq, name)` like gate receipts; a receipt
  without `seq` reads as 0 (`tests/receipt_order.rs`).

Noted, not changed: a receipt's content counts as the same only when the
present content is not empty, while a statement or a cover row compares the
digests alone (`gate::about`). The present content is empty only when a
run-record digest could not be read; both rules are now stated as theorems
(`evidence_spec`, `about_spec`) rather than left implicit.

## Not yet proved (backlog, 17 November 2026)

[`proof-backfill-spec.md`](../rfcs/2026-11-16-proof-backfill.md) steps 0 to 11 are built.
What is left, each tested:

1. *(20 November 2026: proved. `decision::flags` decides which flags a
   change takes and `decision::writes` whether the facts the reads
   established are enough; `write/decide.rs` makes the reads and asks
   them, [#14](https://github.com/writfold-dev/writfold/issues/14).)* The
   reads themselves (the commit on its ref, the evidence grounding, the
   successor present, readiness) stay tested.
2. *(21 November 2026: proved. Statements carry `seq` and are ordered by
   `order::cmp`; `order::newest` chooses the one that stands,
   [#15](https://github.com/writfold-dev/writfold/issues/15).)* The
   next-`seq` read and the file codec stay tested.
3. The splice placement in the fold (G10's first item; relabelled, not
   proved; [#17](https://github.com/writfold-dev/writfold/issues/17)).
   *(20 November 2026: `find`'s tier order is proved: `verbs/find.rs`
   gathers every tier's hits in order and `find::show` decides the rows,
   `proofs/Writfold/Show.lean`; [#16](https://github.com/writfold-dev/writfold/issues/16).)*

## Writing kernels for translation

These rules apply to the functions named in `start_from`; the text
plumbing, rendering and traversal outside it (`command`'s reader, `find`'s
name forms, `diag`'s rendering, `walk`'s traversal, `locate`'s driver) need
not follow them. Owned data and no nested borrows; no derives in translated modules (equality a
theorem relies on is an explicit `same`); no integer casts; explicit `while`
loops; one loop per function when it returns early (Aeneas does not translate a
`return` inside nested loops); conveniences outside
`[package.metadata.charon] start_from`. A module that must keep derives for
its callers (`gate`, `assess`) is named function by function in `start_from`,
so the derives are never reached; inside a translated function, compare
enums with `matches!`, not a derived `==`, and keep a loop's early-return
match flat (Aeneas refuses to expand a nested enum pattern there). A loop
whose body updates several variables under nested `if`s may fail to
translate ("could not match the contexts"); give each test its own small
function and keep one flat `if` per variable.
