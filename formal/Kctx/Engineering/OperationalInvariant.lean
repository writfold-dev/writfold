import Kctx.Engineering.Core

set_option autoImplicit false
namespace Kctx.Engineering

/-- Historical representation deliberately excludes current grant liveness,
expiry, policy equality and withdrawal status. Those are continuation premises. -/
structure RunRepresentation (contracts : Contracts) (s : OperationalState) (r : Run) : Prop where
  catalog : r.procedure ∈ s.catalog
  realm : r.binding.realm = s.realm
  input_type : r.input.type = r.procedure.inputType
  input_retained : ∀ object ∈ r.input.objects, object ∈ s.retained
  closure_retained : ∀ object ∈ r.procedure.closure, object ∈ s.retained
  capabilities : ∀ cap ∈ r.capabilities, cap ∈ r.procedure.ceiling
  grant_bound : ∃ grant ∈ s.authority.grants, grant.run = r.id ∧
    grant.binding = r.binding ∧ ∀ cap ∈ r.capabilities, cap ∈ grant.capabilities
  procedure_floor : ∀ gate ∈ r.procedure.floor, gate ∈ r.floor
  kernel_floor : ∀ gate ∈ contracts.kernelFloor, gate ∈ r.floor
  account : ∃ budget ∈ s.budgets, budget.root = r.id
  loaded : ∀ object ∈ r.loaded, object ∈ r.procedure.closure ∧ object ∈ s.retained
  results_unique : (r.control.results.map NodeResult.node).Nodup
  results : ∀ result ∈ r.control.results, ∃ node ∈ r.procedure.nodes,
    node.id = result.node ∧
    (result.outcome = .success → result.value.type = node.output) ∧
    (∀ object ∈ result.value.objects, object ∈ s.retained)
  routes : ∀ edge ∈ r.control.routes, ∃ source ∈ r.procedure.nodes,
    source.id = edge.1 ∧ ForwardRoute r.procedure source edge.2 ∧
    (edge.2 ∈ source.failure.toList ++ source.cancellation.toList ∨
      match source.form with
      | .select targets => edge.2 ∈ targets
      | _ => False)
  parent : ∀ id, r.parent = some id → ∃ p ∈ s.runs,
    p.invocation = id ∧ p.id = r.id ∧ p.stream = r.stream ∧ p.binding = r.binding ∧
    r.procedure.rank < p.procedure.rank ∧
    ∃ delegation, r.delegation = some delegation ∧ delegation.parent = id ∧
      (∀ cap ∈ r.capabilities, cap ∈ delegation.capabilities) ∧
      (∀ gate ∈ delegation.floor, gate ∈ r.floor)
  root_delegation : r.parent = none → r.delegation = none
  call_origin : ∀ id, r.parent = some id → ∃ p ∈ s.runs,
    p.invocation = id ∧ ∃ node ∈ p.procedure.nodes, ∃ target,
      node.form = .call target ∧ target.procedure = r.procedure.id ∧
      target.revision = r.procedure.revision ∧ target.input = r.procedure.inputType ∧
      target.output = r.procedure.outputType ∧
      (∀ cap ∈ r.capabilities, cap ∈ target.delegation ∧ cap ∈ node.ceiling)
  pending_nodes_unique : (r.control.pending.map PendingCall.node).Nodup
  pending_children_unique : (r.control.pending.map PendingCall.child).Nodup
  pending_results_disjoint : ∀ pending ∈ r.control.pending,
    ∀ result ∈ r.control.results, pending.node ≠ result.node
  pending : ∀ pending ∈ r.control.pending, ∃ node ∈ r.procedure.nodes,
    node.id = pending.node ∧ node.form = .call pending.target ∧
    pending.binding = r.binding ∧
    ∃ child ∈ s.runs, child.invocation = pending.child ∧
      child.parent = some r.invocation ∧ child.id = r.id ∧
      child.procedure.id = pending.target.procedure ∧
      child.procedure.revision = pending.target.revision ∧ child.input = pending.input

