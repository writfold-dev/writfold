import Kctx.Engineering.FixtureCalculus

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

def E01_positive : Bool := promoted.any (fun s =>
  s.knowledge == PortfolioExamples.admitted && s.knowledgeJournal == [⟨PortfolioExamples.plan⟩] &&
  s.knowledge.selected == [PortfolioExamples.first.ref, PortfolioExamples.second.ref, PortfolioExamples.third.ref] &&
  s.acknowledged == [⟨1⟩])
def E01_negative : Bool :=
  let bad := { lesson with plan := PortfolioExamples.unsupportedPlan }
  let c := { contracts with
    approvals := [⟨bad.plan⟩], evaluations := [bad.toCandidateMaterial],
    reviews := [bad.toCandidateMaterial], learningApprovals := [⟨bad.toCandidateMaterial, ⟨1⟩, 0, 100⟩] }
  (do let s ← init c inputs
      trace c s [.propose bad, .evaluate bad.id, .review bad.id]).any (fun s : State =>
        s.candidates == [{ bad with evaluated := true, reviewed := true }] &&
        (submit c s (.prepareAdmission bad.id)).isNone)

def twoRoots : Option State := do
  let s ← started
  trace contracts s [.activate runTwo 20, .reserve ⟨1⟩ [⟨1⟩] 1, .reserve ⟨20⟩ [⟨2⟩] 1]
def E02_positive : Bool := twoRoots.any (fun s =>
  s.budgets == [⟨⟨2⟩, 20, 0⟩, ⟨⟨1⟩, 30, 0⟩] &&
  s.ownership == [⟨⟨2⟩, ⟨2⟩, ⟨20⟩, 1, 0, 0⟩, ⟨⟨1⟩, ⟨1⟩, ⟨1⟩, 1, 0, 0⟩] &&
  (Ledger.currentPrefix s.ledger ⟨1⟩).map (·.payload) == [.reserve ⟨1⟩ [⟨1⟩] 1, .activate run 30] &&
  (Ledger.currentPrefix s.ledger ⟨2⟩).map (·.payload) == [.reserve ⟨20⟩ [⟨2⟩] 1, .activate runTwo 20])
def E02_negative : Bool := started.any (fun s =>
  (submit contracts s (.activate { runTwo with input := evidenceValue } 20)).isNone)

def E03_positive : Bool := joined.any (fun s =>
  (findRun s ⟨1⟩).any (fun r => r.control.routes == [(⟨1⟩, ⟨2⟩)] &&
    r.control.pending == [] && r.control.results ==
      [⟨⟨3⟩, evidenceValue, .success⟩, ⟨⟨2⟩, evidenceValue, .success⟩, ⟨⟨1⟩, unitValue, .success⟩]) &&
  (findRun s ⟨3⟩).any (fun r => r.id == run.id && r.parent == some run.invocation &&
    r.status == .completed && r.capabilities == [.check] && r.input == unitValue) &&
  s.returned == [⟨⟨1⟩, ⟨3⟩, ⟨2⟩, binding, leaf.revision, unitValue, evidenceValue, childReceipt⟩] &&
  s.budgets == [⟨⟨1⟩, 30, 5⟩])
def E03_negative : Bool := called.any (fun s =>
  (submit contracts s (.advanceNode ⟨1⟩ ⟨2⟩ evidenceValue)).isNone &&
  (submit contracts s (.advanceNode ⟨1⟩ ⟨5⟩ evidenceValue)).isNone)

def reused : Option State := prepared.bind (fun s => submit contracts s (.reuse ⟨1⟩ receipt))
def E04_positive : Bool := reused.any (fun s =>
  s.reused == [⟨receipt, ⟨1⟩⟩] && s.receipts == [receipt] && receipt.observedAt == 5)
def wrongReceiptRefused (bad : Receipt) : Bool :=
  let c := { contracts with receipts := bad :: contracts.receipts }
  (start c proc).any (fun s => (submit c s (.recordReceipt bad)).isNone)
def E04_negative : Bool := wrongReceiptRefused { receipt with invocation := ⟨99⟩ } &&
  wrongReceiptRefused { receipt with rootInput := ⟨.unit, [⟨3⟩]⟩ } &&
  wrongReceiptRefused { receipt with binding := { binding with dependencies := [⟨2⟩] } }
