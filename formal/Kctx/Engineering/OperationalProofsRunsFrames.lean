import Kctx.Engineering.OperationalProofs
import Kctx.Engineering.OperationalProofsLists

set_option autoImplicit false
namespace Kctx.Engineering

/-- Immutable historical identity, including the delegation captured at call time.
Resume may narrow capabilities and grow floors without rewriting this anchor. -/
structure RunAnchor where
  invocation : ActivationId
  id : RunId
  stream : StreamId
  parent : Option ActivationId
  binding : Binding
  procedure : Procedure
  input : TypedValue
  delegation : Option DelegationSnapshot

def Run.anchor (r : Run) : RunAnchor :=
  ⟨r.invocation, r.id, r.stream, r.parent, r.binding, r.procedure,
    r.input, r.delegation⟩

theorem runs_find_member {s : State} {id : ActivationId} {r : Run}
    (h : findRun s id = some r) : r ∈ s.runs :=
  List.mem_of_find?_eq_some h

theorem runs_find_id {s : State} {id : ActivationId} {r : Run}
    (h : findRun s id = some r) : r.invocation = id := by
  simpa using List.find?_some h

theorem runs_node_member {p : Procedure} {id : NodeId} {n : Node}
    (h : findNode p id = some n) : n ∈ p.nodes :=
  List.mem_of_find?_eq_some h

theorem runs_node_id {p : Procedure} {id : NodeId} {n : Node}
    (h : findNode p id = some n) : n.id = id := by
  simpa using List.find?_some h

theorem runs_subset {α : Type} [BEq α] [LawfulBEq α] {xs ys : List α}
    (h : subset xs ys = true) : ∀ x ∈ xs, x ∈ ys := by
  simpa [subset, List.all_eq_true] using h

theorem runs_retained {s : State} {objects : List ObjectId}
    (h : retained s objects = true) : ∀ object ∈ objects, object ∈ s.retained :=
  runs_subset h

theorem runs_grant {s : State} {r : Run} {g : Grant}
    (h : currentGrant s r = some g) : g ∈ s.authority.grants ∧ g.run = r.id := by
  refine ⟨List.mem_of_find?_eq_some h, ?_⟩
  have checked := List.find?_some h
  simp only [Bool.and_eq_true, beq_iff_eq] at checked
  grind only

theorem runs_grant_scope {s : State} {r : Run} {g : Grant}
    (h : currentGrant s r = some g) :
    g ∈ s.authority.grants ∧ g.run = r.id ∧ g.binding = r.binding ∧
      g.revoked = false ∧ g.validFrom ≤ s.now ∧ s.now < g.expires := by
  have checked := List.find?_some h
  simp only [Bool.and_eq_true, Bool.not_eq_true, beq_iff_eq, decide_eq_true_eq] at checked
  exact ⟨List.mem_of_find?_eq_some h, by grind only⟩

theorem runs_live_floor {s : State} {r : Run} (h : live s r = true) :
    ∀ gate ∈ s.authority.floor, gate ∈ r.floor := by
  have localLive : locallyLive s r = true := by
    unfold live ancestryLive at h
    simp only [Bool.and_eq_true] at h
    exact h.1
  simp only [locallyLive, Bool.and_eq_true] at localLive
  exact runs_subset localLive.1.2

theorem runs_enabled_fresh {s : State} {r : Run} {n : Node}
    (h : enabled s r n = true) :
    n.id ∉ r.control.results.map NodeResult.node ∧
      n.id ∉ r.control.pending.map PendingCall.node := by
  simp only [enabled, Bool.and_eq_true, Bool.not_eq_true] at h
  constructor
  · intro member
    obtain ⟨result, member, same⟩ := List.mem_map.mp member
    have yes : r.control.results.any (·.node == n.id) = true :=
      List.any_eq_true.mpr ⟨result, member, by simpa using same⟩
    grind
  · intro member
    obtain ⟨pending, member, same⟩ := List.mem_map.mp member
    have yes : r.control.pending.any (·.node == n.id) = true :=
      List.any_eq_true.mpr ⟨pending, member, by simpa using same⟩
    grind

theorem runs_nodeInput_retained {contracts : Contracts} {s : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r) (n : Node) :
    ∀ object ∈ (nodeInput r n).objects, object ∈ s.retained := by
  unfold nodeInput
  split
  · exact rep.input_retained
  · intro object member
    obtain ⟨dependency, _, member⟩ := List.mem_flatMap.mp member
    cases found : r.control.results.find? (·.node == dependency.node) with
    | none => simp [found] at member
    | some result =>
      obtain ⟨_, _, _, _, keep⟩ := rep.results result (List.mem_of_find?_eq_some found)
      exact keep object (by simpa [found] using member)

