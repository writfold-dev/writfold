import Kctx.Engineering.OperationalProofsRunsCalls

set_option autoImplicit false
namespace Kctx.Engineering

theorem runs_cons_represented {contracts : Contracts} {s : OperationalState}
    {xs : List Run} {r : Run} (new : RunRepresentation contracts s r)
    (old : ∀ r ∈ xs, RunRepresentation contracts s r) :
    ∀ value ∈ r :: xs, RunRepresentation contracts s value := by
  intro value member
  rcases List.mem_cons.mp member with rfl | member
  · exact new
  · exact old value member

theorem runs_replace_represented {contracts : Contracts} {s : OperationalState}
    {xs : List Run} {next : Run}
    (old : ∀ r ∈ xs, RunRepresentation contracts s r)
    (new : RunRepresentation contracts s next) :
    ∀ r ∈ xs.map (fun r => if r.invocation == next.invocation then next else r),
      RunRepresentation contracts s r := by
  intro r member
  obtain ⟨previous, sourceMember, rfl⟩ := List.mem_map.mp member
  split
  · exact new
  · exact old previous sourceMember

theorem runs_map_represented {contracts : Contracts} {s : OperationalState}
    {xs : List Run} {f : Run → Run}
    (old : ∀ r ∈ xs, RunRepresentation contracts s r)
    (anchor : ∀ r ∈ xs, (f r).anchor = r.anchor)
    (loaded : ∀ r ∈ xs, (f r).loaded = r.loaded)
    (results : ∀ r ∈ xs, (f r).control.results = r.control.results)
    (routes : ∀ r ∈ xs, (f r).control.routes = r.control.routes)
    (pending : ∀ r ∈ xs, (f r).control.pending = r.control.pending)
    (caps : ∀ r ∈ xs, (f r).capabilities = r.capabilities)
    (floor : ∀ r ∈ xs, (f r).floor = r.floor) :
    ∀ r ∈ xs.map f, RunRepresentation contracts s r := by
  intro r member
  obtain ⟨previous, sourceMember, rfl⟩ := List.mem_map.mp member
  exact (old previous sourceMember).congr_run (anchor previous sourceMember) (loaded previous sourceMember)
    (results previous sourceMember) (routes previous sourceMember) (pending previous sourceMember)
    (caps previous sourceMember) (floor previous sourceMember)

theorem RunRepresentation.status {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) (status : RunStatus) :
    RunRepresentation contracts s { r with status := status } :=
  rep.congr_run rfl rfl rfl rfl rfl rfl rfl

theorem RunRepresentation.design {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) (design : Option DesignProposal) :
    RunRepresentation contracts s { r with design := design } :=
  rep.congr_run rfl rfl rfl rfl rfl rfl rfl


end Kctx.Engineering