def wrongRoleRefused : Bool :=
  let bad := { receipt with role := .procedureResult }
  let c := { contracts with receipts := bad :: contracts.receipts }
  (do let s ← start c proc
      trace c s [.reserve ⟨1⟩ [⟨1⟩] 1, .recordReceipt bad]).any (fun s : State =>
    s.receipts == [bad] && (submit c s (.prepare intent 1)).isNone)

def E05_positive : Bool := started.any (fun s =>
  (submit contracts s (.reserve ⟨1⟩ [⟨1⟩] 1)).any (fun t =>
    (Ledger.currentPrefix t.ledger ⟨1⟩).map (·.payload) ==
      [.reserve ⟨1⟩ [⟨1⟩] 1, .activate run 30] &&
    (step contracts t (.append ⟨1⟩ (Ledger.currentHead s.ledger ⟨1⟩) ⟨1000⟩ (.cancel ⟨1⟩))).isNone))
def E05_negative : Bool := twoRoots.any (fun s =>
  (submit contracts s (.reserve ⟨20⟩ [⟨1⟩] 2)).isNone &&
  (step contracts s (.append ⟨2⟩ (Ledger.currentHead s.ledger ⟨2⟩) ⟨1000⟩ (.cancel ⟨1⟩))).isNone)

def released : Option State := do
  let s ← started
  trace contracts s [.reserve ⟨1⟩ [⟨1⟩] 1, .cancel ⟨1⟩, .quiesce ⟨1⟩]
def E06_positive : Bool := released.any (fun s => s.ownership == [] &&
  s.reservationGenerations == [(⟨1⟩, 1)] && (findRun s ⟨1⟩).any (·.status == .cancelled))
def E06_negative : Bool := issued.any (fun s =>
  s.ownership == [⟨⟨1⟩, ⟨1⟩, ⟨1⟩, 1, 1, 1⟩] &&
  (environment contracts s (.terminateProcess ⟨1⟩ 1 0)).isNone &&
  (submit contracts s (.quiesce ⟨1⟩)).isNone)

def settledTree : Option State := do
  let s ← issued
  let applied ← environment contracts s (.completeExternalEffect event 1 0)
  let reconciled ← trace contracts applied [.observe event, .reconcile event]
  environment contracts reconciled (.terminateProcess ⟨1⟩ 1 1)
def staleQuiescenceRefused : Bool := settledTree.any (fun s =>
  s.ownership == [⟨⟨1⟩, ⟨1⟩, ⟨1⟩, 1, 1, 0⟩] &&
  (submit { contracts with quiescent := [(⟨1⟩, 1, 0)] } s (.quiesce ⟨1⟩)).isNone &&
  (submit contracts s (.quiesce ⟨1⟩)).any (fun t => t.ownership == [] &&
    t.reservationGenerations == [(⟨1⟩, 1)]))

def claimCrash : Option State := claimed.bind (fun s => trace contracts s [.crash, .recover])
def lateReconciled : Option State := do
  let s ← issued
  let cancelled ← submit contracts s (.cancel ⟨1⟩)
  let dead ← submit contracts cancelled .crash
  let applied ← environment contracts dead (.completeExternalEffect event 1 0)
  let lost ← environment contracts applied (.loseReply ⟨1⟩)
  let revoked ← environment contracts lost (.revokeGrant ⟨1⟩)
  trace contracts revoked [.recover, .observe event, .reconcile event]
def E07_positive : Bool := lateReconciled.any (fun s =>
  (findEffect s ⟨1⟩).any (fun e => e.outcome == .confirmed && e.dispatch.any (·.issued)) &&
  s.world.provider == ⟨2⟩ && s.world.events == [event] && s.world.pending == [] &&
  s.world.replies == [] && s.observations == [event] && s.incarnation == 1 &&
  (findRun s ⟨1⟩).any (fun r => r.status == .cancelled && !live s r) &&
  s.authority.grants.any (fun g => g.id == ⟨1⟩ && g.revoked))