/-- Existing records may move under a map, but every historical anchor survives. -/
theorem runs_anchor_map (xs : List Run) (f : Run → Run)
    (same : ∀ r ∈ xs, (f r).anchor = r.anchor) :
    ∀ r ∈ xs, ∃ next ∈ xs.map f, next.anchor = r.anchor := by
  intro r member
  exact ⟨f r, List.mem_map.mpr ⟨r, member, rfl⟩, same r member⟩

theorem runs_anchor_put {s : State} {old next : Run}
    (unique : (s.runs.map Run.invocation).Nodup) (member : old ∈ s.runs)
    (same : next.anchor = old.anchor) :
    ∀ r ∈ s.runs, ∃ after ∈ (putRun s next).runs, after.anchor = r.anchor := by
  apply runs_anchor_map
  intro r hr
  split
  next selected =>
    have inv : next.invocation = old.invocation := congrArg RunAnchor.invocation same
    have equal : r = old := member_eq_of_unique_key unique hr member
      (by simpa [inv] using selected)
    simpa [equal] using same
  next => rfl

/-- Transport only the ambient history: no post-state well-formedness premise. -/
theorem RunRepresentation.transport {contracts : Contracts} {s t : OperationalState}
    {r : Run} (rep : RunRepresentation contracts s r)
    (catalog : ∀ p ∈ s.catalog, p ∈ t.catalog)
    (realm : t.realm = s.realm)
    (retention : ∀ object ∈ s.retained, object ∈ t.retained)
    (grants : ∀ g ∈ s.authority.grants, ∃ g' ∈ t.authority.grants,
      g'.run = g.run ∧ g'.binding = g.binding ∧
      ∀ cap ∈ g.capabilities, cap ∈ g'.capabilities)
    (accounts : ∀ b ∈ s.budgets, ∃ b' ∈ t.budgets, b'.root = b.root)
    (anchors : ∀ old ∈ s.runs, ∃ next ∈ t.runs, next.anchor = old.anchor) :
    RunRepresentation contracts t r := by
  refine { rep with
    catalog := catalog _ rep.catalog
    realm := rep.realm.trans realm.symm
    input_retained := fun object h => retention object (rep.input_retained object h)
    closure_retained := fun object h => retention object (rep.closure_retained object h)
    grant_bound := ?_
    account := ?_
    loaded := ?_
    results := ?_
    parent := ?_
    call_origin := ?_
    pending := ?_ }
  · obtain ⟨g, member, root, binding, caps⟩ := rep.grant_bound
    obtain ⟨g', member', root', binding', caps'⟩ := grants g member
    exact ⟨g', member', root'.trans root, binding'.trans binding,
      fun cap h => caps' cap (caps cap h)⟩
  · obtain ⟨b, member, root⟩ := rep.account
    obtain ⟨b', member', root'⟩ := accounts b member
    exact ⟨b', member', root'.trans root⟩
  · intro object member
    exact ⟨(rep.loaded object member).1, retention object (rep.loaded object member).2⟩
  · intro result member
    obtain ⟨node, hn, id, typed, keep⟩ := rep.results result member
    exact ⟨node, hn, id, typed, fun object h => retention object (keep object h)⟩
  · intro id edge
    obtain ⟨p, hp, facts⟩ := rep.parent id edge
    obtain ⟨p', hp', equal⟩ := anchors p hp
    refine ⟨p', hp', ?_⟩
    have hi := congrArg RunAnchor.invocation equal
    have hr := congrArg RunAnchor.id equal
    have hs := congrArg RunAnchor.stream equal
    have hb := congrArg RunAnchor.binding equal
    have hp := congrArg RunAnchor.procedure equal
    simp only [Run.anchor] at hi hr hs hb hp
    simpa only [hi, hr, hs, hb, hp] using facts
  · intro id edge
    obtain ⟨p, hp, hi, origin⟩ := rep.call_origin id edge
    obtain ⟨p', hp', equal⟩ := anchors p hp
    have inv := congrArg RunAnchor.invocation equal
    have proc := congrArg RunAnchor.procedure equal
    simp only [Run.anchor] at inv proc
    exact ⟨p', hp', inv.trans hi, by simpa only [proc] using origin⟩
  · intro pending member
    obtain ⟨node, hn, id, form, binding, child, hc, facts⟩ := rep.pending pending member
    obtain ⟨child', hc', equal⟩ := anchors child hc
    refine ⟨node, hn, id, form, binding, child', hc', ?_⟩
    have hi := congrArg RunAnchor.invocation equal
    have hp := congrArg RunAnchor.parent equal
    have hr := congrArg RunAnchor.id equal
    have hc := congrArg RunAnchor.procedure equal
    have hv := congrArg RunAnchor.input equal
    simp only [Run.anchor] at hi hp hr hc hv
    simpa only [hi, hp, hr, hc, hv] using facts

end Kctx.Engineering
