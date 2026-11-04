import Kctx.Engineering.Learning
import Kctx.Engineering.OperationalInvariant
import Kctx.Engineering.WorldEffects

set_option autoImplicit false
namespace Kctx.Engineering

/-- Whether this action appends EngineeringLedger history. Corpus publication
has its own completed-release CAS and a later, separately ledgered acknowledgement. -/
def Action.durable : Action → Bool
  | .publishAdmission | .issueEffect _ | .project _ _ _ | .acquireReadCut _ | .releaseReadCut _
  | .crash | .recover => false
  | _ => true

def claimStampBound (expected : Option Ledger.Head) (record : LedgerRecordId) : Action → Bool
  | .claimDispatch _ _ stampExpected stampRecord | .retryClaim _ _ stampExpected stampRecord =>
      stampExpected == expected && stampRecord == record
  | _ => true

def inputClosure (before : OperationalState) (action : Action) : List ObjectId :=
  before.retained ++ match action with
    | .observeMaterial observation => observation.objects
    | _ => []

def activationStream (s : State) (id : ActivationId) : Option StreamId :=
  (findRun s id).map Run.stream

def effectStream (s : State) (id : Id .effect) : Option StreamId := do
  let effect ← findEffect s id
  activationStream s effect.intent.invocation

def candidateStream (s : State) (id : Id .candidate) : Option StreamId :=
  (s.candidates.find? (·.id == id)).map (fun candidate : Candidate => candidate.stream)

/-- The selected owner determines a stream. A caller cannot route a run event
through a second run's head or use the catalog stream to bypass its expected head. -/
def actionStream (s : State) : Action → Option StreamId
  | .resolve _ | .observeMaterial _ => some s.controlStream
  | .activate run _ => some run.stream
  | .loadResource run _ | .advanceNode run _ _ | .failNode run _ _
  | .selectRoute run _ _ _ | .call run _ _ _ _ | .reserve run _ _
  | .reuse run _ | .returnChild run _ _ | .rebind run _ _ _
  | .resume run | .reopen run _ | .cancel run | .retire run => activationStream s run
  | .prepare intent _ => activationStream s intent.invocation
  | .claimDispatch effect _ _ _ | .retryClaim effect _ _ _ | .issueEffect effect => effectStream s effect
  | .observe event | .reconcile event => effectStream s event.effect
  | .recordReceipt receipt => activationStream s receipt.invocation
  | .quiesce root => (s.runs.find? (·.id == root)).map Run.stream
  | .propose candidate => some candidate.stream
  | .evaluate id | .review id | .prepareAdmission id | .acknowledgeAdmission id =>
      candidateStream s id
  | .publishAdmission => s.admission.map (·.candidate.stream)
  | .project _ _ _ | .acquireReadCut _ | .releaseReadCut _ | .crash | .recover => none

/-- Qualified absence is stronger than timeout: there is neither an in-flight
operation nor an applied/partial effect. The adapter must supply the exact
conclusive observation; these checks never synthesize one from an exception. -/
def observationAvailable (contracts : Contracts) (s : State) (intent : Intent)
    (event : ExternalEvent) : Bool :=
  contracts.observations.contains event &&
  (if event.result == .rejected then
    !s.world.pending.any (fun pending => pending.intent.effect == event.effect) &&
    !s.world.applications.any (fun applied => applied.intent.effect == event.effect)
  else canonicalObservation s intent event)

def observeEffect (contracts : Contracts) (s : State) (event : ExternalEvent) : Option State := do
  let effect ← findEffect s event.effect
  if event.binding == effect.intent.binding && event.expected == effect.intent.expectedProvider &&
      (event.result != .rejected || event.claim == effect.dispatch.map Dispatch.id) &&
      observationAvailable contracts s effect.intent event then
    -- Close both a preparation and an unissued claim before retaining no-effect
    -- evidence, so neither claim nor send can race later reconciliation.
    let next := if event.result == .rejected then
      if effect.outcome == .prepared then
        putEffect s { effect with outcome := .cancelledBeforeDispatch }
      else if effect.outcome == .claimed then putEffect s { effect with outcome := .unknown }
      else s
    else s
    some { next with observations := event :: s.observations }
  else none

/-- Observation of completed retention introduces data, never authority or
fresh evidence timestamps. Repeated capture IDs cannot change their input. -/
def observeMaterial (contracts : Contracts) (s : State)
    (observation : RetainedObservation) : Option State :=
  if contracts.materialObservations.contains observation && observation.realm == s.realm &&
      observation.capturedAt ≤ s.now && unique observation.objects &&
      !s.captures.any (·.id == observation.id) then
    some { s with
      retained := observation.objects.filter (fun object => !s.retained.contains object) ++ s.retained
      captures := observation :: s.captures }
  else none

