import Lean
import Kctx.Engineering.LedgerExamples

set_option autoImplicit false
namespace Kctx.Engineering.Ledger.Cases
open Lean Calibration

/-! Shared executable append scenarios over the existing reservation
calibration validator. Each step is applied with the actual `append`; the
exported outcome and resulting head are Lean's. Differential evidence for the
Rust ledger kernel, not a refinement proof. -/

abbrev R := Record Payload Historical

structure Step where
  record : R
  expected : Option Head
  available : List ObjectId

structure Scenario where
  name : String
  steps : List Step

def rec (id seq : Nat) (pred : Option Nat) (payload : Payload)
    (stream : Nat := 1) (actor : Nat := 7) (version : Nat := 1)
    (closure : List ObjectId := [⟨11⟩]) : R :=
  { id := ⟨id⟩, stream := ⟨stream⟩, sequence := seq, predecessor := pred.map (⟨·⟩),
    profile := ⟨version⟩, payload, historical := ⟨actor, 7, 5, ⟨11⟩⟩, closure }

def head (r : R) : Option Head := some r.head
def r1 : R := rec 1 1 none (.reserve 3)
def r2 : R := rec 2 2 (some 1) (.release 1)

def scenarios : List Scenario :=
  [ ⟨"happy-path", [⟨r1, none, [⟨11⟩]⟩, ⟨r2, head r1, []⟩]⟩,
    ⟨"stale-expected-head", [⟨r1, none, [⟨11⟩]⟩, ⟨r2, none, []⟩]⟩,
    ⟨"competing-append", [⟨r1, none, [⟨11⟩]⟩, ⟨r2, head r1, []⟩,
       ⟨rec 3 2 (some 1) (.release 1), head r1, []⟩]⟩,
    ⟨"overbook", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 2 2 (some 1) (.reserve 3), head r1, []⟩]⟩,
    ⟨"release-too-much", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 2 2 (some 1) (.release 4), head r1, []⟩]⟩,
    ⟨"zero-reserve", [⟨rec 1 1 none (.reserve 0), none, [⟨11⟩]⟩]⟩,
    ⟨"wrong-sequence", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 2 3 (some 1) (.release 1), head r1, []⟩]⟩,
    ⟨"first-sequence-zero", [⟨rec 1 0 none (.reserve 1), none, [⟨11⟩]⟩]⟩,
    ⟨"wrong-predecessor", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 2 2 (some 9) (.release 1), head r1, []⟩]⟩,
    ⟨"missing-predecessor", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 2 2 none (.release 1), head r1, []⟩]⟩,
    ⟨"foreign-stream", [⟨rec 1 1 none (.reserve 1) (stream := 2), none, [⟨11⟩]⟩]⟩,
    ⟨"duplicate-record", [⟨r1, none, [⟨11⟩]⟩, ⟨rec 1 2 (some 1) (.release 1), head r1, []⟩]⟩,
    ⟨"missing-material", [⟨r1, none, []⟩]⟩,
    ⟨"material-retained-later", [⟨r1, none, [⟨11⟩]⟩, ⟨r2, head r1, []⟩]⟩,
    ⟨"unadmitted-actor", [⟨rec 1 1 none (.reserve 1) (actor := 8), none, [⟨11⟩]⟩]⟩,
    ⟨"unknown-profile", [⟨rec 1 1 none (.reserve 1) (version := 2), none, [⟨11⟩]⟩]⟩,
    ⟨"authority-object-outside-closure", [⟨rec 1 1 none (.reserve 1) (closure := [⟨12⟩]), none, [⟨12⟩]⟩]⟩,
    ⟨"refusal-then-retry", [⟨r1, none, []⟩, ⟨r1, none, [⟨11⟩]⟩, ⟨r2, head r1, []⟩]⟩ ]

def headJson : Option Head → Json
  | none => .null
  | some h => Json.mkObj [("stream", toJson h.stream.value), ("sequence", toJson h.sequence),
      ("record", toJson h.record.value), ("profile", toJson h.profile.version)]

def payloadJson : Payload → Json
  | .reserve n => Json.mkObj [("kind", "reserve"), ("units", toJson n)]
  | .release n => Json.mkObj [("kind", "release"), ("units", toJson n)]

def recordJson (r : R) : Json := Json.mkObj
  [("id", toJson r.id.value), ("stream", toJson r.stream.value), ("sequence", toJson r.sequence),
   ("predecessor", match r.predecessor with | none => .null | some p => toJson p.value),
   ("profile", toJson r.profile.version), ("payload", payloadJson r.payload),
   ("actor", toJson r.historical.actor), ("admitted_actor", toJson r.historical.admittedActor),
   ("capacity", toJson r.historical.capacity),
   ("authority_object", toJson r.historical.authorityObject.value),
   ("closure", toJson (r.closure.map (·.value)))]

/-- Apply each step to the stream named by the step's expected head or, failing
that, stream 1; Rust appends to the same stream. -/
def run (steps : List Step) : List Json := Id.run do
  let mut store : Store Payload Historical := {}
  let mut out := #[]
  for step in steps do
    let stream : StreamId := ⟨1⟩
    let result := append validate store stream step.expected step.record step.available
    let ok := result.isSome
    store := result.getD store
    out := out.push (Json.mkObj
      [("record", recordJson step.record), ("expected", headJson step.expected),
       ("available", toJson (step.available.map (·.value))), ("lean_ok", toJson ok),
       ("head_after", headJson (currentHead store stream)),
       ("inventory_epoch_after", toJson store.inventoryEpoch)])
  return out.toList

end Kctx.Engineering.Ledger.Cases

open Kctx.Engineering.Ledger.Cases in
def main : IO UInt32 := do
  let doc := Lean.Json.mkObj
    [("schema", "writfold-ledger-lean-cases/1"),
     ("validator", "Kctx.Engineering.Ledger.Calibration.validate (reservation capacity)"),
     ("stream", 1),
     ("scenarios", Lean.Json.arr (scenarios.map (fun s =>
        Lean.Json.mkObj [("name", Lean.toJson s.name), ("steps", Lean.Json.arr (run s.steps).toArray)])).toArray)]
  IO.println doc.pretty
  return 0
