import Kctx.Engineering.EffectsSafetyOnce
import Kctx.Engineering.ContinuationSafetyRetry
import Kctx.Engineering.ContinuationSafetyResume
import Kctx.Engineering.EffectsSafetyApplicationsTrace

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

private theorem closed_claim_initial_claim {contracts : Contracts} {s t : State} {id : Id .claim}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (before : ClaimClosed id s.toOperationalState)
    (h : claimDispatch contracts s effect claim expected record = some t) : ClaimClosed id t.toOperationalState := by
  simp only [claimDispatch] at h
  operational_success h
  all_goals
    apply claim_closed_fresh_append before
    simp_all only [Bool.and_eq_true, Bool.not_eq_true']

private theorem closed_claim_retry {contracts : Contracts} {s t : State} {id : Id .claim}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (before : ClaimClosed id s.toOperationalState)
    (h : retryClaim contracts s effect claim expected record = some t) : ClaimClosed id t.toOperationalState := by
  simp only [retryClaim] at h
  operational_success h
  all_goals
    apply claim_closed_fresh_append before
    simp_all only [Bool.and_eq_true, Bool.not_eq_true']

private theorem closed_claim_issue {contracts : Contracts} {s t : State} {id : Id .claim}
    {effect : Id .effect} (before : ClaimClosed id s.toOperationalState)
    (h : issueEffect contracts s effect = some t) : ClaimClosed id t.toOperationalState := by
  simp only [issueEffect] at h
  operational_success h
  all_goals exact claim_closed_mark before

private theorem passive_claim_inventory_frame {contracts : Contracts} {s t : State} {action : Action}
    (passive : (match action with
      | .claimDispatch _ _ _ _ | .retryClaim _ _ _ _ | .issueEffect _ => false
      | _ => true) = true)
    (h : domainBody contracts s action = some t) : t.claims = s.claims := by
  cases action <;> simp only [Bool.false_eq_true] at passive <;> try contradiction
  all_goals simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
    selectRoute, call, prepare, reserve, observeEffect, observeMaterial, reconcile,
    reuse, returnChild, rebind, resume, reopen, cancel, quiesce, propose, evaluate, review,
    prepareAdmission, publishAdmission, acknowledgeAdmission, retire, putRun,
    putEffect] at h
  all_goals operational_success h
  all_goals rfl

theorem E07_domain_preserves_issued {contracts : Contracts} {s t : State} {id : Id .claim}
    {action : Action} (before : ClaimClosed id s.toOperationalState)
    (h : domain contracts s action = some t) : ClaimClosed id t.toOperationalState := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next =>
      have preserved : ClaimClosed id next.toOperationalState := by
        cases action with
        | claimDispatch effect claim expected record => exact closed_claim_initial_claim before body
        | retryClaim effect claim expected record => exact closed_claim_retry before body
        | issueEffect effect => exact closed_claim_issue before body
        | _ => simpa only [ClaimClosed, passive_claim_inventory_frame (by rfl) body] using before
      simp only [domain, body, Option.map_some, Option.some.injEq] at h
      subst t
      exact preserved

theorem E07_local_preserves_issued {contracts : Contracts} {s t : State} {id : Id .claim}
    {action : Action} (before : ClaimClosed id s.toOperationalState)
    (h : localAction contracts s action = some t) : ClaimClosed id t.toOperationalState := by
  cases action <;> simp only [localAction, project, acquireCut, releaseReadCut,
    crash, recover] at h
  all_goals operational_success h
  all_goals exact before

theorem E07_environment_preserves_issued {contracts : Contracts} {s t : State} {id : Id .claim}
    {action : Environment} (before : ClaimClosed id s.toOperationalState)
    (h : environment contracts s action = some t) : ClaimClosed id t.toOperationalState := by
  cases body : environmentBody contracts s action with
  | none => simp [environment, body] at h
  | some next =>
      have preserved : ClaimClosed id next.toOperationalState := by
        cases action <;> simp only [environmentBody, completeExternalEffect, responseWorld, putEffect] at body
        all_goals operational_success body
        all_goals exact before
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact preserved

theorem E07_issued_persists_reachable {contracts : Contracts} {initial s : State} {id : Id .claim}
    (before : ClaimClosed id initial.toOperationalState) (h : Reachable contracts initial s) :
    ClaimClosed id s.toOperationalState :=
  reachable_operational_property E07_domain_preserves_issued E07_local_preserves_issued
    E07_environment_preserves_issued before h

theorem E07_dispatch_marks_issued {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : Id .effect}
    (h : step contracts s (.dispatch stream expected id) = some t) :
    ∃ checked : DispatchChecked contracts s id, ClaimClosed checked.dispatch.id t.toOperationalState := by
  simp only [step, dispatchSelected] at h
  split at h
  · cases body : domainBody contracts s (.issueEffect id) with
    | none => simp [domain, body] at h
    | some next =>
        obtain ⟨checked, marked⟩ := E07_issue_marks_issued body
        simp only [domain, body, Option.map_some, Option.some.injEq] at h
        subst t
        exact ⟨checked, marked⟩
  · contradiction

/-- At-most-once is per persisted claim, not per logical EffectId: an explicitly
qualified retry may create a fresh claim. No trace can clear the old issued bit. -/
theorem E07_no_second_send_after_trace {contracts : Contracts} {before sent current after : State}
    {stream nextStream : StreamId} {expected nextExpected : Option Ledger.Head}
    {effect nextEffect : Id .effect}
    (first : step contracts before (.dispatch stream expected effect) = some sent)
    (trace : Reachable contracts sent current)
    (second : step contracts current (.dispatch nextStream nextExpected nextEffect) = some after) :
    ∃ original : DispatchChecked contracts before effect,
      ∃ subsequent : DispatchChecked contracts current nextEffect,
        original.dispatch.id ≠ subsequent.dispatch.id := by
  obtain ⟨original, marked⟩ := E07_dispatch_marks_issued first
  obtain ⟨subsequent⟩ := E07_dispatch_request_checks_selected_claim second
  have persisted := E07_issued_persists_reachable marked trace
  exact ⟨original, subsequent, Ne.symm (E07_closed_claim_cannot_issue persisted subsequent)⟩

end Kctx.Engineering
