import Kctx.Engineering.LedgerProofs

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

def cutRoots {P H : Type} (cut : Cut P H) : List ObjectId :=
  cut.activityPin.envelope :: cut.projectionEnvelope :: cut.visibilityEnvelope ::
    (cut.corpusPins.map NativePin.envelope ++ cut.material)

def NativePinsValid {P H : Type} (cut : Cut P H) : Prop :=
  cut.activityPin.realm = cut.realm ∧ cut.activityPin.family = .activity ∧
  (∀ pin ∈ cut.corpusPins, pin.realm = cut.realm ∧ pin.family = .corpus)

/-- Closure is independent of today's heads and current authority. -/
def CutClosed {P H : Type} (validate : Validator P H) (s : Store P H)
    (cut : Cut P H) : Prop :=
  (cut.watermarks.map Selection.stream = cut.inventory ∧ NativePinsValid cut) ∧
  (∀ watermark ∈ cut.watermarks, SelectionValid validate s watermark) ∧
  (∀ object ∈ cutRoots cut, object ∈ s.material)

instance {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (cut : Cut P H) :
    Decidable (CutClosed validate s cut) := by
  unfold CutClosed NativePinsValid SelectionValid Linked Retained
  infer_instance

/-- A watermark may lag, but must name an exact retained suffix of a selected
newest-first history. A forged branch is not a projected stream prefix. -/
def HistoricalWatermark {P H : Type} (s : Store P H)
    (watermark : Selection P H) : Prop :=
  ∃ current ∈ s.selections, current.stream = watermark.stream ∧
    watermark.entries.length ≤ current.entries.length ∧
    current.entries.drop (current.entries.length - watermark.entries.length) = watermark.entries

instance {P H : Type} [DecidableEq P] [DecidableEq H]
    (s : Store P H) (watermark : Selection P H) :
    Decidable (HistoricalWatermark s watermark) := by
  unfold HistoricalWatermark
  infer_instance

def CutReady {P H : Type} (validate : Validator P H) (s : Store P H)
    (realm : Id .realm) (cut : Cut P H) : Prop :=
  cut.realm = realm ∧
  cut.inventoryEpoch = s.inventoryEpoch ∧
  cut.inventory = orderedInventory s ∧
  CutClosed validate s cut ∧
  (∀ watermark ∈ cut.watermarks, HistoricalWatermark s watermark) ∧
  (∀ old ∈ s.cuts, old.id ≠ cut.id)

instance {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (realm : Id .realm) (cut : Cut P H) :
    Decidable (CutReady validate s realm cut) := by
  unfold CutReady
  infer_instance

def pin {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (realm : Id .realm) (cut : Cut P H) :
    Option (Store P H) :=
  if CutReady validate s realm cut then some { s with cuts := cut :: s.cuts } else none

/-- Every page uses this immutable cut; there is deliberately no HEAD branch. -/
def readPinned {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (cutId : Id .cut) (stream : StreamId) :
    Option (List (Record P H)) :=
  match s.cuts.find? (fun cut => cut.id == cutId) with
  | none => none
  | some cut =>
      if CutClosed validate s cut then
        (cut.watermarks.find? (fun watermark => watermark.stream == stream)).map Selection.entries
      else none

/-- Release drops the live-reader obligation, not its immutable evidence.
Destructive GC is disabled for this profile, including released-cut material. -/
def releaseCut {P H : Type} (s : Store P H) (cutId : Id .cut) : Option (Store P H) :=
  if s.cuts.any (fun cut => cut.id == cutId) then
    some { s with releasedCuts := cutId :: s.releasedCuts }
  else none

def gc {P H : Type} (s : Store P H) : Store P H := s

/-- Completed corpus releases root their original evidence independently of
activity/reader lifetimes. Missing evidence cannot be installed as a root. -/
def retainRelease {P H : Type} (s : Store P H) (roots : List ObjectId) :
    Option (Store P H) :=
  if ∀ object ∈ roots, object ∈ s.material then
    some { s with releaseRoots := roots ++ s.releaseRoots }
  else none

def EvidenceClosed {P H : Type} (validate : Validator P H) (s : Store P H) : Prop :=
  (∀ cut ∈ s.cuts, CutClosed validate s cut) ∧
  (∀ object ∈ s.releaseRoots, object ∈ s.material)

theorem cutClosed_commit {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (r : Record P H) (available : List ObjectId)
    (cut : Cut P H) (h : CutClosed validate s cut) :
    CutClosed validate (commit s stream r available) cut := by
  refine ⟨h.1, ?_, ?_⟩
  · intro watermark hw
    obtain ⟨hh, hs, hl, hr⟩ := h.2.1 watermark hw
    exact ⟨hh, hs, hl, commit_retained s stream r available _ hr⟩
  · intro object ho
    exact commit_retains_material s stream r available object (h.2.2 object ho)

theorem commit_evidence_closed {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (r : Record P H) (available : List ObjectId)
    (h : EvidenceClosed validate s) :
    EvidenceClosed validate (commit s stream r available) := by
  constructor
  · intro cut hc
    exact cutClosed_commit validate s stream r available cut (h.1 cut hc)
  · intro object ho
    exact commit_retains_material s stream r available object (h.2 object ho)

theorem append_preserves_evidence {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (evidence : EvidenceClosed validate s)
    (h : append validate s stream expected r available = some t) :
    EvidenceClosed validate t := by
  obtain ⟨_, rfl⟩ := append_success validate s t stream expected r available h
  exact commit_evidence_closed validate s stream r available evidence

theorem append_preserves_roots {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : append validate s stream expected r available = some t) :
    t.cuts = s.cuts ∧ t.releaseRoots = s.releaseRoots ∧ t.releasedCuts = s.releasedCuts := by
  obtain ⟨_, rfl⟩ := append_success validate s t stream expected r available h
  exact ⟨rfl, rfl, rfl⟩

theorem readPinned_commit {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (cutId : Id .cut)
    (readStream appendStream : StreamId) (r : Record P H) (available : List ObjectId)
    (entries : List (Record P H))
    (h : readPinned validate s cutId readStream = some entries) :
    readPinned validate (commit s appendStream r available) cutId readStream = some entries := by
  cases hc : s.cuts.find? (fun cut => cut.id == cutId) with
  | none => simp [readPinned, hc] at h
  | some cut =>
      by_cases closed : CutClosed validate s cut
      · have closed' := cutClosed_commit validate s appendStream r available cut closed
        have hc' : (commit s appendStream r available).cuts.find?
            (fun c => c.id == cutId) = some cut := hc
        simp only [readPinned, hc', if_pos closed']
        simpa only [readPinned, hc, if_pos closed] using h
      · simp [readPinned, hc, closed] at h

theorem append_preserves_interpretation {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (cutId : Id .cut)
    (readStream appendStream : StreamId) (expected : Option Head)
    (r : Record P H) (available : List ObjectId) (entries : List (Record P H))
    (ha : append validate s appendStream expected r available = some t)
    (hr : readPinned validate s cutId readStream = some entries) :
    readPinned validate t cutId readStream = some entries := by
  obtain ⟨_, rfl⟩ := append_success validate s t appendStream expected r available ha
  exact readPinned_commit validate s cutId readStream appendStream r available entries hr

theorem incomplete_inventory_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (realm : Id .realm) (cut : Cut P H)
    (h : cut.inventory ≠ orderedInventory s) : pin validate s realm cut = none := by
  simp only [pin]
  split
  · exact False.elim (h ‹CutReady validate s realm cut›.2.2.1)
  · rfl

theorem missing_cut_material_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (realm : Id .realm) (cut : Cut P H)
    (object : ObjectId) (root : object ∈ cutRoots cut) (missing : object ∉ s.material) :
    pin validate s realm cut = none := by
  simp only [pin]
  split
  · exact False.elim (missing (‹CutReady validate s realm cut›.2.2.2.1.2.2 object root))
  · rfl

theorem missing_pinned_material_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (cutId : Id .cut) (stream : StreamId)
    (cut : Cut P H) (found : s.cuts.find? (fun c => c.id == cutId) = some cut)
    (object : ObjectId) (root : object ∈ cutRoots cut) (missing : object ∉ s.material) :
    readPinned validate s cutId stream = none := by
  have bad : ¬ CutClosed validate s cut := fun h => missing (h.2.2 object root)
  simp [readPinned, found, bad]

theorem disabled_gc_retains_everything {P H : Type} (s : Store P H) : gc s = s := rfl

end Kctx.Engineering.Ledger
