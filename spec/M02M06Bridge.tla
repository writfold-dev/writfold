-------------------------- MODULE M02M06Bridge --------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLAPS
CONSTANTS WriterCount, InputCount, ContextCount
VARIABLES p,r,m
R == INSTANCE M02RecoveryReplay
M == INSTANCE M06MigrationSelection
(* Both component parameter domains are obligations of bridge clients,
   not unconditional facts about these arbitrary constants. *)
BridgeParameters == R!Bounds /\ M!Bounds
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
TypeOK == BridgeParameters /\ R!RecoveryTypeOK /\ R!TypeOK /\ M!TypeOK
Inv == BridgeParameters /\ R!RecoveryInv /\ M!Inv /\ IdentityMap /\ Coupling
Safety == R!RecoverySafety /\ M!Safety /\ Coupling
Init == BridgeParameters /\ R!RecoveryInit /\ M!Init
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

(* Baseline and both negative controls use <<2,8,2>>; wide uses <<3,10,3>>.
   The parameter linkage therefore leaves every admitted fixture unchanged. *)
THEOREM AdmittedFixtureParameters ==
  <<WriterCount,InputCount,ContextCount>> \in {<<2,8,2>>,<<3,10,3>>}
  => BridgeParameters
BY SMT DEF BridgeParameters, R!Bounds, M!Bounds
THEOREM IdentityMapHolds == IdentityMap
BY SMT DEF IdentityMap, R!Input, M!Input, R!OpOf, M!OpOf,
  R!BaseOf, M!BaseOf, R!SchemaOf, M!SchemaOf, R!Inputs, M!Inputs
THEOREM DomainsMatch == BridgeParameters =>
  /\ R!Writers=M!Writers /\ R!Inputs=M!Inputs
  /\ R!Releases=M!Releases /\ R!Inputs \subseteq R!ReplayInputs
  /\ R!LegacyId \in R!ReplayInputs /\ R!LegacyId \notin M!Releases
  /\ M!LegacyInput \in M!LegacyInputs
BY SMT DEF BridgeParameters, R!Bounds, M!Bounds,
  R!Writers, M!Writers, R!Inputs, M!Inputs, R!Releases,
  M!Releases, R!ReplayInputs, R!LegacyId, M!LegacyInputs
USE IdentityMapHolds, DomainsMatch DEF IdentityMapHolds, DomainsMatch
(* Reconstruct the applied-release clauses from the two actual invariants.
   Action proofs only need to establish the cross-machine core equalities. *)
THEOREM CouplingFromPostInvariants ==
  R!RecoveryInv' /\ M!Inv' /\ CouplingCore' => Coupling'
<1>1. M!Inv' =>
  /\ m'.applied = m'.retained \ {0}
  /\ (\A i \in m'.retained : m'.records[i] = M!Record(i))
  BY SMT DEF M!Inv, M!HistorySound, M!HistoryValid
<1>2. R!RecoveryInv' =>
  \A i \in p'.published \ {0} :
    /\ p'.intent[R!OpOf(i)] = i
    /\ R!Closure(i) \subseteq p'.objects
  BY SMT DEF R!RecoveryInv, R!Inv, R!PublicationChain,
    R!Prepared, R!BoundIntent
<1> QED BY SMT, <1>1, <1>2
  DEF Coupling, CouplingCore, M!Record
(* Keep the cross-machine leaves independent of the quantified history and
   accounting invariants. These projections expose only their actual inputs. *)
THEOREM SharedDomains ==
  R!Writers=M!Writers /\ R!Inputs=M!Inputs /\ R!Releases=M!Releases /\ R!Ops=M!Ops
BY DEF R!Writers, M!Writers, R!Inputs, M!Inputs, R!Releases, M!Releases, R!Ops, M!Ops
THEOREM CouplingPremises ==
  Inv => CouplingCore /\ R!TypeOK /\ R!RecoveryTypeOK /\
    M!StateShape /\ M!StagingTypes /\ M!ControlTypes /\
    R!Writers=M!Writers /\ R!Inputs=M!Inputs /\ R!Releases=M!Releases
<1>1. Inv => CouplingCore /\ R!TypeOK /\ R!RecoveryTypeOK /\
  M!StateShape /\ M!StagingTypes /\ M!ControlTypes
  BY ONLY SMT DEF Inv, Coupling, R!RecoveryInv, R!Inv, M!Inv, M!TypeOK
<1>2. QED BY <1>1, SharedDomains DEF SharedDomains
THEOREM MigrationRecordDomains == Inv =>
  m.records=[i \in M!Releases |-> m.records[i]] /\
  m.accounts=[i \in M!Inputs |-> m.accounts[i]]
<1>1. Inv => m.records=[i \in M!Releases |-> m.records[i]]
  BY ONLY SMT DEF Inv, M!Inv, M!TypeOK, M!HistoryTypes, M!HistoryTyped
<1>2. Inv => m.accounts=[i \in M!Inputs |-> m.accounts[i]]
  BY ONLY SMT DEF Inv, M!Inv, M!TypeOK, M!InventoryTypes, M!InventoryTyped
<1>3. QED BY <1>1, <1>2
THEOREM LegacyIdentitySeparate == Inv => R!LegacyId \notin M!Inputs
BY ONLY SMT DEF Inv, BridgeParameters, R!Bounds, M!Bounds, R!LegacyId, M!Inputs
THEOREM CouplingCoreFrame ==
  CouplingCore /\
  UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
    m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>> => CouplingCore'
