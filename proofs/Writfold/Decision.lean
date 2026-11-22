import Extracted.WritfoldCore.Decision
import Extracted.WritfoldCore.Investigation
import Extracted.WritfoldCore.Types

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
- `flags_none_iff`: `status --set` takes its flags exactly when each one
  the change needs is given (`--by`, `--commit`, `--evidence`) and none it
  does not take (`--commit`, `--evidence`, `--run`/`--repo`,
  `--superseded-by`). `writes_spec`: the change is written exactly when
  every fact it needs holds (`Writes`). Composed with the table:
  `verified_written_only_grounded`, `applied_written_only_on_ref`,
  `accepted_written_only_ready`, `superseded_written_only_to_successor`.
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

/-! ## Flags and facts (`status --set`) -/

theorem flags_none_iff (n : decision.Needs) (g : decision.Given) :
    decision.flags n g ⦃ r => r = none ↔
      (n.actor = true → g.actor = true) ∧ g.commit = n.commit ∧ g.evidence = n.evidence ∧
      (g.run_or_repo = true → n.evidence = true) ∧ (g.successor = true → n.successor = true) ⦄ := by
  unfold decision.flags
  rcases n with ⟨r, a, c, e, s⟩
  rcases g with ⟨ga, gc, ge, gr, gs⟩
  cases a <;> cases ga <;> cases c <;> cases gc <;> cases e <;> cases ge <;> cases gr <;>
    cases s <;> cases gs <;> simp

/-- A missing actor is named first. -/
theorem flags_actor_first (n : decision.Needs) (g : decision.Given)
    (ha : n.actor = true) (hg : g.actor = false) :
    decision.flags n g ⦃ r => r = some .NeedsActor ⦄ := by
  unfold decision.flags
  simp [ha, hg]

/-- The facts a change needs, all holding. -/
def Writes (n : decision.Needs) (f : decision.Facts) : Prop :=
  (n.ready = true → f.ready = true) ∧ (n.commit = true → f.on_ref = true) ∧
  (n.evidence = true → f.grounded = true) ∧ (n.successor = true → f.successor = true)

theorem writes_spec (n : decision.Needs) (f : decision.Facts) :
    decision.writes n f ⦃ b => b = true ↔ Writes n f ⦄ := by
  unfold decision.writes Writes
  rcases n with ⟨r, a, c, e, s⟩
  rcases f with ⟨fr, fo, fg, fs⟩
  cases r <;> cases fr <;> cases c <;> cases fo <;> cases e <;> cases fg <;> cases s <;>
    cases fs <;> simp

/-- `verified` is written only from `applied`, on evidence that grounds. -/
theorem verified_written_only_grounded (fr : decision.State) (n : decision.Needs)
    (f : decision.Facts) (hs : stepOf fr .Verified = some n) (hw : Writes n f) :
    fr = .Applied ∧ f.grounded = true := by
  cases fr <;> simp [stepOf, N] at hs <;> subst hs <;> exact ⟨rfl, hw.2.2.1 rfl⟩

/-- `applied` is written only from `accepted`, for a commit on its target ref. -/
theorem applied_written_only_on_ref (fr : decision.State) (n : decision.Needs)
    (f : decision.Facts) (hs : stepOf fr .Applied = some n) (hw : Writes n f) :
    fr = .Accepted ∧ f.on_ref = true := by
  cases fr <;> simp [stepOf, N] at hs <;> subst hs <;> exact ⟨rfl, hw.2.1 rfl⟩

/-- `accepted` is written only from `proposed`, when the record is ready and
`--by` was given. -/
theorem accepted_written_only_ready (fr : decision.State) (n : decision.Needs)
    (f : decision.Facts) (g : decision.Given) (hs : stepOf fr .Accepted = some n)
    (hw : Writes n f) (hf : (n.actor = true → g.actor = true)) :
    fr = .Proposed ∧ f.ready = true ∧ g.actor = true := by
  cases fr <;> simp [stepOf, N] at hs <;> subst hs <;> exact ⟨rfl, hw.1 rfl, hf rfl⟩

/-- `superseded` is written only when the successor exists. -/
theorem superseded_written_only_to_successor (fr : decision.State) (n : decision.Needs)
    (f : decision.Facts) (hs : stepOf fr .Superseded = some n) (hw : Writes n f) :
    f.successor = true := by
  cases fr <;> simp [stepOf, N] at hs <;> subst hs <;> exact hw.2.2.2 rfl

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
