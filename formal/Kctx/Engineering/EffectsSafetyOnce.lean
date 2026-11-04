import Kctx.Engineering.EffectsSafetyDispatch
import Kctx.Engineering.EffectsSafetyPreservation

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- A claim is closed once every retained stamp with that identity is issued.
The inventory is permanent even when a qualified retry replaces Effect.dispatch. -/
def ClaimClosed (id : Id .claim) (s : OperationalState) : Prop :=
  (∃ dispatch ∈ s.claims, dispatch.id = id) ∧
  ∀ dispatch ∈ s.claims, dispatch.id = id → dispatch.issued = true

theorem find_effect_identity {s : State} {id : Id .effect} {effect : Effect}
    (h : findEffect s id = some effect) : effect.intent.effect = id := by
  simpa using List.find?_some h

theorem find_effect_map {effects : List Effect} {id : Id .effect} {effect : Effect}
    (found : effects.find? (·.intent.effect == id) = some effect)
    (f : Effect → Effect) (keys : ∀ old, (f old).intent.effect = old.intent.effect) :
    (effects.map f).find? (·.intent.effect == id) = some (f effect) := by
  rw [List.find?_map]
  have same : ((fun e : Effect => e.intent.effect == id) ∘ f) =
      (fun e : Effect => e.intent.effect == id) := by
    funext old
    simp only [Function.comp_apply, keys]
  rw [same, found]
  rfl

theorem find_effect_put_selected {s : State} {id : Id .effect} {old next : Effect}
    (found : findEffect s id = some old) (key : next.intent.effect = old.intent.effect) :
    findEffect (putEffect s next) id = some next := by
  have identity := find_effect_identity found
  have mapped := find_effect_map found
    (fun e => if e.intent.effect == next.intent.effect then next else e) (by
      intro e
      change (if e.intent.effect == next.intent.effect then next else e).intent.effect = e.intent.effect
      split
      next same =>
        have equality : e.intent.effect = next.intent.effect := by simpa using same
        exact equality.symm
      next => rfl)
  simpa only [findEffect, putEffect, key, beq_self_eq_true, ↓reduceIte] using mapped

theorem claim_closed_fresh_append {s : OperationalState} {id : Id .claim} {dispatch : Dispatch}
    (before : ClaimClosed id s) (fresh : s.claims.any (·.id == dispatch.id) = false) :
    ClaimClosed id { s with claims := dispatch :: s.claims } := by
  obtain ⟨old, member, identity⟩ := before.1
  refine ⟨⟨old, List.mem_cons_of_mem _ member, identity⟩, ?_⟩
  intro candidate present same
  rcases List.mem_cons.mp present with headEq | presentTail
  · subst candidate
    have occupied : s.claims.any (·.id == dispatch.id) = true :=
      List.any_eq_true.mpr ⟨old, member, by simpa [same] using identity⟩
    rw [occupied] at fresh
    contradiction
  · exact before.2 candidate presentTail same

theorem claim_closed_mark {s : OperationalState} {id claim : Id .claim}
    (before : ClaimClosed id s) :
    ClaimClosed id { s with claims := s.claims.map (fun old =>
      if old.id == claim then { old with issued := true } else old) } := by
  obtain ⟨old, member, identity⟩ := before.1
  refine ⟨?_, ?_⟩
  · refine ⟨_, List.mem_map.mpr ⟨old, member, rfl⟩, ?_⟩
    split <;> exact identity
  · intro current currentMember same
    obtain ⟨source, sourceMember, equality⟩ := List.mem_map.mp currentMember
    split at equality
    · subst current
      rfl
    · subst current
      exact before.2 source sourceMember same

theorem claim_mark_closes {s : OperationalState} {dispatch : Dispatch}
    (member : dispatch ∈ s.claims) :
    ClaimClosed dispatch.id { s with claims := s.claims.map (fun old =>
      if old.id == dispatch.id then { old with issued := true } else old) } := by
  refine ⟨?_, ?_⟩
  · refine ⟨{ dispatch with issued := true }, List.mem_map.mpr ⟨dispatch, member, by simp⟩, rfl⟩
  · intro candidate candidateMember same
    obtain ⟨old, oldMember, equality⟩ := List.mem_map.mp candidateMember
    split at equality
    · subst candidate
      rfl
    · subst candidate
      simp_all

