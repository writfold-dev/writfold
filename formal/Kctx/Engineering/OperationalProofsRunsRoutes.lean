import Kctx.Engineering.OperationalProofsRunsFrames

set_option autoImplicit false
namespace Kctx.Engineering

theorem RunRepresentation.failure_routes {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node}
    {catalog : List Procedure} {registry : List PredicateSignature}
    (member : n ∈ r.procedure.nodes) (typed : NodeTyped catalog registry r.procedure n)
    (outcome : Completion) :
    RunRepresentation contracts s { r with
      control := { r.control with
        routes := (failureRoute n outcome).toList.map (fun target => (n.id, target)) ++
          r.control.routes } } := by
  refine { rep with routes := ?_ }
  intro edge present
  rcases List.mem_append.mp present with present | present
  · obtain ⟨target, route, rfl⟩ := List.mem_map.mp present
    have declared : target ∈ n.failure.toList ++ n.cancellation.toList := by
      unfold failureRoute at route
      split at route
      · exact List.mem_append_right _ route
      · exact List.mem_append_left _ route
    exact ⟨n, member, rfl, typed.2.2.2.2.2.2.2.1 target declared, Or.inl declared⟩
  · exact rep.routes edge present

theorem RunRepresentation.select_route {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node} {targets : List NodeId}
    {catalog : List Procedure} {registry : List PredicateSignature} {route : NodeId}
    (member : n ∈ r.procedure.nodes) (typed : NodeTyped catalog registry r.procedure n)
    (form : n.form = .select targets) (chosen : route ∈ targets) :
    RunRepresentation contracts s { r with control := { r.control with
      routes := (n.id, route) :: r.control.routes } } := by
  refine { rep with routes := ?_ }
  intro edge present
  rcases List.mem_cons.mp present with rfl | present
  · have check := typed.2.2.2.2.2.2.2.2.2
    simp only [FormTyped, form] at check
    exact ⟨n, member, rfl, check.2.2 route chosen, Or.inr (by simpa only [form] using chosen)⟩
  · exact rep.routes edge present

end Kctx.Engineering
