import Kctx.Engineering.OperationalProofsRunsProgress

set_option autoImplicit false
namespace Kctx.Engineering

def calledRun (s : State) (p : Run) (n : Node) (g : Grant)
    (target : CallTarget) (invocation : ActivationId) (callee : Procedure) : Run :=
  { p with
    invocation := invocation
    parent := some p.invocation
    procedure := callee
    delegation := some ⟨p.invocation, p.capabilities, effectiveNodeFloor s p n⟩
    input := nodeInput p n
    capabilities := childCapabilities g.capabilities
      (childCapabilities p.capabilities target.delegation n.ceiling) callee.ceiling
    floor := inheritedFloor (effectiveNodeFloor s p n) callee.floor s.authority.floor
    loaded := []
    control := ⟨[], [], []⟩ }

theorem runs_called_representation {contracts : Contracts} {s : State} {t : OperationalState}
    {p : Run} (rep : RunRepresentation contracts t p) {n : Node} {g : Grant}
    {target : CallTarget} {invocation : ActivationId} {callee : Procedure}
    (node : n ∈ p.procedure.nodes) (form : n.form = .call target)
    (catalog : callee ∈ t.catalog)
    (input : (nodeInput p n).type = callee.inputType)
    (closure : ∀ object ∈ callee.closure, object ∈ t.retained)
    (grant : g ∈ t.authority.grants) (grantRoot : g.run = p.id)
    (grantBinding : g.binding = p.binding)
    (rank : callee.rank < p.procedure.rank)
    (pins : target.procedure = callee.id ∧ target.revision = callee.revision ∧
      target.input = callee.inputType ∧ target.output = callee.outputType)
    (parent : ∃ q ∈ t.runs, q.anchor = p.anchor) :
    RunRepresentation contracts t (calledRun s p n g target invocation callee) := by
  obtain ⟨q, hq, anchor⟩ := parent
  have hi := congrArg RunAnchor.invocation anchor
  have hr := congrArg RunAnchor.id anchor
  have hs := congrArg RunAnchor.stream anchor
  have hb := congrArg RunAnchor.binding anchor
  have hc := congrArg RunAnchor.procedure anchor
  simp only [Run.anchor] at hi hr hs hb hc
  refine {
    catalog := catalog
    realm := rep.realm
    input_type := input
    input_retained := runs_nodeInput_retained rep n
    closure_retained := closure
    capabilities := ?_
    grant_bound := ?_
    procedure_floor := ?_
    kernel_floor := ?_
    account := rep.account
    loaded := by simp [calledRun]
    results_unique := by simp [calledRun]
    results := by simp [calledRun]
    routes := by simp [calledRun]
    parent := ?_
    root_delegation := by simp [calledRun]
    call_origin := ?_
    pending_nodes_unique := by simp [calledRun]
    pending_children_unique := by simp [calledRun]
    pending_results_disjoint := by simp [calledRun]
    pending := by simp [calledRun] }
  · intro cap member
    exact (call_capabilities_bounded member).2.2.2.2
  · exact ⟨g, grant, grantRoot, grantBinding,
      fun cap member => (call_capabilities_bounded member).1⟩
  · intro gate member
    change gate ∈ callee.floor at member
    simp [calledRun, inheritedFloor, member]
  · intro gate member
    have keep := rep.kernel_floor gate member
    simp [calledRun, inheritedFloor, effectiveNodeFloor, keep]
  · intro id edge
    have idEq : p.invocation = id := Option.some.inj edge
    refine ⟨q, hq, hi.trans idEq, hr, hs, hb, ?_,
      ⟨p.invocation, p.capabilities, effectiveNodeFloor s p n⟩, rfl, idEq, ?_, ?_⟩
    · simpa [calledRun, hc] using rank
    · intro cap member
      exact (call_capabilities_bounded member).2.1
    · intro gate member
      simp [calledRun, inheritedFloor, member]
  · intro id edge
    have idEq : p.invocation = id := Option.some.inj edge
    refine ⟨q, hq, hi.trans idEq, n, ?_, target, form,
      pins.1, pins.2.1, pins.2.2.1, pins.2.2.2, ?_⟩
    · simpa only [hc] using node
    · intro cap member
      have bound := call_capabilities_bounded member
      exact ⟨bound.2.2.1, bound.2.2.2.1⟩

theorem runs_pending_child_fresh {contracts : Contracts} {s : OperationalState}
    {p : Run} (rep : RunRepresentation contracts s p) {child : ActivationId}
    (fresh : s.runs.any (·.invocation == child) = false) :
    child ∉ p.control.pending.map PendingCall.child := by
  intro member
  obtain ⟨waiting, member, same⟩ := List.mem_map.mp member
  obtain ⟨_, _, _, _, _, c, hc, id, _⟩ := rep.pending waiting member
  have yes : s.runs.any (·.invocation == child) = true :=
    List.any_eq_true.mpr ⟨c, hc, by simpa only [id, same, beq_self_eq_true]⟩
  simp [fresh] at yes

theorem RunRepresentation.wait {contracts : Contracts} {s : OperationalState}
    {p : Run} (rep : RunRepresentation contracts s p) {n : Node} {target : CallTarget}
    {child : Run} (node : n ∈ p.procedure.nodes) (form : n.form = .call target)
    (freshNode : n.id ∉ p.control.pending.map PendingCall.node)
    (freshResult : n.id ∉ p.control.results.map NodeResult.node)
    (freshChild : child.invocation ∉ p.control.pending.map PendingCall.child)
    (member : child ∈ s.runs)
    (parent : child.parent = some p.invocation) (root : child.id = p.id)
    (procedure : child.procedure.id = target.procedure)
    (revision : child.procedure.revision = target.revision) :
    RunRepresentation contracts s { p with control := { p.control with
      pending := ⟨n.id, child.invocation, target, p.binding, child.input⟩ :: p.control.pending } } := by
  refine { rep with
    pending_nodes_unique := List.nodup_cons.mpr ⟨freshNode, rep.pending_nodes_unique⟩
    pending_children_unique := List.nodup_cons.mpr ⟨freshChild, rep.pending_children_unique⟩
    pending_results_disjoint := ?_
    pending := ?_ }
  · intro waiting hw result hr
    rcases List.mem_cons.mp hw with rfl | hw
    · intro equal
      exact freshResult (List.mem_map.mpr ⟨result, hr, equal.symm⟩)
    · exact rep.pending_results_disjoint waiting hw result hr
  · intro waiting hw
    rcases List.mem_cons.mp hw with rfl | hw
    · exact ⟨n, node, rfl, form, rfl, child, member, rfl, parent, root,
        procedure, revision, rfl⟩
    · exact rep.pending waiting hw

end Kctx.Engineering
