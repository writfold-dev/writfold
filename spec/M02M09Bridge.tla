--------------------------- MODULE M02M09Bridge ---------------------------
EXTENDS M02RecoveryReplay
USE ParameterBounds DEF Bounds
CONSTANT AuthoredActorCount
VARIABLE a
A == INSTANCE M09AuthoredDesignLifecycle WITH ActorCount <- AuthoredActorCount, a <- a
bridgeVars == <<p,r,a>>

(* Actual state product, not an identification of publication and file CAS.
   p/r are M02's durable release and recovery state; a is M09's authored state.
   Runtime premise: separate namespace dispatch and managed authored lock cannot
   mutate completed Git refs, release intents, signatures or recovery records.
   Authored actor provenance is deliberately not mapped to knowledge approval.
   Shared discipline is exact retained input + independently checked actual
   durable effects. Hints in either namespace are non-authoritative.
   This is an inductive product contract, NOT a Rust refinement/liveness proof.
   Config-only observation profiles below permit simultaneous competing faults;
   they are not hypotheses of any theorem. Every phase/action remains enabled
   subject to its real guards. No global missing-check/error-count constraint. *)
KnowledgeKey(i) == <<"knowledge-release",OpOf(i)>>
AuthoredKey(i) == <<"authored-file",A!OpOf(i)>>
KnowledgeExact(i) == [key |-> KnowledgeKey(i), input |-> Input(i)]
AuthoredExact(i) == [key |-> AuthoredKey(i), input |-> A!Input(i)]
LegacyKnowledgeKey == <<"knowledge-legacy",LegacyId>>
NamespaceSeparation ==
  /\ (\A i \in Inputs,j \in A!Requests : KnowledgeKey(i) # AuthoredKey(j))
  /\ (\A j \in A!Requests : LegacyKnowledgeKey # AuthoredKey(j))
SharedRecovery ==
  /\ (\A w \in Writers : r.result[w]="complete" =>
       /\ p.intent[OpOf(r.target[w])]=r.target[w]
       /\ r.target[w] \in p.published /\ r.target[w] \in p.receipt
       /\ KnowledgeExact(r.checked[w])=KnowledgeExact(r.target[w]))
  /\ (\A w \in Writers : r.result[w]="already_applied" =>
       /\ r.target[w]=LegacyId /\ r.checked[w]=LegacyId
       /\ HistoricalAuthentic /\ p.legacy.receipt.input=p.legacy.input)
  /\ (\A w \in A!Actors : a.result[w]="complete" =>
       /\ a.intent[A!OpOf(a.request[w])]=a.request[w]
       /\ a.request[w] \in a.applied /\ a.request[w] \in a.retained
       /\ AuthoredExact(a.checked[w])=AuthoredExact(a.request[w]))
(* INSTANCE theorem hypotheses include the instantiated module's parameter
   bound. Carry that existing contract explicitly in Init/Inv; no new ASSUME. *)
BridgeParameters == A!ActorBounds
BridgeInv == BridgeParameters /\ RecoveryInv /\ A!Inv /\ NamespaceSeparation /\ SharedRecovery
BridgeSafety == RecoverySafety /\ A!Safety /\ NamespaceSeparation /\ SharedRecovery
BridgeInit == BridgeParameters /\ RecoveryInit /\ A!Init
KnowledgeStep == RecoveryNext /\ UNCHANGED a
AuthoredStep == A!Next /\ UNCHANGED <<p,r>>
BridgeStutter == UNCHANGED bridgeVars
BridgeNext == KnowledgeStep \/ AuthoredStep \/ BridgeStutter
BridgeSpec == BridgeInit /\ [][BridgeNext]_bridgeVars
(* Authored finite observation profiles are an exploration restriction only;
   the product's unrestricted safety obligations continue to use BridgeNext.
   Knowledge-side observation coverage remains separately disclosed below. *)
BridgeFiniteNext == BridgeNext /\ A!FiniteObservations'
BridgeFiniteSpec == BridgeInit /\ [][BridgeFiniteNext]_bridgeVars
THEOREM BridgeFiniteNextIncluded == BridgeFiniteNext => BridgeNext
BY DEF BridgeFiniteNext

(* Explicit finite representative knowledge failures. Unlike <=1 missing-check
   bounds, the last profile admits distinct simultaneous faults in input 1/2.
   Authored profiles independently vary, so failures coexist across namespaces.
   Wider configs change actors/contexts, not allowed protocol phases. *)
BridgeObservationProfiles == AbsentObservations \in
  {{}, {<<1,"policy",1>>}, {<<2,"signature",2>>},
   {<<1,"source",1>>,<<2,"kernel",2>>}}

THEOREM SharedRecoveryFromInvariants == RecoveryInv /\ A!Inv => SharedRecovery
<1>1. RecoveryInv => (\A w \in Writers : r.result[w]="complete" =>
  /\ p.intent[OpOf(r.target[w])]=r.target[w]
  /\ r.target[w] \in p.published /\ r.target[w] \in p.receipt
  /\ r.checked[w]=r.target[w])
  BY DEF RecoveryInv, RecoveryEvidence, BoundIntent, Inv, ReceiptSound
<1>2. A!Inv => (\A w \in A!Actors : a.result[w]="complete" =>
  /\ a.intent[A!OpOf(a.request[w])]=a.request[w]
  /\ a.request[w] \in a.applied /\ a.request[w] \in a.retained
  /\ a.checked[w]=a.request[w])
  BY DEF A!Inv, A!ReceiptSound, A!Chain, A!BoundIntent
<1>3. RecoveryInv => (\A w \in Writers : r.result[w]="already_applied" =>
  /\ r.target[w]=LegacyId /\ r.checked[w]=LegacyId
  /\ HistoricalAuthentic /\ p.legacy.receipt.input=p.legacy.input)
  BY DEF RecoveryInv, RecoveryEvidence, HistoricalAuthentic
<1>4. QED BY <1>1, <1>2, <1>3 DEF SharedRecovery
THEOREM SharedRecoveryAfterInvariants == RecoveryInv' /\ A!Inv' => SharedRecovery'
<1>1. RecoveryInv' => (\A w \in Writers : r'.result[w]="complete" =>
  /\ p'.intent[OpOf(r'.target[w])]=r'.target[w]
  /\ r'.target[w] \in p'.published /\ r'.target[w] \in p'.receipt
  /\ r'.checked[w]=r'.target[w])
  BY DEF RecoveryInv, RecoveryEvidence, BoundIntent, Inv, ReceiptSound
