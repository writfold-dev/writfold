--------------------------- MODULE M03M04Bridge ---------------------------
EXTENDS Naturals, FiniteSets, TLAPS
CONSTANTS Readers, Releases, InitialRelease,
          Incarnations, Pids, Children, Owner, Instance, KnowledgeBase
VARIABLES r, c, bound, connections, work
R == INSTANCE M03ReaderRetention
C == INSTANCE M04CompanionLifecycle
vars == <<r, c, bound, connections>>
Environment == R!Environment /\ C!Environment
(***************************************************************************
Explicit state projection, not an import of unrelated safety theorems:
M03.r = r; M04.c = c. bound[q] maps the persisted reader lease to the actual
engine boot/start incarnation c.inc; c.pid is NOT that identity. Owner and
Instance are fixed supervisor-scope identities, distinct from KnowledgeBase.
connections is the set of reserved connection/worker resources, including
poisoned, cancelled and uncertain handles, not a claim that TCP is still live.
A concrete connection key must additionally contain native database identity,
exact r.generation[q] commit, schema, credentials and read-only access mode;
those nonchanging dimensions refine the opaque release/incarnation pair here.
Request generations confer no ownership and never discharge a lease.

BindReader is the joint exact-incarnation/query-ready checkout plus coordinated
pin acquisition. Implementations may split this only while preserving a pin
and reservation across both phases and checking the incarnation at checkout.
ReadBound requires the same ready incarnation and exact pinned revision.
CancelReader closes admission; it does NOT remove connections or bound roots.
Stop/escalation/crash never remove them. CloseReaderConnection requires observed
complete teardown of THAT reader's connection and owned descendants, with no
remaining background alias to its pin. It can finish while another reader of
the shared ready engine continues. This is an effect-result premise, never a
request-cancellation inference. ReapEngine certifies engine-wide parent and
descendant teardown and clears unresolved resources; CertifyReaderTeardown
then records that evidence. ReleaseReader only removes a physically closed pin.
New engine incarnation/upgrade cannot replace an engine with bound readers.

Independent effects preserve the OTHER module's complete record (frame).
The resulting invariant binds actual engine ownership/capacity to retained
Git/native/capture closure, even during cancellation and uncertain death.
There is no production adapter or proved Rust refinement yet. Runtime must
implement these joint transitions or establish an explicit refinement for a
more concurrent implementation. Safety needs no fairness. Completion needs
stable storage/process observation, eventual owned descendant termination,
fair stop/reap/certification/release, and no endless crashes. No liveness theorem.
***************************************************************************)
Binding ==
 /\ bound \in [Readers -> Incarnations \cup {"none"}]
 /\ connections \subseteq Readers
 /\ (\A q \in Readers :
       /\ (bound[q] = "none" <=> r.phase[q] = "idle")
       /\ (bound[q] # "none" =>
             /\ bound[q] = c.inc
             /\ (q \in r.tornDown => q \notin connections)
             /\ (q \notin r.tornDown => c.charged \/ c.reaped))
       /\ (q \in connections =>
             /\ bound[q] # "none"
             /\ c.charged
             /\ q \notin r.tornDown))
 /\ (c.reaped => connections = {})