def E07_negative : Bool := claimCrash.any (fun s =>
  (findEffect s ⟨1⟩).any (fun e => e.outcome == .unknown && e.dispatch.any (fun d => !d.issued)) &&
  s.world.pending == [] && s.world.events == [] &&
  (submit contracts s (.issueEffect ⟨1⟩)).isNone)
def replyLost : Option State := do
  let s ← issued
  let applied ← environment contracts s (.completeExternalEffect event 1 0)
  environment contracts applied (.loseReply ⟨1⟩)
def replyLossUnknown : Bool := replyLost.any (fun s => s.alive && s.incarnation == 0 &&
  s.world.events == [event] && s.world.replies == [] &&
  (findEffect s ⟨1⟩).any (·.outcome == .unknown))
def replyReconciled : Option State := replyLost.bind (fun s => trace contracts s [.observe event, .reconcile event])
def replyLossReconciliation : Bool := replyReconciled.any (fun s =>
  (findEffect s ⟨1⟩).any (·.outcome == .confirmed) && s.world.events == [event])
def noEffectPrepared : Bool := prepared.any (fun s =>
  (submit contracts s (.observe noEffect)).any (fun t =>
    (findEffect t ⟨1⟩).any (·.outcome == .cancelledBeforeDispatch) &&
    t.observations == [noEffect] && (claim contracts t).isNone))
def noEffectClaimed : Bool := claimed.any (fun s =>
  (submit contracts s (.observe claimedNoEffect)).any (fun t =>
    (findEffect t ⟨1⟩).any (·.outcome == .unknown) &&
    (submit contracts t (.issueEffect ⟨1⟩)).isNone &&
    (submit contracts t (.reconcile claimedNoEffect)).any (fun u =>
      (findEffect u ⟨1⟩).any (·.outcome == .failed) && u.world.events == [])))

def E08_positive : Bool := promoted.any (fun s =>
  s.catalogRelease == PortfolioExamples.candidate.snapshot && s.corpusPin == nextCorpusPin &&
  s.retained == inputs.objects && s.admission.isNone && s.acknowledged == [⟨1⟩] &&
  s.knowledgeJournal == [⟨PortfolioExamples.plan⟩])
def E08_negative : Bool :=
  let c := { contracts with learningApprovals := [] }
  (do let s ← init c inputs
      trace c s [.propose lesson, .evaluate ⟨1⟩, .review ⟨1⟩]).any (fun s : State =>
    s.candidates == [{ lesson with evaluated := true, reviewed := true }] &&
    (submit c s (.prepareAdmission ⟨1⟩)).isNone)
def exactMaterialRefused : Bool :=
  let altered := { lesson with evidence := [⟨3⟩] }
  let c := { contracts with evaluations := [altered.toCandidateMaterial], reviews := [altered.toCandidateMaterial] }
  (do let s ← init c inputs
      trace c s [.propose altered, .evaluate ⟨1⟩, .review ⟨1⟩]).any (fun s : State =>
        (submit c s (.prepareAdmission ⟨1⟩)).isNone)
def competingPromotion : Bool :=
  (do let s ← admissionReady
      trace contracts s [.propose competitor, .evaluate ⟨2⟩, .review ⟨2⟩,
        .prepareAdmission ⟨2⟩, .publishAdmission]).any (fun s : State =>
    s.knowledge == PortfolioExamples.admitted &&
    s.knowledgeJournal == [PortfolioJournalEntry.mk PortfolioExamples.plan] &&
    (submit contracts s (.prepareAdmission ⟨1⟩)).isNone)
def lostPublicationAcknowledgement : Bool := published.any (fun s =>
  s.acknowledged == [] && s.admission.isNone &&
  (trace contracts s [.crash, .recover, .acknowledgeAdmission ⟨1⟩]).any (fun t =>
    t.knowledge == s.knowledge && t.knowledgeJournal == s.knowledgeJournal &&
    t.knowledgeJournal == [⟨PortfolioExamples.plan⟩] && t.acknowledged == [⟨1⟩] &&
    (submit contracts t .publishAdmission).isNone))

