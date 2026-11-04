import Kctx.Engineering.Identity
import Init.Data.List.Sort.Lemmas

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

/-- Profiles are registry identities, not claims about hash or codec correctness. -/
structure ReducerProfile where
  version : Nat
  deriving DecidableEq, Repr, Inhabited

/-- The caller supplies the closed action and its exact historical premises.
No digest, string, or current authority lookup stands in for either value. -/
structure Record (Payload Historical : Type) where
  id : LedgerRecordId
  stream : StreamId
  sequence : Nat
  predecessor : Option LedgerRecordId
  profile : ReducerProfile
  payload : Payload
  historical : Historical
  closure : List ObjectId
  deriving DecidableEq, Repr, Inhabited

structure Head where
  stream : StreamId
  sequence : Nat
  record : LedgerRecordId
  profile : ReducerProfile
  deriving DecidableEq, Repr, Inhabited

def Record.head {P H : Type} (r : Record P H) : Head :=
  ⟨r.stream, r.sequence, r.id, r.profile⟩

/-- Prefixes are newest first. The separate head is checked, never trusted. -/
structure Selection (P H : Type) where
  stream : StreamId
  head : Head
  entries : List (Record P H)
  deriving DecidableEq, Repr, Inhabited

inductive GraphFamily where
  | corpus | activity
  deriving DecidableEq, Repr, Inhabited

/-- Exact qualified native identities. The envelope retains their authenticated
binding; these names make no claim that the engine or codec is sound. -/
structure NativePin where
  realm : Id .realm
  family : GraphFamily
  graph : Id .graph
  graphCommit : Id .graphCommit
  schemaEpoch : Id .schemaEpoch
  query : Id .query
  compatibilityProfile : Id .compatibilityProfile
  envelope : ObjectId
  deriving DecidableEq, Repr, Inhabited

/-- A cut is an observation manifest, not a global atomic snapshot. Native
corpus/activity pins and visibility contracts are retained typed objects. -/
structure Cut (P H : Type) where
  id : Id .cut
  realm : Id .realm
  inventoryEpoch : Nat
  inventory : List StreamId
  watermarks : List (Selection P H)
  corpusPins : List NativePin
  activityPin : NativePin
  projectionContract : Id .projectionContract
  projectionEnvelope : ObjectId
  visibilityContract : Id .visibilityContract
  visibilityEnvelope : ObjectId
  assessmentTime : Nat
  material : List ObjectId
  deriving DecidableEq, Repr, Inhabited

structure Store (P H : Type) where
  records : List (Record P H) := []
  releasedCuts : List (Id .cut) := []
  selections : List (Selection P H) := []
  material : List ObjectId := []
  cuts : List (Cut P H) := []
  releaseRoots : List ObjectId := []
  inventoryEpoch : Nat := 0
  deriving DecidableEq, Repr, Inhabited

/-- Canonical manifests use nominal stream order, not latest-write order. -/
def orderedSelections {P H : Type} (s : Store P H) : List (Selection P H) :=
  s.selections.mergeSort (fun a b => decide (a.stream.value ≤ b.stream.value))

def orderedHeads {P H : Type} (s : Store P H) : List Head :=
  (orderedSelections s).map Selection.head

def orderedInventory {P H : Type} (s : Store P H) : List StreamId :=
  (orderedSelections s).map Selection.stream

abbrev Validator (P H : Type) := List (Record P H) → Record P H → Bool

def selected {P H : Type} (s : Store P H) (stream : StreamId) :
    Option (Selection P H) :=
  s.selections.find? (fun x => x.stream == stream)

def currentHead {P H : Type} (s : Store P H) (stream : StreamId) : Option Head :=
  (selected s stream).map Selection.head

def currentPrefix {P H : Type} (s : Store P H) (stream : StreamId) : List (Record P H) :=
  ((selected s stream).map Selection.entries).getD []

def predecessor {P H : Type} (entries : List (Record P H)) : Option LedgerRecordId :=
  entries.head?.map Record.id

/-- Own-stream historical replay view. Shared coordinator state and other
streams are captured separately by the caller, never inferred from this view. -/
def fromPrefix {P H : Type} (stream : StreamId) (entries : List (Record P H))
    (material : List ObjectId) : Store P H :=
  { records := entries, material := material
    selections := match entries with
      | [] => []
      | r :: _ => [⟨stream, r.head, entries⟩]
    inventoryEpoch := if entries.isEmpty then 0 else 1 }

/-- Identity and retention are mathematical exact equality here. A storage
refinement must separately supply canonical codec/injective identity (or the
stated collision-resistance assumption), authenticated historical inputs, and
private-coordinator conditional-head durability. This algebra proves none of
those external trust boundaries. Shared ownership ordering is not a Head. -/
def UniqueRecords {P H : Type} (s : Store P H) : Prop :=
  ∀ a ∈ s.records, ∀ b ∈ s.records, a.id = b.id → a = b

def UniqueHeads {P H : Type} (s : Store P H) : Prop :=
  ∀ a ∈ s.selections, ∀ b ∈ s.selections, a.stream = b.stream → a = b

end Kctx.Engineering.Ledger
