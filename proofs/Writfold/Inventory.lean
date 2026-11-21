import Writfold.Policy
import Writfold.Guard

/-!
# Policy inventories (#56)

Theorems over the Aeneas translation of `writfold_core::inventory`: a
policy inventory read from several roots is complete exactly when no root
was unreadable, and no aggregate answer is positive over an incomplete one.

- `readable_spec`, `complete_spec`: complete iff every root is `Absent` or
  `Listed`.
- `unreadable_pos_iff`: some root is counted as a problem iff the
  inventory is incomplete. Each counted root is one invalid definition, so
  with `gates_fact_met_iff` (met needs `invalid = 0`) the `gates.met` fact
  is never met over an incomplete inventory (`gates_unmet_when_incomplete`).
- `lines_clean_over_iff`: planned `gate lines` is clean iff the inventory is
  complete and `lines_clean` holds (a complete inventory keeps today's
  answer; `lines_clean_over_incomplete`).
- `all_true_spec`, `unplanned_clean_iff`, `unplanned_clean_empty`: an
  unplanned run's gates are clean iff the inventory is complete, some gate
  is judged, every one is met and nothing is invalid (the backfill of the
  former `Report::all_met() && invalid == 0`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Inventory

/-- A root that was read in full or is legitimately absent. -/
def readableB : inventory.Root → Bool
  | .Unreadable => false
  | _ => true

theorem readable_spec (r : inventory.Root) :
    inventory.readable r ⦃ b => b = readableB r ⦄ := by
  unfold inventory.readable; cases r <;> simp [readableB]

theorem readableB_iff (r : inventory.Root) : readableB r = true ↔ r ≠ .Unreadable := by
  cases r <;> simp [readableB]

theorem complete_loop_spec (roots : Slice inventory.Root) (i : Usize)
    (hi : i.val ≤ roots.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < roots.val.length),
      readableB roots.val[j] = true) :
    inventory.complete_loop roots i ⦃ b => b = true ↔ ∀ r ∈ roots.val, readableB r = true ⦄ := by
  unfold inventory.complete_loop
  apply loop.spec_decr_nat (fun i : Usize => roots.val.length - i.val)
    (fun i : Usize => i.val ≤ roots.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < roots.val.length), readableB roots.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold inventory.complete_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hmem : roots.val[i.val]'(by scalar_tac) ∈ roots.val := List.getElem_mem _
      step with readable_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx, ← hb]; exact hbt
      · rename_i hbf
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
        refine ⟨_, hmem, ?_⟩
        rw [← hx, ← hb]; simpa using hbf
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro r hr
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hr
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- An inventory is complete exactly when no root is unreadable. -/
theorem complete_spec (roots : Slice inventory.Root) :
    inventory.complete roots ⦃ b => b = true ↔ ∀ r ∈ roots.val, r ≠ .Unreadable ⦄ := by
  unfold inventory.complete
  apply WP.spec_mono (complete_loop_spec roots 0#usize (by simp) (by intro j hj; simp at hj))
  intro b hb
  rw [hb]
  simp [readableB_iff]

theorem unreadable_loop_spec (roots : Slice inventory.Root) (i count : Usize)
    (hi : i.val ≤ roots.val.length) (hc : count.val ≤ i.val)
    (hprefix : 0 < count.val ↔ ∃ j, ∃ (hj : j < i.val) (hj' : j < roots.val.length),
      roots.val[j] = .Unreadable) :
    inventory.unreadable_loop roots i count ⦃ n =>
      0 < n.val ↔ ∃ r ∈ roots.val, r = .Unreadable ⦄ := by
  unfold inventory.unreadable_loop
  apply loop.spec_decr_nat (fun p : Usize × Usize => roots.val.length - p.1.val)
    (fun p : Usize × Usize => p.1.val ≤ roots.val.length ∧ p.2.val ≤ p.1.val ∧
      (0 < p.2.val ↔ ∃ j, ∃ (hj : j < p.1.val) (hj' : j < roots.val.length),
        roots.val[j] = .Unreadable))
  · intro ⟨i, count⟩ ⟨hi, hc, hprefix⟩
    unfold inventory.unreadable_loop.body
    dsimp only at hi hc hprefix ⊢
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with readable_spec as ⟨ b, hb ⟩
      have hval : x = roots.val[i.val]'(by scalar_tac) := by rw [← hx]
      split
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [hprefix]
        constructor
        · rintro ⟨j, hj, hj', h⟩; exact ⟨j, by scalar_tac, hj', h⟩
        · rintro ⟨j, hj, hj', h⟩
          by_cases hji : j < i.val
          · exact ⟨j, hji, hj', h⟩
          · have : j = i.val := by scalar_tac
            subst this
            have : readableB x = true := by rw [← hb]; exact hbt
            rw [hval, h] at this; simp [readableB] at this
      · rename_i hbf
        step as ⟨ c', hc' ⟩
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have hx' : roots.val[i.val]'(by scalar_tac) = .Unreadable := by
          have : readableB x = false := by rw [← hb]; simpa using hbf
          rw [hval] at this
          cases h : roots.val[i.val]'(by scalar_tac) <;> simp_all [readableB]
        exact ⟨fun _ => ⟨i.val, by scalar_tac, by scalar_tac, hx'⟩, fun _ => by scalar_tac⟩
    · rename_i hge
      simp only [WP.spec_ok]
      rw [hprefix]
      constructor
      · rintro ⟨j, hj, hj', h⟩; exact ⟨_, List.getElem_mem hj', h⟩
      · rintro ⟨r, hr, h⟩
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hr
        exact ⟨j, by scalar_tac, hj, h⟩
  · exact ⟨hi, hc, hprefix⟩

/-- Some root is counted as a problem exactly when one is unreadable. -/
theorem unreadable_pos_iff (roots : Slice inventory.Root) :
    inventory.unreadable roots ⦃ n => 0 < n.val ↔ ∃ r ∈ roots.val, r = .Unreadable ⦄ := by
  unfold inventory.unreadable
  exact unreadable_loop_spec roots 0#usize 0#usize (by simp) (by simp) (by simp)

/-- Over an incomplete inventory, the `gates.met` fact counting each
unreadable root as an invalid definition is never met. -/
theorem gates_unmet_when_incomplete (o u i n : Usize) (hn : 0 < n.val) (h : i.val + n.val ≤ Usize.max) :
    (do let total ← i + n; guard.gates_fact o u total) ⦃ f => f = .Unmet ⦄ := by
  step as ⟨ total, htotal ⟩
  unfold guard.gates_fact
  have hz : ¬ total = 0#usize := by intro h0; have : total.val = 0 := by simp [h0]
                                    scalar_tac
  repeat' split
  all_goals simp_all

/-- Planned `gate lines` is clean exactly when the inventory is complete and
`lines_clean` holds. -/
theorem lines_clean_over_iff (c : Bool) (s : policy.Summary) (xs : Slice policy.lines.Listed)
    (v : Bool) :
    inventory.lines_clean_over c s xs v ⦃ b => b = true ↔
      c = true ∧ (s = .Complete ∨ s = .CompleteWithExceptions) ∧
        (∀ l ∈ xs.val, Policy.Owed l) ∧ v = true ⦄ := by
  unfold inventory.lines_clean_over
  cases c
  · simp
  · simp only [true_and, ite_true]
    exact Policy.lines_clean_iff s xs v

/-- An incomplete inventory never gives clean planned lines. -/
theorem lines_clean_over_incomplete (s : policy.Summary) (xs : Slice policy.lines.Listed)
    (v : Bool) : inventory.lines_clean_over false s xs v ⦃ b => b = false ⦄ := by
  unfold inventory.lines_clean_over; simp

theorem all_true_loop_spec (met : Slice Bool) (i : Usize)
    (hi : i.val ≤ met.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < met.val.length), met.val[j] = true) :
    inventory.all_true_loop met i ⦃ b => b = true ↔ ∀ m ∈ met.val, m = true ⦄ := by
  unfold inventory.all_true_loop
  apply loop.spec_decr_nat (fun i : Usize => met.val.length - i.val)
    (fun i : Usize => i.val ≤ met.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < met.val.length), met.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold inventory.all_true_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hmem : met.val[i.val]'(by scalar_tac) ∈ met.val := List.getElem_mem _
      split
      · rename_i hxt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; exact hxt
      · rename_i hxf
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
        exact ⟨_, hmem, by rw [← hx]; simpa using hxf⟩
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro m hm
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hm
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

theorem all_true_spec (met : Slice Bool) :
    inventory.all_true met ⦃ b => b = true ↔ ∀ m ∈ met.val, m = true ⦄ := by
  unfold inventory.all_true
  exact all_true_loop_spec met 0#usize (by simp) (by intro j hj; simp at hj)

/-- An unplanned run's gates are clean exactly when the inventory is
complete, some gate is judged, every one is met and nothing is invalid. -/
theorem unplanned_clean_iff (c : Bool) (met : Slice Bool) (v : Bool) :
    inventory.unplanned_clean c met v ⦃ b => b = true ↔
      c = true ∧ met.val ≠ [] ∧ (∀ m ∈ met.val, m = true) ∧ v = true ⦄ := by
  unfold inventory.unplanned_clean
  cases c
  · simp
  · simp only [ite_true, true_and]
    step as ⟨ e, he ⟩
    split
    · rename_i het
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
      intro hne; exact absurd (by simpa using he.mp het) hne
    · rename_i hef
      step with all_true_spec as ⟨ a, ha ⟩
      have hne : met.val ≠ [] := by
        intro h; exact hef (he.mpr (by simp [h]))
      split
      · rename_i hat
        simp only [WP.spec_ok]
        exact ⟨fun h => ⟨hne, ha.mp hat, h⟩, fun h => h.2.2⟩
      · rename_i haf
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
        exact fun _ h => absurd (ha.mpr h) haf

/-- No gate judged is never clean. -/
theorem unplanned_clean_empty (c : Bool) (met : Slice Bool) (v : Bool) (h : met.val = []) :
    inventory.unplanned_clean c met v ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (unplanned_clean_iff c met v)
  intro b hb
  cases b
  · rfl
  · exact absurd (hb.mp rfl).2.1 (by simp [h])

/-! ## Totality (tauten claims) -/

theorem complete_total (roots : Slice inventory.Root) :
    inventory.complete roots ⦃ _ => True ⦄ :=
  WP.spec_mono (complete_spec roots) (fun _ _ => trivial)

theorem unreadable_total (roots : Slice inventory.Root) :
    inventory.unreadable roots ⦃ _ => True ⦄ :=
  WP.spec_mono (unreadable_pos_iff roots) (fun _ _ => trivial)

theorem all_true_total (met : Slice Bool) :
    inventory.all_true met ⦃ _ => True ⦄ :=
  WP.spec_mono (all_true_spec met) (fun _ _ => trivial)

theorem unplanned_clean_total (c : Bool) (met : Slice Bool) (v : Bool) :
    inventory.unplanned_clean c met v ⦃ _ => True ⦄ :=
  WP.spec_mono (unplanned_clean_iff c met v) (fun _ _ => trivial)

end Writfold.Core.Inventory
