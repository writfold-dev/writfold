import Writfold.Get

/-!
# Project asset catalogs (#84)

Theorems over the Aeneas translation of `writfold_core::catalog`
(`docs/rfcs/2026-11-21-project-asset-catalogs.md`).

- `action_spec`: one slot's action is the RFC's table.
- `create_only_absent`, `replace_remove_only_owned`,
  `foreign_never_written`: a catalog creates only where nothing is,
  replaces or removes only its own records, and never writes a slot
  something else owns (a collision is a conflict, never a last-wins write).
- `owned_delta_exact`, `absent_unwanted_kept`: on its own slots, no row
  means remove, equal bytes mean keep, other bytes mean replace; nothing
  wanted where nothing is stays put.
- `any_conflict_spec`, `any_change_spec`: the folds see every slot.
- `clean_iff`: a plan is clean exactly when the input was read in full, no
  slot conflicts and no slot changes.
- `invalid_iff`, `partial_never_patches`: an incomplete input or a conflict
  anywhere makes the plan invalid, and an invalid plan emits no patch at
  all, not even the part that was valid.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Catalog

/-- `catalog::action` on the model. -/
def actionOf (s : catalog.Slot) : catalog.Action :=
  match s.owner with
  | .Absent => if s.wanted then .Create else .Keep
  | .Mine => if s.wanted then (if s.equal then .Keep else .Replace) else .Remove
  | .Foreign => if s.wanted then .Conflict else .Keep

def conflictB (s : catalog.Slot) : Bool :=
  match actionOf s with
  | .Conflict => true
  | _ => false

def changeB (s : catalog.Slot) : Bool :=
  match actionOf s with
  | .Create => true
  | .Replace => true
  | .Remove => true
  | _ => false

theorem action_spec (s : catalog.Slot) : catalog.action s ⦃ a => a = actionOf s ⦄ := by
  rcases s with ⟨w, o, e⟩
  unfold catalog.action
  cases o <;> cases w <;> cases e <;> simp [actionOf]

/-- A catalog creates a record only where nothing is. -/
theorem create_only_absent (s : catalog.Slot) :
    catalog.action s ⦃ a => a = .Create → s.wanted = true ∧ s.owner = .Absent ⦄ := by
  rcases s with ⟨w, o, e⟩
  unfold catalog.action
  cases o <;> cases w <;> cases e <;> simp

/-- A catalog replaces or removes only a record it generated. -/
theorem replace_remove_only_owned (s : catalog.Slot) :
    catalog.action s ⦃ a => (a = .Replace ∨ a = .Remove) → s.owner = .Mine ⦄ := by
  rcases s with ⟨w, o, e⟩
  unfold catalog.action
  cases o <;> cases w <;> cases e <;> simp

/-- A slot something else owns is never written: it is kept, or it is a
conflict when the catalog wants it. -/
theorem foreign_never_written (s : catalog.Slot) (h : s.owner = .Foreign) :
    catalog.action s ⦃ a => a = .Keep ∨ a = .Conflict ⦄ := by
  rcases s with ⟨w, o, e⟩
  simp only at h
  subst h
  unfold catalog.action
  cases w <;> simp

/-- On its own slots: no row means remove, equal bytes keep, other bytes
replace. -/
theorem owned_delta_exact (s : catalog.Slot) (h : s.owner = .Mine) :
    catalog.action s ⦃ a =>
      (s.wanted = false → a = .Remove) ∧
      (s.wanted = true → s.equal = true → a = .Keep) ∧
      (s.wanted = true → s.equal = false → a = .Replace) ⦄ := by
  rcases s with ⟨w, o, e⟩
  simp only at h
  subst h
  unfold catalog.action
  cases w <;> cases e <;> simp

/-- Nothing wanted where nothing is stays put. -/
theorem absent_unwanted_kept (s : catalog.Slot) (ho : s.owner = .Absent)
    (hw : s.wanted = false) : catalog.action s ⦃ a => a = .Keep ⦄ := by
  rcases s with ⟨w, o, e⟩
  simp only at ho hw
  subst ho hw
  unfold catalog.action
  simp

theorem conflicts_spec (s : catalog.Slot) :
    catalog.conflicts s ⦃ b => b = conflictB s ⦄ := by
  unfold catalog.conflicts
  step with action_spec as ⟨ a, ha ⟩
  subst ha
  unfold conflictB
  split <;> simp_all

theorem changes_spec (s : catalog.Slot) :
    catalog.changes s ⦃ b => b = changeB s ⦄ := by
  unfold catalog.changes
  step with action_spec as ⟨ a, ha ⟩
  subst ha
  unfold changeB
  split <;> simp_all

theorem any_conflict_loop_spec (xs : Slice catalog.Slot) (i : Usize)
    (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), conflictB xs.val[j] = false) :
    catalog.any_conflict_loop xs i ⦃ b => b = true ↔ ∃ s ∈ xs.val, conflictB s = true ⦄ := by
  unfold catalog.any_conflict_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), conflictB xs.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold catalog.any_conflict_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ s, hs ⟩
      step with conflicts_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨s, ?_, by rw [← hb]; exact hbt⟩
        rw [hs]; exact List.getElem_mem _
      · rename_i hbf
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          have : conflictB s = false := by rw [← hb]; simpa using hbf
          rw [hs] at this; exact this
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro s hmem
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hmem
      simp [hprefix j (by scalar_tac) hj]
  · exact ⟨hi, hprefix⟩

