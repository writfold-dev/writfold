import Kctx.Engineering.EffectsSafety

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

private theorem effects_evidence_prepare {contracts : Contracts} {s t : State}
    {intent : Intent} {generation : Nat} (before : EffectsEvidence contracts s.toOperationalState)
    (h : prepare contracts s intent generation = some t) : EffectsEvidence contracts t.toOperationalState := by
  simp only [prepare, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · contradiction
    · split at h <;> try dsimp only at h
      · split at h <;> try dsimp only at h
        · contradiction
        · rename_i charged chargedBy
          cases h
          simp only [EffectsEvidence, (charge_effect_observation_frame chargedBy).2]
          refine ⟨before.1, ?_⟩
          intro effect member
          rcases List.mem_cons.mp member with same | oldMember
          · subst effect
            exact outcome_evidence_unsettled _ _ (by simp)
          · exact before.2 effect oldMember
      · contradiction

private theorem effects_evidence_claim {contracts : Contracts} {s t : State}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : claimDispatch contracts s effect claim expected record = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  simp only [claimDispatch] at h
  operational_success h
  all_goals exact effects_evidence_put before (outcome_evidence_unsettled _ _ (by simp))

private theorem effects_evidence_retry {contracts : Contracts} {s t : State}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : retryClaim contracts s effect claim expected record = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  simp only [retryClaim] at h
  operational_success h
  all_goals exact effects_evidence_put before (outcome_evidence_unsettled _ _ (by simp))

private theorem effects_evidence_issue {contracts : Contracts} {s t : State} {id : Id .effect}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : issueEffect contracts s id = some t) : EffectsEvidence contracts t.toOperationalState := by
  simp only [issueEffect] at h
  operational_success h
  all_goals
    exact effects_evidence_put before
      (outcome_evidence_unsettled _ _ (by simp_all [Bool.and_eq_true]))

private theorem evidence_cancelled_map {contracts : Contracts} {s : State}
    (before : EffectsEvidence contracts s.toOperationalState) (choose : Effect → Bool) :
    ∀ effect ∈ s.effects.map (fun old => if choose old then
      { old with outcome := .cancelledBeforeDispatch } else old),
      OutcomeEvidence s.observations effect := by
  intro effect member
  obtain ⟨old, oldMember, same⟩ := List.mem_map.mp member
  split at same
  · subst effect
    exact outcome_evidence_unsettled _ _ (by simp)
  · subst effect
    exact before.2 old oldMember

private theorem effects_evidence_cancel {contracts : Contracts} {s t : State} {run : ActivationId}
    (before : EffectsEvidence contracts s.toOperationalState) (h : cancel s run = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  simp only [cancel, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      exact ⟨before.1, evidence_cancelled_map before _⟩
    · contradiction

private theorem effects_evidence_rebind {contracts : Contracts} {s t : State}
    {run invocation : ActivationId} {procedure : Procedure} {binding : Binding}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : rebind contracts s run procedure binding invocation = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  simp only [rebind, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · contradiction
    · split at h <;> try dsimp only at h
      · split at h <;> try dsimp only at h
        · split at h <;> try dsimp only at h
          · contradiction
          · rename_i charged chargedBy
            cases h
            have preserved := effects_evidence_charge before chargedBy
            exact ⟨preserved.1, evidence_cancelled_map preserved _⟩
        · contradiction
      · contradiction

/-- Every remaining domain action has a literal effect/observation frame. The
proof unfolds the executable branches, including charged control operations. -/
theorem passive_effect_frame {contracts : Contracts} {s t : State} {action : Action}
    (passive : (match action with
      | .prepare _ _ | .claimDispatch _ _ _ _ | .retryClaim _ _ _ _ | .issueEffect _ | .observe _ |
        .reconcile _ | .cancel _ | .rebind _ _ _ _ => false
      | _ => true) = true)
    (h : domainBody contracts s action = some t) :
    t.effects = s.effects ∧ t.observations = s.observations := by
  cases action <;> simp only [Bool.false_eq_true] at passive <;> try contradiction
  all_goals simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
    selectRoute, call, reserve, observeMaterial, reuse, returnChild, resume, reopen, quiesce,
    propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
    retire, putRun] at h
  all_goals operational_success h
  all_goals exact ⟨rfl, rfl⟩

theorem E07_domain_preserves_evidence {contracts : Contracts} {s t : State} {action : Action}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : domain contracts s action = some t) : EffectsEvidence contracts t.toOperationalState := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next =>
      have preserved : EffectsEvidence contracts next.toOperationalState := by
        cases action with
        | prepare intent generation => exact effects_evidence_prepare before body
        | claimDispatch effect claim expected record => exact effects_evidence_claim before body
        | retryClaim effect claim expected record => exact effects_evidence_retry before body
        | issueEffect effect => exact effects_evidence_issue before body
        | observe event => exact effects_evidence_observe before body
        | reconcile event => exact effects_evidence_reconcile before body
        | cancel run => exact effects_evidence_cancel before body
        | rebind run procedure binding invocation => exact effects_evidence_rebind before body
        | _ =>
            obtain ⟨effects, observations⟩ := passive_effect_frame (by rfl) body
            exact effects_evidence_frame before effects observations
      simp only [domain, body, Option.map_some, Option.some.injEq] at h
      subst t
      exact preserved

theorem E07_local_preserves_evidence {contracts : Contracts} {s t : State} {action : Action}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : localAction contracts s action = some t) : EffectsEvidence contracts t.toOperationalState := by
  cases action <;> simp only [localAction] at h <;> try contradiction
  case crash => cases h; exact effects_evidence_crash before
  case recover => exact effects_evidence_recover before h
  all_goals simp only [project, acquireCut, releaseReadCut] at h
  all_goals operational_success h
  all_goals exact before