/-- The projection observation is an explicit qualified input for this exact
native pin, complete head set, and inventory epoch, not synthesized authority. -/
def projected (s : State) (pin : Ledger.NativePin) : Option State :=
  let heads := Ledger.orderedHeads s.ledger
  let c := { contracts with projections := [(pin, heads, s.ledger.inventoryEpoch)] }
  submit c s (.project pin heads s.ledger.inventoryEpoch)
def cutBefore : Option State := do
  let s ← started
  let p ← projected s activityPin
  submit contracts p (.acquireReadCut ⟨1⟩)
def cutAfter : Option State := do
  let s ← cutBefore
  let t ← trace contracts s [.cancel ⟨1⟩, .propose lesson, .evaluate ⟨1⟩, .review ⟨1⟩,
    .prepareAdmission ⟨1⟩, .publishAdmission, .acknowledgeAdmission ⟨1⟩]
  let p ← projected t nextActivityPin
  submit contracts p (.acquireReadCut ⟨2⟩)
def E09_positive : Bool := cutAfter.any (fun s =>
  (readCut s ⟨1⟩).any (fun old => old.1.corpus == PortfolioExamples.initial &&
    old.2.corpusPins == [corpusPin] && old.2.activityPin == activityPin) &&
  (readCut s ⟨2⟩).any (fun new => new.1.corpus == PortfolioExamples.admitted &&
    new.2.corpusPins == [nextCorpusPin] && new.2.activityPin == nextActivityPin &&
    new.2.watermarks.map (·.head) == Ledger.orderedHeads s.ledger) &&
  (readStreamAtCut s ⟨1⟩ lesson.stream).isNone &&
  (readStreamAtCut s ⟨1⟩ run.stream).any (fun entries =>
    entries.map (·.payload) == [.activate run 30]) &&
  (readStreamAtCut s ⟨2⟩ run.stream).any (fun entries =>
    entries.map (·.payload) == [.cancel ⟨1⟩, .activate run 30]) &&
  (readStreamAtCut s ⟨2⟩ lesson.stream).any (fun entries => entries.map (·.payload) ==
    [.acknowledgeAdmission ⟨1⟩, .prepareAdmission ⟨1⟩, .review ⟨1⟩, .evaluate ⟨1⟩, .propose lesson]))
def E09_negative : Bool := started.any (fun s =>
  (submit contracts s (.acquireReadCut ⟨1⟩)).isNone &&
  (projected s { activityPin with realm := ⟨2⟩ }).isNone &&
  (projected s corpusPin).isNone) && cutAfter.any (fun s =>
    (readCut { s with ledger := { s.ledger with material := [] } } ⟨1⟩).isNone)

def nested : Option State := do
  let s ← initial
  trace contracts s [.resolve leaf, .resolve middle, .resolve outer, .resolve top,
    .activate { run with procedure := top } 3,
    .call ⟨1⟩ ⟨1⟩ ⟨2⟩ outer binding, .call ⟨2⟩ ⟨1⟩ ⟨3⟩ middle binding,
    .call ⟨3⟩ ⟨1⟩ ⟨4⟩ leaf binding]
def E10_positive : Bool := nested.any (fun s =>
  s.budgets == [⟨⟨1⟩, 3, 3⟩] && s.runs.map (·.id) == [⟨1⟩, ⟨1⟩, ⟨1⟩, ⟨1⟩] &&
  s.runs.map (·.parent) == [some ⟨3⟩, some ⟨2⟩, some ⟨1⟩, none])
def E10_negative : Bool := nested.any (fun s =>
  (findRun s ⟨4⟩).any (fun r => live s r && r.loaded == []) &&
  (submit contracts s (.loadResource ⟨4⟩ ⟨1⟩)).isNone)
def revisionsPinned : Bool := started.any (fun s =>
  (submit contracts s (.resolve procV2)).any (fun t =>
    t.catalog == [procV2, proc] && (findRun t ⟨1⟩).any (·.procedure == proc) &&
    (submit contracts t (.loadResource ⟨1⟩ ⟨1⟩)).any (fun u =>
      (findRun u ⟨1⟩).any (fun r => r.procedure == proc && r.loaded == [⟨1⟩]))))