Inv == R!Inv /\ C!Inv /\ Binding
Safety ==
 /\ R!Safety /\ C!Safety
 /\ (\A q \in connections :
       /\ bound[q] = c.inc
       /\ c.charged
       /\ r.pins[q] = {r.generation[q]}
       /\ R!Closure(r.generation[q]) \subseteq r.present)
 /\ (\A q \in Readers :
       (bound[q] # "none" => R!Closure(r.generation[q]) \subseteq r.present))
Init == /\ R!Init /\ C!Init
        /\ bound = [q \in Readers |-> "none"] /\ connections = {}
NoBindings == \A q \in Readers : bound[q] = "none"
BindReader(q) ==
 /\ c.up /\ c.phase = "ready" /\ c.admitResults /\ c.actual[c.pid] = c.inc
 /\ R!AcquirePin(q)
 /\ bound' = [bound EXCEPT ![q] = c.inc]
 /\ connections' = connections \cup {q}
 /\ UNCHANGED c
ReadBound(q) ==
 /\ q \in connections /\ bound[q] = c.inc
 /\ c.up /\ c.phase = "ready" /\ c.admitResults /\ c.actual[c.pid] = c.inc
 /\ R!ReadPage(q) /\ UNCHANGED <<c, bound, connections>>
CancelReader(q) == /\ R!Cancel(q) /\ UNCHANGED <<c, bound, connections>>
CertifyReaderTeardown(q) ==
 /\ c.reaped /\ ~c.charged /\ q \notin connections
 /\ R!CompleteTeardown(q) /\ UNCHANGED <<c, bound, connections>>
CloseReaderConnection(q) ==
 /\ c.up /\ c.phase = "ready" /\ q \in connections /\ bound[q] = c.inc
 /\ R!CompleteTeardown(q)
 /\ connections' = connections \ {q}
 /\ UNCHANGED <<c, bound>>
ReleaseReader(q) ==
 /\ q \notin connections /\ R!ReleasePin(q)
 /\ bound' = [bound EXCEPT ![q] = "none"]
 /\ UNCHANGED <<c, connections>>
SpawnEngine(i, p) ==
 /\ NoBindings /\ connections = {} /\ C!Spawn(i, p)
 /\ UNCHANGED <<r, bound, connections>>
ReapEngine ==
 /\ C!Reap /\ connections' = {} /\ UNCHANGED <<r, bound>>
ToolStep == /\ NoBindings /\ connections = {} /\ C!ToolStep
            /\ UNCHANGED <<r, bound, connections>>
EngineStep ==
 /\ (C!Listening \/ C!QueryReady \/ C!StartFailed \/ C!QueryFailed
     \/ C!Stop \/ C!Cancel \/ C!TimeoutEscalation \/ C!SignalParent
     \/ C!RejectWrongIncarnation \/ C!ParentExit
     \/ (\E d \in Children : C!ChildExit(d) \/ C!SignalDescendant(d)) \/ C!PidReuse
     \/ C!ObserveQuiescent \/ C!FailedIO \/ C!Crash \/ C!Restart)
 /\ UNCHANGED <<r, bound, connections>>
RetentionStep == /\ (R!RootStep \/ R!OtherStep)
                 /\ UNCHANGED <<c, bound, connections>>
ReaderStep == \E q \in Readers : BindReader(q) \/ ReadBound(q) \/ CancelReader(q)
               \/ CertifyReaderTeardown(q) \/ CloseReaderConnection(q) \/ ReleaseReader(q)
Next == ReaderStep \/ (\E i \in Incarnations, p \in Pids : SpawnEngine(i, p))
        \/ ReapEngine \/ ToolStep \/ EngineStep \/ RetentionStep
Spec == Init /\ [][Next]_vars

THEOREM InitInvariant == Environment /\ Init => Inv
 <1>1. Environment /\ Init => R!Inv /\ C!Inv
   BY R!InitInvariant, C!InitInvariant
   DEF R!InitInvariant, C!InitInvariant, Environment, Init
 <1>2. Init => Binding
   BY SMT DEF Init, Binding, R!Init
 <1> QED BY <1>1, <1>2 DEF Inv
(* Local projections preserve M03's existing action/proof bodies. They expose
   only the reader metadata consumed by Binding, not the full storage record. *)
THEOREM ReaderAcquireProjection == \A q \in Readers :
 R!TypeOK /\ R!AcquirePin(q) =>
   /\ r.phase[q] = "idle"
   /\ r'.tornDown = r.tornDown \ {q}
   /\ (\A other \in Readers :
         r'.phase[other] = IF other = q THEN "reading" ELSE r.phase[other])
 BY SMT DEF R!TypeOK, R!Typed, R!StateType, R!AcquirePin
THEOREM ReadyEngineFacts ==
 Environment /\ C!Inv /\ c.phase = "ready" =>
 c.inc \in Incarnations /\ c.inc # "none" /\ c.charged /\ ~c.reaped
 BY SMT DEF Environment, C!Environment, C!Inv, C!TypeOK, C!LifecycleType,
            C!ProcessTypes, C!Readiness, C!Ownership
THEOREM ReaderReadProjection == \A q \in Readers :
 R!TypeOK /\ R!ReadPage(q) => UNCHANGED <<r.phase, r.tornDown>>
 BY SMT DEF R!TypeOK, R!Typed, R!StateType, R!ReadPage
THEOREM BindPreserves == \A q \in Readers : Environment /\ Inv /\ BindReader(q) => Inv'
 <1>1. \A q \in Readers : Environment /\ Inv /\ BindReader(q) => R!Inv' /\ C!Inv'
   BY R!AcquirePreserves, C!StutterPreserves
   DEF R!AcquirePreserves, C!StutterPreserves, Environment, Inv, BindReader, C!vars
 <1>2. \A q \in Readers : Environment /\ Inv /\ BindReader(q) => Binding'
   BY ReaderAcquireProjection, ReadyEngineFacts, SMT
   DEF ReaderAcquireProjection, ReadyEngineFacts, Environment, Inv, R!Inv,
       Binding, BindReader
 <1> QED BY <1>1, <1>2 DEF Inv
THEOREM ReadPreserves == \A q \in Readers : Inv /\ ReadBound(q) => Inv'
 <1>1. \A q \in Readers : Inv /\ ReadBound(q) => R!Inv' /\ C!Inv'
   BY R!ReadPreserves, C!StutterPreserves
   DEF R!ReadPreserves, C!StutterPreserves, Inv, ReadBound, C!vars
 <1>2. \A q \in Readers : Inv /\ ReadBound(q) => Binding'
   BY ReaderReadProjection, SMT DEF ReaderReadProjection, Inv, R!Inv, Binding, ReadBound
 <1> QED BY <1>1, <1>2 DEF Inv
THEOREM CancelPreserves == \A q \in Readers : Inv /\ CancelReader(q) => Inv'
 BY R!CancelPreserves, C!StutterPreserves, SMT
 DEF R!CancelPreserves, C!StutterPreserves, Inv, Binding, CancelReader, R!Cancel, R!Inv, R!TypeOK, R!Typed, R!StateType, C!vars
THEOREM CertifyPreserves == \A q \in Readers : Inv /\ CertifyReaderTeardown(q) => Inv'
 BY R!TeardownPreserves, C!StutterPreserves, SMT
 DEF R!TeardownPreserves, C!StutterPreserves, Inv, Binding, CertifyReaderTeardown, R!CompleteTeardown, R!Inv, R!TypeOK,
     R!Typed, R!StateType, C!vars
THEOREM ConnectionClosePreserves == \A q \in Readers : Inv /\ CloseReaderConnection(q) => Inv'
 BY R!TeardownPreserves, C!StutterPreserves, SMT
 DEF R!TeardownPreserves, C!StutterPreserves, Inv, Binding, CloseReaderConnection, R!CompleteTeardown, R!Inv, R!TypeOK,
     R!Typed, R!StateType, C!vars
THEOREM ReleasePreserves == \A q \in Readers : Inv /\ ReleaseReader(q) => Inv'
 BY R!ReleasePreserves, C!StutterPreserves, SMT
 DEF R!ReleasePreserves, C!StutterPreserves, Inv, Binding, ReleaseReader, R!ReleasePin, R!Inv, R!TypeOK, R!Typed, R!StateType, C!vars
THEOREM SpawnPreserves == \A i \in Incarnations, p \in Pids : Inv /\ SpawnEngine(i, p) => Inv'
 <1>1. \A i \in Incarnations, p \in Pids : Inv /\ SpawnEngine(i,p) => R!Inv' /\ C!Inv'
   BY C!SpawnPreserves, R!StutterPreserves
   DEF C!SpawnPreserves, R!StutterPreserves, Inv, SpawnEngine, R!vars
 <1>2. \A i \in Incarnations, p \in Pids : Inv /\ SpawnEngine(i,p) => Binding'
   BY C!SpawnChanges, SMT
   DEF C!SpawnChanges, C!SpawnChanged, Inv, C!Inv, Binding, SpawnEngine, NoBindings
 <1> QED BY <1>1, <1>2 DEF Inv
THEOREM ReapPreserves == Inv /\ ReapEngine => Inv'
 <1>1. Inv /\ ReapEngine => R!Inv' /\ C!Inv'
   BY C!ReapPreserves, R!StutterPreserves
   DEF C!ReapPreserves, R!StutterPreserves, Inv, ReapEngine, R!vars
 <1>2. Inv /\ ReapEngine => Binding'
   BY C!ReapOwnershipEffects, SMT
   DEF C!ReapOwnershipEffects, Inv, Binding, ReapEngine, C!Inv
 <1> QED BY <1>1, <1>2 DEF Inv
THEOREM ToolPreserves == Inv /\ ToolStep => Inv'
 BY C!StepPreserves, R!StutterPreserves, SMT
 DEF C!StepPreserves, R!StutterPreserves, Inv, Binding, ToolStep, NoBindings, C!Next, C!vars, R!vars
THEOREM EnginePreserves == Inv /\ EngineStep => Inv'
 <1>1. Inv /\ EngineStep => R!Inv' /\ C!Inv'
   BY C!StepPreserves, R!StutterPreserves
   DEF C!StepPreserves, R!StutterPreserves, Inv, EngineStep,
       C!Next, C!ProcessStep, C!vars, R!vars
 <1>2. Inv /\ EngineStep => UNCHANGED <<c.inc, c.charged, c.reaped>>
   BY C!ResidentControlFrame
   DEF C!ResidentControlFrame, C!ControlFrame, C!ResidentControlStep,
       C!ReadinessControlStep, C!ObservationControlStep, C!FailureControlStep,
       Inv, C!Inv, EngineStep
 <1>3. Inv /\ EngineStep => Binding'
   BY <1>2, SMT DEF Inv, Binding, EngineStep
 <1> QED BY <1>1, <1>3 DEF Inv
THEOREM RetentionPreserves == Environment /\ Inv /\ RetentionStep => Inv'
 <1>1. Environment /\ Inv /\ RetentionStep => R!Inv' /\ C!Inv'
   BY R!StepPreserves, C!StutterPreserves
   DEF R!StepPreserves, C!StutterPreserves, Environment, Inv, RetentionStep,
       R!Next, R!vars, C!vars
 <1>2. Inv /\ RetentionStep =>
   /\ UNCHANGED r.tornDown
   /\ (\A q \in Readers : (r'.phase[q] = "idle" <=> r.phase[q] = "idle"))
   BY SMT
   DEF Inv, RetentionStep, R!Inv, R!TypeOK, R!Typed, R!StateType,
       R!RootStep, R!OtherStep, R!RetainRoot, R!RetireRoot, R!PublishCAS,
       R!WriteObjects, R!PlanGC, R!DeletePlan, R!PartialDeleteFailure,
       R!RejectStalePlan, R!FailedIO, R!Crash, R!Restart
 <1>3. Inv /\ RetentionStep => Binding'
   BY <1>2, SMT DEF Inv, Binding, RetentionStep
 <1> QED BY <1>1, <1>3 DEF Inv
THEOREM StutterPreserves == Inv /\ UNCHANGED vars => Inv'
 BY R!StutterPreserves, C!StutterPreserves, SMT
 DEF R!StutterPreserves, C!StutterPreserves, vars, Inv, Binding, R!vars, C!vars
THEOREM StepPreserves == Environment /\ Inv /\ [Next]_vars => Inv'
 BY BindPreserves, ReadPreserves, CancelPreserves, CertifyPreserves, ConnectionClosePreserves,
 ReleasePreserves, SpawnPreserves, ReapPreserves, ToolPreserves,
 EnginePreserves, RetentionPreserves, StutterPreserves DEF BindPreserves, ReadPreserves, CancelPreserves, CertifyPreserves, ConnectionClosePreserves, ReleasePreserves, SpawnPreserves, ReapPreserves, ToolPreserves, EnginePreserves, RetentionPreserves, StutterPreserves, Next, ReaderStep
THEOREM InvariantImpliesSafety == Inv => Safety
 <1>1. Inv => R!Safety /\ C!Safety
   BY R!InvariantImpliesSafety, C!InvariantImpliesSafety
   DEF R!InvariantImpliesSafety, C!InvariantImpliesSafety, Inv
 <1>2. Inv => (\A q \in connections :
   q \in Readers /\ r.phase[q] # "idle" /\ bound[q]=c.inc /\ c.charged)
   BY SMT DEF Inv, Binding
 <1>3. Inv => (\A q \in Readers : bound[q] # "none" => r.phase[q] # "idle")
   BY DEF Inv, Binding
 <1> QED BY <1>1, <1>2, <1>3 DEF Safety, R!Safety
THEOREM RetentionFrame == RetentionStep => UNCHANGED <<c, bound, connections>>
 BY DEF RetentionStep
THEOREM EngineFrame == EngineStep => UNCHANGED <<r, bound, connections>>
 BY DEF EngineStep
THEOREM CancelRetainsOwnership == \A q \in Readers : Inv /\ CancelReader(q) =>
 UNCHANGED <<r.pins, r.tornDown, c.charged, c.inc, bound, connections>>
 BY R!CancellationFrame, SMT
 DEF R!CancellationFrame, Inv, R!Inv, CancelReader
THEOREM IndependentReaderTeardown == \A q, other \in Readers :
 Inv /\ q # other /\ CloseReaderConnection(q) =>
   /\ UNCHANGED <<r.pins[other], r.phase[other], r.generation[other], bound[other], c>>
   /\ (other \in connections <=> other \in connections')
 BY SMT DEF Inv, R!Inv, R!TypeOK, R!Typed, R!StateType,
            CloseReaderConnection, R!CompleteTeardown

(* Share M03's causal workload observer. Unrelated engine and connection effects
   frame work; observer-only conflict/cleanup projects to full bridge stuttering.
   Immutable reader incarnations can reuse physical engine/connection slots. *)
FiniteEnvironment == Environment /\ R!FiniteEnvironment
WorkType == R!WorkType
FiniteInit == Init /\ R!WorkInit
finiteVars == <<r, c, bound, connections, work>>
FiniteReaderStep ==
 \E q \in Readers :
   \/ (/\ ((q \notin r.acquiredReaders /\ BindReader(q))
           \/ CancelReader(q) \/ CertifyReaderTeardown(q)
           \/ CloseReaderConnection(q) \/ ReleaseReader(q))
       /\ UNCHANGED work)
   \/ (ReadBound(q) /\ R!RecordReadEvent(q))
FiniteRetentionStep(aux) ==
 /\ (R!FiniteRootStep(aux) \/ R!FiniteOtherStep(aux) \/ R!FiniteControlStep)
 /\ UNCHANGED <<c, bound, connections>>
FiniteEngineStep ==
 /\ ((\E i \in Incarnations, p \in Pids : SpawnEngine(i, p))
     \/ ReapEngine \/ ToolStep \/ EngineStep)
 /\ UNCHANGED work
FiniteNext(aux) == FiniteReaderStep \/ FiniteRetentionStep(aux) \/ FiniteEngineStep
BackupSpec == FiniteInit /\ [][FiniteNext("backup")]_finiteVars
TransferSpec == FiniteInit /\ [][FiniteNext("transfer")]_finiteVars
HistorySpec == FiniteInit /\ [][FiniteNext("history")]_finiteVars
CorrectionSpec == FiniteInit /\ [][FiniteNext("correction")]_finiteVars
ExplicitSpec == FiniteInit /\ [][FiniteNext("explicit")]_finiteVars
BackupEpochBound == R!ProfileEpochBound("backup")
TransferEpochBound == R!ProfileEpochBound("transfer")
HistoryEpochBound == R!ProfileEpochBound("history")
CorrectionEpochBound == R!ProfileEpochBound("correction")
ExplicitEpochBound == R!ProfileEpochBound("explicit")
OldReaderIsolationReached ==
 R!OldReaderIsolationReached /\ c.phase = "ready" /\ r.up /\ c.up
 /\ (\E q \in connections : r.phase[q] = "reading" /\ r.generation[q] = InitialRelease)
OldReaderWitnessAbsent == ~OldReaderIsolationReached
OverlapWitnessAbsent == ~R!OverlapGCReached("history")
CompetitionWitnessAbsent == ~R!CompetitionReached
THEOREM FiniteInitRefines == FiniteInit => Init
 BY DEF FiniteInit
THEOREM FiniteStepRefines == \A aux \in R!AuxiliaryKinds :
 FiniteEnvironment /\ Inv /\ FiniteNext(aux) => [Next]_vars
 BY R!FiniteRootRefines, R!FiniteOtherRefines, R!FiniteControlFrame, SMT
 DEF R!FiniteRootRefines, R!FiniteOtherRefines, R!FiniteControlFrame, FiniteEnvironment, Inv, R!Inv, FiniteNext, FiniteReaderStep, FiniteEngineStep,
     FiniteRetentionStep, Next, ReaderStep, RetentionStep, vars, R!vars
THEOREM FiniteStepPreserves == \A aux \in R!AuxiliaryKinds :
 FiniteEnvironment /\ Inv /\ [FiniteNext(aux)]_finiteVars => Inv'
 BY FiniteStepRefines, StepPreserves, StutterPreserves
 DEF FiniteStepRefines, StepPreserves, StutterPreserves, FiniteEnvironment, finiteVars, vars
THEOREM FiniteRetentionProjection == \A aux \in R!AuxiliaryKinds :
 FiniteNext(aux) => [R!FiniteNext(aux)]_(R!finiteVars)
 BY SMT DEF FiniteNext, FiniteReaderStep, FiniteRetentionStep, FiniteEngineStep,
     R!FiniteNext, R!FiniteReaderStep, R!finiteVars, BindReader, ReadBound,
     CancelReader, CertifyReaderTeardown, CloseReaderConnection, ReleaseReader,
     SpawnEngine, ReapEngine, ToolStep, EngineStep
THEOREM WorkInitTyped == FiniteEnvironment /\ FiniteInit => WorkType
 BY R!WorkInitTyped DEF R!WorkInitTyped, FiniteEnvironment, FiniteInit, WorkType
THEOREM WorkStepTyped == \A aux \in R!AuxiliaryKinds :
 Inv /\ WorkType /\ [FiniteNext(aux)]_finiteVars => WorkType'
 BY FiniteRetentionProjection, R!WorkStepTyped, SMT
 DEF FiniteRetentionProjection, R!WorkStepTyped, Inv, R!Inv, WorkType, finiteVars, R!finiteVars
=============================================================================
