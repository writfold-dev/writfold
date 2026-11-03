-------------------------- MODULE M02M06Bridge --------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLAPS
CONSTANTS WriterCount, InputCount, ContextCount
VARIABLES p,r,m
R == INSTANCE M02RecoveryReplay
M == INSTANCE M06MigrationSelection
USE R!ParameterBounds, M!ParameterBounds DEF R!Bounds, M!Bounds
vars == <<p,r,m>>

(***************************************************************************
Actual variable map: R.p is M01's durable publication record, R.r is M02's
recovery record, M.m is inventory/history/selection state. No replacement
recovery machine or trusted journal flag is introduced here. p.head=m.head;
p.published=m.retained; p.intent=m.intent; p.native=m.staged. Receipt repair
changes p.receipt and m.receipts, never native state or the correction history.

ExactEnvelope(i) is a common canonical-identity abstraction with two projections:
R.Input(i) carries publication closure/contexts and M.Input(i) carries migration
inventory/selection. Their full records are intentionally NOT asserted equal.
Concrete canonicalization must bind BOTH projections to one retained signed
input; this composition does not prove a byte-codec/refinement theorem. In
particular R.Family is a publication validator abstraction, not a license to
replace M's ordered current contexts with that family during forward selection.
Authority checks from BOTH projections are required at synchronized publication.

Each core publication action wrapped in Publish is an actual R.PublicationStep
subaction, including its corrupt-current guard and unchanged r. Recovery uses
R.RecoverStart/ResumeExact/RepairCompletion and refusal actions. M02 hints remain
untrusted. Exact retained intent identity is synchronized, including collisions.
An already completed authenticated schema-2 release remains readable; no pending
schema-2 input can enter either writer. Originals and mixed-schema byte domains
remain retained. No schema rewrite of policy/backup/migration containers occurs.

No fairness or eventual IO success is assumed; no liveness/refinement completion
is claimed. Finite fixtures below are bounded exploration, not proof premises.
***************************************************************************)
ExactEnvelope(i) == <<R!Input(i),M!Input(i)>>
IdentityMap == \A i \in R!Inputs :
  /\ R!Input(i).canonicalBytes=M!Input(i).canonicalBytes
  /\ R!OpOf(i)=M!OpOf(i) /\ R!BaseOf(i)=M!BaseOf(i)
  /\ R!Input(i).schema=M!Input(i).schema
CouplingCore == /\ p.head=m.head /\ p.published=m.retained
  /\ p.intent=m.intent /\ p.native=m.staged
  /\ m.verified \subseteq p.manifest /\ m.receipts \subseteq p.receipt
Coupling == /\ CouplingCore
  /\ \A i \in m.applied :
       /\ p.intent[R!OpOf(i)]=i
       /\ R!Closure(i) \subseteq p.objects
       /\ m.records[i].history=M!Chain(i)
       /\ m.records[i].corrections=M!Corrections(i)
TypeOK == R!RecoveryTypeOK /\ R!TypeOK /\ M!TypeOK
Inv == R!RecoveryInv /\ M!Inv /\ IdentityMap /\ Coupling
Safety == R!RecoverySafety /\ M!Safety /\ Coupling
Init == R!RecoveryInit /\ M!Init
Publish(a) == a /\ p.head \notin r.corrupt /\ p'.head \notin r.corrupt /\ UNCHANGED r
Start(w,i) == Publish(R!Start(w,i)) /\ M!Start(w,i)
Admit(w) == Publish(R!Admit(w)) /\ UNCHANGED m
RetainIntent(w) == /\ p.request[w]=m.request[w]
  /\ Publish(R!RetainIntent(w)) /\ M!RetainIntent(w)
Stage(w) == /\ p.request[w]=m.request[w]
  /\ Publish(R!NativeCommit(w)) /\ M!Stage(w)
Manifest(w) == Publish(R!RetainManifest(w)) /\ UNCHANGED m
Verify(w) == /\ p.request[w]=m.request[w]
  /\ Publish(R!FinalRevalidate(w)) /\ M!VerifyStage(w)
Apply(w) == /\ p.request[w]=m.request[w]
  /\ Publish(R!DoCAS(w)) /\ M!Apply(w)