structure EffectRepresentation (s : OperationalState) (effect : Effect) : Prop where
  activation : ∃ run ∈ s.runs, run.invocation = effect.intent.invocation ∧
    run.id = effect.intent.run ∧ run.binding = effect.intent.binding ∧
    run.procedure.revision = effect.intent.procedure ∧
    ∃ node ∈ run.procedure.nodes, node.id = effect.intent.node ∧
      node.form = .effect effect.intent.capability ∧
      effect.intent.resources = node.resources ∧ effect.intent.resources ≠ []
  closure : ∀ object ∈ effect.intent.closure, object ∈ s.retained
  dispatch : ∀ claim, effect.dispatch = some claim →
    claim.intent = effect.intent ∧ claim.generation = effect.generation
  grant_bound : ∃ grant ∈ s.authority.grants, grant.run = effect.intent.run ∧
    grant.binding = effect.intent.binding ∧ effect.intent.capability ∈ grant.capabilities ∧
    ∀ resource ∈ effect.intent.resources, resource ∈ grant.resources
  provider_binding : effect.intent.expectedProvider = effect.intent.binding.expectedProvider

structure ReceiptRepresentation (contracts : Contracts) (s : OperationalState)
    (receipt : Receipt) : Prop where
  qualified : receipt ∈ contracts.receipts
  activation : ∃ run ∈ s.runs, run.invocation = receipt.invocation ∧
    run.binding = receipt.binding ∧ run.procedure.revision = receipt.procedure ∧
    receipt.rootInput = run.input ∧ ∃ node ∈ run.procedure.nodes, node.id = receipt.node
  output_type : receipt.outputType = receipt.output.type
  retained : ∀ object ∈ receipt.objects ++ receipt.rootInput.objects ++
    receipt.input.objects ++ receipt.output.objects, object ∈ s.retained

/-- E02 representation over the actual interpreter state. Historical package
pins are existential release witnesses, not equality with today's corpus HEAD.
A revoked grant remains a bounded historical grant; it cannot authorize live work. -/
structure OperationalWellFormed (contracts : Contracts) (s : OperationalState) : Prop where
  portfolio : portfolioStateValid s.knowledge = true
  grants_unique : unique (s.authority.grants.map Grant.id) = true
  governing_floor : ∀ gate ∈ contracts.kernelFloor, gate ∈ s.authority.floor
  corpus_binding : (s.knowledge.kb.snapshot, s.corpusPin) ∈ contracts.corpusBindings
  corpus_realm : s.corpusPin.realm = s.realm
  corpus_family : s.corpusPin.family = .corpus
  activity_realm : s.activityPin.realm = s.realm
  activity_family : s.activityPin.family = .activity
  envelopes_retained : ∀ object ∈ [s.corpusPin.envelope, s.activityPin.envelope,
    s.projectionEnvelope, s.visibilityEnvelope], object ∈ s.retained
  catalog_release : s.catalogRelease = s.knowledge.kb.snapshot
  catalog_unique : (s.catalog.map Procedure.revision).Nodup
  catalog_authority : ∀ procedure ∈ s.catalog,
    procedure ∈ contracts.packages ∧
    (∃ release, (release, procedure) ∈ contracts.packageReleases) ∧
    ProcedureTyped s.catalog contracts.predicates procedure ∧
    (∀ object ∈ procedure.closure, object ∈ s.retained)
  activations_unique : (s.runs.map Run.invocation).Nodup
  accounts_unique : (s.budgets.map RootBudget.root).Nodup
  accounts_bounded : ∀ budget ∈ s.budgets, budget.spent ≤ budget.limit
  runs : ∀ run ∈ s.runs, RunRepresentation contracts s run
  effects_unique : (s.effects.map (fun effect => effect.intent.effect)).Nodup
  effects : ∀ effect ∈ s.effects, EffectRepresentation s effect
  ownership_unique : (s.ownership.map Reservation.resource).Nodup
  ownership : ∀ reservation ∈ s.ownership, ∃ run ∈ s.runs,
    run.invocation = reservation.holder ∧ run.id = reservation.owner
  candidates_unique : (s.candidates.map (fun candidate : Candidate => candidate.id)).Nodup
  candidates : ∀ candidate ∈ s.candidates,
    (∀ object ∈ candidate.evidence, object ∈ s.retained) ∧
    (candidate.evaluated = true → candidate.toCandidateMaterial ∈ contracts.evaluations) ∧
    (candidate.reviewed = true → candidate.evaluated = true ∧
      candidate.toCandidateMaterial ∈ contracts.reviews)
  captures_unique : (s.captures.map RetainedObservation.id).Nodup
  captures : ∀ capture ∈ s.captures,
    capture ∈ contracts.materialObservations ∧ capture.realm = s.realm ∧
    capture.capturedAt ≤ s.now ∧ ∀ object ∈ capture.objects, object ∈ s.retained
  receipts : ∀ receipt ∈ s.receipts, ReceiptRepresentation contracts s receipt
  returned : ∀ result ∈ s.returned,
    result.receipt ∈ s.receipts ∧ result.receipt.role = .procedureResult ∧
    result.receipt.invocation = result.child ∧
    result.receipt.binding = result.binding ∧ result.receipt.procedure = result.procedure ∧
    result.receipt.rootInput = result.input ∧ result.receipt.output = result.output ∧
    ∃ parent ∈ s.runs, ∃ child ∈ s.runs,
      parent.invocation = result.parent ∧ child.invocation = result.child ∧
      child.parent = some parent.invocation ∧ child.id = parent.id ∧
      child.binding = result.binding ∧ child.procedure.revision = result.procedure ∧
      child.input = result.input
  claims_unique : (s.claims.map Dispatch.id).Nodup
  claim_effects : ∀ claim ∈ s.claims, ∃ effect ∈ s.effects,
    effect.intent = claim.intent ∧ effect.generation = claim.generation
  dispatch_inventory : ∀ effect ∈ s.effects, ∀ claim,
    effect.dispatch = some claim → claim ∈ s.claims

