import Extracted.WritfoldCore

/-!
# The order receipts are judged in

Theorems over the Aeneas translation of `writfold_core::order`.

- `less_spec`: `less` is the lexicographic order on bytes (`Lex`).
- `precedes_spec`: a receipt is judged before another exactly when its
  `seq` is lower, or equal with a file name that sorts first (`Prec`).
- `prec_irrefl`, `prec_trans`, `prec_total` (model lemmas, tied to the
  code by `precedes_spec`): a strict total order on `(seq, name)`, so a
  sort by it has one answer, whatever order the files were listed in, and
  no clock enters it.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Order

/-- Byte strings in lexicographic order: at the first place they differ,
the smaller byte; else the shorter. -/
inductive Lex : List U8 → List U8 → Prop
  | nil {b ys} : Lex [] (b :: ys)
  | lt {a b xs ys} : a.val < b.val → Lex (a :: xs) (b :: ys)
  | eq {a xs ys} : Lex xs ys → Lex (a :: xs) (a :: ys)

theorem lex_cons_iff {a b : U8} {xs ys : List U8} :
    Lex (a :: xs) (b :: ys) ↔ a.val < b.val ∨ (a = b ∧ Lex xs ys) := by
  constructor
  · intro h
    cases h with
    | lt h => exact Or.inl h
    | eq h => exact Or.inr ⟨rfl, h⟩
  · rintro (h | ⟨rfl, h⟩)
    · exact .lt h
    · exact .eq h

theorem lex_nil_right {xs : List U8} : ¬ Lex xs [] := by
  intro h; cases h

theorem lex_nil_left {ys : List U8} : Lex [] ys ↔ ys ≠ [] := by
  cases ys with
  | nil => simp; intro h; cases h
  | cons b ys => simp; exact .nil

theorem lex_irrefl (a : List U8) : ¬ Lex a a := by
  induction a with
  | nil => intro h; cases h
  | cons x xs ih =>
    intro h
    rcases lex_cons_iff.mp h with h | ⟨_, h⟩
    · omega
    · exact ih h

theorem lex_trans {a b c : List U8} (hab : Lex a b) (hbc : Lex b c) : Lex a c := by
  induction hab generalizing c with
  | nil => cases hbc <;> exact .nil
  | lt h =>
    cases hbc with
    | lt h' => exact .lt (by omega)
    | eq _ => exact .lt h
  | eq _ ih =>
    cases hbc with
    | lt h' => exact .lt h'
    | eq h' => exact .eq (ih h')

theorem lex_total (a b : List U8) : Lex a b ∨ a = b ∨ Lex b a := by
  induction a generalizing b with
  | nil => cases b with
    | nil => exact Or.inr (Or.inl rfl)
    | cons _ _ => exact Or.inl .nil
  | cons x xs ih =>
    cases b with
    | nil => exact Or.inr (Or.inr .nil)
    | cons y ys =>
      by_cases hxy : x.val < y.val
      · exact Or.inl (.lt hxy)
      · by_cases hyx : y.val < x.val
        · exact Or.inr (Or.inr (.lt hyx))
        · have : x = y := UScalar.eq_of_val_eq (by omega)
          subst this
          rcases ih ys with h | rfl | h
          · exact Or.inl (.eq h)
          · exact Or.inr (Or.inl rfl)
          · exact Or.inr (Or.inr (.eq h))

