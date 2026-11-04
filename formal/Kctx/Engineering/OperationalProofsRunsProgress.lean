import Kctx.Engineering.OperationalProofsRunsAmbient
import Kctx.Engineering.OperationalProofsRunsRoutes

set_option autoImplicit false
namespace Kctx.Engineering

/-- Unobserved mutable fields do not change historical representation. -/
theorem RunRepresentation.congr_run {contracts : Contracts} {s : OperationalState}
    {r next : Run} (rep : RunRepresentation contracts s r)
    (anchor : next.anchor = r.anchor) (loaded : next.loaded = r.loaded)
    (results : next.control.results = r.control.results)
    (routes : next.control.routes = r.control.routes)
    (pending : next.control.pending = r.control.pending)
    (ha : next.capabilities = r.capabilities) (hf : next.floor = r.floor) :
    RunRepresentation contracts s next := by
  have hi := congrArg RunAnchor.invocation anchor
  have hr := congrArg RunAnchor.id anchor
  have hs := congrArg RunAnchor.stream anchor
  have hp := congrArg RunAnchor.parent anchor
  have hb := congrArg RunAnchor.binding anchor
  have hc := congrArg RunAnchor.procedure anchor
  have hv := congrArg RunAnchor.input anchor
  have hd := congrArg RunAnchor.delegation anchor
  simp only [Run.anchor] at hi hr hs hp hb hc hv hd
  rcases rep with ⟨catalog, realm, inputType, inputRetained, closure, capabilities,
    grant, procedureFloor, kernelFloor, account, load, unique, typed, routeProof, parent,
    rootDelegation, origin, pendingNodes, pendingChildren, disjoint, waiting⟩
  constructor <;> simp_all only [hi, hr, hs, hp, hb, hc, hv, hd, ha, hf, loaded, results, routes, pending]
  all_goals first | assumption | simp

theorem RunRepresentation.load {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {object : ObjectId}
    (closure : object ∈ r.procedure.closure) (retained : object ∈ s.retained) :
    RunRepresentation contracts s { r with loaded := object :: r.loaded } := by
  refine { rep with loaded := ?_ }
  intro value member
  rcases List.mem_cons.mp member with rfl | member
  · exact ⟨closure, retained⟩
  · exact rep.loaded value member

/-- A completion preserves disjointness because enabled nodes are neither
completed nor waiting on a child. -/
theorem RunRepresentation.record {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node} {v : TypedValue}
    {outcome : Completion} (member : n ∈ r.procedure.nodes)
    (fresh : n.id ∉ r.control.results.map NodeResult.node)
    (notPending : n.id ∉ r.control.pending.map PendingCall.node)
    (typed : outcome = .success → v.type = n.output)
    (retained : ∀ object ∈ v.objects, object ∈ s.retained) :
    RunRepresentation contracts s (recordResult r n v outcome) := by
  refine { rep with
    results_unique := List.nodup_cons.mpr ⟨fresh, rep.results_unique⟩
    results := ?_
    pending_results_disjoint := ?_ }
  · intro result present
    rcases List.mem_cons.mp present with rfl | present
    · exact ⟨n, member, rfl, typed, retained⟩
    · exact rep.results result present
  · intro pending hp result hr
    rcases List.mem_cons.mp hr with rfl | hr
    · intro equal
      exact notPending (List.mem_map.mpr ⟨pending, hp, equal⟩)
    · exact rep.pending_results_disjoint pending hp result hr

/-- Returning consumes precisely its waiting entry before adding its result. -/
theorem RunRepresentation.return_result {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node} {v : TypedValue}
    {outcome : Completion} {waiting : PendingCall}
    (member : n ∈ r.procedure.nodes) (present : waiting ∈ r.control.pending)
    (node : n.id = waiting.node)
    (typed : outcome = .success → v.type = n.output)
    (retained : ∀ object ∈ v.objects, object ∈ s.retained) :
    RunRepresentation contracts s
      { (recordResult r n v outcome) with control :=
        { (recordResult r n v outcome).control with
          pending := r.control.pending.filter (·.child != waiting.child) } } := by
  have fresh : n.id ∉ r.control.results.map NodeResult.node := by
    intro member
    obtain ⟨result, hr, equal⟩ := List.mem_map.mp member
    exact rep.pending_results_disjoint waiting present result hr (node.symm.trans equal.symm)
  refine { rep with
    results_unique := List.nodup_cons.mpr ⟨fresh, rep.results_unique⟩
    results := ?_
    pending_nodes_unique := ((List.filter_sublist).map PendingCall.node).nodup rep.pending_nodes_unique
    pending_children_unique := ((List.filter_sublist).map PendingCall.child).nodup rep.pending_children_unique
    pending_results_disjoint := ?_
    pending := ?_ }
  · intro result hr
    rcases List.mem_cons.mp hr with rfl | hr
    · exact ⟨n, member, rfl, typed, retained⟩
    · exact rep.results result hr
  · intro pending hp result hr
    have inOld := (List.mem_filter.mp hp).1
    rcases List.mem_cons.mp hr with rfl | hr
    · intro equal
      have same : pending = waiting := member_eq_of_unique_key rep.pending_nodes_unique
        inOld present (equal.trans node)
      simpa [same] using (List.mem_filter.mp hp).2
    · exact rep.pending_results_disjoint pending inOld result hr
  · intro pending hp
    exact rep.pending pending (List.mem_filter.mp hp).1

theorem runs_typed_call_output {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {r : Run} (member : r ∈ s.runs)
    {n : Node} (hn : n ∈ r.procedure.nodes) {target : CallTarget}
    (form : n.form = .call target) : n.output = target.output := by
  have typed := (wf.catalog_authority r.procedure (wf.runs r member).catalog).2.2.1
  have node := typed.2.2.2.2.2 n hn
  have call := node.2.2.2.2.2.2.2.2.2
  simp only [FormTyped, form] at call
  obtain ⟨_, _, _, _, _, _, _, output, _, _⟩ := call
  exact output

theorem RunRepresentation.failure {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node} {receipt : Receipt}
    (member : n ∈ r.procedure.nodes)
    (typed : NodeTyped s.catalog contracts.predicates r.procedure n)
    (fresh : n.id ∉ r.control.results.map NodeResult.node)
    (notPending : n.id ∉ r.control.pending.map PendingCall.node)
    (failed : receipt.outcome ≠ .success)
    (retained : ∀ object ∈ receipt.output.objects, object ∈ s.retained) :
    RunRepresentation contracts s (recordFailure r n receipt) := by
  exact (rep.record member fresh notPending (fun h => False.elim (failed h)) retained).failure_routes
    member typed receipt.outcome

theorem RunRepresentation.return_failure {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) {n : Node} {receipt : Receipt}
    {waiting : PendingCall}
    (member : n ∈ r.procedure.nodes) (present : waiting ∈ r.control.pending)
    (typed : NodeTyped s.catalog contracts.predicates r.procedure n)
    (node : n.id = waiting.node)
    (failed : receipt.outcome ≠ .success)
    (retained : ∀ object ∈ receipt.output.objects, object ∈ s.retained) :
    RunRepresentation contracts s
      { (recordFailure r n receipt) with control :=
        { (recordFailure r n receipt).control with
          pending := r.control.pending.filter (·.child != waiting.child) } } := by
  exact (rep.return_result member present node
    (fun h => False.elim (failed h)) retained).failure_routes member typed receipt.outcome

end Kctx.Engineering