BY ONLY SMT DEF CouplingCore
THEOREM CoreFromInv == Inv => CouplingCore
BY DEF Inv, Coupling
THEOREM MigrationControlPremises == Inv => M!StateShape /\ M!ControlTypes
BY DEF Inv, M!Inv, M!TypeOK
THEOREM PublicationTypePremises == Inv => R!TypeOK
BY DEF Inv, R!RecoveryInv, R!Inv
THEOREM RecoveryTypePremises == Inv => R!RecoveryTypeOK
BY DEF Inv, R!RecoveryInv
THEOREM ModernReplaySelected == \A w \in R!Writers :
  Inv /\ Replay(w) => R!RepairModernCompletion(w)
BY ONLY SMT, LegacyIdentitySeparate DEF LegacyIdentitySeparate,
  Replay, M!Replay, R!RepairCompletion, R!RepairLegacyCompletion, R!Inspecting
THEOREM InitPreserves == BridgeParameters => ( Init => Inv)
<1>1. ASSUME BridgeParameters
      PROVE Init => Inv
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. Init => R!RecoveryInv /\ M!Inv
    BY SMT, R!RecoveryInitPreserves, M!InitPreserves
      DEF Init, R!RecoveryInitPreserves, M!InitPreserves
  <2>2. Init => Coupling
    <3>1. Init => CouplingCore
      <4>1. Init => p.head=0 /\ p.published={0} /\ p.native={} /\
        p.manifest={} /\ p.receipt={}
        BY ONLY SMT DEF Init, R!RecoveryInit, R!Init
      <4>2. Init => m.head=0 /\ m.retained={0} /\ m.staged={} /\
        m.verified={} /\ m.receipts={}
        BY ONLY SMT DEF Init, M!Init
      <4>3. Init => p.intent=m.intent
        <5>1. Init => p.intent=[o \in R!Ops |-> 0]
          BY ONLY SMT DEF Init, R!RecoveryInit, R!Init
        <5>2. Init => m.intent=[o \in M!Ops |-> 0]
          BY ONLY SMT DEF Init, M!Init
        <5>3. QED BY <5>1, <5>2, SharedDomains DEF SharedDomains
      <4>4. QED BY SMT, <4>1, <4>2, <4>3 DEF CouplingCore
    <3>2. Init => m.applied={}
      BY ONLY SMT DEF Init, M!Init
    <3>3. QED BY <3>1, <3>2 DEF Coupling
  <2> QED BY SMT, <2>1, <2>2
    DEF Inv
