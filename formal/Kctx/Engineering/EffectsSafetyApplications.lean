import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- The selected first non-rejected logical application is immutable. Rejected
attempt observations are deliberately not entries in this canonical collection. -/
def ApplicationRecorded (application : LogicalApplication) (s : OperationalState) : Prop :=
  s.world.applications.find? (·.intent.effect == application.intent.effect) = some application

theorem E07_completion_preserves_application {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {producer consumer : Nat} {application : LogicalApplication}
    (before : ApplicationRecorded application s.toOperationalState)
    (h : completeExternalEffect contracts s event producer consumer = some t) :
    ApplicationRecorded application t.toOperationalState := by
  cases claimFound : event.claim with
  | none => simp [completeExternalEffect, claimFound] at h
  | some claim =>
      cases pendingFound : s.world.pending.find? (fun p =>
          p.intent.effect == event.effect && p.claim == claim) with
      | none => simp [completeExternalEffect, claimFound, pendingFound] at h
      | some pending =>
          have pendingIdentity : pending.intent.effect = event.effect := by
            have matched := List.find?_some pendingFound
            simp only [Bool.and_eq_true, beq_iff_eq] at matched
            exact matched.1
          simp only [completeExternalEffect, claimFound, pendingFound, bind, Option.bind] at h
          split at h
          next guard =>
            cases appliedFound : s.world.applications.find? (fun a => a.intent.effect == event.effect) with
            | none =>
                have different : pending.intent.effect ≠ application.intent.effect := by
                  intro same
                  have eventSame : event.effect = application.intent.effect := pendingIdentity.symm.trans same
                  have recorded : s.world.applications.find? (fun a => a.intent.effect == event.effect) = some application := by
                    simpa only [eventSame] using before
                  rw [recorded] at appliedFound
                  contradiction
                simp only [appliedFound] at h
                split at h
                · cases h
                  exact before
                · split at h
                  · cases h
                    simpa [ApplicationRecorded, responseWorld, List.find?_cons, different] using before
                  · contradiction
            | some previous =>
                simp only [appliedFound] at h
                split at h
                · cases h
                  exact before
                · contradiction
          next => contradiction

theorem E07_completion_records_application {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {producer consumer : Nat}
    (h : completeExternalEffect contracts s event producer consumer = some t)
    (applied : event.result ≠ .rejected) :
    ∃ application, ApplicationRecorded application t.toOperationalState ∧
      application.intent.effect = event.effect ∧ application.result = event.result ∧
      application.providerAfter = event.providerAfter := by
  cases claimFound : event.claim with
  | none => simp [completeExternalEffect, claimFound] at h
  | some claim =>
      cases pendingFound : s.world.pending.find? (fun p =>
          p.intent.effect == event.effect && p.claim == claim) with
      | none => simp [completeExternalEffect, claimFound, pendingFound] at h
      | some pending =>
          have pendingIdentity : pending.intent.effect = event.effect := by
            have matched := List.find?_some pendingFound
            simp only [Bool.and_eq_true, beq_iff_eq] at matched
            exact matched.1
          simp only [completeExternalEffect, claimFound, pendingFound, bind, Option.bind] at h
          split at h
          next guard =>
            cases applicationFound : s.world.applications.find? (fun a => a.intent.effect == event.effect) with
            | none =>
                simp only [applicationFound] at h
                split at h
                next rejected =>
                  exact False.elim (applied (by simpa using rejected))
                next =>
                  split at h
                  · cases h
                    refine ⟨⟨pending.intent, event.result, event.providerAfter⟩, ?_, pendingIdentity, rfl, rfl⟩
                    simp [ApplicationRecorded, responseWorld, List.find?_cons]
                  · contradiction
            | some application =>
                have identity : application.intent.effect = event.effect := by
                  simpa using List.find?_some applicationFound
                have recorded : ApplicationRecorded application s.toOperationalState := by
                  simpa only [ApplicationRecorded, identity] using applicationFound
                simp only [applicationFound] at h
                split at h
                next checked =>
                  cases h
                  have exactness : application.intent = pending.intent ∧ application.result ≠ .rejected ∧
                      event.result = application.result ∧ event.providerAfter = application.providerAfter := by
                    simpa [Bool.and_eq_true, and_assoc] using checked
                  exact ⟨application, recorded, identity, exactness.2.2.1.symm, exactness.2.2.2.symm⟩
                next => contradiction
          next => contradiction

/-- A later delivery for an already applied logical effect is a read-only
response, even if current upstream/provider state has moved since application. -/
theorem E07_existing_application_response_is_read_only {contracts : Contracts} {s t : State}
    {application : LogicalApplication} {event : ExternalEvent} {producer consumer : Nat}
    (recorded : ApplicationRecorded application s.toOperationalState)
    (sameEffect : event.effect = application.intent.effect)
    (h : completeExternalEffect contracts s event producer consumer = some t) :
    t.world.applications = s.world.applications ∧ t.world.provider = s.world.provider ∧
      t.world.producer = s.world.producer ∧ t.world.consumer = s.world.consumer ∧
      event.result = application.result ∧ event.providerAfter = application.providerAfter := by
  have selected : s.world.applications.find? (fun a => a.intent.effect == event.effect) = some application := by
    simpa only [sameEffect] using recorded
  cases claimFound : event.claim with
  | none => simp [completeExternalEffect, claimFound] at h
  | some claim =>
      cases pendingFound : s.world.pending.find? (fun pending =>
          pending.intent.effect == event.effect && pending.claim == claim) with
      | none => simp [completeExternalEffect, claimFound, pendingFound] at h
      | some pending =>
          simp only [completeExternalEffect, claimFound, pendingFound, bind, Option.bind] at h
          split at h
          · simp only [selected] at h
            split at h
            next checked =>
              cases h
              simp only [Bool.and_eq_true, beq_iff_eq, bne_iff_ne, and_assoc] at checked
              exact ⟨rfl, rfl, rfl, rfl, checked.2.2.1, checked.2.2.2⟩
            next => contradiction
          · contradiction

end Kctx.Engineering