Receipt(i) == Publish(R!RecordReceipt(i)) /\ UNCHANGED m
Complete(w) == /\ p.request[w]=m.request[w]
  /\ Publish(R!Finish(w)) /\ M!RecordReceipt(w)
RestoreOriginal(w) == M!RestoreOriginal(w) /\ UNCHANGED <<p,r>>
HistoricalRead(w,i) == M!HistoricalRead(w,i) /\ UNCHANGED <<p,r>>
LegacyReplay(w,b) == M!LegacyReplay(w,b) /\ 0 \notin r.corrupt /\ UNCHANGED <<p,r>>
DryRun == M!DryRun /\ UNCHANGED <<p,r>>
InventoryChange(v) == M!InventoryChange(v) /\ UNCHANGED <<p,r>>
ObservePublication(i,a) == Publish(R!Observe(i,a)) /\ UNCHANGED m
ObserveMigration(i,a) == M!Observe(i,a) /\ UNCHANGED <<p,r>>
BeginRecovery(w,i) == R!RecoverStart(w,i) /\ M!Start(w,i)
BeginLegacyRecovery(w) == R!RecoverStart(w,R!LegacyId) /\ UNCHANGED m
RepairLegacy(w) == R!RepairLegacyCompletion(w) /\ M!LegacyReplay(w,M!LegacyInput)
Resume(w) == /\ R!ResumeExact(w) /\ m.request[w]=r.target[w]
  /\ IF M!BoundIntent(r.target[w]) THEN M!Recover(w) ELSE UNCHANGED m
Replay(w) == /\ R!RepairCompletion(w) /\ m.request[w]=r.target[w] /\ M!Replay(w)
ObserveCompletion(w) == R!ObserveCompletion(w) /\ UNCHANGED m
MisleadHint(o,i,h) == R!MisleadHint(o,i,h) /\ UNCHANGED m
DetectCorruption(i) == R!DetectCorruption(i) /\ UNCHANGED m
RefuseRecovery(w) ==
  /\ (R!RefuseCorruption(w) \/ R!ReplayConflict(w) \/ R!RefuseAuthority(w) \/ R!PredecessorConflict(w) \/ R!RefusePendingLegacy(w) \/ R!RecoveryIOFailure(w))
  /\ UNCHANGED m
RefusePublication(w) ==
  /\ Publish(R!InputConflict(w) \/ R!CASMismatch(w) \/ R!RejectAdmission(w) \/ R!FailIO(w) \/ R!LostResponse(w))
  /\ UNCHANGED m
RefuseMigration(w) ==
  /\ (M!Conflict(w) \/ M!Refuse(w) \/ M!FailIO(w))
  /\ UNCHANGED <<p,r>>
Crash(w) == R!RecoveryCrash(w) /\ M!Crash(w)
Restart(w) == R!RecoveryRestart(w) /\ M!Restart(w)
Stutter == UNCHANGED vars
CoreNext ==
  \/ Stutter
  \/ DryRun
  \/ (\E w \in R!Writers, i \in R!Inputs : Start(w,i) \/ BeginRecovery(w,i))
  \/ (\E w \in R!Writers, i \in R!Releases : HistoricalRead(w,i))
  \/ (\E i \in R!Inputs : Receipt(i))
  \/ (\E v \in {0,1} : InventoryChange(v))
  \/ (\E w \in R!Writers, b \in M!LegacyInputs : LegacyReplay(w,b))
  \/ (\E w \in R!Writers :
       Admit(w) \/ RetainIntent(w) \/ Stage(w) \/ Manifest(w)
       \/ Verify(w) \/ Apply(w) \/ Complete(w) \/ RestoreOriginal(w) \/ Resume(w)
       \/ Replay(w) \/ ObserveCompletion(w) \/ RefuseRecovery(w)
       \/ RefusePublication(w) \/ RefuseMigration(w) \/ Crash(w) \/ Restart(w)
       \/ BeginLegacyRecovery(w) \/ RepairLegacy(w))
Next ==
  \/ CoreNext
  \/ (\E i \in R!Inputs, a \in SUBSET R!FullChecks : ObservePublication(i,a))
  \/ (\E i \in M!Inputs, a \in SUBSET M!Checks : ObserveMigration(i,a))
  \/ (\E o \in R!Ops, i \in R!Releases, h \in R!HintPhases : MisleadHint(o,i,h))
  \/ (\E i \in R!Releases : DetectCorruption(i))