<1>2. A!Inv' => (\A w \in A!Actors : a'.result[w]="complete" =>
  /\ a'.intent[A!OpOf(a'.request[w])]=a'.request[w]
  /\ a'.request[w] \in a'.applied /\ a'.request[w] \in a'.retained
  /\ a'.checked[w]=a'.request[w])
  BY DEF A!Inv, A!ReceiptSound, A!Chain, A!BoundIntent
<1>3. RecoveryInv' => (\A w \in Writers : r'.result[w]="already_applied" =>
  /\ r'.target[w]=LegacyId /\ r'.checked[w]=LegacyId
  /\ HistoricalAuthentic' /\ p'.legacy.receipt.input=p'.legacy.input)
  BY DEF RecoveryInv, RecoveryEvidence, HistoricalAuthentic
<1>4. QED BY <1>1, <1>2, <1>3 DEF SharedRecovery
THEOREM NamespacesDisjoint == NamespaceSeparation
BY SMT DEF NamespaceSeparation, KnowledgeKey, AuthoredKey, LegacyKnowledgeKey
THEOREM BridgeInitPreserves == BridgeInit => BridgeInv
<1>1. BridgeInit => RecoveryInv /\ A!Inv
  BY RecoveryInitPreserves, A!InitPreserves
    DEF BridgeInit, BridgeParameters, RecoveryInitPreserves, A!InitPreserves,
      A!ActorUniverse, A!ActorBounds
<1>2. BridgeInit => SharedRecovery
  BY <1>1, SharedRecoveryFromInvariants DEF SharedRecoveryFromInvariants
<1>3. QED BY <1>1, <1>2, NamespacesDisjoint
  DEF BridgeInit, BridgeInv, NamespacesDisjoint
