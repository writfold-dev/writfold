import Kctx.Engineering.OperationalProofs

set_option autoImplicit false
namespace Kctx.Engineering

/-- A dependency edge cannot be satisfied by another node, another declared
type, or a non-success completion. This is the actual enabled-node guard. -/
theorem enabled_exact_dependency {s : State} {run : Run} {node : Node}
    (enabledNode : enabled s run node = true) {dependency : Dependency}
    (member : dependency ∈ node.dependencies) :
    ∃ result ∈ run.control.results, result.node = dependency.node ∧
      result.value.type = dependency.type ∧ result.outcome = .success := by
  have dependencies : node.dependencies.all (fun dependency =>
      run.control.results.any (fun result => result.node == dependency.node &&
        result.value.type == dependency.type && result.outcome == .success)) = true := by
    unfold enabled at enabledNode
    simp only [Bool.and_eq_true] at enabledNode
    grind only
  have result := List.all_eq_true.mp dependencies dependency member
  simpa only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, and_assoc] using result

/-- In the representation invariant the dependency result is unique. A second
receipt with the same shape cannot introduce a competing control result. -/
theorem enabled_unique_dependency {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) {run : Run}
    (member : run ∈ s.runs) {node : Node} (ready : enabled s run node = true)
    {dependency : Dependency} (edge : dependency ∈ node.dependencies) :
    ∃ result ∈ run.control.results, result.node = dependency.node ∧
      result.value.type = dependency.type ∧ result.outcome = .success ∧
      ∀ other ∈ run.control.results, other.node = dependency.node → other = result := by
  obtain ⟨result, resultMember, identity, type, outcome⟩ := enabled_exact_dependency ready edge
  refine ⟨result, resultMember, identity, type, outcome, ?_⟩
  intro other otherMember otherId
  exact member_eq_of_unique_key (wf.runs run member).results_unique otherMember resultMember
    (otherId.trans identity.symm)

/-- Joins inspect completed invocations and exact call-input bindings, not a
bag of similarly typed artifacts or a child from another root execution. -/
theorem joinReady_exact_invocation {s : State} {run : Run} {calls : List NodeId}
    (ready : joinReady s run calls = true) {node : NodeId} (member : node ∈ calls) :
    ∃ result ∈ s.returned, result.parent = run.invocation ∧ result.node = node ∧
      result.binding = run.binding ∧ result.receipt.outcome = .success ∧
      ∃ child, findRun s result.child = some child ∧ child.id = run.id ∧
        child.parent = some run.invocation ∧ child.status = .completed ∧
        child.binding = run.binding ∧ child.procedure.revision = result.procedure ∧
        child.input = result.input := by
  have checked := List.all_eq_true.mp ready node member
  simpa only [List.any_eq_true, Option.any_eq_true, Bool.and_eq_true,
    beq_iff_eq, and_assoc] using checked

/-- Historical qualification composes with the live Join check. Revocation or
expiry does not erase this stored return's exact subject. -/
theorem joinReady_qualified_return {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) {run : Run} {calls : List NodeId}
    (ready : joinReady s run calls = true) {node : NodeId} (member : node ∈ calls) :
    ∃ result ∈ s.returned, result.parent = run.invocation ∧ result.node = node ∧
      result.receipt ∈ contracts.receipts ∧ result.receipt.role = .procedureResult ∧
      result.receipt.invocation = result.child ∧ result.receipt.rootInput = result.input ∧
      result.receipt.output = result.output ∧ result.receipt.outcome = .success := by
  obtain ⟨result, resultMember, parent, nodeEq, _, success, _⟩ :=
    joinReady_exact_invocation ready member
  obtain ⟨receiptMember, role, invocation, _, _, input, output, _⟩ :=
    wf.returned result resultMember
  exact ⟨result, resultMember, parent, nodeEq, (wf.receipts _ receiptMember).qualified,
    role, invocation, input, output, success⟩

