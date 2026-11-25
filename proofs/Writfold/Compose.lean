import Extracted.WritfoldCore.Compose
import Extracted.WritfoldCore.Plan
import Extracted.WritfoldCore.Types
import Writfold.Plan

/-!
# Agent profile closures (#144, RFC 2026-11-25-agent-vocabulary §3.6)

Theorems over the Aeneas translation of `writfold_core::compose`.

- `covers_spec`, `covers_none_iff`: `covers` is the least index of a
  required tool the profile does not grant, and `None` exactly when every
  required tool is granted.
- `closure_spec`, `closure_ok_iff`: the first reference that does not
  resolve is refused first; then the first uncovered tool; admitted exactly
  when every reference resolves and every required tool is granted.
- `more_skills_never_admit`: more references or more required tools (a
  listed skill added) never turn a refused closure into an admitted one.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Plan

namespace Writfold.Core.Compose

/-- Every required tool is granted. -/
def Covered (req granted : List (alloc.vec.Vec U8)) : Prop :=
  ∀ j (_ : j < req.length), KeyIn granted req[j].val

/-- Every reference resolved to one definition of the expected kind. -/
def AllFound (refs : List compose.RefFact) : Prop :=
  ∀ j (hj : j < refs.length), refs[j] = .Found

def CoversSpec (req granted : List (alloc.vec.Vec U8)) : Option Usize → Prop
  | none => Covered req granted
  | some i => ∃ h : i.val < req.length, ¬ KeyIn granted req[i.val].val ∧
      ∀ j (hj : j < i.val), KeyIn granted (req[j]'(by omega)).val

theorem covers_loop_spec (req granted : Slice (alloc.vec.Vec U8)) (i : Usize)
    (hi : i.val ≤ req.val.length)
    (hcov : ∀ j (_ : j < i.val) (hj' : j < req.val.length), KeyIn granted.val req.val[j].val) :
    compose.covers_loop req granted i ⦃ r => CoversSpec req.val granted.val r ⦄ := by
  unfold compose.covers_loop
  apply loop.spec_decr_nat (fun i : Usize => req.val.length - i.val)
    (fun i : Usize => i.val ≤ req.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < req.val.length), KeyIn granted.val req.val[j].val)
  · intro i ⟨hi, hcov⟩
    unfold compose.covers_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with contains_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hcov j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          have := hb.mp hbt
          rw [hx] at this
          simpa [alloc.vec.Vec.deref, Slice.from_val] using this
      · rename_i hbt
        simp only [WP.spec_ok, CoversSpec]
        refine ⟨by scalar_tac, ?_, ?_⟩
        · intro hk
          apply hbt
          apply hb.mpr
          rw [hx]
          simpa [alloc.vec.Vec.deref, Slice.from_val] using hk
        · intro j hj
          exact hcov j hj _
    · rename_i hge
      simp only [WP.spec_ok, CoversSpec, Covered]
      intro j hj
      exact hcov j (by scalar_tac) hj
  · exact ⟨hi, hcov⟩

theorem covers_spec (req granted : Slice (alloc.vec.Vec U8)) :
    compose.covers req granted ⦃ r => CoversSpec req.val granted.val r ⦄ := by
  unfold compose.covers
  apply covers_loop_spec
  · simp
  · intro j hj; simp at hj

theorem covers_none_iff (req granted : List (alloc.vec.Vec U8)) (r : Option Usize)
    (h : CoversSpec req granted r) : r = none ↔ Covered req granted := by
  cases r with
  | none => simpa [CoversSpec] using h
  | some i =>
    obtain ⟨hlt, hmiss, _⟩ := h
    simp only [reduceCtorEq, false_iff]
    intro hc
    exact hmiss (hc i.val hlt)

def ClosureSpec (refs : List compose.RefFact) (req granted : List (alloc.vec.Vec U8)) :
    core.result.Result Unit compose.Refusal → Prop
  | .Ok () => AllFound refs ∧ Covered req granted
  | .Err (.Reference i) => ∃ h : i.val < refs.length, refs[i.val] ≠ .Found ∧
      ∀ j (hj : j < i.val), (refs[j]'(by omega)) = .Found
  | .Err (.Tool i) => AllFound refs ∧ CoversSpec req granted (some i)

theorem closure_loop_spec (refs : Slice compose.RefFact) (req granted : Slice (alloc.vec.Vec U8))
    (i : Usize) (hi : i.val ≤ refs.val.length)
    (hfound : ∀ j (_ : j < i.val) (hj' : j < refs.val.length), refs.val[j] = .Found) :
    compose.closure_loop refs req granted i
      ⦃ r => ClosureSpec refs.val req.val granted.val r ⦄ := by
  unfold compose.closure_loop
  apply loop.spec_decr_nat (fun i : Usize => refs.val.length - i.val)
    (fun i : Usize => i.val ≤ refs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < refs.val.length), refs.val[j] = .Found)
  · intro i ⟨hi, hfound⟩
    unfold compose.closure_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      split
      · apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hfound j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          exact hx.symm
      all_goals
        simp only [WP.spec_ok, ClosureSpec]
        exact ⟨by scalar_tac, by rw [← hx]; simp, fun j hj => hfound j hj _⟩
    · rename_i hge
      step with covers_spec as ⟨ c, hc ⟩
      have hall : AllFound refs.val := fun j hj => hfound j (by scalar_tac) hj
      split
      · rename_i t
        simp only [WP.spec_ok, ClosureSpec]
        exact ⟨hall, hc⟩
      · simp only [WP.spec_ok, ClosureSpec]
        exact ⟨hall, by simpa [CoversSpec] using hc⟩
  · exact ⟨hi, hfound⟩

theorem closure_spec (refs : Slice compose.RefFact) (req granted : Slice (alloc.vec.Vec U8)) :
    compose.closure refs req granted ⦃ r => ClosureSpec refs.val req.val granted.val r ⦄ := by
  unfold compose.closure
  apply closure_loop_spec
  · simp
  · intro j hj; simp at hj

/-- Admitted exactly when every reference resolves and every required tool
is granted. -/
theorem closure_ok_iff (refs : List compose.RefFact) (req granted : List (alloc.vec.Vec U8))
    (r : core.result.Result Unit compose.Refusal) (h : ClosureSpec refs req granted r) :
    r = .Ok () ↔ AllFound refs ∧ Covered req granted := by
  rcases r with _ | (⟨i⟩ | ⟨i⟩)
  · simpa [ClosureSpec] using h
  · obtain ⟨hlt, hne, _⟩ := h
    simp only [reduceCtorEq, false_iff, not_and]
    intro hall
    exact absurd (hall i.val hlt) hne
  · obtain ⟨_, hlt, hmiss, _⟩ := h
    simp only [reduceCtorEq, false_iff, not_and]
    intro _ hc
    exact hmiss (hc i.val hlt)

/-- Adding a skill (more references, more required tools) never turns a
refused closure into an admitted one. -/
theorem more_skills_never_admit (refs refs' : List compose.RefFact)
    (req req' granted : List (alloc.vec.Vec U8))
    (r r' : core.result.Result Unit compose.Refusal)
    (h : ClosureSpec refs req granted r) (h' : ClosureSpec refs' req' granted r')
    (hrefs : refs.Sublist refs') (hreq : ∀ x ∈ req, x ∈ req')
    (hrefused : r ≠ .Ok ()) : r' ≠ .Ok () := by
  intro hok
  apply hrefused
  rw [closure_ok_iff refs req granted r h]
  obtain ⟨hall', hcov'⟩ := (closure_ok_iff refs' req' granted r' h').mp hok
  constructor
  · intro j hj
    have hm : refs[j] ∈ refs' := hrefs.subset (List.getElem_mem hj)
    obtain ⟨k, hk, hkj⟩ := List.getElem_of_mem hm
    rw [← hkj]; exact hall' k hk
  · intro j hj
    have hm : req[j] ∈ req' := hreq _ (List.getElem_mem hj)
    obtain ⟨k, hk, hkj⟩ := List.getElem_of_mem hm
    have := hcov' k hk
    rwa [hkj] at this

end Writfold.Core.Compose