/-- Optional subjects and pinned pairs have finite constructive searches even
though their nominal identity types are unbounded. -/
private instance optionSubjectDecidable {α : Type} (subject : Option α)
    (predicate : α → Prop) [DecidablePred predicate] :
    Decidable (∀ value, subject = some value → predicate value) := by
  cases subject with
  | none => exact isTrue (by simp)
  | some value => exact decidable_of_iff (predicate value) (by simp)

private instance optionSubjectExistsDecidable {α : Type} (subject : Option α)
    (predicate : α → Prop) [DecidablePred predicate] :
    Decidable (∃ value, subject = some value ∧ predicate value) := by
  cases subject with
  | none => exact isFalse (by simp)
  | some value => exact decidable_of_iff (predicate value) (by simp)

private instance callSubjectDecidable (form : NodeForm) (predicate : CallTarget → Prop)
    [DecidablePred predicate] :
    Decidable (∃ target, form = .call target ∧ predicate target) := by
  cases form with
  | call target => exact decidable_of_iff (predicate target) (by simp)
  | read | select | judge | effect | check | join | «end» =>
      exact isFalse (by rintro ⟨target, equal, _⟩; cases equal)

private instance selectedRouteDecidable (form : NodeForm) (target : NodeId) :
    Decidable (match form with
      | .select targets => target ∈ targets
      | _ => False) := by
  cases form <;> infer_instance

private instance pinnedPairDecidable {α β : Type} [DecidableEq β]
    (pairs : List (α × β)) (value : β) :
    Decidable (∃ pin, (pin, value) ∈ pairs) := by
  apply decidable_of_iff (∃ pair ∈ pairs, pair.2 = value)
  constructor
  · rintro ⟨⟨pin, item⟩, member, equal⟩
    exact ⟨pin, equal ▸ member⟩
  · rintro ⟨pin, member⟩
    exact ⟨(pin, value), member, rfl⟩

/-- The call-origin search follows the one stored parent identity, the finite
run/node lists, and the target carried by a Call constructor. It never searches
the unbounded ActivationId or CallTarget types. -/
private def callOriginDecision (s : OperationalState) (r : Run) :
    Decidable (∀ id, r.parent = some id → ∃ parent ∈ s.runs,
      parent.invocation = id ∧ ∃ node ∈ parent.procedure.nodes, ∃ target,
        node.form = .call target ∧ target.procedure = r.procedure.id ∧
        target.revision = r.procedure.revision ∧ target.input = r.procedure.inputType ∧
        target.output = r.procedure.outputType ∧
        (∀ cap ∈ r.capabilities, cap ∈ target.delegation ∧ cap ∈ node.ceiling)) := by
  refine @optionSubjectDecidable ActivationId r.parent _ ?_
  intro id
  refine @List.decidableBEx Run _ ?_ s.runs
  intro parent
  refine @instDecidableAnd _ _ inferInstance ?_
  refine @List.decidableBEx Node _ ?_ parent.procedure.nodes
  intro node
  refine @callSubjectDecidable node.form _ ?_
  intro target
  infer_instance