THEOREM KnowledgeFramesAuthored == KnowledgeStep => UNCHANGED a
BY DEF KnowledgeStep
THEOREM AuthoredFramesKnowledge == AuthoredStep => UNCHANGED <<p,r>>
BY DEF AuthoredStep
THEOREM KnowledgeStepPreserves == BridgeInv /\ KnowledgeStep => BridgeInv'
<1>1. ASSUME BridgeInv, KnowledgeStep PROVE BridgeInv'
  <2> USE BridgeInv, KnowledgeStep
  <2>1. RecoveryInv'
    BY RecoveryNextPreserves DEF BridgeInv, KnowledgeStep, RecoveryNextPreserves
  <2>2. A!Inv'
    BY A!StateFrame DEF BridgeInv, BridgeParameters, KnowledgeStep, A!StateFrame,
      A!ActorUniverse, A!ActorBounds
  <2>3. SharedRecovery'
    BY <2>1, <2>2, SharedRecoveryAfterInvariants
      DEF SharedRecoveryAfterInvariants
  <2>4. QED BY <2>1, <2>2, <2>3, NamespacesDisjoint
    DEF BridgeInv, BridgeParameters, A!ActorBounds, NamespacesDisjoint,
      NamespaceSeparation, KnowledgeKey, AuthoredKey, LegacyKnowledgeKey
<1>2. QED BY <1>1
THEOREM AuthoredStepPreserves == BridgeInv /\ AuthoredStep => BridgeInv'
<1>1. ASSUME BridgeInv, AuthoredStep PROVE BridgeInv'
  <2> USE BridgeInv, AuthoredStep
  <2>1. A!Inv'
    BY A!NextPreserves DEF BridgeInv, BridgeParameters, AuthoredStep, A!NextPreserves,
      A!ActorUniverse, A!ActorBounds
  <2>2. RecoveryInv'
    BY RecoveryStutterPreserves
      DEF BridgeInv, AuthoredStep, RecoveryStutterPreserves, RecoveryStutter, recoveryVars
  <2>3. SharedRecovery'
    BY <2>1, <2>2, SharedRecoveryAfterInvariants
      DEF SharedRecoveryAfterInvariants
  <2>4. QED BY <2>1, <2>2, <2>3, NamespacesDisjoint
    DEF BridgeInv, BridgeParameters, A!ActorBounds, NamespacesDisjoint,
      NamespaceSeparation, KnowledgeKey, AuthoredKey, LegacyKnowledgeKey
<1>2. QED BY <1>1
THEOREM BridgeStutterPreserves == BridgeInv /\ BridgeStutter => BridgeInv'
<1>1. BridgeInv /\ BridgeStutter => RecoveryInv'
  BY RecoveryStutterPreserves
    DEF BridgeInv, BridgeStutter, bridgeVars, RecoveryStutterPreserves, RecoveryStutter, recoveryVars
<1>2. BridgeInv /\ BridgeStutter => A!Inv'
  BY A!StateFrame DEF BridgeInv, BridgeParameters, BridgeStutter, bridgeVars,
    A!StateFrame, A!ActorUniverse, A!ActorBounds
<1>3. BridgeInv /\ BridgeStutter => SharedRecovery'
  BY <1>1, <1>2, SharedRecoveryAfterInvariants DEF SharedRecoveryAfterInvariants
<1>4. QED BY <1>1, <1>2, <1>3, NamespacesDisjoint
  DEF BridgeInv, BridgeParameters, A!ActorBounds, NamespacesDisjoint,
    NamespaceSeparation, KnowledgeKey, AuthoredKey, LegacyKnowledgeKey
THEOREM BridgeNextPreserves == BridgeInv /\ BridgeNext => BridgeInv'
BY KnowledgeStepPreserves, AuthoredStepPreserves, BridgeStutterPreserves
  DEF BridgeNext, KnowledgeStepPreserves, AuthoredStepPreserves, BridgeStutterPreserves
THEOREM BridgeFiniteNextPreserves == BridgeInv /\ BridgeFiniteNext => BridgeInv'
BY BridgeNextPreserves DEF BridgeFiniteNext, BridgeNextPreserves
THEOREM BridgeInvImpliesSafety == BridgeInv => BridgeSafety
<1>1. BridgeInv => RecoverySafety
  BY RecoveryInvImpliesSafety DEF BridgeInv, RecoveryInvImpliesSafety
<1>2. BridgeInv => A!Safety
  BY A!InvImpliesSafety
    DEF BridgeInv, BridgeParameters, A!InvImpliesSafety, A!ActorUniverse, A!ActorBounds
<1>3. QED BY <1>1, <1>2 DEF BridgeInv, BridgeSafety
THEOREM BridgeStepPreserves == BridgeInv /\ [BridgeNext]_bridgeVars => BridgeInv'
BY BridgeNextPreserves, BridgeStutterPreserves
  DEF BridgeNextPreserves, BridgeStutterPreserves, BridgeStutter
(* Product induction does not use BridgeObservationProfiles or the authored
   finite hint/corruption quotient. Neither side establishes engine effects. *)
