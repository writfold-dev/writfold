import Extracted.WritfoldCore.Prose
import Extracted.WritfoldCore.Types

/-!
# Prose rules for delivery drafts (#127)

Theorems over the Aeneas translation of `writfold_core::prose`: what a list
of rule matches means for a delivery check.

- `blocking_spec`: `blocking` counts the blocking matches.
- `verdict_spec`, `verdict_no_rules_unknown`, `verdict_pass_iff`,
  `verdict_fail_iff`: no rules is never a pass; a pass is exactly some rules
  and nothing blocking; a failure is exactly some rules and something
  blocking.
- `advice_never_decides`, `verdict_perm`: the verdict depends only on the
  blocking matches, so adding, removing or reordering advice, or reordering
  anything, cannot change it.
- `agrees_spec`, `agrees_verdict`, `agrees_pass_iff_none`: a receipt written
  from the verdict always reads back consistent, and a pass agrees exactly
  with no blocking findings (the reader's rule for every delivery check).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Prose

/-- Whether a match blocks. -/
def isB (w : prose.Weight) : Bool :=
  match w with
  | .Blocking => true
  | .Advice => false

/-- The blocking matches of a list. -/
def countB (l : List prose.Weight) : Nat := l.countP isB

theorem countB_succ (l : List prose.Weight) (n : Nat) (h : n < l.length) :
    countB (l.take (n + 1)) = countB (l.take n) + (if isB l[n] then 1 else 0) := by
  unfold countB
  rw [List.take_add_one, List.countP_append, List.getElem?_eq_getElem h]
  cases hb : isB l[n] <;> simp [hb]

theorem countB_take_le (l : List prose.Weight) (n : Nat) : countB (l.take n) ≤ n := by
  unfold countB
  have := List.countP_le_length (p := isB) (l := l.take n)
  simp at this
  omega

theorem blocking_loop_spec (found : Slice prose.Weight) (count index : Usize)
    (hi : index.val ≤ found.val.length)
    (hc : count.val = countB (found.val.take index.val)) :
    prose.blocking_loop found count index ⦃ n => n.val = countB found.val ⦄ := by
  unfold prose.blocking_loop
  apply loop.spec_decr_nat (fun p : Usize × Usize => found.val.length - p.2.val)
    (fun p : Usize × Usize => p.2.val ≤ found.val.length ∧
      p.1.val = countB (found.val.take p.2.val))
  · rintro ⟨count, index⟩ ⟨hi, hc⟩
    unfold prose.blocking_loop.body
    dsimp only at hi hc ⊢
    split
    · rename_i hlt
      have hil : index.val < found.val.length := by scalar_tac
      have hs := countB_succ found.val index.val hil
      have hle := countB_take_le found.val index.val
      step as ⟨ w, hw ⟩
      cases w with
      | Advice =>
        simp only
        step as ⟨ index1, hindex1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hindex1, hs, ← hw]; simp [isB, hc]
      | Blocking =>
        simp only
        step as ⟨ count1, hcount1 ⟩
        step as ⟨ index1, hindex1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hindex1, hs, ← hw, hcount1, hc]; simp [isB]
    · rename_i hge
      simp only [WP.spec_ok]
      have hlen : index.val = found.val.length := by scalar_tac
      rw [hc, hlen, List.take_length]
  · exact ⟨hi, hc⟩

/-- `blocking` counts the blocking matches. -/
theorem blocking_spec (found : Slice prose.Weight) :
    prose.blocking found ⦃ n => n.val = countB found.val ⦄ := by
  unfold prose.blocking
  apply blocking_loop_spec
  · simp
  · simp [countB]

/-- The verdict on the model: it reads only the blocking count. -/
def verdictOf (rules blocking : Nat) : gate.Outcome :=
  if rules = 0 then .Unknown else if 0 < blocking then .Fail else .Pass

