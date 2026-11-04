import Kctx.Engineering.OperationalProofsRecordsLinkHelpers

set_option autoImplicit false
namespace Kctx.Engineering

/-- A selected-effect write replaces one payload, not its historical subject. -/
private theorem records_effect_map_replace {s : State} {target : OperationalState}
    (before : ∀ e ∈ s.effects, EffectRepresentation target e)
    {next : Effect} (fresh : EffectRepresentation target next) :
    ∀ e ∈ (putEffect s next).effects, EffectRepresentation target e := by
  apply records_map_links
  intro old member
  split
  · exact fresh
  · exact before old member

/-- Outcome-only writes retain the exact intent, generation and dispatch. -/
private theorem records_effect_outcome {target : OperationalState} {e : Effect}
    (before : EffectRepresentation target e) (outcome : EffectOutcome) :
    EffectRepresentation target { e with outcome := outcome } := by
  exact records_effect_replace (next := { e with outcome := outcome }) before rfl before.dispatch

/-- Pin the replacement payload before proving its dispatch subject. The
intent equality alone does not determine the effect's outcome or stamp. -/
private theorem records_effect_dispatch {target : OperationalState} {e : Effect}
    (before : EffectRepresentation target e) {stamp : Dispatch}
    (subject : stamp.intent = e.intent ∧ stamp.generation = e.generation) :
    EffectRepresentation target { e with dispatch := some stamp } := by
  apply records_effect_replace (next := { e with dispatch := some stamp }) before rfl
  intro dispatch equal
  cases equal
  exact subject

/-- Every effect writer is handled at its concrete list operation. Historical
activation, retention and authority are transported independently of that write. -/
theorem records_domainBody_effects {contracts : Contracts} {s t : State} {action : Action}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ e ∈ (frameDomain s action t).effects,
      EffectRepresentation (frameDomain s action t).toOperationalState e := by
  have success : domain contracts s action = some (frameDomain s action t) := by
    simp [domain, h]
  have runs := records_domain_run_extension wf success
  have retention := (records_domain_retention success).1
  have grants : ∀ grant ∈ s.authority.grants,
      grant ∈ (frameDomain s action t).authority.grants := fun _ member => member
  have before : ∀ e ∈ s.effects,
      EffectRepresentation (frameDomain s action t).toOperationalState e :=
    fun e member => records_effect_transport runs retention grants (wf.effects e member)
  cases action with
  | prepare intent generation =>
    simp only [domainBody, prepare] at h
    obtain ⟨run, found, h⟩ := Option.bind_eq_some_iff.mp h
    obtain ⟨node, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
    obtain ⟨guard, h⟩ := Option.ite_none_right_eq_some.mp h
    have ready : intentReady s run intent generation = true := by
      simp only [Bool.and_eq_true] at guard
      exact guard.1.1.1.1.1.1.1
    have form : node.form = .effect intent.capability := by
      simp only [Bool.and_eq_true, beq_iff_eq] at guard
      exact guard.1.1.1.1.2
    have resources : intent.resources = node.resources := by
      simp only [Bool.and_eq_true, beq_iff_eq] at guard
      exact guard.1.1.1.2
    operational_success h
    apply records_cons_links
    · exact records_prepared (source := s) rfl runs retention grants
        found nodeFound ready form resources
    · exact before
  | claimDispatch effect claim expected record =>
    simp only [domainBody, claimDispatch] at h
    obtain ⟨e, found, h⟩ := Option.bind_eq_some_iff.mp h
    operational_success h
    apply records_effect_map_replace before
    apply records_effect_dispatch
      (records_effect_outcome (before e (records_findEffect found)) .claimed)
    exact ⟨rfl, rfl⟩
  | retryClaim effect claim expected record =>
    simp only [domainBody, retryClaim] at h
    obtain ⟨e, found, h⟩ := Option.bind_eq_some_iff.mp h
    operational_success h
    apply records_effect_map_replace before
    apply records_effect_dispatch
      (records_effect_outcome (before e (records_findEffect found)) .claimed)
    exact ⟨rfl, rfl⟩
  | issueEffect effect =>
    simp only [domainBody, issueEffect] at h
    obtain ⟨e, found, h⟩ := Option.bind_eq_some_iff.mp h
    obtain ⟨dispatch, dispatched, h⟩ := Option.bind_eq_some_iff.mp h
    have subject := (before e (records_findEffect found)).dispatch dispatch dispatched
    operational_success h
    apply records_effect_map_replace before
    apply records_effect_dispatch (before e (records_findEffect found))
    exact subject
  | observe event =>
    simp only [domainBody, observeEffect] at h
    obtain ⟨e, found, h⟩ := Option.bind_eq_some_iff.mp h
    operational_success h
    all_goals first
      | exact before
      | exact records_effect_map_replace before
          (records_effect_outcome (before e (records_findEffect found)) _)
  | reconcile event =>
    simp only [domainBody, reconcile] at h
    obtain ⟨e, found, h⟩ := Option.bind_eq_some_iff.mp h
    operational_success h
    exact records_effect_map_replace before
      (records_effect_outcome (before e (records_findEffect found)) _)
  | cancel run =>
    simp only [domainBody, cancel] at h
    operational_success h
    apply records_map_links
    intro old member
    split
    · exact records_effect_outcome (before old member) _
    · exact before old member
  | rebind run procedure binding invocation =>
    simp only [domainBody, rebind] at h
    operational_success h
    apply records_map_links
    intro old member
    split
    · exact records_effect_outcome (before old member) _
    · exact before old member
  | _ =>
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, reserve, observeMaterial, reuse, returnChild, resume, reopen,
      quiesce, propose, evaluate, review, prepareAdmission, publishAdmission,
      acknowledgeAdmission, retire] at h
    operational_success h
    all_goals exact before

/-- Reservation creation binds the selected activation; counter updates and
quiescence preserve that subject by map and filter membership respectively. -/
theorem records_domainBody_ownership {contracts : Contracts} {s t : State} {action : Action}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ o ∈ (frameDomain s action t).ownership,
      ∃ r ∈ (frameDomain s action t).runs, r.invocation = o.holder ∧ r.id = o.owner := by
  have success : domain contracts s action = some (frameDomain s action t) := by
    simp [domain, h]
  have runs := records_domain_run_extension wf success
  have before : ∀ o ∈ s.ownership, ∃ r ∈ (frameDomain s action t).runs,
      r.invocation = o.holder ∧ r.id = o.owner :=
    fun o member => records_ownership_transport runs (wf.ownership o member)
  cases action with
  | reserve activation keys generation =>
    simp only [domainBody, reserve] at h
    obtain ⟨run, found, h⟩ := Option.bind_eq_some_iff.mp h
    operational_success h
    apply records_append_links
    · intro reservation member
      obtain ⟨key, _, rfl⟩ := List.mem_map.mp member
      exact records_reservation_new runs found key generation
    · exact before
  | issueEffect effect =>
    simp only [domainBody, issueEffect] at h
    operational_success h
    apply records_map_links
    intro old member
    split <;> exact before old member
  | quiesce root =>
    simp only [domainBody, quiesce] at h
    operational_success h
    exact records_filter_links before
  | _ =>
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, prepare, claimDispatch, retryClaim, observeEffect,
      observeMaterial, reconcile, reuse, returnChild, rebind, resume, reopen, cancel,
      propose, evaluate, review, prepareAdmission, publishAdmission,
      acknowledgeAdmission, retire] at h
    operational_success h
    all_goals exact before

end Kctx.Engineering
