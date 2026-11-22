import Extracted.WritfoldCore.Command
import Writfold.Bytes

/-!
# Which command a guard rule names

Theorems over the Aeneas translation of `writfold_core::command::named`
(`command::matches` passes its words' bytes).

- `is_help_spec`: a word is `--help` exactly when its bytes are.
- `named_spec`: a rule's prefix names a command exactly when each prefix
  word is the command's word at the same place and the command is not the
  prefix followed by `--help` alone.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Command

/-- The bytes of `--help`. -/
def HELP : List U8 := [45#u8, 45#u8, 104#u8, 101#u8, 108#u8, 112#u8]

theorem is_help_spec (w : Slice U8) :
    command.is_help w ⦃ b => b = true ↔ w.val = HELP ⦄ := by
  unfold command.is_help
  dsimp only
  split
  · rename_i hlen
    have hl : w.val.length = 6 := by scalar_tac
    generalize hv : w.val = l at hl
    rcases l with _ | ⟨b0, _ | ⟨b1, _ | ⟨b2, _ | ⟨b3, _ | ⟨b4, _ | ⟨b5, _ | ⟨b6, l⟩⟩⟩⟩⟩⟩⟩ <;>
      simp at hl
    step as ⟨ a, ha ⟩
    split
    · step as ⟨ b, hb ⟩
      split
      · step as ⟨ c, hc ⟩
        split
        · step as ⟨ d, hd ⟩
          split
          · step as ⟨ e, he ⟩
            split
            · step as ⟨ f, hf ⟩
              simp only [WP.spec_ok, decide_eq_true_eq, HELP]
              simp_all
            all_goals simp_all [HELP]
          all_goals simp_all [HELP]
        all_goals simp_all [HELP]
      all_goals simp_all [HELP]
    all_goals simp_all [HELP]
  · rename_i hlen
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro he; apply hlen
    have : w.val.length = 6 := by simp [he, HELP]
    exact UScalar.eq_of_val_eq (by simpa [Slice.len] using this)

theorem deref_val {γ : Type} (v : alloc.vec.Vec γ) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref, Slice.from_val]

/-- The prefix words equal the command's first words. -/
def Named (pre ws : List (alloc.vec.Vec U8)) : Prop :=
  pre.length ≤ ws.length ∧ ∀ i (h1 : i < pre.length) (h2 : i < ws.length), pre[i].val = ws[i].val

theorem named_loop_spec (pre ws : Slice (alloc.vec.Vec U8)) (i : Usize)
    (hle : pre.val.length ≤ ws.val.length) (hi : i.val ≤ pre.val.length)
    (heq : ∀ j (h1 : j < pre.val.length) (h2 : j < ws.val.length), j < i.val →
      pre.val[j].val = ws.val[j].val) :
    command.named_loop pre ws i ⦃ b => b = true ↔ Named pre.val ws.val ∧
      ¬ (ws.val.length = pre.val.length + 1 ∧
        ∃ h : pre.val.length < ws.val.length, ws.val[pre.val.length].val = HELP) ⦄ := by
  unfold command.named_loop
  apply loop.spec_decr_nat (fun i : Usize => pre.val.length - i.val)
    (fun i : Usize => i.val ≤ pre.val.length ∧
      ∀ j (h1 : j < pre.val.length) (h2 : j < ws.val.length), j < i.val →
        pre.val[j].val = ws.val[j].val)
  · intro i ⟨hi, heq⟩
    unfold command.named_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ v, hv ⟩
      step as ⟨ v1, hv1 ⟩
      step with same_spec as ⟨ b, hb ⟩
      rw [deref_val, deref_val] at hb
      split
      · rename_i hbt
        step as ⟨ i2, hi2 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j h1 h2 hj
        by_cases hji : j < i.val
        · exact heq j h1 h2 hji
        · have : j = i.val := by scalar_tac
          subst this
          have := hb.mp hbt; rw [hv, hv1] at this; exact this
      · rename_i hbt
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
        intro ⟨_, hall⟩
        exfalso; apply hbt; apply hb.mpr
        rw [hv, hv1]; exact hall _ (by scalar_tac) (by scalar_tac)
    · rename_i hge
      have hall : Named pre.val ws.val := ⟨hle, fun j h1 h2 => heq j h1 h2 (by scalar_tac)⟩
      step as ⟨ d, hd ⟩
      split
      · rename_i hd1
        step as ⟨ v, hv ⟩
        step with is_help_spec as ⟨ h, hh ⟩
        rw [deref_val] at hh
        have hlen : ws.val.length = pre.val.length + 1 := by scalar_tac
        cases h <;> simp_all
      · rename_i hd1
        simp only [bind_ok, WP.spec_ok]
        simp only [Bool.false_eq_true, not_false_eq_true, decide_true, true_iff]
        refine ⟨hall, ?_⟩
        rintro ⟨hlen, _⟩; apply hd1; scalar_tac
  · exact ⟨hi, heq⟩

/-- A rule's prefix names a command exactly when its words begin the
command and the command is not the prefix followed by `--help` alone. -/
theorem named_spec (pre ws : Slice (alloc.vec.Vec U8)) :
    command.named pre ws ⦃ b => b = true ↔ Named pre.val ws.val ∧
      ¬ (ws.val.length = pre.val.length + 1 ∧
        ∃ h : pre.val.length < ws.val.length, ws.val[pre.val.length].val = HELP) ⦄ := by
  unfold command.named
  dsimp only
  split
  · rename_i hgt
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and, Named]
    intro hle; exfalso; scalar_tac
  · rename_i hle
    exact named_loop_spec pre ws 0#usize (by scalar_tac) (by simp)
      (fun j _ _ hj => absurd hj (by simp))

end Writfold.Core.Command
