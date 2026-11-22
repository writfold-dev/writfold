import Extracted.WritfoldCore.Ground
import Extracted.WritfoldCore.Observe
import Extracted.WritfoldCore.Types
import Writfold.Current

/-!
# What grounds evidence

Theorems over the Aeneas translation of `writfold_core::ground` and of
`writfold_core::observe`'s `embeds` and `grounds`: what a cited reference
stands on, for an assessed gate's assumptions and a checklist gate's cover
rows alike.

- `of_gate_spec`, `of_gate_grounded_iff`, `of_gate_attested_iff`: a gate
  reference grounds only while the gate is satisfied by a check, and is
  attested only while it stands on a current attestation.
  `waiver_never_grounds`: a gate that stands on a waiver grounds nothing.
- `of_receipt_grounded_iff`, `of_receipt_never_attested`: a gate receipt
  grounds only while it is about this subject, under the current
  definition, about the present content, passed, and the gate stands met.
- `of_citations_grounded_iff`, `of_token_grounded_iff`: a retrieval grounds
  while it holds a citation and all are fresh; a token, only as exactly one
  fresh citation.
- `of_observation_grounded_iff`: an observation grounds only for an `ok` look
  with at least one citation, all fresh; `grounds_iff`, `embeds_iff`.
- `of_statement_spec`, `moved_iff`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Ground

/-! ## A gate -/

/-- `of_gate` on the model. -/
def ofGate (found : Bool) (s : gate.Standing) : grounding.Grounding :=
  if found then
    match s with
    | .Checked .Satisfied => .Grounded
    | .Attested => .Attested
    | _ => .Ungrounded
  else .Ungrounded

theorem of_gate_spec (found : Bool) (s : gate.Standing) :
    ground.of_gate found s ⦃ g => g = ofGate found s ⦄ := by
  unfold ground.of_gate ofGate
  cases found
  · simp
  · rcases s with st | _ | _
    · cases st <;> simp
    · simp
    · simp

theorem of_gate_grounded_iff (found : Bool) (s : gate.Standing) :
    ground.of_gate found s ⦃ g => g = .Grounded ↔ found = true ∧ s = .Checked .Satisfied ⦄ := by
  apply WP.spec_mono (of_gate_spec found s)
  intro g hg; subst hg
  cases found
  · simp [ofGate]
  · rcases s with st | _ | _
    · cases st <;> simp [ofGate]
    · simp [ofGate]
    · simp [ofGate]

theorem of_gate_attested_iff (found : Bool) (s : gate.Standing) :
    ground.of_gate found s ⦃ g => g = .Attested ↔ found = true ∧ s = .Attested ⦄ := by
  apply WP.spec_mono (of_gate_spec found s)
  intro g hg; subst hg
  cases found
  · simp [ofGate]
  · rcases s with st | _ | _
    · cases st <;> simp [ofGate]
    · simp [ofGate]
    · simp [ofGate]

/-- A waiver is never evidence: a gate that stands on a waiver grounds
nothing. -/
theorem waiver_never_grounds (found : Bool) :
    ground.of_gate found .Waived ⦃ g => g = .Ungrounded ⦄ := by
  apply WP.spec_mono (of_gate_spec found .Waived)
  intro g hg; subst hg
  cases found <;> simp [ofGate]

/-! ## A gate receipt -/

/-- A gate receipt grounds only while all five facts hold. -/
theorem of_receipt_grounded_iff (f : ground.ReceiptFacts) :
    ground.of_receipt f ⦃ g => g = .Grounded ↔
      f.same_subject = true ∧ f.same_definition = true ∧ f.same_content = true ∧
        f.passed = true ∧ f.gate_met = true ⦄ := by
  unfold ground.of_receipt
  rcases f with ⟨a, b, c, d, e⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> simp

/-- A receipt is a check, never an attestation. -/
theorem of_receipt_never_attested (f : ground.ReceiptFacts) :
    ground.of_receipt f ⦃ g => g ≠ .Attested ⦄ := by
  unfold ground.of_receipt
  rcases f with ⟨a, b, c, d, e⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> simp

/-! ## Citations -/