def aliasDriftRefused : Bool :=
  let drift := { leaf with revision := ⟨30⟩ }
  let c := { contracts with
    packages := drift :: contracts.packages,
    packageReleases := (PortfolioExamples.base.snapshot, drift) :: contracts.packageReleases }
  (do let s ← init c inputs
      trace c s [.resolve leaf, .resolve drift, .resolve caller, .activate { run with procedure := caller } 30,
        .selectRoute ⟨1⟩ ⟨1⟩ ⟨2⟩ unitValue]).any (fun s : State =>
    (submit c s (.call ⟨1⟩ ⟨2⟩ ⟨3⟩ drift binding)).isNone)
def recursionRefused : Bool :=
  let recursive := { caller with rank := 0 }
  let c := { contracts with
    packages := recursive :: contracts.packages,
    packageReleases := (PortfolioExamples.base.snapshot, recursive) :: contracts.packageReleases }
  (do let s ← init c inputs
      submit c s (.resolve leaf)).any (fun s : State => (submit c s (.resolve recursive)).isNone)

def E11_positive : Bool := issued.any (fun s =>
  (environment contracts s (.completeExternalEffect event 1 0)).any (fun t =>
    t.world.provider == ⟨2⟩ && t.world.producer == 1 && t.world.consumer == 0 &&
    t.world.events == [event] && t.world.replies == [event] && t.world.pending == []))
def E11_negative : Bool := issued.any (fun s =>
  (environment contracts s (.completeExternalEffect event 0 1)).isNone)

def recovered : Option State := started.bind (fun s => trace contracts s [.crash, .recover])
def migrated : Option State := do
  let s ← recovered
  let advanced ← environment contracts s (.advanceUpstream ⟨2⟩)
  submit contracts advanced (.rebind ⟨1⟩ proc migrationBinding ⟨9⟩)
def E12_positive : Bool := migrated.any (fun s =>
  (findRun s ⟨1⟩).any (fun r => r.status == .retired && r.incarnation == 0 && !live s r) &&
  (findRun s ⟨9⟩).any (fun r => r.status == .active && r.incarnation == 1 &&
    r.binding == migrationBinding && r.binding.expectedProvider == ⟨2⟩ && live s r) &&
  s.world.provider == ⟨2⟩ && s.budgets == [⟨⟨1⟩, 30, 1⟩] &&
  (submit contracts s (.loadResource ⟨9⟩ ⟨1⟩)).any (fun t =>
    (findRun t ⟨9⟩).any (·.loaded == [⟨1⟩])))
def E12_negative : Bool := recovered.any (fun s =>
  (findRun s ⟨1⟩).any (fun r => r.status == .active && !live s r) &&
  (submit contracts s (.loadResource ⟨1⟩ ⟨1⟩)).isNone &&
  (submit { contracts with migrations := [] } s (.rebind ⟨1⟩ proc binding ⟨9⟩)).isNone)
def revokedDispatchRefused : Bool := claimed.any (fun s =>
  (environment contracts s (.revokeGrant ⟨1⟩)).any (fun t =>
    (submit contracts t (.issueEffect ⟨1⟩)).isNone) &&
  (environment contracts s (.advanceTime 51)).any (fun t =>
    (submit contracts t (.issueEffect ⟨1⟩)).isNone))

def captured : Option State := initial.bind (fun s => submit contracts s (.observeMaterial capture))
def captureIntroducesObject : Bool := captured.any (fun s =>
  s.retained == ⟨40⟩ :: inputs.objects && s.captures == [capture] &&
  s.ledger.material.contains ⟨40⟩ &&
  (Ledger.currentPrefix s.ledger inputs.controlStream).map (·.payload) == [.observeMaterial capture])
def duplicateCaptureRefused : Bool := captured.any (fun s =>
  (submit contracts s (.observeMaterial capture)).isNone)
def phantomObjectRefused : Bool :=
  let phantom := { proc with closure := [⟨40⟩] }
  let c := { contracts with
    packages := [phantom],
    packageReleases := [(PortfolioExamples.base.snapshot, phantom)] }
  (init c inputs).any (fun s => (submit c s (.resolve phantom)).isNone &&
    (submit c s (.observeMaterial capture)).any (fun t =>
      (submit c t (.resolve phantom)).any (fun u => u.catalog == [phantom])))