theorem admitted_receipt_retains_exact_inputs {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {receipt : Receipt} (member : receipt ∈ s.receipts) :
    (∀ object ∈ receipt.rootInput.objects, object ∈ s.retained) ∧
    (∀ object ∈ receipt.input.objects, object ∈ s.retained) ∧
    (∀ object ∈ receipt.output.objects, object ∈ s.retained) := by
  have retained := (wf.receipts receipt member).retained
  exact ⟨fun object h => retained object (by simp [h]),
    fun object h => retained object (by simp [h]),
    fun object h => retained object (by simp [h])⟩

/-- Resource authority is retained as an exact historical binding witness;
revocation changes continuation, not the scope under which an intent was admitted. -/
theorem admitted_effect_scoped_grant {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {effect : Effect} (member : effect ∈ s.effects) :
    ∃ grant ∈ s.authority.grants, grant.run = effect.intent.run ∧
      grant.binding = effect.intent.binding ∧ effect.intent.capability ∈ grant.capabilities ∧
      ∀ resource ∈ effect.intent.resources, resource ∈ grant.resources :=
  (wf.effects effect member).grant_bound

theorem admitted_effect_declared_footprint {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {effect : Effect} (member : effect ∈ s.effects) :
    effect.intent.resources ≠ [] ∧ effect.intent.resources.Nodup := by
  obtain ⟨run, runMember, _, _, _, _, node, nodeMember, _, form, footprint, nonempty⟩ :=
    (wf.effects effect member).activation
  have catalog := (wf.runs run runMember).catalog
  have typed := (wf.catalog_authority run.procedure catalog).2.2.1
  have nodeTyped := typed.2.2.2.2.2 node nodeMember
  rcases nodeTyped with ⟨_, _, _, _, _, _, _, _, _, formTyped⟩
  simp only [FormTyped, form] at formTyped
  exact ⟨nonempty, by simpa only [footprint] using formTyped.2.2⟩

/-- Required context is per invocation and must actually be loaded; mere
durable retention or a package mentioning an object cannot satisfy readiness. -/
theorem nodeReady_context_loaded_retained {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) {run : Run}
    (member : run ∈ s.runs) {node : Node} (ready : nodeReady contracts s run node = true) :
    ∀ object ∈ node.requiredContext,
      object ∈ run.loaded ∧ object ∈ run.procedure.closure ∧ object ∈ s.retained := by
  have context : subset node.requiredContext run.loaded = true := by
    simp only [nodeReady, Bool.and_eq_true] at ready
    exact ready.1.2
  have loaded : ∀ object ∈ node.requiredContext, object ∈ run.loaded := by
    simpa only [subset, List.all_eq_true, List.contains_iff_mem] using context
  intro object required
  have present := loaded object required
  exact ⟨present, (wf.runs run member).loaded object present⟩

/-- The provider CAS revision belongs to the immutable invocation binding,
not whichever provider revision happens to be current during replay or retry. -/
theorem admitted_effect_provider_binding {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {effect : Effect} (member : effect ∈ s.effects) :
    ∃ run ∈ s.runs, run.invocation = effect.intent.invocation ∧
      effect.intent.expectedProvider = run.binding.expectedProvider := by
  have represented := wf.effects effect member
  obtain ⟨run, runMember, invocation, _, binding, _⟩ := represented.activation
  exact ⟨run, runMember, invocation,
    represented.provider_binding.trans (congrArg Binding.expectedProvider binding.symm)⟩

theorem retained_claim_stamp_unique {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {left right : Dispatch}
    (leftMember : left ∈ s.claims) (rightMember : right ∈ s.claims)
    (sameId : left.id = right.id) : left = right :=
  member_eq_of_unique_key wf.claims_unique leftMember rightMember sameId

theorem current_dispatch_is_retained {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {effect : Effect} (member : effect ∈ s.effects)
    {stamp : Dispatch} (current : effect.dispatch = some stamp) : stamp ∈ s.claims :=
  wf.dispatch_inventory effect member stamp current

/-- Captured history cannot present issued and unissued versions of one claim
ID as two independently authoritative retained stamps. -/
theorem operationalValid_refuses_conflicting_claim_stamps {contracts : Contracts}
    {s : OperationalState} {left right : Dispatch}
    (leftMember : left ∈ s.claims) (rightMember : right ∈ s.claims)
    (sameId : left.id = right.id) (issued : left.issued = true) (unissued : right.issued = false) :
    operationalValid contracts s = false := by
  cases checked : operationalValid contracts s with
  | false => rfl
  | true =>
      have wf := (operationalValid_iff contracts s).mp checked
      have same := retained_claim_stamp_unique wf leftMember rightMember sameId
      have flags := congrArg Dispatch.issued same
      simp only [issued, unissued, Bool.true_eq_false] at flags

theorem operationalValid_refuses_duplicate_claim_ids {contracts : Contracts}
    {s : OperationalState} (duplicate : ¬(s.claims.map Dispatch.id).Nodup) :
    operationalValid contracts s = false := by
  cases checked : operationalValid contracts s with
  | false => rfl
  | true => exact False.elim (duplicate ((operationalValid_iff contracts s).mp checked).claims_unique)

end Kctx.Engineering
