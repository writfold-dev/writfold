import Writfold.Bytes

/-!
# The Weft assessed gate's basis decision

Theorems over the Aeneas translation of `writfold_core::assess`'s decisions
(`assumption`, `basis`, `met`, `outcome` with `unfinished` and `found_met`).
Weft decides each check's result from the assumptions it was given;
Writfold decides whether those assumptions are grounded.

- An assumption is ungrounded when it cites nothing or any citation is
  ungrounded; attested when otherwise any citation is only attested;
  grounded otherwise (`assumption_spec`).
- A check's basis is grounded exactly when no premise is ungrounded
  (`basis_spec`); a check meets the gate exactly when Weft entailed it on a
  grounded basis (`met_spec`).
- The receipt's outcome is unknown when the run was incomplete, nothing was
  required, or Weft could not finish a required check; otherwise a pass
  exactly when every required check is entailed on a grounded basis
  (`outcome_spec`, `outcome_pass_iff`). A hypothetical entailment is never a
  pass (`hypothetical_never_passes`), so an assessed gate's `state` (Gate.lean)
  is never `satisfied` from it.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Assess

/-! ## `assumption` and `basis` -/

def isUngrounded : assess.Grounding → Bool
  | .Ungrounded => true
  | _ => false

def isAttested : assess.Grounding → Bool
  | .Attested => true
  | _ => false

/-- An assumption's grounding from its citations' groundings. -/
def assumptionOf (cs : List assess.Grounding) : assess.Grounding :=
  if cs = [] then .Ungrounded
  else if cs.any isUngrounded then .Ungrounded
  else if cs.any isAttested then .Attested
  else .Grounded