def unqualifiedCaptureRefused : Bool := initial.any (fun s =>
  (submit contracts s (.observeMaterial { capture with objects := [⟨41⟩] })).isNone)

def unsuccessfulReturn (outcome : Completion) : Option State := do
  let s ← childChecked
  let r := childFailure outcome
  trace contracts s [.recordReceipt r, .failNode ⟨3⟩ ⟨2⟩ r, .returnChild ⟨1⟩ ⟨3⟩ r]
def unsuccessfulJoinRefused (outcome : Completion) : Bool :=
  (unsuccessfulReturn outcome).any (fun s =>
    (findRun s ⟨1⟩).any (fun r => r.control.results.any (fun result =>
      result.node == ⟨2⟩ && result.outcome == outcome) && !joinReady s r [⟨2⟩]) &&
    s.returned == [⟨⟨1⟩, ⟨3⟩, ⟨2⟩, binding, leaf.revision, unitValue, evidenceValue, childFailure outcome⟩] &&
    (submit contracts s (.advanceNode ⟨1⟩ ⟨3⟩ evidenceValue)).isNone)
def failedJoinRefused : Bool := unsuccessfulJoinRefused .failed
def cancelledJoinRefused : Bool := unsuccessfulJoinRefused .cancelled
def unknownJoinRefused : Bool := unsuccessfulJoinRefused .unknownEffect

def legacyContextIsolation : Bool :=
  let c := { contracts with corpusBindings :=
    (PortfolioExamples.isolatedKB.snapshot, corpusPin) :: contracts.corpusBindings }
  (init c { inputs with knowledge := PortfolioExamples.isolatedState }).any (fun s =>
    s.knowledge.kb.observations == [PortfolioExamples.pObservation, PortfolioExamples.qObservation] &&
    PortfolioExamples.pObservation.content != PortfolioExamples.qObservation.content &&
    usableFor s.knowledge.kb PortfolioExamples.pContext PortfolioExamples.pClaim.ref &&
    !usableFor s.knowledge.kb PortfolioExamples.qContext PortfolioExamples.pClaim.ref)

/- Proof snapshots keep sorting equations separate from kernel execution of
the reducer. Their exact-success theorems below rule out every `getD` fallback;
the executable cases above still use the original runtime sorting path. -/
namespace CutEquationProof

def before : State := started.getD default
def beforeFirst : Ledger.Selection Action HistoricalPremises := before.ledger.selections.getD 0 default
def beforeSecond : Ledger.Selection Action HistoricalPremises := before.ledger.selections.getD 1 default
def projectedBefore : State := { before with
  activityPin := activityPin
  projectedHeads := [beforeFirst.head, beforeSecond.head]
  projectionInventory := before.ledger.inventoryEpoch }
def oldManifest : Ledger.Cut Action HistoricalPremises := {
  id := ⟨1⟩
  realm := projectedBefore.realm
  inventoryEpoch := projectedBefore.projectionInventory
  inventory := [beforeFirst.stream, beforeSecond.stream]
  watermarks := [beforeFirst, beforeSecond]
  corpusPins := [projectedBefore.corpusPin]
  activityPin := projectedBefore.activityPin
  projectionContract := projectedBefore.projectionContract
  projectionEnvelope := projectedBefore.projectionEnvelope
  visibilityContract := projectedBefore.visibilityContract
  visibilityEnvelope := projectedBefore.visibilityEnvelope
  assessmentTime := projectedBefore.now
  material := projectedBefore.retained }
def pinnedBefore : State := { projectedBefore with
  ledger := { projectedBefore.ledger with cuts := oldManifest :: projectedBefore.ledger.cuts }
  cuts := ⟨⟨1⟩, projectedBefore.knowledge⟩ :: projectedBefore.cuts }
def advanceActions : List Action := [.cancel ⟨1⟩, .propose lesson, .evaluate ⟨1⟩,
  .review ⟨1⟩, .prepareAdmission ⟨1⟩, .publishAdmission, .acknowledgeAdmission ⟨1⟩]
