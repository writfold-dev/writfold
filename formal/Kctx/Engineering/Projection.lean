import Kctx.Engineering.ActionDomain

set_option autoImplicit false
namespace Kctx.Engineering

/-- Projection observations are qualified data, never authorization. They may
lag the authoritative heads; complete cut acquisition then refuses explicitly. -/
def project (contracts : Contracts) (s : State) (pin : Ledger.NativePin)
    (heads : List Ledger.Head) (inventoryEpoch : Nat) : Option State :=
  if contracts.projections.contains (pin, heads, inventoryEpoch) &&
      pin.realm == s.realm && pin.family == .activity && s.retained.contains pin.envelope &&
      inventoryEpoch ≤ s.ledger.inventoryEpoch &&
      heads.all (fun head => s.ledger.records.any (fun record => record.head == head)) then
    some { s with activityPin := pin, projectedHeads := heads, projectionInventory := inventoryEpoch }
  else none

/-- This profile exposes complete cuts only. A lagging/incomplete projection is
refused, rather than silently supplemented from current HEAD. Older pinned cuts
remain immutable and usable after either family advances. -/
def acquireCut (contracts : Contracts) (s : State) (id : Id .cut) : Option State := do
  if contracts.corpusBindings.contains (s.knowledge.kb.snapshot, s.corpusPin) &&
      s.projectedHeads == Ledger.orderedHeads s.ledger &&
      s.projectionInventory == s.ledger.inventoryEpoch && !s.cuts.any (·.id == id) then
    let cut : Ledger.Cut Action HistoricalPremises := {
      id := id
      realm := s.realm
      inventoryEpoch := s.projectionInventory
      inventory := Ledger.orderedInventory s.ledger
      watermarks := Ledger.orderedSelections s.ledger
      corpusPins := [s.corpusPin]
      activityPin := s.activityPin
      projectionContract := s.projectionContract
      projectionEnvelope := s.projectionEnvelope
      visibilityContract := s.visibilityContract
      visibilityEnvelope := s.visibilityEnvelope
      assessmentTime := s.now
      material := s.retained }
    let ledger ← Ledger.pin historicalValid s.ledger s.realm cut
    some { s with ledger := ledger, cuts := ⟨id, s.knowledge⟩ :: s.cuts }
  else none

def releaseReadCut (s : State) (id : Id .cut) : Option State := do
  let ledger ← Ledger.releaseCut s.ledger id
  some { s with ledger := ledger }

/-- Original corpus interpretation and manifest are returned together. Released,
missing or incomplete material is unavailable; no current-state fallback exists. -/
def readCut (s : State) (id : Id .cut) :
    Option (ContextCut × Ledger.Cut Action HistoricalPremises) := do
  if s.ledger.releasedCuts.contains id then none else do
    let context ← s.cuts.find? (·.id == id)
    let manifest ← s.ledger.cuts.find? (·.id == id)
    if Ledger.CutClosed historicalValid s.ledger manifest then some (context, manifest)
    else none

def readStreamAtCut (s : State) (id : Id .cut) (stream : StreamId) :
    Option (List (Ledger.Record Action HistoricalPremises)) := do
  let _ ← readCut s id
  Ledger.readPinned historicalValid s.ledger id stream

end Kctx.Engineering
