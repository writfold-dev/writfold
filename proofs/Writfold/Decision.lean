import Extracted.WritfoldCore

/-!
# Decision states and investigations

Theorems over the Aeneas translation of `writfold_core::decision::step`
and `writfold_core::investigation`.

- `step_spec`: `step` is exactly the table of `decision.rs` (`stepOf`),
  for every pair of states. From it: `never_written` (`landing`,
  `regressed` and anything outside the vocabulary are never written),
  `verified_needs_evidence`, `applied_needs_commit`,
  `accepted_needs_ready_and_actor`, `rejected_needs_actor`,
  `superseded_needs_successor` (and never from `superseded`): nothing
  skips a stage, and each change needs what the table says.
- `conclude_spec`: what a conclusion lacks: either section without text,
  and the number of hypotheses still open. `none_spec`,
  `conclude_none_iff`: an investigation may be concluded exactly when both
  sections have text and no hypothesis is open.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Decision

/-- The requirements, field by field. -/
def N (ready actor commit evidence successor : Bool) : decision.Needs :=
  { ready, actor, commit, evidence, successor }

/-- The table of `decision.rs`: what a written change needs. -/
def stepOf : decision.State → decision.State → Option decision.Needs
  | .Superseded, .Superseded => none
  | _, .Superseded => some (N false false false false true)
  | .Draft, .Proposed => some (N true false false false false)
  | .Proposed, .Draft => some (N false false false false false)
  | .Proposed, .Accepted => some (N true true false false false)
  | .Proposed, .Rejected => some (N false true false false false)
  | .Accepted, .Deferred => some (N false false false false false)
  | .Accepted, .Applied => some (N false false true false false)
  | .Applied, .Verified => some (N false false false true false)
  | _, _ => none

theorem step_spec (f t : decision.State) :
    decision.step f t ⦃ r => r = stepOf f t ⦄ := by
  unfold decision.step
  cases t <;> cases f <;> simp [decision.needs, stepOf, N]

/-- `landing`, `regressed` and any status outside the vocabulary are never
written. -/
theorem never_written (f t : decision.State)
    (h : t = .Landing ∨ t = .Regressed ∨ t = .Other) :
    decision.step f t ⦃ r => r = none ⦄ := by
  apply WP.spec_mono (step_spec f t)
  intro r hr
  rcases h with rfl | rfl | rfl <;> cases f <;> simp_all [stepOf]

/-- `verified` only from `applied`, and only with grounded evidence. -/
theorem verified_needs_evidence (f : decision.State) :
    decision.step f .Verified ⦃ r => (r ≠ none ↔ f = .Applied) ∧
      ∀ n, r = some n → n.evidence = true ⦄ := by
  apply WP.spec_mono (step_spec f .Verified)
  intro r hr; subst hr
  cases f <;> simp [stepOf, N]

/-- `applied` only from `accepted`, and only with a commit on the target. -/
theorem applied_needs_commit (f : decision.State) :
    decision.step f .Applied ⦃ r => (r ≠ none ↔ f = .Accepted) ∧
      ∀ n, r = some n → n.commit = true ⦄ := by
  apply WP.spec_mono (step_spec f .Applied)
  intro r hr; subst hr
  cases f <;> simp [stepOf, N]

/-- `accepted` only from `proposed`, and only when ready and with an actor. -/
theorem accepted_needs_ready_and_actor (f : decision.State) :
    decision.step f .Accepted ⦃ r => (r ≠ none ↔ f = .Proposed) ∧
      ∀ n, r = some n → n.ready = true ∧ n.actor = true ⦄ := by
  apply WP.spec_mono (step_spec f .Accepted)
  intro r hr; subst hr
  cases f <;> simp [stepOf, N]

/-- `rejected` only from `proposed`, and only with an actor. -/
theorem rejected_needs_actor (f : decision.State) :
    decision.step f .Rejected ⦃ r => (r ≠ none ↔ f = .Proposed) ∧
      ∀ n, r = some n → n.actor = true ⦄ := by
  apply WP.spec_mono (step_spec f .Rejected)
  intro r hr; subst hr
  cases f <;> simp [stepOf, N]

