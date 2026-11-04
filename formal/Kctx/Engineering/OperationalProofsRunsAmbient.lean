import Kctx.Engineering.OperationalProofsRunsHistory
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Domain storage is monotone for the three histories read by run representation. -/
theorem domainBody_run_storage {contracts : Contracts} {s t : State} {action : Action}
    (h : domainBody contracts s action = some t) :
    (∀ p ∈ s.catalog, p ∈ t.catalog) ∧
    (∀ object ∈ s.retained, object ∈ t.retained) ∧
    (∀ b ∈ s.budgets, ∃ b' ∈ t.budgets, b'.root = b.root) := by
  cases action <;>
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, prepare, reserve, claimDispatch, issueEffect, observeEffect,
      observeMaterial, reconcile, reuse, returnChild, rebind, resume, retryClaim, reopen, cancel, quiesce,
      propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
      retire] at h
  all_goals
    operational_success h
  all_goals
    try simp only [putRun, putEffect]
    refine ⟨?_, ?_, ?_⟩
  all_goals
    first
      | exact fun _ member => member
      | exact fun _ member => List.mem_cons_of_mem _ member
      | exact fun _ member => List.mem_append_right _ member
      | exact fun b member => ⟨b, member, rfl⟩
      | exact fun b member => ⟨b, List.mem_cons_of_mem _ member, rfl⟩
      | (intro b member
         refine ⟨_, List.mem_map.mpr ⟨b, member, rfl⟩, ?_⟩
         split <;> rfl)

theorem domainBody_run_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : domainBody contracts s action = some t) :
    t.realm = s.realm ∧ t.authority = s.authority := by
  cases action <;>
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, prepare, reserve, claimDispatch, issueEffect, observeEffect,
      observeMaterial, reconcile, reuse, returnChild, rebind, resume, retryClaim, reopen, cancel, quiesce,
      propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
      retire] at h
  all_goals
    operational_success h
  all_goals
    exact ⟨rfl, rfl⟩

theorem domainBody_transports_old_run_representation {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ r ∈ s.runs, RunRepresentation contracts t.toOperationalState r := by
  obtain ⟨catalog, retained, accounts⟩ := domainBody_run_storage h
  obtain ⟨realm, authority⟩ := domainBody_run_frame h
  intro r member
  apply (wf.runs r member).transport catalog realm retained _ accounts
    (domainBody_preserves_run_anchors wf h)
  intro g member
  exact ⟨g, by simpa only [authority] using member, rfl, rfl, fun _ h => h⟩

theorem domain_transports_old_run_representation {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) :
    ∀ r ∈ s.runs, RunRepresentation contracts t.toOperationalState r := by
  have anchors := domain_preserves_run_anchors wf h
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
      simp only [body, Option.map_some, Option.some.injEq] at h
      subst t
      obtain ⟨catalog, retained, accounts⟩ := domainBody_run_storage body
      intro r member
      exact (wf.runs r member).transport catalog rfl retained
        (fun g hg => ⟨g, hg, rfl, rfl, fun _ h => h⟩) accounts anchors

end Kctx.Engineering
