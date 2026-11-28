import Extracted.WritfoldCore.Claims
import Extracted.WritfoldCore.Types

/-!
# Claims inside text (#189)

Theorems over the Aeneas translation of `writfold_core::claims`.

- `accounted_spec`, `accounted_pass_iff`: `claims.accounted` passes exactly
  when there is at least one block and every block is cited and fresh,
  tagged or excused.
- `bare_never_passes`, `empty_never_passes`: one bare block fails; nothing
  to account for is never a pass.
- `remainder_is_bare`: a pass means no block is left over.
- `tag_is_not_a_citation`: turning a tagged block into a fresh citation,
  or back, changes no outcome; the note counts tags apart.
- `cited_spec`, `below_min_never_passes`, `no_min_unchanged`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Claims

/-- A block fails by itself. -/
def failsB : claims.Block → Bool
  | .Bare => true
  | .Cited .Stale => true
  | _ => false

/-- A block cannot be told. -/
def unsureB : claims.Block → Bool
  | .Cited .Unverifiable => true
  | _ => false

theorem fails_spec (b : claims.Block) : claims.fails b ⦃ r => r = failsB b ⦄ := by
  unfold claims.fails
  rcases b with s | _ | _ | _ <;> try cases s
  all_goals simp [failsB]

theorem unsure_spec (b : claims.Block) : claims.unsure b ⦃ r => r = unsureB b ⦄ := by
  unfold claims.unsure
  rcases b with s | _ | _ | _ <;> try cases s
  all_goals simp [unsureB]

/-- The rest of the walk from `l` with `u` already unknown. -/
def restOf (l : List claims.Block) (u : Bool) : gate.Outcome :=
  if l.any failsB then .Fail else if u || l.any unsureB then .Unknown else .Pass

/-- `claims.accounted`, as a model. -/
def accountedOf (bs : List claims.Block) : gate.Outcome :=
  if bs = [] then .Unknown else restOf bs false

theorem restOf_cons (b : claims.Block) (l : List claims.Block) (u : Bool) (h : failsB b = false) :
    restOf (b :: l) u = restOf l (u || unsureB b) := by
  unfold restOf
  simp only [List.any_cons, h, Bool.false_or, Bool.or_assoc]
  rfl