theorem less_loop_spec (a b : Slice U8) (i : Usize)
    (ha : i.val ≤ a.val.length) (hb : i.val ≤ b.val.length)
    (hlex : Lex a.val b.val ↔ Lex (a.val.drop i.val) (b.val.drop i.val)) :
    order.less_loop a b i ⦃ r => r = true ↔ Lex a.val b.val ⦄ := by
  unfold order.less_loop
  apply loop.spec_decr_nat (fun i : Usize => a.val.length - i.val)
    (fun i : Usize => i.val ≤ a.val.length ∧ i.val ≤ b.val.length ∧
      (Lex a.val b.val ↔ Lex (a.val.drop i.val) (b.val.drop i.val)))
  · intro i ⟨ha, hb, hlex⟩
    unfold order.less_loop.body
    dsimp only
    split
    · rename_i hla
      split
      · rename_i hlb
        step as ⟨ x, hx ⟩
        step as ⟨ y, hy ⟩
        have hda : a.val.drop i.val = x :: a.val.drop (i.val + 1) := by
          rw [hx]; exact List.drop_eq_getElem_cons (by scalar_tac)
        have hdb : b.val.drop i.val = y :: b.val.drop (i.val + 1) := by
          rw [hy]; exact List.drop_eq_getElem_cons (by scalar_tac)
        rw [hda, hdb, lex_cons_iff] at hlex
        split
        · rename_i hne
          simp only [WP.spec_ok, decide_eq_true_eq]
          rw [hlex]
          have : x ≠ y := by simpa using hne
          constructor
          · intro h; left; scalar_tac
          · rintro (h | ⟨h, _⟩)
            · scalar_tac
            · exact absurd h this
        · rename_i heq
          have : x = y := UScalar.eq_of_val_eq (by simpa using heq)
          step as ⟨ i5, hi5 ⟩
          refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
          rw [hlex, hi5]
          constructor
          · rintro (h | ⟨_, h⟩)
            · rw [this] at h; omega
            · exact h
          · intro h; exact Or.inr ⟨this, h⟩
      · rename_i hlb
        simp only [WP.spec_ok, decide_eq_true_eq]
        have hib : i.val = b.val.length := by scalar_tac
        rw [hlex, hib, List.drop_length]
        constructor
        · intro h; exfalso; scalar_tac
        · intro h; exact absurd h lex_nil_right
    · rename_i hla
      simp only [WP.spec_ok, decide_eq_true_eq]
      have hia : i.val = a.val.length := by scalar_tac
      rw [hlex, hia, List.drop_length, lex_nil_left]
      constructor
      · intro h e
        have := List.drop_eq_nil_iff.mp e
        scalar_tac
      · intro h
        by_contra hc
        apply h
        apply List.drop_eq_nil_of_le
        scalar_tac
  · exact ⟨ha, hb, hlex⟩

/-- `less` is the lexicographic order on bytes. -/
theorem less_spec (a b : Slice U8) :
    order.less a b ⦃ r => r = true ↔ Lex a.val b.val ⦄ := by
  unfold order.less
  exact less_loop_spec a b 0#usize (by simp) (by simp) (by simp)

/-- Judged before: the lower `seq`, then the file name. -/
def Prec (sa : U64) (na : List U8) (sb : U64) (nb : List U8) : Prop :=
  sa.val < sb.val ∨ (sa = sb ∧ Lex na nb)

theorem precedes_spec (sa : U64) (na : Slice U8) (sb : U64) (nb : Slice U8) :
    order.precedes sa na sb nb ⦃ r => r = true ↔ Prec sa na.val sb nb.val ⦄ := by
  unfold order.precedes
  split
  · rename_i hne
    have : sa ≠ sb := by simpa using hne
    simp only [WP.spec_ok, decide_eq_true_eq, Prec]
    constructor
    · intro h; left; scalar_tac
    · rintro (h | ⟨h, _⟩)
      · scalar_tac
      · exact absurd h this
  · rename_i heq
    have : sa = sb := UScalar.eq_of_val_eq (by simpa using heq)
    apply WP.spec_mono (less_spec na nb)
    intro r hr
    rw [hr, Prec]
    constructor
    · intro h; exact Or.inr ⟨this, h⟩
    · rintro (h | ⟨_, h⟩)
      · rw [this] at h; omega
      · exact h

theorem prec_irrefl (s : U64) (n : List U8) : ¬ Prec s n s n := by
  rintro (h | ⟨_, h⟩)
  · omega
  · exact lex_irrefl n h

theorem prec_trans {sa sb sc : U64} {na nb nc : List U8}
    (hab : Prec sa na sb nb) (hbc : Prec sb nb sc nc) : Prec sa na sc nc := by
  rcases hab with h1 | ⟨rfl, h1⟩ <;> rcases hbc with h2 | ⟨rfl, h2⟩
  · left; omega
  · left; exact h1
  · left; exact h2
  · right; exact ⟨rfl, lex_trans h1 h2⟩

theorem prec_total (sa sb : U64) (na nb : List U8) :
    Prec sa na sb nb ∨ (sa = sb ∧ na = nb) ∨ Prec sb nb sa na := by
  by_cases h1 : sa.val < sb.val
  · exact Or.inl (Or.inl h1)
  · by_cases h2 : sb.val < sa.val
    · exact Or.inr (Or.inr (Or.inl h2))
    · have : sa = sb := UScalar.eq_of_val_eq (by omega)
      subst this
      rcases lex_total na nb with h | rfl | h
      · exact Or.inl (Or.inr ⟨rfl, h⟩)
      · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
      · exact Or.inr (Or.inr (Or.inr ⟨rfl, h⟩))

end Writfold.Core.Order
