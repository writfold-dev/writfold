import Kctx.Engineering.OperationalProofsRunsFrames
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Replacement of a unique activation changes only its mutable progress. -/
theorem runs_anchor_replace {xs : List Run} {old next : Run}
    (unique : (xs.map Run.invocation).Nodup) (member : old ∈ xs)
    (same : next.anchor = old.anchor) :
    (xs.map (fun r => if r.invocation == next.invocation then next else r)).map Run.anchor =
      xs.map Run.anchor := by
  simp only [List.map_map]
  apply List.map_congr_left
  intro r hr
  dsimp
  split
  next selected =>
    have hi := congrArg RunAnchor.invocation same
    simp only [Run.anchor] at hi
    have equal := member_eq_of_unique_key unique hr member
      (show r.invocation = old.invocation by simpa [hi] using selected)
    simpa only [equal] using same
  next => rfl

theorem runs_anchor_key_unique {xs ys : List Run}
    (unique : (xs.map Run.invocation).Nodup)
    (same : ys.map Run.anchor = xs.map Run.anchor) :
    (ys.map Run.invocation).Nodup := by
  have keys : ys.map Run.invocation = xs.map Run.invocation := by
    simpa only [List.map_map, Function.comp_def, Run.anchor] using
      congrArg (List.map RunAnchor.invocation) same
  rw [keys]
  exact unique

theorem runs_anchor_list_member {xs ys : List Run}
    (same : xs.map Run.anchor = ys.map Run.anchor) :
    ∀ r ∈ ys, ∃ next ∈ xs, next.anchor = r.anchor := by
  intro r member
  have h : r.anchor ∈ xs.map Run.anchor := by
    rw [same]
    exact List.mem_map.mpr ⟨r, member, rfl⟩
  exact List.mem_map.mp h

theorem runs_anchor_replace_after {xs ys : List Run} {old next : Run}
    (unique : (xs.map Run.invocation).Nodup) (member : old ∈ xs)
    (before : ys.map Run.anchor = xs.map Run.anchor)
    (same : next.anchor = old.anchor) :
    (ys.map (fun r => if r.invocation == next.invocation then next else r)).map Run.anchor =
      xs.map Run.anchor := by
  obtain ⟨old', member', equal⟩ := runs_anchor_list_member before old member
  exact (runs_anchor_replace (runs_anchor_key_unique unique before) member'
    (same.trans equal.symm)).trans before

/-- Charge alters spent amounts, not historical root-account identities. -/
theorem runs_charge_accounts {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) :
    ∀ b ∈ s.budgets, ∃ b' ∈ t.budgets, b'.root = b.root := by
  obtain ⟨_, _, _, _, budgets⟩ := charge_has_bounded_positive_cost h
  intro b member
  rw [budgets]
  refine ⟨if b.root == root then { b with spent := b.spent + cost } else b,
    List.mem_map.mpr ⟨b, member, rfl⟩, ?_⟩
  split <;> rfl

/-- Adding an activation does not discard the anchor-preserving updated tail. -/
private theorem runs_anchor_cons_member {xs ys : List Run} {added : Run}
    (same : ys.map Run.anchor = xs.map Run.anchor) :
    ∀ r ∈ xs, ∃ next ∈ added :: ys, next.anchor = r.anchor := by
  intro r member
  obtain ⟨next, present, anchor⟩ := runs_anchor_list_member same r member
  exact ⟨next, List.mem_cons_of_mem _ present, anchor⟩

/-- Every old activation remains present under its immutable historical anchor.
Single replacements preserve the anchor list; return composes two such replacements.
Call and rebind retain that list beneath their new activation, whereas cancellation
maps status changes pointwise. No current-live fields enter the historical anchor. -/
theorem domainBody_preserves_run_anchors {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ r ∈ s.runs, ∃ next ∈ t.runs, next.anchor = r.anchor := by
  have unique := wf.activations_unique
  cases action with
  | activate run budget =>
      simp only [domainBody, activate] at h
      operational_success h
      intro r member
      exact ⟨r, List.mem_cons_of_mem _ member, rfl⟩
  | call parent node invocation callee binding =>
      simp only [domainBody, call] at h
      obtain ⟨p, parentFound, h⟩ := Option.bind_eq_some_iff.mp h
      operational_success h
      apply runs_anchor_cons_member
      exact runs_anchor_replace unique (runs_find_member parentFound) rfl
  | rebind activation procedure binding invocation =>
      simp only [domainBody, rebind] at h
      obtain ⟨r, runFound, h⟩ := Option.bind_eq_some_iff.mp h
      operational_success h
      apply runs_anchor_cons_member
      exact runs_anchor_replace unique (runs_find_member runFound) rfl
  | returnChild parent child receipt =>
      simp only [domainBody, returnChild] at h
      obtain ⟨p, parentFound, h⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨c, childFound, h⟩ := Option.bind_eq_some_iff.mp h
      operational_success h
      all_goals
        apply runs_anchor_list_member
        apply runs_anchor_replace_after unique (runs_find_member parentFound)
        · exact runs_anchor_replace unique (runs_find_member childFound)
            (next := { c with status := .completed }) rfl
        · rfl
  | cancel activation =>
      simp only [domainBody, cancel] at h
      operational_success h
      apply runs_anchor_map
      intro r _
      split <;> rfl
  | _ =>
      simp only [domainBody, resolve, loadResource, advanceNode, failNode,
        selectRoute, prepare, reserve, claimDispatch, issueEffect, observeEffect,
        observeMaterial, reconcile, reuse, resume, retryClaim, reopen, quiesce,
        propose, evaluate, review, prepareAdmission, publishAdmission,
        acknowledgeAdmission, retire] at h
      operational_success h
      -- Reducers not touching runs retain the very same member.
      all_goals try exact fun r member => ⟨r, member, rfl⟩
      -- Every remaining reducer replaces its selected run, changing only
      -- progress, status, loading, or revalidated live authority.
      all_goals
        apply runs_anchor_list_member
        apply runs_anchor_replace unique
        · apply runs_find_member (s := s)
          assumption
        · first | rfl | (split <;> rfl)

theorem domain_preserves_run_anchors {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) :
    ∀ r ∈ s.runs, ∃ next ∈ t.runs, next.anchor = r.anchor := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
      simp only [body, Option.map_some, Option.some.injEq] at h
      subst t
      exact domainBody_preserves_run_anchors (t := next) wf body

end Kctx.Engineering