theorem charge_claims_frame {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) : t.claims = s.claims := by
  simp only [charge, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      rfl
    · contradiction

theorem E07_issue_marks_issued {contracts : Contracts} {s t : State} {id : Id .effect}
    (h : issueEffect contracts s id = some t) :
    ∃ checked : DispatchChecked contracts s id, ClaimClosed checked.dispatch.id t.toOperationalState := by
  obtain ⟨checked⟩ := E07_issue_checks_selected_claim h
  refine ⟨checked, ?_⟩
  simp only [issueEffect, checked.selectedEffect, checked.selectedDispatch,
    checked.selectedRun, checked.selectedGrant, checked.selectedNode, bind, Option.bind_some] at h
  obtain ⟨_, result⟩ := Option.ite_none_right_eq_some.mp h
  cases result
  exact claim_mark_closes checked.inventory

theorem E07_closed_claim_cannot_issue {contracts : Contracts} {s : State} {id : Id .effect}
    {claim : Id .claim} (closed : ClaimClosed claim s.toOperationalState)
    (checked : DispatchChecked contracts s id) : checked.dispatch.id ≠ claim := by
  intro same
  have issued := closed.2 checked.dispatch checked.inventory same
  rw [checked.unissued] at issued
  contradiction

theorem E07_unclaimed_issue_refused {contracts : Contracts} {s : State} {id : Id .effect}
    {effect : Effect} (found : findEffect s id = some effect)
    (unclaimed : effect.outcome ≠ .claimed) : issueEffect contracts s id = none := by
  cases result : issueEffect contracts s id with
  | none => rfl
  | some next =>
      obtain ⟨checked⟩ := E07_issue_checks_selected_claim result
      have same : checked.effect = effect := Option.some.inj (checked.selectedEffect.symm.trans found)
      have claimed := checked.claimed
      rw [same] at claimed
      exact False.elim (unclaimed claimed)

theorem E07_unprepared_initial_claim_refused {contracts : Contracts} {s : State} {id : Id .effect}
    {effect : Effect} {claim : Id .claim} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (found : findEffect s id = some effect) (unprepared : effect.outcome ≠ .prepared) :
    claimDispatch contracts s id claim expected record = none := by
  cases result : claimDispatch contracts s id claim expected record with
  | none => rfl
  | some next =>
      unfold claimDispatch at result
      obtain ⟨selected, selectedBy, withRun⟩ := Option.bind_eq_some_iff.mp result
      obtain ⟨run, _, withGrant⟩ := Option.bind_eq_some_iff.mp withRun
      obtain ⟨grant, _, withNode⟩ := Option.bind_eq_some_iff.mp withGrant
      obtain ⟨node, _, checked⟩ := Option.bind_eq_some_iff.mp withNode
      obtain ⟨guard, _⟩ := Option.ite_none_right_eq_some.mp checked
      have same : selected = effect := Option.some.inj (selectedBy.symm.trans found)
      have prepared : selected.outcome = .prepared := by
        simp only [Bool.and_eq_true, beq_iff_eq, and_assoc] at guard
        exact guard.1
      rw [same] at prepared
      exact False.elim (unprepared prepared)

/-- Qualified no-effect evidence closes the claim before it becomes observable.
Even a fully live grant cannot race a send between observation and reconciliation. -/
theorem E07_no_effect_observation_closes_send {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {effect : Effect}
    (found : findEffect s event.effect = some effect) (claimed : effect.outcome = .claimed)
    (rejected : event.result = .rejected) (h : observeEffect contracts s event = some t) :
    issueEffect contracts t event.effect = none := by
  simp only [observeEffect, found, bind, Option.bind_some] at h
  obtain ⟨_, result⟩ := Option.ite_none_right_eq_some.mp h
  simp only [rejected, claimed, beq_iff_eq, reduceCtorEq, ↓reduceIte] at result
  cases result
  apply E07_unclaimed_issue_refused (effect := { effect with outcome := .unknown })
  · exact find_effect_put_selected found rfl
  · simp

/-- An absence observation before claiming closes preparation as cancellation.
It cannot remain in history while the same effect is subsequently claimed. -/
theorem E07_no_effect_observation_closes_preparation {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {effect : Effect} {claim : Id .claim}
    {expected : Option Ledger.Head} {record : LedgerRecordId}
    (found : findEffect s event.effect = some effect) (prepared : effect.outcome = .prepared)
    (rejected : event.result = .rejected) (h : observeEffect contracts s event = some t) :
    claimDispatch contracts t event.effect claim expected record = none ∧
      issueEffect contracts t event.effect = none := by
  simp only [observeEffect, found, bind, Option.bind_some] at h
  obtain ⟨_, result⟩ := Option.ite_none_right_eq_some.mp h
  simp only [rejected, prepared, beq_iff_eq, reduceCtorEq, ↓reduceIte] at result
  cases result
  have closed := find_effect_put_selected
    (next := { effect with outcome := .cancelledBeforeDispatch }) found rfl
  constructor
  · apply E07_unprepared_initial_claim_refused (effect := { effect with outcome := .cancelledBeforeDispatch })
    · exact closed
    · simp
  · apply E07_unclaimed_issue_refused (effect := { effect with outcome := .cancelledBeforeDispatch })
    · exact closed
    · simp

/-- A no-effect result is attempt-specific. Retaining the old observation does
not permit it to classify a later authorized retry as Failed. -/
theorem E07_reconcile_refuses_old_rejection {s : State} {event : ExternalEvent} {effect : Effect}
    (found : findEffect s event.effect = some effect) (rejected : event.result = .rejected)
    (oldClaim : event.claim ≠ effect.dispatch.map Dispatch.id) :
    reconcile s event = none := by
  simp [reconcile, found, rejected, oldClaim]

end Kctx.Engineering
