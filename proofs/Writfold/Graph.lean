import Writfold.Get

/-!
# Which graph facts a refresh rewrites (#59)

Theorems over the Aeneas translation of `writfold_core::graph`, and the
model fact that justifies its delta.

- `any_spec`: some file's references changed (`true ∈ changed`).
- `affected_iff` (#65): a file is affected when its references, its
  duplicate standing or one of its tokens' resolutions changed.
- `plan_skip_iff`, `plan_delta_iff`: a refresh rewrites the affected files'
  edges, or none when no file is affected. A key change no longer resolves
  everything.
- `delta_eq_full_keyed` (model, #65): with a file's edges local to its own
  tokens, keeping every unaffected file and resolving the affected ones
  against the new keys equals resolving every file.
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

/-- A file is affected exactly when its references changed, its
duplicate standing changed, or one of its tokens moved (#65). -/
theorem affected_iff (r d m : Bool) :
    graph.affected r d m ⦃ a => a = true ↔ r = true ∨ d = true ∨ m = true ⦄ := by
  unfold graph.affected
  cases r <;> cases d <;> cases m <;> simp

/-- Nothing is rewritten exactly when no file is affected. -/
theorem plan_skip_iff (xs : Slice Bool) :
    graph.plan xs ⦃ p => p = .Skip ↔ ∀ x ∈ xs.val, x = false ⦄ := by
  unfold graph.plan
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

/-- A delta is chosen exactly when some file is affected. -/
theorem plan_delta_iff (xs : Slice Bool) :
    graph.plan xs ⦃ p => p = .Delta ↔ true ∈ xs.val ⦄ := by
  unfold graph.plan
  step with any_spec as ⟨ b, hb ⟩
  split
  · rename_i hbt; simp only [WP.spec_ok, true_iff]; exact hb.mp hbt
  · rename_i hbf
    simp only [WP.spec_ok, reduceCtorEq, false_iff]
    exact fun h => hbf (hb.mpr h)

theorem plan_total (xs : Slice Bool) : graph.plan xs ⦃ _ => True ⦄ :=
  WP.spec_mono (plan_skip_iff xs) (fun _ _ => trivial)

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

/-! ## The keyed delta (#65), on the model

A file's edges depend on the keys only through how its own tokens
resolve, and on whether it lost a duplicate ID. -/

/-- One file before and after a refresh, with the keys' resolution. -/
structure KFile (R T : Type) where
  old : R
  new : R
  dupOld : Bool
  dupNew : Bool
  affected : Bool

/-- What the keyed delta keeps or recomputes for one file. -/
def keyedOf {R T D E : Type} (edges : (T → D) → Bool → R → List E)
    (resOld resNew : T → D) (f : KFile R T) : List E :=
  if f.affected then edges resNew f.dupNew f.new else edges resOld f.dupOld f.old

/-- With edges local to a file's tokens, and every unaffected file's
references, duplicate standing and token resolutions unchanged, the keyed
delta (old edges kept, affected files resolved against the new keys)
equals resolving every file against the new keys. -/
theorem delta_eq_full_keyed {R T D E : Type} (edges : (T → D) → Bool → R → List E)
    (tokens : R → List T) (resOld resNew : T → D) (files : List (KFile R T))
    (local_ : ∀ (r : R) (d : Bool) (a b : T → D),
      (∀ t ∈ tokens r, a t = b t) → edges a d r = edges b d r)
    (sound : ∀ f ∈ files, f.affected = false →
      f.old = f.new ∧ f.dupOld = f.dupNew ∧ ∀ t ∈ tokens f.new, resOld t = resNew t) :
    files.flatMap (keyedOf edges resOld resNew) =
      files.flatMap (fun f => edges resNew f.dupNew f.new) := by
  induction files with
  | nil => rfl
  | cons f rest ih =>
    simp only [List.flatMap_cons]
    rw [ih (fun g hg => sound g (List.mem_cons_of_mem _ hg))]
    congr 1
    unfold keyedOf
    cases ha : f.affected
    · obtain ⟨hr, hd, ht⟩ := sound f List.mem_cons_self ha
      simp only [Bool.false_eq_true, ite_false]
      rw [hr, hd]
      exact local_ f.new f.dupNew resOld resNew ht
    · simp

end Writfold.Core.Graph
