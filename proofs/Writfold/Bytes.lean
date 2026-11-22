import Extracted.WritfoldCore.Bytes

/-!
# Byte identities

Theorems over the Aeneas translation of `writfold_core::bytes`: `same` is
exact equality of byte strings. Every theorem is a total-correctness Hoare
triple `f args ⦃ r => P r ⦄`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core

/-- The loop of `same` from position `i`, given equal lengths and an equal prefix. -/
theorem same_loop_spec (a b : Slice U8) (i : Usize)
    (hlen : a.val.length = b.val.length) (hi : i.val ≤ a.val.length)
    (hprefix : ∀ j, j < i.val → a.val[j]? = b.val[j]?) :
    bytes.same_loop a b i ⦃ r => r ↔ a.val = b.val ⦄ := by
  unfold bytes.same_loop
  apply loop.spec_decr_nat (fun i : Usize => a.val.length - i.val)
    (fun i : Usize => i.val ≤ a.val.length ∧ ∀ j, j < i.val → a.val[j]? = b.val[j]?)
  · intro i ⟨hi, hprefix⟩
    unfold bytes.same_loop.body
    dsimp only
    split_ifs with hlt
    · step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      split_ifs with hne
      · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        intro heq
        rw [bne_iff_ne] at hne
        apply hne
        rw [hx, hy]
        simp only [heq]
      · rw [bne_iff_ne, not_not] at hne
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro j hj
        by_cases hji : j < i.val
        · exact hprefix j hji
        · have : j = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac), List.getElem?_eq_getElem (by scalar_tac),
            ← hx, ← hy, hne]
    · simp only [WP.spec_ok, true_iff]
      apply List.ext_getElem?
      intro j
      by_cases hj : j < a.val.length
      · exact hprefix j (by scalar_tac)
      · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]
  · exact ⟨hi, hprefix⟩

/-- `same` is exact equality of the two byte strings. -/
theorem same_spec (a b : Slice U8) : bytes.same a b ⦃ r => r ↔ a.val = b.val ⦄ := by
  unfold bytes.same
  dsimp only
  split_ifs with hne
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro heq
    rw [bne_iff_ne] at hne
    apply hne
    simp [Slice.len, heq]
  · rw [bne_iff_ne, not_not] at hne
    apply same_loop_spec
    · have h := congrArg (fun x : Usize => x.val) hne
      simpa [Slice.len] using h
    · simp
    · intro j hj; simp at hj

end Writfold.Core
