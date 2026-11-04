import Kctx.Engineering.Control

set_option autoImplicit false
namespace Kctx.Engineering

def owns (s : State) (i : Intent) (generation : Nat) : Bool := i.resources.all (fun key =>
  s.ownership.any (fun o => o.resource == key && o.owner == i.run &&
    o.holder == i.invocation && o.generation == generation))

def unsettled (outcome : EffectOutcome) : Bool :=
  outcome == .prepared || outcome == .claimed || outcome == .unknown || outcome == .partial

/-- Reservation ownership alone is not enough: two nodes of one activation can
conflict. Only the exact logical effect being retried is excluded. -/
def noConflictingEffect (s : State) (i : Intent) : Bool :=
  !s.effects.any (fun e => e.intent.effect != i.effect && unsettled e.outcome &&
    e.intent.resources.any i.resources.contains)

def ownedTreesQuiet (s : State) (i : Intent) (generation : Nat) : Bool :=
  i.resources.all (fun key => s.ownership.any (fun o =>
    o.resource == key && o.owner == i.run && o.holder == i.invocation &&
    o.generation == generation && o.children == 0))

def retryAuthorizationValid (s : State) (intent : Intent) (a : RetryAuthorization) : Bool :=
  a.intent == intent && a.incarnation == s.incarnation && a.policy == s.authority.policy &&
  a.validFrom ≤ s.now && s.now < a.expires

def dispatchRetryValid (contracts : Contracts) (s : State) (d : Dispatch) : Bool :=
  match d.retryAuthorization with
  | none => d.retry.isNone
  | some a => d.retry == some a.kind && contracts.retryAuthorizations.contains a &&
      retryAuthorizationValid s d.intent a

def designAccepted (r : Run) (now : Nat) : Bool :=
  match r.design with
  | none => false
  | some d => designCandidateValid d .accepted now && implementationBound d now &&
      d.targetSource == r.binding.targetSource &&
      d.authored.targetRevision == some r.binding.targetRevision


def intentReady (s : State) (r : Run) (i : Intent) (generation : Nat) : Bool :=
  live s r && i.run == r.id && i.invocation == r.invocation &&
  i.binding == r.binding && i.procedure == r.procedure.revision &&
  i.expectedProvider == r.binding.expectedProvider &&
  r.capabilities.contains i.capability &&
  (match currentGrant s r with
   | some g => g.capabilities.contains i.capability && subset i.resources g.resources
   | none => false) &&
  !i.resources.isEmpty &&
  (findNode r.procedure i.node).any (fun n => i.resources == n.resources) &&
  owns s i generation && noConflictingEffect s i && retained s i.closure &&
  (i.capability == .read || i.capability == .check || designAccepted r s.now)

def prepare (contracts : Contracts) (s : State) (i : Intent) (generation : Nat) : Option State := do
  let r ← findRun s i.invocation
  let n ← findNode r.procedure i.node
  if intentReady s r i generation && nodeReady contracts s r n && evidenceReady s r n &&
      n.form == .effect i.capability && i.resources == n.resources &&
      i.expectedProvider == s.world.provider &&
      !s.effects.any (fun e => e.intent.invocation == i.invocation && e.intent.node == i.node) &&
      !(findEffect s i.effect).isSome then
    let charged ← charge s r.id n.cost
    some { charged with effects := ⟨i, generation, .prepared, none⟩ :: s.effects }
  else none

def reserve (s : State) (run : ActivationId) (keys : List (Id .resource)) (generation : Nat) : Option State := do
  let r ← findRun s run
  let g ← currentGrant s r
  if live s r && !keys.isEmpty && unique keys && subset keys g.resources &&
      keys.all (fun key => !s.ownership.any (·.resource == key) &&
        s.reservationGenerations.all (fun old => old.1 != key || old.2 < generation)) then
    some { s with
      ownership := keys.map (fun key => ⟨key, r.id, r.invocation, generation, 0, 0⟩) ++ s.ownership
      reservationGenerations := keys.map (fun key => (key, generation)) ++ s.reservationGenerations }
  else none

def claimDispatch (contracts : Contracts) (s : State) (effect : Id .effect) (claim : Id .claim)
    (expected : Option Ledger.Head) (record : LedgerRecordId) : Option State := do
  let e ← findEffect s effect
  let r ← findRun s e.intent.invocation
  let g ← currentGrant s r
  let n ← findNode r.procedure e.intent.node
  if e.outcome == .prepared && intentReady s r e.intent e.generation && evidenceReady s r n &&
      nodeReady contracts s r n && ownedTreesQuiet s e.intent e.generation &&
      e.intent.expectedProvider == s.world.provider &&
      Ledger.currentHead s.ledger r.stream == expected &&
      !s.ledger.records.any (·.id == record) && !s.claims.any (·.id == claim) then
    let dispatch : Dispatch := ⟨claim, e.intent, expected, record, e.generation,
      s.incarnation, g.id, s.authority.policy, none, none, false⟩
    some { (putEffect s { e with outcome := .claimed, dispatch := some dispatch }) with
      claims := dispatch :: s.claims }
  else none