theorem E07_environment_preserves_evidence {contracts : Contracts} {s t : State} {action : Environment}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : environment contracts s action = some t) : EffectsEvidence contracts t.toOperationalState := by
  cases body : environmentBody contracts s action with
  | none => simp [environment, body] at h
  | some next =>
      have preserved : EffectsEvidence contracts next.toOperationalState := by
        cases action <;> simp only [environmentBody, completeExternalEffect, responseWorld] at body
        all_goals operational_success body
        all_goals first
          | exact before
          | exact effects_evidence_put before (outcome_evidence_unsettled _ _ (by simp))
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact preserved

theorem E07_transition_preserves_evidence {contracts : Contracts} {s t : State}
    (before : EffectsEvidence contracts s.toOperationalState) (h : Transition contracts s t) :
    EffectsEvidence contracts t.toOperationalState :=
  transition_preserves_operational_property E07_domain_preserves_evidence
    E07_local_preserves_evidence E07_environment_preserves_evidence before h

theorem E07_init_evidence {contracts : Contracts} {input : InitialInputs} {s : State}
    (h : init contracts input = some s) : EffectsEvidence contracts s.toOperationalState := by
  unfold init at h
  split at h
  · cases h
    constructor
    · change ∀ event ∈ ([] : List ExternalEvent), event ∈ contracts.observations
      simp
    · change ∀ effect ∈ ([] : List Effect), OutcomeEvidence ([] : List ExternalEvent) effect
      simp
  · contradiction

theorem E07_reachable_evidence {contracts : Contracts} {input : InitialInputs} {initial s : State}
    (initialized : init contracts input = some initial) (h : Reachable contracts initial s) :
    EffectsEvidence contracts s.toOperationalState :=
  reachable_operational_property E07_domain_preserves_evidence E07_local_preserves_evidence
    E07_environment_preserves_evidence (E07_init_evidence initialized) h

/-- The only external truth assumption is the named adapter soundness boundary.
The interpreter proof supplies the exact qualified observation and classification. -/
theorem E07_reachable_settled_sound {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {input : InitialInputs} {initial s : State}
    (initialized : init contracts input = some initial) (reachable : Reachable contracts initial s)
    {effect : Effect} (member : effect ∈ s.effects)
    (settled : effect.outcome = .confirmed ∨ effect.outcome = .failed ∨ effect.outcome = .partial) :
    ∃ event ∈ s.observations, truth event ∧ event.effect = effect.intent.effect ∧
      event.binding = effect.intent.binding ∧ event.expected = effect.intent.expectedProvider ∧
      (event.result = .rejected → event.claim = effect.dispatch.map Dispatch.id) ∧
      effect.outcome = classify event.result := by
  obtain ⟨event, observed, qualified, exactness⟩ :=
    E07_settled_has_qualified_exact_evidence (E07_reachable_evidence initialized reachable) member settled
  exact ⟨event, observed, sound.observations event qualified, exactness⟩

end Kctx.Engineering
