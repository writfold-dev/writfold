import Kctx.Engineering.LedgerCutProofs

set_option autoImplicit false
namespace Kctx.Engineering.Ledger.Calibration

/-- A finite closed reservation payload demonstrates a prefix-sensitive input
validator. The application instantiates the same algebra with its full Action. -/
inductive Payload where
  | reserve (units : Nat)
  | release (units : Nat)
  deriving DecidableEq, Repr, Inhabited

structure Historical where
  actor : Nat
  admittedActor : Nat
  capacity : Nat
  authorityObject : ObjectId
  deriving DecidableEq, Repr, Inhabited

def occupied : List (Record Payload Historical) → Nat
  | [] => 0
  | r :: rest => match r.payload with
    | .reserve n => occupied rest + n
    | .release n => occupied rest - n

/-- admittedActor and the authority object are authenticated inputs under the
stated coordinator boundary; mere record linkage cannot authenticate them. -/
def validate (entries : List (Record Payload Historical))
    (r : Record Payload Historical) : Bool :=
  decide (r.historical.actor = r.historical.admittedActor) &&
  decide (r.profile.version = 1) &&
  decide (r.historical.authorityObject ∈ r.closure) &&
  match r.payload with
  | .reserve n => decide (0 < n ∧ occupied entries + n ≤ r.historical.capacity)
  | .release n => decide (0 < n ∧ n ≤ occupied entries)

def first : Record Payload Historical :=
  { id := ⟨1⟩, stream := ⟨1⟩, sequence := 1, predecessor := none
    profile := ⟨1⟩, payload := .reserve 3
    historical := ⟨7, 7, 5, ⟨11⟩⟩, closure := [⟨11⟩] }

def empty : Store Payload Historical := {}

def afterFirst : Store Payload Historical := commit empty ⟨1⟩ first [⟨11⟩]

def second : Record Payload Historical :=
  { first with
    id := ⟨2⟩, sequence := 2, predecessor := some first.id
    payload := .release 1 }

def afterSecond : Store Payload Historical := commit afterFirst ⟨1⟩ second []

def overbook : Record Payload Historical := { second with payload := .reserve 3 }

/-- This mutant violates capacity established by actual prior payloads, not
an incidental field inequality. A post-state guard is not used. -/
def Overbooked (entries : List (Record Payload Historical)) : Prop := occupied entries > 5

def corpusPin : NativePin :=
  { realm := ⟨1⟩, family := .corpus, graph := ⟨1⟩, graphCommit := ⟨7⟩
    schemaEpoch := ⟨1⟩, query := ⟨1⟩, compatibilityProfile := ⟨1⟩, envelope := ⟨11⟩ }

def activityPin : NativePin := { corpusPin with family := .activity, graph := ⟨2⟩ }

def cut : Cut Payload Historical :=
  { id := ⟨1⟩, realm := ⟨1⟩, inventoryEpoch := 1, inventory := orderedInventory afterFirst
    watermarks := orderedSelections afterFirst, corpusPins := [corpusPin]
    activityPin := activityPin, projectionContract := ⟨1⟩, projectionEnvelope := ⟨11⟩
    visibilityContract := ⟨1⟩, visibilityEnvelope := ⟨11⟩
    assessmentTime := 1, material := [⟨11⟩] }

def pinned : Store Payload Historical := { afterFirst with cuts := [cut] }

def advanced : Store Payload Historical := commit pinned ⟨1⟩ second []

/-- HEAD pagination would change the interpreted reservation balance from 3
to 2. The calibration observes semantics, not merely different record IDs. -/
def PaginationChangesMeaning (store : Store Payload Historical) : Prop :=
  occupied (currentPrefix store ⟨1⟩) ≠ occupied cut.watermarks.head!.entries

def otherStreamRecord : Record Payload Historical :=
  { first with id := ⟨3⟩, stream := ⟨2⟩, payload := .reserve 1 }

def twoStreams : Store Payload Historical :=
  commit afterFirst ⟨2⟩ otherStreamRecord []

def canonicalCut : Cut Payload Historical :=
  { cut with
    inventoryEpoch := 2
    inventory := orderedInventory twoStreams
    watermarks := orderedSelections twoStreams }

/-- Concrete snapshots are proved equal to the runtime-sorted fixtures below;
they only expose finite sorting equations to kernel reduction. -/
def firstSelection : Selection Payload Historical := ⟨⟨1⟩, first.head, [first]⟩

def otherSelection : Selection Payload Historical :=
  ⟨⟨2⟩, otherStreamRecord.head, [otherStreamRecord]⟩

def exactCut : Cut Payload Historical :=
  { cut with inventory := [⟨1⟩], watermarks := [firstSelection] }

def exactPinned : Store Payload Historical := { afterFirst with cuts := [exactCut] }

def exactAdvanced : Store Payload Historical := commit exactPinned ⟨1⟩ second []

def exactCanonicalCut : Cut Payload Historical :=
  { cut with
    inventoryEpoch := 2
    inventory := [⟨1⟩, ⟨2⟩]
    watermarks := [firstSelection, otherSelection] }

theorem first_selections_exact : afterFirst.selections = [firstSelection] := by decide

