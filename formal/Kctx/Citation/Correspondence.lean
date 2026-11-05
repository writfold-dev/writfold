import Kctx.Citation.Resolution

set_option autoImplicit false
namespace Kctx.Citation

/-- These are all possible starting offsets, including the final empty occurrence.
If the needle is longer than the object there are no possible positions. -/
def possiblePositions (bytes needle : List UInt8) : List Nat :=
  if needle.length ≤ bytes.length then
    List.range (bytes.length - needle.length + 1)
  else []

def examinedPositions (bytes needle : List UInt8) (budget : Nat) : List Nat :=
  (possiblePositions bytes needle).take budget

def positionSelector (offset width : Nat) : Selector :=
  ⟨offset, offset + width, .byte⟩

/-- Only examined offsets can enter the diagnostic list. No completeness input. -/
def textCandidates (bytes needle : List UInt8) (budget : Nat) : List Selector :=
  ((examinedPositions bytes needle budget).filter fun offset =>
    decide (selectedBytes bytes (positionSelector offset needle.length) = needle)).map
      (fun offset => positionSelector offset needle.length)

def scanCoverage (bytes needle : List UInt8) (budget : Nat) : Coverage :=
  .measured (examinedPositions bytes needle budget).length (possiblePositions bytes needle).length

/-- A singleton text match is not a correspondence proof, even with full coverage.
No source-history deletion rule is supported by the initial calculus. -/
def diagnose (bytes needle : List UInt8) (budget : Nat) : CompareResult :=
  let candidates := textCandidates bytes needle budget
  let coverage := scanCoverage bytes needle budget
  if 2 ≤ candidates.length then .multiple candidates coverage
  else .unknown
    (if coverage.complete then
      if candidates.isEmpty then .noTextMatch else .unsupportedCorrespondence
    else .incompleteSearch) candidates coverage

/-- Identity transports an occurrence without changing its evidence role. Release
and citation IDs may differ; reference, offsets, role and context may not. -/
def SameOccurrence (original target : Binding) : Prop :=
  original.representation = target.representation ∧
  original.selector = target.selector ∧ original.expected = target.expected ∧
  original.context = target.context ∧ original.role = target.role

instance (original target : Binding) : Decidable (SameOccurrence original target) := by
  unfold SameOccurrence
  infer_instance

def sameScope (original target : Binding) : Prop :=
  original.representation.realm = target.representation.realm ∧
  original.representation.repository = target.representation.repository ∧
  original.context = target.context

instance (original target : Binding) : Decidable (sameScope original target) := by
  unfold sameScope
  infer_instance

/-- Both access checks precede any diagnostic disclosure. Original selection must
resolve; target diagnostics inspect exact pinned bytes, never a supplied match list.
Identity additionally re-resolves the target binding with its own policy decision. -/
def compare (store : Store) (access : Access) (original target : Binding)
    (witness : Option CorrespondenceWitness) (budget : Nat) : CompareResult :=
  if access original = false ∨ access target = false then .denied
  else match resolve store access original with
    | .denied => .denied
    | .invalid reason => .unknown (.originalInvalid reason) [] .unavailable
    | .unavailable => .unknown .originalUnavailable [] .unavailable
    | .exact _ _ =>
        if ¬ sameScope original target then .unknown .scopeMismatch [] .unavailable
        else if target.representation.profile ≠ .gitBlob ∨ target.selector.unit ≠ .byte then
          .unknown .unsupportedCorrespondence [] .unavailable
        else match lookupRepresentation store target.representation with
          | none => .unknown .targetUnavailable [] .unavailable
          | some object =>
              if original = target then .sameBinding
              else if witness = some .identicalPosition ∧ SameOccurrence original target ∧
                  resolve store access target = .exact target.expected target.representation then
                .uniqueJustified target .identicalPosition
              else diagnose object.bytes original.expected budget

end Kctx.Citation
