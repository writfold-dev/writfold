import Extracted.WritfoldCore

/-!
# Small claims: a complete walk, a blocking lint, a `find` row

Theorems over the Aeneas translation of `writfold_core::walk::complete`,
`writfold_core::diag::blocks` and `writfold_core::find::admit`.

- `complete_iff`: `coverage=complete` is claimed exactly when nothing is
  left on the frontier and no reason skipped an edge. The traversal that
  counts them is compared with a reference search, not proved.
- `blocks_iff`: a lint blocks exactly on an error, or on a warning under
  `--deny-warnings`.
- `admit_iff`: `find` shows a hit exactly when fewer than the limit are
  shown, its record is not shown yet and it passes the filter; since every
  tier's hits go through it in tier order, each record is shown once,
  under the first tier that found it.
- `full_spec`, `full_admits_nothing`, `not_full_admits`: `find` stops running
  tiers once the rows are full, exactly when no later hit could be shown;
  before that, any hit not yet shown that passes the filter is shown. So
  skipping the later tiers changes no row.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Walk

theorem complete_iff (frontier skipped : Usize) :
    walk.complete frontier skipped ⦃ b => b = true ↔ frontier.val = 0 ∧ skipped.val = 0 ⦄ := by
  unfold walk.complete
  split
  · rename_i h; simp only [WP.spec_ok, decide_eq_true_eq]
    constructor
    · intro e; exact ⟨by simp [h], by simp [e]⟩
    · rintro ⟨_, e⟩; exact UScalar.eq_of_val_eq (by simpa using e)
  · rename_i h; simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro e; exact absurd (UScalar.eq_of_val_eq (by simpa using e)) h

theorem blocks_iff (errors warnings : Usize) (deny : Bool) :
    diag.blocks errors warnings deny ⦃ b => b = true ↔
      errors.val > 0 ∨ (deny = true ∧ warnings.val > 0) ⦄ := by
  unfold diag.blocks
  split
  · simp only [WP.spec_ok, true_iff]; left; scalar_tac
  · cases deny <;> simp <;> scalar_tac

theorem admit_iff (shown limit : Usize) (seen passes : Bool) :
    find.admit shown limit seen passes ⦃ b => b = true ↔
      shown.val < limit.val ∧ seen = false ∧ passes = true ⦄ := by
  unfold find.admit
  split
  · cases seen <;> simp <;> scalar_tac
  · simp; scalar_tac

theorem full_spec (shown limit : Usize) :
    find.full shown limit ⦃ b => b = true ↔ shown.val ≥ limit.val ⦄ := by
  unfold find.full
  simp

/-- Full rows admit no further hit, whatever it is. -/
theorem full_admits_nothing (shown limit : Usize) (seen passes : Bool) :
    (do let f ← find.full shown limit; let a ← find.admit shown limit seen passes; ok (f, a))
      ⦃ r => r.1 = true → r.2 = false ⦄ := by
  unfold find.full find.admit
  cases seen <;> cases passes <;> split <;> simp
  all_goals scalar_tac

/-- Rows not yet full admit any hit not yet shown that passes the filter. -/
theorem not_full_admits (shown limit : Usize) :
    (do let f ← find.full shown limit; let a ← find.admit shown limit false true; ok (f, a))
      ⦃ r => r.1 = false → r.2 = true ⦄ := by
  unfold find.full find.admit
  split
  · simp
  · simp; scalar_tac

end Writfold.Core.Walk