theorem first_sorted_exact : orderedSelections afterFirst = [firstSelection] := by
  rw [orderedSelections, first_selections_exact, List.mergeSort_singleton]

theorem two_selections_exact : twoStreams.selections = [otherSelection, firstSelection] := by decide

theorem two_sorted_exact : orderedSelections twoStreams = [firstSelection, otherSelection] := by
  rw [orderedSelections, two_selections_exact]
  simp [List.mergeSort, List.MergeSort.Internal.splitInTwo, List.merge,
    firstSelection, otherSelection]

theorem cut_exact : cut = exactCut := by
  simp [cut, exactCut, orderedInventory, first_sorted_exact, firstSelection]

theorem pinned_exact : pinned = exactPinned := by
  simp only [pinned, exactPinned, cut_exact]

theorem advanced_exact : advanced = exactAdvanced := by
  simp only [advanced, exactAdvanced, pinned_exact]

theorem canonicalCut_exact : canonicalCut = exactCanonicalCut := by
  simp [canonicalCut, exactCanonicalCut, cut, orderedInventory,
    first_sorted_exact, two_sorted_exact, firstSelection, otherSelection]

theorem accepted_origin_and_payload :
    append validate empty ⟨1⟩ none first [⟨11⟩] = some afterFirst := by decide

theorem valid_historical_prefix : WellFormed validate afterFirst := by
  exact append_preserves validate empty afterFirst ⟨1⟩ none first [⟨11⟩]
    (empty_wellFormed validate) accepted_origin_and_payload

theorem accepted_prefix_sensitive_release :
    append validate afterFirst ⟨1⟩ (some first.head) second [] = some afterSecond := by decide


theorem overbooking_is_semantic_violation : Overbooked (overbook :: [first]) := by
  change 6 > 5
  decide

theorem overbooking_refused :
    append validate afterFirst ⟨1⟩ (some first.head) overbook [] = none := by decide

theorem forged_origin_refused :
    append validate empty ⟨1⟩ none
      { first with historical := { first.historical with actor := 99 } } [⟨11⟩] = none := by decide

theorem missing_authority_material_refused :
    append validate empty ⟨1⟩ none first [] = none := by decide

theorem unknown_head_refused :
    append validate empty ⟨1⟩ (some first.head) first [⟨11⟩] = none := by decide

theorem two_from_one_predecessor_refused :
    append validate afterFirst ⟨1⟩ none { first with id := ⟨3⟩ } [] = none := by decide


theorem complete_cut_is_inhabited : pin validate afterFirst ⟨1⟩ cut = some pinned := by
  have ready : CutReady validate afterFirst ⟨1⟩ cut := by
    simp only [CutReady, cut_exact, orderedInventory, first_sorted_exact]
    decide
  simp only [pin, if_pos ready]
  rfl

theorem old_page_is_exact : readPinned validate advanced ⟨1⟩ ⟨1⟩ = some [first] := by
  rw [advanced_exact]
  decide


theorem head_pagination_is_semantically_wrong : PaginationChangesMeaning advanced := by
  unfold PaginationChangesMeaning
  rw [advanced_exact, cut_exact]
  change (2 : Nat) ≠ 3
  decide

theorem incomplete_inventory_refused_example :
    pin validate afterFirst ⟨1⟩ { cut with inventory := [], watermarks := [] } = none := by
  apply incomplete_inventory_refuses
  simp only [orderedInventory, first_sorted_exact]
  decide

theorem missing_material_never_falls_back_to_head :
    readPinned validate { advanced with material := [] } ⟨1⟩ ⟨1⟩ = none := by
  rw [advanced_exact]
  decide

theorem wrong_realm_cut_refused : pin validate afterFirst ⟨2⟩ cut = none := by decide

theorem false_predecessor_refused :
    append validate afterFirst ⟨1⟩ (some first.head)
      { second with predecessor := some ⟨99⟩ } [] = none := by decide

theorem malformed_sequence_refused :
    append validate afterFirst ⟨1⟩ (some first.head)
      { second with sequence := 19 } [] = none := by decide


theorem second_stream_append_is_inhabited :
    append validate afterFirst ⟨2⟩ none otherStreamRecord [] = some twoStreams := by decide

theorem canonical_two_stream_cut_is_inhabited :
    pin validate twoStreams ⟨1⟩ canonicalCut =
      some { twoStreams with cuts := [canonicalCut] } := by
  have ready : CutReady validate twoStreams ⟨1⟩ canonicalCut := by
    simp only [CutReady, canonicalCut_exact, orderedInventory, two_sorted_exact]
    decide
  simp only [pin, if_pos ready]
  rfl

/-- Complete membership alone is insufficient: write-order serialization
would change the canonical manifest when another stream advances. -/
theorem write_order_inventory_refused :
    pin validate twoStreams ⟨1⟩
      { canonicalCut with
        inventory := twoStreams.selections.map Selection.stream
        watermarks := twoStreams.selections } = none := by
  apply incomplete_inventory_refuses
  simp only [orderedInventory, two_sorted_exact]
  decide

end Kctx.Engineering.Ledger.Calibration
