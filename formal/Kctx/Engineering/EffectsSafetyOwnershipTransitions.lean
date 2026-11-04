import Kctx.Engineering.EffectsSafetyOwnership

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

def ReservationIdentity (owner : Reservation) : Id .resource × RunId × ActivationId × Nat :=
  (owner.resource, owner.owner, owner.holder, owner.generation)

def ReservationsRetained (before after : OperationalState) : Prop :=
  ∀ owner ∈ before.ownership, ∃ next ∈ after.ownership,
    ReservationIdentity next = ReservationIdentity owner

/-- A release or transfer is justified at that edge, not retroactively by a
later cancellation, reconciliation, or unrelated process certificate. -/
def OwnershipEdgeSafe (contracts : Contracts) (before after : OperationalState) : Prop :=
  ∀ owner ∈ before.ownership,
    (∃ next ∈ after.ownership, ReservationIdentity next = ReservationIdentity owner) ∨
      ReleaseChecked contracts before owner.owner

private theorem reservations_retained_refl (s : OperationalState) : ReservationsRetained s s := by
  intro owner member
  exact ⟨owner, member, rfl⟩

private theorem reservations_retained_map (s : OperationalState) (f : Reservation → Reservation)
    (identities : ∀ owner, ReservationIdentity (f owner) = ReservationIdentity owner) :
    ReservationsRetained s { s with ownership := s.ownership.map f } := by
  intro owner member
  exact ⟨f owner, List.mem_map.mpr ⟨owner, member, rfl⟩, identities owner⟩

private theorem retained_edge_safe {contracts : Contracts} {s t : OperationalState}
    (retained : ReservationsRetained s t) : OwnershipEdgeSafe contracts s t := by
  intro owner member
  exact Or.inl (retained owner member)

private theorem passive_ownership_frame {contracts : Contracts} {s t : State} {action : Action}
    (passive : (match action with | .reserve _ _ _ | .issueEffect _ | .quiesce _ => false | _ => true) = true)
    (h : domainBody contracts s action = some t) : t.ownership = s.ownership := by
  cases action <;> simp only [Bool.false_eq_true] at passive <;> try contradiction
  all_goals simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
    selectRoute, call, prepare, claimDispatch, retryClaim, observeEffect, observeMaterial, reconcile,
    reuse, returnChild, rebind, resume, reopen, cancel, propose, evaluate, review, prepareAdmission,
    publishAdmission, acknowledgeAdmission, retire, putRun, putEffect] at h
  all_goals operational_success h
  all_goals rfl

private theorem reserve_retains_ownership {s t : State} {run : ActivationId}
    {keys : List (Id .resource)} {generation : Nat} (h : reserve s run keys generation = some t) :
    ReservationsRetained s.toOperationalState t.toOperationalState := by
  simp only [reserve] at h
  operational_success h
  all_goals
    intro owner member
    exact ⟨owner, List.mem_append_right _ member, rfl⟩

private theorem issue_retains_ownership {contracts : Contracts} {s t : State} {id : Id .effect}
    (h : issueEffect contracts s id = some t) :
    ReservationsRetained s.toOperationalState t.toOperationalState := by
  simp only [issueEffect] at h
  operational_success h
  all_goals
    apply reservations_retained_map
    intro owner
    split <;> rfl

theorem E06_domain_ownership_edge {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) :
    OwnershipEdgeSafe contracts s.toOperationalState t.toOperationalState := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next =>
      have checked : OwnershipEdgeSafe contracts s.toOperationalState next.toOperationalState := by
        cases action with
        | reserve run keys generation => exact retained_edge_safe (reserve_retains_ownership body)
        | issueEffect id => exact retained_edge_safe (issue_retains_ownership body)
        | quiesce root =>
            obtain ⟨release, ownership⟩ := E06_quiesce_release_checked body
            intro owner member
            by_cases sameRoot : owner.owner = root
            · exact Or.inr (sameRoot.symm ▸ release)
            · left
              refine ⟨owner, ?_, rfl⟩
              rw [ownership]
              exact List.mem_filter.mpr ⟨member, by simpa using sameRoot⟩
        | _ =>
            apply retained_edge_safe
            intro owner member
            exact ⟨owner, (passive_ownership_frame (by rfl) body).symm ▸ member, rfl⟩
      simp only [domain, body, Option.map_some, Option.some.injEq] at h
      subst t
      exact checked