def advanced : State := (trace contracts pinnedBefore advanceActions).getD default
def afterFirst : Ledger.Selection Action HistoricalPremises := advanced.ledger.selections.getD 0 default
def afterSecond : Ledger.Selection Action HistoricalPremises := advanced.ledger.selections.getD 1 default
def afterThird : Ledger.Selection Action HistoricalPremises := advanced.ledger.selections.getD 2 default
def projectedAfter : State := { advanced with
  activityPin := nextActivityPin
  projectedHeads := [afterSecond.head, afterFirst.head, afterThird.head]
  projectionInventory := advanced.ledger.inventoryEpoch }
def newManifest : Ledger.Cut Action HistoricalPremises := {
  id := ⟨2⟩
  realm := projectedAfter.realm
  inventoryEpoch := projectedAfter.projectionInventory
  inventory := [afterSecond.stream, afterFirst.stream, afterThird.stream]
  watermarks := [afterSecond, afterFirst, afterThird]
  corpusPins := [projectedAfter.corpusPin]
  activityPin := projectedAfter.activityPin
  projectionContract := projectedAfter.projectionContract
  projectionEnvelope := projectedAfter.projectionEnvelope
  visibilityContract := projectedAfter.visibilityContract
  visibilityEnvelope := projectedAfter.visibilityEnvelope
  assessmentTime := projectedAfter.now
  material := projectedAfter.retained }
def pinnedAfter : State := { projectedAfter with
  ledger := { projectedAfter.ledger with cuts := newManifest :: projectedAfter.ledger.cuts }
  cuts := ⟨⟨2⟩, projectedAfter.knowledge⟩ :: projectedAfter.cuts }

end CutEquationProof
open CutEquationProof

theorem CutEquationProof.started_exact : started = some before := by decide
theorem CutEquationProof.before_selections : before.ledger.selections = [beforeFirst, beforeSecond] := by decide
theorem CutEquationProof.before_sorted : Ledger.orderedSelections before.ledger = [beforeFirst, beforeSecond] := by
  rw [Ledger.orderedSelections, before_selections]
  simp [List.mergeSort, List.MergeSort.Internal.splitInTwo, List.merge] <;> decide
theorem CutEquationProof.projection_before_exact : projected before activityPin = some projectedBefore := by
  simp only [projected, submit, step, localAction, project, Ledger.orderedHeads, before_sorted]
  decide
theorem CutEquationProof.acquire_before_exact : submit contracts projectedBefore (.acquireReadCut ⟨1⟩) = some pinnedBefore := by
  have projected_sorted : Ledger.orderedSelections projectedBefore.ledger = [beforeFirst, beforeSecond] := by
    change Ledger.orderedSelections before.ledger = [beforeFirst, beforeSecond]
    exact before_sorted
  simp only [submit, step, localAction, acquireCut, Ledger.pin, Ledger.CutReady,
    Ledger.orderedHeads, Ledger.orderedInventory, projected_sorted]
  decide
theorem CutEquationProof.cut_before_exact : cutBefore = some pinnedBefore := by
  simp only [cutBefore, started_exact, bind, Option.bind, projection_before_exact, acquire_before_exact]
theorem CutEquationProof.advance_exact : trace contracts pinnedBefore advanceActions = some advanced := by decide
theorem CutEquationProof.after_selections : advanced.ledger.selections = [afterFirst, afterSecond, afterThird] := by decide
theorem CutEquationProof.after_sorted : Ledger.orderedSelections advanced.ledger = [afterSecond, afterFirst, afterThird] := by
  have first_second : ¬ afterFirst.stream.value ≤ afterSecond.stream.value := by decide
  have first_third : afterFirst.stream.value ≤ afterThird.stream.value := by decide
  have second_third : afterSecond.stream.value ≤ afterThird.stream.value := by decide
  rw [Ledger.orderedSelections, after_selections]
  simp [List.mergeSort, List.MergeSort.Internal.splitInTwo, List.merge,
    first_second, first_third, second_third]
theorem CutEquationProof.projection_after_exact : projected advanced nextActivityPin = some projectedAfter := by
  simp only [projected, submit, step, localAction, project, Ledger.orderedHeads, after_sorted]
  decide
