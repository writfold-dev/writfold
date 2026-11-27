# One calculus for agent work under gates

Companion to three draft RFCs:
[agent vocabulary](../2026-11-25-agent-vocabulary.md) (#143),
[evidence-governed execution](../2026-11-25-evidence-governed-execution.md)
(#153) and [KB research contracts](../2026-11-25-kb-research-contracts.md)
(#155, #156). The last two are in
[PR #158](https://github.com/writfold-dev/writfold/pull/158), whose branch
carries this file too.

The first drafts held two algebras: one over tools, obligations,
capabilities and budgets, and one over receipts, issuers, phases and
coverage. The [grooming review](grooming.md) found they are one algebra,
and that most of it is already shipped and proved. This file states that
one calculus, names the theorem behind each law, and says what is new.

Extended on 27 November 2026 by
[referents, remainders and sites](../2026-11-27-referents-remainders-sites.md)
(§3.11 to §3.13): which thing a fact is about, what a universal word
ranges over, and where a decision is asked.

Nothing new below is proved yet. Every theorem marked **shipped** is in
`proofs/Writfold/` today. Every theorem marked **new** is an obligation of
the issue named beside it.

## 1. The five layers

Every decision Writfold makes has the same shape. The caller reads files
and Git and turns what it found into small facts. A kernel folds the
facts. Nothing is stored that can be derived.

| Layer | Question | Shipped kernels | New |
|---|---|---|---|
| L0 fact | Are these the same bytes? Does this pin hold? Could this be read? | `bytes::same`, `gate::about`, `pin`, `inventory`, `observe` | none |
| L1 standing | What does the evidence for one obligation say now? | `gate::judge`, `gate::standing`, `checklist`, `bind`, `ground`, `delivery::standing` | `invoke::bound` |
| L2 policy | Does that standing meet the obligation? | `policy::met` | none |
| L3 requirement | Is a named requirement met, unmet or unknown? | `guard::require`, `guard::all_met` | none |
| L4 admission | May this operation proceed? | `plan::advance`, `guard::decide`, `delivery::guard`, `handoff::checked` | `compose::closure`, `invoke::prepare_check` |

The vocabulary draft's "composition, admission, applicability, record
transition" and the enforcement draft's "evidence applicability, obligation
evaluation, admission, obligation composition" are these layers under
other names.

## 2. Objects

- A **definition** is a file under `.writfold/`: a gate, a mode, a plan
  template, and now a tool, a skill or an agent profile.
- A **closure** is a profile with its skills, in listed order, and the
  tools they name. Its digest `cl` covers the keys that change behaviour
  and every instruction byte.
- An **invocation** `ι = (run, step, agent, cl, input, seq)` is a record
  written by `agent prepare`. The step is a path with its round, as the
  run graph names it.
- A **candidate** `c = (mode, bytes, seq, ι?)` is a delivery candidate.
  With `ι` it is a worker's report.
- An **obligation** is a gate or a mode named by a plan step, a guard
  rule, a receiver's required set, or (later) a profile's `needs`. It has
  a class and an `accept` policy. There is no other obligation type.

The enforcement draft's obligation tuple `(k, p, b, K, I, φ)` maps onto
these without remainder:

| Draft coordinate | Here |
|---|---|
| key `k` | the step path and the gate or mode id |
| predicate `p` | a built-in name: a gate class, a `require` entry, a `checks` entry |
| bindings `b` | the definition digest, the content digest and the pins |
| evidence kinds `K`, issuer roles `I` | the class and `accept` |
| phase `φ` | which verb asks. It is not data |

## 3. Laws

### 3.1 Binding

A record is about the present exactly when its recorded definition digest
and content digest equal the present ones, byte for byte, and it is filed
in this run, names this subject and is intact.

- Shipped: `about_spec`, `bound_spec`, `state_other_subject`,
  `judge_other_bytes`, `judge_empty_content`, `unretained_not_bound`.
- For an invocation report: definition ↦ the closure digest; content ↦ the
  candidate's bytes; run and step ↦ the invocation's. New:
  `bound_pass_iff` (#149).

Three identities are kept apart, where the draft had one `configId` over
everything: the closure (stable across tasks), the input (one task) and
the invocation id (one attempt). Equal closures mean comparable workers,
not equal output.

Further bindings are pins. Evidence holds while every pin holds; one pin
that moved, aged or cannot be read is enough to stop it
(`unheld_never_holds`, `current_under_refines`). A scope membership basis,
a context epoch or another repository are pins, not new coordinates.

### 3.2 One record decides

For each obligation one record decides: the newest, by `seq`, that is
bound and about the present. There is no "some passing record exists".

- Shipped: `state_spec`, `state_satisfied_iff`, `newest_is_max`,
  `deciding_spec`, `item_spec`.

The enforcement draft's receipt rule was existential:

$$
\frac{e\in E\quad \ldots\quad Check_p(e,B)}{E\vdash_B o}
$$

With receipts `[pass, fail]`, oldest first, both about the present, that
rule derives `o`. The shipped kernel says `failed`. The existential rule is
withdrawn. Read `E ⊢ o` as: `deciding(E, o)` exists and passes.

### 3.3 A class is never promoted

A statement is never a passed check. A waiver is never evidence. A report
is never a mechanical result.

- Shipped: `standing_satisfied_iff`, `satisfied_only_by_current_pass`,
  `statement_never_passes`, `standing_never_satisfied` (checklists),
  `posted_never_satisfied`, `waiver_never_grounds`, `kept_never_promotes`.
- For an invocation: `invocation.bound` judges the binding of the bytes,
  which Writfold can check from its own records. What the report says
  reaches no standing. What a harness says it sent is a statement
  (`deliver attest`), shown as attested.

So the drafts' `judgment_not_mechanical_pass` and `judgment_not_promoted`
need no new theorem.

### 3.4 Policy narrows, and narrowing is a meet

`Accept` is three booleans: satisfied, attested, waived. Order policies by
`a ≤ b` when every word `a` takes, `b` takes. Then
`a ∧ b` (field by field) is the meet, `(true, true, true)` is the top, and

$$
met(s, a \wedge b) \iff met(s,a) \wedge met(s,b).
$$

The law holds exactly, with no "up to semantic equality", for two reasons.
`Accept` is a canonical representation, so `∧` is associative, commutative
and idempotent on the nose. And by 3.2 one standing `s` decides, so both
sides ask about the same word. Under the withdrawn existential rule the
law fails: `e₁` could meet `a` and `e₂` meet `b` while nothing meets
`a ∧ b`.

- Shipped: `met_spec`, `met_default`, `met_narrows`,
  `waiver_rejected_never_met`, `accept_only_narrows`. A list that leaves a
  class no standing is refused when the definition loads
  (`value.unmeetable`), which is the drafts' "empty intersection is a
  conflict".

This one operator replaces the vocabulary draft's `Ω₁ ⊎? Ω₂` and the
enforcement draft's `⊓`. No kernel computes `a ∧ b` today, and none is
added: plans and modes are the only source of obligations, so two
policies never meet on one key. The two places that already combine
policies are instances of the law. A mode's `waivable` narrows a check's
policy. The strict receiver takes `(true, false, false)` whatever a gate's
`accept` says. A later source of obligations must combine by `∧`, and then
proves `narrow_met_iff` with the display above as its statement.

It follows that adding an obligation or narrowing a policy never makes an
unmet set met. Nothing follows about time: evidence can stop holding.

### 3.5 Unknown and empty

A fact is met, unmet or unknown. Unknown never admits. An empty set of
obligations never reads as met where something asks "is everything met?".

- Shipped: `unknown_never_passes`, `decide_pass_iff`, `outcome_pass_iff`,
  `gates_fact_met_iff`, `gates_unmet_when_incomplete`,
  `unplanned_clean_empty`, `empty_never_ready`, `empty_never_complete`,
  `incomplete_evidence_never_accepted`, `truncated_never_complete`,
  `complete_iff` (packet), `omitted_counted`.
- New: `bound_unread_unknown` (#149), `absent_universe_never_covered`
  (#156).

Several rules combine by `guard::combine`, where a denial beats a question
and a question beats a pass. It is associative and commutative with `Pass`
as identity (`combineOf_assoc`, `combineOf_comm`, `combineOf_pass_left`).

### 3.6 Tools: coverage, and the rule for more grant sources

Let `R` be the tools the listed skills need and `Q` the tools the profile
lists. The closure is admitted only when `R ⊆ Q`.

- New: `covers_none_iff`, `closure_ok_iff`, `more_skills_never_admit`
  (#144).

The vocabulary draft defined an effective authority
`C = Q ∩ G_h ∩ G_r ∩ G_p` over host, run and parent grants. The slice has
one source, `Q`. When more arrive (#159, #150), the rule is the
intersection, and it needs no set to be built:

$$
R \subseteq A \cap B \iff R \subseteq A \wedge R \subseteq B.
$$

So admission runs `covers` once per source, and "a child never exceeds its
parent" is one more call. Writfold cannot observe what a host grants. An
adapter's word about it is a statement (3.3).

Capability predicates over paths, effects and scopes are out of the
calculus until a use needs them: inclusion of one glob in another is not a
fold over facts.

### 3.7 Order: plans, splices, instructions

A plan is an ordered spine. Sequencing is associative with the empty plan
as identity, and not commutative. A splice is idempotent on its key
(parent, template, digest, ordinal).

- Shipped: `current_spec`, `splice_spec`, `splice_keeps_existing`,
  `splice_idempotent`, `splice_check_depth`. The draft's ledger row named
  `splice_idempotent` is this theorem, not a new one.
- The fold that places steps is tested, not proved
  ([#17](https://github.com/writfold-dev/writfold/issues/17) was declined).
  No new theorem may rest on it.

Instruction text concatenates in listed order. That is a free monoid:
associative, with the empty text as identity, never commutative. The
consequence is one rule for the closure digest: keep the listed order.
It is tested with the digest, not proved.

The draft's typed tool composition `t₂ ∘ t₁ : I → O ! E₁ ∪ E₂` is
withdrawn. No verb composes tools.

### 3.8 Use sites, and why a postcondition must be judged again

The same obligations are consulted at four sites. Each is a verb, with its
own admission kernel over the same L1 to L3 answers.

| Site | Verb | Kernel | Refusal on unknown |
|---|---|---|---|
| before a worker is set up | `agent prepare` | `invoke::prepare_check` (new) | yes |
| before a tool call | `hook pre-tool-use` | `guard::decide`, `delivery::guard` | yes (`unknown_never_passes`) |
| before a step is done | `plan advance` | `plan::advance` | yes (`advance_undefined_refused`) |
| before a candidate is accepted elsewhere | `handoff check` | `handoff::checked` | yes |

The enforcement draft gave a sequencing rule for skills:

$$
\{P\}\ S_1\ \{Q\},\quad \{Q\}\ S_2\ \{R\}\ \Longrightarrow\ \{P\}\ S_1;S_2\ \{R\}.
$$

As written it is false here. `Q` is judged when `S₁`'s step is done, at
time `t₁`. `S₂` starts at `t₂ ≥ t₁`, and evidence has no monotonic
freshness: the subject can move in between. The shipped behaviour says so:
a done step can read `!stale` and the current step does not move back
(`stale_never_current`). The rule that holds is:

> `S₁; S₂` gives `R` when `Q` is judged again at `S₂`'s admission.

That is what a `needs` list at prepare does (#154), and what a guard rule
does before a command. No kernel consumes triples, so there is no theorem
to prove. The frame rule is withdrawn with the effect system it needs.

### 3.9 Coverage of a generated universe

Let a manifest `M` list `n` members, and let `U(M)` be the unordered pairs
of distinct members, named by record id and not by version:
`|U(M)| = n(n−1)/2`. Each unit is a checklist item. Then

$$
Covered \iff present(M) \wedge (n > 0 \vee allow\_empty) \wedge
\forall u \in U(M):\ covered(item(newest\ row\ of\ u)).
$$

The drafts wrote `∃ r ∈ E: Eligible(r, u)`. By 3.2 the newest row of a
unit decides, so a later "does not hold" row is not outvoted by an earlier
one.

- Shipped: `item_spec`, `item_covered_iff`, `rollup_covered_iff`,
  `other_never_answers`, `item_under_narrows`, `complete_within_iff`,
  `truncated_never_complete`.
- New (#156): `pairs_spec`, `pairs_len`, `over_cap_refused`,
  `generated_covered_iff`, `absent_universe_never_covered`,
  `empty_universe_needs_allow`, `of_delivery_grounded_iff`.

`present(M)` is the guard the shipped roll-up lacks: over zero items "every
required item is covered" is true, so an absent manifest would read as a
finished review.

Four judgments stay separate and are never folded into one word: the scope
(a mechanical gate), the coverage (a checklist, which stands as attested),
the assessment (what each report says, which reaches no standing) and the
disposition of findings (an attested gate).

### 3.10 Specialization

For two profiles, write `a′ ⪯ a` when `a′` lists no tool `a` lacks and
owes every obligation `a` owes. On the pairs
(tools, obligations) this is a partial order. On profiles it is a
preorder: two profiles with different instructions and the same pair are
equivalent under it and not equal. It says nothing about which worker is
better.

The enforcement draft's `O₂ ⪯ O₁` ("every evidence set that satisfies
`O₂` satisfies `O₁`") is the semantic form of the obligation half. By 3.4
the syntactic form implies it.

No decision consumes this order until delegation exists (#159). It is a
definition, not a kernel.

## 4. Invocation rules

State is derived from records. An invocation has no stored phase.

| Word | Derived as |
|---|---|
| `prepared` | the record exists; no live candidate names it |
| `delivered` | the live candidates naming it hold one payload |
| `conflicting` | they hold more than one |
| `retried` | a later invocation names it in `retry_of` |

The draft's eight phases (`new`, `prepared`, `running-reported`,
`returned`, `validated`, `recorded`, `refused`, `unknown`) are withdrawn.
"Running", "returned" and a worker's own "failed" are things Writfold
cannot see. An invocation with no report is unknown and stays so.

Write `H` for the run's records and `→` for one publication under the
write lock.

**PREPARE.** When `prepare_check` admits, publish the invocation, then
write the package outside the KB.

$$
\frac{prepare\_check(facts(H, a, step)) = Ok \quad \iota \notin H}
     {H \to H + \iota}
$$

A refusal publishes nothing. Two prepares with equal inputs give two
invocations.

**DELIVER.** `deliver prepare --invocation ι` publishes a candidate naming
`ι`. It writes no plan record and judges nothing.

**CHECK.** `deliver check` records `bound(named(H, c))` for
`invocation.bound`, beside the mode's other checks. On every later read
the receipt is judged again: when the closure moved or a second payload
appeared, a pass reads stale (`rechecked_spec`).

**ATTEST.** A harness or a person may state what it observed
(`deliver attest`). The statement is bound to the candidate's bytes and
goes stale when they change.

**ADVANCE.** Unchanged: the step is done only when its `deliver:<mode>`
candidate is ready (`fact_met_iff_ready`, `advance_facts_ok_iff`). The
done record binds the candidate.

**RETRY.** A new invocation with `retry_of`. Its report is a newer
candidate, so it decides (`deciding_spec`).

**WITHDRAW.** A conflict is resolved by withdrawing one candidate, with a
reason and a name. Both files stay.

**MERGE.** Two clones merge in Git. Records are create-new files, so a
merge adds and never rewrites. A merge can turn `delivered` into
`conflicting`; a step already done on one of the reports then reads
`!stale` and stays done.

What the rules do not say: that a worker ran, that it read
`instructions.md`, that its report is true, or that an effect happened
once. A re-run after an unknown outcome can repeat an effect outside
Writfold.

## 5. Proof ledger

### 5.1 The drafts' obligations, resolved

| Draft obligation | Resolution |
|---|---|
| `compose_typed` | new, narrowed: `closure_spec`, `closure_ok_iff` (#144). Schema compatibility is out of scope |
| `obligations_preserved`, `obligations_not_weakened` | by construction (a profile and a skill cannot name a gate or `accept`), with shipped `advance_facts_ok_iff`, `done_only_when_met`, `met_narrows` |
| `splice_idempotent` | shipped under that name |
| `capability_non_amplification` | new, narrowed: `covers_none_iff`, `more_skills_never_admit` (#144); the intersection rule of 3.6 for #159 |
| `unsupported_refused` | deferred with the adapter (#150): the slice has no support facts |
| `admit_iff` | new: `prepare_ok_iff` (#148) |
| `admission_requires_evidence` | shipped at three sites (`advance_facts_ok_iff`, `decide_pass_iff`, `guard_allow_iff`, `strict_accepted_iff`); new at the fourth: `prepare_ok_iff` (#148) |
| `reservation_conservation`, `retry_charged`, `overflow_refused` | deferred, with corrections (#159) |
| `evidence_bound`, `evidence_bound_to_use` | shipped (3.1); new for reports: `bound_pass_iff` (#149) |
| `context_submission_bound` | split: the binding is `bound_pass_iff`; "submitted" is a statement (3.3) |
| `unknown_not_met`, `unknown_not_success` | shipped (3.5); new: `bound_unread_unknown` (#149) |
| `incomplete_never_complete` | shipped (3.5) |
| `coverage_requires_universe` | shipped `rollup_covered_iff`; new `absent_universe_never_covered` (#156) |
| `dependencies_preserved` | shipped pins and grounding (`unheld_never_holds`, `moved_iff`). References are leaves, so there is no graph and no acyclicity to prove |
| `judgment_not_mechanical_pass`, `judgment_not_promoted` | shipped (3.3) |
| `record_idempotent` | restated: delivering the same bytes again changes no judgment. Follows from `bound_pass_iff` (the `single` fact compares bytes) |
| `conflict_not_success` | new: `conflict_never_passes` (#149) |
| `record_does_not_advance` | not a kernel property. A test and the TLC invariant `DeliveryDoesNotAdvance` |
| `governed_action_requires_bound_run` | shipped: `run_current_met_iff`, `decide_pass_iff`, `unknown_never_passes` |
| Scope eligibility, scope freshness | shipped: `complete_within_iff`, `truncated_never_complete`, `judge_other_bytes` |
| Unit generation, coverage | new (#156), listed in 3.9 |
| Report eligibility | shipped `bind`; new `of_delivery_grounded_iff` (#156) |
| Finding disposition | shipped: an attested gate with `accept` (`waiver_rejected_never_met`) |

The three drafts named 31 distinct obligations. Eleven need no new
theorem. Nine are shipped for today's evidence and need one new theorem
for the new kind of record. Five are new. One is restated. Four are
deferred. One is not a property of a pure function.

### 5.2 New theorems, by slice

| Slice | Kernel | Theorems | Mutants |
|---|---|---|---|
| #144 | `compose::covers`, `compose::closure` | `covers_spec`, `covers_none_iff`, `closure_spec`, `closure_ok_iff`, `more_skills_never_admit` | `covers-ignores-last`, `closure-wrong-kind-found`, `closure-skips-tools` |
| #147, #148 | `invoke::prepare_check` | `prepare_check_spec`, `prepare_ok_iff` | `prepare-skips-packet`, `prepare-skips-plan` |
| #149 | `invoke::bound` | `bound_spec`, `bound_pass_iff`, `bound_unread_unknown`, `conflict_never_passes` | `bound-ignores-closure`, `bound-ignores-step`, `bound-ignores-single`, `bound-unread-passes` |
| #154 | `invoke::prepare_check` gains the `needs` facts | `prepare_check_spec` and `prepare_ok_iff` restated; `needs_unknown_refused`, `needs_first`, `no_needs_unchanged` | `prepare-skips-needs`, `needs-unknown-met` |
| #156 | `units::pairs`, `units::generated`, `ground::of_delivery` | `pairs_spec`, `pairs_len`, `over_cap_refused`, `generated_covered_iff`, `absent_universe_never_covered`, `empty_universe_needs_allow`, `of_delivery_grounded_iff` | `pairs-drops-last`, `pairs-keeps-diagonal`, `generated-absent-covered`, `generated-empty-covered`, `delivery-unready-grounds` |

Status (updated as slices land): the #144 row is proved (`proofs/Writfold/Compose.lean`, with `covers_loop_spec` and `closure_loop_spec`); its mutants are calibrated against the loop specs. The #147, #148 row is proved (`proofs/Writfold/Invoke.lean`, with `firstRefusal_none_iff`, `plan_missing_refused` and `packet_incomplete_refused`). The #149 row is proved in the same file. The #154 row is proved as `invoke::prepare` over `prepare_check` and `first_unmet` (`prepare_spec`, `prepare_needs_ok_iff`, `needs_unknown_refused`, `needs_first`, `no_needs_unchanged`), leaving #148's theorems unchanged. The #156 row is proved in `Units.lean` and `Ground.lean`. The protocol model of §6 is checked (`spec/Invocation.tla`). The other rows are proposed.

Each mutant names the spec theorem whose proof unfolds the mutated
function. The corollaries follow from it. Each slice passes the proofs
gate (drift, `lake build`, axiom audit, mutants) and refreshes
`proofs/claims.json` before its verb is wired.

Kernel shape, for every row: owned data, no nested borrows, no derives
reached, no casts, explicit `while` loops with one early return per
function, facts as small enums and booleans. A function that needs a
derive for its callers is named one by one in `start_from`.

## 6. Protocol model

One small module, `spec/Invocation.tla`, checked before `agent prepare` is
wired (#145). It does not change `KbWrite` or `RunGraph`.

Constants: two clones, one step that owes `deliver:m`, at most two
invocations, payloads `{p, q}`.

Actions: `Prepare(c)`; `Deliver(c, ι, payload)` (the worker is this
action, enabled at any time for any payload); `Check(c)`; `EditClosure(c)`;
`Withdraw(c, cand)`; `Advance(c)`; `Merge(c, d)`; `Crash(c)` between a
publication and the next step of the same verb.

Invariants:

- `BoundOnlyWithIntent`: `invocation.bound` passes in a clone only for a
  candidate whose invocation record is in that clone.
- `ConflictNeverReady`: when the live candidates naming one invocation
  hold two payloads, none of them is ready.
- `DoneOnlyWhenReady`: every done record was written in a clone where its
  candidate was ready at that moment.
- `DeliveryDoesNotAdvance`: `Deliver` and `Check` leave the done set
  unchanged.
- `OtherStepNeverBinds`: an invocation prepared at one step never makes a
  candidate ready for another.
- `StaleNeverMovesBack`: no action removes a done record or moves the
  current step back.

Mutants, each of which must break its invariant: `NoSingle`
(`ConflictNeverReady`), `NoClosureRecheck` (`DoneOnlyWhenReady`),
`BindAcrossSteps` (`OtherStepNeverBinds`), `AdvanceOnDeliver`
(`DeliveryDoesNotAdvance`).

A probe that must stay reachable: `probe-MergeAfterDone`. Clone A is done
on report `p`; clone B delivers `q` for the same invocation; after the
merge the done step is stale and still done. That is the honest outcome,
not a failure to hide.

Not modelled, because Writfold does not do them: launching, cancelling or
acknowledging a worker, budgets, delegation. A progress property would
need fairness of a harness Writfold does not run, so none is claimed.

## 7. Deferred, with the corrections found

| Item | Why deferred | Correction to carry | Owner |
|---|---|---|---|
| Budget reservations | Writfold cannot stop a model; no first-slice decision reserves | no release rule (crashes leak for ever, `k` retries cost `k·r`); settlement must be keyed by invocation, idempotent, with conflict; limits live in `(ℕ ∪ {∞})^k`; parent and child accounting is undefined; two clones break the invariant on merge | #159 |
| Authority intersection, delegation | one grant source today | check coverage per source; never build the intersection | #159 |
| Tool schemas, effects, path scopes, bindings | nothing checks them; scope inclusion is not a fold | choose a validator and a grammar first | #146 |
| Authenticated issuers, signed observations | no producer of such a fact in a serverless, unsigned product | an observation is a statement until a receiver verifies it | #64 |
| Requirements at prepare (`needs`) | not needed for the first slice | reuse `guard::require` facts | #154 |
| Unit-local reuse of reviews after the corpus moves | needs a per-unit content digest and an argument for it | the first rule is: the corpus moved, every row is stale | #156 |
| Context epochs for inline use | Writfold sees "emitted", not "read" | join #141's stages | #123, #141 |
| A general rule language over evidence | it is the removed assessed gate under another name | fixed built-in names only | not planned |

## 8. What is claimed

Lean covers pure decisions over the facts they are given. TLC covers
bounded interleavings of the publications Writfold makes. Tests cover the
reading, hashing and writing that turn files into facts.

None of them shows that a model read its instructions, that a review is
right, that a harness confined a process, or that a recorded statement is
true. Receipts resist mistakes, not adversaries
([mission](../../writfold/mission.md)). The boundary that resists a worker
that does not cooperate is a receiver running `handoff check` on its own
policy revision (#57, #64), and it accepts no statement.
