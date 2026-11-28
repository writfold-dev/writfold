import Extracted.WritfoldCore.Site
import Extracted.WritfoldCore.Types

/-!
# The guard site (#184)

Theorems over the Aeneas translation of `writfold_core::site`, the decision
`kb hook pre-tool-use` and `kb guard check` make for one event once they
know how far they got with it (law S, `proofs/Calculus/Site.lean`).

- `decide_spec`, `open_is_today`: under `open` an undecided event passes,
  as every site did before postures existed.
- `closed_pass_iff`: under `ask` or `deny` an event passes only with no
  rules, no match, or a judged pass.
- `judged_unchanged`, `posture_only_narrows`: a posture never changes a
  judged decision, and what passes under any posture passes under `open`.
- `stricter_spec`, `stricter_comm`, `stricter_assoc`, `stricter_open_left`:
  the effective posture is the stricter of the registration's and the KB's,
  in any order.
- `block_again_spec`, `block_bounded`: a blocking stop rule blocks only
  while its requirement is unmet, at most `limit` times.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Site

/-- The site's decision, as a model. -/
def decideOf : site.Basis → site.Posture → guard.Decision
  | .NoRules, _ => .Pass
  | .NoMatch, _ => .Pass
  | .Judged d, _ => d
  | .Undecided, .Open => .Pass
  | .Undecided, .Ask => .Ask
  | .Undecided, .Deny => .Deny

theorem decide_spec (b : site.Basis) (p : site.Posture) :
    site.decide b p ⦃ d => d = decideOf b p ⦄ := by
  unfold site.decide
  cases b <;> cases p <;> simp [decideOf]

/-- Under `open` an event passes unless a rule judged it ask or deny. -/
theorem open_is_today (b : site.Basis) :
    site.decide b .Open ⦃ d => d = .Pass ↔ b ≠ .Judged .Deny ∧ b ≠ .Judged .Ask ⦄ :=
  WP.spec_mono (decide_spec b .Open) (fun d h => by
    subst h; cases b with
    | Judged x => cases x <;> simp [decideOf]
    | _ => simp [decideOf])

/-- Under a closed posture an undecided event never passes. -/
theorem closed_pass_iff (b : site.Basis) (p : site.Posture) (h : p ≠ .Open) :
    site.decide b p ⦃ d => d = .Pass ↔ b = .NoRules ∨ b = .NoMatch ∨ b = .Judged .Pass ⦄ :=
  WP.spec_mono (decide_spec b p) (fun d hd => by
    subst hd; cases b <;> cases p <;> simp_all [decideOf])

/-- A posture never changes a judged decision. -/
theorem judged_unchanged (d : guard.Decision) (p : site.Posture) :
    site.decide (.Judged d) p ⦃ r => r = d ⦄ :=
  WP.spec_mono (decide_spec (.Judged d) p) (fun r h => by simpa [decideOf] using h)

/-- What passes under any posture passes under `open`. -/
theorem posture_only_narrows (b : site.Basis) (p : site.Posture) (h : decideOf b p = .Pass) :
    site.decide b .Open ⦃ d => d = .Pass ⦄ :=
  WP.spec_mono (decide_spec b .Open) (fun d hd => by
    subst hd; revert h; cases b <;> cases p <;> simp [decideOf])

/-- The stricter posture, as a model. -/
def stricterOf : site.Posture → site.Posture → site.Posture
  | .Deny, _ => .Deny
  | _, .Deny => .Deny
  | .Ask, _ => .Ask
  | _, .Ask => .Ask
  | .Open, .Open => .Open

theorem stricter_spec (a b : site.Posture) :
    site.stricter a b ⦃ r => r = stricterOf a b ⦄ := by
  unfold site.stricter
  cases a <;> cases b <;> simp [stricterOf]

theorem stricterOf_comm (a b : site.Posture) : stricterOf a b = stricterOf b a := by
  cases a <;> cases b <;> rfl

theorem stricterOf_assoc (a b c : site.Posture) :
    stricterOf (stricterOf a b) c = stricterOf a (stricterOf b c) := by
  cases a <;> cases b <;> cases c <;> rfl

/-- The order of the two postures does not matter. -/
theorem stricter_comm (a b : site.Posture) :
    site.stricter a b ⦃ r => r = stricterOf b a ⦄ :=
  WP.spec_mono (stricter_spec a b) (fun r h => by rw [h, stricterOf_comm])

/-- Three postures combine in any grouping. -/
theorem stricter_assoc (a b c : site.Posture) :
    site.stricter (stricterOf a b) c ⦃ r => r = stricterOf a (stricterOf b c) ⦄ :=
  WP.spec_mono (stricter_spec _ c) (fun r h => by rw [h, stricterOf_assoc])

/-- `open` adds nothing. -/
theorem stricter_open_left (a : site.Posture) :
    site.stricter .Open a ⦃ r => r = a ⦄ :=
  WP.spec_mono (stricter_spec .Open a) (fun r h => by rw [h]; cases a <;> rfl)

theorem block_again_spec (u : Bool) (b l : Usize) :
    site.block_again u b l ⦃ r => r = (u && decide (b.val < l.val)) ⦄ := by
  unfold site.block_again
  cases u <;> simp

/-- At `limit` blocks a stop rule blocks no more. -/
theorem block_bounded (u : Bool) (b l : Usize) (h : l.val ≤ b.val) :
    site.block_again u b l ⦃ r => r = false ⦄ :=
  WP.spec_mono (block_again_spec u b l) (fun r hr => by
    rw [hr]; cases u <;> simp; omega)

end Writfold.Core.Site
