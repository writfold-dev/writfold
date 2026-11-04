import Kctx.Engineering.OperationalProofsRunsLists
import Kctx.Engineering.OperationalProofsRunsRoots
import Kctx.Engineering.OperationalProofsRunsResume
import Kctx.Engineering.OperationalProofsRunsActions
import Kctx.Engineering.OperationalProofsRunsComposition
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 16384
set_option maxHeartbeats 6400000
namespace Kctx.Engineering


/-- The actual reducer preserves historical run representation. The ambient
transport is established separately from local progress, so no live-policy
premise is imposed on completed, cancelled or retired historical records. -/
theorem domainBody_preserves_run_representation {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ run ∈ t.runs, RunRepresentation contracts t.toOperationalState run := by
  have before := domainBody_transports_old_run_representation wf h
  cases action with
  | activate run budget => exact activate_preserves_run_representation wf h
  | rebind activation procedure binding invocation => exact rebind_preserves_run_representation wf h
  | resume activation => exact resume_preserves_run_representation wf h
  | loadResource activation object => exact loadResource_preserves_run_representation wf h
  | advanceNode activation node value => exact advanceNode_preserves_run_representation wf h
  | failNode activation node receipt => exact failNode_preserves_run_representation wf h
  | selectRoute activation node route value => exact selectRoute_preserves_run_representation wf h
  | call parent node child callee binding => exact call_preserves_run_representation wf h
  | returnChild parent child receipt => exact returnChild_preserves_run_representation wf h
  | reopen activation command =>
      simp only [domainBody, reopen] at h
      obtain ⟨run, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      operational_success continuation
      all_goals
        apply runs_replace_represented before
        exact (before run (runs_find_member found)).design _
  | retire activation =>
      simp only [domainBody, retire] at h
      obtain ⟨run, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      operational_success continuation
      all_goals
        apply runs_replace_represented before
        exact (before run (runs_find_member found)).status .retired
  | cancel activation =>
      simp only [domainBody, cancel] at h
      operational_success h
      all_goals
        apply runs_map_represented before <;> intro run member <;> split <;> rfl
  | _ =>
      simp only [domainBody, resolve, prepare, reserve, claimDispatch, issueEffect, observeEffect,
        observeMaterial, reconcile, reuse, retryClaim, quiesce, propose, evaluate, review,
        prepareAdmission, publishAdmission, acknowledgeAdmission] at h
      operational_success h
      all_goals exact before

theorem domain_preserves_run_representation {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) :
    ∀ run ∈ t.runs, RunRepresentation contracts t.toOperationalState run := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
      simp only [body, Option.map_some, Option.some.injEq] at h
      subst t
      have represented := domainBody_preserves_run_representation wf body
      have frame := domainBody_run_frame body
      intro r member
      apply (represented r member).transport (t := (frameDomain s action next).toOperationalState)
        (fun _ h => h) frame.1.symm (fun _ h => h) _
        (fun b h => ⟨b, h, rfl⟩) (fun old h => ⟨old, h, rfl⟩)
      intro g hg
      exact ⟨g, by simpa only [frame.2] using hg, rfl, rfl, fun _ h => h⟩

end Kctx.Engineering
