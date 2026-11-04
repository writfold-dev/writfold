import Kctx.Engineering.Projection
import Kctx.Engineering.Environment

set_option autoImplicit false
namespace Kctx.Engineering

/-- Engineering heads, corpus CAS and physical send have distinct linearization
boundaries. Neither send nor completed publication invents an atomic second log write. -/
inductive Request where
  | append (stream : StreamId) (expected : Option Ledger.Head)
      (record : LedgerRecordId) (action : Action)
  | publish (stream : StreamId) (expected : Option Ledger.Head)
  | dispatch (stream : StreamId) (expected : Option Ledger.Head) (effect : Id .effect)
  | local (action : Action)
  deriving DecidableEq, Repr, Inhabited

def recordFor (contracts : Contracts) (s : State) (stream : StreamId)
    (id : LedgerRecordId) (action : Action) : Ledger.Record Action HistoricalPremises :=
  { id := id
    stream := stream
    sequence := (Ledger.currentPrefix s.ledger stream).length + 1
    predecessor := Ledger.predecessor (Ledger.currentPrefix s.ledger stream)
    profile := ⟨1⟩
    payload := action
    historical := ⟨s.toOperationalState, contracts⟩
    closure := inputClosure s.toOperationalState action }


/-- The exact prepared admission must be selected by the learning stream. Its
old premises are replayed at their recorded time, then publication rechecks live policy. -/
def preparedAdmissionRecorded (admission : Admission) :
    List (Ledger.Record Action HistoricalPremises) → Bool
  | [] => false
  | record :: entries =>
      (record.payload == .prepareAdmission admission.candidate.id &&
        (replay entries record).any (·.admission == some admission)) ||
      preparedAdmissionRecorded admission entries

def publishSelected (contracts : Contracts) (s : State) (stream : StreamId)
    (expected : Option Ledger.Head) : Option State := do
  let admission ← s.admission
  if stream == admission.candidate.stream && Ledger.currentHead s.ledger stream == expected &&
      preparedAdmissionRecorded admission (Ledger.currentPrefix s.ledger stream) then
    -- All currently retained inputs become release roots before the corpus CAS.
    let rooted ← Ledger.retainRelease s.ledger s.retained
    domain contracts { s with ledger := rooted } .publishAdmission
  else none

def dispatchSelected (contracts : Contracts) (s : State) (stream : StreamId)
    (expected : Option Ledger.Head) (effect : Id .effect) : Option State :=
  if effectStream s effect == some stream && Ledger.currentHead s.ledger stream == expected then
    domain contracts s (.issueEffect effect)
  else none

def localAction (contracts : Contracts) (s : State) : Action → Option State
  | .project pin heads epoch => project contracts s pin heads epoch
  | .acquireReadCut id => acquireCut contracts s id
  | .releaseReadCut id => releaseReadCut s id
  | .crash => some (crash s)
  | .recover => recover s
  | _ => none

/-- There is one authoritative append path. Historical replay supplies the
checked domain result; every record stores the exact closed action and inputs.
Foreign stream routing, stale heads, forged claim stamps and invalid history refuse. -/
def step (contracts : Contracts) (s : State) : Request → Option State
  | .append stream expected id action => do
      if action.durable && s.alive && actionStream s action == some stream &&
          claimStampBound expected id action then
        let record := recordFor contracts s stream id action
        let next ← replay (Ledger.currentPrefix s.ledger stream) record
        let ledger ← Ledger.append historicalValid s.ledger stream expected record next.retained
        some { toOperationalState := next, ledger := ledger, cuts := s.cuts }
      else none
  | .publish stream expected => publishSelected contracts s stream expected
  | .dispatch stream expected effect => dispatchSelected contracts s stream expected effect
  | .local action => localAction contracts s action

inductive Transition (contracts : Contracts) : State → State → Prop where
  | operational {s t : State} {request : Request} :
      step contracts s request = some t → Transition contracts s t
  | external {s t : State} {action : Environment} :
      environment contracts s action = some t → Transition contracts s t
  | refused (s : State) : Transition contracts s s

inductive Reachable (contracts : Contracts) (initial : State) : State → Prop where
  | initial : Reachable contracts initial initial
  | next {s t : State} : Reachable contracts initial s → Transition contracts s t →
      Reachable contracts initial t

structure InitialInputs where
  realm : Id .realm
  controlStream : StreamId
  knowledge : PortfolioState
  authority : Authority
  objects : List ObjectId
  world : World
  corpusPin : Ledger.NativePin
  activityPin : Ledger.NativePin
  projectionContract : Id .projectionContract
  projectionEnvelope : ObjectId
  visibilityContract : Id .visibilityContract
  visibilityEnvelope : ObjectId
  now : Nat
  deriving DecidableEq, Repr, Inhabited

/-- A new realm has no prior admitted invocations or issued effects. Restored
history is validated separately and recovery never makes old activations live. -/
def initialInputsValid (contracts : Contracts) (input : InitialInputs) : Bool :=
  portfolioStateValid input.knowledge && unique (input.authority.grants.map Grant.id) &&
  subset contracts.kernelFloor input.authority.floor &&
  contracts.corpusBindings.contains (input.knowledge.kb.snapshot, input.corpusPin) &&
  input.corpusPin.realm == input.realm && input.corpusPin.family == .corpus &&
  input.activityPin.realm == input.realm && input.activityPin.family == .activity &&
  subset [input.corpusPin.envelope, input.activityPin.envelope,
    input.projectionEnvelope, input.visibilityEnvelope] input.objects &&
  input.world.pending.isEmpty && input.world.applications.isEmpty &&
  input.world.events.isEmpty && input.world.replies.isEmpty

def init (contracts : Contracts) (input : InitialInputs) : Option State :=
  if initialInputsValid contracts input then
    some {
      toOperationalState := { (default : OperationalState) with
        realm := input.realm
        controlStream := input.controlStream
        knowledge := input.knowledge
        catalogRelease := input.knowledge.kb.snapshot
        corpusPin := input.corpusPin
        activityPin := input.activityPin
        projectionContract := input.projectionContract
        projectionEnvelope := input.projectionEnvelope
        visibilityContract := input.visibilityContract
        visibilityEnvelope := input.visibilityEnvelope
        authority := input.authority
        retained := input.objects
        world := input.world
        now := input.now
        alive := true }
      ledger := { material := input.objects }
      cuts := [] }
  else none

end Kctx.Engineering
