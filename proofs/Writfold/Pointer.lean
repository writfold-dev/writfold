import Aeneas
import Extracted.WritfoldCore

/-!
# The current run of a seat

Theorems over the Aeneas translation of `writfold_core::pointer`: which of
a KB clone's three local current-run pointers (the session's, the work
tree's, the shared one) decides for a seat, and which pointers a seat
writes when it makes a run current.

- `level_spec`: the deciding level is the most specific pointer that is
  set; none exactly when no pointer is set.
- `open_spec`: the seat's current run is open exactly when the deciding
  pointer names an open run.
- `session_decides`, `tree_decides`: a seat whose session pointer is set
  reads the same whatever the tree and shared pointers hold; one with no
  session pointer and a tree pointer reads the same whatever the shared
  pointer holds. Another seat's writes there change nothing for it.
- `ended_not_skipped`: a deciding pointer that names a closed or missing
  run is never skipped for a less specific pointer to an open run.
- `shared_alone`: with only the shared pointer, the answer is that
  pointer's (the rule before sessions and trees had pointers).
- `writes_spec`: making a run current writes the pointer of
  each level the seat has, and the shared one always.
- `writer_reads_its_run`: after a seat makes an open run current, that
  seat reads an open current run, whatever the pointers held before.
- `other_session_undisturbed`: a seat whose session pointer is set reads
  the same before and after any other seat makes a run current.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Pointer

/-- The deciding pointer: the most specific one that is set. -/
def deciding : pointer.Pointer → pointer.Pointer → pointer.Pointer → pointer.Pointer
  | .Unset, .Unset, h => h
  | .Unset, t, _ => t
  | s, _, _ => s

theorem is_set_spec (p : pointer.Pointer) :
    pointer.is_set p ⦃ b => b = true ↔ p ≠ .Unset ⦄ := by
  unfold pointer.is_set
  cases p <;> simp

/-- The deciding level is the most specific pointer that is set. -/
theorem level_spec (s t h : pointer.Pointer) :
    pointer.level s t h ⦃ l =>
      (l = some .Session ↔ s ≠ .Unset) ∧
      (l = some .Tree ↔ s = .Unset ∧ t ≠ .Unset) ∧
      (l = some .Shared ↔ s = .Unset ∧ t = .Unset ∧ h ≠ .Unset) ∧
      (l = none ↔ s = .Unset ∧ t = .Unset ∧ h = .Unset) ⦄ := by
  unfold pointer.level pointer.is_set
  cases s <;> cases t <;> cases h <;> simp

/-- The seat's current run is open exactly when the deciding pointer names
an open run. -/
theorem open_spec (s t h : pointer.Pointer) :
    pointer.open s t h ⦃ b => b = true ↔ deciding s t h = .Open ⦄ := by
  unfold pointer.open pointer.level pointer.is_set
  cases s <;> cases t <;> cases h <;> simp [deciding]

/-- A set session pointer decides alone. -/
theorem session_decides (s t h t' h' : pointer.Pointer) (hs : s ≠ .Unset) :
    pointer.open s t h = pointer.open s t' h' := by
  unfold pointer.open pointer.level pointer.is_set
  cases s <;> simp at hs <;> simp

/-- With no session pointer, a set tree pointer decides alone. -/
theorem tree_decides (t h h' : pointer.Pointer) (ht : t ≠ .Unset) :
    pointer.open .Unset t h = pointer.open .Unset t h' := by
  unfold pointer.open pointer.level pointer.is_set
  cases t <;> simp at ht <;> simp

/-- A deciding pointer that names a closed or missing run is not skipped:
the seat has no open current run, whatever the less specific pointers
name. -/
theorem ended_not_skipped (s t h : pointer.Pointer)
    (hd : deciding s t h = .Closed ∨ deciding s t h = .Gone) :
    pointer.open s t h ⦃ b => b = false ⦄ := by
  unfold pointer.open pointer.level pointer.is_set
  cases s <;> cases t <;> cases h <;> simp [deciding] at hd ⊢

/-- With only the shared pointer the answer is that pointer's. -/
theorem shared_alone (h : pointer.Pointer) :
    pointer.open .Unset .Unset h ⦃ b => b = true ↔ h = .Open ⦄ := by
  unfold pointer.open pointer.level pointer.is_set
  cases h <;> simp

/-- A seat writes the pointer of each level it has, and the shared one
always. -/
theorem writes_spec (hs ht : Bool) :
    pointer.writes hs ht ⦃ w => w.session = hs ∧ w.tree = ht ∧ w.shared = true ⦄ := by
  unfold pointer.writes
  simp

/-- A pointer after a seat made an open run current: the run when the seat
wrote it, else as it was. -/
def after (old : pointer.Pointer) (written : Bool) : pointer.Pointer :=
  if written then .Open else old

/-- A level the seat lacks reads as unset. -/
def seen (has : Bool) (p : pointer.Pointer) : pointer.Pointer :=
  if has then p else .Unset

/-- After a seat makes an open run current, it reads an open current run,
whatever the three pointers held before. -/
theorem writer_reads_its_run (hs ht : Bool) (s t h : pointer.Pointer) :
    (do
      let w ← pointer.writes hs ht
      pointer.open (seen hs (after s w.session)) (seen ht (after t w.tree))
        (after h w.shared)) = ok true := by
  unfold pointer.writes pointer.open pointer.level pointer.is_set
  cases hs <;> cases ht <;> simp [seen, after]

/-- A seat whose session pointer is set reads the same before and after
another seat makes a run current (that seat writes the tree and shared
pointers, and its own session pointer, never this one's). -/
theorem other_session_undisturbed (s t h : pointer.Pointer) (wt wh : Bool)
    (hs : s ≠ .Unset) :
    pointer.open s (after t wt) (after h wh) = pointer.open s t h :=
  session_decides s _ _ _ _ hs

end Writfold.Core.Pointer