theorem E06_local_ownership_edge {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) :
    OwnershipEdgeSafe contracts s.toOperationalState t.toOperationalState := by
  cases action <;> simp only [localAction, project, acquireCut, releaseReadCut,
    crash, recover] at h
  all_goals operational_success h
  all_goals exact retained_edge_safe (reservations_retained_refl _)

theorem E06_environment_ownership_edge {contracts : Contracts} {s t : State} {action : Environment}
    (h : environment contracts s action = some t) :
    OwnershipEdgeSafe contracts s.toOperationalState t.toOperationalState := by
  cases body : environmentBody contracts s action with
  | none => simp [environment, body] at h
  | some next =>
      have kept : ReservationsRetained s.toOperationalState next.toOperationalState := by
        cases action <;> simp only [environmentBody, completeExternalEffect, responseWorld, putEffect] at body
        all_goals operational_success body
        all_goals first
          | exact reservations_retained_refl _
          | (apply reservations_retained_map; intro owner; split <;> rfl)
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact retained_edge_safe kept

theorem E06_transition_ownership_edge {contracts : Contracts} {s t : State}
    (h : Transition contracts s t) : OwnershipEdgeSafe contracts s.toOperationalState t.toOperationalState := by
  cases h with
  | operational executed =>
      exact step_operational_relation E06_domain_ownership_edge E06_local_ownership_edge executed
  | external executed => exact E06_environment_ownership_edge executed
  | refused => exact retained_edge_safe (reservations_retained_refl _)

/-- This applies to every actual edge of every Reachable execution. A tree or
Prepared/Claimed/Unknown/Partial effect retains the same owner, holder and reservation generation. -/
theorem E06_transition_cannot_release_or_transfer_early {contracts : Contracts} {s t : State}
    {owner : Reservation} (h : Transition contracts s t) (member : owner ∈ s.ownership)
    (blocked : 0 < owner.children ∨ ∃ effect ∈ s.effects, effect.intent.run = owner.owner ∧
      (effect.outcome = .prepared ∨ effect.outcome = .claimed ∨
        effect.outcome = .unknown ∨ effect.outcome = .partial)) :
    ∃ next ∈ t.ownership, ReservationIdentity next = ReservationIdentity owner := by
  rcases E06_transition_ownership_edge h owner member with retained | released
  · exact retained
  · exfalso
    rcases blocked with tree | ⟨effect, memberEffect, root, unresolved⟩
    · have zero := (released.2.1 owner member rfl).1
      omega
    · have outcomes := released.2.2 effect memberEffect root
      rcases unresolved with prepared | claimed | unknown | partialOutcome
      · exact outcomes.1 prepared
      · exact outcomes.2.1 claimed
      · exact outcomes.2.2.1 unknown
      · exact outcomes.2.2.2 partialOutcome

/-- Release ordering holds along the complete actual trace, without making an
initial empty-world assumption or granting recovery a capacity reset. -/
inductive OwnershipAudited (contracts : Contracts) (initial : State) : State → Prop where
  | initial : OwnershipAudited contracts initial initial
  | next {s t : State} : OwnershipAudited contracts initial s → Transition contracts s t →
      OwnershipEdgeSafe contracts s.toOperationalState t.toOperationalState → OwnershipAudited contracts initial t

theorem E06_reachable_ownership_audited {contracts : Contracts} {initial s : State}
    (h : Reachable contracts initial s) : OwnershipAudited contracts initial s := by
  induction h with
  | initial => exact OwnershipAudited.initial
  | next _ edge ih => exact OwnershipAudited.next ih edge (E06_transition_ownership_edge edge)

end Kctx.Engineering
