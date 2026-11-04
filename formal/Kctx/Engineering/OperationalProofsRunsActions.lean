import Kctx.Engineering.OperationalProofsRunsLists

set_option autoImplicit false
namespace Kctx.Engineering

/-- Loading changes only the selected run's retained closure inventory. -/
theorem loadResource_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation : ActivationId} {object : ObjectId}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : loadResource s activation object = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have action : domainBody contracts s (.loadResource activation object) = some t := h
  have before := domainBody_transports_old_run_representation wf action
  have storage := domainBody_run_storage action
  unfold loadResource at h
  obtain ⟨r, found, success⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨guard, success⟩ := Option.ite_none_right_eq_some.mp success
  simp only [Bool.and_eq_true, List.contains_iff_mem] at guard
  have next := (before r (runs_find_member found)).load guard.1.1.2
    (storage.2.1 object guard.1.2)
  operational_success success
  exact runs_replace_represented before next

/-- Every successful advance records the selected node; only a root end also
changes status. Effect nodes retain their already charged state. -/
theorem advanceNode_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation : ActivationId} {id : NodeId} {value : TypedValue}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : advanceNode contracts s activation id value = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have action : domainBody contracts s (.advanceNode activation id value) = some t := h
  have before := domainBody_transports_old_run_representation wf action
  have storage := domainBody_run_storage action
  unfold advanceNode at h
  obtain ⟨r, found, success⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, success⟩ := Option.bind_eq_some_iff.mp success
  obtain ⟨guard, success⟩ := Option.ite_none_right_eq_some.mp success
  simp only [Bool.and_eq_true, beq_iff_eq] at guard
  have ready : enabled s r n = true := by
    have checked := guard.1.1.1
    simp only [nodeReady, Bool.and_eq_true] at checked
    exact checked.1.1
  have fresh := runs_enabled_fresh ready
  have next := (before r (runs_find_member found)).record (outcome := .success)
    (runs_node_member nodeFound) fresh.1 fresh.2 (fun _ => guard.1.1.2)
    (fun object member => storage.2.1 object (runs_retained guard.1.2 object member))
  operational_success success
  all_goals
    apply runs_replace_represented before
    first
    | exact next
    | exact next.status .completed

/-- Authenticated non-success records retain their output and declared failure
route, without treating that output as a successful typed result. -/
theorem failNode_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation : ActivationId} {id : NodeId} {receipt : Receipt}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : failNode contracts s activation id receipt = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have action : domainBody contracts s (.failNode activation id receipt) = some t := h
  have before := domainBody_transports_old_run_representation wf action
  have storage := domainBody_run_storage action
  unfold failNode at h
  obtain ⟨r, found, success⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, success⟩ := Option.bind_eq_some_iff.mp success
  obtain ⟨guard, success⟩ := Option.ite_none_right_eq_some.mp success
  have member := runs_node_member nodeFound
  have typed := (wf.catalog_authority r.procedure
    (wf.runs r (runs_find_member found)).catalog).2.2.1.2.2.2.2.2 n member
  simp only [Bool.and_eq_true, beq_iff_eq] at guard
  have ready : enabled s r n = true := by exact guard.1.1.1.1.1.1.1.1
  have failed : receipt.outcome ≠ .success := by simpa using guard.1.1.1.1.1.2
  have kept : retained s receipt.output.objects = true := guard.1.1.1.1.2
  have fresh := runs_enabled_fresh ready
  operational_success success
  apply runs_replace_represented before
  exact (before r (runs_find_member found)).failure member typed fresh.1 fresh.2 failed
    (fun object present => storage.2.1 object (runs_retained kept object present))

/-- Selection records a typed result and precisely the route named by the
selected node's catalog declaration. -/
theorem selectRoute_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation : ActivationId} {id route : NodeId} {value : TypedValue}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : selectRoute contracts s activation id route value = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have action : domainBody contracts s (.selectRoute activation id route value) = some t := h
  have before := domainBody_transports_old_run_representation wf action
  have storage := domainBody_run_storage action
  unfold selectRoute at h
  obtain ⟨r, found, success⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, success⟩ := Option.bind_eq_some_iff.mp success
  have member := runs_node_member nodeFound
  have nodeId := runs_node_id nodeFound
  have typed := (wf.catalog_authority r.procedure
    (wf.runs r (runs_find_member found)).catalog).2.2.1.2.2.2.2.2 n member
  cases form : n.form <;> simp only [form] at success
  all_goals first | contradiction | skip
  obtain ⟨guard, success⟩ := Option.ite_none_right_eq_some.mp success
  simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem] at guard
  have ready : enabled s r n = true := by
    have checked := guard.1.1.1.1
    simp only [nodeReady, Bool.and_eq_true] at checked
    exact checked.1.1
  have fresh := runs_enabled_fresh ready
  have next := ((before r (runs_find_member found)).record (outcome := .success) member fresh.1 fresh.2
    (fun _ => guard.1.1.2)
    (fun object present => storage.2.1 object (runs_retained guard.1.2 object present))).select_route
      member typed form guard.1.1.1.2
  operational_success success
  apply runs_replace_represented before
  simpa only [nodeId] using next

end Kctx.Engineering
