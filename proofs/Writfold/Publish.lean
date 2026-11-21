import Writfold.Get

/-!
# Publishing a derived index (#58)

Theorems over the Aeneas translation of `writfold_core::publish`: when a
freshly derived index may replace the stored one. The protocol around it is
`spec/KbRepair.tla` (rename) and `spec/KbWrite.tla` (transactional refresh).

- `repair_replace_iff`: a repair replaces the index exactly on a mismatch
  while neither the KB nor the live index moved.
- `drift_never_replaces`: if the KB or the live index moved, a repair never
  replaces, whatever the comparison found.
- `equal_keeps`: no mismatch keeps the index.
- `derived_replace_iff`: a full rebuild or a refresh publishes exactly when
  the KB did not move while it read.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Publish

theorem repair_replace_iff (r : publish.Repair) :
    publish.repair r ⦃ p => p = .Replace ↔
      r.mismatch = true ∧ r.source_moved = false ∧ r.live_moved = false ⦄ := by
  rcases r with ⟨m, s, l⟩
  unfold publish.repair
  cases m <;> cases s <;> cases l <;> simp

theorem drift_never_replaces (r : publish.Repair)
    (h : r.source_moved = true ∨ r.live_moved = true) :
    publish.repair r ⦃ p => p ≠ .Replace ⦄ := by
  rcases r with ⟨m, s, l⟩
  simp only at h
  unfold publish.repair
  rcases h with h | h <;> subst h <;> cases m <;> (try cases s) <;> (try cases l) <;> simp

theorem equal_keeps (r : publish.Repair) (h : r.mismatch = false) :
    publish.repair r ⦃ p => p = .Keep ⦄ := by
  rcases r with ⟨m, s, l⟩
  simp only at h
  subst h
  unfold publish.repair
  simp

theorem derived_replace_iff (moved : Bool) :
    publish.derived moved ⦃ p => p = .Replace ↔ moved = false ⦄ := by
  unfold publish.derived
  cases moved <;> simp

end Writfold.Core.Publish
