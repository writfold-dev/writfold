import Kctx.Engineering.OperationalProofsIdentity
import Kctx.Engineering.OperationalProofsRecords
import Kctx.Engineering.OperationalProofsRuns
import Kctx.Engineering.OperationalProofsClaims
import Kctx.Engineering.OperationalProofsEnvironment
import Kctx.Engineering.OperationalProofsBudget
import Kctx.Engineering.OperationalProofsControl
import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
namespace Kctx.Engineering

/-- E02's complete structural conjunction is preserved by the actual reducer.
The proof families establish distinct fields; none assumes preservation
of the assembled invariant, and the interpreter does not filter its output. -/
theorem domain_preserves_operationalWellFormed {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  have identities := operationalWellFormed_domain_identities wf h
  have records := operationalWellFormed_domain_records wf h
  have claims := domain_preserves_operationalClaims wf h
  exact {
    portfolio := records.portfolio
    grants_unique := identities.grants_unique
    governing_floor := records.governing_floor
    corpus_binding := records.corpus_binding
    corpus_realm := records.corpus_realm
    corpus_family := records.corpus_family
    activity_realm := records.activity_realm
    activity_family := records.activity_family
    envelopes_retained := records.envelopes_retained
    catalog_release := records.catalog_release
    catalog_unique := identities.catalog_unique
    catalog_authority := records.catalog_authority
    activations_unique := identities.activations_unique
    accounts_unique := identities.accounts_unique
    accounts_bounded := identities.accounts_bounded
    runs := domain_preserves_run_representation wf h
    effects_unique := identities.effects_unique
    effects := records.effects
    ownership_unique := identities.ownership_unique
    ownership := records.ownership
    candidates_unique := identities.candidates_unique
    candidates := records.candidates
    captures_unique := identities.captures_unique
    captures := records.captures
    receipts := records.receipts
    returned := records.returned
    claims_unique := claims.claims_unique
    claim_effects := claims.claim_effects
    dispatch_inventory := claims.dispatch_inventory }

/-- Replay preserves representation under the captured contracts and captured
before-state, not today's authority, time, projection or mutable package HEAD. -/
theorem replay_preserves_operationalWellFormed
    {entries : List (Ledger.Record Action HistoricalPremises)}
    {record : Ledger.Record Action HistoricalPremises} {after : OperationalState}
    (wf : OperationalWellFormed record.historical.contracts record.historical.before)
    (h : replay entries record = some after) :
    OperationalWellFormed record.historical.contracts after := by
  unfold replay at h
  cases executed : domain record.historical.contracts
      (historicalState entries record) record.payload with
  | none => simp [executed] at h
  | some next =>
      simp only [executed, Option.map_some, Option.some.injEq] at h
      subst after
      exact domain_preserves_operationalWellFormed wf executed

theorem step_preserves_operationalWellFormed {contracts : Contracts} {s t : State}
    {request : Request} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : step contracts s request = some t) :
    OperationalWellFormed contracts t.toOperationalState :=
  step_preserves_operational_property domain_preserves_operationalWellFormed
    localAction_preserves_operationalWellFormed wf h

theorem transition_preserves_operationalWellFormed {contracts : Contracts} {s t : State}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : Transition contracts s t) : OperationalWellFormed contracts t.toOperationalState :=
  transition_preserves_operational_property domain_preserves_operationalWellFormed
    localAction_preserves_operationalWellFormed environment_preserves_operationalWellFormed wf h

theorem reachable_operational_wellFormed {contracts : Contracts} {initial s : State}
    (base : OperationalWellFormed contracts initial.toOperationalState)
    (reach : Reachable contracts initial s) : OperationalWellFormed contracts s.toOperationalState :=
  reachable_operational_property domain_preserves_operationalWellFormed
    localAction_preserves_operationalWellFormed environment_preserves_operationalWellFormed base reach

theorem initialized_reachable_operational_wellFormed {contracts : Contracts}
    {input : InitialInputs} {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) : OperationalWellFormed contracts s.toOperationalState :=
  reachable_operational_wellFormed (init_operational_wellFormed initialized) reach

/-- All Requests, environment actions and refused steps participate in this
induction from the checked initializer. -/
theorem E02_representation_preserved {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) : OperationalWellFormed contracts s.toOperationalState :=
  initialized_reachable_operational_wellFormed initialized reach

theorem E04_E10_arbitrary_invocation_bounds {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s.toOperationalState root leaf depth) (leafLive : live s leaf = true) :
    (∀ cap ∈ leaf.capabilities, cap ∈ root.capabilities) ∧
    (∀ gate ∈ root.floor, gate ∈ leaf.floor) ∧
    leaf.procedure.rank + depth ≤ root.procedure.rank :=
  invocation_path_bounded (initialized_reachable_operational_wellFormed initialized reach) path leafLive

theorem E10_historical_invocation_rank_bound {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s.toOperationalState root leaf depth) :
    leaf.procedure.rank + depth ≤ root.procedure.rank :=
  invocation_path_rank_bound (initialized_reachable_operational_wellFormed initialized reach) path

theorem E03_exact_pending_invocation {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {parent : Run} (member : parent ∈ s.runs)
    {pending : PendingCall} (waiting : pending ∈ parent.control.pending) :
    ∃ child ∈ s.runs, child.invocation = pending.child ∧
      child.parent = some parent.invocation ∧ child.id = parent.id ∧
      child.procedure.id = pending.target.procedure ∧
      child.procedure.revision = pending.target.revision ∧ child.input = pending.input ∧
      ∀ other ∈ s.runs, other.invocation = pending.child → other = child :=
  pending_call_exact_child (initialized_reachable_operational_wellFormed initialized reach)
    member waiting

theorem E10_shared_account_traversal_bound {contracts : Contracts} {input : InitialInputs}
    {initial s t : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {root : RunId} {count : Nat}
    (trace : SharedAccountTrace contracts root s count t) {account : RootBudget}
    (found : s.budgets.find? (·.root == root) = some account) :
    account.spent + count ≤ account.limit :=
  shared_account_trace_bounded (initialized_reachable_operational_wellFormed initialized reach)
    trace found

theorem E04_governing_gate_floor {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) (run : Run) (node : Node)
    {gate : Gate} (required : gate ∈ contracts.kernelFloor) :
    gate ∈ effectiveNodeFloor s run node :=
  operational_effective_floor (initialized_reachable_operational_wellFormed initialized reach)
    run node required

theorem E03_exact_join_returns {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {run : Run} {calls : List NodeId}
    (ready : joinReady s run calls = true) {node : NodeId} (member : node ∈ calls) :
    ∃ result ∈ s.returned, result.parent = run.invocation ∧ result.node = node ∧
      result.receipt ∈ contracts.receipts ∧ result.receipt.role = .procedureResult ∧
      result.receipt.invocation = result.child ∧ result.receipt.rootInput = result.input ∧
      result.receipt.output = result.output ∧ result.receipt.outcome = .success :=
  joinReady_qualified_return (initialized_reachable_operational_wellFormed initialized reach)
    ready member

theorem E10_acyclic_invocation_composition {contracts : Contracts} {input : InitialInputs}
    {initial s : State} (initialized : init contracts input = some initial)
    (reach : Reachable contracts initial s) {run : Run} {depth : Nat}
    (path : InvocationPath s.toOperationalState run run depth) : depth = 0 :=
  invocation_cycles_impossible (initialized_reachable_operational_wellFormed initialized reach) path

end Kctx.Engineering
