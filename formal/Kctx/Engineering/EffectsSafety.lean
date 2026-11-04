import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 1600000
namespace Kctx.Engineering

/-- A settled outcome is justified by the adapter's exact retained observation.
Unknown is deliberately not classified as failure. -/
def OutcomeEvidence (observations : List ExternalEvent) (effect : Effect) : Prop :=
  effect.outcome = .confirmed ∨ effect.outcome = .failed ∨ effect.outcome = .partial →
    ∃ event ∈ observations, event.effect = effect.intent.effect ∧
      event.binding = effect.intent.binding ∧ event.expected = effect.intent.expectedProvider ∧
      (event.result = .rejected → event.claim = effect.dispatch.map Dispatch.id) ∧
      effect.outcome = classify event.result

/-- Qualification and exact outcome support persist independently of authority,
coordinator lifetime, reply delivery and the current provider revision. -/
def EffectsEvidence (contracts : Contracts) (s : OperationalState) : Prop :=
  (∀ event ∈ s.observations, event ∈ contracts.observations) ∧
  (∀ effect ∈ s.effects, OutcomeEvidence s.observations effect)

theorem outcome_evidence_monotone {before after : List ExternalEvent} {effect : Effect}
    (included : ∀ event ∈ before, event ∈ after) (h : OutcomeEvidence before effect) :
    OutcomeEvidence after effect := by
  intro settled
  obtain ⟨event, member, exactness⟩ := h settled
  exact ⟨event, included event member, exactness⟩

theorem outcome_evidence_unsettled (observations : List ExternalEvent) (effect : Effect)
    (h : effect.outcome ≠ .confirmed ∧ effect.outcome ≠ .failed ∧ effect.outcome ≠ .partial) :
    OutcomeEvidence observations effect := by
  intro settled
  rcases settled with confirmed | failed | partialOutcome
  · exact False.elim (h.1 confirmed)
  · exact False.elim (h.2.1 failed)
  · exact False.elim (h.2.2 partialOutcome)

theorem outcome_evidence_exact {observations : List ExternalEvent} {effect : Effect}
    {event : ExternalEvent} (member : event ∈ observations)
    (id : event.effect = effect.intent.effect) (binding : event.binding = effect.intent.binding)
    (provider : event.expected = effect.intent.expectedProvider)
    (attempt : event.result = .rejected → event.claim = effect.dispatch.map Dispatch.id) :
    OutcomeEvidence observations { effect with outcome := classify event.result } := by
  intro _
  exact ⟨event, member, id, binding, provider, attempt, rfl⟩

theorem effects_evidence_put {contracts : Contracts} {s : State} {effect : Effect}
    (before : EffectsEvidence contracts s.toOperationalState)
    (supported : OutcomeEvidence s.observations effect) :
    EffectsEvidence contracts (putEffect s effect).toOperationalState := by
  refine ⟨before.1, ?_⟩
  intro result member
  obtain ⟨old, oldMember, same⟩ := List.mem_map.mp member
  split at same
  · subst result
    exact supported
  · subst result
    exact before.2 old oldMember

theorem effects_evidence_frame {contracts : Contracts} {s t : State}
    (before : EffectsEvidence contracts s.toOperationalState)
    (effects : t.effects = s.effects) (observations : t.observations = s.observations) :
    EffectsEvidence contracts t.toOperationalState := by
  simpa only [EffectsEvidence, effects, observations] using before

