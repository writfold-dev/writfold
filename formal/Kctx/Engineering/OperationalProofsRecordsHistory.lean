import Kctx.Engineering.OperationalProofsRecordsCatalog

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Only the immutable subjects of an activation are needed to interpret old
records. Status, control progress and current authority are deliberately absent. -/
def RecordsRunExtension (s t : OperationalState) : Prop :=
  ∀ run ∈ s.runs, ∃ next ∈ t.runs,
    next.invocation = run.invocation ∧ next.id = run.id ∧ next.parent = run.parent ∧
    next.binding = run.binding ∧ next.procedure = run.procedure ∧ next.input = run.input

theorem records_effect_transport {s t : OperationalState} {effect : Effect}
    (runs : RecordsRunExtension s t)
    (retained : ∀ object ∈ s.retained, object ∈ t.retained)
    (grants : ∀ grant ∈ s.authority.grants, grant ∈ t.authority.grants)
    (before : EffectRepresentation s effect) : EffectRepresentation t effect := by
  obtain ⟨run, member, invocation, root, binding, procedure, node, nodeMember, nodeId, form⟩ :=
    before.activation
  obtain ⟨next, nextMember, nextInvocation, nextRoot, _, nextBinding, nextProcedure, _⟩ :=
    runs run member
  refine ⟨?_, fun object member => retained object (before.closure object member),
    before.dispatch, ?_, before.provider_binding⟩
  exact ⟨next, nextMember, nextInvocation.trans invocation, nextRoot.trans root,
    nextBinding.trans binding, by simpa only [nextProcedure] using procedure,
    node, by simpa only [nextProcedure] using nodeMember, nodeId, form⟩
  obtain ⟨grant, member, scope⟩ := before.grant_bound
  exact ⟨grant, grants grant member, scope⟩

theorem records_receipt_transport {contracts : Contracts} {s t : OperationalState} {receipt : Receipt}
    (runs : RecordsRunExtension s t)
    (retained : ∀ object ∈ s.retained, object ∈ t.retained)
    (before : ReceiptRepresentation contracts s receipt) : ReceiptRepresentation contracts t receipt := by
  obtain ⟨run, member, invocation, binding, procedure, input, node, nodeMember, nodeId⟩ :=
    before.activation
  obtain ⟨next, nextMember, nextInvocation, _, _, nextBinding, nextProcedure, nextInput⟩ :=
    runs run member
  refine ⟨before.qualified, ?_, before.output_type,
    fun object member => retained object (before.retained object member)⟩
  exact ⟨next, nextMember, nextInvocation.trans invocation, nextBinding.trans binding,
    by simpa only [nextProcedure] using procedure,
    input.trans nextInput.symm, node, by simpa only [nextProcedure] using nodeMember, nodeId⟩

theorem records_ownership_transport {s t : OperationalState} {reservation : Reservation}
    (runs : RecordsRunExtension s t)
    (before : ∃ run ∈ s.runs, run.invocation = reservation.holder ∧ run.id = reservation.owner) :
    ∃ run ∈ t.runs, run.invocation = reservation.holder ∧ run.id = reservation.owner := by
  obtain ⟨run, member, invocation, root⟩ := before
  obtain ⟨next, nextMember, nextInvocation, nextRoot, _⟩ := runs run member
  exact ⟨next, nextMember, nextInvocation.trans invocation, nextRoot.trans root⟩

/-- Membership and exact subjects, not reuse validity at the present time. -/
def ReturnedRepresentation (s : OperationalState) (result : ChildReturn) : Prop :=
  result.receipt ∈ s.receipts ∧ result.receipt.role = .procedureResult ∧
  result.receipt.invocation = result.child ∧
  result.receipt.binding = result.binding ∧ result.receipt.procedure = result.procedure ∧
  result.receipt.rootInput = result.input ∧ result.receipt.output = result.output ∧
  ∃ parent ∈ s.runs, ∃ child ∈ s.runs,
    parent.invocation = result.parent ∧ child.invocation = result.child ∧
    child.parent = some parent.invocation ∧ child.id = parent.id ∧
    child.binding = result.binding ∧ child.procedure.revision = result.procedure ∧
    child.input = result.input

theorem records_returned_transport {s t : OperationalState} {result : ChildReturn}
    (runs : RecordsRunExtension s t)
    (receipts : ∀ receipt ∈ s.receipts, receipt ∈ t.receipts)
    (before : ReturnedRepresentation s result) : ReturnedRepresentation t result := by
  rcases before with ⟨receipt, role, invocation, binding, procedure, input, output,
    parent, parentMember, child, childMember, parentId, childId, edge, root,
    childBinding, childProcedure, childInput⟩
  obtain ⟨p, pMember, pId, pRoot, _, _, _, _⟩ := runs parent parentMember
  obtain ⟨c, cMember, cId, cRoot, cParent, cBinding, cProcedure, cInput⟩ := runs child childMember
  exact ⟨receipts _ receipt, role, invocation, binding, procedure, input, output,
    p, pMember, c, cMember, pId.trans parentId, cId.trans childId,
    by simpa only [pId] using cParent.trans edge,
    cRoot.trans (root.trans pRoot.symm), cBinding.trans childBinding,
    by simpa only [cProcedure] using childProcedure, cInput.trans childInput⟩

theorem records_domainBody_retention {contracts : Contracts} {s t : State} {action : Action}
    (h : domainBody contracts s action = some t) :
    (∀ object ∈ s.retained, object ∈ (frameDomain s action t).retained) ∧
    (∀ receipt ∈ s.receipts, receipt ∈ (frameDomain s action t).receipts) := by
  records_split_domain h action
  all_goals constructor
  all_goals first
    | exact fun _ member => member
    | exact records_retained_extension _ _
    | exact fun _ member => List.mem_cons_of_mem _ member

theorem records_domain_retention {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) :
    (∀ object ∈ s.retained, object ∈ t.retained) ∧
    (∀ receipt ∈ s.receipts, receipt ∈ t.receipts) := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    exact records_domainBody_retention body

end Kctx.Engineering
