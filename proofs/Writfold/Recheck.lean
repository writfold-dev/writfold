import Extracted.WritfoldCore.Plan
import Extracted.WritfoldCore.Types

/-!
# Plan writes: the closed run and the re-check under the lock

Theorems over the Aeneas translation of `writfold_core::plan::recheck`.

- `still_stands_spec`: the re-check under the KB write lock, case by case.
  A write decided on an unplanned run stands only while no plan was set; a
  write about the current step only while that step is still current; a
  splice only while its parent is still current (when it was) and its
  ordinal is still the next one. `stands_iff` states when it stands.
- `writable_spec`: a run takes plan writes unless the state in force says
  `closed`; the sidecar wins over the manifest (`sidecar_wins`), and a
  closed run never takes one (`closed_refuses`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Recheck

/-- When a decision still stands on the fold read again. -/
def Holds (d : plan.recheck.Decided) (planned cur : Bool) (next : U64) : Prop :=
  match d with
  | .Unplanned => planned = false
  | .Current => cur = true
  | .Splice pc o => (pc = true → cur = true) ∧ next = o

/-- The re-check's answer, case by case. -/
def StillSpec (d : plan.recheck.Decided) (planned cur : Bool) (next : U64) :
    Option plan.recheck.Moved → Prop
  | none => Holds d planned cur next
  | some .Planned => d = .Unplanned ∧ planned = true
  | some .NotCurrent => (d = .Current ∨ ∃ o, d = .Splice true o) ∧ cur = false
  | some .Taken => ∃ pc o, d = .Splice pc o ∧ (pc = true → cur = true) ∧ next ≠ o

theorem still_stands_spec (d : plan.recheck.Decided) (planned cur : Bool) (next : U64) :
    plan.recheck.still_stands d planned cur next ⦃ r => StillSpec d planned cur next r ⦄ := by
  unfold plan.recheck.still_stands
  cases d with
  | Unplanned => cases planned <;> simp [StillSpec, Holds]
  | Current => cases cur <;> simp [StillSpec, Holds]
  | Splice pc o =>
    cases pc <;> cases cur <;> by_cases h : next = o <;> simp [StillSpec, Holds, h] <;>
      exact fun e => h (UScalar.eq_of_val_eq e)

/-- A decision stands exactly when what it read still holds. -/
theorem stands_iff (d : plan.recheck.Decided) (planned cur : Bool) (next : U64) :
    plan.recheck.still_stands d planned cur next ⦃ r => r = none ↔ Holds d planned cur next ⦄ := by
  apply WP.spec_mono (still_stands_spec d planned cur next)
  intro r hr
  cases r with
  | none => simpa [StillSpec] using hr
  | some m =>
    simp only [reduceCtorEq, false_iff]
    intro hh
    cases m <;> simp only [StillSpec] at hr
    · obtain ⟨rfl, hp⟩ := hr; simp [Holds, hp] at hh
    · obtain ⟨hd | ⟨o, rfl⟩, hc⟩ := hr
      · subst hd; simp [Holds, hc] at hh
      · simp [Holds, hc] at hh
    · obtain ⟨pc, o, rfl, _, hn⟩ := hr; exact hn hh.2

/-- A run takes plan writes unless the state in force says `closed`: the
sidecar's when it says anything, else the manifest's, else open. -/
theorem writable_spec (sidecar manifest : Option Bool) :
    plan.recheck.writable sidecar manifest ⦃ b =>
      b = true ↔ (sidecar.getD (manifest.getD false)) = false ⦄ := by
  unfold plan.recheck.writable
  cases sidecar with
  | none => cases manifest with
    | none => simp
    | some c => cases c <;> simp
  | some c => cases c <;> simp

/-- The sidecar wins over the manifest. -/
theorem sidecar_wins (c : Bool) (manifest : Option Bool) :
    plan.recheck.writable (some c) manifest ⦃ b => b = !c ⦄ := by
  unfold plan.recheck.writable
  cases c <;> simp

/-- A closed run never takes a plan write. -/
theorem closed_refuses (manifest : Option Bool) :
    plan.recheck.writable (some true) manifest ⦃ b => b = false ⦄ := by
  unfold plan.recheck.writable
  simp

end Writfold.Core.Recheck
