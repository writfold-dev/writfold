import Kctx.Engineering.OperationalProofsRecordsHistory
import Kctx.Engineering.OperationalProofsRunsHistory

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

structure OperationalLinks (contracts : Contracts) (s : OperationalState) : Prop where
  effects : ∀ effect ∈ s.effects, EffectRepresentation s effect
  ownership : ∀ reservation ∈ s.ownership, ∃ run ∈ s.runs,
    run.invocation = reservation.holder ∧ run.id = reservation.owner
  receipts : ∀ receipt ∈ s.receipts, ReceiptRepresentation contracts s receipt
  returned : ∀ result ∈ s.returned, ReturnedRepresentation s result

theorem OperationalWellFormed.linkFields {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) : OperationalLinks contracts s :=
  ⟨wf.effects, wf.ownership, wf.receipts, wf.returned⟩

theorem records_domain_run_extension {contracts : Contracts} {s t : State} {action : Action}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) :
    RecordsRunExtension s.toOperationalState t.toOperationalState := by
  intro run member
  obtain ⟨next, nextMember, same⟩ := domain_preserves_run_anchors wf h run member
  exact ⟨next, nextMember, congrArg RunAnchor.invocation same,
    congrArg RunAnchor.id same, congrArg RunAnchor.parent same,
    congrArg RunAnchor.binding same, congrArg RunAnchor.procedure same,
    congrArg RunAnchor.input same⟩

theorem records_effect_iff {s : OperationalState} {effect : Effect} :
    EffectRepresentation s effect ↔
      (∃ run ∈ s.runs, run.invocation = effect.intent.invocation ∧
        run.id = effect.intent.run ∧ run.binding = effect.intent.binding ∧
        run.procedure.revision = effect.intent.procedure ∧
        ∃ node ∈ run.procedure.nodes, node.id = effect.intent.node ∧
          node.form = .effect effect.intent.capability ∧
          effect.intent.resources = node.resources ∧ effect.intent.resources ≠ []) ∧
      (∀ object ∈ effect.intent.closure, object ∈ s.retained) ∧
      (∀ claim, effect.dispatch = some claim →
        claim.intent = effect.intent ∧ claim.generation = effect.generation) ∧
      (∃ grant ∈ s.authority.grants, grant.run = effect.intent.run ∧
        grant.binding = effect.intent.binding ∧ effect.intent.capability ∈ grant.capabilities ∧
        ∀ resource ∈ effect.intent.resources, resource ∈ grant.resources) ∧
      effect.intent.expectedProvider = effect.intent.binding.expectedProvider := by
  constructor
  · intro h; exact ⟨h.activation, h.closure, h.dispatch, h.grant_bound, h.provider_binding⟩
  · rintro ⟨activation, closure, dispatch, grant, provider⟩
    exact ⟨activation, closure, dispatch, grant, provider⟩

theorem records_receipt_iff {contracts : Contracts} {s : OperationalState} {receipt : Receipt} :
    ReceiptRepresentation contracts s receipt ↔
      receipt ∈ contracts.receipts ∧
      (∃ run ∈ s.runs, run.invocation = receipt.invocation ∧
        run.binding = receipt.binding ∧ run.procedure.revision = receipt.procedure ∧
        receipt.rootInput = run.input ∧ ∃ node ∈ run.procedure.nodes, node.id = receipt.node) ∧
      receipt.outputType = receipt.output.type ∧
      (∀ object ∈ receipt.objects ++ receipt.rootInput.objects ++
        receipt.input.objects ++ receipt.output.objects, object ∈ s.retained) := by
  constructor
  · intro h; exact ⟨h.qualified, h.activation, h.output_type, h.retained⟩
  · rintro ⟨qualified, activation, output, retained⟩; exact ⟨qualified, activation, output, retained⟩

theorem records_findRun {s : State} {id : ActivationId} {run : Run}
    (h : findRun s id = some run) : run ∈ s.runs ∧ run.invocation = id := by
  exact ⟨List.mem_of_find?_eq_some h, by simpa using List.find?_some h⟩

theorem records_findNode {p : Procedure} {id : NodeId} {node : Node}
    (h : findNode p id = some node) : node ∈ p.nodes ∧ node.id = id := by
  exact ⟨List.mem_of_find?_eq_some h, by simpa using List.find?_some h⟩