Spec == Init /\ [][Next]_vars

(* Representative simultaneous failed validator observations, not a shared cap.
   One profile can invalidate distinct checks on both colliding operations while
   input 4 remains valid. All writer phases and all inputs remain enabled.
   Hints are finitely restricted to the colliding op; corruption to 0,1,2,5.
   These fixture restrictions are NOT hypotheses of any inductive obligation.
   Profile replacement is a batch of validator observations with final-validation
   invalidation. It has its own preservation/frame obligations; it is not claimed
   to be one R.Next step or a checked stuttering-refinement theorem. *)
PublicationProfiles == { {}, {<<1,"signature",1>>},
  {<<1,"policy",1>>,<<2,"signature",2>>},
  {<<1,"kernel",2>>,<<2,"source",1>>,<<3,"policy",2>>} }
PublicationProfile(f) ==
  /\ p' = [p EXCEPT !.auth =
       [i \in R!Inputs |-> {a \in R!FullChecks : <<i,a[1],a[2]>> \notin f}],
       !.validated = {}]
  /\ UNCHANGED <<r,m>>
MigrationProfile(f) == M!ObserveProfile(f) /\ UNCHANGED <<p,r>>
FiniteNext == CoreNext
  \/ (\E f \in PublicationProfiles : PublicationProfile(f))
  \/ (\E f \in M!Profiles : MigrationProfile(f))
  \/ (\E i \in {0,1,2}, h \in {"none","published"} : MisleadHint(1,i,h))
  \/ (\E i \in {0,1,2,5} : DetectCorruption(i))
FiniteSpec == Init /\ [][FiniteNext]_vars

THEOREM IdentityMapHolds == IdentityMap
BY SMT DEF IdentityMap, R!Input, M!Input, R!OpOf, M!OpOf,
  R!BaseOf, M!BaseOf, R!SchemaOf, M!SchemaOf, R!Inputs, M!Inputs
THEOREM DomainsMatch == /\ R!Writers=M!Writers /\ R!Inputs=M!Inputs
  /\ R!Releases=M!Releases /\ R!Inputs \subseteq R!ReplayInputs
  /\ R!LegacyId \in R!ReplayInputs /\ R!LegacyId \notin M!Releases
  /\ M!LegacyInput \in M!LegacyInputs
BY SMT DEF R!Writers, M!Writers, R!Inputs, M!Inputs, R!Releases,
  M!Releases, R!ReplayInputs, R!LegacyId, M!LegacyInputs
USE IdentityMapHolds, DomainsMatch DEF IdentityMapHolds, DomainsMatch
THEOREM InitPreserves == Init => Inv
<1>1. Init => R!RecoveryInv /\ M!Inv
  BY SMT, R!RecoveryInitPreserves, M!InitPreserves
    DEF Init, R!RecoveryInitPreserves, M!InitPreserves
<1>2. Init => Coupling
  BY SMT DEF Init, Coupling, CouplingCore, R!RecoveryInit, R!Init, M!Init
<1> QED BY SMT, <1>1, <1>2
  DEF Inv
THEOREM StutterPreserves == Inv /\ Stutter => Inv'
<1>1. Inv /\ Stutter => R!RecoveryInv' /\ M!Inv'
  BY SMT, R!RecoveryStutterPreserves, M!StutterPreserves
    DEF Inv, Stutter, vars, R!RecoveryStutter, R!recoveryVars, M!Stutter,
        R!RecoveryStutterPreserves, M!StutterPreserves
<1>2. Inv /\ Stutter => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, Stutter, vars
<1> QED BY <1>1, <1>2 DEF Inv
THEOREM PublicationSimulation ==
  ((\E w \in R!Writers, i \in R!Inputs : Start(w,i)) \/
   (\E w \in R!Writers : Admit(w) \/ RetainIntent(w) \/ Stage(w) \/ Manifest(w)
     \/ Verify(w) \/ Apply(w) \/ Complete(w) \/ RefusePublication(w)) \/
   (\E i \in R!Inputs : Receipt(i)) \/
   (\E i \in R!Inputs, a \in SUBSET R!FullChecks : ObservePublication(i,a)))
  => R!PublicationStep
