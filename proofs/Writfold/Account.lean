import Extracted.WritfoldCore.Account
import Extracted.WritfoldCore.Types

/-!
# An accounted universe (#181, law U)

Theorems over the Aeneas translation of `writfold_core::account`.

- `remainder_spec`: `remainder` is the number of members left over.
- `whole_iff_no_remainder`: nothing is left over exactly when no member
  is `Remainder`.
- `judged_never_counts`, `declared_out_never_counts`: such a member never
  adds to the remainder; `append_remainder`: the remainder of two parts
  adds up.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Account

/-- A member left over. -/
def leftB : account.Member → Bool
  | .Remainder => true
  | _ => false

/-- How many members are left over. -/
def leftCount (ms : List account.Member) : Nat := ms.countP leftB

theorem left_spec (m : account.Member) : account.left m ⦃ b => b = leftB m ⦄ := by
  unfold account.left; cases m <;> simp [leftB]

theorem countP_le (ms : List account.Member) : leftCount ms ≤ ms.length :=
  List.countP_le_length

theorem remainder_loop_spec (ms : Slice account.Member) (count index : Usize)
    (hi : index.val ≤ ms.val.length)
    (hc : count.val + leftCount (ms.val.drop index.val) = leftCount ms.val)
    (hb : count.val ≤ index.val) :
    account.remainder_loop ms count index ⦃ r => r.val = leftCount ms.val ⦄ := by
  unfold account.remainder_loop
  apply loop.spec_decr_nat (fun p : Usize × Usize => ms.val.length - p.2.val)
    (fun p : Usize × Usize => p.2.val ≤ ms.val.length ∧
      p.1.val + leftCount (ms.val.drop p.2.val) = leftCount ms.val ∧ p.1.val ≤ p.2.val)
  · rintro ⟨c, i⟩ ⟨hi, hc, hb⟩
    unfold account.remainder_loop.body
    dsimp only at hi hc hb ⊢
    split
    · rename_i hlt
      have hil : i.val < ms.val.length := by scalar_tac
      step as ⟨ m, hm ⟩
      step with left_spec as ⟨ b, hbm ⟩
      have hd : leftCount (ms.val.drop i.val) =
          (if leftB ms.val[i.val] then 1 else 0) + leftCount (ms.val.drop (i.val + 1)) := by
        rw [List.drop_eq_getElem_cons hil]
        unfold leftCount
        rw [List.countP_cons]
        cases leftB ms.val[i.val] <;> simp <;> omega
      split
      · rename_i hbt
        have hl : leftB ms.val[i.val] = true := by rw [← hm, ← hbm]; exact hbt
        step as ⟨ c', hc' ⟩
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac, by scalar_tac⟩
        rw [hd, hl] at hc
        simp at hc
        scalar_tac
      · rename_i hbt
        have hl : leftB ms.val[i.val] = false := by
          rw [← hm, ← hbm]; simpa using hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac, by scalar_tac⟩
        rw [hd, hl] at hc
        simp at hc
        scalar_tac
    · rename_i hge
      simp only [WP.spec_ok]
      rw [List.drop_of_length_le (by scalar_tac)] at hc
      simpa [leftCount] using hc
  · exact ⟨hi, hc, hb⟩

/-- `remainder` is the number of members left over. -/
theorem remainder_spec (ms : Slice account.Member) :
    account.remainder ms ⦃ r => r.val = leftCount ms.val ⦄ := by
  unfold account.remainder
  exact remainder_loop_spec ms 0#usize 0#usize (by simp) (by simp) (by simp)

/-- Nothing is left over exactly when no member is `Remainder`. -/
theorem whole_iff_no_remainder (ms : Slice account.Member) :
    account.whole ms ⦃ b => b = true ↔ account.Member.Remainder ∉ ms.val ⦄ := by
  unfold account.whole
  step with remainder_spec as ⟨ r, hr ⟩
  simp only [WP.spec_ok, decide_eq_true_eq]
  constructor
  · intro h hm
    have : r.val = 0 := by rw [h]; rfl
    rw [hr, leftCount, List.countP_eq_zero] at this
    exact absurd (this _ hm) (by simp [leftB])
  · intro h
    apply UScalar.eq_of_val_eq
    rw [hr]
    show List.countP leftB ms.val = 0
    rw [List.countP_eq_zero]
    intro m hm hl
    cases m <;> simp_all [leftB]

/-- A judged member never adds to the remainder. -/
theorem judged_never_counts (ms : List account.Member) :
    leftCount (.Judged :: ms) = leftCount ms := by simp [leftCount, leftB]

/-- A member declared out never adds to the remainder. -/
theorem declared_out_never_counts (ms : List account.Member) :
    leftCount (.DeclaredOut :: ms) = leftCount ms := by simp [leftCount, leftB]

/-- The remainder of two parts is the sum of theirs. -/
theorem append_remainder (xs ys : List account.Member) :
    leftCount (xs ++ ys) = leftCount xs + leftCount ys := by
  simp [leftCount, List.countP_append]

end Writfold.Core.Account
