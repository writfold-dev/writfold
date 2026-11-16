import Writfold.Assess

/-!
# Retrieval is not evidence

These statements concern the extracted Rust boundary, not a second decision
implementation. A checked source preserves exactly the supplied prerequisite;
semantic and lexical retrieval pointers cannot become evidence even if their
proposed grounding is `Grounded`. Scores, freshness metadata and purpose cannot
alter this decision because they are not authority inputs. A later independent
source check is `CheckedSource`, irrespective of how the source was discovered.

The caller's classification and source/IO checks are not proved here. The nested
postconditions below compose the existing assumption and assessed-gate kernels;
they do not claim to verify checklist storage, citation parsing, or Weft.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.SemanticEvidence

/-- The semantic mutation is calibrated against this theorem alone. -/
theorem semantic_never_grounds (checked : assess.Grounding) :
    semantic_evidence.grounding .SemanticRetrieval checked
      ⦃ g => g = .Ungrounded ⦄ := by
  cases checked <;> simp [semantic_evidence.grounding]

/-- Lexical retrieval is likewise a pointer, not an independent source check. -/
theorem lexical_never_grounds (checked : assess.Grounding) :
    semantic_evidence.grounding .LexicalRetrieval checked
      ⦃ g => g = .Ungrounded ⦄ := by
  cases checked <;> simp [semantic_evidence.grounding]

/-- The checked-negative mutation is calibrated against this theorem alone. -/
theorem checked_preserved (checked : assess.Grounding) :
    semantic_evidence.grounding .CheckedSource checked
      ⦃ g => g = checked ⦄ := by
  cases checked <;> simp [semantic_evidence.grounding]

/-- Exact acceptance condition used by consumers that accept attested evidence,
including checklist support: real checked origin and a nonnegative prerequisite. -/
theorem accepted_iff (origin : semantic_evidence.Origin) (checked : assess.Grounding) :
    semantic_evidence.grounding origin checked
      ⦃ g => g ≠ .Ungrounded ↔ origin = .CheckedSource ∧ checked ≠ .Ungrounded ⦄ := by
  cases origin with
  | CheckedSource =>
    apply WP.spec_mono (checked_preserved checked)
    intro g hg
    simp [hg]
  | LexicalRetrieval =>
    apply WP.spec_mono (lexical_never_grounds checked)
    intro g hg
    simp [hg]
  | SemanticRetrieval =>
    apply WP.spec_mono (semantic_never_grounds checked)
    intro g hg
    simp [hg]

/-- Strictly grounded consumers cannot upgrade an attestation either. -/
theorem grounded_iff (origin : semantic_evidence.Origin) (checked : assess.Grounding) :
    semantic_evidence.grounding origin checked
      ⦃ g => g = .Grounded ↔ origin = .CheckedSource ∧ checked = .Grounded ⦄ := by
  cases origin with
  | CheckedSource =>
    apply WP.spec_mono (checked_preserved checked)
    intro g hg
    simp [hg]
  | LexicalRetrieval =>
    apply WP.spec_mono (lexical_never_grounds checked)
    intro g hg
    simp [hg]
  | SemanticRetrieval =>
    apply WP.spec_mono (semantic_never_grounds checked)
    intro g hg
    simp [hg]

/-- A semantic pointer included among an assumption's citations makes the actual
assumption kernel return ungrounded, even alongside other grounded citations. -/
theorem semantic_assumption_ungrounded (checked : assess.Grounding) :
    semantic_evidence.grounding .SemanticRetrieval checked
      ⦃ g => ∀ cs : Slice assess.Grounding, g ∈ cs.val →
        assess.assumption cs ⦃ a => a = .Ungrounded ⦄ ⦄ := by
  apply WP.spec_mono (semantic_never_grounds checked)
  intro g hg cs hmem
  subst g
  apply WP.spec_mono (Assess.assumption_spec cs)
  intro a ha
  have hany : cs.val.any Assess.isUngrounded = true :=
    List.any_eq_true.mpr ⟨.Ungrounded, hmem, rfl⟩
  simpa [Assess.assumptionOf, hany] using ha

/-- If a semantic pointer is a required premise, even an entailed check cannot
meet the assessed gate. This composes the shipped basis and met decisions. -/
theorem semantic_premise_never_meets (checked : assess.Grounding) :
    semantic_evidence.grounding .SemanticRetrieval checked
      ⦃ g => ∀ (ps : Slice assess.Grounding) (v : assess.Verdict), g ∈ ps.val →
        (do
          let b ← assess.basis ps
          assess.met v b) ⦃ r => r = false ⦄ ⦄ := by
  apply WP.spec_mono (semantic_never_grounds checked)
  intro g hg ps v hmem
  subst g
  apply WP.spec_bind (Assess.basis_spec ps)
  intro b hb
  have hnot : b ≠ .Grounded := by
    intro hgrounded
    exact (hb.mp hgrounded .Ungrounded hmem) rfl
  have hbase : b = .Hypothetical := by
    cases b <;> simp_all
  subst b
  apply WP.spec_mono (Assess.met_spec v .Hypothetical)
  intro r hr
  cases r <;> simp_all

end Writfold.Core.SemanticEvidence
