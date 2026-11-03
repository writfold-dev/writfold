-------------------- MODULE M09AuthoredDesignLifecycle --------------------
EXTENDS Naturals, FiniteSets, TLAPS

(***************************************************************************
Runtime mapping/premises (not Rust refinement): revision tokens denote exact
lossless authored bytes and proposal revisions; design-v3 identity denotes the
normative material projection, not bookkeeping. ResolveTarget represents a
trusted adapter observation of a real source/revision, never a caller Boolean.
The stable managed-path lock survives rename of the data file. Conditional
write is atomic under that lock and compares BOTH exact revisions. Durable
intent and replacement require the qualified file+directory fsync contract;
process crash preserves acknowledged bytes. Host power-loss is not inferred.
Authored actor/time/reason are provenance, never Ed25519 knowledge approval.
This module changes no Git ref. Retained actual bytes permit receipt repair
when a later authored mutation has already replaced the current file.

Finite representatives: two/three actors, twelve exact requests, two targets,
three observation times, four initial lifecycle/migration cases. Profiles can
fail different checks simultaneously on competing inputs. No error-count or
phase bound is imposed. Inductive contracts quantify all declared counts and
all profiles, not a TLC exploration constraint. No fairness/liveness claim.
***************************************************************************)
CONSTANT ActorCount
ActorBounds == ActorCount \in Nat /\ ActorCount >= 2
ASSUME ActorUniverse == ActorBounds
Actors == 1..ActorCount
USE ActorUniverse DEF ActorUniverse, ActorBounds, Actors
Requests == 1..12
Revisions == 0..12
Ids == 1..11
Kinds == {"bind","reopen","land","apply","verify","adopt","migrate-reopen","supersede"}
Origins == {"accepted","legacy-proposed","legacy-accepted","legacy-applied"}
Phases == {"idle","planned","locked","intent","written","inspect","done","down"}
Results == {"none","pending","complete","conflict","refused","unknown"}
Checks == {"design","source","resolved","time","readiness","actor","scope","environment","gate"}
Profiles == { {}, {<<1,"source">>}, {<<1,"design">>,<<2,"time">>},
  {<<1,"resolved">>,<<2,"actor">>}, {<<5,"gate">>,<<6,"readiness">>},
  {<<1,"time">>,<<5,"scope">>,<<6,"environment">>} }
VARIABLE a
vars == <<a>>
OpOf(i) == IF i <= 2 THEN 1 ELSE i-1
BaseOf(i) == CASE i \in {1,2,10,11,12} -> 0 [] i \in {3,7,9} -> 1
  [] i \in {4,5,8} -> 3 [] OTHER -> 5
KindOf(i) == CASE i \in {1,4,8,9} -> "bind" [] i \in {2,7} -> "reopen"
  [] i=3 -> "land" [] i=5 -> "apply" [] i=6 -> "verify"
  [] i=10 -> "adopt" [] i=11 -> "migrate-reopen" [] OTHER -> "supersede"
ActorOf(i) == IF i \in {2,6,7,11} THEN 2 ELSE 1
TargetOf(i) == IF i=8 THEN "target-B" ELSE "target-A"
Input(i) == [namespace |-> "authored-file-v3", operation |-> OpOf(i),
  bytes |-> i, fileRevision |-> BaseOf(i),
  proposalRevision |-> IF i=9 THEN 0 ELSE BaseOf(i),
  acceptedDesign |-> "design-v3:D", intendedSource |-> "source-S",
  target |-> TargetOf(i), actor |-> ActorOf(i), time |-> 1,
  reason |-> <<"explicit-reason",i>>, kind |-> KindOf(i), path |-> "managed/proposal"]
ResolveTarget(i) == [source |-> "source-S", revision |-> TargetOf(i), observed |-> 1]
SeedWorkflow(o) == IF o="accepted" THEN 3 ELSE 2
SeedState(o) == CASE o="legacy-proposed" -> "proposed"
  [] o="legacy-applied" -> "applied" [] OTHER -> "accepted"
SeedAcceptance(o) == IF o="accepted" THEN "design-v3:D" ELSE "none"
SeedAcceptor(o) == IF o="accepted" THEN 1 ELSE 0
HistoricalGates == {<<"historical-gate","legacy-design","legacy-target",0>>}
Seed(o) == [revision |-> 0, proposal |-> 0,
  workflow |-> SeedWorkflow(o), state |-> SeedState(o),
  design |-> "design-v3:D", source |-> "source-S",
  acceptance |-> SeedAcceptance(o), acceptor |-> SeedAcceptor(o),
  executor |-> 0, verifier |-> 0, target |-> "none", binding |-> 0,
  landed |-> 0, gates |-> HistoricalGates,
  supersedes |-> <<"none",0>>]
(* Direct records avoid deeply nested EXCEPT terms in finite byte identities.
   Every field is explicit: reopened/migrated records retain historical gates,
   rebind 4/8 clears landed, and only 5/6 introduce executor/verifier facts. *)
RecordAt(o,i) == [revision |-> i, proposal |-> i,
  workflow |-> IF i \in {0,1,3,4,5,6,8} THEN SeedWorkflow(o) ELSE 3,
  state |-> CASE i \in {0,1} -> SeedState(o)
    [] i \in {3,4,8} -> "landing" [] i=5 -> "applied"
    [] i=6 -> "verified" [] OTHER -> "proposed",
  design |-> "design-v3:D", source |-> "source-S",
  acceptance |-> IF i \in {0,1,3,4,5,6,8} THEN SeedAcceptance(o) ELSE "none",
  acceptor |-> IF i \in {0,1,3,4,5,6,8} THEN SeedAcceptor(o) ELSE 0,
  executor |-> IF i \in {5,6} THEN 1 ELSE 0,
  verifier |-> IF i=6 THEN 2 ELSE 0,
  target |-> IF i \in {1,3,4,5,6,8} THEN TargetOf(i) ELSE "none",
  binding |-> IF i \in {4,8} THEN i ELSE IF i \in {1,3,5,6} THEN 1 ELSE 0,
  landed |-> IF i \in {3,5,6} THEN 1 ELSE 0,
  gates |-> HistoricalGates \cup
    (IF i \in {3,4,5,6,8} THEN {<<"current-gate","design-v3:D","target-A",1>>} ELSE {}),
  supersedes |-> IF i \in 0..11 THEN <<"none",0>> ELSE <<"legacy-record",0>>]
