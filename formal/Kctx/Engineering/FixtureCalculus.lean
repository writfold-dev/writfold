import Kctx.Engineering.Actions
import Kctx.PortfolioExamples

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- These are explicit qualified boundary inputs. Membership does not prove
source truth, predicate soundness, script correctness, or adapter conformance. -/
def binding : Binding := { (default : Binding) with
  realm := ⟨1⟩, subject := ⟨1⟩, dependencies := [⟨1⟩], revision := ⟨1⟩,
  expectedProvider := ⟨1⟩, inputType := .unit }
def migrationBinding : Binding := { binding with revision := ⟨2⟩, expectedProvider := ⟨2⟩ }
def unitValue : TypedValue := ⟨.unit, []⟩
def evidenceValue : TypedValue := ⟨.evidence, [⟨2⟩]⟩
def baseNode (id : Nat) (form : NodeForm) : Node :=
  { (default : Node) with
    id := ⟨id⟩, form := form, input := .unit, output := .unit,
    context := ⟨1⟩, scope := [⟨1⟩], ceiling := [.read, .check], floor := [.review], cost := 1 }
def proc : Procedure :=
  { (default : Procedure) with
    id := ⟨1⟩, revision := ⟨1⟩, package := ⟨1⟩,
    closure := [⟨1⟩], inputType := .unit, outputType := .unit,
    ceiling := [.read, .check], floor := [.review], entry := ⟨1⟩,
    nodes := [{ baseNode 1 (.effect .check) with resources := [⟨1⟩] },
      { baseNode 2 .«end» with prerequisites := [⟨1⟩] }] }
def procV2 : Procedure := { proc with revision := ⟨2⟩, package := ⟨2⟩ }
def leaf : Procedure := { proc with
  id := ⟨3⟩, revision := ⟨3⟩,
  outputType := .evidence,
  nodes := [{ baseNode 1 (.check ⟨1⟩) with output := .evidence },
    { baseNode 2 .«end» with
      input := .evidence, output := .evidence,
      dependencies := [⟨⟨1⟩, .evidence⟩] }] }
def target (p : Procedure) : CallTarget := ⟨p.id, p.revision, p.inputType, p.outputType, [.check]⟩
def caller : Procedure := { proc with
  id := ⟨4⟩, revision := ⟨4⟩, rank := 1,
  outputType := .evidence,
  nodes := [baseNode 1 (.select [⟨2⟩, ⟨5⟩]),
    { baseNode 2 (.call (target leaf)) with output := .evidence, preconditions := [⟨1⟩] },
    { baseNode 3 (.join [⟨2⟩]) with output := .evidence, prerequisites := [⟨1⟩] },
    { baseNode 4 .«end» with
      input := .evidence, output := .evidence,
      dependencies := [⟨⟨3⟩, .evidence⟩] },
    { baseNode 5 .«end» with output := .evidence }] }
def wrapper (id : Nat) (p : Procedure) : Procedure :=
  { proc with
    id := ⟨id⟩, revision := ⟨id⟩, rank := p.rank + 1,
    outputType := p.outputType,
    nodes := [{ baseNode 1 (.call (target p)) with output := p.outputType },
      { baseNode 2 .«end» with
        input := p.outputType, output := p.outputType,
        dependencies := [⟨⟨1⟩, p.outputType⟩] }] }
def middle : Procedure := wrapper 5 leaf
def outer : Procedure := wrapper 6 middle
def top : Procedure := wrapper 7 outer

def run : Run := { (default : Run) with
  id := ⟨1⟩, stream := ⟨1⟩, invocation := ⟨1⟩,
  procedure := proc, binding := binding, capabilities := [.read, .check], floor := [.review],
  status := .active, policy := ⟨1⟩, input := unitValue }
def runTwo : Run := { run with id := ⟨2⟩, stream := ⟨2⟩, invocation := ⟨20⟩ }
def grant (id root : Nat) : Grant := ⟨⟨id⟩, ⟨root⟩, binding, [⟨1⟩, ⟨2⟩],
  [.read, .check], 100, 0, 100, false⟩
def authority : Authority := { (default : Authority) with
  policy := ⟨1⟩,
  floor := [.review], grants := [grant 1 1, grant 2 2,
    { grant 3 1 with binding := migrationBinding }] }
