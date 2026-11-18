import Aeneas
import Extracted.WritfoldCore

/-!
# Attached stores

Theorems over the Aeneas translation of `writfold_core::store`: which rule
judges a citation into a companion repository of evidence, the verdict
under the pinned revision, and where a declared store's clone is found.

- `basis_pin_iff`, `basis_head_default`: the pin judges exactly a citation
  into a declared store whose freshness is `pinned` and whose token names
  a revision; every other repository is judged at head, as before.
- `pinned_fresh_iff`: under the pin, fresh exactly when the cited commit
  is in the clone, some ref reaches it, and `rev:path` is the cited blob.
  `pinned` takes nothing about the store's head, so a store reorganised
  since reads the same (its signature, not a theorem).
- `pinned_behind_unverifiable`: a commit the clone lacks is unverifiable,
  never gone and never fresh: a clone that is behind is not evidence of
  loss. `pinned_unanchored_gone`: a commit no ref reaches is gone.
  `pinned_other_bytes`: an anchored commit whose path holds other bytes,
  or nothing, is gone.
- `resolve_map_wins`: the local map, written by the machine's owner, wins
  whenever it names a clone, and is flagged when that clone's origin is
  not the declared one. `resolve_committed_needs_origin`: a clone found
  through a search root or a committed path hint is used only when its
  origin is the declared one. `resolve_search_before_hint`.
  `resolve_flag_only_map`: only the map's answer is ever flagged.
  `resolve_missing_iff`, `resolve_mismatch_iff`: missing exactly when no
  way names a clone; a mismatch exactly when one does but none that may be
  used.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Store

/-- The pin judges exactly a declared, pinned store's citation with a
revision. -/
theorem basis_pin_iff (d p r : Bool) :
    store.basis d p r ⦃ b => b = .Pin ↔ d = true ∧ p = true ∧ r = true ⦄ := by
  unfold store.basis
  cases d <;> cases p <;> cases r <;> simp

/-- A repository that is not declared, or a store not declared `pinned`,
is judged at head: every KB without a store declaration reads as before. -/
theorem basis_head_default (d p r : Bool) (h : d = false ∨ p = false) :
    store.basis d p r ⦃ b => b = .Head ⦄ := by
  unfold store.basis
  cases d <;> cases p <;> cases r <;> simp_all

/-- Under the pin, fresh exactly for the cited bytes at an anchored
commit. -/
theorem pinned_fresh_iff (c : store.Commit) (a : store.AtPin) :
    store.pinned c a ⦃ v => v = .Fresh ↔ c = .Anchored ∧ a = .Cited ⦄ := by
  unfold store.pinned
  cases c <;> cases a <;> simp

/-- A commit the clone lacks is unverifiable, whatever the path. -/
theorem pinned_behind_unverifiable (a : store.AtPin) :
    store.pinned .Absent a ⦃ v => v = .Unverifiable ⦄ := by
  unfold store.pinned
  simp

/-- A commit no ref reaches is gone, even while its objects are on disk. -/
theorem pinned_unanchored_gone (a : store.AtPin) :
    store.pinned .Unanchored a ⦃ v => v = .Gone ⦄ := by
  unfold store.pinned
  simp

/-- An anchored commit whose path holds other bytes, or nothing, is gone:
the token never described that commit. -/
theorem pinned_other_bytes (a : store.AtPin) (h : a ≠ .Cited) :
    store.pinned .Anchored a ⦃ v => v = .Gone ⦄ := by
  unfold store.pinned
  cases a <;> simp at h ⊢

/-- The local map wins whenever it names a clone, flagged exactly when the
clone's origin is not the declared one. -/
theorem resolve_map_wins (o : Bool) (s h : store.Candidate) :
    store.resolve (.Clone o) s h ⦃ r => r = .Found .Map (!o) ⦄ := by
  unfold store.resolve
  cases o <;> simp

/-- A clone found through a search root or a committed hint is used only
when its origin is the declared one. -/
theorem resolve_committed_needs_origin (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      (∀ f, r = .Found .SearchRoot f → s = .Clone true ∧ f = false) ∧
      (∀ f, r = .Found .Hint f → h = .Clone true ∧ f = false) ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- With no map entry, a search root with the declared origin is used
before a committed hint. -/
theorem resolve_search_before_hint (h : store.Candidate) :
    store.resolve .None (.Clone true) h ⦃ r => r = .Found .SearchRoot false ⦄ := by
  unfold store.resolve
  simp

/-- Only the map's answer is ever flagged. -/
theorem resolve_flag_only_map (m s h : store.Candidate) :
    store.resolve m s h ⦃ r => ∀ v, r = .Found v true → v = .Map ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- Missing exactly when no way names a clone. -/
theorem resolve_missing_iff (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      r = .Missing ↔ m = .None ∧ s = .None ∧ h = .None ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- A mismatch exactly when no map entry, no committed way with the
declared origin, and some committed way names a clone. -/
theorem resolve_mismatch_iff (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      r = .Mismatch ↔ m = .None ∧ s ≠ .Clone true ∧ h ≠ .Clone true ∧
        (s ≠ .None ∨ h ≠ .None) ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

end Writfold.Core.Store