Observed(i) == \A c \in Checks : <<i,c>> \notin a.faults
Valid(i) == LET b == RecordAt(a.origin,BaseOf(i)) IN
  /\ Observed(i)
  /\ Input(i).proposalRevision=b.proposal /\ Input(i).fileRevision=b.revision
  /\ Input(i).time <= 2 /\ ResolveTarget(i).observed <= 2
  /\ Input(i).intendedSource=b.source /\ ResolveTarget(i).source=b.source
  /\ ResolveTarget(i).revision=Input(i).target
  /\ CASE KindOf(i)="bind" -> /\ b.workflow=3 /\ b.state \in {"accepted","landing"}
         /\ b.design=b.acceptance /\ Input(i).acceptedDesign=b.acceptance
     [] KindOf(i)="reopen" -> b.workflow=3 /\ b.state \in {"accepted","landing"}
     [] KindOf(i)="land" -> b.workflow=3 /\ b.binding=1 /\ b.acceptance=b.design
     [] KindOf(i) \in {"apply","verify"} ->
         /\ b.workflow=3 /\ b.binding=1 /\ b.landed=b.binding
         /\ b.target="target-A" /\ b.acceptance=b.design /\ b.acceptor # 0
         /\ <<"current-gate",b.design,b.target,b.binding>> \in b.gates
         /\ IF KindOf(i)="apply" THEN b.state="landing"
            ELSE b.state="applied" /\ ActorOf(i) # b.executor
     [] KindOf(i)="adopt" -> b.workflow=2 /\ b.state="proposed"
     [] KindOf(i)="migrate-reopen" -> b.workflow=2 /\ b.state="accepted"
     [] OTHER -> b.workflow=2 /\ b.state="applied"
BoundIntent(i) == a.intent[OpOf(i)]=i
Durable == <<a.origin,a.faults,a.head,a.file,a.applied,a.intent,a.retained,a.receipt>>
(* A concrete record domain is part of the strengthening: field observations
   alone do not establish that nested EXCEPT updates are defined functions. *)
AuthoredFields == {"origin","faults","head","file","applied","retained","receipt",
  "intent","lock","request","pc","result","checked","hint","corrupt"}
AuthoredShape(s) == s = [field \in AuthoredFields |-> s[field]]
DurableTypes(s) ==
  /\ s.origin \in Origins /\ s.faults \in Profiles /\ s.head \in Revisions
  /\ (\E o \in Origins, i \in Revisions : s.file=RecordAt(o,i))
  /\ s.applied \subseteq Requests /\ s.retained \subseteq Requests
  /\ s.receipt \subseteq Requests /\ s.intent \in [Ids -> Revisions]
ClientTypes(s) ==
  /\ s.lock \in 0..ActorCount /\ s.request \in [Actors -> Revisions]
  /\ s.pc \in [Actors -> Phases] /\ s.result \in [Actors -> Results]
  /\ s.checked \in [Actors -> Revisions]
ObservationTypes(s) ==
  /\ s.hint \in [Ids -> Revisions] /\ s.corrupt \subseteq Revisions
