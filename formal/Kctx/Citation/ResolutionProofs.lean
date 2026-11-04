import Kctx.Citation.Resolution

set_option autoImplicit false
namespace Kctx.Citation

theorem lookupRepresentation_provenance (store : Store) (reference : RepresentationRef)
    (object : Representation) (h : lookupRepresentation store reference = some object) :
    object.reference = reference := by
  induction store with
  | nil => simp [lookupRepresentation] at h
  | cons first rest ih =>
      simp only [lookupRepresentation] at h
      split at h
      next equal => cases h; exact equal
      next => exact ih h

/-- Every success establishes exact preimage, provenance, units and full bounds. -/
theorem resolve_exact (store : Store) (access : Access) (binding : Binding)
    (bytes : List UInt8) (reference : RepresentationRef)
    (h : resolve store access binding = .exact bytes reference) :
    access binding = true ∧ binding.representation.profile = .gitBlob ∧
    binding.selector.unit = .byte ∧ reference = binding.representation ∧
    bytes = binding.expected ∧ ∃ object,
      lookupRepresentation store binding.representation = some object ∧
      binding.selector.first ≤ binding.selector.last ∧
      binding.selector.last ≤ object.bytes.length ∧
      selectedBytes object.bytes binding.selector = bytes := by
  unfold resolve at h
  split at h
  · contradiction
  · rename_i allowed
    split at h
    · contradiction
    · rename_i profile
      split at h
      · contradiction
      · rename_i unit
        split at h
        · contradiction
        · rename_i object found
          split at h
          · rename_i bounds
            split at h
            · rename_i selected
              cases h
              have provenance := lookupRepresentation_provenance store
                binding.representation object found
              simp_all
            · contradiction
          · contradiction

theorem resolve_denied (store : Store) (access : Access) (binding : Binding)
    (h : access binding = false) : resolve store access binding = .denied := by
  simp [resolve, h]

theorem resolve_wrong_expected (store : Store) (access : Access) (binding : Binding)
    (object : Representation)
    (found : lookupRepresentation store binding.representation = some object)
    (wrong : selectedBytes object.bytes binding.selector ≠ binding.expected)
    (bytes : List UInt8) (reference : RepresentationRef) :
    resolve store access binding ≠ .exact bytes reference := by
  intro h
  obtain ⟨_, _, _, _, equal, actual, actualFound, _, _, selected⟩ :=
    resolve_exact store access binding bytes reference h
  have same : actual = object := Option.some.inj (actualFound.symm.trans found)
  subst actual
  exact wrong (selected.trans equal)

theorem lookupRepresentation_append_present (store suffix : Store)
    (reference : RepresentationRef) (object : Representation)
    (found : lookupRepresentation store reference = some object) :
    lookupRepresentation (store ++ suffix) reference = some object := by
  induction store with
  | nil => simp [lookupRepresentation] at found
  | cons first rest ih =>
      simp only [List.cons_append, lookupRepresentation] at found ⊢
      split at found
      next equal => simp [equal, found]
      next unequal => simp [unequal, ih found]

theorem retain_rejects_existing (store : Store) (object previous : Representation)
    (found : lookupRepresentation store object.reference = some previous) :
    retain store object = none := by
  simp [retain, found]

theorem retain_preserves_lookup (store next : Store) (added old : Representation)
    (reference : RepresentationRef) (step : retain store added = some next)
    (found : lookupRepresentation store reference = some old) :
    lookupRepresentation next reference = some old := by
  unfold retain at step
  split at step
  · contradiction
  · cases step
    exact lookupRepresentation_append_present store [added] reference old found

/-- A finite history consists solely of successful append-only retention steps. -/
inductive RetainedEvolution : Store → Store → Prop where
  | refl (store : Store) : RetainedEvolution store store
  | step {first middle last : Store} (object : Representation) :
      RetainedEvolution first middle → retain middle object = some last →
      RetainedEvolution first last

theorem evolution_preserves_lookup {store next : Store}
    (history : RetainedEvolution store next) (reference : RepresentationRef)
    (object : Representation) (found : lookupRepresentation store reference = some object) :
    lookupRepresentation next reference = some object := by
  induction history with
  | refl => exact found
  | step added history retained ih =>
      exact retain_preserves_lookup _ _ added object reference retained ih

/-- Pinned successes survive arbitrary finite retention, with the same policy.
Changing authorization is deliberately not claimed to preserve disclosure. -/
theorem evolution_preserves_resolution {store next : Store}
    (history : RetainedEvolution store next) (access : Access) (binding : Binding)
    (bytes : List UInt8) (reference : RepresentationRef)
    (resolved : resolve store access binding = .exact bytes reference) :
    resolve next access binding = .exact bytes reference := by
  obtain ⟨allowed, profile, unit, provenance, equal, object, found, bounds, endBound, selected⟩ :=
    resolve_exact store access binding bytes reference resolved
  have nextFound := evolution_preserves_lookup history binding.representation object found
  have objectReference := lookupRepresentation_provenance store binding.representation object found
  simp [resolve, allowed, profile, unit, nextFound, bounds, endBound,
    selected, equal, objectReference, provenance]

theorem resolve_unsupported_profile (store : Store) (access : Access) (binding : Binding)
    (allowed : access binding = true)
    (profile : binding.representation.profile ≠ .gitBlob) :
    resolve store access binding = .invalid .unsupportedProfile := by
  simp [resolve, allowed, profile]

theorem resolve_mixed_units (store : Store) (access : Access) (binding : Binding)
    (allowed : access binding = true) (profile : binding.representation.profile = .gitBlob)
    (units : binding.selector.unit ≠ .byte) :
    resolve store access binding = .invalid .mixedUnits := by
  simp [resolve, allowed, profile, units]

theorem resolve_invalid_bounds (store : Store) (access : Access) (binding : Binding)
    (object : Representation) (allowed : access binding = true)
    (profile : binding.representation.profile = .gitBlob) (units : binding.selector.unit = .byte)
    (found : lookupRepresentation store binding.representation = some object)
    (bounds : ¬ (binding.selector.first ≤ binding.selector.last ∧
      binding.selector.last ≤ object.bytes.length)) :
    resolve store access binding = .invalid .invalidBounds := by
  simp [resolve, allowed, profile, units, found, bounds]

theorem resolve_missing_object (store : Store) (access : Access) (binding : Binding)
    (allowed : access binding = true) (profile : binding.representation.profile = .gitBlob)
    (units : binding.selector.unit = .byte)
    (missing : lookupRepresentation store binding.representation = none) :
    resolve store access binding = .unavailable := by
  simp [resolve, allowed, profile, units, missing]

/-- Valid authorized inputs actually succeed; exactness is not only a refusal law. -/
theorem resolve_of_valid (store : Store) (access : Access) (binding : Binding)
    (object : Representation) (allowed : access binding = true)
    (profile : binding.representation.profile = .gitBlob) (units : binding.selector.unit = .byte)
    (found : lookupRepresentation store binding.representation = some object)
    (bounds : binding.selector.first ≤ binding.selector.last ∧
      binding.selector.last ≤ object.bytes.length)
    (selected : selectedBytes object.bytes binding.selector = binding.expected) :
    resolve store access binding = .exact binding.expected binding.representation := by
  have provenance := lookupRepresentation_provenance store binding.representation object found
  simp [resolve, allowed, profile, units, found, bounds, selected, provenance]

end Kctx.Citation