theorem accounted_loop_spec (bs : Slice claims.Block) (u : Bool) (i : Usize) (hi : i.val ≤ bs.val.length) :
    claims.accounted_loop bs u i ⦃ r => r = restOf (bs.val.drop i.val) u ⦄ := by
  unfold claims.accounted_loop
  apply loop.spec_decr_nat (fun q : Bool × Usize => bs.val.length - q.2.val)
    (fun q : Bool × Usize => q.2.val ≤ bs.val.length ∧
      restOf (bs.val.drop q.2.val) q.1 = restOf (bs.val.drop i.val) u)
  · rintro ⟨u', j⟩ ⟨hj, hinv⟩
    unfold claims.accounted_loop.body
    dsimp only at hj hinv ⊢
    split
    · rename_i hlt
      have hjl : j.val < bs.val.length := by scalar_tac
      have hdrop : bs.val.drop j.val = bs.val[j.val] :: bs.val.drop (j.val + 1) :=
        List.drop_eq_getElem_cons hjl
      step as ⟨ x, hx ⟩
      step with fails_spec as ⟨ f, hf ⟩
      split
      · rename_i hfail
        simp only [WP.spec_ok]
        have hb : failsB bs.val[j.val] = true := by rw [← hx, ← hf]; exact hfail
        rw [← hinv]
        have hmem : bs.val[j.val] ∈ bs.val.drop j.val := by
          rw [hdrop]; exact List.mem_cons_self
        have hany : (bs.val.drop j.val).any failsB = true := List.any_eq_true.mpr ⟨_, hmem, hb⟩
        simp only [restOf, hany, ↓reduceIte]
      · rename_i hok
        have hfb : failsB bs.val[j.val] = false := by rw [← hx, ← hf]; simpa using hok
        split
        · rename_i hu
          step as ⟨ j', hj' ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          have : j'.val = j.val + 1 := by scalar_tac
          rw [this, ← hinv, hdrop, restOf_cons _ _ _ hfb, hu]
          simp
        · rename_i hu
          step with unsure_spec as ⟨ s, hs ⟩
          step as ⟨ j', hj' ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          have : j'.val = j.val + 1 := by scalar_tac
          have hu' : u' = false := by simpa using hu
          rw [this, ← hinv, hdrop, restOf_cons _ _ _ hfb, hs, hx, hu']
          simp
    · rename_i hge
      have hlen : j.val = bs.val.length := by scalar_tac
      rw [← hinv, hlen, List.drop_length]
      cases u' <;> simp [restOf]
  · exact ⟨hi, rfl⟩

theorem accounted_spec (bs : Slice claims.Block) :
    claims.accounted bs ⦃ r => r = accountedOf bs.val ⦄ := by
  unfold claims.accounted
  step as ⟨ e, he ⟩
  split
  · simp_all [accountedOf]
  · rename_i hne
    have hbs : bs.val ≠ [] := by simp_all
    apply WP.spec_mono (accounted_loop_spec bs false 0#usize (by simp))
    intro r hr
    simp [accountedOf, hbs, hr]

/-- A block that accounts for itself. -/
def okB (b : claims.Block) : Prop :=
  b = .Cited .Fresh ∨ b = .Tagged ∨ b = .Excused

theorem okB_iff (b : claims.Block) : okB b ↔ failsB b = false ∧ unsureB b = false := by
  rcases b with s | _ | _ | _ <;> try cases s
  all_goals simp [okB, failsB, unsureB]

/-- Pass exactly when there is a block and every block is cited and fresh,
tagged or excused. -/
theorem accounted_pass_iff (bs : Slice claims.Block) :
    claims.accounted bs ⦃ r => r = .Pass ↔ bs.val ≠ [] ∧ ∀ b ∈ bs.val, okB b ⦄ :=
  WP.spec_mono (accounted_spec bs) (fun r hr => by
    subst hr
    unfold accountedOf restOf
    by_cases h : bs.val = []
    · simp [h]
    · simp only [h, ↓reduceIte, ne_eq, not_false_eq_true, true_and, Bool.false_or]
      constructor
      · intro hp b hb
        rw [okB_iff]
        by_cases hf : bs.val.any failsB = true
        · simp [hf] at hp
        · by_cases hu : bs.val.any unsureB = true
          · simp [hf, hu] at hp
          · simp only [List.any_eq_true, not_exists, not_and, Bool.not_eq_true] at hf hu
            exact ⟨hf b hb, hu b hb⟩
      · intro hall
        have hf : bs.val.any failsB = false := by
          simp only [List.any_eq_false]; intro b hb; simp [((okB_iff b).mp (hall b hb)).1]
        have hu : bs.val.any unsureB = false := by
          simp only [List.any_eq_false]; intro b hb; simp [((okB_iff b).mp (hall b hb)).2]
        simp [hf, hu])

/-- One bare block fails the check. -/
theorem bare_never_passes (bs : Slice claims.Block) (h : claims.Block.Bare ∈ bs.val) :
    claims.accounted bs ⦃ r => r = .Fail ⦄ :=
  WP.spec_mono (accounted_spec bs) (fun r hr => by
    subst hr
    have hne : bs.val ≠ [] := List.ne_nil_of_mem h
    have hf : bs.val.any failsB = true := List.any_eq_true.mpr ⟨_, h, rfl⟩
    simp [accountedOf, restOf, hne, hf])

/-- Nothing to account for is never a pass. -/
theorem empty_never_passes (bs : Slice claims.Block) (h : bs.val = []) :
    claims.accounted bs ⦃ r => r = .Unknown ⦄ :=
  WP.spec_mono (accounted_spec bs) (fun r hr => by simp [hr, accountedOf, h])

/-- A pass leaves no block over: no block is bare. -/
theorem remainder_is_bare (bs : Slice claims.Block) :
    claims.accounted bs ⦃ r => r = .Pass → claims.Block.Bare ∉ bs.val ⦄ :=
  WP.spec_mono (accounted_pass_iff bs) (fun r hr hp hb => by
    rcases (hr.mp hp).2 _ hb with h | h | h <;> cases h)

/-- A tag is not a citation, and not a lack of one: the outcome is the same. -/
def retag : claims.Block → claims.Block
  | .Tagged => .Cited .Fresh
  | b => b

theorem tag_is_not_a_citation (bs : Slice claims.Block) :
    claims.accounted bs ⦃ r => r = accountedOf (bs.val.map retag) ⦄ :=
  WP.spec_mono (accounted_spec bs) (fun r hr => by
    rw [hr]
    have hf : ∀ b, failsB (retag b) = failsB b := by
      intro b; rcases b with s | _ | _ | _ <;> try cases s
      all_goals rfl
    have hu : ∀ b, unsureB (retag b) = unsureB b := by
      intro b; rcases b with s | _ | _ | _ <;> try cases s
      all_goals rfl
    unfold accountedOf restOf
    simp [List.any_map, Function.comp_def, hf, hu])

theorem cited_spec (t m : Usize) (s : gate.Outcome) :
    claims.cited t m s ⦃ r => r = if t.val < m.val then .Fail else s ⦄ := by
  unfold claims.cited
  split <;> simp_all <;> scalar_tac

/-- Below the minimum, never a pass. -/
theorem below_min_never_passes (t m : Usize) (s : gate.Outcome) (h : t.val < m.val) :
    claims.cited t m s ⦃ r => r = .Fail ⦄ :=
  WP.spec_mono (cited_spec t m s) (fun r hr => by simp [hr, h])

/-- With no minimum the shipped outcome stands: an existing mode notices nothing. -/
theorem no_min_unchanged (t m : Usize) (s : gate.Outcome) (h : m.val = 0) :
    claims.cited t m s ⦃ r => r = s ⦄ :=
  WP.spec_mono (cited_spec t m s) (fun r hr => by simp [hr, h])

end Writfold.Core.Claims
