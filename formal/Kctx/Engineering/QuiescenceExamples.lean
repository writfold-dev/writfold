import Kctx.Engineering.FixtureCalculus

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- Qualified evidence for another root does not inherit the first root's
invocation or effect identity, even when its immutable procedure inputs match. -/
def nextOwnerReceipt : Receipt := { receipt with invocation := ⟨20⟩, effect := ⟨2⟩ }
def nextOwnerIntent : Intent := { intent with run := ⟨2⟩, invocation := ⟨20⟩, effect := ⟨2⟩ }
def ownershipContracts : Contracts := { contracts with
  receipts := nextOwnerReceipt :: contracts.receipts }
def preparedOwner : Option State := do
  let s ← start ownershipContracts proc
  trace ownershipContracts s [.reserve ⟨1⟩ [⟨1⟩] 1,
    .recordReceipt receipt, .prepare intent 1]

/-- A prepared intent is not silently abandoned by releasing its reservation.
Explicit cancellation closes it before a new owner can prepare an overlapping effect. -/
def preparedReleaseRefused : Bool := preparedOwner.any (fun s =>
  (findEffect s ⟨1⟩).any (·.outcome == .prepared) &&
  s.ownership == [⟨⟨1⟩, ⟨1⟩, ⟨1⟩, 1, 0, 0⟩] &&
  (submit ownershipContracts s (.quiesce ⟨1⟩)).isNone &&
  (trace ownershipContracts s [.cancel ⟨1⟩, .quiesce ⟨1⟩,
    .activate runTwo 20, .reserve ⟨20⟩ [⟨1⟩] 2,
    .recordReceipt nextOwnerReceipt, .prepare nextOwnerIntent 2]).any (fun t =>
      (findEffect t ⟨1⟩).any (·.outcome == .cancelledBeforeDispatch) &&
      (findEffect t ⟨2⟩).any (·.outcome == .prepared) &&
      t.ownership == [⟨⟨1⟩, ⟨2⟩, ⟨20⟩, 2, 0, 0⟩]))

theorem prepared_release_nonvacuous : preparedReleaseRefused = true := by decide

end Kctx.Engineering.Examples
