import Kctx.Engineering.EffectsSafetyApplications

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

private theorem domain_applications_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) : t.world.applications = s.world.applications := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next =>
      have frame : next.world.applications = s.world.applications := by
        cases action <;> simp only [domainBody, resolve, activate, loadResource, advanceNode,
          failNode, selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect,
          observeEffect, observeMaterial, reconcile, reuse, returnChild, rebind, resume,
          reopen, cancel, quiesce, propose, evaluate, review, prepareAdmission, publishAdmission,
          acknowledgeAdmission, retire, putRun, putEffect] at body
        all_goals operational_success body
        all_goals rfl
      simp only [domain, body, Option.map_some, Option.some.injEq] at h
      subst t
      exact frame

private theorem local_applications_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) : t.world.applications = s.world.applications := by
  cases action <;> simp only [localAction, project, acquireCut, releaseReadCut, crash,
    recover] at h
  all_goals operational_success h
  all_goals rfl

theorem E07_environment_preserves_application {contracts : Contracts} {s t : State}
    {action : Environment} {application : LogicalApplication}
    (before : ApplicationRecorded application s.toOperationalState)
    (h : environment contracts s action = some t) : ApplicationRecorded application t.toOperationalState := by
  cases body : environmentBody contracts s action with
  | none => simp [environment, body] at h
  | some next =>
      have preserved : ApplicationRecorded application next.toOperationalState := by
        cases action with
        | completeExternalEffect event producer consumer =>
            exact E07_completion_preserves_application before body
        | _ =>
            simp only [environmentBody, putEffect] at body
            operational_success body
            all_goals exact before
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact preserved

theorem E07_application_persists_reachable {contracts : Contracts} {initial s : State}
    {application : LogicalApplication} (recorded : ApplicationRecorded application initial.toOperationalState)
    (trace : Reachable contracts initial s) : ApplicationRecorded application s.toOperationalState := by
  apply reachable_operational_property (P := ApplicationRecorded application) ?_ ?_
    E07_environment_preserves_application recorded trace
  · intro before after action old executed
    simpa only [ApplicationRecorded, domain_applications_frame executed] using old
  · intro before after action old executed
    simpa only [ApplicationRecorded, local_applications_frame executed] using old

theorem E07_environment_records_application {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {producer consumer : Nat}
    (h : environment contracts s (.completeExternalEffect event producer consumer) = some t)
    (applied : event.result ≠ .rejected) :
    ∃ application, ApplicationRecorded application t.toOperationalState ∧
      application.intent.effect = event.effect ∧ application.result = event.result ∧
      application.providerAfter = event.providerAfter := by
  cases body : environmentBody contracts s (.completeExternalEffect event producer consumer) with
  | none => simp [environment, body] at h
  | some next =>
      have recorded := E07_completion_records_application body applied
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact recorded

theorem E07_environment_existing_application_is_read_only {contracts : Contracts} {s t : State}
    {application : LogicalApplication} {event : ExternalEvent} {producer consumer : Nat}
    (recorded : ApplicationRecorded application s.toOperationalState)
    (sameEffect : event.effect = application.intent.effect)
    (h : environment contracts s (.completeExternalEffect event producer consumer) = some t) :
    t.world.applications = s.world.applications ∧ t.world.provider = s.world.provider ∧
      t.world.producer = s.world.producer ∧ t.world.consumer = s.world.consumer ∧
      event.result = application.result ∧ event.providerAfter = application.providerAfter := by
  cases body : environmentBody contracts s (.completeExternalEffect event producer consumer) with
  | none => simp [environment, body] at h
  | some next =>
      have response := E07_existing_application_response_is_read_only recorded sameEffect body
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact response

/-- No second logical application across arbitrary actual transitions: a later
qualified retry returns the first Applied/Partial result and cannot mutate the
current provider vector or append another canonical application. Rejection of
an earlier attempt is intentionally excluded: proven-no-effect retry may apply. -/
theorem E07_no_second_logical_application_after_trace {contracts : Contracts}
    {before applied current after : State} {first second : ExternalEvent}
    {producer consumer nextProducer nextConsumer : Nat}
    (firstCompletion : environment contracts before (.completeExternalEffect first producer consumer) = some applied)
    (nonRejected : first.result ≠ .rejected) (trace : Reachable contracts applied current)
    (sameEffect : second.effect = first.effect)
    (secondCompletion : environment contracts current
      (.completeExternalEffect second nextProducer nextConsumer) = some after) :
    after.world.applications = current.world.applications ∧ after.world.provider = current.world.provider ∧
      after.world.producer = current.world.producer ∧ after.world.consumer = current.world.consumer ∧
      second.result = first.result ∧ second.providerAfter = first.providerAfter := by
  obtain ⟨application, recorded, identity, result, providerAfter⟩ :=
    E07_environment_records_application firstCompletion nonRejected
  have persisted := E07_application_persists_reachable recorded trace
  obtain ⟨applications, provider, producer, consumer, response, originalRevision⟩ :=
    E07_environment_existing_application_is_read_only persisted
      (sameEffect.trans identity.symm) secondCompletion
  exact ⟨applications, provider, producer, consumer, response.trans result,
    originalRevision.trans providerAfter⟩

end Kctx.Engineering