/-- Domain changes do not write the ledger or read-cut collection. Local projection,
crash and recovery have separate owners and cannot be smuggled in as ledger events. -/
def domainBody (contracts : Contracts) (s : State) : Action → Option State
  | .resolve p => resolve contracts s p
  | .activate r budget => activate contracts s r budget
  | .loadResource r object => loadResource s r object
  | .advanceNode r node value => advanceNode contracts s r node value
  | .failNode r node receipt => failNode contracts s r node receipt
  | .selectRoute r node route value => selectRoute contracts s r node route value
  | .call parent node child procedure binding => call contracts s parent node child procedure binding
  | .prepare intent generation => prepare contracts s intent generation
  | .reserve r keys generation => reserve s r keys generation
  | .claimDispatch effect claim expected record => claimDispatch contracts s effect claim expected record
  | .retryClaim effect claim expected record => retryClaim contracts s effect claim expected record
  | .issueEffect effect => issueEffect contracts s effect
  | .observe event => observeEffect contracts s event
  | .observeMaterial observation => observeMaterial contracts s observation
  | .reconcile event => reconcile s event
  | .recordReceipt receipt => do
      let run ← findRun s receipt.invocation
      if contracts.receipts.contains receipt &&
          retained s (receipt.objects ++ receipt.rootInput.objects ++ receipt.input.objects ++ receipt.output.objects) &&
          receipt.rootInput == run.input && receipt.binding == run.binding &&
          receipt.procedure == run.procedure.revision &&
          (findNode run.procedure receipt.node).isSome && receipt.outputType == receipt.output.type then
        some { s with receipts := receipt :: s.receipts }
      else none
  | .reuse run receipt => reuse s run receipt
  | .returnChild parent child receipt => returnChild contracts s parent child receipt
  | .rebind run procedure binding invocation => rebind contracts s run procedure binding invocation
  | .resume run => resume contracts s run
  | .reopen run command => reopen s run command
  | .cancel run => cancel s run
  | .quiesce root => quiesce contracts s root
  | .propose candidate => propose s candidate
  | .evaluate id => evaluate contracts s id
  | .review id => review contracts s id
  | .prepareAdmission id => prepareAdmission contracts s id
  | .publishAdmission => publishAdmission contracts s
  | .acknowledgeAdmission id => acknowledgeAdmission s id
  | .retire run => retire s run
  | .project _ _ _ | .acquireReadCut _ | .releaseReadCut _ | .crash | .recover => none

/-- A structural writer boundary, not a post-state invariant filter. -/
def frameDomain (before : State) (action : Action) (after : State) : State :=
  { after with
    realm := before.realm
    controlStream := before.controlStream
    ledger := before.ledger
    cuts := before.cuts
    authority := before.authority
    now := before.now
    incarnation := before.incarnation
    alive := before.alive
    knowledge := if action = .publishAdmission then after.knowledge else before.knowledge
    knowledgeJournal := if action = .publishAdmission then after.knowledgeJournal else before.knowledgeJournal }

def domain (contracts : Contracts) (s : State) (action : Action) : Option State :=
  (domainBody contracts s action).map (frameDomain s action)

def historicalState (entries : List (Ledger.Record Action HistoricalPremises))
    (record : Ledger.Record Action HistoricalPremises) : State :=
  { toOperationalState := record.historical.before
    ledger := Ledger.fromPrefix record.stream entries record.historical.before.retained
    cuts := [] }

/-- Replay uses the actual checked reducer and exact captured inputs. Present
policy, current graph HEAD and projection rows are not substituted into old events. -/
def replay (entries : List (Ledger.Record Action HistoricalPremises))
    (record : Ledger.Record Action HistoricalPremises) : Option OperationalState :=
  (domain record.historical.contracts (historicalState entries record) record.payload).map
    State.toOperationalState

def historicalValid : Ledger.Validator Action HistoricalPremises := fun entries record =>
  record.profile == ⟨1⟩ && record.payload.durable && record.historical.before.alive &&
  operationalValid record.historical.contracts record.historical.before &&
  record.closure == inputClosure record.historical.before record.payload &&
  claimStampBound (entries.head?.map Ledger.Record.head) record.id record.payload &&
  actionStream (historicalState entries record) record.payload == some record.stream &&
  (replay entries record).isSome

end Kctx.Engineering
