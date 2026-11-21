import Writfold.Get

/-!
# Which graph facts a refresh rewrites (#59)

Theorems over the Aeneas translation of `writfold_core::graph`, and the
model fact that justifies its delta.

- `any_spec`: some file's references changed (`true ∈ changed`).
- `plan_whole_iff`, `plan_skip_iff`, `plan_delta_iff`: a key change resolves
  everything; with the keys unchanged, a refresh rewrites the changed files'
  edges, or none when no reference changed. `Delta` is never chosen when a
  key changed.
- `delta_eq_full` (model, not extracted): edges are the concatenation over
  files of a function of each file's references and the shared keys (edges
  partition by `prov_path`). With the keys unchanged, keeping every file
  whose references did not change and recomputing the others equals
  resolving every file. `frame`: an unaffected file's edges are unchanged.
  That the SQL (`PHASE_B_EDGES_OF`) computes the per-file function is tested
  against a full rebuild, not proved.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Graph

theorem any_loop_spec (xs : Slice Bool) (i : Usize)
    (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = false) :
    graph.any_loop xs i ⦃ b => b = true ↔ true ∈ xs.val ⦄ := by
  unfold graph.any_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold graph.any_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hmem : xs.val[i.val]'(by scalar_tac) ∈ xs.val := List.getElem_mem _
      split
      · rename_i hxt
        simp only [WP.spec_ok, true_iff]
        have h1 : xs.val[i.val]'(by scalar_tac) = true := by rw [← hx]; exact hxt
        rw [← h1]; exact hmem
      · rename_i hxf
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; simpa using hxf
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro hmem
      obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hmem
      have := hprefix j (by scalar_tac) hj
      simp_all
  · exact ⟨hi, hprefix⟩

theorem any_spec (xs : Slice Bool) :
    graph.any xs ⦃ b => b = true ↔ true ∈ xs.val ⦄ := by
  unfold graph.any
  exact any_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem any_total (xs : Slice Bool) : graph.any xs ⦃ _ => True ⦄ :=
  WP.spec_mono (any_spec xs) (fun _ _ => trivial)

/-- A key change resolves everything. -/
theorem plan_whole_iff (k : Bool) (xs : Slice Bool) :
    graph.plan k xs ⦃ p => p = .Whole ↔ k = false ⦄ := by
  unfold graph.plan
  cases k
  · simp
  · rw [if_pos rfl]
    step with any_spec as ⟨ b, hb ⟩
    repeat' split
    all_goals simp

/-- Nothing is rewritten exactly when the keys and every file's
references are unchanged. -/
theorem plan_skip_iff (k : Bool) (xs : Slice Bool) :
    graph.plan k xs ⦃ p => p = .Skip ↔ k = true ∧ ∀ x ∈ xs.val, x = false ⦄ := by
  unfold graph.plan
  cases k
  · simp
  · simp only [true_and]
    rw [if_pos trivial]
    step with any_spec as ⟨ b, hb ⟩
    split
    · rename_i hbt
      simp only [WP.spec_ok, reduceCtorEq, false_iff, not_forall]
      exact ⟨true, hb.mp hbt, by simp⟩
    · rename_i hbf
      simp only [WP.spec_ok, true_iff]
      intro x hx
      cases x
      · rfl
      · exact absurd (hb.mpr hx) hbf

/-- A delta is chosen exactly when the keys are unchanged and some file's
references changed. -/
theorem plan_delta_iff (k : Bool) (xs : Slice Bool) :
    graph.plan k xs ⦃ p => p = .Delta ↔ k = true ∧ true ∈ xs.val ⦄ := by
  unfold graph.plan
  cases k
  · simp
  · simp only [true_and]
    rw [if_pos trivial]
    step with any_spec as ⟨ b, hb ⟩
    split
    · rename_i hbt; simp only [WP.spec_ok, true_iff]; exact hb.mp hbt
    · rename_i hbf
      simp only [WP.spec_ok, reduceCtorEq, false_iff]
      exact fun h => hbf (hb.mpr h)

theorem plan_total (k : Bool) (xs : Slice Bool) : graph.plan k xs ⦃ _ => True ⦄ :=
  WP.spec_mono (plan_whole_iff k xs) (fun _ _ => trivial)

/-! ## The delta, on the model -/

/-- One file before and after a refresh: its references, and whether they
changed. -/
structure File (R : Type) where
  old : R
  new : R
  changed : Bool

/-- What the delta keeps or recomputes for one file. -/
def deltaOf {R E K : Type} (edges : K → R → List E) (k : K) (f : File R) : List E :=
  if f.changed then edges k f.new else edges k f.old

/-- An unaffected file's edges are unchanged by the delta. -/
theorem frame {R E K : Type} (edges : K → R → List E) (k : K) (f : File R)
    (h : f.changed = false) : deltaOf edges k f = edges k f.old := by
  simp [deltaOf, h]

/-- With the keys unchanged, and a file's references unchanged whenever it
is not marked changed, the delta equals resolving every file anew. -/
theorem delta_eq_full {R E K : Type} (edges : K → R → List E) (k : K) (files : List (File R))
    (sound : ∀ f ∈ files, f.changed = false → f.old = f.new) :
    files.flatMap (deltaOf edges k) = files.flatMap (fun f => edges k f.new) := by
  induction files with
  | nil => rfl
  | cons f rest ih =>
    simp only [List.flatMap_cons]
    have hrest := ih (fun g hg => sound g (List.mem_cons_of_mem _ hg))
    rw [hrest]
    congr 1
    unfold deltaOf
    cases hc : f.changed
    · simp [sound f List.mem_cons_self hc]
    · simp

end Writfold.Core.Graph
