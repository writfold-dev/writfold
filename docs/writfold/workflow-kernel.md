# Pure workflow kernel

The Rust kernel implements the reference judgments in the immutable Python
`src/kctx/semantics/workflow.py`, `portfolio.py`, and `design_workflow.py`.
It uses the existing version-2 checker rather than adding inference rules or
combining contexts. These are pure decisions over supplied records, not a store,
a signing adapter, or an authenticated publication service.

## Source-to-Rust mapping

All public judgment names preserve their reference spellings.

| Python source | Rust source | Judgments |
| --- | --- | --- |
| `workflow.py` admission | `crates/writfold-kernel/src/workflow.rs` | `preserves_prefix`, `plan_valid`, `apply_plan`, `transition`, `conclude` |
| `workflow.py` authored | `crates/writfold-kernel/src/workflow/authored.rs`, reexported from `workflow` | `authored_gate_valid`, `authored_ready`, `authored_gate_passes`, `authored_edge`, `authored_supersession_valid`, `authored_transition`, `authored_candidate_problems`, `authored_candidate_valid` |
| `workflow.py` selection | `crates/writfold-kernel/src/workflow/selection.rs`, reexported from `workflow` | `selection_plan_valid`, `apply_selection` |
| `portfolio.py` admission/query | `crates/writfold-kernel/src/portfolio.rs` | `find_context`, `context_family_valid`, `portfolio_state_valid`, `retired_context_ids`, `context_evolution_valid`, `assignment_valid`, `portfolio_added_refs`, `admitted_portfolio`, `portfolio_plan_valid`, `apply_portfolio`, `query_context_allowed`, `assess_portfolio` |
| `portfolio.py` selection | `crates/writfold-kernel/src/portfolio/selection.rs`, reexported from `portfolio` | `selected_portfolio`, `selection_contexts_valid`, `portfolio_selection_valid`, `apply_portfolio_selection` |
| `design_workflow.py` | `crates/writfold-kernel/src/design_workflow.rs` | `implementation_binding_valid`, `implementation_bound`, `design_candidate_valid`, `design_candidate_problems`, `design_transition`, `design_with_binding`, `bind_implementation`, `design_reopened`, `reopen_design` |

The private Python `_identity_present` predicate is represented by typed optional
identity checks. Private Rust helpers compare unchanged context fields and
validate candidate projections without constructing owned intermediate states.

## Return and authority contracts

Inputs are borrowed domain records; tuple inputs are slices. Status arguments
are closed enums, and evaluation times borrow arbitrary-precision `Integer`.
Problem APIs return ordered `Vec<&'static str>` values, including every applicable
problem, with the original landed-reference/gate branch precedence.

`apply_plan`, `apply_selection`, `apply_portfolio`, and
`apply_portfolio_selection` return kernel `WorkflowResult` aliases. `state()`
(`kb()` additionally for `Application`), `journal()`, and `outcome()` expose
borrowed views. `into_parts()` returns state/journal `Cow` values and the outcome.
Rejection, replay, and conflicts retain borrowed input state and journal; only
application constructs owned changed records. Journal insertion is at the front.
The first journal entry with a matching operation ID controls replay; complete
plan equality, not an ID-only comparison, distinguishes replay from conflict.
Replay precedes predecessor and approval checks.

`ModelApproval`, `ModelSelectionApproval`, `ModelPortfolioApproval`, and
`ModelPortfolioSelectionApproval` represent reference premises. Their
constructors do **not** authenticate anyone or authorize production IO. The
kernel has no production verified-approval constructor. Design binding and
landed resolution similarly consume supplied model observations; real source
resolution and authenticated publication remain adapter obligations.

## Preserved boundaries

- Admission preserves every ordered knowledge-history prefix. Portfolio
  assignments correspond exactly, in order, to the appended claim suffix;
  each witness is checked and registered under its one named context.
- Context removal requires the exact prior-order retired-ID sequence. Existing
  IDs preserve target and intended use. Queries may narrow target and advance
  time, but may not substitute sources, policy, or intended use.
- Selection retains the current complete knowledge history and corrections.
  It selects the retained selection, uses a fresh snapshot, and preserves the
  current ordered context family and its policy/source inputs, except snapshot
  and nondecreasing evaluation time.
- Authored readiness, latest eligible gate result, current material/target
  binding, landed resolution, and all authored problem ordering are retained.
- Design acceptance can precede binding. Bind/rebind retains acceptance and
  gate history but clears landing references/resolutions, including a same-target
  rebind. Reopen clears acceptance, actor roles, target, binding, and landing
  authority while preserving gate history. Both require exact expected-state
  equality; applied/verified records cannot bind or reopen in place.
- `DesignRevision` remains nominally distinct from
  `ProposalMaterialRevision`. The design judgment explicitly compares their
  reference identity spellings at the authored material boundary; there is no
  blanket conversion or alias.

## Evidence and outstanding qualification

Behavioral tests in `crates/writfold-kernel/tests/workflows.rs` cover complete-plan
replay conflict and borrowed rejection, omitted assignments, explicit retirement,
context source isolation, forward selection retaining corrections/current
policy, rebind invalidation, reopen authority clearing/final-state refusal, and
complete authored problem ordering. These tests were authored without running
checks in the workflow implementation task; their presence is not a passing
receipt.

Source-bound Verus verification of these actual executable bodies and caller
conversions remains outstanding: V05/V06 and V12/V13. No `assume`,
`external_body`, proof twin, or new Lean claim is provided. Three-host complete
response conformance, integration execution, mutation qualification, and the
M09 authored lifecycle / publication-recovery protocol obligations remain
separate qualification work. Historical Lean proofs and these source mappings
do not prove the Rust implementation.