/-- `superseded` from anything but `superseded`, and only with a successor. -/
theorem superseded_needs_successor (f : decision.State) :
    decision.step f .Superseded ⦃ r => (r ≠ none ↔ f ≠ .Superseded) ∧
      ∀ n, r = some n → n.successor = true ⦄ := by
  apply WP.spec_mono (step_spec f .Superseded)
  intro r hr; subst hr
  cases f <;> simp [stepOf, N]

/-! ## Investigations -/

/-- A hypothesis still open. -/
def isOpen : investigation.Hypothesis → Bool
  | .Open => true
  | _ => false

theorem conclude_loop_spec (hs : Slice investigation.Hypothesis) (o i : Usize)
    (hi : i.val ≤ hs.val.length) (ho : o.val = (hs.val.take i.val).countP isOpen) :
    investigation.conclude_loop hs o i ⦃ r => r.val = hs.val.countP isOpen ⦄ := by
  unfold investigation.conclude_loop
  apply loop.spec_decr_nat (fun (x : Usize × Usize) => hs.val.length - x.2.val)
    (fun (x : Usize × Usize) => x.2.val ≤ hs.val.length ∧
      x.1.val = (hs.val.take x.2.val).countP isOpen)
  · rintro ⟨o, i⟩ ⟨hi, ho⟩
    dsimp only at hi ho
    unfold investigation.conclude_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ h, hh ⟩
      have hlt' : i.val < hs.val.length := by scalar_tac
      have htake : hs.val.take (i.val + 1) = hs.val.take i.val ++ [hs.val[i.val]] :=
        List.take_succ_eq_append_getElem hlt'
      have hle : (hs.val.take i.val).countP isOpen ≤ i.val := by
        have := List.countP_le_length (p := isOpen) (l := hs.val.take i.val)
        simp at this; omega
      rcases h with _ | _ | _ | _
      · step as ⟨ o1, ho1 ⟩
        step as ⟨ i1, hi1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, htake, List.countP_append, ← hh]
        simp [isOpen]; omega
      all_goals
        step as ⟨ i1, hi1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, htake, List.countP_append, ← hh]
        simp [isOpen]; omega
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = hs.val.length := by scalar_tac
      rw [ho, this, List.take_length]
  · exact ⟨hi, ho⟩

/-- What a conclusion lacks: each section without text, and how many
hypotheses are still open. -/
theorem conclude_spec (c s : Bool) (hs : Slice investigation.Hypothesis) :
    investigation.conclude c s hs ⦃ m => m.checked_against = !c ∧
      m.scope_and_limits = !s ∧ m.open.val = hs.val.countP isOpen ⦄ := by
  unfold investigation.conclude
  apply WP.spec_bind (conclude_loop_spec hs 0#usize 0#usize (by simp) (by simp))
  intro o ho
  simp [ho]

theorem none_spec (m : investigation.Missing) :
    investigation.Missing.none m ⦃ b => b = true ↔
      m.checked_against = false ∧ m.scope_and_limits = false ∧ m.open.val = 0 ⦄ := by
  unfold investigation.Missing.none
  rcases m with ⟨ca, sl, o⟩
  cases ca <;> cases sl <;> simp
  constructor
  · intro h; rw [h]; rfl
  · intro h; exact UScalar.eq_of_val_eq (by simpa using h)

/-- An investigation may be concluded exactly when both sections have text
and no hypothesis is open. -/
theorem conclude_none_iff (c s : Bool) (hs : Slice investigation.Hypothesis) :
    (do let m ← investigation.conclude c s hs; investigation.Missing.none m) ⦃ b =>
      b = true ↔ c = true ∧ s = true ∧ ∀ h ∈ hs.val, h ≠ .Open ⦄ := by
  step with conclude_spec as ⟨ m, hc, hsl, ho ⟩
  apply WP.spec_mono (none_spec m)
  intro b hb
  rw [hb, hc, hsl, ho, List.countP_eq_zero]
  cases c <;> cases s <;> simp
  constructor
  · intro h x hx hxo; subst hxo; simpa [isOpen] using h _ hx
  · intro h x hx; cases x <;> simp [isOpen]; exact h _ hx rfl

end Writfold.Core.Decision