AuthoredType(s) == AuthoredShape(s) /\ DurableTypes(s) /\ ClientTypes(s) /\ ObservationTypes(s)
TypeOK == AuthoredType(a)
(* Exact reconstruction remains mandatory. The update laws expose only the
   shape/select facts required by actions, as in M04's lifecycle induction. *)
THEOREM AuthoredShapeDomain == \A s : AuthoredShape(s) => DOMAIN s = AuthoredFields
BY SMT DEF AuthoredShape
THEOREM AuthoredShapeUpdate == \A s, field, value :
  AuthoredShape(s) /\ field \in AuthoredFields =>
    AuthoredShape([s EXCEPT ![field]=value])
BY SMT DEF AuthoredShape
THEOREM AuthoredShapeSelect == \A s, field, value, selected :
  AuthoredShape(s) /\ field \in AuthoredFields /\ selected \in AuthoredFields =>
    [s EXCEPT ![field]=value][selected] =
      (IF selected=field THEN value ELSE s[selected])
BY SMT DEF AuthoredShape
THEOREM AuthoredNestedUpdate == \A s, field, inner, value :
  AuthoredShape(s) /\ field \in AuthoredFields =>
    [s EXCEPT ![field][inner]=value] =
      [s EXCEPT ![field]=[s[field] EXCEPT ![inner]=value]]
BY SMT DEF AuthoredShape
(* Keep generative update laws local: globally usable closure axioms create
   fresh nested-update terms even in pure stutter and propositional joins. *)
USE DEF AuthoredFields, AuthoredShape, AuthoredType, DurableTypes, ClientTypes, ObservationTypes
Chain ==
  /\ a.head \in {0} \cup a.applied
  /\ a.file=RecordAt(a.origin,a.head)
  /\ a.retained=a.applied
  /\ (\A i \in a.applied :
       /\ BaseOf(i) \in {0} \cup a.applied
       /\ BaseOf(i)<i
       /\ BoundIntent(i)
       /\ Valid(i)
       /\ i <= a.head)
  /\ (\A i,j \in a.applied : BaseOf(i)=BaseOf(j) => i=j)
IntentIdentity == \A o \in Ids : a.intent[o] # 0 => OpOf(a.intent[o])=o
ReceiptSound ==
  /\ a.receipt \subseteq a.applied
  /\ (\A w \in Actors : a.result[w]="complete" =>
       a.request[w] \in a.receipt /\ a.checked[w]=a.request[w])
Inv == TypeOK /\ Chain /\ IntentIdentity /\ ReceiptSound
Safety ==
  /\ a.file=RecordAt(a.origin,a.head)
  /\ (\A i \in a.applied : Valid(i) /\ BoundIntent(i))
  /\ a.receipt \subseteq a.retained
  /\ (\A i,j \in a.applied : BaseOf(i)=BaseOf(j) => i=j)
  /\ (\A w \in Actors : a.result[w]="complete" =>
       a.request[w] \in a.applied /\ Input(a.checked[w])=Input(a.request[w]))
Init == \E o \in Origins, f \in Profiles : a = [origin |-> o, faults |-> f,
  head |-> 0, file |-> Seed(o), applied |-> {}, retained |-> {}, receipt |-> {},
  intent |-> [id \in Ids |-> 0], lock |-> 0,
  request |-> [w \in Actors |-> 0], pc |-> [w \in Actors |-> "idle"],
  result |-> [w \in Actors |-> "none"], checked |-> [w \in Actors |-> 0],
  hint |-> [id \in Ids |-> 0], corrupt |-> {}]
Active(w) == a.request[w] \in Requests
Start(w,i) == /\ a.pc[w] \in {"idle","done"}
  /\ a'=[a EXCEPT !.request[w]=i, !.pc[w]="planned", !.result[w]="pending", !.checked[w]=0]
Acquire(w) == /\ a.pc[w]="planned" /\ a.lock=0
  /\ a'=[a EXCEPT !.lock=w, !.pc[w]="locked"]
RetainIntent(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="locked" /\ a.lock=w /\ Valid(i)
  /\ a.head=BaseOf(i) /\ a.intent[OpOf(i)] \in {0,i}
  /\ a'=[a EXCEPT !.intent[OpOf(i)]=i, !.pc[w]="intent"]
WriteCore(w,stale,keepLanded) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="intent" /\ a.lock=w /\ BoundIntent(i) /\ Valid(i)
  /\ (stale \/ (a.file.revision=Input(i).fileRevision /\ a.file.proposal=Input(i).proposalRevision))
  /\ a.head \notin a.corrupt /\ i \notin a.corrupt
  /\ a'=[a EXCEPT !.head=i,
       !.file=IF keepLanded THEN [RecordAt(a.origin,i) EXCEPT !.landed=a.file.landed]
                ELSE RecordAt(a.origin,i),
       !.applied=@ \cup {i}, !.retained=@ \cup {i}, !.pc[w]="written"]
ConditionalWrite(w) == WriteCore(w,FALSE,FALSE)
Complete(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="written" /\ a.lock=w
  /\ BoundIntent(i) /\ i \in a.retained /\ Valid(i) /\ i \notin a.corrupt
  /\ a'=[a EXCEPT !.receipt=@ \cup {i}, !.pc[w]="done", !.result[w]="complete",
       !.checked[w]=i, !.lock=0]
Conflict(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w] \in {"locked","intent","inspect"}
  /\ (a.intent[OpOf(i)] \notin {0,i} \/ (i \notin a.applied /\ a.head # BaseOf(i)))
  /\ a'=[a EXCEPT !.pc[w]="done", !.result[w]="conflict", !.lock=IF @=w THEN 0 ELSE @]
Refuse(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w] \in {"locked","intent","written","inspect"}
  /\ (~Valid(i) \/ i \in a.corrupt \/ a.head \in a.corrupt)
  /\ a'=[a EXCEPT !.pc[w]="done", !.result[w]="refused", !.lock=IF @=w THEN 0 ELSE @]
FailIO(w) == /\ a.pc[w] \notin {"idle","done","down"}
  /\ a'=[a EXCEPT !.pc[w]="done", !.result[w]="unknown", !.lock=IF @=w THEN 0 ELSE @]
Crash(w) == a'=[a EXCEPT !.pc[w]="down", !.result[w]="unknown", !.lock=IF @=w THEN 0 ELSE @]
Restart(w) == /\ a.pc[w]="down"
  /\ a'=[a EXCEPT !.pc[w]="idle", !.request[w]=0, !.result[w]="none", !.checked[w]=0]
RecoverStart(w,i) == /\ a.pc[w] \in {"idle","done"}
  /\ a'=[a EXCEPT !.request[w]=i, !.pc[w]="inspect", !.result[w]="pending", !.checked[w]=0]
ResumeExact(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="inspect" /\ a.lock=0 /\ Valid(i)
  /\ a.intent[OpOf(i)] \in {0,i} /\ i \notin a.applied /\ a.head=BaseOf(i)
  /\ a.head \notin a.corrupt /\ i \notin a.corrupt
  /\ a'=[a EXCEPT !.pc[w]="locked", !.lock=w]
RepairCompletion(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="inspect" /\ BoundIntent(i) /\ Valid(i)
  /\ i \in a.retained /\ i \notin a.corrupt /\ a.head \notin a.corrupt
  /\ a'=[a EXCEPT !.receipt=@ \cup {i}, !.pc[w]="done", !.result[w]="complete", !.checked[w]=i]
MisleadHint(o,i) == a'=[a EXCEPT !.hint[o]=i]
DetectCorruption(i) == a'=[a EXCEPT !.corrupt=@ \cup {i}]
Stutter == UNCHANGED vars
Next ==
  \/ (\E w \in Actors,i \in Requests : Start(w,i) \/ RecoverStart(w,i))
  \/ (\E w \in Actors :
       Acquire(w) \/ RetainIntent(w) \/ ConditionalWrite(w) \/ Complete(w)
       \/ Conflict(w) \/ Refuse(w) \/ FailIO(w) \/ Crash(w) \/ Restart(w)
       \/ ResumeExact(w) \/ RepairCompletion(w))
  \/ (\E o \in Ids,i \in Revisions : MisleadHint(o,i))
  \/ (\E i \in Revisions : DetectCorruption(i))
  \/ Stutter
Spec == Init /\ [][Next]_vars
(* Dropping final revision comparison is exercised by a stale retained intent
   after crash and competing recovery: the mutant resume also skips freshness. *)
MutantResumeStale(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="inspect" /\ a.lock=0 /\ BoundIntent(i) /\ Valid(i)
  /\ i \notin a.applied
  /\ a'=[a EXCEPT !.pc[w]="intent", !.lock=w]
MutantTrustHint(w) == LET i == a.request[w] IN
  /\ Active(w) /\ a.pc[w]="inspect" /\ a.hint[OpOf(i)]=i
  /\ a'=[a EXCEPT !.receipt=@ \cup {i}, !.pc[w]="done", !.result[w]="complete", !.checked[w]=i]
StaleNext == Next \/ (\E w \in Actors : MutantResumeStale(w) \/ WriteCore(w,TRUE,FALSE))
HintNext == Next \/ (\E w \in Actors : MutantTrustHint(w))
LandedNext == Next \/ (\E w \in Actors : a.request[w] \in {4,8} /\ WriteCore(w,FALSE,TRUE))
StaleSpec == Init /\ [][StaleNext]_vars
HintSpec == Init /\ [][HintNext]_vars
LandedSpec == Init /\ [][LandedNext]_vars

(* TLC-only observation quotient. These profiles restrict adversarial hints
   and discovered corruption, NOT requests, actors, intent slots or phases.
   ZeroHints is the no-hint profile. Slot 1 has exact/colliding inputs 1/2;
   slot 2 can carry stale input 1 or current input 3; slot 10 carries legacy
   migration input 11. The final profile combines independent slots 1 and 10.
   Every individual revision can be observed corrupt; each predecessor/request
   pair can be discovered in either order, including two simultaneous failures.
   Other subsets and arbitrary hint vectors remain in unrestricted Next/Inv.
   No finite-profile condition is a hypothesis of the inductive contract. *)
ZeroHints == [o \in Ids |-> 0]
FiniteHintProfiles == {ZeroHints,
  [ZeroHints EXCEPT ![1]=1], [ZeroHints EXCEPT ![1]=2],
  [ZeroHints EXCEPT ![2]=1], [ZeroHints EXCEPT ![2]=3],
  [ZeroHints EXCEPT ![10]=11],
  [ZeroHints EXCEPT ![1]=2, ![10]=11]}
FiniteCorruptionProfiles ==
  {{}} \cup {{i} : i \in Revisions}
       \cup {{BaseOf(i),i} : i \in Requests}
FiniteObservations ==
  a.hint \in FiniteHintProfiles /\ a.corrupt \in FiniteCorruptionProfiles
FiniteNext == Next /\ FiniteObservations'
FiniteSpec == Init /\ [][FiniteNext]_vars
FiniteStaleNext == StaleNext /\ FiniteObservations'
FiniteHintNext == HintNext /\ FiniteObservations'
FiniteLandedNext == LandedNext /\ FiniteObservations'
FiniteStaleSpec == Init /\ [][FiniteStaleNext]_vars
FiniteHintSpec == Init /\ [][FiniteHintNext]_vars
FiniteLandedSpec == Init /\ [][FiniteLandedNext]_vars

(* Reachability probes intentionally ask TLC to violate their named invariant.
   They are directed subsets of Next, not safety-positive specifications.
   Competition starts BOTH bind(1) and reopen(2) before either durable intent.
   Either winner then forces the other's same-ID/different-input conflict.
   Repair probe has no Complete action: its only receipt-producing step is
   RepairCompletion after written -> crash -> restart -> inspect. *)
ProbeInit == Init /\ a.origin="accepted" /\ a.faults={}
CompetingProbeNext ==
  \/ (a.pc[2]="idle" /\ Start(1,1))
  \/ (a.pc[1]="planned" /\ Start(2,2))
  \/ (a.request[1]=1 /\ a.request[2]=2 /\
       (\E w \in {1,2} : Acquire(w) \/ RetainIntent(w) \/
          ConditionalWrite(w) \/ Complete(w) \/ Conflict(w)))
  \/ Stutter
CompetingProbeSpec == ProbeInit /\ [][CompetingProbeNext]_vars
NoCompetingBindReopenConflict ==
  ~(/\ a.request[1]=1 /\ a.request[2]=2
    /\ ((a.result[1]="complete" /\ a.result[2]="conflict") \/
        (a.result[2]="complete" /\ a.result[1]="conflict")))
RepairProbeNext ==
  \/ (a.head=0 /\ Start(1,1))
  \/ Acquire(1)
  \/ RetainIntent(1)
  \/ ConditionalWrite(1)
  \/ (a.pc[1]="written" /\ a.receipt={} /\ Crash(1))
  \/ Restart(1)
  \/ (a.head=1 /\ a.pc[1]="idle" /\ RecoverStart(1,1))
  \/ RepairCompletion(1)
  \/ Stutter
RepairProbeSpec == ProbeInit /\ [][RepairProbeNext]_vars
NoCompletionRepairAfterCrash == ~(1 \in a.receipt /\ a.result[1]="complete")

(* Small finite-domain facts isolate arithmetic and the record constructors
   from action proofs. ActorUniverse names the existing bound premise; naming
   it makes the hypothesis available to TLAPS without adding an assumption. *)
THEOREM IndexFacts ==
  /\ {1,2} \subseteq Actors
  /\ Requests \subseteq Revisions /\ Revisions \subseteq Nat /\ 0 \in Revisions
  /\ (\A i \in Requests : OpOf(i) \in Ids /\ BaseOf(i) \in Revisions /\ BaseOf(i)<i)
BY SMT, ActorUniverse DEF ActorBounds, Actors, Requests, Revisions, Ids, OpOf, BaseOf, ActorUniverse
THEOREM RecordRevisions == \A o \in Origins, i \in Revisions :
  RecordAt(o,i).revision=i /\ RecordAt(o,i).proposal=i
BY DEF RecordAt
THEOREM InputFileRevision == \A i : Input(i).fileRevision=BaseOf(i)
BY ONLY DEF Input
THEOREM SeedIdentity == \A o \in Origins : RecordAt(o,0)=Seed(o)
<1> HIDE DEF AuthoredFields, AuthoredShape, AuthoredType, DurableTypes, ClientTypes, ObservationTypes
<1> QED BY ONLY SMT DEF RecordAt, Seed, HistoricalGates
THEOREM RecordLandedProjection == \A o,i :
  RecordAt(o,i).landed = IF i \in {3,5,6} THEN 1 ELSE 0
BY ONLY DEF RecordAt
THEOREM RecordGateProjection == \A o,i :
  RecordAt(o,i).gates = Seed(o).gates \cup
    (IF i \in {3,4,5,6,8} THEN {<<"current-gate","design-v3:D","target-A",1>>} ELSE {})
BY ONLY DEF RecordAt, Seed
THEOREM ReopenedFields == \A o : \A i \in {2,7,11} :
  /\ RecordAt(o,i).state="proposed" /\ RecordAt(o,i).acceptance="none"
  /\ RecordAt(o,i).acceptor=0 /\ RecordAt(o,i).executor=0 /\ RecordAt(o,i).verifier=0
  /\ RecordAt(o,i).target="none" /\ RecordAt(o,i).binding=0 /\ RecordAt(o,i).landed=0
BY ONLY DEF RecordAt
THEOREM ValidationFrame == UNCHANGED <<a.origin,a.faults>> =>
  (\A i : Valid(i)'=Valid(i))
BY DEF Valid, Observed
THEOREM ChainFrame == Chain /\ UNCHANGED
  <<a.origin,a.faults,a.head,a.file,a.applied,a.retained,a.intent>> => Chain'
BY ValidationFrame DEF Chain, BoundIntent, ValidationFrame
THEOREM IdentityFrame == IntentIdentity /\ UNCHANGED a.intent => IntentIdentity'
BY DEF IntentIdentity
THEOREM ReceiptFrame == ReceiptSound /\ UNCHANGED
  <<a.receipt,a.applied,a.result,a.request,a.checked>> => ReceiptSound'
BY DEF ReceiptSound
THEOREM StateFrame == Inv /\ UNCHANGED a => Inv'
<1>1. ASSUME Inv, UNCHANGED a PROVE Inv'
  <2> USE Inv, UNCHANGED a
  <2>1. TypeOK' BY ONLY SMT, Inv, UNCHANGED a DEF Inv, TypeOK
  <2>2. Chain' BY ChainFrame DEF Inv, ChainFrame
  <2>3. IntentIdentity' BY IdentityFrame DEF Inv, IdentityFrame
  <2>4. ReceiptSound' BY ReceiptFrame DEF Inv, ReceiptFrame
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1
THEOREM FiniteNextIncluded == FiniteNext => Next
BY DEF FiniteNext
THEOREM CompetingProbeIncluded == CompetingProbeNext => Next
BY IndexFacts DEF CompetingProbeNext, Next, Requests, IndexFacts
THEOREM RepairProbeIncluded == RepairProbeNext => Next
BY IndexFacts DEF RepairProbeNext, Next, Requests, IndexFacts

(* The final CAS compares both byte revisions under the managed-path lock.
   These projections expose its exact effect without unfolding validation
   records in the chain induction. They are consequences, not extra guards. *)
THEOREM ConditionalWriteEffects == \A w \in Actors :
  TypeOK /\ ConditionalWrite(w) =>
    /\ a.request[w] \in Requests
    /\ a'.head=a.request[w]
    /\ a'.file=RecordAt(a.origin,a.request[w])
    /\ a'.applied=a.applied \cup {a.request[w]}
    /\ a'.retained=a.retained \cup {a.request[w]}
    /\ UNCHANGED <<a.origin,a.faults,a.intent,a.receipt,a.result,a.request,a.checked>>
BY SMT DEF TypeOK, ConditionalWrite, WriteCore, Active
THEOREM ConditionalWritePredecessor == \A w \in Actors :
  Inv /\ ConditionalWrite(w) => a.head=BaseOf(a.request[w])
BY SMT, RecordRevisions, InputFileRevision
  DEF Inv, TypeOK, Chain, ConditionalWrite, WriteCore, Active,
    RecordRevisions, InputFileRevision
THEOREM ConditionalWriteFresh == \A w \in Actors :
  Inv /\ ConditionalWrite(w) =>
    /\ a.request[w] \notin a.applied
    /\ (\A j \in a.applied : j<a.request[w] /\ BaseOf(j) # BaseOf(a.request[w]))
BY SMT, ConditionalWritePredecessor, ConditionalWriteEffects, IndexFacts
  DEF ConditionalWritePredecessor, ConditionalWriteEffects, IndexFacts,
    Inv, TypeOK, Chain

THEOREM InitPreserves == Init => Inv
<1>1. Init => TypeOK
  <2>1. Init => AuthoredShape(a)
    BY ONLY DEF Init, AuthoredShape, AuthoredFields
  <2>2. Init => DurableTypes(a)
    BY ONLY SMT, IndexFacts, SeedIdentity
      DEF Init, DurableTypes, IndexFacts, SeedIdentity
  <2>3. Init => ClientTypes(a)
    BY ONLY SMT, ActorUniverse, IndexFacts
      DEF Init, ClientTypes, Phases, Results, Actors, ActorUniverse, ActorBounds, IndexFacts
  <2>4. Init => ObservationTypes(a)
    BY ONLY SMT, IndexFacts DEF Init, ObservationTypes, IndexFacts
  <2>5. QED BY ONLY <2>1, <2>2, <2>3, <2>4 DEF TypeOK, AuthoredType
<1>2. Init => Chain
  BY SeedIdentity DEF Init, Chain, BoundIntent, SeedIdentity
<1>3. Init => IntentIdentity
  BY DEF Init, IntentIdentity
<1>4. Init => ReceiptSound
  BY DEF Init, ReceiptSound
<1>5. QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv

THEOREM StartPreserves == \A w \in Actors, i \in Requests : Inv /\ Start(w,i) => Inv'
<1>1. ASSUME NEW w \in Actors, NEW i \in Requests, Inv, Start(w,i)
       PROVE Inv'
  <2> USE Inv, Start(w,i)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, AuthoredShapeUpdate, AuthoredNestedUpdate
        DEF Inv, TypeOK, Start, AuthoredShapeUpdate, AuthoredNestedUpdate
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Start, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Start, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Start, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY SMT, <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Start, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Start, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Start
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM AcquirePreserves == \A w \in Actors : Inv /\ Acquire(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Acquire(w)
       PROVE Inv'
  <2> USE Inv, Acquire(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Acquire, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Acquire, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Acquire, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Acquire, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Acquire, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Acquire, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Acquire
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM RetainIntentPreserves == \A w \in Actors : Inv /\ RetainIntent(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, RetainIntent(w)
       PROVE Inv'
  <2> USE Inv, RetainIntent(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RetainIntent, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RetainIntent, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RetainIntent, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RetainIntent, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. UNCHANGED <<a.origin,a.faults,a.head,a.file,a.applied,a.retained>>
    BY DEF Inv, TypeOK, RetainIntent
  <2>3. \A j \in a.applied : a'.intent[OpOf(j)]=a.intent[OpOf(j)]
    BY SMT, IndexFacts DEF Inv, TypeOK, Chain, BoundIntent, RetainIntent, Active, IndexFacts
  <2>4. Chain'
    BY <2>2, <2>3, ValidationFrame DEF Inv, Chain, BoundIntent, ValidationFrame
  <2>5. IntentIdentity'
    BY SMT, IndexFacts DEF Inv, IntentIdentity, TypeOK, RetainIntent, Active, IndexFacts
  <2>6. ReceiptSound'
    BY SMT, ReceiptFrame DEF Inv, RetainIntent, ReceiptFrame, TypeOK
  <2>7. QED BY <2>1, <2>4, <2>5, <2>6 DEF Inv
<1>2. QED BY <1>1

THEOREM ConditionalWritePreserves == \A w \in Actors : Inv /\ ConditionalWrite(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, ConditionalWrite(w)
       PROVE Inv'
  <2> USE Inv, ConditionalWrite(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ConditionalWrite, WriteCore, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ConditionalWrite, WriteCore, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ConditionalWrite, WriteCore, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ConditionalWrite, WriteCore, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. a.head=BaseOf(a.request[w]) /\ a'.head=a.request[w]
    BY ConditionalWritePredecessor, ConditionalWriteEffects
      DEF ConditionalWritePredecessor, ConditionalWriteEffects, Inv
  <2>3. UNCHANGED <<a.origin,a.faults,a.intent>>
    BY ConditionalWriteEffects DEF ConditionalWriteEffects, Inv
  <2>4. Chain'
    <3>1. a'.head \in {0} \cup a'.applied /\
          a'.file=RecordAt(a'.origin,a'.head) /\ a'.retained=a'.applied
      BY SMT, ConditionalWriteEffects
        DEF ConditionalWriteEffects, Inv, Chain
    <3>2. \A j \in a.applied :
      /\ BaseOf(j) \in {0} \cup a'.applied /\ BaseOf(j)<j
      /\ BoundIntent(j)' /\ Valid(j)' /\ j<=a'.head
      BY SMT, <2>2, <2>3, ConditionalWriteEffects, ConditionalWriteFresh, ValidationFrame
        DEF ConditionalWriteEffects, ConditionalWriteFresh, ValidationFrame,
          Inv, Chain, BoundIntent
    <3>3. LET i == a.request[w] IN
      /\ BaseOf(i) \in {0} \cup a'.applied /\ BaseOf(i)<i
      /\ BoundIntent(i)' /\ Valid(i)' /\ i<=a'.head
      BY SMT, <2>2, <2>3, ConditionalWriteEffects, IndexFacts, ValidationFrame
        DEF ConditionalWriteEffects, IndexFacts, ValidationFrame, Inv, Chain,
          BoundIntent, ConditionalWrite, WriteCore, Active
    <3>4. \A i,j \in a'.applied : BaseOf(i)=BaseOf(j) => i=j
      BY SMT, ConditionalWriteEffects, ConditionalWriteFresh
        DEF ConditionalWriteEffects, ConditionalWriteFresh, Inv, Chain
    <3>5. QED BY <3>1, <3>2, <3>3, <3>4, ConditionalWriteEffects
      DEF Chain, ConditionalWriteEffects, Inv
  <2>5. IntentIdentity'
    BY <2>3, IdentityFrame DEF Inv, IdentityFrame
  <2>6. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, ConditionalWrite, WriteCore
  <2>7. QED BY <2>1, <2>4, <2>5, <2>6 DEF Inv
<1>2. QED BY <1>1

THEOREM CompletePreserves == \A w \in Actors : Inv /\ Complete(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Complete(w)
       PROVE Inv'
  <2> USE Inv, Complete(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Complete, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Complete, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Complete, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Complete, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Complete, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Complete, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Complete
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM ConflictPreserves == \A w \in Actors : Inv /\ Conflict(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Conflict(w)
       PROVE Inv'
  <2> USE Inv, Conflict(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Conflict, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Conflict, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Conflict, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Conflict, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Conflict, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Conflict, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Conflict
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM RefusePreserves == \A w \in Actors : Inv /\ Refuse(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Refuse(w)
       PROVE Inv'
  <2> USE Inv, Refuse(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Refuse, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Refuse, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Refuse, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Refuse, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Refuse, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Refuse, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Refuse
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM FailIOPreserves == \A w \in Actors : Inv /\ FailIO(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, FailIO(w)
       PROVE Inv'
  <2> USE Inv, FailIO(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, FailIO, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, FailIO, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, FailIO, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, FailIO, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, FailIO, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, FailIO, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, FailIO
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM CrashPreserves == \A w \in Actors : Inv /\ Crash(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Crash(w)
       PROVE Inv'
  <2> USE Inv, Crash(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Crash, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Crash, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Crash, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Crash, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Crash, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Crash, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Crash
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM RestartPreserves == \A w \in Actors : Inv /\ Restart(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, Restart(w)
       PROVE Inv'
  <2> USE Inv, Restart(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Restart, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Restart, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Restart, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, Restart, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, Restart, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, Restart, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, Restart
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM RecoverStartPreserves == \A w \in Actors, i \in Requests : Inv /\ RecoverStart(w,i) => Inv'
<1>1. ASSUME NEW w \in Actors, NEW i \in Requests, Inv, RecoverStart(w,i)
       PROVE Inv'
  <2> USE Inv, RecoverStart(w,i)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RecoverStart, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RecoverStart, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RecoverStart, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RecoverStart, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, RecoverStart, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, RecoverStart, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, RecoverStart
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM ResumeExactPreserves == \A w \in Actors : Inv /\ ResumeExact(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, ResumeExact(w)
       PROVE Inv'
  <2> USE Inv, ResumeExact(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ResumeExact, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ResumeExact, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ResumeExact, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, ResumeExact, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, ResumeExact, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, ResumeExact, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, ResumeExact
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM RepairCompletionPreserves == \A w \in Actors : Inv /\ RepairCompletion(w) => Inv'
<1>1. ASSUME NEW w \in Actors, Inv, RepairCompletion(w)
       PROVE Inv'
  <2> USE Inv, RepairCompletion(w)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RepairCompletion, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RepairCompletion, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RepairCompletion, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, RepairCompletion, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, RepairCompletion, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, RepairCompletion, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, RepairCompletion
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM MisleadHintPreserves == \A o \in Ids, i \in Revisions : Inv /\ MisleadHint(o,i) => Inv'
<1>1. ASSUME NEW o \in Ids, NEW i \in Revisions, Inv, MisleadHint(o,i)
       PROVE Inv'
  <2> USE Inv, MisleadHint(o,i)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, MisleadHint, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, MisleadHint, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, MisleadHint, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, MisleadHint, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, MisleadHint, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, MisleadHint, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, MisleadHint
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM DetectCorruptionPreserves == \A i \in Revisions : Inv /\ DetectCorruption(i) => Inv'
<1>1. ASSUME NEW i \in Revisions, Inv, DetectCorruption(i)
       PROVE Inv'
  <2> USE Inv, DetectCorruption(i)
  <2>1. TypeOK'
    <3>1. AuthoredShape(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, DetectCorruption, Active, Phases, Results, IndexFacts
    <3>2. DurableTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, DetectCorruption, Active, Phases, Results, IndexFacts
    <3>3. ClientTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, DetectCorruption, Active, Phases, Results, IndexFacts
    <3>4. ObservationTypes(a')
      BY SMT, IndexFacts DEF Inv, TypeOK, DetectCorruption, Active, Phases, Results, IndexFacts
    <3>5. QED BY ONLY <3>1, <3>2, <3>3, <3>4 DEF TypeOK, AuthoredType
  <2>2. Chain'
    BY SMT, ChainFrame DEF Inv, DetectCorruption, ChainFrame, TypeOK
  <2>3. IntentIdentity'
    BY SMT, IdentityFrame DEF Inv, DetectCorruption, IdentityFrame, TypeOK
  <2>4. ReceiptSound'
    BY SMT DEF Inv, TypeOK, ReceiptSound, Chain, DetectCorruption
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv
<1>2. QED BY <1>1

THEOREM StutterPreserves == Inv /\ Stutter => Inv'
BY StateFrame DEF Stutter, vars, StateFrame
THEOREM NextPreserves == Inv /\ Next => Inv'
BY StartPreserves, AcquirePreserves, RetainIntentPreserves, ConditionalWritePreserves,
  CompletePreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves,
  RestartPreserves, RecoverStartPreserves, ResumeExactPreserves, RepairCompletionPreserves,
  MisleadHintPreserves, DetectCorruptionPreserves, StutterPreserves DEF Next, StartPreserves, AcquirePreserves, RetainIntentPreserves, ConditionalWritePreserves, CompletePreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves, RestartPreserves, RecoverStartPreserves, ResumeExactPreserves, RepairCompletionPreserves, MisleadHintPreserves, DetectCorruptionPreserves, StutterPreserves
THEOREM FiniteNextPreserves == Inv /\ FiniteNext => Inv'
BY FiniteNextIncluded, NextPreserves DEF FiniteNextIncluded, NextPreserves
THEOREM CompetingProbePreserves == Inv /\ CompetingProbeNext => Inv'
BY CompetingProbeIncluded, NextPreserves DEF CompetingProbeIncluded, NextPreserves
THEOREM RepairProbePreserves == Inv /\ RepairProbeNext => Inv'
BY RepairProbeIncluded, NextPreserves DEF RepairProbeIncluded, NextPreserves
THEOREM InvImpliesSafety == Inv => Safety
<1>1. Inv => a.receipt \subseteq a.retained
  BY DEF Inv, Chain, ReceiptSound
<1>2. Inv => (\A w \in Actors : a.result[w]="complete" =>
  a.request[w] \in a.applied /\ a.checked[w]=a.request[w])
  BY DEF Inv, ReceiptSound
<1>3. QED BY <1>1, <1>2 DEF Inv, Chain, Safety
(* Induction is over unrestricted Next. Finite observation profiles and
   directed probes remain separate TLC campaigns, not safety assumptions. *)
THEOREM InductiveSafety == Spec => []Safety
<1>1. Init => Inv BY InitPreserves DEF InitPreserves
<1>2. Inv /\ [Next]_vars => Inv'
  BY NextPreserves, StutterPreserves DEF NextPreserves, StutterPreserves, Stutter
<1>3. Spec => []Inv BY <1>1, <1>2, PTL DEF Spec
<1>4. QED BY <1>3, InvImpliesSafety, PTL DEF InvImpliesSafety
THEOREM FailureFrames == \A w \in Actors :
  TypeOK /\ (Conflict(w) \/ Refuse(w) \/ FailIO(w) \/ Crash(w) \/ Restart(w))
    => UNCHANGED Durable
<1>1. \A w \in Actors : TypeOK /\ Conflict(w) => UNCHANGED Durable
  BY SMT DEF TypeOK, Conflict, Durable
<1>2. \A w \in Actors : TypeOK /\ Refuse(w) => UNCHANGED Durable
  BY SMT DEF TypeOK, Refuse, Durable
<1>3. \A w \in Actors : TypeOK /\ FailIO(w) => UNCHANGED Durable
  BY SMT DEF TypeOK, FailIO, Durable
<1>4. \A w \in Actors : TypeOK /\ Crash(w) => UNCHANGED Durable
  BY SMT DEF TypeOK, Crash, Durable
<1>5. \A w \in Actors : TypeOK /\ Restart(w) => UNCHANGED Durable
  BY SMT DEF TypeOK, Restart, Durable
<1>6. QED BY <1>1, <1>2, <1>3, <1>4, <1>5
THEOREM HintFrames == \A o \in Ids,i \in Revisions :
  TypeOK /\ MisleadHint(o,i) => UNCHANGED Durable
BY SMT DEF TypeOK, MisleadHint, Durable
THEOREM RepairFrames == \A w \in Actors : TypeOK /\ RepairCompletion(w) =>
  UNCHANGED <<a.head,a.file,a.applied,a.intent,a.retained,a.origin,a.faults>>
BY SMT DEF TypeOK, RepairCompletion
THEOREM ExactRecovery == \A w \in Actors : Inv /\ RepairCompletion(w) =>
  /\ a.intent[OpOf(a.request[w])]=a.request[w]
  /\ a.request[w] \in a.retained
  /\ Input(a'.checked[w])=Input(a.request[w])
<1>1. \A w \in Actors : Inv /\ RepairCompletion(w) => a'.checked[w]=a.request[w]
  BY SMT DEF Inv, TypeOK, RepairCompletion
<1>2. QED BY <1>1 DEF RepairCompletion, BoundIntent
THEOREM ChangedInputConflicts == \A w \in Actors :
  Active(w) /\ a.intent[OpOf(a.request[w])] \notin {0,a.request[w]} =>
    ~ResumeExact(w) /\ ~RepairCompletion(w) /\ ~RetainIntent(w)
BY SMT DEF ResumeExact, RepairCompletion, RetainIntent, BoundIntent
THEOREM WrittenRecord == \A w \in Actors : Inv /\ ConditionalWrite(w) =>
  /\ a.file=RecordAt(a.origin,BaseOf(a.request[w]))
  /\ a'.file=RecordAt(a.origin,a.request[w])
<1>1. \A w \in Actors : Inv /\ ConditionalWrite(w) => a.head=BaseOf(a.request[w])
  BY ConditionalWritePredecessor DEF ConditionalWritePredecessor
<1>2. QED BY <1>1, ConditionalWriteEffects
  DEF Inv, Chain, ConditionalWriteEffects
THEOREM RebindRecordFacts == \A o \in Origins, i \in {4,8} :
  RecordAt(o,i).landed=0 /\ RecordAt(o,BaseOf(i)).gates \subseteq RecordAt(o,i).gates
BY ONLY SMT, RecordLandedProjection, RecordGateProjection
  DEF RecordLandedProjection, RecordGateProjection, BaseOf
THEOREM RebindClearsCurrentLanded == \A w \in Actors :
  Inv /\ ConditionalWrite(w) /\ a.request[w] \in {4,8} =>
    a'.file.landed=0 /\ a.file.gates \subseteq a'.file.gates
BY ONLY SMT, WrittenRecord, RebindRecordFacts
  DEF Inv, TypeOK, AuthoredType, DurableTypes, WrittenRecord, RebindRecordFacts
THEOREM ReopenRecordFacts == \A o \in Origins, i \in {2,7,11} :
  /\ RecordAt(o,i).state="proposed" /\ RecordAt(o,i).acceptance="none"
  /\ RecordAt(o,i).acceptor=0 /\ RecordAt(o,i).executor=0 /\ RecordAt(o,i).verifier=0
  /\ RecordAt(o,i).target="none" /\ RecordAt(o,i).binding=0 /\ RecordAt(o,i).landed=0
  /\ RecordAt(o,BaseOf(i)).gates \subseteq RecordAt(o,i).gates
<1>1. \A o \in Origins, i \in {2,7,11} :
  RecordAt(o,BaseOf(i)).gates \subseteq RecordAt(o,i).gates
  BY ONLY SMT, RecordGateProjection DEF RecordGateProjection, BaseOf
<1>2. QED BY ONLY <1>1, ReopenedFields DEF ReopenedFields
THEOREM ReopenClearsAuthority == \A w \in Actors :
  Inv /\ ConditionalWrite(w) /\ a.request[w] \in {2,7,11} =>
    /\ a'.file.state="proposed" /\ a'.file.acceptance="none"
    /\ a'.file.acceptor=0 /\ a'.file.executor=0 /\ a'.file.verifier=0
    /\ a'.file.target="none" /\ a'.file.binding=0 /\ a'.file.landed=0
    /\ a.file.gates \subseteq a'.file.gates
BY ONLY SMT, WrittenRecord, ReopenRecordFacts
  DEF Inv, TypeOK, AuthoredType, DurableTypes, WrittenRecord, ReopenRecordFacts
=============================================================================