theorem assumption_loop_spec (cs : Slice assess.Grounding) (attested : Bool) (i : Usize)
    (hne : cs.val ≠ []) (hi : i.val ≤ cs.val.length)
    (hpre : (cs.val.take i.val).any isUngrounded = false)
    (hatt : attested = (cs.val.take i.val).any isAttested) :
    assess.assumption_loop cs attested i ⦃ g => g = assumptionOf cs.val ⦄ := by
  unfold assess.assumption_loop
  apply loop.spec_decr_nat (fun p : Bool × Usize => cs.val.length - p.2.val)
    (fun p : Bool × Usize => p.2.val ≤ cs.val.length ∧
      (cs.val.take p.2.val).any isUngrounded = false ∧
      p.1 = (cs.val.take p.2.val).any isAttested)
  · rintro ⟨attested, i⟩ ⟨hi, hpre, hatt⟩
    unfold assess.assumption_loop.body
    dsimp only at hi hpre hatt ⊢
    split
    · rename_i hlt
      have hlt' : i.val < cs.val.length := by scalar_tac
      have htake : cs.val.take (i.val + 1) = cs.val.take i.val ++ [cs.val[i.val]] := by
        rw [List.take_add_one, List.getElem?_eq_getElem hlt']
        simp
      step as ⟨ g, hg ⟩
      cases g with
      | Grounded =>
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        · rw [hi1, htake, ← hg]; simp [hpre, isUngrounded]
        · rw [hi1, htake, ← hg]; simp [hatt, isAttested]
      | Attested =>
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        · rw [hi1, htake, ← hg]; simp [hpre, isUngrounded]
        · rw [hi1, htake, ← hg]; simp [isAttested]
      | Ungrounded =>
        simp only [WP.spec_ok]
        unfold assumptionOf
        have hany : cs.val.any isUngrounded = true := by
          rw [List.any_eq_true]
          exact ⟨_, List.getElem_mem hlt', by rw [← hg]; rfl⟩
        simp [hne, hany]
    · rename_i hge
      have hall : cs.val.take i.val = cs.val := List.take_of_length_le (by scalar_tac)
      rw [hall] at hpre hatt
      unfold assumptionOf
      cases attested <;> simp [hne, hpre, ← hatt]
  · exact ⟨hi, hpre, hatt⟩

theorem assumption_spec (cs : Slice assess.Grounding) :
    assess.assumption cs ⦃ g => g = assumptionOf cs.val ⦄ := by
  unfold assess.assumption
  step as ⟨ e, he ⟩
  split
  · rename_i h
    simp only [WP.spec_ok]
    have : cs.val = [] := by
      apply List.eq_nil_of_length_eq_zero
      simpa [he, Slice.length] using h
    simp [assumptionOf, this]
  · rename_i h
    apply assumption_loop_spec
    · intro hnil; apply h; simp [he, Slice.length, hnil]
    · simp
    · simp
    · simp

theorem basis_loop_spec (ps : Slice assess.Grounding) (i : Usize) (hi : i.val ≤ ps.val.length)
    (hpre : (ps.val.take i.val).any isUngrounded = false) :
    assess.basis_loop ps i ⦃ b => b = .Grounded ↔ ps.val.any isUngrounded = false ⦄ := by
  unfold assess.basis_loop
  apply loop.spec_decr_nat (fun i : Usize => ps.val.length - i.val)
    (fun i : Usize => i.val ≤ ps.val.length ∧ (ps.val.take i.val).any isUngrounded = false)
  · intro i ⟨hi, hpre⟩
    unfold assess.basis_loop.body
    dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < ps.val.length := by scalar_tac
      step as ⟨ g, hg ⟩
      cases g with
      | Ungrounded =>
        simp only [bind_ok, ↓reduceIte, WP.spec_ok, reduceCtorEq, false_iff,
          Bool.not_eq_false]
        rw [List.any_eq_true]
        exact ⟨_, List.getElem_mem hlt', by rw [← hg]; rfl⟩
      | _ =>
        simp only [bind_ok, Bool.false_eq_true, ↓reduceIte]
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        rw [hi1, List.take_add_one, List.getElem?_eq_getElem hlt', ← hg]
        simp [hpre, isUngrounded]
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      rwa [List.take_of_length_le (by scalar_tac)] at hpre
  · exact ⟨hi, hpre⟩

/-- A check's basis is grounded exactly when no premise is ungrounded (and
`Hypothetical` otherwise: the type has two values). -/
theorem basis_spec (ps : Slice assess.Grounding) :
    assess.basis ps ⦃ b => b = .Grounded ↔ ∀ g ∈ ps.val, g ≠ .Ungrounded ⦄ := by
  unfold assess.basis
  apply WP.spec_mono (basis_loop_spec ps 0#usize (by simp) (by simp))
  intro b hb
  rw [hb, Bool.eq_false_iff, ne_eq, List.any_eq_true]
  constructor
  · intro h g hg hu
    exact h ⟨g, hg, by subst hu; rfl⟩
  · rintro h ⟨g, hg, hu⟩
    cases g <;> simp [isUngrounded] at hu
    exact h _ hg rfl

/-- A check meets the gate exactly when Weft entailed it on a grounded
basis. -/
theorem met_spec (v : assess.Verdict) (b : assess.Basis) :
    assess.met v b ⦃ r => r = true ↔ v = .Entailed ∧ b = .Grounded ⦄ := by
  unfold assess.met
  cases v <;> cases b <;> simp

/-! ## `outcome` -/

/-- Weft could not finish this required check. -/
def unfinishedB : assess.Required → Bool
  | .Found .Unknown _ => true
  | .Found .NotEvaluated _ => true
  | _ => false

/-- This required check is present, entailed and grounded. -/
def foundMetB : assess.Required → Bool
  | .Found .Entailed .Grounded => true
  | _ => false

theorem unfinished_spec (r : assess.Required) :
    assess.unfinished r ⦃ b => b = unfinishedB r ⦄ := by
  unfold assess.unfinished
  rcases r with _ | ⟨v, b⟩
  · simp [unfinishedB]
  · cases v <;> simp [unfinishedB]

theorem found_met_spec (r : assess.Required) :
    assess.found_met r ⦃ b => b = foundMetB r ⦄ := by
  unfold assess.found_met
  rcases r with _ | ⟨v, b⟩
  · simp [foundMetB]
  · apply WP.spec_mono (met_spec v b)
    intro x hx
    cases v <;> cases b <;> cases x <;> simp_all [foundMetB]

/-- The receipt's outcome. -/
def outcomeOf (complete : Bool) (req : List assess.Required) : gate.Outcome :=
  if complete = false ∨ req = [] then .Unknown
  else if req.any unfinishedB then .Unknown
  else if req.all foundMetB then .Pass
  else .Fail

theorem outcome_loop_spec (req : Slice assess.Required) (all : Bool) (i : Usize)
    (hne : req.val ≠ []) (hi : i.val ≤ req.val.length)
    (hpre : (req.val.take i.val).any unfinishedB = false)
    (hall : all = (req.val.take i.val).all foundMetB) :
    assess.outcome_loop req all i ⦃ o => o = outcomeOf true req.val ⦄ := by
  unfold assess.outcome_loop
  apply loop.spec_decr_nat (fun p : Bool × Usize => req.val.length - p.2.val)
    (fun p : Bool × Usize => p.2.val ≤ req.val.length ∧
      (req.val.take p.2.val).any unfinishedB = false ∧
      p.1 = (req.val.take p.2.val).all foundMetB)
  · rintro ⟨all, i⟩ ⟨hi, hpre, hall⟩
    unfold assess.outcome_loop.body
    dsimp only at hi hpre hall ⊢
    split
    · rename_i hlt
      have hlt' : i.val < req.val.length := by scalar_tac
      have htake : req.val.take (i.val + 1) = req.val.take i.val ++ [req.val[i.val]] := by
        rw [List.take_add_one, List.getElem?_eq_getElem hlt']
        simp
      step as ⟨ r, hr ⟩
      step with unfinished_spec as ⟨ u, hu ⟩
      split
      · rename_i hut
        simp only [WP.spec_ok]
        have hany : req.val.any unfinishedB = true := by
          rw [List.any_eq_true]
          exact ⟨_, List.getElem_mem hlt', by rw [← hr, ← hu, hut]⟩
        simp [outcomeOf, hne, hany]
      · rename_i hut
        step with found_met_spec as ⟨ m, hm ⟩
        have hu' : unfinishedB req.val[i.val] = false := by
          rw [← hr, ← hu]; simpa using hut
        cases m <;> simp only [Bool.false_eq_true, ↓reduceIte, bind_ok]
        all_goals
          apply WP.spec_bind (Usize.add_spec (by scalar_tac))
          intro i' hi'
          have hi1 : i'.val = i.val + 1 := by scalar_tac
          simp only [WP.spec_ok]
          and_intros
          all_goals try scalar_tac
          · rw [hi1, htake, List.any_append]; simp [hpre, hu']
          · rw [hi1, htake, List.all_append, ← hall]; simp [← hr, ← hm]
    · rename_i hge
      have htk : req.val.take i.val = req.val := List.take_of_length_le (by scalar_tac)
      rw [htk] at hpre hall
      unfold outcomeOf
      cases all <;> simp [hne, hpre, ← hall]
  · exact ⟨hi, hpre, hall⟩

theorem outcome_spec (complete : Bool) (req : Slice assess.Required) :
    assess.outcome complete req ⦃ o => o = outcomeOf complete req.val ⦄ := by
  unfold assess.outcome
  split
  · rename_i hc
    step as ⟨ e, he ⟩
    split
    · rename_i h
      simp only [WP.spec_ok]
      have : req.val = [] := by
        apply List.eq_nil_of_length_eq_zero
        simpa [he, Slice.length] using h
      simp [outcomeOf, this]
    · rename_i h
      have hne : req.val ≠ [] := by
        intro hnil; apply h; simp [he, Slice.length, hnil]
      have := outcome_loop_spec req true 0#usize hne (by simp) (by simp) (by simp)
      simpa [hc] using this
  · rename_i hc
    simp [outcomeOf, hc]

/-- The assessed gate passes exactly when the run completed and every required
check (at least one) is entailed on a grounded basis. -/
theorem outcome_pass_iff (complete : Bool) (req : Slice assess.Required) :
    assess.outcome complete req ⦃ o => o = .Pass ↔
      complete = true ∧ req.val ≠ [] ∧ ∀ r ∈ req.val, r = .Found .Entailed .Grounded ⦄ := by
  apply WP.spec_mono (outcome_spec complete req)
  intro o ho
  subst ho
  unfold outcomeOf
  have hfm : ∀ r, foundMetB r = true ↔ r = .Found .Entailed .Grounded := by
    intro r; rcases r with _ | ⟨v, b⟩ <;> [simp [foundMetB]; (cases v <;> cases b <;> simp [foundMetB])]
  have hfu : ∀ r, foundMetB r = true → unfinishedB r = false := by
    intro r h; rw [hfm] at h; subst h; rfl
  by_cases hc : complete = false
  · simp [hc]
  · by_cases hn : req.val = []
    · simp [hn]
    · rw [if_neg (show ¬(complete = false ∨ req.val = []) by tauto)]
      have hct : complete = true := by simpa using hc
      simp only [hct, ne_eq, hn, not_false_eq_true, true_and]
      split
      · rename_i hany
        simp only [reduceCtorEq, false_iff]
        intro hall
        rw [List.any_eq_true] at hany
        obtain ⟨r, hr, hu⟩ := hany
        have := hfu r ((hfm r).mpr (hall r hr))
        simp_all
      · split
        · rename_i _ hall
          simp only [true_iff]
          intro r hr
          exact (hfm r).mp (List.all_eq_true.mp hall r hr)
        · rename_i _ hall
          simp only [reduceCtorEq, false_iff]
          intro h
          apply hall
          exact List.all_eq_true.mpr (fun r hr => (hfm r).mpr (h r hr))

/-- A hypothetical entailment is never a pass: an assessed receipt resting on
an ungrounded assumption is never evidence of success. -/
theorem hypothetical_never_passes (complete : Bool) (req : Slice assess.Required)
    (v : assess.Verdict) (h : assess.Required.Found v .Hypothetical ∈ req.val) :
    assess.outcome complete req ⦃ o => o ≠ .Pass ⦄ := by
  apply WP.spec_mono (outcome_pass_iff complete req)
  intro o ho hpass
  have := (ho.mp hpass).2.2 _ h
  simp at this

end Writfold.Core.Assess