theorem records_findNode_isSome {p : Procedure} {id : NodeId}
    (h : (findNode p id).isSome = true) : ∃ node ∈ p.nodes, node.id = id := by
  cases found : findNode p id with
  | none => simp [found] at h
  | some node => exact ⟨node, (records_findNode found).1, (records_findNode found).2⟩

theorem records_findEffect {s : State} {id : Id .effect} {effect : Effect}
    (h : findEffect s id = some effect) : effect ∈ s.effects :=
  List.mem_of_find?_eq_some h

theorem records_intentReady_provider {s : State} {run : Run} {intent : Intent}
    {generation : Nat} (checked : intentReady s run intent generation = true) :
    intent.expectedProvider = intent.binding.expectedProvider := by
  simp only [intentReady, Bool.and_eq_true, beq_iff_eq] at checked
  grind only

/-- Preparing an effect records the exact scoped authority that authorized its
declared, nonempty footprint; later transport needs only historical membership. -/
theorem records_intentReady_scope {s : State} {run : Run} {intent : Intent}
    {generation : Nat} (checked : intentReady s run intent generation = true) :
    intent.resources ≠ [] ∧
      ∃ grant ∈ s.authority.grants, grant.run = intent.run ∧
        grant.binding = intent.binding ∧ intent.capability ∈ grant.capabilities ∧
        ∀ resource ∈ intent.resources, resource ∈ grant.resources := by
  have nonempty : intent.resources ≠ [] := by
    intro empty
    simp [intentReady, empty] at checked
  refine ⟨nonempty, ?_⟩
  cases granted : currentGrant s run with
  | none => simp [intentReady, granted] at checked
  | some grant =>
    have scope := runs_grant_scope granted
    simp only [intentReady, granted, Bool.and_eq_true, beq_iff_eq,
      subset, List.all_eq_true, List.contains_iff_mem] at checked
    exact ⟨grant, scope.1, by grind⟩

/-- The prepare guard supplies the immutable subject; debit and framing only
transport its historical run and retained closure. -/
theorem records_prepared {s t : OperationalState} {source : State}
    (source_eq : source.toOperationalState = s)
    (runs : RecordsRunExtension s t)
    (retention : ∀ object ∈ s.retained, object ∈ t.retained)
    (grants : ∀ grant ∈ s.authority.grants, grant ∈ t.authority.grants)
    {run : Run} {node : Node} {intent : Intent} {generation : Nat}
    (found : findRun source intent.invocation = some run)
    (nodeFound : findNode run.procedure intent.node = some node)
    (ready : intentReady source run intent generation = true)
    (form : node.form = .effect intent.capability)
    (resources : intent.resources = node.resources) :
    EffectRepresentation t ⟨intent, generation, .prepared, none⟩ := by
  subst s
  apply records_effect_transport runs retention grants
  obtain ⟨nonempty, scope⟩ := records_intentReady_scope ready
  have provider := records_intentReady_provider ready
  have subject : intent.run = run.id ∧ intent.invocation = run.invocation ∧
      intent.binding = run.binding ∧ intent.procedure = run.procedure.revision ∧
      ∀ object ∈ intent.closure, object ∈ source.retained := by
    simp only [intentReady, Bool.and_eq_true, beq_iff_eq, retained, subset,
      List.all_eq_true, List.contains_iff_mem] at ready
    grind only
  refine ⟨?_, subject.2.2.2.2, ?_, scope, provider⟩
  · exact ⟨run, (records_findRun found).1, subject.2.1.symm, subject.1.symm,
      subject.2.2.1.symm, subject.2.2.2.1.symm, node,
      (records_findNode nodeFound).1, (records_findNode nodeFound).2,
      form, resources, nonempty⟩
  · intro claim impossible
    cases impossible

/-- Replacement changes only dispatch/outcome, never the recorded subject. -/
theorem records_effect_replace {s : OperationalState} {old next : Effect}
    (before : EffectRepresentation s old)
    (intent : next.intent = old.intent)
    (dispatch : ∀ claim, next.dispatch = some claim →
      claim.intent = next.intent ∧ claim.generation = next.generation) :
    EffectRepresentation s next := by
  exact ⟨by simpa only [intent] using before.activation,
    by simpa only [intent] using before.closure, dispatch,
    by simpa only [intent] using before.grant_bound,
    by simpa only [intent] using before.provider_binding⟩