theorem of_citations_grounded_iff (n nf : Usize) :
    ground.of_citations n nf ⦃ g => g = .Grounded ↔ 0 < n.val ∧ nf.val = 0 ⦄ := by
  unfold ground.of_citations
  by_cases h : 0 < n.val ∧ nf.val = 0
  · have h1 : n > 0#usize := by scalar_tac
    have h2 : nf = 0#usize := by scalar_tac
    simp [h1, h2, h]
  · split <;> simp_all <;> scalar_tac

theorem of_citations_never_attested (n nf : Usize) :
    ground.of_citations n nf ⦃ g => g ≠ .Attested ⦄ := by
  unfold ground.of_citations
  repeat' split
  all_goals simp

theorem of_token_grounded_iff (n nf : Usize) :
    ground.of_token n nf ⦃ g => g = .Grounded ↔ n.val = 1 ∧ nf.val = 0 ⦄ := by
  unfold ground.of_token
  by_cases h : n.val = 1 ∧ nf.val = 0
  · have h1 : n = 1#usize := by scalar_tac
    have h2 : nf = 0#usize := by scalar_tac
    simp [h1, h2, h]
  · split <;> simp_all <;> scalar_tac

/-! ## Observations -/

theorem embeds_iff (s : observe.Status) :
    observe.embeds s ⦃ b => b = true ↔ s = .Ok ∨ s = .Partial ⦄ := by
  unfold observe.embeds
  cases s <;> simp

theorem grounds_iff (s : observe.Status) (c : Bool) :
    observe.grounds s c ⦃ b => b = true ↔ s = .Ok ∧ c = true ⦄ := by
  unfold observe.grounds
  cases s <;> simp

/-- An observation grounds only for an `ok` look with at least one embedded
citation, every one fresh; a partial, empty, failed or refused look never
does. -/
theorem of_observation_grounded_iff (s : observe.Status) (n nf : Usize) :
    ground.of_observation s n nf ⦃ g => g = .Grounded ↔
      s = .Ok ∧ 0 < n.val ∧ nf.val = 0 ⦄ := by
  unfold ground.of_observation
  have hb : (if n > 0#usize then ok (decide (nf = 0#usize)) else ok false : Result Bool) =
      ok (decide (0 < n.val ∧ nf.val = 0)) := by
    by_cases h1 : 0 < n.val
    · have : n > 0#usize := by scalar_tac
      simp only [this, ↓reduceIte, h1, true_and]
      by_cases h2 : nf.val = 0
      · have : nf = 0#usize := by scalar_tac
        simp [this, h2]
      · have : nf ≠ 0#usize := by intro h0; apply h2; rw [h0]; rfl
        simp [this, h2]
    · have : ¬ n > 0#usize := by scalar_tac
      simp [this, h1]
  rw [hb, bind_ok]
  apply WP.spec_bind (grounds_iff s _)
  intro b hb1
  cases b <;> simp_all

/-- Adding a citation to an observation, or one of its citations ceasing to
be fresh, never grounds a look that did not ground before: a snapshot
cannot revive an expired look. -/
theorem of_observation_antitone (s : observe.Status) (c c' n n' : Usize)
    (hc : 0 < c.val) (hn : n.val ≤ n'.val) :
    ground.of_observation s c' n' ⦃ g' => g' = .Grounded →
      ground.of_observation s c n ⦃ g => g = .Grounded ⦄ ⦄ := by
  apply WP.spec_mono (of_observation_grounded_iff s c' n')
  intro g' hg' hG
  apply WP.spec_mono (of_observation_grounded_iff s c n)
  intro g hg
  obtain ⟨hs, _, hnf⟩ := hg'.mp hG
  exact hg.mpr ⟨hs, hc, by omega⟩

/-! ## Attestations and moved references -/

theorem of_statement_spec (c : Bool) :
    ground.of_statement c ⦃ g => g = if c then .Attested else .Ungrounded ⦄ := by
  unfold ground.of_statement
  cases c <;> simp

/-- A recorded reference moved exactly when it no longer grounds or its
standing word changed. -/
theorem moved_iff (g : grounding.Grounding) (same : Bool) :
    ground.moved g same ⦃ b => b = true ↔ g = .Ungrounded ∨ same = false ⦄ := by
  unfold ground.moved
  cases g <;> cases same <;> simp

end Writfold.Core.Ground
