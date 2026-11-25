# Grooming review, 25 November 2026

A review of two draft sets, made at the owner's request before
acceptance:

- the agent vocabulary RFC and its calculus (PR #152, issues #143–#151),
  read at `a3a686e`;
- evidence-governed execution, KB research contracts and the ten dry runs
  (PR #158, issues #153–#157), read at `1643d08`.

They were checked against the shipped kernels and proofs at `2b9c9db`,
the [mission](../../writfold/mission.md), [AGENTS.md](../../../AGENTS.md)
and the [RFC process](../README.md). Each row says **agree**,
**disagree** (with the fix), **generalize** (two constructs are one) or
**tweak**. The groomed text is in the three RFCs and the
[calculus](calculus.md).

## The main finding

The two sets are one calculus, and it is mostly built. Both drafts define
evidence bound to a subject, obligations with accepted standings,
three-valued judgments, an admission rule and "a record does not
advance". Writfold ships exactly these as proved kernels: `gate`,
`policy`, `pin`, `checklist`, `bind`, `ground`, `guard`, `delivery`,
`handoff`, `inventory`, `packet` and `plan`. Of the 31 obligations the
drafts list, 11 are shipped theorems as they stand and 9 more need one new
theorem for one new kind of record
([calculus §5.1](calculus.md#51-the-drafts-obligations-resolved)).

Three reuses remove most of the proposed surface:

1. **A worker's report is a delivery candidate.** Delivery already keeps
   exact bytes in the run, orders them by `seq`, lets the newest decide,
   withdraws by statement, judges them with checks, script gates and
   attestations, holds a plan step as `deliver:<mode>`, binds the done
   record to the candidate and shows `!stale`. The new part is one flag
   and one check.
2. **A research scope is a mechanical gate with retained evidence.** A
   graph read that exits non-zero when it is partial, a `scope` over the
   whole KB and `retain` give completeness, membership invalidation and a
   kept manifest with no new kernel.
3. **Review units are generated checklist items.** Coverage is the shipped
   roll-up over items that a manifest generates instead of a file listing.

What is new after that: two definition kinds and a resolver, the
invocation record, `invocation.bound`, and later a pair generator and a
guard for an absent universe.

## Vocabulary RFC and calculus

| # | Claim or construct | Verdict | Reason and change |
|---|---|---|---|
| V1 | Skill, agent profile and invocation are different things; plans own steps; gates own evidence | agree | Kept as the vocabulary |
| V2 | Writfold records and judges; an external harness executes | agree | The mission's record keeper. Kept |
| V3 | A Writfold skill format | tweak | The mission lists "a Writfold skill format" and "profile composition" under "Not now" (12 November). The owner's request is later, so acceptance adds a dated mission line. A manifest may bind the `SKILL.md` a harness loads (`instructions_file`, repository-relative) instead of copying it |
| V4 | Ten directories, with a git-ignored `.writfold/local/` | disagree | Most hold declarations nothing judges. Untracked state that changes a decision is state a fresh clone lacks, against "Git text is the source of truth". Reduced to `agents/`, `skills/`, `tools/` |
| V5 | Tool contracts with JSON schemas, effect atoms and path scopes | disagree for now | Nothing checks them without a validator dependency; inclusion of one glob in another is not a fold over facts; a declared effect confines nothing. A tool is a name and a description. The rest moves to #146 |
| V6 | Typed tool composition `t₂ ∘ t₁` | disagree | No verb composes tools. Removed |
| V7 | Instruction concatenation is associative and not commutative | agree | Its one consequence: the closure digest keeps listed order. Tested, not proved |
| V8 | Plan sequencing laws | agree, with a limit | They restate shipped behaviour. The fold is tested only and #17 was declined, so no new theorem rests on it. The "coordinate with #17" line is removed |
| V9 | `use` is idempotent on its key | generalize | It is `Plan.splice_idempotent`, shipped. The draft's ledger would have added a second theorem of the same name |
| V10 | Partial obligation combination `Ω₁ ⊎? Ω₂` | generalize, and no kernel | Its laws hold "up to" a normal form the draft did not choose. With `policy::Accept` (three booleans) they hold exactly, and `met(s, a∧b) ⟺ met(s,a) ∧ met(s,b)`. But no decision in the design merges two sources: plans and modes are the only source of obligations. `obligations_preserved` holds by construction |
| V11 | The empty map is an identity but an empty requirement is not readiness | agree | Shipped: `empty_never_ready`, `gates_fact_met_iff`, `unplanned_clean_empty` |
| V12 | Effective authority `Q ∩ G_h ∩ G_r ∩ G_p` | tweak | The law is true and has one operand today. Writfold cannot observe a host grant. The slice proves coverage (`covers_none_iff`). For more sources, `R ⊆ A∩B ⟺ R ⊆ A ∧ R ⊆ B`, so coverage is checked per source and no intersection is built (#159) |
| V13 | Specialization order | tweak | A partial order on (tools, obligations) pairs, a preorder on profiles. It is the syntactic form of the enforcement draft's `⪯`. No decision uses it before delegation, so it is a definition, not a kernel |
| V14 | Budget algebra | disagree for now | The invariant is kept by reserve and settle as stated, given a live reservation and `c ≤ r`. Gaps: no release rule, so a crash holds a reservation for ever and `k` retries cost `k·r`; settlement is not keyed, idempotent or conflict-aware; parent and child accounting is undefined; an omitted limit needs `∞`; a merge of two clones breaks the invariant; and Writfold cannot enforce what it records. Limits become exported hints. The algebra and its corrections move to #159 |
| V15 | One `configId` over definitions, context and task input | tweak | Three identities: closure, input, invocation id. The closure plays a gate's definition digest and the report's bytes its content digest, so `gate::about` judges them |
| V16 | `Applicable(e, o, σ)` as a new kernel | generalize | It is shipped `gate::bound`, `gate::about`, `pin::holds` and `unretained_not_bound` |
| V17 | No "some passing receipt exists"; the deciding record is selected | agree | Shipped `state_spec`. The enforcement draft contradicted it (E2) |
| V18 | Eight stored phases | disagree | Writfold derives state and stores none. "Running" and "returned" are not observable. Four derived words replace them; an invocation with no report is unknown |
| V19 | `agent record`, a result slot, duplicate and conflict rules | generalize | A delivery candidate. `agent record` is removed. One new rule survives: two different reports for one invocation never pass (`conflict_never_passes`) |
| V20 | `record_does_not_advance` as a Lean theorem | tweak | No mutant of a pure function can break it. It is a test and the TLC invariant `DeliveryDoesNotAdvance` |
| V21 | A model's judgment is never a mechanical pass | agree | Shipped (`standing_satisfied_iff`, `statement_never_passes`, `posted_never_satisfied`). `invocation.bound` judges binding only; a harness's "submitted" is `deliver attest` |
| V22 | Kernel signatures `compose(ResolvedDefs, UseSite)`, `admit(Composition, Grants, Support, SnapshotFacts)`, `transition(State, Event, Facts)` | disagree | Not the shipped shape and not extractable as written: maps, sets and a generic state machine. Each decision is a fold over facts the caller classified |
| V23 | `spec/AgentInvocation.tla` with dispatch, acknowledgement, budgets and delegation | tweak | Most of that is the harness's protocol. `spec/Invocation.tla` models what Writfold publishes: prepare, deliver, check, advance, merge |
| V24 | Exit codes and refusal wording | agree | Shipped conventions |
| V25 | Adapters and provisioning are optional; no Gripsack key | agree | The portable boundary is the package directory and `deliver prepare`. One adapter is #150 |
| V26 | No benefit is claimed; evaluation reuses #112 | agree | The evaluation plan is removed from #151 and stays with #112 |
| V27 | Ids may contain `.` | tweak | `name.name` already means a built-in check or a checklist item. Agent and skill ids take none; a tool id may take one |
| V28 | `default_plan` on a profile | disagree | A second place to name a plan. The skill names it |
| V29 | Captured snapshots of every definition | tweak | The record holds paths and digests; Git holds the bytes. Definitions a mode names join the run's policy pins, so drift reads `policy moved` |

## Evidence-governed execution

| # | Claim or construct | Verdict | Reason and change |
|---|---|---|---|
| E1 | Prose cannot show a skill reached a model; a hash alone identifies content, not delivery | agree | Kept, with the table of what each kind of evidence shows |
| E2 | Receipt rule `e ∈ E … ⟹ E ⊢ o` | disagree | It is existential: an old pass outvotes a newer failure. It contradicts shipped `state_spec` and V17, and under it the composition is a lower bound and not a meet. Replaced by "the deciding record passes" |
| E3 | Obligation tuple `(k, p, b, K, I, φ)` | generalize | Key = step and gate; `p` = a built-in name; `b` = digests and pins; `K` and `I` = class and `accept`; `φ` = which verb asks. No new type |
| E4 | Issuer roles and authenticated issuer facts as kernel input | disagree | Nothing in a serverless, unsigned product produces such a fact. The mission: receipts resist mistakes, not adversaries; no signing. An adapter's observation is a statement and stands as attested. The strict tier is the receiver, which already refuses statements. Signing stays with #64 |
| E5 | One evaluator at three use sites | agree, and shipped | `plan::advance`, `guard::decide` and `delivery::guard`, `handoff::checked`. The new site is `agent prepare` |
| E6 | An exit gate cannot undo earlier work; judge again at use | agree | Kept as the rule of [calculus §3.8](calculus.md#38-use-sites-and-why-a-postcondition-must-be-judged-again) |
| E7 | Hoare sequencing for skills, and a frame rule | disagree as stated | `Q` judged at the end of `S₁` can be stale when `S₂` starts. The rule holds only with `Q` judged again at `S₂`'s admission. No kernel consumes triples; there is no effect system for a frame rule. One paragraph remains |
| E8 | `⊓` as a partial meet, laws up to semantic equality | generalize | The same operator as V10, with exact laws on `Accept` |
| E9 | `class = "derived"`, `[[requires]] predicate = …`, a finite rule language, an evidence graph, Datalog later | disagree | A general evaluator that derives "met" from grounded premises is the assessed gate removed in #54. Fixed built-in names only. References stay leaves |
| E10 | Dependencies preserved over an acyclic graph | generalize | Shipped pins and grounding. With leaf references there is no graph |
| E11 | Context epochs | tweak | One invocation is one package and one context. Epochs matter for inline use in a long session, where Writfold sees "emitted" at most. Deferred to #123 and #141 |
| E12 | Prepared is not submitted; no circular requirement | agree | Two levels: prepared (Writfold wrote the package, known first hand) and submitted (a statement) |
| E13 | Bootstrap, a task-entry owner, strict run binding | generalize, and cut | Shipped: the `pointer` kernel, the `run.current` fact (`run_current_met_iff`) and a guard rule on the governed command. A fail-closed host is not something Writfold can be; hooks are "a guard, not enforcement". `prepare` takes its run and step and re-checks them under the lock |
| E14 | Three guarantee tiers | agree | Stated in shipped words: recorded, guarded, received |
| E15 | Portability across models and harnesses | agree, one cut | The boundary is the package and the CLI. "An optional MCP wrapper" is removed: MCP was dropped on 15 November |
| E16 | Delivery modes are one instance of evidence-governed execution | generalize, reversed | Delivery is the general shipped mechanism. A worker's report is the instance |
| E17 | Ten properties to prove | tweak | Four are shipped as they stand. Four are shipped for today's evidence and need one new theorem for reports or generated units. One is new (`bound_pass_iff`). One is not a kernel property |
| E18 | Seventeen conformance cases | agree | Kept as a fixture list, each mapped to a slice. Cases that need a protected host are marked out of scope |
| E19 | First milestone: a trusted adapter, a research scope and coverage together | disagree | Too wide. The first slice is the vocabulary RFC's: definitions, prepare, one bound report, one fake worker |
| E20 | A Python model and its traces under `docs/rfcs/` | tweak | Kept as the record of the exercise and not maintained. `spec/Invocation.tla` is the checked model. Each scenario is mapped to the groomed design in the dry-run page |

## KB research contracts

| # | Claim or construct | Verdict | Reason and change |
|---|---|---|---|
| R1 | Scope, coverage, assessment and disposition are four judgments | agree | Kept: a mechanical gate, a checklist, the report text, an attested gate |
| R2 | A scope-eligibility kernel | generalize | Shipped `walk::complete_within` (`truncated_never_complete`) and a mechanical gate whose command exits non-zero on a partial answer. The manifest is retained evidence (#121) |
| R3 | A scope-freshness kernel for new members and edges | generalize | The gate's `scope` covers the KB tree, so any added or changed record moves the scope digest (`judge_other_bytes`). No retrieval receipt returns (#105) |
| R4 | Deterministic unit generation | agree | New kernel `units::pairs`. A unit is named by record ids, not versions, so a changed member makes rows stale and does not mint new units |
| R5 | `Covered ⟺ Complete ∧ ∀u ∃r Eligible(r, u)` | tweak, generalize | The newest row of a unit decides (shipped `item_spec`). Units are generated checklist items and coverage is `rollup_covered_iff`. One new guard: the roll-up over zero items is true, so an absent manifest must never read as covered |
| R6 | Report eligibility | generalize | Shipped `bind`. New: a ready candidate bound to an invocation prepared for that unit grounds a row |
| R7 | Old pair reviews stay eligible when a document is added | tweak | The first rule is the safe one the draft itself starts from: the corpus moved, every row is stale. Unit-local reuse needs a per-unit digest and its own argument. Deferred |
| R8 | A finding-disposition kernel | generalize | An attested gate with `accept` (`waiver_rejected_never_met`) |
| R9 | `writfold-gate/proposal` with `class = "derived"`, an `[execution]` block, a scope recipe schema | disagree | See E9. The plan's three steps are the contract; the recipe is the scope gate's command |
| R10 | An empty scope needs a positive witness and an explicit policy | agree | `empty_universe_needs_allow` |
| R11 | All pairs is quadratic | agree | A cap, refused when exceeded (`over_cap_refused`). Other unit rules are later work |

## Checks the owner asked for

| Check | Result |
|---|---|
| Are the laws true as stated? | Sequencing and splice laws: yes. Obligation combination: only after choosing a normal form, and exact only when one record decides. Budget invariant: yes under its premises, with the gaps of V14. Specialization: a preorder, not a partial order, on profiles. Hoare sequencing: false without re-judging. Receipt rule: wrong (E2) |
| Are they the right laws for the product's decisions? | Tool composition, the specialization order, triples, the frame rule and obligation merging decide nothing today and are removed or kept as definitions |
| Is each decision a small extractable kernel? | Not as drafted (V22, E9). The groomed kernels are folds over classified facts, four functions in the first milestone |
| Is each TLA+ protocol real and minimal? | The drafted model mostly described the harness. One small module remains |
| Is the scope pre-alpha small? | Now: three file kinds, two verbs, one flag, one check |
| Does anything reintroduce a removed component? | E9 and R9 would have rebuilt the assessed gate; R3 risked retrieval receipts; E15 named MCP; a protected receipt ledger would be a companion process. All removed |
| Is Git text the truth and SQLite derived? | Yes after V4. Invocation records are create-new files in the run |
| Are the trust boundaries sound? | The drafts gave the kernel facts no component can produce (E4). Now: everything in the run is recorded and resists mistakes; a guard is a guard; the receiver is the protected boundary |
| Is the first vertical slice identifiable? | Yes: [the vocabulary RFC's rollout](../2026-11-25-agent-vocabulary.md#rollout) |