theorem records_receipt_new {contracts : Contracts} {s t : State}
    (runs : RecordsRunExtension s.toOperationalState t.toOperationalState)
    (retention : ∀ object ∈ s.retained, object ∈ t.retained)
    {receipt : Receipt} {run : Run}
    (found : findRun s receipt.invocation = some run)
    (qualified : receipt ∈ contracts.receipts)
    (objects : ∀ object ∈ receipt.objects ++ receipt.rootInput.objects ++
      receipt.input.objects ++ receipt.output.objects, object ∈ s.retained)
    (input : receipt.rootInput = run.input) (binding : receipt.binding = run.binding)
    (procedure : receipt.procedure = run.procedure.revision)
    (node : (findNode run.procedure receipt.node).isSome = true)
    (output : receipt.outputType = receipt.output.type) :
    ReceiptRepresentation contracts t.toOperationalState receipt := by
  apply records_receipt_transport runs retention
  obtain ⟨n, member, id⟩ := records_findNode_isSome node
  exact ⟨qualified, ⟨run, (records_findRun found).1,
    (records_findRun found).2, binding.symm, procedure.symm, input,
    n, member, id⟩, output, objects⟩

theorem records_return_new {s t : State}
    (runs : RecordsRunExtension s.toOperationalState t.toOperationalState)
    (receipts : ∀ receipt ∈ s.receipts, receipt ∈ t.receipts)
    {parent child : Run} {parentId childId : ActivationId} {receipt : Receipt}
    (parentFound : findRun s parentId = some parent)
    (childFound : findRun s childId = some child)
    (root : child.id = parent.id) (edge : child.parent = some parent.invocation)
    (member : receipt ∈ s.receipts) (role : receipt.role = .procedureResult)
    (invocation : receipt.invocation = child.invocation)
    (input : receipt.rootInput = child.input)
    (valid : receiptValid s child receipt = true) (node : NodeId) :
    ReturnedRepresentation t.toOperationalState
      ⟨parent.invocation, child.invocation, node, child.binding,
        child.procedure.revision, child.input, receipt.output, receipt⟩ := by
  apply records_returned_transport runs receipts
  have subject : receipt.binding = child.binding ∧
      receipt.procedure = child.procedure.revision := by
    simp only [receiptValid, Bool.and_eq_true, beq_iff_eq] at valid
    grind only
  exact ⟨member, role, invocation, subject.1, subject.2, input, rfl,
    parent, (records_findRun parentFound).1, child, (records_findRun childFound).1,
    rfl, rfl, edge, root, rfl, rfl, rfl⟩

/-- List surgery is discharged before inspecting the small record payload. -/
theorem records_map_links {α : Type} {P : α → Prop} {xs : List α} {f : α → α}
    (step : ∀ old ∈ xs, P (f old)) : ∀ next ∈ xs.map f, P next := by
  intro next member
  obtain ⟨old, oldMember, rfl⟩ := List.mem_map.mp member
  exact step old oldMember

theorem records_filter_links {α : Type} {P : α → Prop} {xs : List α}
    {keep : α → Bool} (before : ∀ old ∈ xs, P old) :
    ∀ next ∈ xs.filter keep, P next := by
  intro next member
  exact before next (List.mem_filter.mp member).1

theorem records_cons_links {α : Type} {P : α → Prop} {xs : List α} {x : α}
    (fresh : P x) (before : ∀ old ∈ xs, P old) :
    ∀ next ∈ x :: xs, P next := by
  intro next member
  rcases List.mem_cons.mp member with rfl | oldMember
  · exact fresh
  · exact before next oldMember

theorem records_append_links {α : Type} {P : α → Prop} {xs ys : List α}
    (left : ∀ old ∈ xs, P old) (right : ∀ old ∈ ys, P old) :
    ∀ next ∈ xs ++ ys, P next := by
  intro next member
  rcases List.mem_append.mp member with oldMember | oldMember
  · exact left next oldMember
  · exact right next oldMember

theorem records_reservation_new {s t : State}
    (runs : RecordsRunExtension s.toOperationalState t.toOperationalState)
    {run : Run} {id : ActivationId} (found : findRun s id = some run)
    (key : Id .resource) (generation : Nat) :
    ∃ next ∈ t.runs, next.invocation =
        (Reservation.mk key run.id run.invocation generation 0 0).holder ∧
      next.id = (Reservation.mk key run.id run.invocation generation 0 0).owner := by
  exact records_ownership_transport runs
    ⟨run, (records_findRun found).1, rfl, rfl⟩

end Kctx.Engineering