theorem verdict_spec (rules : Usize) (found : Slice prose.Weight) :
    prose.verdict rules found ⦃ o => o = verdictOf rules.val (countB found.val) ⦄ := by
  unfold prose.verdict verdictOf
  split
  · rename_i h0
    simp only [WP.spec_ok]
    have : rules.val = 0 := by scalar_tac
    simp [this]
  · rename_i h0
    have hr : rules.val ≠ 0 := by scalar_tac
    step with blocking_spec as ⟨ n, hn ⟩
    split
    · rename_i hpos
      simp only [WP.spec_ok]
      have : 0 < countB found.val := by scalar_tac
      simp [this]
    · rename_i hz
      simp only [WP.spec_ok]
      have : ¬ 0 < countB found.val := by scalar_tac
      simp [this]

/-- No rules is never a pass, whatever was found. -/
theorem verdict_no_rules_unknown (found : Slice prose.Weight) :
    prose.verdict 0#usize found ⦃ o => o = .Unknown ⦄ := by
  apply WP.spec_mono (verdict_spec 0#usize found)
  intro o ho; rw [ho]; simp [verdictOf]

theorem verdict_pass_iff (rules : Usize) (found : Slice prose.Weight) :
    prose.verdict rules found ⦃ o =>
      o = .Pass ↔ rules.val ≠ 0 ∧ countB found.val = 0 ⦄ := by
  apply WP.spec_mono (verdict_spec rules found)
  intro o ho; rw [ho]; unfold verdictOf
  by_cases h0 : rules.val = 0 <;> by_cases hb : 0 < countB found.val <;> simp [h0, hb] <;> omega

theorem verdict_fail_iff (rules : Usize) (found : Slice prose.Weight) :
    prose.verdict rules found ⦃ o =>
      o = .Fail ↔ rules.val ≠ 0 ∧ 0 < countB found.val ⦄ := by
  apply WP.spec_mono (verdict_spec rules found)
  intro o ho; rw [ho]; unfold verdictOf
  by_cases h0 : rules.val = 0 <;> by_cases hb : 0 < countB found.val <;> simp [h0, hb]

/-- Removing every piece of advice leaves the verdict unchanged. -/
theorem advice_never_decides (rules : Nat) (l : List prose.Weight) :
    verdictOf rules (countB (l.filter isB)) = verdictOf rules (countB l) := by
  unfold countB
  rw [List.countP_filter]
  congr 2
  funext x
  simp

/-- Reordering the matches leaves the verdict unchanged. -/
theorem verdict_perm (rules : Nat) (l l' : List prose.Weight) (h : l.Perm l') :
    verdictOf rules (countB l) = verdictOf rules (countB l') := by
  unfold countB
  rw [h.countP_eq]

/-- `agrees` on the model. -/
def agreesOf (o : gate.Outcome) (n : Nat) : Bool :=
  match o with
  | .Pass => n = 0
  | .Fail => 0 < n
  | .Unknown => true

theorem agrees_spec (o : gate.Outcome) (n : Usize) :
    prose.agrees o n ⦃ b => b = agreesOf o n.val ⦄ := by
  unfold prose.agrees agreesOf
  cases o <;> simp
  all_goals scalar_tac

/-- A pass agrees exactly with no blocking findings. -/
theorem agrees_pass_iff_none (n : Usize) :
    prose.agrees .Pass n ⦃ b => b = true ↔ n.val = 0 ⦄ := by
  apply WP.spec_mono (agrees_spec .Pass n)
  intro b hb; rw [hb]; simp [agreesOf]

/-- A receipt written from the verdict always reads back consistent. -/
theorem agrees_verdict (rules n : Nat) : agreesOf (verdictOf rules n) n = true := by
  unfold verdictOf agreesOf
  by_cases h0 : rules = 0 <;> by_cases hb : 0 < n <;> simp [h0, hb]
  all_goals omega

end Writfold.Core.Prose
