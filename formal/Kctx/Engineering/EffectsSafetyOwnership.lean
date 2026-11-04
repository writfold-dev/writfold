import Kctx.Engineering.EffectsSafetyStep
import Kctx.Engineering.OperationalProofsLists

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 1600000
namespace Kctx.Engineering

private theorem nodup_append_fresh {α : Type} {xs ys : List α}
    (left : xs.Nodup) (right : ys.Nodup) (fresh : ∀ x ∈ xs, x ∉ ys) :
    (xs ++ ys).Nodup := by
  induction xs with
  | nil => exact right
  | cons x xs ih =>
      obtain ⟨absent, rest⟩ := List.nodup_cons.mp left
      apply List.nodup_cons.mpr
      refine ⟨?_, ih rest (fun a member => fresh a (List.mem_cons_of_mem _ member))⟩
      intro member
      rcases List.mem_append.mp member with member | member
      · exact absent member
      · exact fresh x (by simp) member

theorem reserve_preserves_ownership_unique {s t : State} {run : ActivationId}
    {keys : List (Id .resource)} {generation : Nat}
    (before : (s.ownership.map Reservation.resource).Nodup)
    (h : reserve s run keys generation = some t) :
    (t.ownership.map Reservation.resource).Nodup := by
  cases found : findRun s run with
  | none => simp [reserve, found] at h
  | some owner =>
      cases grantFound : currentGrant s owner with
      | none => simp [reserve, found, grantFound] at h
      | some grant =>
          simp only [reserve, found, grantFound, bind, Option.bind] at h
          split at h
          next guard =>
            cases h
            have checked : live s owner = true ∧ keys.isEmpty = false ∧ unique keys = true ∧
                subset keys grant.resources = true ∧
                ∀ key ∈ keys, s.ownership.any (·.resource == key) = false ∧
                  s.reservationGenerations.all (fun old => old.1 != key || old.2 < generation) = true := by
              simpa only [Bool.and_eq_true, Bool.not_eq_true', List.all_eq_true, and_assoc] using guard
            simp only [List.map_append, List.map_map, Function.comp_def, List.map_id_fun']
            change (keys ++ s.ownership.map Reservation.resource).Nodup
            apply nodup_append_fresh (unique_sound checked.2.2.1) before
            intro key member old
            obtain ⟨reservation, owned, same⟩ := List.mem_map.mp old
            have absent := (checked.2.2.2.2 key member).1
            have present : s.ownership.any (·.resource == key) = true := by
              apply List.any_eq_true.mpr
              exact ⟨reservation, owned, by simpa only [beq_iff_eq] using same⟩
            rw [present] at absent
            contradiction
          next => contradiction

private theorem nodup_keys_injective {α β : Type} {key : α → β} {xs : List α}
    (uniqueKeys : (xs.map key).Nodup) {a b : α} (left : a ∈ xs) (right : b ∈ xs)
    (same : key a = key b) : a = b := by
  induction xs generalizing a b with
  | nil => simp at left
  | cons x xs ih =>
      obtain ⟨absent, rest⟩ := List.nodup_cons.mp uniqueKeys
      rcases List.mem_cons.mp left with rfl | leftTail
      · rcases List.mem_cons.mp right with rfl | rightTail
        · rfl
        · exact False.elim (absent (same.symm ▸ List.mem_map.mpr ⟨b, rightTail, rfl⟩))
      · rcases List.mem_cons.mp right with rfl | rightTail
        · exact False.elim (absent (same ▸ List.mem_map.mpr ⟨a, leftTail, rfl⟩))
        · exact ih rest leftTail rightTail same

/-- Conflicting ownership is impossible: equality of the resource also fixes
owner, activation holder, reservation generation and process generation. -/
theorem E06_exclusive_conflicting_ownership {contracts : Contracts} {s : State}
    (wellFormed : OperationalWellFormed contracts s.toOperationalState) {left right : Reservation}
    (leftOwned : left ∈ s.ownership) (rightOwned : right ∈ s.ownership)
    (same : left.resource = right.resource) : left = right :=
  nodup_keys_injective wellFormed.ownership_unique leftOwned rightOwned same

/-- Release rechecks both generations and every unresolved effect, rather than
interpreting a cancellation or timeout as process teardown. -/
def ReleaseChecked (contracts : Contracts) (s : OperationalState) (root : RunId) : Prop :=
  (∀ run ∈ s.runs, run.id = root → run.parent.isSome = true → run.status ≠ .active) ∧
  (∀ owner ∈ s.ownership, owner.owner = root → owner.children = 0 ∧
    (owner.resource, owner.generation, owner.processGeneration) ∈ contracts.quiescent) ∧
  (∀ effect ∈ s.effects, effect.intent.run = root →
    effect.outcome ≠ .prepared ∧ effect.outcome ≠ .claimed ∧
      effect.outcome ≠ .unknown ∧ effect.outcome ≠ .partial)

theorem E06_quiesce_release_checked {contracts : Contracts} {s t : State} {root : RunId}
    (h : quiesce contracts s root = some t) : ReleaseChecked contracts s.toOperationalState root ∧
      t.ownership = s.ownership.filter (·.owner != root) := by
  unfold quiesce at h
  split at h
  next guard =>
    cases h
    have parts :
        (∀ run ∈ s.runs, ¬ (run.id = root ∧ run.parent.isSome = true ∧ run.status = .active)) ∧
        (∀ owner ∈ s.ownership, owner.owner ≠ root ∨
          (owner.children = 0 ∧ (owner.resource, owner.generation, owner.processGeneration) ∈ contracts.quiescent)) ∧
        (∀ effect ∈ s.effects, ¬ (effect.intent.run = root ∧
          (effect.outcome = .prepared ∨ effect.outcome = .claimed ∨
            effect.outcome = .unknown ∨ effect.outcome = .partial))) := by
      simpa only [unsettled, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', List.any_eq_false,
        List.all_eq_true, List.contains_iff, beq_iff_eq, bne_iff_ne, and_assoc,
        or_assoc] using guard
    refine ⟨⟨?_, ?_, ?_⟩, rfl⟩
    · intro run member rootOf parent active
      exact parts.1 run member ⟨rootOf, parent, active⟩
    · intro owner member rootOf
      rcases parts.2.1 owner member with other | checked
      · exact False.elim (other rootOf)
      · exact checked
    · intro effect member rootOf
      have blocked := parts.2.2 effect member
      exact ⟨fun prepared => blocked ⟨rootOf, Or.inl prepared⟩,
        fun claimed => blocked ⟨rootOf, Or.inr (Or.inl claimed)⟩,
        fun unknown => blocked ⟨rootOf, Or.inr (Or.inr (Or.inl unknown))⟩,
        fun partialOutcome => blocked ⟨rootOf, Or.inr (Or.inr (Or.inr partialOutcome))⟩⟩
  next => contradiction

theorem E06_tree_prevents_release {contracts : Contracts} {s : State} {root : RunId}
    {owner : Reservation} (member : owner ∈ s.ownership) (owned : owner.owner = root)
    (tree : 0 < owner.children) : quiesce contracts s root = none := by
  cases result : quiesce contracts s root with
  | none => rfl
  | some next =>
      have zero := ((E06_quiesce_release_checked result).1.2.1 owner member owned).1
      omega

/-- An unclaimed preparation is still a globally conflicting intent. It must be
cancelled before quiescence can release its reservations for a different owner. -/
theorem E06_prepared_prevents_release {contracts : Contracts} {s : State} {root : RunId}
    {effect : Effect} (member : effect ∈ s.effects) (owned : effect.intent.run = root)
    (prepared : effect.outcome = .prepared) : quiesce contracts s root = none := by
  cases result : quiesce contracts s root with
  | none => rfl
  | some next =>
      have settled := (E06_quiesce_release_checked result).1.2.2 effect member owned
      exact False.elim (settled.1 prepared)

theorem E06_unknown_prevents_release {contracts : Contracts} {s : State} {root : RunId}
    {effect : Effect} (member : effect ∈ s.effects) (owned : effect.intent.run = root)
    (unknown : effect.outcome = .claimed ∨ effect.outcome = .unknown ∨ effect.outcome = .partial) :
    quiesce contracts s root = none := by
  cases result : quiesce contracts s root with
  | none => rfl
  | some next =>
      have settled := (E06_quiesce_release_checked result).1.2.2 effect member owned
      rcases unknown with claimed | unknownOutcome | partialOutcome
      · exact False.elim (settled.2.1 claimed)
      · exact False.elim (settled.2.2.1 unknownOutcome)
      · exact False.elim (settled.2.2.2 partialOutcome)

theorem E06_cancel_preserves_capacity {s t : State} {run : ActivationId}
    (h : cancel s run = some t) :
    t.ownership = s.ownership ∧ t.reservationGenerations = s.reservationGenerations ∧
      t.budgets = s.budgets := by
  simp only [cancel, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      exact ⟨rfl, rfl, rfl⟩
    · contradiction

theorem E06_crash_preserves_capacity (s : State) :
    (crash s).ownership = s.ownership ∧ (crash s).reservationGenerations = s.reservationGenerations ∧
      (crash s).budgets = s.budgets := ⟨rfl, rfl, rfl⟩

theorem E06_recovery_preserves_capacity {s t : State} (h : recover s = some t) :
    t.ownership = s.ownership ∧ t.reservationGenerations = s.reservationGenerations ∧
      t.budgets = s.budgets := by
  unfold recover at h
  split at h
  · cases h
    exact ⟨rfl, rfl, rfl⟩
  · contradiction

theorem E06_rebind_requires_released_ownership {contracts : Contracts} {s t : State}
    {run invocation : ActivationId} {procedure : Procedure} {binding : Binding}
    (h : rebind contracts s run procedure binding invocation = some t) :
    ∃ old, findRun s run = some old ∧ ∀ reservation ∈ s.ownership, reservation.owner ≠ old.id := by
  simp only [rebind, bind, Option.bind] at h
  repeat first
    | contradiction
    | (split at h <;> try dsimp only at h)
    | (cases h; simp_all [Bool.and_eq_true, Bool.not_eq_true', List.any_eq_false,
        beq_iff_eq])

/-- Released resources cannot reuse an ownership generation even when the old
reservation is no longer in the live ownership list. The tombstone is retained. -/
theorem E06_reservation_generation_fresh {s t : State} {run : ActivationId}
    {keys : List (Id .resource)} {generation : Nat}
    (h : reserve s run keys generation = some t) {key : Id .resource} {oldGeneration : Nat}
    (requested : key ∈ keys) (previous : (key, oldGeneration) ∈ s.reservationGenerations) :
    oldGeneration < generation := by
  simp only [reserve, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · contradiction
    · split at h <;> try dsimp only at h
      next guard =>
        have ready : keys.all (fun key => !s.ownership.any (·.resource == key) &&
            s.reservationGenerations.all (fun old => old.1 != key || old.2 < generation)) = true := by
          simp only [Bool.and_eq_true] at guard
          exact guard.2
        have keyReady := List.all_eq_true.mp ready key requested
        simp only [Bool.and_eq_true] at keyReady
        have historyReady := keyReady.2
        have newer := List.all_eq_true.mp historyReady (key, oldGeneration) previous
        simpa using newer
      next => contradiction

end Kctx.Engineering
