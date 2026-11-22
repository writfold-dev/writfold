import Extracted.WritfoldCore.Subject
import Extracted.WritfoldCore.Types

/-!
# The subject repository

Theorems over the Aeneas translation of `writfold_core::subject::pick`: which
repository a gate judges when no `--repo` is given.

- `pick_spec`: the rule, row by row: the repository enclosing the working
  directory when it is not the KB's own; else the run's recorded worktree
  when it is not another checkout of the KB's repository; else the
  enclosing repository; else the KB's own when the recorded worktree was
  such a checkout; else none.
- `recorded_never_kb_checkout`: the recorded worktree is judged only when
  it is not a checkout of the KB's repository. So the subject is never
  another checkout of the KB's repository than the one the command runs
  in (the enclosing one, or the KB's own).
- `here_outside_kb_wins`: a working directory in a repository other than the
  KB's is always the subject.
- `kb_only_for_checkout`: the KB's own repository stands in only for a
  recorded checkout of it, outside any repository.
- `unchanged_unless_checkout`: when the recorded worktree is not a checkout
  of the KB's repository, the choice is the earlier rule (enclosing, else
  recorded, else enclosing).
- `some_subject`: a repository found means a subject.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Subject

/-- The rule, as a table. -/
def Rule (here hereKb recorded recordedKb : Bool) : subject.Pick :=
  if here = true ∧ hereKb = false then .Here
  else if recorded = true ∧ recordedKb = false then .Recorded
  else if here = true then .Here
  else if recorded = true then .Kb
  else .Nothing

/-- The rule before a recorded checkout of the KB's repository was told
apart: enclosing (not the KB's), else recorded, else enclosing. -/
def Earlier (here hereKb recorded : Bool) : subject.Pick :=
  if here = true ∧ hereKb = false then .Here
  else if recorded = true then .Recorded
  else if here = true then .Here
  else .Nothing

theorem pick_spec (h hk r rk : Bool) :
    subject.pick h hk r rk ⦃ p => p = Rule h hk r rk ⦄ := by
  unfold subject.pick Rule
  cases h <;> cases hk <;> cases r <;> cases rk <;> simp

theorem recorded_never_kb_checkout (h hk r rk : Bool) :
    subject.pick h hk r rk ⦃ p => p = .Recorded → r = true ∧ rk = false ⦄ := by
  unfold subject.pick
  cases h <;> cases hk <;> cases r <;> cases rk <;> simp

theorem here_outside_kb_wins (r rk : Bool) :
    subject.pick true false r rk ⦃ p => p = .Here ⦄ := by
  unfold subject.pick
  simp

theorem kb_only_for_checkout (h hk r rk : Bool) :
    subject.pick h hk r rk ⦃ p => p = .Kb → h = false ∧ r = true ∧ rk = true ⦄ := by
  unfold subject.pick
  cases h <;> cases hk <;> cases r <;> cases rk <;> simp

theorem unchanged_unless_checkout (h hk r : Bool) :
    subject.pick h hk r false ⦃ p => p = Earlier h hk r ⦄ := by
  unfold subject.pick Earlier
  cases h <;> cases hk <;> cases r <;> simp

theorem some_subject (h hk r rk : Bool) (found : h = true ∨ r = true) :
    subject.pick h hk r rk ⦃ p => p ≠ .Nothing ⦄ := by
  unfold subject.pick
  cases h <;> cases hk <;> cases r <;> cases rk <;> simp_all

end Writfold.Core.Subject