def corpusPin : Ledger.NativePin :=
  { realm := ⟨1⟩, family := .corpus, graph := ⟨1⟩, graphCommit := ⟨1⟩,
    schemaEpoch := ⟨1⟩, query := ⟨1⟩, compatibilityProfile := ⟨1⟩, envelope := ⟨10⟩ }
def nextCorpusPin : Ledger.NativePin := { corpusPin with
  graphCommit := ⟨2⟩,
  schemaEpoch := ⟨2⟩, envelope := ⟨11⟩ }
def activityPin : Ledger.NativePin := { corpusPin with family := .activity, graph := ⟨2⟩ }
def nextActivityPin : Ledger.NativePin := { activityPin with
  graphCommit := ⟨2⟩,
  schemaEpoch := ⟨2⟩, envelope := ⟨11⟩ }
def inputs : InitialInputs :=
  { realm := ⟨1⟩, controlStream := ⟨100⟩, knowledge := PortfolioExamples.initial,
    authority := authority, objects := [⟨1⟩, ⟨2⟩, ⟨3⟩, ⟨10⟩, ⟨11⟩],
    world := { (default : World) with provider := ⟨1⟩ },
    corpusPin := corpusPin, activityPin := activityPin,
    projectionContract := ⟨1⟩, projectionEnvelope := ⟨10⟩,
    visibilityContract := ⟨1⟩, visibilityEnvelope := ⟨10⟩, now := 10 }
def receipt : Receipt := { (default : Receipt) with
  role := .gate, effect := ⟨1⟩,
  binding := binding, procedure := proc.revision, invocation := ⟨1⟩, node := ⟨1⟩,
  gates := [.review], objects := [⟨2⟩], observedAt := 5, expires := 50,
  policy := ⟨1⟩, complete := true, outputType := .evidence,
  rootInput := unitValue, input := unitValue, output := evidenceValue, outcome := .success }
def childReceipt : Receipt := { receipt with
  role := .procedureResult,
  procedure := leaf.revision, invocation := ⟨3⟩, node := ⟨2⟩, input := evidenceValue }
def childFailure (outcome : Completion) : Receipt := { childReceipt with outcome := outcome }
def predicateEvidence (activation : Nat) : TypedEvidence :=
  ⟨⟨1⟩, ⟨activation⟩, binding, unitValue, evidenceValue, [⟨2⟩], ⟨1⟩, 0, 50⟩
def intent : Intent := ⟨⟨1⟩, ⟨1⟩, ⟨1⟩, ⟨1⟩, binding, proc.revision,
  .check, [⟨1⟩], ⟨1⟩, [⟨1⟩, ⟨2⟩]⟩
def event : ExternalEvent := {
  effect := ⟨1⟩, binding := binding, expected := ⟨1⟩,
  result := .applied, providerAfter := ⟨2⟩, claim := some ⟨1⟩ }
def noEffect : ExternalEvent := { event with
  result := .rejected, providerAfter := ⟨1⟩,
  claim := none }
def claimedNoEffect : ExternalEvent := { noEffect with claim := some ⟨1⟩ }
def lesson : Candidate :=
  { id := ⟨1⟩, stream := ⟨10⟩, plan := PortfolioExamples.plan, corpusPin := nextCorpusPin,
    evidence := [⟨1⟩], procedure := none, rubric := ⟨2⟩, trials := [⟨3⟩],
    evaluated := false, reviewed := false }