theorem CutEquationProof.acquire_after_exact : submit contracts projectedAfter (.acquireReadCut ⟨2⟩) = some pinnedAfter := by
  have projected_sorted : Ledger.orderedSelections projectedAfter.ledger = [afterSecond, afterFirst, afterThird] := by
    change Ledger.orderedSelections advanced.ledger = [afterSecond, afterFirst, afterThird]
    exact after_sorted
  simp only [submit, step, localAction, acquireCut, Ledger.pin, Ledger.CutReady,
    Ledger.orderedHeads, Ledger.orderedInventory, projected_sorted]
  decide
theorem CutEquationProof.cut_after_exact : cutAfter = some pinnedAfter := by
  change (do
    let s ← cutBefore
    let t ← trace contracts s advanceActions
    let p ← projected t nextActivityPin
    submit contracts p (.acquireReadCut ⟨2⟩)) = some pinnedAfter
  simp only [cut_before_exact, bind, Option.bind, advance_exact,
    projection_after_exact, acquire_after_exact]
theorem CutEquationProof.positive : E09_positive = true := by
  have pinned_sorted : Ledger.orderedSelections pinnedAfter.ledger = [afterSecond, afterFirst, afterThird] := by
    change Ledger.orderedSelections advanced.ledger = [afterSecond, afterFirst, afterThird]
    exact after_sorted
  simp only [E09_positive, cut_after_exact, Option.any, Ledger.orderedHeads, pinned_sorted]
  decide
theorem CutEquationProof.negative : E09_negative = true := by
  simp only [E09_negative, cut_after_exact, started_exact, Option.any,
    submit, step, localAction, acquireCut, projected, project,
    Ledger.orderedHeads, Ledger.orderedInventory, before_sorted]
  decide

/-- Kernel-reduced finite witnesses, not native-code axioms or product qualification. -/
theorem E01_nonvacuous : E01_positive = true ∧ E01_negative = true := by decide
theorem E02_nonvacuous : E02_positive = true ∧ E02_negative = true := by decide
theorem E03_nonvacuous : E03_positive = true ∧ E03_negative = true := by decide
theorem E04_nonvacuous : E04_positive = true ∧ E04_negative = true ∧ wrongRoleRefused = true := by decide
theorem E05_nonvacuous : E05_positive = true ∧ E05_negative = true := by decide
theorem E06_nonvacuous : E06_positive = true ∧ E06_negative = true := by decide
theorem E07_nonvacuous : E07_positive = true ∧ E07_negative = true := by decide
theorem E08_nonvacuous : E08_positive = true ∧ E08_negative = true := by decide
/-- Concrete sort equations and exact transition witnesses compose without
native evaluation or changes to the executable cut assertions. -/
theorem E09_nonvacuous : E09_positive = true ∧ E09_negative = true :=
  ⟨CutEquationProof.positive, CutEquationProof.negative⟩
theorem E10_nonvacuous : E10_positive = true ∧ E10_negative = true := by decide
theorem E11_nonvacuous : E11_positive = true ∧ E11_negative = true := by decide
theorem E12_nonvacuous : E12_positive = true ∧ E12_negative = true := by decide
theorem reply_loss_nonvacuous : replyLossUnknown = true ∧ replyLossReconciliation = true := by decide
theorem no_effect_closes_admission : noEffectPrepared = true ∧ noEffectClaimed = true := by decide
theorem exact_learning_nonvacuous : exactMaterialRefused = true ∧ competingPromotion = true ∧
  lostPublicationAcknowledgement = true := by decide
theorem package_pins_nonvacuous : revisionsPinned = true ∧ aliasDriftRefused = true ∧ recursionRefused = true := by decide
theorem capture_nonvacuous : captureIntroducesObject = true ∧ duplicateCaptureRefused = true ∧
  phantomObjectRefused = true ∧ unqualifiedCaptureRefused = true := by decide
theorem non_success_cannot_join : failedJoinRefused = true ∧ cancelledJoinRefused = true ∧ unknownJoinRefused = true := by decide
theorem revoked_dispatch_nonvacuous : revokedDispatchRefused = true := by decide
theorem legacy_context_isolation_nonvacuous : legacyContextIsolation = true := by decide
theorem stale_quiescence_nonvacuous : staleQuiescenceRefused = true := by decide

end Kctx.Engineering.Examples
