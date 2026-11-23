import Extracted.WritfoldCore.Runs
import Extracted.WritfoldCore.Types

/-!
# The run list (#98)

Theorems over the Aeneas translation of `writfold_core::runs`: what
`kb runs` shows of a run, and when its list may claim to be complete.

- `shown_spec`: `shown` is its table (`shownOf`).
- `hidden_iff`: a run is left out exactly when `--open` is asked and the
  run is known to be closed. A state outside the vocabulary is never taken
  as closed, and without `--open` nothing is left out (`all_listed`).
- `flagged_iff`: a run is flagged exactly when its state is outside the
  vocabulary, so such a run is never shown as an ordinary open or closed
  one, and an `open` or `closed` run is never flagged.
- `complete_spec`, `no_manifest_incomplete`, `unreadable_incomplete`: the
  list is complete exactly when no manifest is unreadable and no run
  directory lacks one.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Runs

/-- `shown`, as a table. -/
def shownOf : runs.State → Bool → runs.Shown
  | .Closed, true => .Hidden
  | .Closed, false => .Listed
  | .Open, _ => .Listed
  | .Other, _ => .Flagged

theorem shown_spec (s : runs.State) (o : Bool) :
    runs.shown s o ⦃ r => r = shownOf s o ⦄ := by
  unfold runs.shown
  cases s <;> cases o <;> simp [shownOf]

/-- `--open` hides only a run known to be closed. -/
theorem hidden_iff (s : runs.State) (o : Bool) :
    shownOf s o = .Hidden ↔ o = true ∧ s = .Closed := by
  cases s <;> cases o <;> simp [shownOf]

/-- Without `--open`, every run is listed. -/
theorem all_listed (s : runs.State) : shownOf s false ≠ .Hidden := by
  cases s <;> simp [shownOf]

/-- A state outside the vocabulary is flagged, and only such a state is. -/
theorem flagged_iff (s : runs.State) (o : Bool) :
    shownOf s o = .Flagged ↔ s = .Other := by
  cases s <;> cases o <;> simp [shownOf]

/-- The list is complete exactly when nothing is unreadable and no run
directory lacks a manifest. -/
theorem complete_spec (u n : Usize) :
    runs.complete u n ⦃ b => b = true ↔ u.val = 0 ∧ n.val = 0 ⦄ := by
  unfold runs.complete
  split
  · rename_i hu
    have hu' : u.val = 0 := by scalar_tac
    simp only [WP.spec_ok, decide_eq_true_eq, hu', true_and]
    constructor
    · intro h; scalar_tac
    · intro h; have : n = 0#usize := by scalar_tac
      exact this
  · rename_i hu
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro h; exact absurd (by scalar_tac : u = 0#usize) hu

theorem no_manifest_incomplete (u n : Usize) (h : 0 < n.val) :
    runs.complete u n ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (complete_spec u n)
  intro b hb
  cases b <;> simp_all

theorem unreadable_incomplete (u n : Usize) (h : 0 < u.val) :
    runs.complete u n ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (complete_spec u n)
  intro b hb
  cases b <;> simp_all

end Writfold.Core.Runs