instance (contracts : Contracts) (s : OperationalState) (r : Run) :
    Decidable (RunRepresentation contracts s r) := by
  have fields : RunRepresentation contracts s r ↔
      (_ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧
        _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _) :=
    ⟨fun h => ⟨h.catalog, h.realm, h.input_type, h.input_retained, h.closure_retained,
      h.capabilities, h.grant_bound, h.procedure_floor, h.kernel_floor, h.account,
      h.loaded, h.results_unique, h.results, h.routes, h.parent, h.root_delegation, h.call_origin,
      h.pending_nodes_unique, h.pending_children_unique, h.pending_results_disjoint, h.pending⟩,
      fun ⟨a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u⟩ =>
        ⟨a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u⟩⟩
  exact @decidable_of_iff _ _ fields.symm (by
    repeat first
      | refine @instDecidableAnd _ _ ?_ ?_
      | exact callOriginDecision s r
      | infer_instance)

instance (s : OperationalState) (effect : Effect) :
    Decidable (EffectRepresentation s effect) := by
  have fields : EffectRepresentation s effect ↔ (_ ∧ _ ∧ _ ∧ _ ∧ _) :=
    ⟨fun h => ⟨h.activation, h.closure, h.dispatch, h.grant_bound, h.provider_binding⟩,
      fun ⟨a, b, c, d, e⟩ => ⟨a, b, c, d, e⟩⟩
  exact decidable_of_iff _ fields.symm

instance (contracts : Contracts) (s : OperationalState) (receipt : Receipt) :
    Decidable (ReceiptRepresentation contracts s receipt) := by
  have fields : ReceiptRepresentation contracts s receipt ↔ (_ ∧ _ ∧ _ ∧ _) :=
    ⟨fun h => ⟨h.qualified, h.activation, h.output_type, h.retained⟩,
      fun ⟨a, b, c, d⟩ => ⟨a, b, c, d⟩⟩
  exact decidable_of_iff _ fields.symm

instance (contracts : Contracts) (s : OperationalState) :
    Decidable (OperationalWellFormed contracts s) := by
  have fields : OperationalWellFormed contracts s ↔
      (_ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧
        _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _) :=
    ⟨fun h => ⟨h.portfolio, h.grants_unique, h.governing_floor, h.corpus_binding,
      h.corpus_realm, h.corpus_family, h.activity_realm, h.activity_family,
      h.envelopes_retained, h.catalog_release, h.catalog_unique, h.catalog_authority,
      h.activations_unique, h.accounts_unique, h.accounts_bounded, h.runs,
      h.effects_unique, h.effects, h.ownership_unique, h.ownership,
      h.candidates_unique, h.candidates, h.captures_unique, h.captures,
      h.receipts, h.returned, h.claims_unique, h.claim_effects, h.dispatch_inventory⟩,
      fun ⟨a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z,
          aa, bb, cc⟩ =>
        ⟨a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z,
          aa, bb, cc⟩⟩
  exact @decidable_of_iff _ _ fields.symm (by
    repeat first
      | refine @instDecidableAnd _ _ ?_ ?_
      | infer_instance)

/-- This checker validates captured inputs before replay. It is never a filter
on a computed successor and supplies no preservation conclusion. -/
def operationalValid (contracts : Contracts) (state : OperationalState) : Bool :=
  decide (OperationalWellFormed contracts state)

theorem operationalValid_iff (contracts : Contracts) (state : OperationalState) :
    operationalValid contracts state = true ↔ OperationalWellFormed contracts state := by
  constructor
  · intro checked
    exact of_decide_eq_true checked
  · intro represented
    exact decide_eq_true represented

end Kctx.Engineering