<1> QED BY <1>1
THEOREM StutterPreserves == BridgeParameters => ( Inv /\ Stutter => Inv')
<1>1. ASSUME BridgeParameters
      PROVE Inv /\ Stutter => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. Inv /\ Stutter => R!RecoveryInv' /\ M!Inv'
    BY SMT, R!RecoveryStutterPreserves, M!StutterPreserves
      DEF Inv, Stutter, vars, R!RecoveryStutter, R!recoveryVars, M!Stutter,
          R!RecoveryStutterPreserves, M!StutterPreserves
  <2>2. Inv /\ Stutter => Coupling'
    BY SMT DEF Inv, Coupling, CouplingCore, Stutter, vars
  <2> QED BY <2>1, <2>2 DEF Inv
<1> QED BY <1>1
THEOREM PublicationSimulation == BridgeParameters => (
  ((\E w \in R!Writers, i \in R!Inputs : Start(w,i)) \/
   (\E w \in R!Writers : Admit(w) \/ RetainIntent(w) \/ Stage(w) \/ Manifest(w)
     \/ Verify(w) \/ Apply(w) \/ Complete(w) \/ RefusePublication(w)) \/
   (\E i \in R!Inputs : Receipt(i)) \/
   (\E i \in R!Inputs, a \in SUBSET R!FullChecks : ObservePublication(i,a)))
  => R!PublicationStep)
<1>1. ASSUME BridgeParameters
      PROVE 
        ((\E w \in R!Writers, i \in R!Inputs : Start(w,i)) \/
         (\E w \in R!Writers : Admit(w) \/ RetainIntent(w) \/ Stage(w) \/ Manifest(w)
           \/ Verify(w) \/ Apply(w) \/ Complete(w) \/ RefusePublication(w)) \/
         (\E i \in R!Inputs : Receipt(i)) \/
         (\E i \in R!Inputs, a \in SUBSET R!FullChecks : ObservePublication(i,a)))
        => R!PublicationStep
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY SMT DEF Start, Admit, RetainIntent, Stage, Manifest, Verify, Apply, Complete,
    RefusePublication, Receipt, ObservePublication, Publish,
    R!PublicationStep, R!Next, R!NonObservationNext
<1> QED BY <1>1
THEOREM StartPreserves == BridgeParameters => ( \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Start, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => M!Inv'
    BY SMT, M!StartPreserves DEF M!StartPreserves, Inv, Start, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) => CouplingCore'
    <3>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ Start(w,i) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, CouplingPremises DEF CouplingPremises, Start, Publish,
        R!Start, M!Start, R!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RetainIntentPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RetainIntent(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RetainIntent(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RetainIntent(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, RetainIntent, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RetainIntent(w) => M!Inv'
    BY SMT, M!RetainIntentPreserves DEF M!RetainIntentPreserves, Inv, RetainIntent, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RetainIntent(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, RetainIntent, R!RetainIntent, M!RetainIntent,
      Publish, R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape,
      M!StageReady, M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes,
      M!ControlTyped, R!OpOf, M!OpOf
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM StagePreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Stage(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Stage(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Stage(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Stage, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Stage(w) => M!Inv'
    BY SMT, M!StagePreserves DEF M!StagePreserves, Inv, Stage, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Stage(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises, MigrationRecordDomains
      DEF CouplingPremises, MigrationRecordDomains, CouplingCore, Stage, R!NativeCommit, M!Stage,
      Publish, R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape,
      M!StageReady, M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes,
      M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM VerifyPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Verify(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Verify(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Verify(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Verify, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Verify(w) => M!Inv'
    BY SMT, M!VerifyStagePreserves DEF M!VerifyStagePreserves, Inv, Verify, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Verify(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, Verify, R!FinalRevalidate, M!VerifyStage,
      Publish, R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape,
      M!StageReady, M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes,
      M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ApplyPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Apply(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Apply(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Apply(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Apply, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Apply(w) => M!Inv'
    BY SMT, M!ApplyPreserves DEF M!ApplyPreserves, Inv, Apply, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Apply(w) => CouplingCore'
    <3>1. \A w \in R!Writers : Inv /\ Apply(w) =>
      p'.head=m'.head /\ p'.published=m'.retained
      BY ONLY SMT, CoreFromInv, PublicationTypePremises, MigrationControlPremises,
        MigrationRecordDomains, SharedDomains
        DEF CoreFromInv, PublicationTypePremises, MigrationControlPremises,
          MigrationRecordDomains, SharedDomains, CouplingCore,
          Apply, Publish, R!DoCAS, R!CASCore, M!Apply, R!TypeOK,
          M!StateShape, M!ControlTypes, M!ControlTyped
    <3>2. \A w \in R!Writers : Inv /\ Apply(w) =>
      UNCHANGED <<p.intent,p.native,p.manifest,p.receipt,
        m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, PublicationTypePremises, MigrationControlPremises,
        MigrationRecordDomains, SharedDomains
        DEF PublicationTypePremises, MigrationControlPremises,
          MigrationRecordDomains, SharedDomains, Apply, Publish,
          R!DoCAS, R!CASCore, M!Apply, R!TypeOK,
          M!StateShape, M!ControlTypes, M!ControlTyped
    <3>3. QED BY SMT, <3>1, <3>2, CoreFromInv DEF CoreFromInv, CouplingCore
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM CompletePreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Complete(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Complete(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Complete(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Complete, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Complete(w) => M!Inv'
    BY SMT, M!RecordReceiptPreserves DEF M!RecordReceiptPreserves, Inv, Complete, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Complete(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, Complete, R!Finish, M!RecordReceipt,
      Publish, R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape,
      M!StageReady, M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes,
      M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM AdmitPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Admit(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Admit(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Admit(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Admit, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Admit(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Admit, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Admit(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, Admit, R!Admit, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ManifestPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Manifest(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Manifest(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Manifest(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Manifest, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Manifest(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Manifest, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Manifest(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, Manifest, R!RetainManifest, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ReceiptPreserves == BridgeParameters => ( \A i \in R!Inputs : Inv /\ Receipt(i) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A i \in R!Inputs : Inv /\ Receipt(i) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A i \in R!Inputs : Inv /\ Receipt(i) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, Receipt, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A i \in R!Inputs : Inv /\ Receipt(i) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, Receipt, M!Stutter, M!BoundIntent
  <2>3. \A i \in R!Inputs : Inv /\ Receipt(i) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, Receipt, R!RecordReceipt, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ObservePublicationPreserves == BridgeParameters => ( \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, ObservePublication, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, ObservePublication, M!Stutter, M!BoundIntent
  <2>3. \A i \in R!Inputs, a \in SUBSET R!FullChecks : Inv /\ ObservePublication(i,a) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, ObservePublication, R!Observe, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RestoreOriginalPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RestoreOriginal(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RestoreOriginal(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, RestoreOriginal, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => M!Inv'
    BY SMT, M!RestoreOriginalPreserves DEF M!RestoreOriginalPreserves, Inv, RestoreOriginal, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RestoreOriginal(w) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, RestoreOriginal, M!RestoreOriginal, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM HistoricalReadPreserves == BridgeParameters => ( \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, HistoricalRead, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => M!Inv'
    BY SMT, M!HistoricalReadPreserves DEF M!HistoricalReadPreserves, Inv, HistoricalRead, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers, i \in R!Releases : Inv /\ HistoricalRead(w,i) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, HistoricalRead, M!HistoricalRead, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM LegacyReplayPreserves == BridgeParameters => ( \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, LegacyReplay, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => M!Inv'
    BY SMT, M!LegacyReplayPreserves DEF M!LegacyReplayPreserves, Inv, LegacyReplay, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) => CouplingCore'
    <3>1. \A w \in R!Writers, b \in M!LegacyInputs : Inv /\ LegacyReplay(w,b) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, CouplingPremises DEF CouplingPremises, LegacyReplay,
        M!LegacyReplay, M!StateShape, M!ControlTypes, M!ControlTyped
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM DryRunPreserves == BridgeParameters => ( Inv /\ DryRun => Inv')
<1>1. ASSUME BridgeParameters
      PROVE Inv /\ DryRun => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. Inv /\ DryRun => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, DryRun, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. Inv /\ DryRun => M!Inv'
    BY SMT, M!DryRunPreserves DEF M!DryRunPreserves, Inv, DryRun, M!Stutter, M!BoundIntent
  <2>3. Inv /\ DryRun => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, DryRun, M!DryRun, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM InventoryChangePreserves == BridgeParameters => ( \A v \in {0,1} : Inv /\ InventoryChange(v) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A v \in {0,1} : Inv /\ InventoryChange(v) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A v \in {0,1} : Inv /\ InventoryChange(v) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, InventoryChange, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A v \in {0,1} : Inv /\ InventoryChange(v) => M!Inv'
    BY SMT, M!InventoryChangePreserves DEF M!InventoryChangePreserves, Inv, InventoryChange, M!Stutter, M!BoundIntent
  <2>3. \A v \in {0,1} : Inv /\ InventoryChange(v) => CouplingCore'
    BY ONLY SMT, CoreFromInv, MigrationControlPremises
      DEF CoreFromInv, MigrationControlPremises, CouplingCore, InventoryChange,
        M!InventoryChange, M!StateShape
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ObserveMigrationPreserves == BridgeParameters => ( \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, ObserveMigration, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => M!Inv'
    BY SMT, M!ObservePreserves DEF M!ObservePreserves, Inv, ObserveMigration, M!Stutter, M!BoundIntent
  <2>3. \A i \in M!Inputs, a \in SUBSET M!Checks : Inv /\ ObserveMigration(i,a) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, ObserveMigration, M!Observe, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM MigrationProfilePreserves == BridgeParameters => ( \A f \in M!Profiles : Inv /\ MigrationProfile(f) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A f \in M!Profiles : Inv /\ MigrationProfile(f) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, MigrationProfile, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => M!Inv'
    BY SMT, M!ObserveProfilePreserves DEF M!ObserveProfilePreserves, Inv, MigrationProfile, M!Stutter, M!BoundIntent
  <2>3. \A f \in M!Profiles : Inv /\ MigrationProfile(f) => CouplingCore'
    BY ONLY SMT, CoreFromInv, MigrationControlPremises
      DEF CoreFromInv, MigrationControlPremises, CouplingCore, MigrationProfile,
        M!ObserveProfile, M!StateShape
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM BeginRecoveryPreserves == BridgeParameters => ( \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => R!RecoveryInv'
    BY SMT, R!RecoverStartPreserves DEF R!RecoverStartPreserves, Inv, BeginRecovery, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => M!Inv'
    BY SMT, M!StartPreserves DEF M!StartPreserves, Inv, BeginRecovery, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) => CouplingCore'
    <3>1. \A w \in R!Writers, i \in R!Inputs : Inv /\ BeginRecovery(w,i) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, MigrationControlPremises, SharedDomains
        DEF MigrationControlPremises, SharedDomains, BeginRecovery,
          R!RecoverStart, M!Start, M!StateShape, M!ControlTypes, M!ControlTyped
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM BeginLegacyRecoveryPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => R!RecoveryInv'
    BY SMT, R!RecoverStartPreserves DEF R!RecoverStartPreserves, Inv, BeginLegacyRecovery, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, BeginLegacyRecovery, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ BeginLegacyRecovery(w) => CouplingCore'
    <3>1. \A w \in R!Writers : BeginLegacyRecovery(w) => UNCHANGED <<p,m>>
      BY DEF BeginLegacyRecovery, R!RecoverStart
    <3>2. QED BY SMT, <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RepairLegacyPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RepairLegacy(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RepairLegacy(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RepairLegacy(w) => R!RecoveryInv'
    BY SMT, R!RepairLegacyCompletionPreserves DEF R!RepairLegacyCompletionPreserves, Inv, RepairLegacy, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RepairLegacy(w) => M!Inv'
    BY SMT, M!LegacyReplayPreserves DEF M!LegacyReplayPreserves, Inv, RepairLegacy, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RepairLegacy(w) => CouplingCore'
    <3>1. \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, CouplingPremises DEF CouplingPremises, RepairLegacy,
        R!RepairLegacyCompletion, M!LegacyReplay, M!StateShape,
        M!ControlTypes, M!ControlTyped
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ResumePreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Resume(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Resume(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Resume(w) => R!RecoveryInv'
    BY SMT, R!ResumeExactPreserves DEF R!ResumeExactPreserves, Inv, Resume, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Resume(w) => M!Inv'
    BY SMT, M!RecoverPreserves, M!StutterPreserves DEF M!RecoverPreserves, M!StutterPreserves, Inv, Resume, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Resume(w) => CouplingCore'
    <3>1. \A w \in R!Writers : Inv /\ Resume(w) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, PublicationTypePremises, MigrationControlPremises, SharedDomains
        DEF PublicationTypePremises, MigrationControlPremises, SharedDomains,
          Resume, R!ResumeExact, M!Recover, R!TypeOK, M!StateShape,
          M!ControlTypes, M!ControlTyped
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ReplayPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Replay(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Replay(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Replay(w) => R!RecoveryInv'
    BY SMT, R!RepairCompletionPreserves DEF R!RepairCompletionPreserves, Inv, Replay, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Replay(w) => M!Inv'
    BY SMT, M!ReplayPreserves DEF M!ReplayPreserves, Inv, Replay, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Replay(w) => CouplingCore'
    BY ONLY SMT, CoreFromInv, PublicationTypePremises, MigrationControlPremises,
      SharedDomains, ModernReplaySelected
      DEF CoreFromInv, PublicationTypePremises, MigrationControlPremises,
        SharedDomains, ModernReplaySelected, CouplingCore, Replay,
        R!RepairModernCompletion, M!Replay, R!TypeOK,
        M!StateShape, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ObserveCompletionPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ ObserveCompletion(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ ObserveCompletion(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => R!RecoveryInv'
    BY SMT, R!ObserveCompletionPreserves DEF R!ObserveCompletionPreserves, Inv, ObserveCompletion, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, ObserveCompletion, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ ObserveCompletion(w) => CouplingCore'
    <3>1. \A w \in R!Writers : ObserveCompletion(w) => UNCHANGED <<p,m>>
      BY DEF ObserveCompletion, R!ObserveCompletion
    <3>2. QED BY SMT, <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM MisleadHintPreserves == BridgeParameters => ( \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => R!RecoveryInv'
    BY SMT, R!MisleadHintPreserves DEF R!MisleadHintPreserves, Inv, MisleadHint, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, MisleadHint, M!Stutter, M!BoundIntent
  <2>3. \A o \in R!Ops, i \in R!Releases, h \in R!HintPhases : Inv /\ MisleadHint(o,i,h) => CouplingCore'
    BY ONLY SMT, CoreFromInv
      DEF CoreFromInv, CouplingCore, MisleadHint, R!MisleadHint
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM DetectCorruptionPreserves == BridgeParameters => ( \A i \in R!Releases : Inv /\ DetectCorruption(i) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A i \in R!Releases : Inv /\ DetectCorruption(i) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A i \in R!Releases : Inv /\ DetectCorruption(i) => R!RecoveryInv'
    BY SMT, R!DetectCorruptionPreserves DEF R!DetectCorruptionPreserves, Inv, DetectCorruption, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A i \in R!Releases : Inv /\ DetectCorruption(i) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, DetectCorruption, M!Stutter, M!BoundIntent
  <2>3. \A i \in R!Releases : Inv /\ DetectCorruption(i) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore, DetectCorruption, R!DetectCorruption, Publish,
      R!TypeOK, R!Prepared, R!BoundIntent, M!StateShape, M!StageReady,
      M!BoundIntent, M!StagingTypes, M!StagingTyped, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM CrashPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Crash(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Crash(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Crash(w) => R!RecoveryInv'
    BY SMT, R!RecoveryCrashPreserves DEF R!RecoveryCrashPreserves, Inv, Crash, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Crash(w) => M!Inv'
    BY SMT, M!CrashPreserves DEF M!CrashPreserves, Inv, Crash, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Crash(w) => CouplingCore'
    BY ONLY SMT, CoreFromInv, PublicationTypePremises, MigrationControlPremises, SharedDomains
      DEF CoreFromInv, PublicationTypePremises, MigrationControlPremises, SharedDomains,
        CouplingCore, Crash, R!RecoveryCrash, R!Crash, M!Crash,
        R!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RestartPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ Restart(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ Restart(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Restart(w) => R!RecoveryInv'
    BY SMT, R!RecoveryRestartPreserves DEF R!RecoveryRestartPreserves, Inv, Restart, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ Restart(w) => M!Inv'
    BY SMT, M!RestartPreserves DEF M!RestartPreserves, Inv, Restart, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ Restart(w) => CouplingCore'
    BY ONLY SMT, CoreFromInv, PublicationTypePremises, MigrationControlPremises, SharedDomains
      DEF CoreFromInv, PublicationTypePremises, MigrationControlPremises, SharedDomains,
        CouplingCore, Restart, R!RecoveryRestart, R!Restart, M!Restart,
        R!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RefuseRecoveryPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RefuseRecovery(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RefuseRecovery(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => R!RecoveryInv'
    BY SMT, R!RefuseCorruptionPreserves, R!ReplayConflictPreserves, R!RefuseAuthorityPreserves, R!PredecessorConflictPreserves, R!RefusePendingLegacyPreserves, R!RecoveryIOFailurePreserves DEF R!RefuseCorruptionPreserves, R!ReplayConflictPreserves, R!RefuseAuthorityPreserves, R!PredecessorConflictPreserves, R!RefusePendingLegacyPreserves, R!RecoveryIOFailurePreserves, Inv, RefuseRecovery, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, RefuseRecovery, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RefuseRecovery(w) => CouplingCore'
    <3>1. \A w \in R!Writers : RefuseRecovery(w) => UNCHANGED <<p,m>>
      BY DEF RefuseRecovery, R!RefuseCorruption, R!ReplayConflict,
        R!RefuseAuthority, R!PredecessorConflict, R!RefusePendingLegacy, R!RecoveryIOFailure
    <3>2. QED BY SMT, <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RefusePublicationPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RefusePublication(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RefusePublication(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RefusePublication(w) => R!RecoveryInv'
    BY SMT, PublicationSimulation, R!PublicationStepPreserves DEF PublicationSimulation, R!PublicationStepPreserves, Inv, RefusePublication, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RefusePublication(w) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, RefusePublication, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RefusePublication(w) => CouplingCore'
    <3>1. \A w \in R!Writers : Inv /\ RefusePublication(w) =>
      UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.receipt,
        m.head,m.retained,m.intent,m.staged,m.verified,m.receipts>>
      BY ONLY SMT, CouplingPremises DEF CouplingPremises, RefusePublication, Publish,
        R!InputConflict, R!CASMismatch, R!RejectAdmission, R!FailIO, R!LostResponse, R!TypeOK
    <3>2. QED BY <3>1, CouplingPremises, CouplingCoreFrame
      DEF CouplingPremises, CouplingCoreFrame
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM RefuseMigrationPreserves == BridgeParameters => ( \A w \in R!Writers : Inv /\ RefuseMigration(w) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RefuseMigration(w) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RefuseMigration(w) => R!RecoveryInv'
    BY SMT, R!RecoveryStutterPreserves DEF R!RecoveryStutterPreserves, Inv, RefuseMigration, Publish, R!RecoveryStutter, R!recoveryVars
  <2>2. \A w \in R!Writers : Inv /\ RefuseMigration(w) => M!Inv'
    BY SMT, M!ConflictPreserves, M!RefusePreserves, M!FailIOPreserves DEF M!ConflictPreserves, M!RefusePreserves, M!FailIOPreserves, Inv, RefuseMigration, M!Stutter, M!BoundIntent
  <2>3. \A w \in R!Writers : Inv /\ RefuseMigration(w) => CouplingCore'
    BY ONLY SMT, CoreFromInv, MigrationControlPremises, SharedDomains
      DEF CoreFromInv, MigrationControlPremises, SharedDomains, CouplingCore,
        RefuseMigration, M!Conflict, M!Refuse, M!FailIO,
        M!StateShape, M!ControlTypes, M!ControlTyped
  <2> QED BY SMT, <2>1, <2>2, <2>3, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM PublicationProfilePreserves == BridgeParameters => ( \A f \in PublicationProfiles :
  Inv /\ PublicationProfile(f) => Inv')
<1>1. ASSUME BridgeParameters
      PROVE \A f \in PublicationProfiles :
        Inv /\ PublicationProfile(f) => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) =>
         R!TypeOK' /\ R!ValidationSound' /\ R!ReplyBound'
    BY SMT DEF Inv, R!RecoveryInv, R!Inv, R!TypeOK, R!ReplyBound,
        R!ValidationSound, R!Prepared, R!BoundIntent, R!Valid, PublicationProfile
  <2>2. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => R!RecoveryInv'
    <3>1. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => R!Inv'
      BY SMT, <2>1, R!ObservationFrameInv
        DEF R!ObservationFrameInv, Inv, R!RecoveryInv, R!Inv, R!TypeOK,
          PublicationProfile, R!Durable
    <3>2. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) =>
      UNCHANGED <<r,p.receipt,p.intent>>
      BY SMT DEF Inv, R!RecoveryInv, R!Inv, R!TypeOK, PublicationProfile
    <3>3. QED BY <3>1, <3>2, R!RecoveryEvidenceFrame
      DEF R!RecoveryEvidenceFrame, Inv
  <2>3. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => M!Inv'
    BY SMT, M!StutterPreserves DEF M!StutterPreserves, Inv, PublicationProfile, M!Stutter
  <2>4. \A f \in PublicationProfiles : Inv /\ PublicationProfile(f) => CouplingCore'
    BY ONLY SMT, CouplingPremises DEF CouplingPremises, CouplingCore,
      PublicationProfile, R!TypeOK
  <2> QED BY SMT, <2>2, <2>3, <2>4, CouplingFromPostInvariants
    DEF Inv, CouplingFromPostInvariants
<1> QED BY <1>1
THEOREM ExactRecoveryBridge == BridgeParameters => ( \A w \in R!Writers :
  Inv /\ Replay(w) =>
    /\ p'.intent[R!OpOf(r.target[w])]=r.target[w]
    /\ ExactEnvelope(r.target[w]) = <<R!Input(m.request[w]),M!Input(m.request[w])>>
    /\ r'.checked[w]=r.target[w] /\ m'.request[w]=r.target[w]
    /\ UNCHANGED <<p.head,p.published,p.native,m.head,m.retained,m.records,m.applied>>)
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers :
        Inv /\ Replay(w) =>
          /\ p'.intent[R!OpOf(r.target[w])]=r.target[w]
          /\ ExactEnvelope(r.target[w]) = <<R!Input(m.request[w]),M!Input(m.request[w])>>
          /\ r'.checked[w]=r.target[w] /\ m'.request[w]=r.target[w]
          /\ UNCHANGED <<p.head,p.published,p.native,m.head,m.retained,m.records,m.applied>>
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Replay(w) =>
    /\ p'.intent[R!OpOf(r.target[w])]=r.target[w]
    /\ r'.checked[w]=r.target[w] /\ m'.request[w]=r.target[w]
    BY ONLY SMT, PublicationTypePremises, RecoveryTypePremises, MigrationControlPremises,
      SharedDomains, ModernReplaySelected
      DEF PublicationTypePremises, RecoveryTypePremises, MigrationControlPremises,
        SharedDomains, ModernReplaySelected, Replay, R!RepairModernCompletion, M!Replay,
        R!RecoveryTypeOK, R!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped, R!BoundIntent
  <2>2. \A w \in R!Writers : Inv /\ Replay(w) =>
    ExactEnvelope(r.target[w]) = <<R!Input(m.request[w]),M!Input(m.request[w])>>
    BY SMT DEF Replay, ExactEnvelope
  <2>3. \A w \in R!Writers : Inv /\ Replay(w) =>
    UNCHANGED <<p.head,p.published,p.native,m.head,m.retained,m.records,m.applied>>
    BY ONLY SMT, PublicationTypePremises, MigrationControlPremises, SharedDomains
      DEF PublicationTypePremises, MigrationControlPremises, SharedDomains,
        Replay, R!RepairCompletion, R!RepairModernCompletion, R!RepairLegacyCompletion,
        M!Replay, R!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped
  <2> QED BY <2>1, <2>2, <2>3
<1> QED BY <1>1
THEOREM LegacyRecoveryBridge == BridgeParameters => ( \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
  /\ r'.target[w]=R!LegacyId /\ r'.checked[w]=R!LegacyId
  /\ r'.result[w]="already_applied" /\ m'.result[w]="historical"
  /\ p'.legacy=p.legacy /\ m'.records[0]=m.records[0]
  /\ UNCHANGED <<R!Durable,M!Durable>>)
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
        /\ r'.target[w]=R!LegacyId /\ r'.checked[w]=R!LegacyId
        /\ r'.result[w]="already_applied" /\ m'.result[w]="historical"
        /\ p'.legacy=p.legacy /\ m'.records[0]=m.records[0]
        /\ UNCHANGED <<R!Durable,M!Durable>>
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
    /\ r'.target[w]=R!LegacyId /\ r'.checked[w]=R!LegacyId
    /\ r'.result[w]="already_applied" /\ m'.result[w]="historical"
    BY ONLY SMT, RecoveryTypePremises, MigrationControlPremises, SharedDomains
      DEF RecoveryTypePremises, MigrationControlPremises, SharedDomains,
        RepairLegacy, R!RepairLegacyCompletion, R!Inspecting, M!LegacyReplay,
        R!RecoveryTypeOK, M!StateShape, M!ControlTypes, M!ControlTyped
  <2>2. \A w \in R!Writers : Inv /\ RepairLegacy(w) =>
    /\ p'.legacy=p.legacy /\ m'.records[0]=m.records[0]
    /\ UNCHANGED <<R!Durable,M!Durable>>
    BY ONLY SMT, MigrationControlPremises, SharedDomains
      DEF MigrationControlPremises, SharedDomains, RepairLegacy,
        R!RepairLegacyCompletion, M!LegacyReplay, M!StateShape,
        M!ControlTypes, M!ControlTyped, R!Durable, M!Durable
  <2> QED BY <2>1, <2>2
<1> QED BY <1>1
THEOREM RefusalFrame == BridgeParameters => ( \A w \in R!Writers : TypeOK /\
  (RefuseRecovery(w) \/ RefusePublication(w) \/ RefuseMigration(w)
   \/ Crash(w) \/ Restart(w)) => UNCHANGED <<R!Durable,M!Durable>>)
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers : TypeOK /\
        (RefuseRecovery(w) \/ RefusePublication(w) \/ RefuseMigration(w)
         \/ Crash(w) \/ Restart(w)) => UNCHANGED <<R!Durable,M!Durable>>
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : TypeOK /\
    (RefuseRecovery(w) \/ RefusePublication(w) \/ RefuseMigration(w)
     \/ Crash(w) \/ Restart(w)) => UNCHANGED R!Durable
    <3>1. \A w \in R!Writers : RefuseRecovery(w) \/ RefuseMigration(w) => UNCHANGED p
      BY DEF RefuseRecovery, RefuseMigration, R!RefuseCorruption, R!ReplayConflict,
        R!RefuseAuthority, R!PredecessorConflict, R!RefusePendingLegacy, R!RecoveryIOFailure
    <3>2. \A w \in R!Writers : TypeOK /\ RefusePublication(w) => UNCHANGED R!Durable
      BY ONLY SMT DEF TypeOK, R!TypeOK, RefusePublication, Publish,
        R!InputConflict, R!CASMismatch, R!RejectAdmission, R!FailIO, R!LostResponse, R!Durable
    <3>3. \A w \in R!Writers : TypeOK /\ (Crash(w) \/ Restart(w)) => UNCHANGED R!Durable
      BY ONLY SMT DEF TypeOK, R!TypeOK, Crash, Restart,
        R!RecoveryCrash, R!Crash, R!RecoveryRestart, R!Restart, R!Durable
    <3>4. QED BY <3>1, <3>2, <3>3 DEF R!Durable
  <2>2. \A w \in R!Writers : TypeOK /\
    (RefuseRecovery(w) \/ RefusePublication(w) \/ RefuseMigration(w)
     \/ Crash(w) \/ Restart(w)) => UNCHANGED M!Durable
    BY SMT DEF TypeOK, M!TypeOK, M!StateShape, M!ControlTypes, M!ControlTyped,
      RefuseRecovery, RefusePublication, RefuseMigration, Crash, Restart,
      M!Conflict, M!Refuse, M!FailIO, M!Crash, M!Restart, M!Durable
  <2> QED BY <2>1, <2>2
<1> QED BY <1>1
THEOREM ForwardSelectionBridge == BridgeParameters => ( \A w \in R!Writers :
  Inv /\ Apply(w) /\ M!Kind(m.request[w]) \in {"selection","restoration"} =>
  /\ p'.head=m'.head /\ p'.published=m'.retained
  /\ m'.records[m'.head].corrections=m.records[m.head].corrections
  /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history))
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers :
        Inv /\ Apply(w) /\ M!Kind(m.request[w]) \in {"selection","restoration"} =>
        /\ p'.head=m'.head /\ p'.published=m'.retained
        /\ m'.records[m'.head].corrections=m.records[m.head].corrections
        /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. \A w \in R!Writers : Inv /\ Apply(w) =>
    /\ p'.head=m'.head /\ p'.published=m'.retained
    BY SMT, ApplyPreserves DEF ApplyPreserves, Inv, Coupling, CouplingCore
  <2>2. \A w \in R!Writers : Inv /\ Apply(w) =>
    m'.head=m.request[w]
    BY SMT DEF Inv, M!Inv, M!TypeOK, M!StateShape, Apply, M!Apply,
      M!ControlTypes, M!ControlTyped
  <2>3. \A w \in R!Writers :
    Inv /\ Apply(w) /\ M!Kind(m.request[w]) \in {"selection","restoration"} =>
    m'.records[m'.head].corrections=m.records[m.head].corrections
    BY SMT, <2>2, M!ForwardSelectionFrame
      DEF M!ForwardSelectionFrame, Inv, Apply
  <2>4. \A w \in R!Writers : Inv /\ Apply(w) =>
    M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
    BY SMT, M!ApplyHistoryMonotone DEF M!ApplyHistoryMonotone, Inv, Apply
  <2> QED BY <2>1, <2>3, <2>4
<1> QED BY <1>1
THEOREM InvImpliesSafety == BridgeParameters => ( Inv => Safety)
<1>1. ASSUME BridgeParameters
      PROVE Inv => Safety
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY SMT, R!RecoveryInvImpliesSafety, M!InvImpliesSafety
    DEF R!RecoveryInvImpliesSafety, M!InvImpliesSafety, Inv, Safety
<1> QED BY <1>1
THEOREM LegacyReplayFrame == BridgeParameters => ( \A w \in R!Writers, b \in M!LegacyInputs :
  TypeOK /\ LegacyReplay(w,b) => UNCHANGED <<R!Durable,M!Durable>>)
<1>1. ASSUME BridgeParameters
      PROVE \A w \in R!Writers, b \in M!LegacyInputs :
        TypeOK /\ LegacyReplay(w,b) => UNCHANGED <<R!Durable,M!Durable>>
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY SMT, M!LegacyReplayFrame DEF M!LegacyReplayFrame, LegacyReplay, TypeOK, R!Durable
<1> QED BY <1>1
THEOREM NextPreserves == BridgeParameters => ( Inv /\ Next => Inv')
<1>1. ASSUME BridgeParameters
      PROVE Inv /\ Next => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, ObservePublicationPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, ObserveMigrationPreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, StutterPreserves
    DEF StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, ObservePublicationPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, ObserveMigrationPreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, StutterPreserves, Next, CoreNext
<1> QED BY <1>1
(* Fixture domains are arithmetic facts, not additional restrictions on Next.
   Expose them before instantiating the unrestricted hint/corruption lemmas. *)
THEOREM FiniteFixtureDomains == BridgeParameters => (
  /\ 1 \in R!Ops
  /\ {0,1,2} \subseteq R!Releases
  /\ {"none","published"} \subseteq R!HintPhases
  /\ {0,1,2,5} \subseteq R!Releases)
<1>1. ASSUME BridgeParameters
      PROVE 
        /\ 1 \in R!Ops
        /\ {0,1,2} \subseteq R!Releases
        /\ {"none","published"} \subseteq R!HintPhases
        /\ {0,1,2,5} \subseteq R!Releases
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY SMT, M!ParameterBounds
    DEF M!Bounds, R!Ops, R!Releases, R!HintPhases
<1> QED BY <1>1
USE FiniteFixtureDomains DEF FiniteFixtureDomains
THEOREM FiniteNextPreserves == BridgeParameters => ( Inv /\ FiniteNext => Inv')
<1>1. ASSUME BridgeParameters
      PROVE Inv /\ FiniteNext => Inv'
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, MigrationProfilePreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, PublicationProfilePreserves, StutterPreserves
    DEF StartPreserves, RetainIntentPreserves, StagePreserves, VerifyPreserves, ApplyPreserves, CompletePreserves, AdmitPreserves, ManifestPreserves, ReceiptPreserves, RestoreOriginalPreserves, HistoricalReadPreserves, LegacyReplayPreserves, DryRunPreserves, InventoryChangePreserves, MigrationProfilePreserves, BeginRecoveryPreserves, BeginLegacyRecoveryPreserves, RepairLegacyPreserves, ResumePreserves, ReplayPreserves, ObserveCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, CrashPreserves, RestartPreserves, RefuseRecoveryPreserves, RefusePublicationPreserves, RefuseMigrationPreserves, PublicationProfilePreserves, StutterPreserves, FiniteNext, CoreNext
<1> QED BY <1>1
THEOREM HistoryMonotoneBridge == BridgeParameters => ( Inv /\ Next =>
  /\ p.published \subseteq p'.published
  /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
  /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections)
<1>1. ASSUME BridgeParameters
      PROVE Inv /\ Next =>
        /\ p.published \subseteq p'.published
        /\ M!Prefix(m.records[m.head].history,m'.records[m'.head].history)
        /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2>1. Inv /\ Next => M!Next
    BY SMT DEF Inv, Next, CoreNext, Start, Admit, RetainIntent, Stage, Manifest,
      Verify, Apply, Receipt, Complete, RestoreOriginal, HistoricalRead,
      LegacyReplay, BeginLegacyRecovery, RepairLegacy, DryRun, InventoryChange,
      ObservePublication, ObserveMigration, BeginRecovery, Resume, Replay,
      ObserveCompletion, MisleadHint, DetectCorruption, RefuseRecovery,
      RefusePublication, RefuseMigration, Crash, Restart, Stutter, vars,
      M!Next, M!CoreNext, M!Stutter
  <2>2. Inv /\ Next => p.published \subseteq p'.published
    BY SMT, NextPreserves, M!NextHistoryMonotone, <2>1
      DEF NextPreserves, M!NextHistoryMonotone, Inv, Coupling, CouplingCore
  <2> QED BY SMT, <2>1, <2>2, M!NextHistoryMonotone
    DEF M!NextHistoryMonotone, Inv
<1> QED BY <1>1
THEOREM ProfileFrames == BridgeParameters => ( TypeOK /\
  ((\E f \in PublicationProfiles : PublicationProfile(f)) \/
   (\E f \in M!Profiles : MigrationProfile(f))) => UNCHANGED <<R!Durable,M!Durable>>)
<1>1. ASSUME BridgeParameters
      PROVE TypeOK /\
        ((\E f \in PublicationProfiles : PublicationProfile(f)) \/
         (\E f \in M!Profiles : MigrationProfile(f))) => UNCHANGED <<R!Durable,M!Durable>>
  <2>bounds. R!ParameterBounds /\ M!ParameterBounds /\ R!Bounds /\ M!Bounds
    BY SMT, <1>1 DEF BridgeParameters, R!ParameterBounds, M!ParameterBounds,
      R!Bounds, M!Bounds
  <2> USE <1>1, <2>bounds DEF BridgeParameters, R!Bounds, M!Bounds
  <2> QED BY SMT DEF TypeOK, R!TypeOK, M!TypeOK, M!StateShape, PublicationProfile,
    MigrationProfile, M!ObserveProfile, R!Durable, M!Durable
<1> QED BY <1>1
MutantDropDisposition(w) == M!MutantDropDisposition(w) /\ UNCHANGED <<p,r>>
MutantRestoreOld(w) == M!MutantRestoreOld(w) /\ UNCHANGED <<p,r>>
DropDispositionNext == FiniteNext \/ \E w \in R!Writers : MutantDropDisposition(w)
RestoreOldNext == FiniteNext \/ \E w \in R!Writers : MutantRestoreOld(w)
DropDispositionSpec == Init /\ [][DropDispositionNext]_vars
RestoreOldSpec == Init /\ [][RestoreOldNext]_vars
=============================================================================
