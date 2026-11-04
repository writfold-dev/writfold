import Kctx.Citation.Correspondence
import Kctx.Citation.ResolutionProofs

set_option autoImplicit false
namespace Kctx.Citation

theorem diagnose_ne_unique (bytes needle : List UInt8) (budget : Nat)
    (target : Binding) (witness : CorrespondenceWitness) :
    diagnose bytes needle budget ≠ .uniqueJustified target witness := by
  dsimp only [diagnose]
  split <;> simp

/-- Unique justification can only come from checked identity of the occurrence,
with independent successful resolution of both bindings, never from search. -/
theorem compare_unique_identity (store : Store) (access : Access)
    (original target successor : Binding) (witness : Option CorrespondenceWitness)
    (proof : CorrespondenceWitness) (budget : Nat)
    (h : compare store access original target witness budget = .uniqueJustified successor proof) :
    successor = target ∧ proof = .identicalPosition ∧ witness = some .identicalPosition ∧
    SameOccurrence original target ∧
    resolve store access original = .exact original.expected original.representation ∧
    resolve store access target = .exact target.expected target.representation := by
  unfold compare at h
  split at h
  · contradiction
  · cases resolved : resolve store access original with
    | denied => simp [resolved] at h
    | invalid reason => simp [resolved] at h
    | unavailable => simp [resolved] at h
    | exact bytes reference =>
      simp only [resolved] at h
      obtain ⟨_, _, _, provenance, selected, _⟩ :=
        resolve_exact store access original bytes reference resolved
      have originalResolved : resolve store access original =
          .exact original.expected original.representation := by
        simpa [provenance, selected] using resolved
      split at h
      · contradiction
      · split at h
        · contradiction
        · split at h
          · contradiction
          · rename_i object found
            split at h
            · contradiction
            · split at h
              · rename_i identity
                cases h
                exact ⟨rfl, rfl, identity.1, identity.2.1,
                  by simpa only [resolved] using originalResolved, identity.2.2⟩
              · exact False.elim (diagnose_ne_unique object.bytes original.expected
                  budget successor proof h)

theorem compare_text_only_ne_unique (store : Store) (access : Access)
    (original target successor : Binding) (proof : CorrespondenceWitness) (budget : Nat) :
    compare store access original target none budget ≠ .uniqueJustified successor proof := by
  intro h
  have identity := compare_unique_identity store access original target successor none proof budget h
  cases identity.2.2.1

theorem compare_sameBinding_exact (store : Store) (access : Access)
    (original target : Binding) (witness : Option CorrespondenceWitness) (budget : Nat)
    (h : compare store access original target witness budget = .sameBinding) :
    original = target := by
  unfold compare at h
  split at h
  · contradiction
  · cases resolved : resolve store access original with
    | denied => simp [resolved] at h
    | invalid reason => simp [resolved] at h
    | unavailable => simp [resolved] at h
    | exact bytes reference =>
      simp only [resolved] at h
      split at h
      · contradiction
      · split at h
        · contradiction
        · split at h
          · contradiction
          · split at h
            · assumption
            · split at h
              · contradiction
              · dsimp only [diagnose] at h
                split at h <;> contradiction

/-- Coverage reports exactly the enumerated prefix length, bounded by the budget. -/
theorem scanCoverage_examined (bytes needle : List UInt8) (budget : Nat) :
    (scanCoverage bytes needle budget).examined =
      min budget (possiblePositions bytes needle).length := by
  simp [scanCoverage, Coverage.examined, examinedPositions, List.length_take]

/-- An honest candidate is an actual examined byte position with exact text. -/
theorem textCandidates_sound (bytes needle : List UInt8) (budget : Nat)
    (selector : Selector) (h : selector ∈ textCandidates bytes needle budget) :
    ∃ offset, offset ∈ examinedPositions bytes needle budget ∧
      selector = positionSelector offset needle.length ∧ selectedBytes bytes selector = needle := by
  simp only [textCandidates, List.mem_map, List.mem_filter, decide_eq_true_eq] at h
  obtain ⟨offset, ⟨examined, exactText⟩, equal⟩ := h
  exact ⟨offset, examined, equal.symm, equal ▸ exactText⟩

/-- A truncated search with fewer than two candidates reports incompleteness,
not absence. With two or more it reports only the observed ambiguity. -/
theorem incomplete_search_not_absence (bytes needle : List UInt8) (budget : Nat)
    (incomplete : (scanCoverage bytes needle budget).complete = false)
    (few : (textCandidates bytes needle budget).length < 2) :
    diagnose bytes needle budget = .unknown .incompleteSearch
      (textCandidates bytes needle budget) (scanCoverage bytes needle budget) := by
  simp [diagnose, Nat.not_le.mpr few, incomplete]

/-- Every emitted diagnostic has byte units and lies wholly inside the target. -/
theorem textCandidates_bounds (bytes needle : List UInt8) (budget : Nat)
    (selector : Selector) (h : selector ∈ textCandidates bytes needle budget) :
    selector.unit = .byte ∧ selector.first ≤ selector.last ∧
      selector.last ≤ bytes.length := by
  obtain ⟨offset, examined, equal, _⟩ := textCandidates_sound bytes needle budget selector h
  have possible : offset ∈ possiblePositions bytes needle :=
    List.mem_of_mem_take examined
  unfold possiblePositions at possible
  split at possible
  · rename_i fits
    have bound : offset < bytes.length - needle.length + 1 := List.mem_range.mp possible
    subst selector
    change CoordinateUnit.byte = .byte ∧ offset ≤ offset + needle.length ∧
      offset + needle.length ≤ bytes.length
    exact ⟨rfl, by omega, by omega⟩
  · simp at possible

/-- Every exact match in the examined prefix is retained; diagnostic filtering
does not silently discard inconvenient duplicated occurrences. -/
theorem textCandidates_complete (bytes needle : List UInt8) (budget offset : Nat)
    (examined : offset ∈ examinedPositions bytes needle budget)
    (exactText : selectedBytes bytes (positionSelector offset needle.length) = needle) :
    positionSelector offset needle.length ∈ textCandidates bytes needle budget := by
  apply List.mem_map.mpr
  refine ⟨offset, ?_, rfl⟩
  exact List.mem_filter.mpr ⟨examined, by simpa using exactText⟩

/-- Reporting multiple candidates requires at least two actually found offsets. -/
theorem diagnose_multiple_observed (bytes needle : List UInt8) (budget : Nat)
    (candidates : List Selector) (coverage : Coverage)
    (h : diagnose bytes needle budget = .multiple candidates coverage) :
    candidates = textCandidates bytes needle budget ∧
    coverage = scanCoverage bytes needle budget ∧ 2 ≤ candidates.length := by
  dsimp only [diagnose] at h
  split at h
  · cases h
    exact ⟨rfl, rfl, by assumption⟩
  · contradiction

end Kctx.Citation
