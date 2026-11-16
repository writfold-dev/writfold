# Proof backfill: what is proved, what is not, and the order to close it

Status: assessment and plan, 16 November 2026, against `main` at `e489d98` (v0.6.0-dev.2). Steps 0 to 5 (the first round, up to the cut line) are built; as built: [design §43](kb-v1-design.md#43-proof-backfill-first-round-as-built-16-november-2026) and [aeneas.md](aeneas.md#the-proof-backfill-first-round-16-november-2026). Steps 6 to 11 are not built. The sections below are the plan as written; where the build differs (names such as `gate::Recorded` and `gate::Said`, `guard::call` and `guard::require`, the theorem names) the as-built pages are authoritative.

`AGENTS.md` ("Lean proofs first", 16 November 2026) requires every product decision (met, stale, ready, skipped, complete, refused, allowed) to live in a pure kernel in `crates/writfold-core`, extracted with Charon/Aeneas and proved in Lean. Parsers, IO, rendering and numeric plumbing stay tested-only. This document measures the distance to that rule and orders the work.

How it was produced: by reading `crates/writfold-core`, `proofs/`, `spec/` and the decision paths of `crates/writfold-kb`. Nothing was run: no proofs gate, no tests. The Lean statements in section 2 are sketches in the style of `proofs/Writfold/*.lean`; none has been compiled. The Docker commands in section 4 are derived from `proofs/Dockerfile` and `proofs/gate.sh` and were not executed.

## Summary

| | Count |
|---|---|
| Decision functions proved (in `start_from`, with theorems) | 21, in 4 modules (`bytes`, `plan`, `gate`, `assess`) |
| Theorems / audited headline claims / mutants | 62 / 33 / 20 |
| Decision functions in `writfold-core` that are tested only | 37, in 13 modules |
| Decision sites in `writfold-kb` with no core kernel | 21 (K1–K21 below) |

The four proved kernels are proved well: most theorems are exact functional specifications over all inputs. The weakness is at their edges:

1. **The kernels take booleans that `writfold-kb` computes.** "A receipt about other bytes never satisfies" is proved about a flag named `same_scope`; the digest comparison that sets the flag is in `writfold-kb`, tested only.
2. **Three paths reach "met" through unproved code**: checklist gates (a `writfold-kb` mapping onto the attested class), deliveries (`delivery::standing`, tested only) and the guard (`guard::decide`, `gates_fact`, tested only).
3. **The splice theorems are about a function production never calls.** `plan::splice` is called only from `crates/writfold-kb/tests/plan_splice.rs`; the product places steps in the fold.
4. **One inconsistency was found by reading** (section 2, G3): a delivery mode that declares nothing is "not met" for a plan step and "ready" for `deliver status` and the delivery guard.

## 1. Inventory

### 1.1 Proved kernels

All theorems are total-correctness triples over the extracted functions unless marked "model lemma" (a statement about a Lean definition only, tied to the Rust through another theorem).

**`writfold_core::bytes`** (`proofs/Writfold/Bytes.lean`)

| Function | Theorems | Strength | Mutants |
|---|---|---|---|
| `same` | `same_spec`: result ↔ the two byte lists are equal | Strong | `same-ignores-last-byte` |
| `same_option` | `same_option_spec`: result ↔ the options are equal | Strong, but **no production caller** | none |
| `contains_digest` | `contains_digest_spec`: result ↔ membership | Strong, but **no production caller** | none |

`same` is reached in production only through `plan::contains`. `same_option` and `contains_digest` are left from kernels that were archived; they should be deleted or given a caller.

**`writfold_core::plan`** (`Plan.lean`)

| Function | Theorems | Strength | Mutants |
|---|---|---|---|
| `closed` | `closed_spec` | Strong | `closed-ignores-skip` |
| `current` | `current_spec`, `current_least`: the least index neither done nor skipped; `None` ↔ all closed | Strong | only through `closed`; none on the loop (for example, returning the last open step) |
| `first_owed` | `first_owed_spec` | Strong | none of its own |
| `advance_check` | `advance_check_spec` (every branch, in order), `advance_ok_iff` | Strong as stated; see "as wired" below | `advance-without-current-check`, `advance-ignores-gates` |
| `stale` | `stale_spec`: ↔ done and some gate not met; `stale_never_current` | `stale_spec` strong. `stale_never_current` is **near-vacuous**: it follows from "stale implies done" alone and would hold for any `stale` that implies `done`, whatever it does with gates | `stale-ignores-done` |
| `contains` | `contains_spec` | Strong | none of its own |
| `splice_check` | `splice_check_spec`, `splice_check_already_iff`, `splice_idempotent`, `splice_check_depth`, `splice_admitted` | Strong for the returned value. `splice_idempotent` proves "returns `Already`"; that nothing is then written is `writfold-kb` | `splice-ignores-key`, `splice-depth-off-by-one`; none for the empty-list and complete branches |
| `splice` | `splice_spec` (for lists whose new length fits `usize`); model lemmas `splice_keeps_existing`, `splice_keeps_prefix`, `splice_new_open`, `splice_current`, `splice_none` | `splice_spec` strong, **about a function production does not call**. Five of the 33 headline claims are model lemmas about `spliceList` | three splice mutants |

As wired, two of these are weaker than they read:

- `plan advance` calls `advance_check(nodes, at, met)` with `at` set to the kernel's own `current` (`plan/write.rs`). The `NotCurrent` branch cannot occur, and the "is the current step" half of `advance_ok_iff` is trivially true. What decides the outcome is the `met` vector, which `writfold-kb` builds (K1).
- `plan use` discards the index in `Spliced::Insert(_)` (`plan/place.rs`). Where the new steps go is decided by the fold's post-order flattening (`plan/fold.rs`). `splice_spec`, the model lemmas and the conclusion of `splice_admitted` describe a model of the fold; one test compares the two.

**`writfold_core::gate`** (`Gate.lean`)

| Function | Theorems | Strength | Mutants |
|---|---|---|---|
| `outcome` | `outcome_spec`, `outcome_pass_iff`, `outcome_fail_iff` | Strong | `outcome-ignores-moved`, `outcome-any-exit-passes`; none for "another exit status is a failure, not unknown" |
| `state` | `state_spec`, `state_satisfied_iff`, `state_other_subject` | Strong over `Evidence`; the flags `same_definition` and `same_scope` are inputs | `state-ignores-scope`; none for ignoring the definition, for oldest-first, or for a failed receipt mapping to `Satisfied` |
| `standing` | `standing_spec`, `standing_satisfied_iff`, `standing_attested_iff`, `standing_waived_iff` | Strong | `waiver-satisfies`, `attestation-satisfies`; none for an attested-class obligation standing on its checked state |
| `Standing::met` | `met_spec`, `standing_met_iff`, `satisfied_only_by_current_pass` | Strong | `met-counts-stale` |

**`writfold_core::assess`** (`Assess.lean`)

| Function | Theorems | Strength | Mutants |
|---|---|---|---|
| `assumption` | `assumption_spec` | Strong | `assumption-ignores-attested`; none for "cites nothing is ungrounded" |
| `basis` | `basis_spec` | Strong | `basis-ignores-premises` |
| `met`, `found_met`, `unfinished` | `met_spec`, `found_met_spec`, `unfinished_spec` | Strong | `met-accepts-hypothetical` |
| `outcome` | `outcome_spec`, `outcome_pass_iff`, `hypothetical_never_passes` | Strong | `outcome-passes-unfinished`; none for an incomplete run or an empty requirement list passing |

**The gate itself.** `proofs/Audit.lean` audits every theorem under `Writfold.Core` and requires 33 names. Six theorems that mutants name as their failing theorem (`closed_spec`, `outcome_spec`, `standing_spec` and the loop specifications) are not among the required names; a rename is caught only by the mutant run. The extracted file holds no `axiom` or opaque declaration. The proofs gate is not one of the `.writfold/gates/` definitions; it runs only when someone runs the Compose service.

### 1.2 In `writfold-core`, tested only

| Module | Decision functions | What a wrong answer does | Translation obstacles |
|---|---|---|---|
| `checklist` | `item`, `covered`, `rollup` | An uncovered checklist reads covered, so its gate is met | Derives (name the functions one by one in `start_from`); tuple slice in `rollup` |
| `delivery` | `deciding`, `standing` | A delivery is met on an unmet obligation, or judged on a bound candidate | `for` loop; nested pattern with an early return inside the loop |
| `guard` | `all_met`, `decide`, `combine`, `effect`, `gates_fact`, `nudge`, `nudge_with` | A guarded command passes | Derived `!=` and `==` on enums |
| `observe` | `embeds`, `grounds` | A failed look grounds evidence | None |
| `decision` | `step` | A state change is written without what it needs | Tuple match over two enums; `parse` stays out |
| `investigation` | `conclude`, `Missing::none` | An investigation with open hypotheses is concluded | None expected |
| `verdict` | `observed`, `with_mark`, `decide`, `handshake`, `exchange`, `aged`, `lifetime`, `locate` | A changed citation reads fresh | Deep nested patterns; a slice pattern; a match guard; a generic; `locate` takes a closure and casts integers |
| `verdict::recorded` | `recordable`, `recorded_age`, `newest` | A stale shared verdict is used | `newest` is iterator chains with `min_by_key`; saturating arithmetic |
| `scope` | `in_scope`, `glob` (with `segments`, `segment`) | A changed file falls outside a gate's scope, so an old receipt still satisfies | Iterators, closures, recursion, `char` |
| `walk` | `walk`, `Walk::complete` | `coverage=complete` is claimed for a partial walk | `BTreeMap`, `BTreeSet`, `String`, closures |
| `command` | `matches` (the rest is a reader) | A guarded command is not recognised | Iterators; strings |
| `idle` | `newest`, `idle` | A run is listed idle or not (a filter; decides no state) | `?`, `then_some`; `now - last` can overflow |
| `find` | `prefix_query`, `prefix_match` (`norm`, `tokens` are text plumbing) | A name does or does not match by prefix | Iterators; `char` |
| `diag` | `Diagnostic::is_error` and the counts | An error is read as a warning | Rendering; stays tested-only apart from the severity test |

The module headers say why each was left: "the tests below enumerate it, and that enumeration is the proof" (`guard`), "the input space is small" (`delivery`), "proof candidate" (`verdict`, `walk`). Under the new rule none of these reasons stands.

### 1.3 Decided in `writfold-kb`, with no kernel

| | Where | What it decides |
|---|---|---|
| K1 | `plan/judge.rs` `Gate::met`, `plan/write.rs` `advance` | The `met` vector: a gate with no usable definition is not met; a step with no gates is met; which index is asked about |
| K2 | `plan/write.rs` `skip` | Whether a step may be skipped (run open, a current step exists, the end guard) |
| K3 | `plan/write.rs` `complete_guard`, `plan/required.rs` `missing_end` | Whether the last open step may close while a required end template is missing |
| K4 | `plan/write.rs` `open_run`, `runs.rs` `state` | A closed run refuses plan writes; the sidecar state wins over the manifest |
| K5 | `plan/recheck.rs` `changed` | Whether a decision still stands under the write lock |
| K6 | `plan/place.rs`, `plan/fold.rs`, `plan/disorder.rs` | Where a splice goes; which keys count for idempotence (the step, its ancestors, the root); the three disorder flags |
| K7 | `gate/status.rs` `judge_gate`, `Filings::valid`, `gate/receipt.rs` `order` | Which receipts are about this gate and repository; `same_definition`; `same_scope` (including "empty content is never the same"); the order by `seq` then name |
| K8 | `gate/statement.rs` `summary` | Which statements are current (definition and content digests equal); `attested_any` |
| K9 | `gate/status.rs` `Report::all_met`, `Filings::invalid` | "Every gate met" (false for none); the invalid count |
| K10 | `gate/checklist/judge.rs` | A cover row is current; cited evidence moved (ungrounded, or its standing word changed); the mapping of a rollup onto `standing(true, …)` with `attested_current = covered` and `attested_any = stale` |
| K11 | `gate/assessed/judge.rs` | An assessed receipt is about the present (definition, source digest, no citation moved); receipts ordered by recorded time, then name |
| K12 | `gate/ground.rs` `resolve` | What grounds evidence: a gate reference (a waiver never does), a receipt (five ordered conditions), an attestation, a retrieval, an observation, an edit receipt, a citation token (exactly one, fresh) |
| K13 | `guard/require.rs` `fact`, `step`, `gates_met`, `judged` | Each requirement's fact: `run.current` is met only for an open run; `step.met` is met with no plan or a complete plan; an undefined gate is unmet; which errors are `Unknown`; the `repos` filter that skips a rule |
| K14 | `guard/pre.rs` `judge`, `logged`; `edit/guarded.rs` | Folding rule decisions into one; when a decision is logged |
| K15 | `guard/delivery.rs` `decide` | The delivery guard: allow when a ready candidate holds the bytes, or when every candidate is ready and there is at least one |
| K16 | `deliver/status.rs` `ready`, `deliver/mod.rs` `readiness`, `deliver/recheck.rs` `apply`, `deliver/step.rs` `view` | A candidate is ready; a passed claims receipt goes stale when a citation moved (and when the re-check fails); a mode that declares nothing is open; receipts ordered by recorded time |
| K17 | `write/decide.rs` `given`, `ready`, `grounded`, `landed`, `successor_of` | Whether what `decision::step` needs was supplied: review readiness, the commit on the target ref, evidence that is grounded and is a gate receipt, no supersession cycle |
| K18 | `write/conclude.rs` | Composes section presence with `investigation::conclude` |
| K19 | `stale/` (`code.rs`, `kinds.rs`, `mod.rs`), `time.rs` `expired` | The facts handed to `verdict::observed`; expiry (`now - seen > ttl`) |
| K20 | `verbs/find.rs`, `verbs/names.rs` | Tier order and merge of `find` hits |
| K21 | `defs/` (`lint_refs.rs`, `lint_out.rs`, `mod.rs`) | Whether a diagnostic blocks (any error) |

Closing a run (`write/status.rs`) checks only the vocabulary today; it makes no decision about an incomplete plan.

## 2. Gaps, ranked by risk to users

A wrong "met", "ready" or "allowed" is worst; a wrong "stale" or "refused" costs a re-run. Each gap lists the kernel shape, the theorems and the mutants. All new functions are named one by one in `start_from`.

### G1. A receipt is "about the current subject" (K7, K8, K1, K9)

The central sentence of `docs/guide/assurance.md`, "a receipt about another definition or other bytes never satisfies", is proved only relative to two booleans.

Move into `writfold_core::gate`:

```rust
pub struct Subject<'a> { pub definition: &'a [u8], pub content: &'a [u8] }
/// Both digests equal, and the present content is not empty.
pub fn current(recorded_definition: &[u8], recorded_content: &[u8], now: &Subject) -> bool
/// Receipts oldest first, each with its recorded digests.
pub struct Filed { pub outcome: Outcome, pub definition: Vec<u8>, pub content: Vec<u8> }
pub fn judge(receipts: &[Filed], now: &Subject) -> (State, Option<usize>)
pub fn statements(filed: &[Said], now: &Subject) -> Statements   // Said { attested: bool, definition, content }
```

`judge` is `state` with the comparison inside it; the existing `state` stays for the assessed gate and deliveries until G3 and G5 move them. If borrowed struct fields trouble the translation, pass the two digests as separate slice arguments.

And into `writfold_core::plan`:

```rust
pub enum GateFact { Undefined, Stands(Standing) }
pub fn advance(nodes: &[Node], at: usize, gates: &[GateFact]) -> Result<(), Refusal>
```

Theorems:

```lean
theorem current_iff : gate.current d c now ⦃ r => r = true ↔
    d.val = now.definition.val ∧ c.val = now.content.val ∧ now.content.val ≠ [] ⦄
theorem judge_satisfied_iff : gate.judge rs now ⦃ r => r.1 = .Satisfied ↔
    ∃ i (h : i < rs.val.length), About rs.val[i] now ∧ rs.val[i].outcome = .Pass ∧
      ∀ j (hj : j < rs.val.length), i < j → ¬ About rs.val[j] now ⦄
theorem judge_other_bytes (h : ∀ r ∈ rs.val, r.content.val ≠ now.content.val) :
    gate.judge rs now ⦃ r => r.1 ≠ .Satisfied ⦄
theorem statements_spec : gate.statements fs now ⦃ s =>
    (s.attested_current = true ↔ ∃ f ∈ fs.val, f.attested = true ∧ AboutS f now) ∧
    (s.waived_current = true ↔ ∃ f ∈ fs.val, f.attested = false ∧ AboutS f now) ∧
    (s.attested_any = true ↔ ∃ f ∈ fs.val, f.attested = true) ⦄
theorem advance_ok_iff' : plan.advance nodes pos gates ⦃ r => r = .Ok () ↔
    cur nodes.val = some pos.val ∧ ∀ g ∈ gates.val, ∃ s, g = .Stands s ∧ MetS s ⦄
theorem done_only_when_met :   -- the assurance sentence, end to end
    composition of judge, statements, standing and advance: Ok → every gate is
    Stands of Waived, Attested, or Checked Satisfied with a passing newest receipt
    whose digests equal the present ones
```

Mutants: `current` ignoring the definition; ignoring the content; accepting empty content; `judge` scanning oldest first; `statements` counting a moved waiver as current; `advance` treating `Undefined` as met; `advance` on an empty gate list refusing.

What stays in `writfold-kb`: parsing receipts, filtering by gate and repository, the sort by `(seq, name)`. The sort is the next thing to move: a kernel `newest_first(seq_a, name_a, seq_b, name_b)` comparator with a total-order theorem is half a day and removes the last adapter decision.

### G2. Checklist gates (core `checklist`, K10)

A checklist becomes "attested" (met) through a hand-written mapping in `writfold-kb`. Move the mapping into core:

```rust
pub fn standing(rollup: Rollup, waived_current: bool) -> crate::gate::Standing
pub fn row_grounded(moved: &[bool]) -> bool      // no cited reference moved
```

Theorems:

```lean
theorem item_covered_iff : (do let i ← checklist.item row; checklist.covered i) ⦃ b => b = true ↔
    ∃ r, row = some r ∧ r.current = true ∧
      ((r.outcome = .Supported ∧ r.evidence.val > 0 ∧ r.grounded = true) ∨ r.outcome = .NotApplicable) ⦄
theorem rollup_covered_iff : checklist.rollup items ⦃ r => r.covered = true ↔
    ∀ c ∈ items.val, c.2 = true → CoveredI c.1 ⦄
theorem rollup_first_owed : … r.first_owed is the least required uncovered index
theorem rollup_optional_irrelevant : changing an item with required = false leaves the result unchanged
theorem standing_attested_iff : checklist.standing r w ⦃ s => s = .Attested ↔ r.covered = true ⦄
theorem standing_never_satisfied : checklist.standing r w ⦃ s => s ≠ .Checked .Satisfied ⦄
theorem standing_met_iff : … met ↔ r.covered = true ∨ w = true
```

Reshape: none expected beyond naming the functions; if the tuple slice does not translate, use `struct Cell { item, required }`.

Mutants: `item` accepting a supported row with no evidence; accepting ungrounded evidence; accepting a row that is not current; `covered` counting `Unsupported`; `rollup` ignoring `required`; `standing` returning `Attested` when stale.

### G3. Deliveries and the delivery guard (core `delivery`, K15, K16)

Found by reading: `delivery::standing(&[])` is `Checked(Open)` ("an empty set is not met"), and `deliver/step.rs` shows a mode that declares nothing as open. But `deliver/status.rs` `ready` is `all(met)`, true for no obligations, and `readiness` returns no owed items, so `deliver status` shows such a candidate `ready` and the delivery guard lets its bytes through. One definition must serve all three. Recommended: `ready` is `delivery::standing(..).met()`; this changes `deliver status` and the guard for an empty mode and needs a test and a changelog line. Confirm with a failing test first; this was read, not run.

Move into core:

```rust
pub fn ready(obligations: &[Standing]) -> bool                    // standing(..).met()
pub fn rechecked(checked: State, moved: usize) -> State           // Satisfied and moved > 0 is Stale
pub enum Body { Known, Unknown }
/// `same_ready`: some candidate holding the posted bytes is ready.
pub fn guard(body: Body, same_ready: bool, candidates: usize, all_ready: bool) -> bool
```

Theorems:

```lean
theorem standing_met_iff : (do let s ← delivery.standing obs; gate.Standing.met s) ⦃ b => b = true ↔
    obs.val ≠ [] ∧ ∀ o ∈ obs.val, MetS o ⦄
theorem standing_satisfied_iff : delivery.standing obs ⦃ s => s = .Checked .Satisfied ↔
    obs.val ≠ [] ∧ ∀ o ∈ obs.val, o = .Checked .Satisfied ⦄
theorem standing_first_unmet : not met → the result is the first unmet obligation's standing
theorem deciding_spec : delivery.deciding bound ⦃ r => r is the greatest index with bound[i] = false; none ↔ all bound ⦄
theorem ready_iff : delivery.ready obs ⦃ b => b = true ↔ obs.val ≠ [] ∧ ∀ o ∈ obs.val, MetS o ⦄
theorem guard_allow_iff : delivery.guard body sr n ar ⦃ b => b = true ↔
    (body = .Known ∧ sr = true) ∨ (body = .Unknown ∧ n.val > 0 ∧ ar = true) ⦄
```

Reshape `standing`: a `while` loop, and the loop body's nested pattern becomes a flat test through a helper (`fn unmet(s: Standing) -> bool`), returning `obligations[i]`.

Mutants: empty set met; a waived obligation yielding `Checked(Satisfied)`; `deciding` returning the first unbound; `deciding` ignoring `bound`; `guard` allowing with no candidate; `guard` allowing unknown bytes when one candidate is ready; `rechecked` keeping `Satisfied`.

Also noted: delivery receipts, assessed receipts and statements are ordered by recorded time (a clock); gate receipts and cover rows by `seq`. The comparator from G1 should replace all of them when delivery and assessed receipts gain a `seq`.

### G4. Guard decisions (core `guard`, K13, K14)

Move the fact assembly into core:

```rust
pub enum Looked { No, Failed, Read }                 // was the thing judged, and could it be read
pub fn run_current(found: bool, open: bool) -> Fact
pub fn step_fact(read: Looked, planned: bool, has_current: bool, owed: usize) -> Fact
pub fn gate_fact(read: Looked, defined: bool, met: bool) -> Fact
pub fn decide_all(rules: &[Ruled]) -> Decision       // Ruled { mode, action, facts-all-met: bool }
```

Theorems:

```lean
theorem decide_pass_iff : guard.decide m a fs ⦃ d => d = .Pass ↔ m = false ∨ ∀ f ∈ fs.val, f = .Met ⦄
theorem unknown_never_passes (h : Fact.Unknown ∈ fs.val) : guard.decide true a fs ⦃ d => d ≠ .Pass ⦄
theorem combine_deny_iff, combine_pass_iff; combine is commutative, associative, Pass is its unit
theorem effect_audit_passes : guard.effect .Audit d ⦃ r => r = .Pass ⦄ ; effect_enforce : … r = d
theorem gates_fact_met_iff : guard.gates_fact o u i ⦃ f => f = .Met ↔ o.val > 0 ∧ u.val = 0 ∧ i.val = 0 ⦄
theorem gates_fact_never_unknown
theorem nudge_iff : guard.nudge_with r a n fs ⦃ b => b = true ↔
    a = false ∧ ¬ (n = true ∧ r = .Session) ∧ ∃ f ∈ fs.val, f ≠ .Met ⦄
theorem step_fact_met_iff : … = .Met ↔ read = .Read ∧ (planned = false ∨ has_current = false ∨ owed.val = 0)
theorem gate_fact_met_iff : … = .Met ↔ read = .Read ∧ defined = true ∧ met = true
theorem decide_all_pass_iff : … = .Pass ↔ every enforcing rule's facts are all met
theorem decide_all_deny_iff : … = .Deny ↔ some enforcing deny rule has an unmet fact
```

Reshape: replace derived `!=` and `==` with `matches!`.

Mutants: `all_met` accepting `Unknown`; `decide` passing a matched rule with no facts met; `combine` preferring `Ask` over `Deny`; `effect` enforcing an audit rule; `gates_fact` met on nothing owed; met with invalid filings; `nudge` firing while a stop hook is active; `step_fact` met when the plan could not be read.

`step_fact` writes down a rule that is easy to miss: `step.met` holds for a run with no plan, while `gates.met` does not hold for a run that owes nothing. If that asymmetry is not intended, the theorem is where it gets fixed.

Which command a rule matches (`command::matches`, paths, `repos` globs) is G10 and G6.

### G5. What grounds evidence (K12, core `observe`)

Grounding feeds the assessed gate's premises (proved), checklist rows (G2) and `status --set verified` (G8). "A waiver is never evidence" is a `match` arm in `writfold-kb`.

New module `writfold_core::ground`:

```rust
pub fn of_gate(found: bool, standing: Standing) -> Grounding
pub struct ReceiptFacts { same_subject, same_definition, same_content, passed, gate_met: bool }
pub fn of_receipt(f: ReceiptFacts) -> Grounding
pub fn of_citations(count: usize, not_fresh: usize) -> Grounding     // retrievals; tokens with count = 1
pub fn of_observation(status: observe::Status, count: usize, not_fresh: usize) -> Grounding
pub fn of_statement(current_attestation: bool) -> Grounding
pub fn moved(now: Grounding, same_word: bool) -> bool                // a cover row's reference moved
```

Theorems:

```lean
theorem of_gate_grounded_iff : ground.of_gate f s ⦃ g => g = .Grounded ↔ f = true ∧ s = .Checked .Satisfied ⦄
theorem of_gate_attested_iff : … g = .Attested ↔ f = true ∧ s = .Attested
theorem waiver_never_grounds : ground.of_gate f .Waived ⦃ g => g = .Ungrounded ⦄
theorem of_receipt_grounded_iff : … g = .Grounded ↔ all five flags are true ; of_receipt_never_attested
theorem of_citations_grounded_iff : … g = .Grounded ↔ count.val > 0 ∧ not_fresh.val = 0
theorem of_observation_grounded_iff : … g = .Grounded ↔ status = .Ok ∧ count.val > 0 ∧ not_fresh.val = 0
theorem grounds_iff : observe.grounds s c ⦃ b => b = true ↔ s = .Ok ∧ c = true ⦄
theorem embeds_iff : observe.embeds s ⦃ b => b = true ↔ s = .Ok ∨ s = .Partial ⦄
```

Mutants: a waived gate grounding; a receipt grounding when the content differs; when the gate is not met; a `Partial` observation grounding; an observation with no citation grounding; zero citations grounding.

The standing words (`"satisfied"`, `"not-fresh:3"`) are rendering and stay in `writfold-kb`. Edit receipts (`holds` or `changed`) reduce to one boolean and can use `of_citations`-style counting; do it only if it stays that simple.

### G6. Scope globs (core `scope`)

A pattern that fails to match a changed file leaves that file out of the scope digest, and an old receipt keeps satisfying. This is a false "met" and the largest unproved computation.

Reshaped design (behaviour unchanged, including `?` matching one character):

- The caller decodes pattern and path to code points and splits on `/` (text plumbing, tested): `&[Vec<u32>]` each.
- `segment(pattern: &[u32], text: &[u32]) -> bool` stays the existing two-index loop with one backtrack point; it is already a single `while` with no closure.
- `segments(wanted: &[Vec<u32>], parts: &[Vec<u32>]) -> bool` becomes the same two-index loop one level up, with `**` in the role of `*` and `segment` in the role of character equality. One backtrack point is enough because `**` is the only wildcard at that level. This replaces the recursion and the closure.
- `in_scope(patterns: &[Pattern], path: &[Vec<u32>]) -> bool` with `struct Pattern { exclude: bool, segments: Vec<Vec<u32>> }`, one `while` loop.

Theorems, against an inductive specification:

```lean
inductive SegMatch : List U32 → List U32 → Prop     -- [] [] ; '?' :: p, c :: t ; c :: p, c :: t (c ≠ '*','?') ; '*' :: p with any split
inductive PathMatch : List (List U32) → List (List U32) → Prop   -- "**" matches any number of segments, none included
theorem segment_iff : scope.segment p t ⦃ b => b = true ↔ SegMatch p.val t.val ⦄
theorem segments_iff : scope.segments w ps ⦃ b => b = true ↔ PathMatch (w.val.map (·.val)) (ps.val.map (·.val)) ⦄
theorem in_scope_iff : scope.in_scope pats path ⦃ b => b = true ↔
    (∃ p ∈ pats.val, p.exclude = false ∧ PathMatch p.segs path) ∧
    (∀ p ∈ pats.val, p.exclude = true → ¬ PathMatch p.segs path) ⦄
theorem exclusion_wins, no_pattern_no_scope      -- corollaries
```

The greedy-backtrack proofs are the hardest in this plan (the invariant is "no match exists that starts the star earlier than the recorded point"). If they overrun, land `in_scope_iff` over an abstract matcher first (half a day) and keep the matcher tested for one more release.

Mutants: `in_scope` including when an exclusion matches; including with no positive pattern; `segment` not backtracking; `*` crossing nothing (must match empty); `**` requiring at least one segment; `segments` accepting a trailing unmatched part.

### G7. Plan closing writes (K2–K5)

```rust
pub enum Close { Ok, RunClosed, Complete, MissingEnd(usize) }
/// `missing_end`: the required end templates not yet spliced at the root.
pub fn close_check(run_open: bool, nodes: &[Node], at: usize, missing_end: usize) -> Close
pub enum Decided { Unplanned, Current(usize), Splice { parent_current: bool, ordinal: u64 } }
pub fn still_stands(decided: &Decided, planned: bool, current: Option<usize>, next_ordinal: u64) -> bool
```

Theorems: `close_check` is `Ok` exactly when the run is open, `at` is the current step, and either another step stays open after closing `at` or no required end template is missing; `MissingEnd` only when closing `at` would complete the plan; `still_stands` per case. This is today's behaviour for both `advance` and `skip`. The policy spec is expected to change skipping; see section 5 before building this.

Mutants: closing on a closed run; closing the last step with a required end missing; `still_stands` accepting a moved current step; accepting a taken ordinal.

### G8. Decision states and investigations (core `decision`, `investigation`, K17, K18)

```rust
pub struct Given { ready, actor, commit_on_target, evidence_grounded_receipt, successor_acyclic: bool }
pub enum Refused { NoSuchChange, NotReady, NoActor, NoCommit, NoEvidence, NoSuccessor }
pub fn admit(from: State, to: State, given: Given) -> Result<(), Refused>
```

Theorems:

```lean
theorem step_table : decision.step f t ⦃ r => r = stepOf f t ⦄        -- the eight rows, as a Lean table
theorem never_written : t = .Landing ∨ t = .Regressed ∨ t = .Other → decision.step f t ⦃ r => r = none ⦄
theorem verified_needs_evidence : decision.admit .Applied .Verified g ⦃ r => r = .Ok () ↔ g.evidence_grounded_receipt = true ⦄
theorem admit_ok_iff : … = .Ok () ↔ ∃ n, stepOf f t = some n ∧ every field n requires is true in g
theorem conclude_none_iff : (do let m ← investigation.conclude c s hs; m.none) ⦃ b => b = true ↔
    c = true ∧ s = true ∧ ∀ h ∈ hs.val, h ≠ .Open ⦄
theorem conclude_open_count : m.open = the number of Open hypotheses
```

Mutants: `accepted` without readiness; `verified` without evidence; `superseded → superseded` allowed; a written `landing`; `conclude` ignoring one section; not counting the last hypothesis.

The readiness test itself (sections present, two alternatives, a table header) reads Markdown and stays tested.

### G9. Citation verdicts (core `verdict`, K19)

```lean
theorem observed_fresh_iff : verdict.observed f ⦃ v => v = .Fresh ↔ FreshFact f ⦄     -- the seven fresh facts, listed
theorem changed_never_fresh, gone_never_fresh
theorem with_mark_spec : verdict.with_mark v m ⦃ r => r = if v ∈ {Changed, Gone, Expired} then v else if m then .Marked else v ⦄
theorem mark_never_fresh : verdict.with_mark v true ⦃ r => r ≠ .Fresh ⦄
theorem handshake_agreed_iff ; exchange_said_iff : = .Said s ↔ agreed ∧ clean ∧ readable ∧ replies = [some s]
theorem aged_spec ; lifetime_spec ; recordable_iff
theorem recorded_age_spec : none ↔ checked_at > now + 3600 ; otherwise max 0 (now - checked_at)
theorem newest_spec : the pick minimises (age, local-after-recorded, worst, index) among usable statements; none ↔ none usable
theorem locate_same_iff, locate_at_first, locate_absent_iff       -- over a precomputed match vector
```

Reshape: `observed` split into flat helpers (`code`, `state`, `checker`) if the nested patterns are refused; `exchange` tests `replies.len() == 1` and indexes; `aged` uses `if`; `lifetime` monomorphic over `i64` if the generic does not translate; `recorded_age` with explicit comparisons in place of saturating calls; `newest` as one `while` loop over an explicit `better(a, b)`; `locate` over `usize` inputs and `matches: &[bool]` (one flag per window start, the cited position first). The match vector costs a digest per window where today's search stops at the first hit; if that matters, keep the closure version as the tested driver and prove a step function instead. Expiry (`now - seen > ttl`) and `idle` subtract two `i64` values; the translation will demand a bound or a checked subtraction, and that is a real finding to fix, not to assume away.

Mutants: a moved span reading fresh; `Unsearched` reading fresh; a mark overriding `Changed`; a failed handshake counting a reply; two replies counting; an expired cache entry kept; `newest` preferring the older statement; preferring local on a tie; `recordable` accepting `Unverifiable`.

### G10. The remainder (lower risk)

- **The splice model.** Either the fold calls a core kernel for placement, or the documents stop calling the splice properties proved for the product. Recommended now: relabel (section 3). Later: a core `flatten` kernel over splice keys with `splice_keeps_existing` restated on it; about three days, and only worth it if splicing changes.
- **`walk`.** Prove only the claim: `complete(frontier, skipped) = true ↔ frontier = 0 ∧ skipped = 0`, with `skipped` passed as a total. The traversal stays compared with the reference search.
- **`command::matches`** over `&[Vec<u8>]`: `matches_iff` (prefix word by word, and not the lone `--help`). The reader is a parser and stays tested.
- **`find`**: `prefix_query` and `prefix_match` over byte tokens; a tier-order kernel (`exact` first, then the fixed order, each record once) shared with any later tier.
- **`idle`**: `newest` is the maximum with the first index; `idle` after the overflow fix.
- **`diag`**: `blocks(errors) = errors > 0`; trivial, do it with the tier kernel.

## 3. What the assurance documents get wrong today

`docs/guide/assurance.md`:

1. **Status line** stops at v0.6.0-dev.1 and names "investigation, checklist and guard … decision states and checker verdicts … observations" as tested exhaustively. It omits `delivery`, `idle`, `find`, `diag` and `verdict::recorded`, and "exhaustively" is not true of `command`, `find`, `scope`, `verdict::locate` or `walk`.
2. **The "Tested" table** has no row for `checklist`, `investigation`, `decision`, `observe`, `delivery`, `idle`, `find`, `diag` or the `guard` kernel's own enumeration.
3. **"Put together: a step is recorded as done only when … That sentence is proved for the kernels."** It is proved for a command or attested gate given the two comparison flags. For a checklist gate, a delivery, and any gate with no usable definition, the path to `met` runs through unproved code (K10, `delivery::standing`, K1).
4. **"`plan advance` is admitted exactly for the current step"** is true of the kernel and vacuous as wired: the caller always asks about the current step.
5. **"a splice keeps every existing step … and makes the first of them current"** under "Proved": proved for `plan::splice`, which production does not call. It belongs under "Tested" as "the fold agrees with the proved model step for step", or under "Proved" with that qualifier.
6. **"Each proved claim has at least one mutant."** Not so for `same_option_spec`, `contains_digest_spec`, `outcome_fail_iff` (no mutant makes a non-zero exit unknown), the newest-first and definition halves of `state_spec`, `standing_attested_iff`, the empty and incomplete branches of `assess::outcome`, `current`'s loop, and the empty-list and complete branches of `splice_check`.
7. **"One caveat today: delivery receipts are still ordered by their recorded time."** So are assessed receipts (`gate/assessed/judge.rs`) and statements (`gate/statement.rs`).
8. **"Not proved"** omits the guard, checklist, delivery, grounding, decision and investigation decisions.

`docs/writfold/aeneas.md`:

1. **"Not yet proved"** lists `investigation, checklist, guard, decision, observe, delivery, command` as "tested exhaustively"; it omits `find`, `idle`, `diag` and `verdict::recorded`, and under the new rule the section should be a dated backlog, not a resting state.
2. **"Writing kernels for translation"** reads as the crate's rule. Thirteen of seventeen modules break it (derives, iterators, closures, casts, `BTreeMap`). Say it applies to functions named in `start_from`.
3. **The gate command** is given only as `docker compose … run`; the `docker run --network none` form used on hosts short of networks is not written down (section 4).
4. It does not say that the `bytes` theorems other than `same_spec` have no production caller.

In the crate: `crates/writfold-core/Cargo.toml` describes the crate as "translated to Lean 4 … and proved" and `src/lib.rs` says "the `proofs` gate translates this crate"; both are true of four modules. `Cargo.toml` says "no `dyn`", while `diag` passes function pointers. The headers of `guard.rs`, `delivery.rs`, `verdict.rs`, `walk.rs` and `scope.rs` each justify not proving the module.

## 4. Build plan

For one implementer. Every step has the same shape:

1. **Move or reshape** the Rust; `writfold-kb` calls the new kernel; its tests and golden files are unchanged unless the step says otherwise.
2. **Extract**: add the functions to `[package.metadata.charon] start_from`, regenerate `proofs/Extracted/`.
3. **Prove**: a file per module under `proofs/Writfold/`, imported from `proofs/Writfold.lean`.
4. **Gate**: add the headline theorems to `claims` in `proofs/Audit.lean` and the mutants to `proofs/mutants.py`; run the proofs gate (drift, `lake build`, axiom audit with only `propext`, `Classical.choice`, `Quot.sound`, mutants).
5. **Document**: move the kernel's row from "Tested" to "Proved" in `docs/guide/assurance.md`, update `docs/writfold/aeneas.md` and the module header.

### Running the gate without a Compose network

`docker compose run` creates a project network; `docker build` and `docker run --network none` do not. The image copies the sources in at build time, so rebuild after every Rust or Lean edit. These commands follow `proofs/Dockerfile`, `proofs/gate.sh` and `proofs/refresh.sh`; they were not run for this document.

```sh
# build (the tools and deps layers need the network once; later rebuilds only copy sources and warm the extraction)
docker build -f proofs/Dockerfile --target gate -t writfold-proofs:gate .

# the whole gate: extract, drift check, lake build, axiom audit, mutants
docker run --rm --network none writfold-proofs:gate

# refresh proofs/Extracted (what proofs/refresh.sh does through Compose)
docker run --rm --network none writfold-proofs:gate \
  sh -c 'sh proofs/extract.sh crates/writfold-core /tmp/fresh >&2 && tar -C /tmp/fresh -cf - .' > /tmp/fresh.tar
rm -rf proofs/Extracted && mkdir -p proofs/Extracted && tar -C proofs/Extracted -xf /tmp/fresh.tar

# one stage or one mutant while iterating
docker run --rm --network none writfold-proofs:gate sh -c 'cd proofs && lake build && lake env lean Audit.lean'
docker run --rm --network none writfold-proofs:gate python3 proofs/mutants.py guard-unknown-passes
```

To iterate without rebuilding, keep one container from the image and `docker cp` the changed files in. Do not bind-mount over `/work/proofs`: it hides the prebuilt `.lake`. After any `rustfmt` of the core, refresh: the translation records line numbers. Mutant calibration is almost all of the gate's time (about 13 seconds each today); 20 mutants grow to about 85 under this plan, so the full gate goes from about four and a half minutes to roughly twenty. Run the new mutants by name while iterating and the full set once per step.

### Steps

| # | Step | Days | Notes |
|---|---|---|---|
| 0 | Truth and calibration of what exists | 1 | Fix section 3 in the documents. Add the missing mutants from section 1.1 (about ten). Add every theorem a mutant names to `claims`. Delete `same_option` and `contains_digest` or give them a caller. Relabel the splice claims. Write the `docker run` form into `aeneas.md`. |
| 1 | G1: the current subject, `judge`, `statements`, `advance` on standings | 2.5 | Moves K7, K8, K1. Byte-slice digests; `bytes::same` is reused. Largest proof: `done_only_when_met`. |
| 2 | G2: checklist | 1.5 | Moves K10's mapping. |
| 3 | G3: deliveries and the delivery guard | 1.5 | Starts with a failing test for the empty-mode inconsistency; the fix changes `deliver status` and the guard for that case. |
| 4 | G4: guard | 2 | Moves K13, K14. The conformance cases in `fixtures/guard-conformance/` must still pass unchanged. |
| 5 | G5: grounding and observations | 2 | Moves K12's decisions; words stay. |
| | **Cut line for a first release** | **10.5** | |
| 6 | G6: scope globs | 3.5 | `in_scope_iff` over an abstract matcher first (0.5), then `segment_iff` (1.5), then `segments_iff` (1.5). |
| 7 | G7: plan closing writes | 1.5 | After the policy spec's plan decisions are fixed (section 5). |
| 8 | G8: decisions and investigations | 1.5 | |
| 9 | G9: citation verdicts | 3 | `newest` and `locate` are the reshapes; the overflow fix lands here. |
| 10 | G10: walk's claim, `command::matches`, `find`, `idle`, `diag`, tier order | 2 | |
| 11 | Ordering without a clock: the `(seq, name)` comparator, used for assessed and delivery receipts | 1.5 | Needs a `seq` field in two receipt formats; a format change, so it is last. |
| | **Total** | **24** | |

The order is risk first, with one exception: scope globs (G6) carry a false-"met" risk as large as G2–G5 but cost the most and depend on nothing else, so they come immediately after the cut rather than inside it.

**At the cut line**, proved end to end: a gate stands satisfied only on a passing newest receipt whose recorded digests equal the present ones; a statement stands only when its digests do; a checklist is met only when every required item is covered by a current, grounded row; a delivery is met only when it has obligations and each is met; a guard passes only when the rule did not match or every requirement is met, and never on an unknown; a waiver never grounds evidence. Still tested only at the cut, and to be said so in `assurance.md`: which paths a scope covers, citation verdicts, skipping and the end guard, decision transitions, the splice placement, and everything that reads files.

Sizes assume the pinned toolchain translates the reshaped code as described. The two places most likely to need a second shape are tuple and nested-pattern matches (`decision::step`, `verdict::observed`) and borrowed fields in structs (G1's `Subject`); each has a fallback named in its gap.

## 5. Coordination with the policy spec

`docs/writfold/policy-spec.md` is not in the repository yet. Its announced scope is five policy kernels: accepted outcomes, unskippable required steps, drift, closed-incomplete runs and evidence binding. The overlaps, and a rule for each:

| Policy topic | Touches | Rule |
|---|---|---|
| Accepted outcomes | `gate::outcome`, `outcome_pass_iff`, `outcome_fail_iff`; possibly `assess::outcome` | Owned by the policy spec. This plan adds only the missing mutant for the existing function (step 0). If the policy changes the signature, it replaces those theorems and mutants; it does not add a second set beside them. |
| Unskippable required steps | `plan` skip admission (K2), the end guard (K3) | This is G7. Build G7 only after the policy's signature is fixed, or let the policy spec build `close_check` with its extra input and drop G7's skip half. One function decides skip; not two. |
| Drift | `gate::state` (stale), `plan::stale`, template drift (`plan/moved.rs`), G1's `current` | G1 defines "about the current subject" once (`gate::current`). Policy drift theorems are stated on top of `current_iff` and `judge_satisfied_iff`, not on a second comparison. |
| Closed-incomplete runs | K4 (`open_run`), run closing in `write/status.rs`, `plan::current` | Owned by the policy spec; no decision exists today. G7's `run_open` input is the only contact; keep it a boolean the policy kernel produces. |
| Evidence binding | The done record's evidence and digests, `delivery::deciding` and the bound candidates, G1, G5 | G1 proves what a receipt must equal to count; G3 proves which candidate decides. A policy theorem that a done record binds its evidence should take these as lemmas. If the policy changes what is bound, it changes `Filed` or `Subject` in place. |

Two rules keep the plans from colliding:

1. **This plan never changes behaviour** (one exception, named: the empty delivery mode in G3). The policy spec changes behaviour. When both touch a function, the backfill lands first with the as-built specification and the policy change replaces the theorem and its mutants in the same commit.
2. **`advance`'s signature is shared.** G1 changes it to take `GateFact`s. If the policy needs a per-step input (required, unskippable), add it to `Node` or as one further argument to the same function, and agree the shape before step 1 starts. `Plan.lean` and `Gate.lean` have one owner at a time; new policy theorems go in their own file (`proofs/Writfold/Policy.lean`) and import the others.

The semantic-search work (`docs/writfold/semantic-search-spike.md`) asks for a tier-ordering kernel and "a semantic hit never grounds". The first is the tier kernel of step 10; the second belongs in `ground` (G5) as one more input that is never `Grounded`. Whichever lands first defines the function.

## Decisions (owner, 16 November 2026)

1. Adopted. Backfill comes before the policy work; where the policy spec changes a kernel proved here, it replaces the theorem in place rather than adding a parallel one.
2. First round: steps 0–5 (documentation truth and the missing mutants, then gaps 1–5), each through the proofs gate before any wiring change. The rest follows in later rounds in risk order.
3. The empty delivery mode: a mode that declares nothing is never `ready` (as "empty is not met" elsewhere); fix it with the failing test first.
