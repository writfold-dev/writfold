import Kctx.Engineering.EffectsSafetyDispatch

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Retry permission names the immutable logical intent, the exact previous
claim, current incarnation, policy and a bounded authorization interval. -/
theorem E12_retry_has_exact_authorization {contracts : Contracts} {s t : State}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (h : retryClaim contracts s effect claim expected record = some t) :
    ∃ old authorization, findEffect s effect = some old ∧ old.outcome = .unknown ∧
      authorization ∈ contracts.retryAuthorizations ∧
      authorization.previousClaim = old.dispatch.map Dispatch.id ∧
      authorization.intent = old.intent ∧ authorization.incarnation = s.incarnation ∧
      authorization.policy = s.authority.policy ∧ authorization.validFrom ≤ s.now ∧
      s.now < authorization.expires := by
  unfold retryClaim at h
  obtain ⟨old, effectFound, withRun⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨run, _, withGrant⟩ := Option.bind_eq_some_iff.mp withRun
  obtain ⟨grant, _, withNode⟩ := Option.bind_eq_some_iff.mp withGrant
  obtain ⟨node, _, withAuthorization⟩ := Option.bind_eq_some_iff.mp withNode
  obtain ⟨authorization, authorizationFound, selected⟩ := Option.bind_eq_some_iff.mp withAuthorization
  have qualified := List.mem_of_find?_eq_some authorizationFound
  have exactness := List.find?_some authorizationFound
  have checked : authorization.intent = old.intent ∧
      authorization.incarnation = s.incarnation ∧ authorization.policy = s.authority.policy ∧
      authorization.validFrom ≤ s.now ∧ s.now < authorization.expires ∧
      authorization.previousClaim = old.dispatch.map Dispatch.id := by
    simpa [retryAuthorizationValid, Bool.and_eq_true, and_assoc] using exactness
  obtain ⟨guard, _⟩ := Option.ite_none_right_eq_some.mp selected
  have unknown : old.outcome = .unknown := by
    simp only [Bool.and_eq_true, beq_iff_eq, and_assoc] at guard
    exact guard.1
  exact ⟨old, authorization, effectFound, unknown,
    qualified, checked.2.2.2.2.2, checked.1, checked.2.1, checked.2.2.1,
    checked.2.2.2.1, checked.2.2.2.2.1⟩

/-- The corresponding ledgered request uses the same captured operational
inputs. Retained older authorizations cannot be silently rebound to a new claim. -/
theorem E12_append_retry_has_exact_authorization {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {effect : Id .effect} {claim : Id .claim}
    (h : step contracts s (.append stream expected record (.retryClaim effect claim expected record)) = some t) :
    ∃ old authorization, findEffect s effect = some old ∧ old.outcome = .unknown ∧
      authorization ∈ contracts.retryAuthorizations ∧
      authorization.previousClaim = old.dispatch.map Dispatch.id ∧
      authorization.intent = old.intent ∧ authorization.incarnation = s.incarnation ∧
      authorization.policy = s.authority.policy ∧ authorization.validFrom ≤ s.now ∧
      s.now < authorization.expires := by
  obtain ⟨next, executed, _⟩ := append_domain_witness h
  cases body : domainBody contracts
      (historicalState (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream record (.retryClaim effect claim expected record)))
      (.retryClaim effect claim expected record) with
  | none => simp [domain, body] at executed
  | some result =>
      have authorized := E12_retry_has_exact_authorization body
      exact authorized

/-- The authorization retained in a retry claim is revalidated at the physical
send. Claim persistence does not freeze its validity interval or policy. -/
theorem E12_dispatch_retry_revalidates {contracts : Contracts} {s : State} {id : Id .effect}
    (checked : DispatchChecked contracts s id) {authorization : RetryAuthorization}
    (retry : checked.dispatch.retryAuthorization = some authorization) :
    checked.dispatch.retry = some authorization.kind ∧
      authorization ∈ contracts.retryAuthorizations ∧ authorization.intent = checked.dispatch.intent ∧
      authorization.incarnation = s.incarnation ∧ authorization.policy = s.authority.policy ∧
      authorization.validFrom ≤ s.now ∧ s.now < authorization.expires := by
  have valid := checked.retryValid
  simpa [dispatchRetryValid, retry, retryAuthorizationValid, Bool.and_eq_true, and_assoc] using valid

/-- Current-grant Binding equality includes the pinned native provider revision.
A retry can cross a changed provider only through its qualified idempotent mode;
WorldEffects still fences a first application with the original provider CAS. -/
theorem E12_dispatch_provider_is_binding_pinned {contracts : Contracts} {s : State} {id : Id .effect}
    (checked : DispatchChecked contracts s id) :
    checked.effect.intent.expectedProvider = checked.run.binding.expectedProvider := by
  have ready := checked.ownershipAndAuthority
  simp only [intentReady, Bool.and_eq_true, beq_iff_eq, and_assoc] at ready
  exact ready.2.2.2.2.2.1

/-- A root's ownership cannot authorize overlapping unresolved effects from
other nodes. The same immutable EffectId is the sole explicit retry exemption. -/
theorem E06_dispatch_excludes_other_unsettled_effects {contracts : Contracts} {s : State} {id : Id .effect}
    (checked : DispatchChecked contracts s id) {other : Effect} (member : other ∈ s.effects)
    (different : other.intent.effect ≠ checked.effect.intent.effect)
    (unresolved : unsettled other.outcome = true) {resource : Id .resource}
    (used : resource ∈ other.intent.resources) : resource ∉ checked.effect.intent.resources := by
  have ready := checked.ownershipAndAuthority
  simp only [intentReady, Bool.and_eq_true, and_assoc] at ready
  rcases ready with ⟨_, _, _, _, _, _, _, _, _, _, _, noConflict, _, _⟩
  intro shared
  have conflict : s.effects.any (fun e => e.intent.effect != checked.effect.intent.effect &&
      unsettled e.outcome && e.intent.resources.any checked.effect.intent.resources.contains) = true := by
    apply List.any_eq_true.mpr
    refine ⟨other, member, ?_⟩
    have overlap : other.intent.resources.any checked.effect.intent.resources.contains = true :=
      List.any_eq_true.mpr ⟨resource, used, by simpa using shared⟩
    simp [different, unresolved, overlap]
  simp only [noConflictingEffect, conflict, Bool.not_true, Bool.false_eq_true] at noConflict

end Kctx.Engineering
