--------------------------- MODULE M01M02Bridge ---------------------------
EXTENDS M02RecoveryReplay
USE ParameterBounds DEF Bounds

(***************************************************************************
Shared-state composition: p is exactly M01's state, not a separately evolving
copy. r is recovery-local control/observations. The bridge proves a durable
observation contract, NOT an undocumented refinement of volatile writer PCs.

PublicationView: completed ref, retained chain, exact release input records.
ReplayView: the same ref/chain, immutable operation-ID -> input binding and
receipt knowledge. The equality below is nontrivial: matching operation IDs
alone does not imply equal input; BoundIntent and the published invariant do.

Before CAS: durable intent/native/manifest may exist, but no application or
receipt is visible. Resume changes only volatile request/control and must use
exact input plus unchanged predecessor. After CAS: application already exists
in retained ancestry; RepairCompletion is M01 RecordReceipt, NOT DoCAS.
Hints, corruption refusal and crashes are stutters of durable observations.

Adapter requirements: one exact canonical-identity interning/byte-equality
mapping across reducers; one completed Git ref/ancestry observation; actual
native commit and manifest closure retention acknowledgments; verified current
per-context admission; CAS effect includes observed predecessor and definitive
success, failure or unknown (unknown must not be synthesized as rejection).
Absent such an adapter these are abstract projection theorems only.
***************************************************************************)
PublicationView == [head |-> p.head, applied |-> p.published \ {0},
  records |-> [i \in p.published \ {0} |-> Input(i)], retained |-> p.objects,
  legacyInput |-> p.legacy.manifest.input]
ReplayView == [head |-> p.head, applied |-> p.published \ {0},
  operation |-> p.intent, completed |-> p.receipt,
  legacyInput |-> p.legacy.receipt.input]
ExactAgreement ==
  /\ PublicationView.head = ReplayView.head
  /\ PublicationView.applied = ReplayView.applied
  /\ (\A i \in PublicationView.applied :
       PublicationView.records[i] = Input(ReplayView.operation[OpOf(i)]))
  /\ PublicationView.legacyInput = ReplayView.legacyInput
  /\ PublicationView.legacyInput = LegacyInput
SingleApplication == \A i,j \in PublicationView.applied :
  OpOf(i) = OpOf(j) => Input(i) = Input(j)
BeforeCASInvisible == \A i \in p.manifest \ p.published : i \notin ReplayView.completed
BridgeInv == RecoveryInv /\ ExactAgreement /\ SingleApplication /\ BeforeCASInvisible
BridgeSafety == RecoverySafety /\ SingleApplication /\ ExactAgreement

(* Abstract durable effect order. It is deliberately not Safety == Inv: it
   constrains pairs of states, including all failed/stuttering effect results. *)
