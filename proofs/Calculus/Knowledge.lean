import Calculus.Word
import Calculus.Referent
/-!
# Knowing more (calculus.md 3.5, laws A and R; #212)

A vocabulary of verdicts (`Words`) is ordered by knowledge exactly as
`Tri` is: the unknown word is below every word, and each other word only
below itself. A judgment that is monotone from an order on facts into this
order never contradicts itself when it learns more: a verdict changes only
where it was the unknown word. Three shipped refinement theorems (more
history, fetched bytes, a referent that is not behind) are instances.
-/
namespace Calculus

variable {W : Type} (ws : Words W)

/-- `v` refines to `w`: they are the same word, or `v` could not be told. -/
def Words.refines (v w : W) : Prop := v = w ∨ v = ws.unknown

/-- A word that was told: anything but the unknown word. -/
def Words.told (w : W) : Prop := w ≠ ws.unknown

/-- Every word refines to itself. -/
theorem Words.refines_refl (w : W) : ws.refines w w := Or.inl rfl

/-- Refinement composes: learning twice is learning once. -/
theorem Words.refines_trans {u v w : W} (h1 : ws.refines u v) (h2 : ws.refines v w) :
    ws.refines u w := by
  unfold Words.refines at *
  rcases h1 with rfl | h1
  · exact h2
  · exact Or.inr h1

/-- The unknown word refines to every word. -/
theorem Words.unknown_refines (w : W) : ws.refines ws.unknown w := Or.inr rfl

/-- Only the unknown word refines to the unknown word: learning more never
takes a told verdict back. -/
theorem Words.refines_unknown_iff (v : W) : ws.refines v ws.unknown ↔ v = ws.unknown := by
  unfold Words.refines; constructor
  · rintro (h | h) <;> exact h
  · exact Or.inr

/-- A told word is final: what refines it is itself. -/
theorem Words.told_refines {v w : W} (ht : ws.told v) (h : ws.refines v w) : v = w := by
  unfold Words.refines at h; unfold Words.told at ht
  rcases h with h | h
  · exact h
  · exact absurd h ht

/-- The unknown word is never told. -/
theorem Words.unknown_never_told : ¬ ws.told ws.unknown := fun h => h rfl

/-- Two fuller observations that disagree leave the lesser one unknown: a
fact that two different outcomes both refine decided nothing. -/
theorem Words.refines_disagree {v w₁ w₂ : W} (h₁ : ws.refines v w₁) (h₂ : ws.refines v w₂)
    (hne : w₁ ≠ w₂) : v = ws.unknown := by
  unfold Words.refines at h₁ h₂
  rcases h₁ with rfl | h₁
  · rcases h₂ with rfl | h₂
    · exact absurd rfl hne
    · exact h₂
  · exact h₁

/-- `Tri` as a vocabulary: refinement is the knowledge order. -/
theorem Tri.words_refines_iff (a b : Tri) : Tri.words.refines a b ↔ a ≤ b := by
  unfold Words.refines
  cases a <;> cases b <;> simp [Tri.words] <;> decide

/-- Law A, monotone: a referent known to be resolved and refutable, and a
verdict that refines, give a settled verdict that refines. -/
theorem settle_monotone (r f r' f' : Bool) {v w : W} (hr : r = true → r' = true)
    (hf : f = true → f' = true) (h : ws.refines v w) :
    ws.refines (settle ws r f v) (settle ws r' f' w) := by
  unfold Words.refines at *
  rcases h with rfl | rfl
  · rcases settle_refines ws r f r' f' v hr hf with h | h
    · exact Or.inl h
    · exact Or.inr h
  · right; unfold settle
    cases r <;> cases f <;> simp [ws.unknown_claims_nothing]

end Calculus
