import Kctx.Engineering.OperationalProofsRunsLists
import Kctx.Engineering.OperationalProofsState

set_option autoImplicit false
set_option maxRecDepth 8192
namespace Kctx.Engineering

/-- Resumption preserves the historical delegation witness: filtering authority
can only strengthen its capability bound, and inherited floors keep its gates.
Control, call identity and the original snapshot are not reconstructed. -/
theorem RunRepresentation.resume {contracts : Contracts} {t : OperationalState}
    {r : Run} (rep : RunRepresentation contracts t r) (s : State) (g : Grant) :
    RunRepresentation contracts t { r with
      incarnation := s.incarnation
      capabilities := r.capabilities.filter (fun cap =>
        g.capabilities.contains cap && r.procedure.ceiling.contains cap &&
          (((r.parent.bind (findRun s)).map Run.capabilities).getD r.capabilities).contains cap)
      floor := inheritedFloor r.floor r.procedure.floor
        (s.authority.floor ++ (((r.parent.bind (findRun s)).map Run.floor).getD []))
      correctionGeneration := s.authority.correctionGeneration
      policy := s.authority.policy
      loaded := [] } := by
  refine { rep with
    capabilities := ?_
    grant_bound := ?_
    procedure_floor := ?_
    kernel_floor := ?_
    loaded := by simp
    parent := ?_
    call_origin := ?_ }
  · intro cap member
    exact rep.capabilities cap (List.mem_filter.mp member).1
  · obtain ⟨grant, member, root, binding, bound⟩ := rep.grant_bound
    exact ⟨grant, member, root, binding,
      fun cap present => bound cap (List.mem_filter.mp present).1⟩
  · intro gate member
    simp [inheritedFloor, member]
  · intro gate member
    have keep := rep.kernel_floor gate member
    simp [inheritedFloor, keep]
  · intro id edge
    obtain ⟨parent, member, invocation, root, stream, binding, rank,
      delegation, snapshot, owner, caps, floor⟩ := rep.parent id edge
    refine ⟨parent, member, invocation, root, stream, binding, rank,
      delegation, snapshot, owner, ?_, ?_⟩
    · intro cap present
      exact caps cap (List.mem_filter.mp present).1
    · intro gate present
      have keep := floor gate present
      simp [inheritedFloor, keep]
  · intro id edge
    obtain ⟨parent, member, invocation, node, nodeMember, target, form,
      procedure, revision, input, output, bounds⟩ := rep.call_origin id edge
    exact ⟨parent, member, invocation, node, nodeMember, target, form,
      procedure, revision, input, output,
      fun cap present => bounds cap (List.mem_filter.mp present).1⟩

/-- The actual resume reducer charges the existing root and replaces only the
selected activation; all other run records retain their historical meaning. -/
theorem resume_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation : ActivationId} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : resume contracts s activation = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have before := domainBody_transports_old_run_representation wf
    (show domainBody contracts s (.resume activation) = some t from h)
  unfold resume at h
  obtain ⟨r, found, success⟩ := Option.bind_eq_some_iff.mp h
  clear h
  obtain ⟨g, granted, guarded⟩ := Option.bind_eq_some_iff.mp success
  clear success
  obtain ⟨checked, debiting⟩ := Option.ite_none_right_eq_some.mp guarded
  clear guarded
  obtain ⟨charged, debited, result⟩ := Option.bind_eq_some_iff.mp debiting
  clear debiting
  have chargedState := charge_exact_state debited
  subst charged
  cases result
  apply runs_replace_represented before
  exact (before r (runs_find_member found)).resume s g

end Kctx.Engineering