theorem charge_effect_observation_frame {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) : t.effects = s.effects ∧ t.observations = s.observations := by
  simp only [charge, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      exact ⟨rfl, rfl⟩
    · contradiction

theorem effects_evidence_charge {contracts : Contracts} {s t : State} {root : RunId} {cost : Nat}
    (before : EffectsEvidence contracts s.toOperationalState) (h : charge s root cost = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  unfold charge at h
  simp only [bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      exact before
    · contradiction

theorem effects_evidence_reconcile {contracts : Contracts} {s t : State} {event : ExternalEvent}
    (before : EffectsEvidence contracts s.toOperationalState) (h : reconcile s event = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  cases found : findEffect s event.effect with
  | none => simp [reconcile, found] at h
  | some effect =>
      simp only [reconcile, found, bind, Option.bind] at h
      split at h
      next guard =>
        cases h
        have parts : event ∈ s.observations ∧ event.binding = effect.intent.binding ∧
            event.expected = effect.intent.expectedProvider ∧
            (event.result ≠ .rejected ∨ event.claim = effect.dispatch.map Dispatch.id) ∧
            (effect.outcome = .claimed ∨ effect.outcome = .unknown) := by
          simpa [Bool.and_eq_true, Bool.or_eq_true, List.contains_iff, and_assoc] using guard
        have identity : effect.intent.effect = event.effect := by
          have selected := List.find?_some found
          simpa only [findEffect, beq_iff_eq] using selected
        have attempt : event.result = .rejected → event.claim = effect.dispatch.map Dispatch.id := by
          intro rejected
          rcases parts.2.2.2.1 with different | same
          · exact False.elim (different rejected)
          · exact same
        exact effects_evidence_put before
          (outcome_evidence_exact parts.1 identity.symm parts.2.1 parts.2.2.1 attempt)
      next => contradiction

theorem effects_evidence_crash {contracts : Contracts} {s : State}
    (before : EffectsEvidence contracts s.toOperationalState) :
    EffectsEvidence contracts (crash s).toOperationalState := by
  refine ⟨before.1, ?_⟩
  intro effect member
  obtain ⟨old, oldMember, same⟩ := List.mem_map.mp member
  split at same
  · subst effect
    exact outcome_evidence_unsettled _ _ (by simp)
  · subst effect
    exact before.2 old oldMember

theorem effects_evidence_recover {contracts : Contracts} {s t : State}
    (before : EffectsEvidence contracts s.toOperationalState) (h : recover s = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  unfold recover at h
  split at h
  · cases h
    exact before
  · contradiction

theorem effects_evidence_add_observation {contracts : Contracts} {s : OperationalState}
    {event : ExternalEvent} (before : EffectsEvidence contracts s)
    (qualified : event ∈ contracts.observations) :
    EffectsEvidence contracts { s with observations := event :: s.observations } := by
  refine ⟨?_, ?_⟩
  · intro observation member
    rcases List.mem_cons.mp member with same | oldMember
    · simpa only [same] using qualified
    · exact before.1 observation oldMember
  · intro effect member
    exact outcome_evidence_monotone (fun _ old => List.mem_cons_of_mem _ old)
      (before.2 effect member)

theorem effects_evidence_observe {contracts : Contracts} {s t : State} {event : ExternalEvent}
    (before : EffectsEvidence contracts s.toOperationalState)
    (h : observeEffect contracts s event = some t) :
    EffectsEvidence contracts t.toOperationalState := by
  cases found : findEffect s event.effect with
  | none => simp [observeEffect, found] at h
  | some effect =>
      simp only [observeEffect, found, bind, Option.bind] at h
      split at h
      next guard =>
        have qualified : event ∈ contracts.observations := by
          simp_all only [Bool.and_eq_true, observationAvailable, List.contains_iff]
        repeat first
          | (split at h <;> try dsimp only at h)
          | (cases h; exact effects_evidence_add_observation before qualified)
          | (cases h
             exact effects_evidence_add_observation
               (effects_evidence_put before (outcome_evidence_unsettled _ _ (by simp))) qualified)
      next => contradiction

theorem E07_settled_has_qualified_exact_evidence {contracts : Contracts} {s : State}
    (invariant : EffectsEvidence contracts s.toOperationalState) {effect : Effect}
    (member : effect ∈ s.effects)
    (settled : effect.outcome = .confirmed ∨ effect.outcome = .failed ∨ effect.outcome = .partial) :
    ∃ event ∈ s.observations, event ∈ contracts.observations ∧
      event.effect = effect.intent.effect ∧ event.binding = effect.intent.binding ∧
      event.expected = effect.intent.expectedProvider ∧
      (event.result = .rejected → event.claim = effect.dispatch.map Dispatch.id) ∧
      effect.outcome = classify event.result := by
  obtain ⟨event, observed, exactness⟩ := invariant.2 effect member settled
  exact ⟨event, observed, invariant.1 event observed, exactness⟩

end Kctx.Engineering