/-- A retry is a new bounded dispatch claim for the same immutable logical
intent. Neither an unknown outcome nor a fresh claim is evidence of failure. -/
def retryClaim (contracts : Contracts) (s : State) (effect : Id .effect) (claim : Id .claim)
    (expected : Option Ledger.Head) (record : LedgerRecordId) : Option State := do
  let e ← findEffect s effect
  let r ← findRun s e.intent.invocation
  let g ← currentGrant s r
  let n ← findNode r.procedure e.intent.node
  let authorization ← contracts.retryAuthorizations.find? (fun a =>
    retryAuthorizationValid s e.intent a && a.previousClaim == e.dispatch.map Dispatch.id)
  if e.outcome == .unknown &&
      (match e.dispatch with | none => true | some previous => s.claims.contains previous) &&
      intentReady s r e.intent e.generation && evidenceReady s r n &&
      nodeReady contracts s r n && ownedTreesQuiet s e.intent e.generation &&
      (e.intent.expectedProvider == s.world.provider || authorization.kind == .providerIdempotent) &&
      Ledger.currentHead s.ledger r.stream == expected &&
      !s.ledger.records.any (·.id == record) && !s.claims.any (·.id == claim) then
    let charged ← charge s r.id n.cost
    let dispatch : Dispatch := ⟨claim, e.intent, expected, record, e.generation,
      s.incarnation, g.id, s.authority.policy, some authorization.kind, some authorization, false⟩
    some { (putEffect charged { e with outcome := .claimed, dispatch := some dispatch }) with
      claims := dispatch :: charged.claims }
  else none

/-- This transition models the physical send, not the claim persistence. -/
def issueEffect (contracts : Contracts) (s : State) (effect : Id .effect) : Option State := do
  let e ← findEffect s effect
  let d ← e.dispatch
  let r ← findRun s e.intent.invocation
  let g ← currentGrant s r
  let n ← findNode r.procedure e.intent.node
  if e.outcome == .claimed && !d.issued && d.incarnation == s.incarnation &&
      d.intent == e.intent && d.grant == g.id && d.policy == s.authority.policy &&
      d.generation == e.generation && intentReady s r e.intent d.generation &&
      evidenceReady s r n && nodeReady contracts s r n &&
      ownedTreesQuiet s e.intent d.generation && s.claims.contains d &&
      dispatchRetryValid contracts s d &&
      (Ledger.currentPrefix s.ledger r.stream).any (fun record => record.id == d.record &&
        record.payload == (if d.retryAuthorization.isSome then
          .retryClaim effect d.id d.expectedHead d.record
        else .claimDispatch effect d.id d.expectedHead d.record)) &&
      (e.intent.expectedProvider == s.world.provider ||
        d.retry == some .providerIdempotent) then
    some { (putEffect s { e with dispatch := some { d with issued := true } }) with
      claims := s.claims.map (fun old => if old.id == d.id then { old with issued := true } else old),
      world := { s.world with pending := ⟨e.intent, d.id⟩ :: s.world.pending },
      ownership := s.ownership.map (fun o =>
        if e.intent.resources.contains o.resource && o.owner == e.intent.run &&
            o.holder == e.intent.invocation && o.generation == d.generation then
          { o with children := o.children + 1, processGeneration := o.processGeneration + 1 } else o) }
  else none

def classify : ExternalOutcome → EffectOutcome
  | .applied => .confirmed | .rejected => .failed | .partial => .partial

def reconcile (s : State) (observation : ExternalEvent) : Option State := do
  let e ← findEffect s observation.effect
  if s.observations.contains observation && observation.binding == e.intent.binding &&
      observation.expected == e.intent.expectedProvider &&
      (observation.result != .rejected || observation.claim == e.dispatch.map Dispatch.id) &&
      (e.outcome == .claimed || e.outcome == .unknown) then
    some (putEffect s { e with outcome := classify observation.result })
  else none

def reuse (s : State) (run : ActivationId) (receipt : Receipt) : Option State := do
  let r ← findRun s run
  if s.receipts.contains receipt && receiptValid s r receipt then
    some { s with reused := ⟨receipt, r.invocation⟩ :: s.reused }
  else none

def quiesce (contracts : Contracts) (s : State) (run : RunId) : Option State :=
  if !s.runs.any (fun r => r.id == run && r.parent.isSome && r.status == .active) &&
      s.ownership.all (fun o => o.owner != run ||
      (o.children == 0 &&
        contracts.quiescent.contains (o.resource, o.generation, o.processGeneration))) &&
      !s.effects.any (fun e => e.intent.run == run &&
        (e.outcome == .prepared || e.outcome == .claimed || e.outcome == .unknown ||
          e.outcome == .partial)) then
    some { s with ownership := s.ownership.filter (·.owner != run) }
  else none

def crash (s : State) : State :=
  { s with alive := false, effects := s.effects.map (fun e =>
    if e.outcome == .prepared || e.outcome == .claimed then { e with outcome := .unknown } else e) }

def recover (s : State) : Option State :=
  if !s.alive then some { s with alive := true, incarnation := s.incarnation + 1 } else none


end Kctx.Engineering