THEOREM BridgeInductiveSafety == BridgeSpec => []BridgeSafety
<1>1. BridgeInit => BridgeInv BY BridgeInitPreserves DEF BridgeInitPreserves
<1>2. BridgeInv /\ [BridgeNext]_bridgeVars => BridgeInv'
  BY BridgeStepPreserves DEF BridgeStepPreserves
<1>3. BridgeSpec => []BridgeInv BY <1>1, <1>2, PTL DEF BridgeSpec
<1>4. QED BY <1>3, BridgeInvImpliesSafety, PTL DEF BridgeInvImpliesSafety

THEOREM KnowledgeRepairExactAndFrame == \A w \in Writers :
  BridgeInv /\ RepairCompletion(w) /\ UNCHANGED a =>
    /\ r'.checked[w]=r.target[w]
    /\ (IF r.target[w] \in Inputs
        THEN p.intent[OpOf(r.target[w])]=r.target[w]
             /\ Input(r'.checked[w])=Input(r.target[w])
        ELSE r.target[w]=LegacyId /\ HistoricalAuthentic
             /\ r'.result[w]="already_applied")
    /\ UNCHANGED <<a,p.head,p.published,p.intent,p.native,p.manifest,p.objects,p.evidence,p.legacy>>
<1>1. \A w \in Writers : BridgeInv /\ RepairModernCompletion(w) =>
  r'.checked[w]=r.target[w] /\ r.target[w] \in Inputs
  BY SMT DEF BridgeInv, RecoveryInv, RecoveryTypeOK, RepairModernCompletion
<1>2. \A w \in Writers : BridgeInv /\ RepairLegacyCompletion(w) =>
  r'.checked[w]=r.target[w] /\ r.target[w]=LegacyId /\ r'.result[w]="already_applied"
  BY SMT DEF BridgeInv, RecoveryInv, RecoveryTypeOK, RepairLegacyCompletion, Inspecting
<1>3. QED BY <1>1, <1>2, RepairDoesNotApplyAgain
  DEF BridgeInv, RepairDoesNotApplyAgain, RepairCompletion,
    RepairModernCompletion, RepairLegacyCompletion, BoundIntent, Inputs, LegacyId
THEOREM AuthoredRepairExactAndFrame == \A w \in A!Actors :
  BridgeInv /\ A!RepairCompletion(w) /\ UNCHANGED <<p,r>> =>
    /\ a.intent[A!OpOf(a.request[w])]=a.request[w]
    /\ A!Input(a'.checked[w])=A!Input(a.request[w])
    /\ UNCHANGED <<p,r,a.head,a.file,a.applied,a.intent,a.retained>>
BY A!ExactRecovery, A!RepairFrames
  DEF BridgeInv, BridgeParameters, A!Inv, A!ExactRecovery, A!RepairFrames,
    A!ActorUniverse, A!ActorBounds
THEOREM KnowledgeFailureFrame == \A w \in Writers :
  BridgeInv /\ (RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
   \/ PredecessorConflict(w) \/ RecoveryIOFailure(w) \/ RefusePendingLegacy(w))
   /\ UNCHANGED a => UNCHANGED <<Durable,a>>
BY RefusalFrames DEF BridgeInv, RefusalFrames
THEOREM AuthoredFailureFrame == \A w \in A!Actors :
  BridgeInv /\ (A!Conflict(w) \/ A!Refuse(w) \/ A!FailIO(w) \/ A!Crash(w) \/ A!Restart(w))
  /\ UNCHANGED <<p,r>> => UNCHANGED <<p,r,A!Durable>>
BY A!FailureFrames DEF BridgeInv, BridgeParameters, A!Inv, A!FailureFrames,
  A!ActorUniverse, A!ActorBounds
THEOREM KnowledgeHintFrame == \A o \in Ops,i \in Releases,h \in HintPhases :
  MisleadHint(o,i,h) /\ UNCHANGED a => UNCHANGED <<p,a>>
BY DEF MisleadHint
THEOREM AuthoredHintFrame == \A o \in A!Ids,i \in A!Revisions :
  BridgeInv /\ A!MisleadHint(o,i) /\ UNCHANGED <<p,r>> => UNCHANGED <<p,r,A!Durable>>
BY A!HintFrames DEF BridgeInv, BridgeParameters, A!Inv, A!HintFrames,
  A!ActorUniverse, A!ActorBounds
=============================================================================
