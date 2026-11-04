import Kctx.Citation.Types

set_option autoImplicit false
namespace Kctx.Citation

/-- Exact provenance-key lookup; there is no mutable branch or HEAD interpretation. -/
def lookupRepresentation : Store → RepresentationRef → Option Representation
  | [], _ => none
  | object :: rest, reference =>
      if object.reference = reference then some object
      else lookupRepresentation rest reference

/-- Retention never replaces an existing key, even with equal bytes. -/
def retain (store : Store) (object : Representation) : Option Store :=
  match lookupRepresentation store object.reference with
  | some _ => none
  | none => some (store ++ [object])

def selectedBytes (bytes : List UInt8) (selector : Selector) : List UInt8 :=
  (bytes.drop selector.first).take (selector.last - selector.first)

/-- Policy is checked before either object existence or validation is disclosed.
Nat offsets model nonnegative integers, not wrapping machine arithmetic. -/
def resolve (store : Store) (access : Access) (binding : Binding) : ResolveResult :=
  if access binding = false then .denied
  else if binding.representation.profile ≠ .gitBlob then .invalid .unsupportedProfile
  else if binding.selector.unit ≠ .byte then .invalid .mixedUnits
  else match lookupRepresentation store binding.representation with
    | none => .unavailable
    | some object =>
        if binding.selector.first ≤ binding.selector.last ∧
            binding.selector.last ≤ object.bytes.length then
          if selectedBytes object.bytes binding.selector = binding.expected then
            .exact binding.expected object.reference
          else .invalid .selectionMismatch
        else .invalid .invalidBounds

end Kctx.Citation