DurableOrder ==
  /\ p.published \subseteq p'.published
  /\ p.native \subseteq p'.native /\ p.manifest \subseteq p'.manifest
  /\ p.objects \subseteq p'.objects /\ p.receipt \subseteq p'.receipt
  /\ (\A o \in Ops : p.intent[o] # 0 => p'.intent[o] = p.intent[o])
  /\ (\A i \in p'.native \ p.native : BoundIntent(i))
  /\ (\A i \in p'.manifest \ p.manifest : i \in p.native)
  /\ UNCHANGED p.legacy
  /\ p'.receipt \ p.receipt \subseteq p.published
  /\ (p'.head = p.head => p'.published = p.published)
  /\ (p'.head # p.head =>
       \E i \in Inputs :
         /\ p'.head = i /\ p.head = Input(i).predecessor
         /\ i \notin p.published /\ p'.published = p.published \cup {i}
         /\ Prepared(i) /\ Valid(i) /\ p'.evidence[i] = p.auth[i])
AbstractInit == /\ PublicationView.head = 0 /\ PublicationView.applied = {}
                /\ ReplayView.completed = {}
AbstractSpec == AbstractInit /\ [][DurableOrder]_Durable
BridgeSpec == RecoverySpec
FiniteBridgeSpec == FiniteRecoverySpec
WideBridgeSpec(profile) == WideRecoverySpec(profile)
WideBridgeSpec0 == WideBridgeSpec(0)
WideBridgeSpec1 == WideBridgeSpec(1)
WideBridgeSpec2 == WideBridgeSpec(2)
WideBridgeSpec3 == WideBridgeSpec(3)
CrossHintBridgeSpec == CrossHintRecoverySpec
WorkloadBridgeSpec(workload) == WorkloadRecoverySpec(workload)
SameInputBridgeSpec == WorkloadBridgeSpec(1)
SuccessorBridgeSpec == WorkloadBridgeSpec(2)
LegacyWorkloadBridgeSpec == WorkloadBridgeSpec(3)

(* Keep record typing separate from the immutable historical bytes. Action
   frames need the former, but never the expansion of LegacyHistory. *)
THEOREM BridgePublicationTypes == RecoveryInv => TypeOK /\ RecoveryTypeOK
BY DEF RecoveryInv, Inv

THEOREM HistoricalAgreement == HistoricalAuthentic =>
  /\ PublicationView.legacyInput = ReplayView.legacyInput
  /\ PublicationView.legacyInput = LegacyInput
  /\ p.legacy.input.domain = "kctx-operation-v2"
BY SMT DEF HistoricalAuthentic, PublicationView, ReplayView

THEOREM PublishedInputBinding == Inv =>
  \A i \in p.published \ {0} :
    /\ p.intent[OpOf(i)] = i
    /\ p.evidence[i] = FullChecks
    /\ Closure(i) \subseteq p.objects
BY SMT DEF Inv, PublicationChain, Prepared, BoundIntent

THEOREM AgreementFromStrengthening == RecoveryInv =>
  ExactAgreement /\ SingleApplication /\ BeforeCASInvisible
<1>1. RecoveryInv => HistoricalAuthentic
  BY InvHistoricalAuthentic DEF InvHistoricalAuthentic, RecoveryInv
<1>2. RecoveryInv =>
  /\ PublicationView.legacyInput = ReplayView.legacyInput
  /\ PublicationView.legacyInput = LegacyInput
  BY <1>1, HistoricalAgreement DEF HistoricalAgreement
<1>3. RecoveryInv => ExactAgreement
  BY SMT, <1>2, PublishedInputBinding
    DEF PublishedInputBinding, RecoveryInv, ExactAgreement, PublicationView, ReplayView
<1>4. RecoveryInv => SingleApplication
  BY SMT, PublishedInputBinding
    DEF PublishedInputBinding, RecoveryInv, SingleApplication, PublicationView
<1>5. RecoveryInv => BeforeCASInvisible
  BY SMT DEF RecoveryInv, Inv, ReceiptSound, BeforeCASInvisible, ReplayView
<1>6. QED BY <1>3, <1>4, <1>5

(* Post-state agreement is established directly from the post-state invariant;
   a primed theorem name is not a separately proved backend hypothesis. *)
THEOREM AgreementFromStrengtheningNext == RecoveryInv' =>
  ExactAgreement' /\ SingleApplication' /\ BeforeCASInvisible'
<1>1. RecoveryInv' => HistoricalAuthentic'
  BY InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext, RecoveryInv
<1>2. RecoveryInv' =>
  /\ p'.legacy.manifest.input = p'.legacy.receipt.input
  /\ p'.legacy.manifest.input = LegacyInput
  BY SMT, <1>1 DEF HistoricalAuthentic
<1>3. RecoveryInv' => \A i \in p'.published \ {0} : p'.intent[OpOf(i)] = i
  BY SMT DEF RecoveryInv, Inv, PublicationChain, Prepared, BoundIntent
<1>4. RecoveryInv' => ExactAgreement'
  BY SMT, <1>2, <1>3 DEF ExactAgreement, PublicationView, ReplayView
<1>5. RecoveryInv' => SingleApplication'
  BY SMT, <1>3 DEF SingleApplication, PublicationView
<1>6. RecoveryInv' => BeforeCASInvisible'
  BY SMT DEF RecoveryInv, Inv, ReceiptSound, BeforeCASInvisible, ReplayView
<1>7. QED BY <1>4, <1>5, <1>6

THEOREM BridgeInitPreserves == RecoveryInit => BridgeInv /\ AbstractInit
<1>1. RecoveryInit => RecoveryInv
  BY RecoveryInitPreserves DEF RecoveryInitPreserves
<1>2. RecoveryInit => BridgeInv
  BY <1>1, AgreementFromStrengthening DEF AgreementFromStrengthening, BridgeInv
<1>3. RecoveryInit => AbstractInit
  BY SMT DEF AbstractInit, PublicationView, ReplayView, RecoveryInit, Init
<1>4. QED BY <1>2, <1>3

THEOREM DurableFrameOrder == Inv /\ UNCHANGED Durable => DurableOrder
BY SMT DEF Inv, TypeOK, Durable, DurableOrder

THEOREM ReceiptEffectOrder == \A i \in Inputs :
  Inv /\ RecordReceipt(i) => DurableOrder
BY SMT DEF Inv, TypeOK, RecordReceipt, DurableOrder

THEOREM ReceiptPublicationFrame == \A i \in Inputs :
  Inv /\ RecordReceipt(i) =>
    UNCHANGED <<PublicationView,p.intent,p.native,p.manifest,p.evidence>>
BY SMT DEF Inv, TypeOK, RecordReceipt, PublicationView

THEOREM RetainIntentEffectOrder == \A w \in Writers :
  Inv /\ RetainIntent(w) => DurableOrder
BY SMT, InputShape DEF InputShape, Inv, TypeOK, RetainIntent, DurableOrder

THEOREM NativeCommitEffectOrder == \A w \in Writers :
  Inv /\ NativeCommit(w) => DurableOrder
BY SMT DEF Inv, TypeOK, NativeCommit, DurableOrder

THEOREM RetainManifestEffectOrder == \A w \in Writers :
  Inv /\ RetainManifest(w) => DurableOrder
BY SMT DEF Inv, TypeOK, RetainManifest, DurableOrder

THEOREM PublicationControlFrames == Inv /\
  (\/ \E w \in Writers, i \in Inputs : Start(w,i)
   \/ \E w \in Writers : Admit(w) \/ FinalRevalidate(w) \/ Finish(w)
   \/ \E i \in Inputs, a \in SUBSET FullChecks : Observe(i,a)
   \/ Stutter) => UNCHANGED Durable
BY SMT DEF Inv, TypeOK, Start, Admit, FinalRevalidate, Finish, Observe, Stutter, Durable

THEOREM PreparationEffectOrder == \A w \in Writers :
  Inv /\ (RetainIntent(w) \/ NativeCommit(w) \/ RetainManifest(w) \/ FinalRevalidate(w))
    => DurableOrder
BY RetainIntentEffectOrder, NativeCommitEffectOrder, RetainManifestEffectOrder,
  PublicationControlFrames, DurableFrameOrder
  DEF RetainIntentEffectOrder, NativeCommitEffectOrder, RetainManifestEffectOrder,
    PublicationControlFrames, DurableFrameOrder

THEOREM CASEffectOrder == \A w \in Writers : Inv /\ DoCAS(w) => DurableOrder
<1>1. ASSUME NEW w \in Writers, Inv, DoCAS(w)
      PROVE DurableOrder
  <2>1. /\ p'.head = p.request[w]
        /\ p.head = Input(p.request[w]).predecessor
        /\ p'.published = p.published \cup {p.request[w]}
        /\ p.request[w] \notin p.published
        /\ Prepared(p.request[w]) /\ Valid(p.request[w])
    BY <1>1, CASLinearization DEF CASLinearization
  <2>2. p'.head # p.head
    BY SMT, <1>1, <2>1 DEF Inv, PublicationChain
  <2>3. /\ p.request[w] \in Inputs
        /\ p'.evidence[p.request[w]] = p.auth[p.request[w]]
        /\ UNCHANGED <<p.native,p.manifest,p.objects,p.receipt,p.intent,p.legacy>>
    BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore
  <2>4. QED BY SMT, <2>1, <2>2, <2>3 DEF DurableOrder
<1>2. QED BY <1>1

THEOREM PublicationEffectOrder == RecoveryInv /\ PublicationStep => DurableOrder
BY PublicationControlFrames, FailureFrames, DurableFrameOrder,
  PreparationEffectOrder, CASEffectOrder, ReceiptEffectOrder
  DEF PublicationControlFrames, FailureFrames, DurableFrameOrder,
    PreparationEffectOrder, CASEffectOrder, ReceiptEffectOrder,
    RecoveryInv, PublicationStep, Next, NonObservationNext

THEOREM RecoverStartEffectOrder == \A w \in Writers, i \in ReplayInputs :
  RecoveryInv /\ RecoverStart(w,i) => DurableOrder
BY SMT, DurableFrameOrder DEF DurableFrameOrder, RecoveryInv, RecoverStart, Durable
THEOREM MisleadHintEffectOrder == \A o \in Ops, i \in Releases, h \in HintPhases :
  RecoveryInv /\ MisleadHint(o,i,h) => DurableOrder
BY SMT, DurableFrameOrder DEF DurableFrameOrder, RecoveryInv, MisleadHint, Durable
THEOREM DetectCorruptionEffectOrder == \A i \in Releases :
  RecoveryInv /\ DetectCorruption(i) => DurableOrder
BY SMT, DurableFrameOrder DEF DurableFrameOrder, RecoveryInv, DetectCorruption, Durable
THEOREM RefuseCorruptionEffectOrder == \A w \in Writers :
  RecoveryInv /\ RefuseCorruption(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv
THEOREM ReplayConflictEffectOrder == \A w \in Writers :
  RecoveryInv /\ ReplayConflict(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv
THEOREM RefuseAuthorityEffectOrder == \A w \in Writers :
  RecoveryInv /\ RefuseAuthority(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv
THEOREM PredecessorConflictEffectOrder == \A w \in Writers :
  RecoveryInv /\ PredecessorConflict(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv

THEOREM ResumeDurableFrame == \A w \in Writers :
  RecoveryInv /\ ResumeExact(w) => UNCHANGED Durable
BY SMT, BridgePublicationTypes
  DEF BridgePublicationTypes, TypeOK, ResumeExact, Durable
THEOREM ResumeExactEffectOrder == \A w \in Writers :
  RecoveryInv /\ ResumeExact(w) => DurableOrder
BY ResumeDurableFrame, DurableFrameOrder
  DEF ResumeDurableFrame, DurableFrameOrder, RecoveryInv

THEOREM ModernReceiptStep == \A w \in Writers :
  RepairModernCompletion(w) => r.target[w] \in Inputs /\ RecordReceipt(r.target[w])
BY SMT, InputDomainFacts
  DEF InputDomainFacts, RepairModernCompletion, RecordReceipt
THEOREM RepairModernCompletionEffectOrder == \A w \in Writers :
  RecoveryInv /\ RepairModernCompletion(w) => DurableOrder
BY ModernReceiptStep, ReceiptEffectOrder
  DEF ModernReceiptStep, ReceiptEffectOrder, RecoveryInv
THEOREM RepairLegacyCompletionEffectOrder == \A w \in Writers :
  RecoveryInv /\ RepairLegacyCompletion(w) => DurableOrder
BY SMT, DurableFrameOrder
  DEF DurableFrameOrder, RecoveryInv, RepairLegacyCompletion, Durable
THEOREM RefusePendingLegacyEffectOrder == \A w \in Writers :
  RecoveryInv /\ RefusePendingLegacy(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv
THEOREM RepairCompletionEffectOrder == \A w \in Writers :
  RecoveryInv /\ RepairCompletion(w) => DurableOrder
BY RepairModernCompletionEffectOrder, RepairLegacyCompletionEffectOrder
  DEF RepairModernCompletionEffectOrder, RepairLegacyCompletionEffectOrder,
    RepairCompletion
THEOREM ObserveCompletionEffectOrder == \A w \in Writers :
  RecoveryInv /\ ObserveCompletion(w) => DurableOrder
BY SMT, DurableFrameOrder
  DEF DurableFrameOrder, RecoveryInv, ObserveCompletion, Durable
THEOREM RecoveryIOFailureEffectOrder == \A w \in Writers :
  RecoveryInv /\ RecoveryIOFailure(w) => DurableOrder
BY RefusalFrames, DurableFrameOrder
  DEF RefusalFrames, DurableFrameOrder, RecoveryInv
THEOREM RecoveryCrashEffectOrder == \A w \in Writers :
  RecoveryInv /\ RecoveryCrash(w) => DurableOrder
BY FailureFrames, DurableFrameOrder
  DEF FailureFrames, DurableFrameOrder, RecoveryInv, RecoveryCrash
THEOREM RecoveryRestartEffectOrder == \A w \in Writers :
  RecoveryInv /\ RecoveryRestart(w) => DurableOrder
BY FailureFrames, DurableFrameOrder
  DEF FailureFrames, DurableFrameOrder, RecoveryInv, RecoveryRestart

THEOREM StutterEffectOrder == RecoveryStutter => DurableOrder
BY SMT DEF RecoveryStutter, recoveryVars, DurableOrder
THEOREM EveryRecoveryEffectOrdered == RecoveryInv /\ RecoveryNext => DurableOrder
BY PublicationEffectOrder, RecoverStartEffectOrder, MisleadHintEffectOrder,
  DetectCorruptionEffectOrder, RefuseCorruptionEffectOrder, ReplayConflictEffectOrder,
  RefuseAuthorityEffectOrder, PredecessorConflictEffectOrder, ResumeExactEffectOrder,
  RepairCompletionEffectOrder, ObserveCompletionEffectOrder, RecoveryIOFailureEffectOrder,
  RecoveryCrashEffectOrder, RecoveryRestartEffectOrder, RefusePendingLegacyEffectOrder,
  StutterEffectOrder
  DEF PublicationEffectOrder, RecoverStartEffectOrder, MisleadHintEffectOrder,
    DetectCorruptionEffectOrder, RefuseCorruptionEffectOrder, ReplayConflictEffectOrder,
    RefuseAuthorityEffectOrder, PredecessorConflictEffectOrder, ResumeExactEffectOrder,
    RepairCompletionEffectOrder, ObserveCompletionEffectOrder, RecoveryIOFailureEffectOrder,
    RecoveryCrashEffectOrder, RecoveryRestartEffectOrder, RefusePendingLegacyEffectOrder,
    StutterEffectOrder, RecoveryNext, LocalRecoveryNext
THEOREM BeforeCASExactResume == \A w \in Writers : RecoveryInv /\ ResumeExact(w) =>
  /\ UNCHANGED Durable
  /\ Input(p'.request[w]) = Input(r.target[w])
  /\ p'.head = Input(r.target[w]).predecessor
  /\ r.target[w] \notin p.published /\ r.target[w] \notin p.receipt
<1>1. \A w \in Writers : RecoveryInv /\ ResumeExact(w) =>
  /\ Input(p'.request[w]) = Input(r.target[w])
  /\ p'.head = Input(r.target[w]).predecessor
  <2>1. \A w \in Writers : RecoveryInv /\ ResumeExact(w) =>
    p'.request[w] = r.target[w] /\ p'.head = BaseOf(r.target[w])
    BY SMT, BridgePublicationTypes
      DEF BridgePublicationTypes, TypeOK, ResumeExact
  <2>2. QED BY SMT, <2>1 DEF Input
<1>2. \A w \in Writers : RecoveryInv /\ ResumeExact(w) =>
  r.target[w] \notin p.published /\ r.target[w] \notin p.receipt
  BY SMT DEF RecoveryInv, Inv, ReceiptSound, ResumeExact
<1>3. QED BY <1>1, <1>2, ResumeDurableFrame DEF ResumeDurableFrame
THEOREM ModernRecoverySelection == \A w \in Writers :
  r.target[w] \in Inputs /\ RepairCompletion(w) => RepairModernCompletion(w)
BY SMT, ParameterBounds
  DEF RepairCompletion, RepairLegacyCompletion, Inspecting, Inputs, LegacyId, Bounds
THEOREM AfterCASReceiptSimulation == \A w \in Writers :
  RecoveryInv /\ r.target[w] \in Inputs /\ RepairCompletion(w) =>
    /\ RecordReceipt(r.target[w])
    /\ UNCHANGED <<PublicationView,p.intent,p.native,p.manifest,p.evidence>>
    /\ Input(r'.checked[w]) = Input(p.intent[OpOf(r.target[w])])
<1>1. \A w \in Writers :
  RecoveryInv /\ r.target[w] \in Inputs /\ RepairCompletion(w) =>
    RecordReceipt(r.target[w])
  BY ModernRecoverySelection, ModernReceiptStep
    DEF ModernRecoverySelection, ModernReceiptStep
<1>2. \A w \in Writers :
  RecoveryInv /\ r.target[w] \in Inputs /\ RepairCompletion(w) =>
    UNCHANGED <<PublicationView,p.intent,p.native,p.manifest,p.evidence>>
  BY <1>1, ReceiptPublicationFrame DEF ReceiptPublicationFrame, RecoveryInv
<1>3. \A w \in Writers :
  RecoveryInv /\ r.target[w] \in Inputs /\ RepairCompletion(w) =>
    Input(r'.checked[w]) = Input(p.intent[OpOf(r.target[w])])
  BY SMT, ModernRecoverySelection, BridgePublicationTypes
    DEF ModernRecoverySelection, BridgePublicationTypes, RecoveryTypeOK,
      RepairModernCompletion, BoundIntent
<1>4. QED BY <1>1, <1>2, <1>3
THEOREM LegacyReceiptSimulation == \A w \in Writers :
  RecoveryInv /\ RepairLegacyCompletion(w) =>
    /\ UNCHANGED Durable
    /\ r'.result[w] = "already_applied"
    /\ PublicationView.legacyInput = ReplayView.legacyInput
    /\ ReplayView.legacyInput = LegacyInput
    /\ p.legacy.input.domain = "kctx-operation-v2"
<1>1. \A w \in Writers : RecoveryInv /\ RepairLegacyCompletion(w) =>
  /\ UNCHANGED Durable
  /\ r'.result[w] = "already_applied"
  BY SMT, BridgePublicationTypes
    DEF BridgePublicationTypes, RecoveryTypeOK, RepairLegacyCompletion, Durable
<1>2. \A w \in Writers : RepairLegacyCompletion(w) =>
  /\ PublicationView.legacyInput = ReplayView.legacyInput
  /\ ReplayView.legacyInput = LegacyInput
  /\ p.legacy.input.domain = "kctx-operation-v2"
  BY SMT, HistoricalAgreement DEF HistoricalAgreement, RepairLegacyCompletion
<1>3. QED BY <1>1, <1>2
THEOREM PostCASCrashStillApplied == \A w \in Writers, i \in Inputs :
  RecoveryInv /\ i \in p.published /\ (Crash(w) \/ RecoveryCrash(w)) =>
    /\ i \in p'.published /\ p'.intent[OpOf(i)] = i
    /\ p'.evidence[i] = FullChecks /\ Closure(i) \subseteq p'.objects
BY SMT, FailureFrames, PublishedInputBinding, InputDomainFacts
  DEF FailureFrames, PublishedInputBinding, InputDomainFacts, RecoveryInv,
    RecoveryCrash, Durable

(* Decompose the application-only proof at the same checked durable frames
   used above; no expansion of every publication/recovery action is needed. *)
THEOREM ReceiptVisibleFrame == \A i \in Inputs :
  Inv /\ RecordReceipt(i) => UNCHANGED Visible
BY SMT DEF Inv, TypeOK, RecordReceipt, Visible

THEOREM PublicationApplicationStep == Inv /\ Next /\ p'.published # p.published =>
  \E w \in Writers : DoCAS(w) /\ p'.head = p.request[w]
BY SMT, PublicationControlFrames, FailureFrames, PreparationFrames,
  ReceiptVisibleFrame, CASLinearization
  DEF PublicationControlFrames, FailureFrames, PreparationFrames,
    ReceiptVisibleFrame, CASLinearization, Next, NonObservationNext, Durable, Visible

THEOREM LocalRecoveryPublicationFrame ==
  RecoveryInv /\ LocalRecoveryNext => UNCHANGED p.published
BY SMT, RefusalFrames, ResumeDurableFrame, RepairDoesNotApplyAgain, FailureFrames
  DEF RefusalFrames, ResumeDurableFrame, RepairDoesNotApplyAgain, FailureFrames,
    RecoveryInv, LocalRecoveryNext, RecoverStart, MisleadHint, DetectCorruption,
    ObserveCompletion, RecoveryCrash, RecoveryRestart, RecoveryStutter,
    recoveryVars, Durable
THEOREM PublicationIsOnlyApplication == RecoveryInv /\ RecoveryNext
  /\ p'.published # p.published =>
    \E w \in Writers : DoCAS(w) /\ p'.head = p.request[w]
BY PublicationApplicationStep, LocalRecoveryPublicationFrame
  DEF PublicationApplicationStep, LocalRecoveryPublicationFrame,
    RecoveryInv, RecoveryNext, PublicationStep
THEOREM BridgeNextPreserves == BridgeInv /\ RecoveryNext => BridgeInv'
<1>1. BridgeInv /\ RecoveryNext => RecoveryInv'
  BY RecoveryNextPreserves DEF RecoveryNextPreserves, BridgeInv
<1>2. BridgeInv /\ RecoveryNext =>
  ExactAgreement' /\ SingleApplication' /\ BeforeCASInvisible'
  BY <1>1, AgreementFromStrengtheningNext DEF AgreementFromStrengtheningNext
<1>3. QED BY <1>1, <1>2 DEF BridgeInv
THEOREM BridgeStutterPreserves == BridgeInv /\ UNCHANGED recoveryVars => BridgeInv'
<1>1. BridgeInv /\ UNCHANGED recoveryVars => RecoveryInv'
  BY RecoveryStutterPreserves
    DEF RecoveryStutterPreserves, BridgeInv, RecoveryStutter
<1>2. BridgeInv /\ UNCHANGED recoveryVars =>
  ExactAgreement' /\ SingleApplication' /\ BeforeCASInvisible'
  BY <1>1, AgreementFromStrengtheningNext DEF AgreementFromStrengtheningNext
<1>3. QED BY <1>1, <1>2 DEF BridgeInv
THEOREM BridgeInvImpliesSafety == BridgeInv => BridgeSafety
BY RecoveryInvImpliesSafety DEF RecoveryInvImpliesSafety, BridgeInv, BridgeSafety
THEOREM DurableProjectionSimulation == BridgeSpec => AbstractSpec
<1>1. RecoveryInit => AbstractInit
  BY BridgeInitPreserves DEF BridgeInitPreserves
<1>2. RecoveryInv /\ [RecoveryNext]_recoveryVars => [DurableOrder]_Durable
  BY EveryRecoveryEffectOrdered DEF EveryRecoveryEffectOrdered, recoveryVars, Durable
<1>3. RecoverySpec => []RecoveryInv
  BY RecoveryInitPreserves, RecoveryNextPreserves, RecoveryStutterPreserves, PTL
    DEF RecoveryInitPreserves, RecoveryNextPreserves, RecoveryStutterPreserves,
      RecoverySpec, RecoveryStutter
<1>4. QED BY <1>1, <1>2, <1>3, PTL DEF BridgeSpec, RecoverySpec, AbstractSpec
THEOREM BridgeInductiveSafety == BridgeSpec => []BridgeSafety
<1>1. RecoveryInit => BridgeInv BY BridgeInitPreserves DEF BridgeInitPreserves
<1>2. BridgeInv /\ [RecoveryNext]_recoveryVars => BridgeInv'
  BY BridgeNextPreserves, BridgeStutterPreserves
    DEF BridgeNextPreserves, BridgeStutterPreserves
<1>3. BridgeSpec => []BridgeInv BY <1>1, <1>2, PTL DEF BridgeSpec, RecoverySpec
<1>4. QED BY <1>3, BridgeInvImpliesSafety, PTL DEF BridgeInvImpliesSafety

THEOREM FiniteBridgeSpecRefines == FiniteBridgeSpec => BridgeSpec
BY FiniteRecoverySpecRefines
  DEF FiniteRecoverySpecRefines, FiniteBridgeSpec, BridgeSpec
THEOREM FiniteBridgeProjection == FiniteBridgeSpec => AbstractSpec
BY FiniteBridgeSpecRefines, DurableProjectionSimulation
  DEF FiniteBridgeSpecRefines, DurableProjectionSimulation

THEOREM WideBridgeSpecRefines == \A profile \in 0..3 :
  WideBridgeSpec(profile) => BridgeSpec
BY WideRecoverySpecRefines DEF WideRecoverySpecRefines, WideBridgeSpec, BridgeSpec
THEOREM WideBridgeProjection == \A profile \in 0..3 :
  WideBridgeSpec(profile) => AbstractSpec
BY WideBridgeSpecRefines, DurableProjectionSimulation
  DEF WideBridgeSpecRefines, DurableProjectionSimulation

THEOREM CrossHintBridgeSpecRefines == CrossHintBridgeSpec => BridgeSpec
BY CrossHintRecoverySpecRefines
  DEF CrossHintRecoverySpecRefines, CrossHintBridgeSpec, BridgeSpec
THEOREM CrossHintBridgeProjection == CrossHintBridgeSpec => AbstractSpec
BY CrossHintBridgeSpecRefines, DurableProjectionSimulation
  DEF CrossHintBridgeSpecRefines, DurableProjectionSimulation

THEOREM WorkloadBridgeSpecRefines == \A workload \in 0..3 :
  WorkloadBridgeSpec(workload) => BridgeSpec
BY WorkloadRecoverySpecRefines
  DEF WorkloadRecoverySpecRefines, WorkloadBridgeSpec, BridgeSpec
THEOREM WorkloadBridgeProjection == \A workload \in 0..3 :
  WorkloadBridgeSpec(workload) => AbstractSpec
BY WorkloadBridgeSpecRefines, DurableProjectionSimulation
  DEF WorkloadBridgeSpecRefines, DurableProjectionSimulation

THEOREM SameInputBridgeSpecRefines == SameInputBridgeSpec => BridgeSpec
BY SMT, WorkloadBridgeSpecRefines
  DEF WorkloadBridgeSpecRefines, SameInputBridgeSpec
THEOREM SameInputBridgeProjection == SameInputBridgeSpec => AbstractSpec
BY SameInputBridgeSpecRefines, DurableProjectionSimulation
  DEF SameInputBridgeSpecRefines, DurableProjectionSimulation

THEOREM SuccessorBridgeSpecRefines == SuccessorBridgeSpec => BridgeSpec
BY SMT, WorkloadBridgeSpecRefines
  DEF WorkloadBridgeSpecRefines, SuccessorBridgeSpec
THEOREM SuccessorBridgeProjection == SuccessorBridgeSpec => AbstractSpec
BY SuccessorBridgeSpecRefines, DurableProjectionSimulation
  DEF SuccessorBridgeSpecRefines, DurableProjectionSimulation

THEOREM LegacyWorkloadBridgeSpecRefines == LegacyWorkloadBridgeSpec => BridgeSpec
BY SMT, WorkloadBridgeSpecRefines
  DEF WorkloadBridgeSpecRefines, LegacyWorkloadBridgeSpec
THEOREM LegacyWorkloadBridgeProjection == LegacyWorkloadBridgeSpec => AbstractSpec
BY LegacyWorkloadBridgeSpecRefines, DurableProjectionSimulation
  DEF LegacyWorkloadBridgeSpecRefines, DurableProjectionSimulation

(* A typed hint replacement describes every state with the same observable
   recovery core, not merely one hand-picked empty journal. The explicit proof
   projections change r.hint only and leave p and all exact identities intact.
   VIEW remains disabled pending checked noninterference/observation proofs. *)
HintMaps == [Ops -> [input : Releases, phase : HintPhases]]
EmptyHintMap == [o \in Ops |-> [input |-> 0, phase |-> "none"]]
SetHintMap(q,h) == [q EXCEPT !.hint = h]
RecoveryCore(q) == <<q.target,q.pc,q.result,q.checked,q.corrupt>>
RecoveryRecordSpace == [
  target : [Writers -> Releases \cup {LegacyId}],
  pc : [Writers -> RecoveryPhases], result : [Writers -> RecoveryOutcomes],
  hint : HintMaps, corrupt : SUBSET Releases,
  checked : [Writers -> Releases \cup {LegacyId}]]
(* Explicit capture-free projections of the M02 recovery definitions.
   Publication-only dependencies are shared, not re-instantiated. Each operator
   below replaces r by SetHintMap(r,hintMap), including under prime/UNCHANGED;
   the hintMap parameter is fixed across both endpoints of an action. This
   avoids TLAPM's loss of the bound parameter in parametric INSTANCE expansion.
   No kernel action is replaced: the replay theorems relate the original
   actions to these proof-only projections, preserving guards and full frames. *)
RehintrecoveryVars(hintMap) == <<p,SetHintMap(r,hintMap)>>

RehintRecoveryTypeOK(hintMap) == SetHintMap(r,hintMap) \in [target : [Writers -> Releases \cup {LegacyId}],
  pc : [Writers -> RecoveryPhases], result : [Writers -> RecoveryOutcomes],
  hint : [Ops -> [input : Releases, phase : HintPhases]],
  corrupt : SUBSET Releases, checked : [Writers -> Releases \cup {LegacyId}]]

RehintRecoveryEvidence(hintMap) ==
  /\ (\A w \in Writers : SetHintMap(r,hintMap).result[w] = "complete" =>
       /\ SetHintMap(r,hintMap).target[w] \in Inputs
       /\ SetHintMap(r,hintMap).checked[w] = SetHintMap(r,hintMap).target[w]
       /\ SetHintMap(r,hintMap).target[w] \in p.receipt
       /\ BoundIntent(SetHintMap(r,hintMap).target[w]))
  /\ (\A w \in Writers : SetHintMap(r,hintMap).result[w] = "already_applied" =>
       /\ SetHintMap(r,hintMap).target[w] = LegacyId
       /\ SetHintMap(r,hintMap).checked[w] = LegacyId
       /\ HistoricalAuthentic)
  /\ (\A w \in Writers : SetHintMap(r,hintMap).pc[w] = "resumed" =>
       /\ SetHintMap(r,hintMap).target[w] \in Inputs
       /\ SchemaOf(SetHintMap(r,hintMap).target[w]) = 3)

RehintRecoveryInv(hintMap) == Inv /\ RehintRecoveryTypeOK(hintMap) /\ RehintRecoveryEvidence(hintMap)

RehintRecoverySafety(hintMap) ==
  /\ Safety
  /\ (\A w \in Writers : SetHintMap(r,hintMap).result[w] = "complete" =>
       /\ SetHintMap(r,hintMap).target[w] \in p.published
       /\ Input(SetHintMap(r,hintMap).checked[w]) = Input(SetHintMap(r,hintMap).target[w])
       /\ p.intent[OpOf(SetHintMap(r,hintMap).target[w])] = SetHintMap(r,hintMap).target[w])
  /\ (\A w \in Writers : SetHintMap(r,hintMap).result[w] = "already_applied" =>
       /\ SetHintMap(r,hintMap).target[w] = p.legacy.input.identity
       /\ p.legacy.receipt.input = LegacyInput
       /\ p.legacy.input.schema = 2
       /\ p.legacy.input.domain = "kctx-operation-v2"
       /\ HistoricalAuthentic)
  /\ (\A w \in Writers : SetHintMap(r,hintMap).target[w] = InputCount => SetHintMap(r,hintMap).pc[w] # "resumed")

RehintRecoveryInit(hintMap) == Init /\ SetHintMap(r,hintMap) = [target |-> [w \in Writers |-> 0],
  pc |-> [w \in Writers |-> "idle"], result |-> [w \in Writers |-> "none"],
  hint |-> [o \in Ops |-> [input |-> 0, phase |-> "none"]],
  corrupt |-> {}, checked |-> [w \in Writers |-> 0]]

RehintInspecting(hintMap,w,i) ==
  /\ SetHintMap(r,hintMap).pc[w] = "inspect"
  /\ SetHintMap(r,hintMap).target[w] = i
  /\ i \in ReplayInputs

RehintIntact(hintMap,i) == p.head \notin SetHintMap(r,hintMap).corrupt /\
  (IF i = LegacyId THEN 0 \notin SetHintMap(r,hintMap).corrupt ELSE i \notin SetHintMap(r,hintMap).corrupt)

RehintRecoverStart(hintMap,w,i) ==
  /\ SetHintMap(r,hintMap).pc[w] \in {"idle","done"}
  /\ p.pc[w] \in {"idle","failed","conflict","done","down"}
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.target[w] = i, !.pc[w] = "inspect",
       !.result[w] = "pending", !.checked[w] = 0]
  /\ UNCHANGED p

RehintMisleadHint(hintMap,o,i,h) ==
  /\ SetHintMap(r',hintMap) =
       [SetHintMap(r,hintMap) EXCEPT !.hint[o] = [input |-> i, phase |-> h]]
  /\ UNCHANGED p

RehintDetectCorruption(hintMap,i) ==
  /\ SetHintMap(r',hintMap) =
       [SetHintMap(r,hintMap) EXCEPT !.corrupt = @ \cup {i}]
  /\ UNCHANGED p

RehintRefuseCorruption(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ RehintInspecting(hintMap,w,i) /\ ~RehintIntact(hintMap,i)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "refused"] /\ UNCHANGED p

RehintReplayConflict(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ RehintIntact(hintMap,i) /\ p.intent[OpOf(i)] \notin {0,i}
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "conflict"] /\ UNCHANGED p

RehintRefuseAuthority(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ RehintIntact(hintMap,i) /\ ~Valid(i)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "refused"] /\ UNCHANGED p

RehintPredecessorConflict(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ RehintIntact(hintMap,i) /\ i \notin p.published
  /\ p.intent[OpOf(i)] \in {0,i} /\ p.head # BaseOf(i)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "conflict"] /\ UNCHANGED p

RehintResumeExact(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ RehintIntact(hintMap,i) /\ Valid(i)
  /\ i \notin p.published /\ p.intent[OpOf(i)] \in {0,i}
  /\ p.head = BaseOf(i)
  /\ p' = [p EXCEPT !.request[w] = i, !.outcome[w] = "pending",
       !.pc[w] = IF i \in p.manifest THEN "manifest"
                 ELSE IF i \in p.native THEN "native"
                 ELSE IF BoundIntent(i) THEN "intent" ELSE "planned"]
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "resumed"]

RehintRepairModernCompletion(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ RehintIntact(hintMap,i) /\ Valid(i) /\ BoundIntent(i)
  /\ i \in p.published
  /\ p' = [p EXCEPT !.receipt = @ \cup {i}]
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]

RehintRepairLegacyCompletion(hintMap,w) ==
  /\ RehintInspecting(hintMap,w,LegacyId)
  /\ RehintIntact(hintMap,LegacyId)
  /\ HistoricalAuthentic
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "already_applied",
       !.checked[w] = LegacyId]
  /\ UNCHANGED p

RehintRepairCompletion(hintMap,w) == RehintRepairModernCompletion(hintMap,w) \/ RehintRepairLegacyCompletion(hintMap,w)

RehintRefusePendingLegacy(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ i \in Inputs /\ RehintInspecting(hintMap,w,i) /\ SchemaOf(i) = 2
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "refused"]
  /\ UNCHANGED p

RehintObserveCompletion(hintMap,w) == LET i == SetHintMap(r,hintMap).target[w] IN
  /\ SetHintMap(r,hintMap).pc[w] = "resumed" /\ i \in Inputs /\ RehintIntact(hintMap,i) /\ Valid(i)
  /\ i \in p.receipt /\ BoundIntent(i)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]
  /\ UNCHANGED p

RehintRecoveryIOFailure(hintMap,w) ==
  /\ SetHintMap(r,hintMap).pc[w] \in {"inspect","resumed"}
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "done", !.result[w] = "unknown"] /\ UNCHANGED p

RehintRecoveryCrash(hintMap,w) ==
  /\ Crash(w)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "down", !.result[w] = "unknown"]

RehintRecoveryRestart(hintMap,w) ==
  /\ Restart(w)
  /\ SetHintMap(r',hintMap) = [SetHintMap(r,hintMap) EXCEPT !.pc[w] = "idle", !.target[w] = 0,
       !.result[w] = "none", !.checked[w] = 0]

RehintPublicationStep(hintMap) ==
  /\ Next
  /\ p.head \notin SetHintMap(r,hintMap).corrupt
  /\ p'.head \notin SetHintMap(r,hintMap).corrupt
  /\ UNCHANGED SetHintMap(r,hintMap)

RehintRecoveryStutter(hintMap) == UNCHANGED RehintrecoveryVars(hintMap)

RehintLocalRecoveryNext(hintMap) ==
  \/ (\E w \in Writers, i \in ReplayInputs : RehintRecoverStart(hintMap,w,i))
  \/ (\E o \in Ops, i \in Releases, h \in HintPhases : RehintMisleadHint(hintMap,o,i,h))
  \/ (\E i \in Releases : RehintDetectCorruption(hintMap,i))
  \/ (\E w \in Writers :
       \/ RehintRefuseCorruption(hintMap,w) \/ RehintReplayConflict(hintMap,w) \/ RehintRefuseAuthority(hintMap,w)
       \/ RehintPredecessorConflict(hintMap,w) \/ RehintResumeExact(hintMap,w) \/ RehintRepairCompletion(hintMap,w)
       \/ RehintObserveCompletion(hintMap,w) \/ RehintRecoveryIOFailure(hintMap,w) \/ RehintRecoveryCrash(hintMap,w)
       \/ RehintRecoveryRestart(hintMap,w) \/ RehintRefusePendingLegacy(hintMap,w))
  \/ RehintRecoveryStutter(hintMap)

RehintRecoveryNext(hintMap) == RehintPublicationStep(hintMap) \/ RehintLocalRecoveryNext(hintMap)

RehintNonHintRecoveryNext(hintMap) ==
  \/ (\E w \in Writers, i \in ReplayInputs : RehintRecoverStart(hintMap,w,i))
  \/ (\E i \in Releases : RehintDetectCorruption(hintMap,i))
  \/ (\E w \in Writers :
       \/ RehintRefuseCorruption(hintMap,w) \/ RehintReplayConflict(hintMap,w) \/ RehintRefuseAuthority(hintMap,w)
       \/ RehintPredecessorConflict(hintMap,w) \/ RehintResumeExact(hintMap,w) \/ RehintRepairCompletion(hintMap,w)
       \/ RehintObserveCompletion(hintMap,w) \/ RehintRecoveryIOFailure(hintMap,w) \/ RehintRecoveryCrash(hintMap,w)
       \/ RehintRecoveryRestart(hintMap,w) \/ RehintRefusePendingLegacy(hintMap,w))
  \/ RehintRecoveryStutter(hintMap)

RehintFiniteHintStep(hintMap) == \E i \in {0,1,2,InputCount}, h \in HintPhases : RehintMisleadHint(hintMap,1,i,h)

RehintCrossHintStep(hintMap) ==
  \/ (\E o \in {1,2}, h \in HintPhases :
       RehintMisleadHint(hintMap,o,(IF o = 1 THEN 3 ELSE 1),h))
  \/ (\E o \in {1,2} : RehintMisleadHint(hintMap,o,0,"none"))

RehintRecoveryObservation(hintMap) == <<p,SetHintMap(r,hintMap).target,SetHintMap(r,hintMap).pc,SetHintMap(r,hintMap).result,SetHintMap(r,hintMap).checked,SetHintMap(r,hintMap).corrupt>>

RehintRecoveryServiceNext(hintMap) ==
  \/ (\E i \in Releases : RehintDetectCorruption(hintMap,i))
  \/ (\E w \in Writers :
       \/ RehintRefuseCorruption(hintMap,w) \/ RehintReplayConflict(hintMap,w) \/ RehintRefuseAuthority(hintMap,w)
       \/ RehintPredecessorConflict(hintMap,w) \/ RehintResumeExact(hintMap,w) \/ RehintRepairCompletion(hintMap,w)
       \/ RehintObserveCompletion(hintMap,w) \/ RehintRecoveryIOFailure(hintMap,w) \/ RehintRecoveryCrash(hintMap,w)
       \/ RehintRecoveryRestart(hintMap,w) \/ RehintRefusePendingLegacy(hintMap,w))
  \/ RehintRecoveryStutter(hintMap)

RehintWorkloadLocalNext(hintMap,workload) == RehintRecoveryServiceNext(hintMap) \/
  (\E w \in Writers : RehintRecoverStart(hintMap,w,WorkloadTarget(w,workload)))

RehintFinitePublicationStep(hintMap) ==
  /\ FiniteNext
  /\ p.head \notin SetHintMap(r,hintMap).corrupt
  /\ p'.head \notin SetHintMap(r,hintMap).corrupt
  /\ UNCHANGED SetHintMap(r,hintMap)

RehintWorkloadRecoveryNext(hintMap,workload) == RehintFinitePublicationStep(hintMap) \/ RehintWorkloadLocalNext(hintMap,workload) \/ RehintFiniteHintStep(hintMap)

RehintFiniteRecoveryNext(hintMap) == RehintWorkloadRecoveryNext(hintMap,0)

RehintWidePublicationStep(hintMap,profile) ==
  /\ WideNext(profile)
  /\ p.head \notin SetHintMap(r,hintMap).corrupt
  /\ p'.head \notin SetHintMap(r,hintMap).corrupt
  /\ UNCHANGED SetHintMap(r,hintMap)

RehintWideRecoveryNext(hintMap,profile) == RehintWidePublicationStep(hintMap,profile) \/ RehintWorkloadLocalNext(hintMap,0) \/ RehintFiniteHintStep(hintMap)

RehintCrossHintRecoveryNext(hintMap) == RehintWidePublicationStep(hintMap,0) \/ RehintWorkloadLocalNext(hintMap,2) \/ RehintCrossHintStep(hintMap)

(* Positive hint-erasing VIEW: state half of the certificate.
   A recovery record has exactly the six fields below. Erasing hints does not
   identify different publication records, corruption observations, targets,
   checked identities, phases, or outcomes. On this typed record space, equality
   of the five-field core is precisely equality up to hint replacement.

   These preservation results assume only recovery record typing, never the
   invariant or safety property whose truth value is being preserved. Reachable
   typing is connected to the existing inductive proof at the end of this
   fragment. In particular, the certificate does not make failed safety checks
   disappear by restricting its state-equivalence premise to safe states. *)

THEOREM HintRecordType == RecoveryTypeOK <=> r \in RecoveryRecordSpace
BY DEF RecoveryTypeOK, RecoveryRecordSpace, HintMaps

THEOREM HintEmptyMapTyped == EmptyHintMap \in HintMaps
BY SMT, ParameterBounds
  DEF ParameterBounds, Bounds, EmptyHintMap, HintMaps, Releases, HintPhases

THEOREM HintRetargetTyped ==
  \A q \in RecoveryRecordSpace, h \in HintMaps :
    SetHintMap(q,h) \in RecoveryRecordSpace
BY SMT DEF SetHintMap, RecoveryRecordSpace

THEOREM HintRetargetFields ==
  \A q \in RecoveryRecordSpace, h \in HintMaps :
    /\ SetHintMap(q,h).target = q.target
    /\ SetHintMap(q,h).pc = q.pc
    /\ SetHintMap(q,h).result = q.result
    /\ SetHintMap(q,h).checked = q.checked
    /\ SetHintMap(q,h).corrupt = q.corrupt
    /\ SetHintMap(q,h).hint = h
BY SMT DEF SetHintMap, RecoveryRecordSpace

THEOREM HintRetargetCore ==
  \A q \in RecoveryRecordSpace, h \in HintMaps :
    RecoveryCore(SetHintMap(q,h)) = RecoveryCore(q)
BY SMT, HintRetargetFields DEF HintRetargetFields, RecoveryCore

THEOREM HintRetargetIdentity ==
  \A q \in RecoveryRecordSpace : SetHintMap(q,q.hint) = q
BY SMT DEF SetHintMap, RecoveryRecordSpace

THEOREM HintCoreCharacterizes ==
  \A q,t \in RecoveryRecordSpace :
    RecoveryCore(q) = RecoveryCore(t) => SetHintMap(q,t.hint) = t
<1>1. ASSUME NEW q \in RecoveryRecordSpace,
             NEW t \in RecoveryRecordSpace,
             RecoveryCore(q) = RecoveryCore(t)
      PROVE SetHintMap(q,t.hint) = t
  <2>1. /\ DOMAIN q = {"target","pc","result","hint","corrupt","checked"}
        /\ DOMAIN t = {"target","pc","result","hint","corrupt","checked"}
    BY SMT, <1>1 DEF RecoveryRecordSpace
  <2>2. /\ q.target = t.target /\ q.pc = t.pc
        /\ q.result = t.result /\ q.checked = t.checked
        /\ q.corrupt = t.corrupt
    BY SMT, <1>1 DEF RecoveryCore
  <2>3. QED BY SMT, <1>1, <2>1, <2>2 DEF SetHintMap, RecoveryRecordSpace
<1>2. QED BY <1>1

HintStateObservation(s,q) ==
  <<s,q.target,q.pc,q.result,q.checked,q.corrupt>>

THEOREM HintViewCharacterizes ==
  \A s,u : \A q,t \in RecoveryRecordSpace :
    (HintStateObservation(s,q) = HintStateObservation(u,t))
      <=> (s = u /\ RecoveryCore(q) = RecoveryCore(t))
BY SMT DEF HintStateObservation, RecoveryCore

THEOREM HintViewRepresentatives == RecoveryTypeOK =>
  \A q \in RecoveryRecordSpace :
    /\ (HintStateObservation(p,q) = HintErasedView
          <=> RecoveryCore(q) = RecoveryCore(r))
    /\ (HintStateObservation(p,q) = HintErasedView
          => SetHintMap(r,q.hint) = q)
BY SMT, HintRecordType, HintViewCharacterizes, HintCoreCharacterizes
  DEF HintRecordType, HintViewCharacterizes, HintCoreCharacterizes,
  HintStateObservation, HintErasedView, RecoveryObservation

THEOREM HintObservedState ==
  \A h \in HintMaps : RecoveryTypeOK =>
    RehintRecoveryObservation(h) = RecoveryObservation
BY SMT, HintRecordType
  DEF HintRecordType, RehintRecoveryObservation, RecoveryObservation, SetHintMap,
  RecoveryRecordSpace

(* This is a direct action-level proof about the next record, not an appeal to
   a primed theorem name. The bound hint map is constant across the action. *)
THEOREM HintObservedNextState ==
  \A h \in HintMaps : RecoveryTypeOK' =>
    (RehintRecoveryObservation(h))' = RecoveryObservation'
BY SMT DEF RehintRecoveryObservation, RecoveryObservation, RecoveryTypeOK, SetHintMap

(* The projections substitute r only. Isolate p-only predicates, especially the
   immutable historical record, rather than unfolding historical bytes in each
   recovery or bridge property proof. These equalities require no truth or
   typing assumption about those publication predicates. *)
THEOREM HintPublicationStateUnchanged == \A h \in HintMaps :
  /\ (Inv <=> Inv)
  /\ (Safety <=> Safety)
  /\ (Init <=> Init)
OBVIOUS

THEOREM HintHistoricalEquivalent == \A h \in HintMaps :
  HistoricalAuthentic <=> HistoricalAuthentic
OBVIOUS

THEOREM HintRecoveryTypePreserved ==
  \A h \in HintMaps : RecoveryTypeOK => RehintRecoveryTypeOK(h)
BY SMT, HintRecordType, HintRetargetTyped
  DEF HintRecordType, HintRetargetTyped, RehintRecoveryTypeOK,
  RecoveryRecordSpace, HintMaps, SetHintMap, Writers, Releases, LegacyId, Ops,
  RecoveryPhases, RecoveryOutcomes, HintPhases

THEOREM HintRecoveryEvidenceEquivalent ==
  \A h \in HintMaps : RecoveryTypeOK =>
    (RehintRecoveryEvidence(h) <=> RecoveryEvidence)
BY SMT, HintRecordType, HintRetargetFields, HintHistoricalEquivalent
  DEF HintRecordType, HintRetargetFields, HintHistoricalEquivalent,
  RehintRecoveryEvidence, RecoveryEvidence, SetHintMap, RecoveryRecordSpace,
  Writers, Inputs, LegacyId, BoundIntent, OpOf, SchemaOf

THEOREM HintRecoveryInvEquivalent ==
  \A h \in HintMaps : RecoveryTypeOK =>
    (RehintRecoveryInv(h) <=> RecoveryInv)
BY SMT, HintPublicationStateUnchanged, HintRecoveryTypePreserved,
  HintRecoveryEvidenceEquivalent
  DEF HintPublicationStateUnchanged, HintRecoveryTypePreserved,
  HintRecoveryEvidenceEquivalent, RehintRecoveryInv, RecoveryInv

THEOREM HintRecoverySafetyEquivalent ==
  \A h \in HintMaps : RecoveryTypeOK =>
    (RehintRecoverySafety(h) <=> RecoverySafety)
<1>1. ASSUME NEW h \in HintMaps, RecoveryTypeOK
      PROVE RehintRecoverySafety(h) <=> RecoverySafety
  <2> USE RecoveryTypeOK
  <2>1. r \in RecoveryRecordSpace
    BY HintRecordType DEF HintRecordType
  <2>2. SetHintMap(r,h).target=r.target /\ SetHintMap(r,h).pc=r.pc /\
    SetHintMap(r,h).result=r.result /\ SetHintMap(r,h).checked=r.checked
    BY SMT, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. QED BY SMT, <2>2 DEF RehintRecoverySafety, RecoverySafety
<1>2. QED BY <1>1

(* No self-instance of this bridge is needed. Its extra checked predicates use
   only p: PublicationView and ReplayView, and therefore ExactAgreement,
   SingleApplication and BeforeCASInvisible, contain no occurrence of r.
   These explicit retargeted predicates are exactly BridgeInv/BridgeSafety
   with the recovery conjunct retargeted and the shared p left unchanged. *)
HintBridgeInv(h) == RehintRecoveryInv(h)
  /\ ExactAgreement /\ SingleApplication /\ BeforeCASInvisible
HintBridgeSafety(h) == RehintRecoverySafety(h)
  /\ SingleApplication /\ ExactAgreement
HintCheckedProperties ==
  <<RecoveryInv,RecoverySafety,HistoricalAuthentic,BridgeInv,BridgeSafety>>
HintRetargetedChecks(h) ==
  <<RehintRecoveryInv(h),RehintRecoverySafety(h),
    HistoricalAuthentic,HintBridgeInv(h),HintBridgeSafety(h)>>

THEOREM HintBridgeInvEquivalent ==
  \A h \in HintMaps : RecoveryTypeOK => (HintBridgeInv(h) <=> BridgeInv)
BY SMT, HintRecoveryInvEquivalent
  DEF HintRecoveryInvEquivalent, HintBridgeInv, BridgeInv

THEOREM HintBridgeSafetyEquivalent ==
  \A h \in HintMaps : RecoveryTypeOK => (HintBridgeSafety(h) <=> BridgeSafety)
BY SMT, HintRecoverySafetyEquivalent
  DEF HintRecoverySafetyEquivalent, HintBridgeSafety, BridgeSafety

THEOREM HintCheckedPropertiesPreserved ==
  \A h \in HintMaps : RecoveryTypeOK =>
    HintRetargetedChecks(h) = HintCheckedProperties
<1>1. ASSUME NEW h \in HintMaps, RecoveryTypeOK
      PROVE HintRetargetedChecks(h)=HintCheckedProperties
  <2> USE RecoveryTypeOK
  <2>1. RehintRecoveryInv(h)=RecoveryInv
    BY SMT, HintRecoveryInvEquivalent
      DEF HintRecoveryInvEquivalent, RehintRecoveryInv, RecoveryInv
  <2>2. RehintRecoverySafety(h)=RecoverySafety
    BY SMT, HintRecoverySafetyEquivalent
      DEF HintRecoverySafetyEquivalent, RehintRecoverySafety, RecoverySafety
  <2>3. HintBridgeInv(h)=BridgeInv
    BY SMT, HintBridgeInvEquivalent
      DEF HintBridgeInvEquivalent, HintBridgeInv, BridgeInv
  <2>4. HintBridgeSafety(h)=BridgeSafety
    BY SMT, HintBridgeSafetyEquivalent
      DEF HintBridgeSafetyEquivalent, HintBridgeSafety, BridgeSafety
  <2>5. QED BY SMT, <2>1, <2>2, <2>3, <2>4
    DEF HintRetargetedChecks, HintCheckedProperties
<1>2. QED BY <1>1

(* Init chooses one canonical hint map; arbitrary retargeting need not remain
   initial. Its erased initial class nevertheless has an exact reconstruction:
   resetting the hint of ANY typed record with the initial core gives the
   original initial record, including all six fields. This is stronger than
   merely saying that initial observations agree. *)
THEOREM HintInitialFacts == RecoveryInit =>
  /\ RecoveryTypeOK
  /\ r \in RecoveryRecordSpace
  /\ r.hint = EmptyHintMap
BY SMT, RecoveryInitPreserves, HintRecordType
  DEF RecoveryInitPreserves, HintRecordType, RecoveryInv, RecoveryInit,
  EmptyHintMap

THEOREM HintCanonicalInitPreserves ==
  RecoveryInit => RehintRecoveryInit(EmptyHintMap)
<1>1. RecoveryInit => SetHintMap(r,EmptyHintMap) = r
  BY SMT, HintInitialFacts, HintRetargetIdentity
    DEF HintInitialFacts, HintRetargetIdentity
<1>2. QED BY SMT, <1>1, HintEmptyMapTyped, HintPublicationStateUnchanged
  DEF HintEmptyMapTyped, HintPublicationStateUnchanged, RecoveryInit,
  RehintRecoveryInit, SetHintMap, Writers, Ops

THEOREM HintInitialRecordReconstruction == RecoveryInit =>
  \A q \in RecoveryRecordSpace : RecoveryCore(q) = RecoveryCore(r) =>
    SetHintMap(q,EmptyHintMap) = r
BY SMT, HintInitialFacts, HintCoreCharacterizes
  DEF HintInitialFacts, HintCoreCharacterizes

(* The state-shape premise above is already an inductive consequence of the
   unrestricted positive kernel. Restricted positive profiles inherit that
   premise through their existing refinement proofs, without phase, writer,
   publication-input, or corruption restrictions added by this certificate. *)
THEOREM HintReachableTyping == RecoverySpec =>
  /\ []RecoveryTypeOK
  /\ [](r \in RecoveryRecordSpace)
<1>1. RecoveryInit => RecoveryInv
  BY RecoveryInitPreserves DEF RecoveryInitPreserves
<1>2. RecoveryInv /\ [RecoveryNext]_recoveryVars => RecoveryInv'
  BY RecoveryNextPreserves, RecoveryStutterPreserves
    DEF RecoveryNextPreserves, RecoveryStutterPreserves, RecoveryStutter
<1>3. RecoverySpec => []RecoveryInv
  BY <1>1, <1>2, PTL DEF RecoverySpec
<1>4. RecoveryInv => RecoveryTypeOK /\ r \in RecoveryRecordSpace
  BY HintRecordType DEF HintRecordType, RecoveryInv
<1>5. QED BY <1>3, <1>4, PTL

THEOREM HintPositiveReachableTyping ==
  /\ (FiniteRecoverySpec => []RecoveryTypeOK)
  /\ (CrossHintRecoverySpec => []RecoveryTypeOK)
  /\ (\A profile \in 0..3 : WideRecoverySpec(profile) => []RecoveryTypeOK)
  /\ (\A workload \in 0..3 : WorkloadRecoverySpec(workload) => []RecoveryTypeOK)
<1>1. FiniteRecoverySpec => []RecoveryTypeOK
  BY FiniteRecoverySpecRefines, HintReachableTyping
    DEF FiniteRecoverySpecRefines, HintReachableTyping
<1>2. CrossHintRecoverySpec => []RecoveryTypeOK
  BY CrossHintRecoverySpecRefines, HintReachableTyping
    DEF CrossHintRecoverySpecRefines, HintReachableTyping
<1>3. \A profile \in 0..3 : WideRecoverySpec(profile) => []RecoveryTypeOK
  BY WideRecoverySpecRefines, HintReachableTyping
    DEF WideRecoverySpecRefines, HintReachableTyping
<1>4. \A workload \in 0..3 : WorkloadRecoverySpec(workload) => []RecoveryTypeOK
  BY WorkloadRecoverySpecRefines, HintReachableTyping
    DEF WorkloadRecoverySpecRefines, HintReachableTyping
<1>5. QED BY <1>1, <1>2, <1>3, <1>4

(***************************************************************************
Positive-only hint replacement: the bound h is one fixed value for both ends
of an action. Each Rehint action evaluates its M02 action at SetHintMap(r,h)
and SetHintMap(r',h), with exactly the original p and p'. No mutant is included.
The atomic replay lemmas require recovery typing, not RecoverySafety or any
claimed observation property. The matching lemmas obtain successor typing
from RecoveryNextPreserves and the existing positive-kernel refinements.
***************************************************************************)

(* Publication predicates are shared verbatim: none depends on r. *)
THEOREM HintPublicationKernelIdentity == \A h \in HintMaps :
  /\ (Next <=> Next)
  /\ (FiniteNext <=> FiniteNext)
  /\ (\A profile \in 0..3 :
       WideNext(profile) <=> WideNext(profile))
OBVIOUS

THEOREM HintPublicationStepReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ PublicationStep => RehintPublicationStep(h)
BY SMT, HintPublicationKernelIdentity
  DEF HintPublicationKernelIdentity, RecoveryTypeOK, SetHintMap, PublicationStep,
  RehintPublicationStep

THEOREM HintFinitePublicationStepReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ FinitePublicationStep => RehintFinitePublicationStep(h)
BY SMT, HintPublicationKernelIdentity
  DEF HintPublicationKernelIdentity, RecoveryTypeOK, SetHintMap,
  FinitePublicationStep, RehintFinitePublicationStep

THEOREM HintWidePublicationStepReplay ==
  \A h \in HintMaps, profile \in 0..3 :
    RecoveryTypeOK /\ WidePublicationStep(profile) =>
      RehintWidePublicationStep(h,profile)
BY SMT, HintPublicationKernelIdentity
  DEF HintPublicationKernelIdentity, RecoveryTypeOK, SetHintMap,
  WidePublicationStep, RehintWidePublicationStep

THEOREM HintRecoverStartReplay ==
  \A h \in HintMaps, w \in Writers, i \in ReplayInputs :
    RecoveryTypeOK /\ RecoverStart(w,i) => RehintRecoverStart(h,w,i)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             NEW i \in ReplayInputs,
             RecoveryTypeOK, RecoverStart(w,i)
      PROVE RehintRecoverStart(h,w,i)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.target[w] = i, !.pc[w] = "inspect",
               !.result[w] = "pending", !.checked[w] = 0]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RecoverStart
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RecoverStart, RehintRecoverStart
<1>2. QED BY <1>1

THEOREM HintDetectCorruptionReplay == \A h \in HintMaps, i \in Releases :
  RecoveryTypeOK /\ DetectCorruption(i) => RehintDetectCorruption(h,i)
<1>1. ASSUME NEW h \in HintMaps,
             NEW i \in Releases,
             RecoveryTypeOK, DetectCorruption(i)
      PROVE RehintDetectCorruption(h,i)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.corrupt = @ \cup {i}]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, DetectCorruption
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, DetectCorruption, RehintDetectCorruption
<1>2. QED BY <1>1

THEOREM HintRefuseCorruptionReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RefuseCorruption(w) => RehintRefuseCorruption(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RefuseCorruption(w)
      PROVE RehintRefuseCorruption(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "refused"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RefuseCorruption
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RefuseCorruption, RehintRefuseCorruption,
  Inspecting, RehintInspecting, Intact, RehintIntact, ReplayInputs, Inputs,
  LegacyId
<1>2. QED BY <1>1

THEOREM HintReplayConflictReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ ReplayConflict(w) => RehintReplayConflict(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, ReplayConflict(w)
      PROVE RehintReplayConflict(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "conflict"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, ReplayConflict
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, ReplayConflict, RehintReplayConflict,
  Inspecting, RehintInspecting, Intact, RehintIntact, ReplayInputs, Inputs,
  LegacyId, OpOf
<1>2. QED BY <1>1

THEOREM HintRefuseAuthorityReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RefuseAuthority(w) => RehintRefuseAuthority(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RefuseAuthority(w)
      PROVE RehintRefuseAuthority(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "refused"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RefuseAuthority
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RefuseAuthority, RehintRefuseAuthority,
  Inspecting, RehintInspecting, Intact, RehintIntact, ReplayInputs, Inputs,
  LegacyId, Valid, SchemaOf, Contexts, Checks
<1>2. QED BY <1>1

THEOREM HintPredecessorConflictReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ PredecessorConflict(w) => RehintPredecessorConflict(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, PredecessorConflict(w)
      PROVE RehintPredecessorConflict(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "conflict"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, PredecessorConflict
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, PredecessorConflict, RehintPredecessorConflict,
  Inspecting, RehintInspecting, Intact, RehintIntact, ReplayInputs, Inputs,
  LegacyId, OpOf, BaseOf
<1>2. QED BY <1>1

THEOREM HintResumeExactReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ ResumeExact(w) => RehintResumeExact(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, ResumeExact(w)
      PROVE RehintResumeExact(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "resumed"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, ResumeExact
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, ResumeExact, RehintResumeExact, Inspecting,
  RehintInspecting, Intact, RehintIntact, ReplayInputs, Inputs, LegacyId,
  Valid, SchemaOf, Contexts, Checks, BoundIntent, OpOf, BaseOf
<1>2. QED BY <1>1

THEOREM HintRepairModernCompletionReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RepairModernCompletion(w) =>
    RehintRepairModernCompletion(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RepairModernCompletion(w)
      PROVE RehintRepairModernCompletion(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = SetHintMap(r,h).target[w]]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RepairModernCompletion
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RepairModernCompletion,
  RehintRepairModernCompletion, Inspecting, RehintInspecting, Intact,
  RehintIntact, ReplayInputs, Inputs, LegacyId, Valid, SchemaOf, Contexts,
  Checks, BoundIntent, OpOf
<1>2. QED BY <1>1

THEOREM HintRepairLegacyCompletionReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RepairLegacyCompletion(w) =>
    RehintRepairLegacyCompletion(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RepairLegacyCompletion(w)
      PROVE RehintRepairLegacyCompletion(h,w)
  <2> USE RecoveryTypeOK, RepairLegacyCompletion(w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "already_applied",
               !.checked[w] = LegacyId]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RepairLegacyCompletion
  <2>4. RehintInspecting(h,w,LegacyId) /\ RehintIntact(h,LegacyId)
    BY SMT, <2>2 DEF RepairLegacyCompletion, Inspecting, RehintInspecting,
      Intact, RehintIntact
  <2>5. HistoricalAuthentic /\ UNCHANGED p
    BY DEF RepairLegacyCompletion
  <2>6. QED BY <2>3, <2>4, <2>5 DEF RehintRepairLegacyCompletion
<1>2. QED BY <1>1

THEOREM HintRepairCompletionReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RepairCompletion(w) => RehintRepairCompletion(h,w)
BY SMT, HintRepairModernCompletionReplay, HintRepairLegacyCompletionReplay
  DEF HintRepairModernCompletionReplay, HintRepairLegacyCompletionReplay,
  RepairCompletion, RehintRepairCompletion

THEOREM HintRefusePendingLegacyReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RefusePendingLegacy(w) => RehintRefusePendingLegacy(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RefusePendingLegacy(w)
      PROVE RehintRefusePendingLegacy(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "refused"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RefusePendingLegacy
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RefusePendingLegacy, RehintRefusePendingLegacy,
  Inspecting, RehintInspecting, ReplayInputs, Inputs, LegacyId, SchemaOf
<1>2. QED BY <1>1

THEOREM HintObserveCompletionReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ ObserveCompletion(w) => RehintObserveCompletion(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, ObserveCompletion(w)
      PROVE RehintObserveCompletion(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = SetHintMap(r,h).target[w]]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, ObserveCompletion
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, ObserveCompletion, RehintObserveCompletion,
  Intact, RehintIntact, Inputs, LegacyId, Valid, SchemaOf, Contexts, Checks,
  BoundIntent, OpOf
<1>2. QED BY <1>1

THEOREM HintRecoveryIOFailureReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RecoveryIOFailure(w) => RehintRecoveryIOFailure(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RecoveryIOFailure(w)
      PROVE RehintRecoveryIOFailure(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "done", !.result[w] = "unknown"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RecoveryIOFailure
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RecoveryIOFailure, RehintRecoveryIOFailure
<1>2. QED BY <1>1

THEOREM HintRecoveryCrashReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RecoveryCrash(w) => RehintRecoveryCrash(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RecoveryCrash(w)
      PROVE RehintRecoveryCrash(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "down", !.result[w] = "unknown"]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RecoveryCrash
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RecoveryCrash, RehintRecoveryCrash, Crash
<1>2. QED BY <1>1

THEOREM HintRecoveryRestartReplay == \A h \in HintMaps, w \in Writers :
  RecoveryTypeOK /\ RecoveryRestart(w) => RehintRecoveryRestart(h,w)
<1>1. ASSUME NEW h \in HintMaps,
             NEW w \in Writers,
             RecoveryTypeOK, RecoveryRestart(w)
      PROVE RehintRecoveryRestart(h,w)
  <2>1. r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType
  <2>2. /\ SetHintMap(r,h).target = r.target
        /\ SetHintMap(r,h).pc = r.pc
        /\ SetHintMap(r,h).result = r.result
        /\ SetHintMap(r,h).checked = r.checked
        /\ SetHintMap(r,h).corrupt = r.corrupt
    BY SMT, <1>1, <2>1, HintRetargetFields DEF HintRetargetFields
  <2>3. SetHintMap(r',h) = [SetHintMap(r,h) EXCEPT !.pc[w] = "idle", !.target[w] = 0,
               !.result[w] = "none", !.checked[w] = 0]
    BY SMT, <1>1 DEF RecoveryTypeOK, SetHintMap, RecoveryRestart
  <2>4. QED BY SMT, <1>1, <2>1, <2>2, <2>3
    DEF RecoveryTypeOK, SetHintMap, RecoveryRestart, RehintRecoveryRestart,
  Restart
<1>2. QED BY <1>1

THEOREM HintRecoveryStutterReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ RecoveryStutter => RehintRecoveryStutter(h)
BY SMT DEF SetHintMap, RecoveryStutter, RehintRecoveryStutter, recoveryVars,
  RehintrecoveryVars

(* An original hint write need not be the same hint write after replacement.
   Its matching action is the explicit stutter available in every kernel. *)
THEOREM HintMisleadReplaysAsStutter ==
  \A h \in HintMaps, o \in Ops, i \in Releases, phase \in HintPhases :
    RecoveryTypeOK /\ MisleadHint(o,i,phase) => RehintRecoveryStutter(h)
BY SMT DEF RecoveryTypeOK, SetHintMap, MisleadHint, RehintRecoveryStutter,
  RehintrecoveryVars

THEOREM HintUnrestrictedHintReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ UnrestrictedHintStep => RehintRecoveryStutter(h)
BY SMT, HintMisleadReplaysAsStutter
  DEF HintMisleadReplaysAsStutter, UnrestrictedHintStep

THEOREM HintFiniteHintReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ FiniteHintStep => RehintRecoveryStutter(h)
BY SMT, FiniteHintRefines, HintUnrestrictedHintReplay
  DEF FiniteHintRefines, HintUnrestrictedHintReplay

THEOREM HintCrossHintReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ CrossHintStep => RehintRecoveryStutter(h)
BY SMT, CrossHintRefines, HintUnrestrictedHintReplay
  DEF CrossHintRefines, HintUnrestrictedHintReplay

THEOREM HintRecoveryServiceReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ RecoveryServiceNext => RehintRecoveryServiceNext(h)
BY SMT, HintDetectCorruptionReplay, HintRefuseCorruptionReplay,
  HintReplayConflictReplay, HintRefuseAuthorityReplay,
  HintPredecessorConflictReplay, HintResumeExactReplay,
  HintRepairCompletionReplay, HintObserveCompletionReplay,
  HintRecoveryIOFailureReplay, HintRecoveryCrashReplay,
  HintRecoveryRestartReplay, HintRefusePendingLegacyReplay,
  HintRecoveryStutterReplay
  DEF HintDetectCorruptionReplay, HintRefuseCorruptionReplay,
  HintReplayConflictReplay, HintRefuseAuthorityReplay,
  HintPredecessorConflictReplay, HintResumeExactReplay,
  HintRepairCompletionReplay, HintObserveCompletionReplay,
  HintRecoveryIOFailureReplay, HintRecoveryCrashReplay,
  HintRecoveryRestartReplay, HintRefusePendingLegacyReplay,
  HintRecoveryStutterReplay, RecoveryServiceNext, RehintRecoveryServiceNext,
  Writers, Releases

THEOREM HintNonHintRecoveryReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ NonHintRecoveryNext => RehintNonHintRecoveryNext(h)
BY SMT, HintRecoverStartReplay, HintRecoveryServiceReplay
  DEF HintRecoverStartReplay, HintRecoveryServiceReplay, NonHintRecoveryNext,
  RehintNonHintRecoveryNext, RecoveryServiceNext, RehintRecoveryServiceNext,
  Writers, Releases, ReplayInputs, Inputs, LegacyId

THEOREM HintLocalRecoveryReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ LocalRecoveryNext => RehintLocalRecoveryNext(h)
BY SMT, HintNonHintRecoveryReplay, HintUnrestrictedHintReplay
  DEF HintNonHintRecoveryReplay, HintUnrestrictedHintReplay, LocalRecoveryNext,
  RehintLocalRecoveryNext, NonHintRecoveryNext, RehintNonHintRecoveryNext,
  UnrestrictedHintStep

THEOREM HintRecoveryNextReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ RecoveryNext => RehintRecoveryNext(h)
BY SMT, HintPublicationStepReplay, HintLocalRecoveryReplay
  DEF HintPublicationStepReplay, HintLocalRecoveryReplay, RecoveryNext,
  RehintRecoveryNext

THEOREM HintWorkloadLocalReplay == \A h \in HintMaps, workload \in 0..3 :
  RecoveryTypeOK /\ WorkloadLocalNext(workload) =>
    RehintWorkloadLocalNext(h,workload)
BY SMT, HintRecoveryServiceReplay, HintRecoverStartReplay, WorkloadTargetsTyped
  DEF HintRecoveryServiceReplay, HintRecoverStartReplay, WorkloadTargetsTyped,
  WorkloadLocalNext, RehintWorkloadLocalNext, WorkloadTarget, Writers,
  LegacyId

THEOREM HintWorkloadRecoveryNextReplay ==
  \A h \in HintMaps, workload \in 0..3 :
    RecoveryTypeOK /\ WorkloadRecoveryNext(workload) =>
      RehintWorkloadRecoveryNext(h,workload)
BY SMT, HintFinitePublicationStepReplay, HintWorkloadLocalReplay,
  HintFiniteHintReplay
  DEF HintFinitePublicationStepReplay, HintWorkloadLocalReplay,
  HintFiniteHintReplay, WorkloadRecoveryNext, RehintWorkloadRecoveryNext,
  RehintWorkloadLocalNext, RehintRecoveryServiceNext

THEOREM HintFiniteRecoveryNextReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ FiniteRecoveryNext => RehintFiniteRecoveryNext(h)
BY SMT, HintWorkloadRecoveryNextReplay
  DEF HintWorkloadRecoveryNextReplay, FiniteRecoveryNext,
  RehintFiniteRecoveryNext

THEOREM HintWideRecoveryNextReplay == \A h \in HintMaps, profile \in 0..3 :
  RecoveryTypeOK /\ WideRecoveryNext(profile) => RehintWideRecoveryNext(h,profile)
BY SMT, HintWidePublicationStepReplay, HintWorkloadLocalReplay, HintFiniteHintReplay
  DEF HintWidePublicationStepReplay, HintWorkloadLocalReplay,
  HintFiniteHintReplay, WideRecoveryNext, RehintWideRecoveryNext,
  RehintWorkloadLocalNext, RehintRecoveryServiceNext

THEOREM HintCrossHintRecoveryNextReplay == \A h \in HintMaps :
  RecoveryTypeOK /\ CrossHintRecoveryNext => RehintCrossHintRecoveryNext(h)
BY SMT, HintWidePublicationStepReplay, HintWorkloadLocalReplay, HintCrossHintReplay
  DEF HintWidePublicationStepReplay, HintWorkloadLocalReplay,
  HintCrossHintReplay, CrossHintRecoveryNext, RehintCrossHintRecoveryNext,
  RehintWorkloadLocalNext, RehintRecoveryServiceNext

(* q is an arbitrary typed alternate recovery record, not a canonical-hint
   restriction. t is its explicit matching successor; p and p' are shared.
   The action projection in each theorem below uses the fixed map q.hint. *)
HintMatchingPair(q,t) ==
  /\ q = SetHintMap(r,q.hint)
  /\ t = SetHintMap(r',q.hint)
  /\ RecoveryCore(t) = RecoveryCore(r')
  /\ <<p,q.target,q.pc,q.result,q.checked,q.corrupt>> = RecoveryObservation
  /\ <<p',t.target,t.pc,t.result,t.checked,t.corrupt>> = RecoveryObservation'
  /\ RehintRecoveryObservation(q.hint) = RecoveryObservation
  /\ (RehintRecoveryObservation(q.hint))' = RecoveryObservation'

THEOREM HintMatchingEndpoints == \A q \in RecoveryRecordSpace :
  RecoveryInv /\ RecoveryNext /\ RecoveryCore(q) = RecoveryCore(r) =>
    \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
<1>1. ASSUME NEW q \in RecoveryRecordSpace,
             RecoveryInv, RecoveryNext, RecoveryCore(q) = RecoveryCore(r)
      PROVE \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
  <2>1. RecoveryTypeOK /\ r \in RecoveryRecordSpace
    BY <1>1, HintRecordType DEF HintRecordType, RecoveryInv
  <2>2. q.hint \in HintMaps
    BY SMT, <1>1 DEF RecoveryRecordSpace
  <2>3. RecoveryTypeOK' /\ r' \in RecoveryRecordSpace
    BY SMT, <1>1, RecoveryNextPreserves
      DEF RecoveryNextPreserves, RecoveryInv, RecoveryTypeOK, RecoveryRecordSpace,
  HintMaps
  <2>4. q = SetHintMap(r,q.hint)
    BY SMT, <1>1, <2>1, HintCoreCharacterizes DEF HintCoreCharacterizes
  <2>5. /\ SetHintMap(r',q.hint) \in RecoveryRecordSpace
        /\ RecoveryCore(SetHintMap(r',q.hint)) = RecoveryCore(r')
    BY SMT, <2>2, <2>3, HintRetargetTyped, HintRetargetCore
      DEF HintRetargetTyped, HintRetargetCore
  <2>6. /\ RehintRecoveryObservation(q.hint) = RecoveryObservation
        /\ (RehintRecoveryObservation(q.hint))' = RecoveryObservation'
    BY SMT, <2>1, <2>2, <2>3, HintObservedState, HintObservedNextState
      DEF HintObservedState, HintObservedNextState
  <2>7. /\ <<p,q.target,q.pc,q.result,q.checked,q.corrupt>>
             = RecoveryObservation
        /\ <<p',SetHintMap(r',q.hint).target,SetHintMap(r',q.hint).pc,
                SetHintMap(r',q.hint).result,SetHintMap(r',q.hint).checked,
                SetHintMap(r',q.hint).corrupt>> = RecoveryObservation'
    BY SMT, <1>1, <2>1, <2>2, <2>3, <2>4
      DEF SetHintMap, RecoveryObservation, RecoveryRecordSpace
  <2>8. HintMatchingPair(q,SetHintMap(r',q.hint))
    BY <2>4, <2>5, <2>6, <2>7 DEF HintMatchingPair
  <2>9. QED BY <2>5, <2>8
<1>2. QED BY <1>1

THEOREM HintRecoveryNextMatches == \A q \in RecoveryRecordSpace :
  RecoveryInv /\ RecoveryNext /\ RecoveryCore(q) = RecoveryCore(r) =>
    /\ RehintRecoveryNext(q.hint)
    /\ \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
BY SMT, HintRecoveryNextReplay, HintMatchingEndpoints
  DEF HintRecoveryNextReplay, HintMatchingEndpoints, RecoveryInv,
  RecoveryRecordSpace

THEOREM HintWorkloadRecoveryNextMatches ==
  \A q \in RecoveryRecordSpace, workload \in 0..3 :
    RecoveryInv /\ WorkloadRecoveryNext(workload) /\
      RecoveryCore(q) = RecoveryCore(r) =>
      /\ RehintWorkloadRecoveryNext(q.hint,workload)
      /\ \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
BY SMT, HintWorkloadRecoveryNextReplay, WorkloadRecoveryStepRefines,
  HintMatchingEndpoints
  DEF HintWorkloadRecoveryNextReplay, WorkloadRecoveryStepRefines,
  HintMatchingEndpoints, RecoveryInv, RecoveryRecordSpace

THEOREM HintFiniteRecoveryNextMatches == \A q \in RecoveryRecordSpace :
  RecoveryInv /\ FiniteRecoveryNext /\ RecoveryCore(q) = RecoveryCore(r) =>
    /\ RehintFiniteRecoveryNext(q.hint)
    /\ \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
BY SMT, HintFiniteRecoveryNextReplay, FiniteRecoveryStepRefines,
  HintMatchingEndpoints
  DEF HintFiniteRecoveryNextReplay, FiniteRecoveryStepRefines,
  HintMatchingEndpoints, RecoveryInv, RecoveryRecordSpace

THEOREM HintWideRecoveryNextMatches ==
  \A q \in RecoveryRecordSpace, profile \in 0..3 :
    RecoveryInv /\ WideRecoveryNext(profile) /\ RecoveryCore(q) = RecoveryCore(r) =>
      /\ RehintWideRecoveryNext(q.hint,profile)
      /\ \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
BY SMT, HintWideRecoveryNextReplay, WideRecoveryStepRefines, HintMatchingEndpoints
  DEF HintWideRecoveryNextReplay, WideRecoveryStepRefines, HintMatchingEndpoints,
  RecoveryInv, RecoveryRecordSpace

THEOREM HintCrossHintRecoveryNextMatches == \A q \in RecoveryRecordSpace :
  RecoveryInv /\ CrossHintRecoveryNext /\ RecoveryCore(q) = RecoveryCore(r) =>
    /\ RehintCrossHintRecoveryNext(q.hint)
    /\ \E t \in RecoveryRecordSpace : HintMatchingPair(q,t)
BY SMT, HintCrossHintRecoveryNextReplay, CrossHintRecoveryStepRefines,
  HintMatchingEndpoints
  DEF HintCrossHintRecoveryNextReplay, CrossHintRecoveryStepRefines,
  HintMatchingEndpoints, RecoveryInv, RecoveryRecordSpace
=============================================================================