def competitor : Candidate := { lesson with id := ⟨2⟩, stream := ⟨11⟩ }
def capture : RetainedObservation := ⟨⟨1⟩, ⟨1⟩, [⟨40⟩], 9⟩
def packages : List Procedure := [proc, procV2, leaf, caller, middle, outer, top]
def contracts : Contracts := { (default : Contracts) with
  kernelFloor := [.review], packages := packages, predicates := [⟨⟨1⟩, .unit, .evidence⟩],
  typedEvidence := [predicateEvidence 1, predicateEvidence 3],
  packageReleases := packages.map (fun p => (PortfolioExamples.base.snapshot, p)) ++
    packages.map (fun p => (PortfolioExamples.candidate.snapshot, p)),
  corpusBindings := [(PortfolioExamples.base.snapshot, corpusPin),
    (PortfolioExamples.candidate.snapshot, nextCorpusPin)],
  receipts := [receipt, childReceipt, childFailure .failed, childFailure .cancelled,
    childFailure .unknownEffect], observations := [event, noEffect, claimedNoEffect],
  effectAuthorizations := [intent], compatibility := [(⟨2⟩, 1, 0), (⟨2⟩, 1, 1)],
  approvals := [⟨PortfolioExamples.plan⟩],
  evaluations := [lesson.toCandidateMaterial, competitor.toCandidateMaterial],
  reviews := [lesson.toCandidateMaterial, competitor.toCandidateMaterial],
  learningApprovals := [⟨lesson.toCandidateMaterial, ⟨1⟩, 0, 100⟩,
    ⟨competitor.toCandidateMaterial, ⟨1⟩, 0, 100⟩],
  quiescent := [(⟨1⟩, 1, 0), (⟨1⟩, 1, 1), (⟨2⟩, 1, 0)],
  migrations := [⟨⟨1⟩, binding, binding, 1, ⟨1⟩⟩,
    ⟨⟨1⟩, binding, migrationBinding, 1, ⟨2⟩⟩],
  materialObservations := [capture] }
def initial : Option State := init contracts inputs

/-- Convenience only: every durable action still traverses Request.append and
historical replay. Physical send, publication, and local actions stay distinct. -/
def submit (c : Contracts) (s : State) (a : Action) : Option State :=
  match a with
  | .publishAdmission => do
      let stream ← actionStream s a
      step c s (.publish stream (Ledger.currentHead s.ledger stream))
  | .issueEffect effect => do
      let stream ← effectStream s effect
      step c s (.dispatch stream (Ledger.currentHead s.ledger stream) effect)
  | .project _ _ _ | .acquireReadCut _ | .releaseReadCut _ | .crash | .recover => step c s (.local a)
  | _ => do
      let stream ← actionStream s a
      step c s (.append stream (Ledger.currentHead s.ledger stream)
        ⟨s.ledger.records.length + 1⟩ a)
def trace (c : Contracts) : State → List Action → Option State
  | s, [] => some s
  | s, a :: rest => (submit c s a).bind (fun next => trace c next rest)
def start (c : Contracts) (p : Procedure) (budget : Nat := 30) : Option State := do
  let s ← init c inputs
  trace c s [.resolve p, .activate { run with procedure := p } budget]
def started : Option State := start contracts proc
def prepared : Option State := do
  let s ← started
  trace contracts s [.reserve ⟨1⟩ [⟨1⟩] 1, .recordReceipt receipt, .prepare intent 1]
def claim (c : Contracts) (s : State) : Option State :=
  submit c s (.claimDispatch ⟨1⟩ ⟨1⟩ (Ledger.currentHead s.ledger ⟨1⟩)
    ⟨s.ledger.records.length + 1⟩)
def claimed : Option State := prepared.bind (claim contracts)
def issued : Option State := claimed.bind (fun s => submit contracts s (.issueEffect ⟨1⟩))
def admissionReady : Option State := do
  let s ← initial
  trace contracts s [.propose lesson, .evaluate ⟨1⟩, .review ⟨1⟩, .prepareAdmission ⟨1⟩]
def published : Option State := admissionReady.bind (fun s => submit contracts s .publishAdmission)
def promoted : Option State := published.bind (fun s => submit contracts s (.acknowledgeAdmission ⟨1⟩))
def called : Option State := do
  let s ← initial
  trace contracts s [.resolve leaf, .resolve caller, .activate { run with procedure := caller } 30,
    .selectRoute ⟨1⟩ ⟨1⟩ ⟨2⟩ unitValue, .call ⟨1⟩ ⟨2⟩ ⟨3⟩ leaf binding]
def childChecked : Option State := called.bind (fun s => submit contracts s (.advanceNode ⟨3⟩ ⟨1⟩ evidenceValue))
def returnedCall : Option State := do
  let s ← childChecked
  trace contracts s [.advanceNode ⟨3⟩ ⟨2⟩ evidenceValue, .recordReceipt childReceipt,
    .returnChild ⟨1⟩ ⟨3⟩ childReceipt]
def joined : Option State := returnedCall.bind (fun s => submit contracts s (.advanceNode ⟨1⟩ ⟨3⟩ evidenceValue))

end Kctx.Engineering.Examples