theorem any_conflict_spec (xs : Slice catalog.Slot) :
    catalog.any_conflict xs ⦃ b => b = true ↔ ∃ s ∈ xs.val, conflictB s = true ⦄ := by
  unfold catalog.any_conflict
  exact any_conflict_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem any_change_loop_spec (xs : Slice catalog.Slot) (i : Usize)
    (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), changeB xs.val[j] = false) :
    catalog.any_change_loop xs i ⦃ b => b = true ↔ ∃ s ∈ xs.val, changeB s = true ⦄ := by
  unfold catalog.any_change_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), changeB xs.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold catalog.any_change_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ s, hs ⟩
      step with changes_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨s, ?_, by rw [← hb]; exact hbt⟩
        rw [hs]; exact List.getElem_mem _
      · rename_i hbf
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          have : changeB s = false := by rw [← hb]; simpa using hbf
          rw [hs] at this; exact this
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro s hmem
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hmem
      simp [hprefix j (by scalar_tac) hj]
  · exact ⟨hi, hprefix⟩

theorem any_change_spec (xs : Slice catalog.Slot) :
    catalog.any_change xs ⦃ b => b = true ↔ ∃ s ∈ xs.val, changeB s = true ⦄ := by
  unfold catalog.any_change
  exact any_change_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

/-- `verdict` on the model. -/
def verdictOf (complete conflict changed : Bool) : catalog.Verdict :=
  if complete = false ∨ conflict = true then .Invalid
  else if changed then .Drift else .Clean

theorem verdict_spec (c k ch : Bool) :
    catalog.verdict c k ch ⦃ v => v = verdictOf c k ch ⦄ := by
  unfold catalog.verdict
  cases c <;> cases k <;> cases ch <;> simp [verdictOf]

/-- What a plan decides: the verdict over the whole input. -/
theorem plan_spec (c : Bool) (xs : Slice catalog.Slot) :
    catalog.plan c xs ⦃ v => v = verdictOf c
      (decide (∃ s ∈ xs.val, conflictB s = true))
      (decide (∃ s ∈ xs.val, changeB s = true)) ⦄ := by
  unfold catalog.plan
  step with any_conflict_spec as ⟨ k, hk ⟩
  step with any_change_spec as ⟨ ch, hch ⟩
  apply WP.spec_mono (verdict_spec c k ch)
  intro v hv
  rw [hv]
  have e1 : k = decide (∃ s ∈ xs.val, conflictB s = true) := by
    cases k <;> simp_all
  have e2 : ch = decide (∃ s ∈ xs.val, changeB s = true) := by
    cases ch <;> simp_all
  rw [e1, e2]

/-- Clean exactly when the input was read in full, nothing conflicts and
nothing would change. -/
theorem clean_iff (c : Bool) (xs : Slice catalog.Slot) :
    catalog.plan c xs ⦃ v => v = .Clean ↔ c = true ∧
      (∀ s ∈ xs.val, conflictB s = false) ∧ (∀ s ∈ xs.val, changeB s = false) ⦄ := by
  apply WP.spec_mono (plan_spec c xs)
  intro v hv
  rw [hv]
  generalize hk : decide (∃ s ∈ xs.val, conflictB s = true) = k
  generalize hch : decide (∃ s ∈ xs.val, changeB s = true) = ch
  have hk' : (∃ s ∈ xs.val, conflictB s = true) ↔ k = true := by rw [← hk]; simp
  have hch' : (∃ s ∈ xs.val, changeB s = true) ↔ ch = true := by rw [← hch]; simp
  cases c <;> cases k <;> cases ch <;> simp only [verdictOf] at * <;> simp_all

/-- Invalid exactly when the input was not read in full or some slot
conflicts. -/
theorem invalid_iff (c : Bool) (xs : Slice catalog.Slot) :
    catalog.plan c xs ⦃ v => v = .Invalid ↔ c = false ∨ ∃ s ∈ xs.val, conflictB s = true ⦄ := by
  apply WP.spec_mono (plan_spec c xs)
  intro v hv
  rw [hv]
  generalize hk : decide (∃ s ∈ xs.val, conflictB s = true) = k
  generalize hch : decide (∃ s ∈ xs.val, changeB s = true) = ch
  have hk' : (∃ s ∈ xs.val, conflictB s = true) ↔ k = true := by rw [← hk]; simp
  have hch' : (∃ s ∈ xs.val, changeB s = true) ↔ ch = true := by rw [← hch]; simp
  cases c <;> cases k <;> cases ch <;> simp only [verdictOf] at * <;> simp_all

theorem patches_spec (v : catalog.Verdict) :
    catalog.patches v ⦃ p => p = true ↔ v ≠ .Invalid ⦄ := by
  unfold catalog.patches
  cases v <;> simp

/-- An incomplete input, or a conflict anywhere, emits no patch at all. -/
theorem partial_never_patches (c : Bool) (xs : Slice catalog.Slot)
    (h : c = false ∨ ∃ s ∈ xs.val, conflictB s = true) :
    (do let v ← catalog.plan c xs; catalog.patches v) ⦃ p => p = false ⦄ := by
  step with invalid_iff as ⟨ v, hv ⟩
  have hinv : v = .Invalid := hv.mpr h
  subst hinv
  unfold catalog.patches
  simp

/-! ## Totality (tauten claims) -/

theorem any_conflict_total (xs : Slice catalog.Slot) : catalog.any_conflict xs ⦃ _ => True ⦄ :=
  WP.spec_mono (any_conflict_spec xs) (fun _ _ => trivial)

theorem any_change_total (xs : Slice catalog.Slot) : catalog.any_change xs ⦃ _ => True ⦄ :=
  WP.spec_mono (any_change_spec xs) (fun _ _ => trivial)

theorem plan_total (c : Bool) (xs : Slice catalog.Slot) : catalog.plan c xs ⦃ _ => True ⦄ :=
  WP.spec_mono (plan_spec c xs) (fun _ _ => trivial)

end Writfold.Core.Catalog