BY SMT DEF Start, Admit, RetainIntent, Stage, Manifest, Verify, Apply, Complete,
  RefusePublication, Receipt, ObservePublication, Publish,
  R!PublicationStep, R!Next, R!NonObservationNext
THEOREM StartPreserves == \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => Inv'
<1>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Start, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => M!Inv'
  BY SMT, M!StartPreserves DEF M!StartPreserves, Inv, Start, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Start, R!Start, M!Start, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RetainIntentPreserves == \A w \in R!Writers : Inv /\ RetainIntent(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RetainIntent(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, RetainIntent, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RetainIntent(w) => M!Inv'
  BY SMT, M!RetainIntentPreserves DEF M!RetainIntentPreserves, Inv, RetainIntent, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RetainIntent(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RetainIntent, R!RetainIntent, M!RetainIntent, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM StagePreserves == \A w \in R!Writers : Inv /\ Stage(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Stage(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Stage, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Stage(w) => M!Inv'
  BY SMT, M!StagePreserves DEF M!StagePreserves, Inv, Stage, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Stage(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Stage, R!NativeCommit, M!Stage, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM VerifyPreserves == \A w \in R!Writers : Inv /\ Verify(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Verify(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Verify, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Verify(w) => M!Inv'
  BY SMT, M!VerifyStagePreserves DEF M!VerifyStagePreserves, Inv, Verify, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Verify(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Verify, R!FinalRevalidate, M!VerifyStage, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ApplyPreserves == \A w \in R!Writers : Inv /\ Apply(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Apply(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Apply, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Apply(w) => M!Inv'
  BY SMT, M!ApplyPreserves DEF M!ApplyPreserves, Inv, Apply, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Apply(w) => Coupling'
  BY SMT, M!CandidatePreservesHistory DEF M!CandidatePreservesHistory, M!HistorySound, M!HistoryValid, M!HistoryTypes, M!HistoryTyped, M!Record, R!Fresh, R!ValidationSound, Inv, Coupling, CouplingCore, IdentityMap, Apply, R!DoCAS, M!Apply, R!CASCore, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM CompletePreserves == \A w \in R!Writers : Inv /\ Complete(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Complete(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Complete, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Complete(w) => M!Inv'
  BY SMT, M!RecordReceiptPreserves DEF M!RecordReceiptPreserves, Inv, Complete, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Complete(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Complete, R!Finish, M!RecordReceipt, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM AdmitPreserves == \A w \in R!Writers : Inv /\ Admit(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Admit(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Admit, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Admit(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Admit, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Admit(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Admit, R!Admit, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ManifestPreserves == \A w \in R!Writers : Inv /\ Manifest(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Manifest(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Manifest, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Manifest(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Manifest, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Manifest(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Manifest, R!RetainManifest, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ReceiptPreserves == \A i \in R!Inputs : Inv /\ Receipt(i) => Inv'
<1>1. \A i \in R!Inputs : Inv /\ Receipt(i) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Receipt, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A i \in R!Inputs : Inv /\ Receipt(i) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Receipt, M!Stutter, M!BoundIntent
<1>3. \A i \in R!Inputs : Inv /\ Receipt(i) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Receipt, R!RecordReceipt, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ObservePublicationPreserves == \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => Inv'
<1>1. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, ObservePublication, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, ObservePublication, M!Stutter, M!BoundIntent
<1>3. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, ObservePublication, R!Observe, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RestoreOriginalPreserves == \A w \in R!Writers : Inv /\ RestoreOriginal(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, RestoreOriginal, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => M!Inv'
  BY SMT, M!RestoreOriginalPreserves DEF M!RestoreOriginalPreserves, Inv, RestoreOriginal, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RestoreOriginal, M!RestoreOriginal, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM HistoricalReadPreserves == \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => Inv'
<1>1. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, HistoricalRead, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => M!Inv'
  BY SMT, M!HistoricalReadPreserves DEF M!HistoricalReadPreserves, Inv, HistoricalRead, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, HistoricalRead, M!HistoricalRead, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM LegacyReplayPreserves == \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => Inv'
<1>1. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, LegacyReplay, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => M!Inv'
  BY SMT, M!LegacyReplayPreserves DEF M!LegacyReplayPreserves, Inv, LegacyReplay, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, LegacyReplay, M!LegacyReplay, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM DryRunPreserves == Inv /\ DryRun => Inv'
<1>1. Inv /\ DryRun => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, DryRun, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. Inv /\ DryRun => M!Inv'
  BY SMT, M!DryRunPreserves DEF M!DryRunPreserves, Inv, DryRun, M!Stutter, M!BoundIntent
<1>3. Inv /\ DryRun => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, DryRun, M!DryRun, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM InventoryChangePreserves == \A v \in {0,1} : Inv /\ InventoryChange(v) => Inv'
<1>1. \A v \in {0,1} : Inv /\ InventoryChange(v) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, InventoryChange, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A v \in {0,1} : Inv /\ InventoryChange(v) => M!Inv'
  BY SMT, M!InventoryChangePreserves DEF M!InventoryChangePreserves, Inv, InventoryChange, M!Stutter, M!BoundIntent
<1>3. \A v \in {0,1} : Inv /\ InventoryChange(v) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, InventoryChange, M!InventoryChange, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ObserveMigrationPreserves == \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => Inv'
<1>1. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, ObserveMigration, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => M!Inv'
  BY SMT, M!ObservePreserves DEF M!ObservePreserves, Inv, ObserveMigration, M!Stutter, M!BoundIntent
<1>3. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, ObserveMigration, M!Observe, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM MigrationProfilePreserves == \A f \in M!Profiles : Inv /\ MigrationProfile(f) => Inv'
<1>1. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, MigrationProfile, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => M!Inv'
  BY SMT, M!ObserveProfilePreserves DEF M!ObserveProfilePreserves, Inv, MigrationProfile, M!Stutter, M!BoundIntent
<1>3. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, MigrationProfile, M!ObserveProfile, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM BeginRecoveryPreserves == \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => Inv'
<1>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => R!RecoveryInv'
  BY SMT, R!RecoverStartPreserves DEF R!RecoverStartPreserves, Inv, BeginRecovery, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => M!Inv'
  BY SMT, M!StartPreserves DEF M!StartPreserves, Inv, BeginRecovery, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, BeginRecovery, R!RecoverStart, M!Start, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM BeginLegacyRecoveryPreserves == \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => R!RecoveryInv'
  BY SMT, R!RecoverStartPreserves DEF R!RecoverStartPreserves, Inv, BeginLegacyRecovery, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, BeginLegacyRecovery, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, BeginLegacyRecovery, R!RecoverStart, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RepairLegacyPreserves == \A w \in R!Writers : Inv /\ RepairLegacy(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RepairLegacy(w) => R!RecoveryInv'
  BY SMT, R!RepairLegacyCompletionPreserves DEF R!RepairLegacyCompletionPreserves, Inv, RepairLegacy, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RepairLegacy(w) => M!Inv'
  BY SMT, M!LegacyReplayPreserves DEF M!LegacyReplayPreserves, Inv, RepairLegacy, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RepairLegacy(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RepairLegacy, R!RepairLegacyCompletion, M!LegacyReplay, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ResumePreserves == \A w \in R!Writers : Inv /\ Resume(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Resume(w) => R!RecoveryInv'
  BY SMT, R!ResumeExactPreserves DEF R!ResumeExactPreserves, Inv, Resume, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Resume(w) => M!Inv'
  BY SMT, M!RecoverPreserves, M!StutterPreserves DEF M!RecoverPreserves, M!StutterPreserves, Inv, Resume, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Resume(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Resume, R!ResumeExact, M!Recover, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ReplayPreserves == \A w \in R!Writers : Inv /\ Replay(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Replay(w) => R!RecoveryInv'
  BY SMT, R!RepairCompletionPreserves DEF R!RepairCompletionPreserves, Inv, Replay, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Replay(w) => M!Inv'
  BY SMT, M!ReplayPreserves DEF M!ReplayPreserves, Inv, Replay, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Replay(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Replay, R!RepairCompletion, R!RepairModernCompletion, R!RepairLegacyCompletion, R!Inspecting, M!Replay, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM ObserveCompletionPreserves == \A w \in R!Writers : Inv /\ ObserveCompletion(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => R!RecoveryInv'
  BY SMT, R!ObserveCompletionPreserves DEF R!ObserveCompletionPreserves, Inv, ObserveCompletion, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, ObserveCompletion, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, ObserveCompletion, R!ObserveCompletion, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM MisleadHintPreserves == \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => Inv'
<1>1. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => R!RecoveryInv'
  BY SMT, R!MisleadHintPreserves DEF R!MisleadHintPreserves, Inv, MisleadHint, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, MisleadHint, M!Stutter, M!BoundIntent
<1>3. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, MisleadHint, R!MisleadHint, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM DetectCorruptionPreserves == \A i \in R!Releases : Inv /\ DetectCorruption(i) => Inv'
<1>1. \A i \in R!Releases : Inv /\ DetectCorruption(i) => R!RecoveryInv'
  BY SMT, R!DetectCorruptionPreserves DEF R!DetectCorruptionPreserves, Inv, DetectCorruption, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A i \in R!Releases : Inv /\ DetectCorruption(i) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, DetectCorruption, M!Stutter, M!BoundIntent
<1>3. \A i \in R!Releases : Inv /\ DetectCorruption(i) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, DetectCorruption, R!DetectCorruption, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM CrashPreserves == \A w \in R!Writers : Inv /\ Crash(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Crash(w) => R!RecoveryInv'
  BY SMT, R!RecoveryCrashPreserves DEF R!RecoveryCrashPreserves, Inv, Crash, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Crash(w) => M!Inv'
  BY SMT, M!CrashPreserves DEF M!CrashPreserves, Inv, Crash, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Crash(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Crash, R!RecoveryCrash, R!Crash, M!Crash, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RestartPreserves == \A w \in R!Writers : Inv /\ Restart(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ Restart(w) => R!RecoveryInv'
  BY SMT, R!RecoveryRestartPreserves DEF R!RecoveryRestartPreserves, Inv, Restart, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ Restart(w) => M!Inv'
  BY SMT, M!RestartPreserves DEF M!RestartPreserves, Inv, Restart, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ Restart(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, Restart, R!RecoveryRestart, R!Restart, M!Restart, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RefuseRecoveryPreserves == \A w \in R!Writers : Inv /\ RefuseRecovery(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => R!RecoveryInv'
  BY SMT, R!RefuseCorruptionPreserves, R!ReplayConflictPreserves, R!RefuseAuthorityPreserves, R!PredecessorConflictPreserves, R!RefusePendingLegacyPreserves, R!RecoveryIOFailurePreserves DEF R!RefuseCorruptionPreserves, R!ReplayConflictPreserves, R!RefuseAuthorityPreserves, R!PredecessorConflictPreserves, R!RefusePendingLegacyPreserves, R!RecoveryIOFailurePreserves, Inv, RefuseRecovery, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, RefuseRecovery, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RefuseRecovery, R!RefuseCorruption, R!ReplayConflict, R!RefuseAuthority, R!PredecessorConflict, R!RefusePendingLegacy, R!RecoveryIOFailure, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RefusePublicationPreserves == \A w \in R!Writers : Inv /\ RefusePublication(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RefusePublication(w) => R!RecoveryInv'
  BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, RefusePublication, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RefusePublication(w) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, RefusePublication, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RefusePublication(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RefusePublication, R!InputConflict, R!CASMismatch, R!RejectAdmission, R!FailIO, R!LostResponse, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM RefuseMigrationPreserves == \A w \in R!Writers : Inv /\ RefuseMigration(w) => Inv'
<1>1. \A w \in R!Writers : Inv /\ RefuseMigration(w) => R!RecoveryInv'
  BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, RefuseMigration, Publish, R!RecoveryStutter, R!recoveryVars
<1>2. \A w \in R!Writers : Inv /\ RefuseMigration(w) => M!Inv'
  BY SMT, M!ConflictPreserves, M!RefusePreserves, M!FailIOPreserves DEF M!ConflictPreserves, M!RefusePreserves, M!FailIOPreserves, Inv, RefuseMigration, M!Stutter, M!BoundIntent
<1>3. \A w \in R!Writers : Inv /\ RefuseMigration(w) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, IdentityMap, RefuseMigration, M!Conflict, M!Refuse, M!FailIO, Publish,
    R!RecoveryInv, R!Inv, R!TypeOK, R!Prepared, R!BoundIntent,
    M!Inv, M!TypeOK, M!StateShape, M!StageReady, M!BoundIntent
<1> QED BY SMT, <1>1, <1>2, <1>3
  DEF Inv
THEOREM PublicationProfilePreserves == \A f \in PublicationProfiles :
  Inv /\ PublicationProfile(f) => Inv'
<1>1. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) =>
       R!TypeOK' /\ R!ValidationSound' /\ R!ReplyBound'
  BY SMT DEF Inv, R!RecoveryInv, R!Inv, R!TypeOK, R!ReplyBound,
      R!ValidationSound, R!Prepared, R!BoundIntent, R!Valid, PublicationProfile
<1>2. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => R!RecoveryInv'
  BY SMT, <1>1, R!ObservationFrameInv DEF R!ObservationFrameInv, Inv,
      PublicationProfile, R!RecoveryInv, R!RecoveryTypeOK, R!RecoveryEvidence,
      R!HistoricalAuthentic, R!Inv, R!TypeOK, R!BoundIntent, R!Durable
<1>3. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => M!Inv'
  BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, PublicationProfile, M!Stutter
<1>4. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => Coupling'
  BY SMT DEF Inv, Coupling, CouplingCore, PublicationProfile, R!RecoveryInv, R!Inv, R!TypeOK
<1> QED BY SMT, <1>2, <1>3, <1>4
  DEF Inv
THEOREM ExactRecoveryBridge == \A w \in R!Writers :
  Inv /\ Replay(w) =>
    /\ p'.intent[R!OpOf(r.target[w])]=r.target[w]
    /\ ExactEnvelope(r.target[w]) = <<R!Input(m.request[w]),M!Input(m.request[w])>>
    /\ r'.checked[w]=r.target[w] /\ m'.request[w]=r.target[w]
    /\ UNCHANGED <<p.head,p.published,p.native,m.head,m.retained,m.records,m.applied>>
BY SMT DEF Replay, R!RepairCompletion, R!RepairModernCompletion,
  R!RepairLegacyCompletion, R!Inspecting, M!Replay, ExactEnvelope, Inv,
  Coupling, CouplingCore, R!RecoveryInv, R!RecoveryTypeOK, R!Inv, R!TypeOK,
  M!Inv, M!TypeOK, M!StateShape, R!BoundIntent, M!BoundIntent
THEOREM LegacyRecoveryBridge == \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
  /\ r'.target[w]=R!LegacyId /\ r'.checked[w]=R!LegacyId
  /\ r'.result[w]="already_applied" /\ m'.result[w]="historical"
  /\ p'.legacy=p.legacy /\ m'.records[0]=m.records[0]
  /\ UNCHANGED <<R!Durable,M!Durable>>
BY SMT DEF RepairLegacy, R!RepairLegacyCompletion, R!Inspecting,
  M!LegacyReplay, Inv, R!RecoveryInv, R!RecoveryTypeOK, R!Inv, R!TypeOK,
  M!Inv, M!TypeOK, M!StateShape, R!Durable, M!Durable
THEOREM RefusalFrame == \A w \in R!Writers : TypeOK /\
  (RefuseRecovery(w) \/ RefusePublication(w) \/ RefuseMigration(w)
   \/ Crash(w) \/ Restart(w)) => UNCHANGED <<R!Durable,M!Durable>>
BY SMT DEF TypeOK, M!TypeOK, M!StateShape, R!TypeOK, R!RecoveryTypeOK,
  RefuseRecovery, RefusePublication, RefuseMigration, Crash, Restart,
  Publish, R!RefuseCorruption, R!ReplayConflict, R!RefuseAuthority,
  R!PredecessorConflict, R!RefusePendingLegacy, R!RecoveryIOFailure,
  R!InputConflict, R!CASMismatch, R!RejectAdmission, R!FailIO, R!LostResponse,
  R!RecoveryCrash, R!Crash, R!RecoveryRestart, R!Restart, M!Conflict,
  M!Refuse, M!FailIO, M!Crash, M!Restart, R!Durable, M!Durable
THEOREM ForwardSelectionBridge == \A w \in R!Writers :
  Inv /\ Apply(w) /\ M!Kind(m.request[w]) \in {"selection","restoration"} =>
  /\ p'.head=m'.head /\ p'.published=m'.retained
  /\ m'.records[m'.head].corrections=m.records[m.head].corrections
  /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
BY SMT, M!ForwardSelectionFrame, M!ApplyHistoryMonotone
  DEF M!ForwardSelectionFrame, M!ApplyHistoryMonotone, Inv, Coupling,
    CouplingCore, Apply, Publish, R!DoCAS, R!CASCore, M!Apply,
    R!RecoveryInv, R!Inv, R!TypeOK, M!Inv, M!TypeOK, M!StateShape
THEOREM InvImpliesSafety == Inv => Safety
BY SMT, R!RecoveryInvImpliesSafety, M!InvImpliesSafety
  DEF R!RecoveryInvImpliesSafety, M!InvImpliesSafety, Inv, Safety
THEOREM LegacyReplayFrame == \A w \in R!Writers, b \in M!LegacyInputs :
  TypeOK /\ LegacyReplay(w,b) => UNCHANGED <<R!Durable,M!Durable>>
BY SMT, M!LegacyReplayFrame DEF M!LegacyReplayFrame, LegacyReplay, TypeOK, R!Durable
THEOREM NextPreserves == Inv /\ Next => Inv'
BY StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, ObservePublicationPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, ObserveMigrationPreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, StutterPreserves
  DEF StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, ObservePublicationPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, ObserveMigrationPreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, StutterPreserves, Next, CoreNext
THEOREM FiniteNextPreserves == Inv /\ FiniteNext => Inv'
BY StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, MigrationProfilePreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, PublicationProfilePreserves, StutterPreserves
  DEF StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, MigrationProfilePreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, PublicationProfilePreserves, StutterPreserves, FiniteNext, CoreNext
THEOREM HistoryMonotoneBridge == Inv /\ Next =>
  /\ p.published \subseteq p'.published
  /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
  /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
<1>1. Inv /\ Next => M!Next
  BY SMT DEF Inv, Next, CoreNext, Start, Admit, RetainIntent, Stage, Manifest,
    Verify, Apply, Receipt, Complete, RestoreOriginal, HistoricalRead,
    LegacyReplay, BeginLegacyRecovery, RepairLegacy, DryRun, InventoryChange,
    ObservePublication, ObserveMigration, BeginRecovery, Resume, Replay,
    ObserveCompletion, MisleadHint, DetectCorruption, RefuseRecovery,
    RefusePublication, RefuseMigration, Crash, Restart, Stutter, vars,
    M!Next, M!CoreNext, M!Stutter
<1>2. Inv /\ Next => p.published \subseteq p'.published
  BY SMT, NextPreserves, M!NextHistoryMonotone, <1>1
    DEF NextPreserves, M!NextHistoryMonotone, Inv, Coupling, CouplingCore
<1> QED BY SMT, <1>1, <1>2, M!NextHistoryMonotone
  DEF M!NextHistoryMonotone, Inv
THEOREM ProfileFrames == TypeOK /\
  ((\E f \in PublicationProfiles : PublicationProfile(f)) \/
   (\E f \in M!Profiles : MigrationProfile(f))) => UNCHANGED <<R!Durable,M!Durable>>
BY SMT DEF TypeOK, R!TypeOK, M!TypeOK, M!StateShape, PublicationProfile,
  MigrationProfile, M!ObserveProfile, R!Durable, M!Durable
MutantDropDisposition(w) == M!MutantDropDisposition(w) /\ UNCHANGED <<p,r>>
MutantRestoreOld(w) == M!MutantRestoreOld(w) /\ UNCHANGED <<p,r>>
DropDispositionNext == FiniteNext \/ \E w \in R!Writers : MutantDropDisposition(w)
RestoreOldNext == FiniteNext \/ \E w \in R!Writers : MutantRestoreOld(w)
DropDispositionSpec == Init /\ [][DropDispositionNext]_vars
RestoreOldSpec == Init /\ [][RestoreOldNext]_vars
=============================================================================
