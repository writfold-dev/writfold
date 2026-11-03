------------------------- MODULE M03ReaderRetention -------------------------
EXTENDS Naturals, FiniteSets, TLAPS
CONSTANTS Readers, Releases, InitialRelease
VARIABLES r, work
vars == <<r>>
Kinds == {"published", "pending", "prepared", "backup", "transfer",
          "history", "correction", "explicit"}
PublishedId == <<"published", InitialRelease>>
RootIds == {PublishedId} \cup (Kinds \X Releases)
ObjectKinds == {"git", "native", "capture"}
Objects == ObjectKinds \X Releases
Closure(g) == ObjectKinds \X {g}
Closures(gs) == ObjectKinds \X gs
Phases == {"idle", "reading", "draining", "uncertain"}
RootSet(roots, pins) ==
 (UNION {roots[k] : k \in RootIds})
 \cup (UNION {Closures(pins[q]) : q \in Readers})
RootObjects == RootSet(r.roots, r.pins)
Environment == /\ Readers # {} /\ Releases # {}
               /\ InitialRelease \in Releases
(***************************************************************************
Trust boundary: Objects represents heterogeneous transitive Git/native/capture
closure, NOT a database generation. Capture/commit authentication, durable root
files and qualified fsync, atomic completed-ref CAS, and exclusive cooperating
retention coordination are external premises. No hostile same-user edits.
AcquirePin, root mutation, PublishCAS and DeletePlan are atomic under that SAME
coordination. A result of an attempted IO is not an observed success.

roots maps immutable ROOT IDENTITIES to object sets, not one slot per category.
The finite identity product uses model labels; it is not a runtime narrowing.
Independent same-kind roots may overlap, and retiring one never removes the
other root's membership. PublishedId is the stable completed-retention-policy
root; prepared root <<"prepared", g>> owns candidate g's closure.
M01 projection: head <- p.head; roots[PublishedId] <- Closures(p.published);
the union of prepared roots covers Closures(p.manifest); pending identity roots
cover actual retained input/native objects of nonempty p.intent; present <- p.objects.
M01 supplies signature/predecessor validation and completed Git CAS. RetireRoot
requires approval to retire that EXACT owner root, not a category-wide cleanup.

Epochs are unbounded natural numbers in the unrestricted protocol and induction.
The finite profiles below bound the WORKLOAD, not epoch values or teardown.
acquiredReaders and retainedRootIds are monotone observation histories: unrestricted
actions never use them to deny work. Profiles schedule one request per immutable
reader incarnation (not a reusable connection slot) and one payload per root ID,
with completion, cancellation, crash/restart, GC and teardown transitions enabled.
No fairness is needed for safety. Eventual completion requires stable IO,
certified physical teardown and fair teardown/release/replanning. No temporal
progress theorem or runtime numeric narrowing is claimed.

Runtime map: AcquirePin = resolve completed ref + persist exact pin under lock;
ReadPage = exact commit/context/filter/order continuation (abstracted by release);
Cancel = close result admission, not teardown; CompleteTeardown = adapter's
certified connection AND descendant cleanup; ReleasePin = unlink/fsync pin;
PlanGC = locked root snapshot; DeletePlan = locked epoch recheck + observed
object deletions. Crash/Restart preserve persisted roots; uncertainty is sticky
until certified teardown. No adapter/refinement implementation is claimed.
***************************************************************************)
StateType ==
 [head : Releases, roots : [RootIds -> SUBSET Objects],
  present : SUBSET Objects, pins : [Readers -> SUBSET Releases],
  phase : [Readers -> Phases], generation : [Readers -> Releases],
  seen : [Readers -> SUBSET Releases], tornDown : SUBSET Readers,
  acquiredReaders : SUBSET Readers, retainedRootIds : SUBSET RootIds,
  epoch : Nat, planValid : BOOLEAN, planEpoch : Nat,
  candidates : SUBSET Objects, up : BOOLEAN,
  outcome : {"ok", "ioFailed", "stale", "uncertain", "restarted"}]
Typed(s) ==
 /\ s \in StateType
 /\ (\A q \in Readers : s.pins[q] \in {{}, {s.generation[q]}})
Rooted(roots, pins, present) ==
 /\ (\A k \in RootIds : roots[k] \subseteq present)
 /\ (\A q \in Readers : Closures(pins[q]) \subseteq present)
RootState(head, roots, pins, present) ==
 /\ Rooted(roots, pins, present)
 /\ Closure(head) \subseteq roots[PublishedId]
ReaderState(phase, pins, generation, seen, tornDown) ==
 \A q \in Readers :
   /\ seen[q] \subseteq {generation[q]}
   /\ (phase[q] = "idle" <=> pins[q] = {})
   /\ (phase[q] = "reading" => q \notin tornDown)
   /\ (phase[q] # "idle" => pins[q] = {generation[q]})
PlanState(epoch, valid, planEpoch, candidates, roots, pins) ==
 /\ planEpoch <= epoch
 /\ (valid /\ planEpoch = epoch => candidates \cap RootSet(roots, pins) = {})
TypeOK == Typed(r)
RootIntegrity == RootState(r.head, r.roots, r.pins, r.present)
ReaderIntegrity == ReaderState(r.phase, r.pins, r.generation, r.seen, r.tornDown)
PlanIntegrity == PlanState(r.epoch, r.planValid, r.planEpoch, r.candidates, r.roots, r.pins)
Inv == TypeOK /\ RootIntegrity /\ ReaderIntegrity /\ PlanIntegrity
Safety ==
 /\ (\A q \in Readers :
       (r.phase[q] # "idle" =>
         /\ Closure(r.generation[q]) \subseteq r.present
         /\ r.pins[q] = {r.generation[q]}
         /\ r.seen[q] \subseteq {r.generation[q]}))
 /\ Closure(r.head) \subseteq r.present
 /\ (\A k \in RootIds : r.roots[k] \subseteq r.present)
Init ==
 r = [head |-> InitialRelease,
      roots |-> [k \in RootIds |-> IF k = PublishedId THEN Closure(InitialRelease) ELSE {}],
      present |-> Objects, pins |-> [q \in Readers |-> {}],
      phase |-> [q \in Readers |-> "idle"],
      generation |-> [q \in Readers |-> InitialRelease],
      seen |-> [q \in Readers |-> {}], tornDown |-> Readers,
      acquiredReaders |-> {}, retainedRootIds |-> {},
      epoch |-> 0, planValid |-> FALSE, planEpoch |-> 0,
      candidates |-> {}, up |-> TRUE, outcome |-> "ok"]
AcquirePin(q) ==
 /\ r.up /\ r.phase[q] = "idle"
 /\ r' = [r EXCEPT !.pins[q] = {r.head}, !.generation[q] = r.head,
          !.phase[q] = "reading", !.seen[q] = {},
          !.tornDown = @ \ {q}, !.epoch = @ + 1,
          !.acquiredReaders = @ \cup {q}]
ReadPage(q) ==
 /\ r.up /\ r.phase[q] = "reading"
 /\ r' = [r EXCEPT !.seen[q] = @ \cup {r.generation[q]}]
Cancel(q) ==
 /\ r.up /\ r.phase[q] = "reading"
 /\ r' = [r EXCEPT !.phase[q] = "draining"]
CompleteTeardown(q) ==
 /\ r.up /\ r.phase[q] \in {"draining", "uncertain"}
 /\ r' = [r EXCEPT !.tornDown = @ \cup {q}]
ReleasePin(q) ==
 /\ r.up /\ q \in r.tornDown /\ r.phase[q] \in {"draining", "uncertain"}
 /\ r' = [r EXCEPT !.pins[q] = {}, !.phase[q] = "idle", !.epoch = @ + 1]
RetainRoot(k, os) ==
 /\ r.up /\ os \subseteq r.present
 /\ r' = [r EXCEPT !.roots[k] = @ \cup os, !.epoch = @ + 1,
          !.retainedRootIds = @ \cup {k}]
RetireRoot(k, os) ==
 /\ r.up
 /\ (k = PublishedId => Closure(r.head) \cap os = {})
 /\ r' = [r EXCEPT !.roots[k] = @ \ os, !.epoch = @ + 1]
PublishCAS(g) ==
 /\ r.up /\ Closure(g) \subseteq r.roots[<<"prepared", g>>]
 /\ r' = [r EXCEPT !.head = g, !.roots[PublishedId] = @ \cup Closure(g),
          !.epoch = @ + 1]
WriteObjects(g) ==
 /\ r.up /\ r' = [r EXCEPT !.present = @ \cup Closure(g), !.outcome = "ok"]
PlanGC ==
 /\ r.up
 /\ r' = [r EXCEPT !.planValid = TRUE, !.planEpoch = r.epoch,
          !.candidates = r.present \ RootObjects]
DeletePlan ==
 /\ r.up /\ r.planValid /\ r.planEpoch = r.epoch
 /\ r' = [r EXCEPT !.present = @ \ r.candidates, !.planValid = FALSE]
PartialDeleteFailure(os) ==
 /\ r.up /\ r.planValid /\ r.planEpoch = r.epoch /\ os \subseteq r.candidates
 /\ r' = [r EXCEPT !.present = @ \ os, !.planValid = FALSE, !.outcome = "ioFailed"]
RejectStalePlan ==
 /\ r.up /\ r.planValid /\ r.planEpoch # r.epoch
 /\ r' = [r EXCEPT !.planValid = FALSE, !.outcome = "stale"]
FailedIO == /\ r.up /\ r' = [r EXCEPT !.outcome = "ioFailed"]
Crash ==
 /\ r.up
 /\ r' = [r EXCEPT !.up = FALSE, !.outcome = "uncertain",
          !.phase = [q \in Readers |-> IF r.phase[q] = "idle"
                     THEN "idle" ELSE "uncertain"]]
Restart == /\ ~r.up /\ r' = [r EXCEPT !.up = TRUE, !.outcome = "restarted"]
ReaderStep == \E q \in Readers : AcquirePin(q) \/ ReadPage(q) \/ Cancel(q)
                                \/ CompleteTeardown(q) \/ ReleasePin(q)
RootStep == (\E k \in RootIds, os \in SUBSET Objects : RetainRoot(k, os) \/ RetireRoot(k, os))
            \/ (\E g \in Releases : PublishCAS(g) \/ WriteObjects(g))
OtherStep == PlanGC \/ DeletePlan \/ RejectStalePlan \/ FailedIO \/ Crash \/ Restart
             \/ (\E os \in SUBSET Objects : PartialDeleteFailure(os))
Next == ReaderStep \/ RootStep \/ OtherStep
Spec == Init /\ [][Next]_vars

(* Pure set lemmas isolate UNION membership from record/action reasoning. *)
THEOREM RootIdDomains ==
 /\ PublishedId \in RootIds
 /\ (\A g \in Releases : <<"prepared", g>> \in RootIds)
 BY SMT DEF RootIds, Kinds
THEOREM ClosureBound == \A g \in Releases : Closure(g) \subseteq Objects
 BY SMT DEF Closure, Objects
THEOREM EmptyClosure == Closures({}) = {}
 BY SMT DEF Closures
THEOREM SingletonClosure == \A g : Closures({g}) = Closure(g)
 BY DEF Closures, Closure
THEOREM RootMembership == \A roots, pins, obj :
 (obj \in RootSet(roots, pins) <=>
   ((\E k \in RootIds : obj \in roots[k])
    \/ (\E q \in Readers : obj \in Closures(pins[q]))))
 BY SMT DEF RootSet
THEOREM RootObjectBound == \A roots, pins, present, obj :
 Rooted(roots, pins, present) /\ obj \in RootSet(roots, pins) => obj \in present
 BY RootMembership, SMT DEF RootMembership, Rooted
THEOREM RootIncludesOwner == \A roots, pins : \A owner \in RootIds :
 roots[owner] \subseteq RootSet(roots, pins)
 BY RootMembership, SMT DEF RootMembership
THEOREM RootIncludesReader == \A roots, pins : \A reader \in Readers :
 Closures(pins[reader]) \subseteq RootSet(roots, pins)
 BY RootMembership, SMT DEF RootMembership
THEOREM RootedEquivalent == \A roots, pins, present :
 (Rooted(roots, pins, present) <=> RootSet(roots, pins) \subseteq present)
 <1>1. \A roots, pins, present :
   Rooted(roots, pins, present) => RootSet(roots, pins) \subseteq present
   BY RootObjectBound, SMT DEF RootObjectBound
 <1>2. \A roots, pins, present :
   RootSet(roots, pins) \subseteq present => Rooted(roots, pins, present)
   BY RootIncludesOwner, RootIncludesReader, SMT DEF RootIncludesOwner, RootIncludesReader, Rooted
 <1> QED BY <1>1, <1>2
THEOREM RootedGrow == \A roots, pins, old, new :
 Rooted(roots, pins, old) /\ old \subseteq new => Rooted(roots, pins, new)
 BY SMT DEF Rooted
THEOREM RootedDelete == \A roots, pins, present, removed :
 Rooted(roots, pins, present) /\ removed \cap RootSet(roots, pins) = {} =>
 Rooted(roots, pins, present \ removed)
 BY RootedEquivalent, SMT DEF RootedEquivalent
THEOREM RootedPin == \A roots, pins, present, q, g :
 Rooted(roots, pins, present) /\ pins \in [Readers -> SUBSET Releases]
 /\ q \in Readers /\ Closure(g) \subseteq present =>
 Rooted(roots, [pins EXCEPT ![q] = {g}], present)
 BY SingletonClosure, SMT DEF SingletonClosure, Rooted
THEOREM RootedUnpin == \A roots, pins, present, q :
 Rooted(roots, pins, present) /\ pins \in [Readers -> SUBSET Releases]
 /\ q \in Readers =>
 Rooted(roots, [pins EXCEPT ![q] = {}], present)
 BY EmptyClosure, SMT DEF EmptyClosure, Rooted
THEOREM RootedRetain == \A roots, pins, present, k, os :
 Rooted(roots, pins, present) /\ roots \in [RootIds -> SUBSET Objects]
 /\ k \in RootIds /\ os \subseteq present =>
 Rooted([roots EXCEPT ![k] = @ \cup os], pins, present)
 BY SMT DEF Rooted
THEOREM RootedRetire == \A roots, pins, present, k, os :
 Rooted(roots, pins, present) /\ roots \in [RootIds -> SUBSET Objects]
 /\ k \in RootIds =>
 Rooted([roots EXCEPT ![k] = @ \ os], pins, present)
 BY SMT DEF Rooted
THEOREM RootIntegrityEquivalent ==
 RootIntegrity <=> (RootObjects \subseteq r.present
                    /\ Closure(r.head) \subseteq r.roots[PublishedId])
 BY RootedEquivalent DEF RootedEquivalent, RootIntegrity, RootState, RootObjects

THEOREM InitTyped == Environment /\ Init => TypeOK
 BY ClosureBound, SMT DEF ClosureBound, Environment, Init, TypeOK, Typed, StateType, Phases
THEOREM InitRooted == Environment /\ Init => RootIntegrity
 BY ClosureBound, EmptyClosure, RootIdDomains, SMT
 DEF ClosureBound, EmptyClosure, RootIdDomains, Environment, Init, RootIntegrity, RootState, Rooted
THEOREM InitReaders == Init => ReaderIntegrity
 BY SMT DEF Init, ReaderIntegrity, ReaderState
THEOREM InitPlan == Init => PlanIntegrity
 BY SMT DEF Init, PlanIntegrity, PlanState
THEOREM InitInvariant == Environment /\ Init => Inv
 BY InitTyped, InitRooted, InitReaders, InitPlan DEF InitTyped, InitRooted, InitReaders, InitPlan, Inv
THEOREM AcquirePreserves == \A q \in Readers : Environment /\ Inv /\ AcquirePin(q) => Inv'
 <1>1. \A q \in Readers : Inv /\ AcquirePin(q) => TypeOK' /\ ReaderIntegrity'
   BY SMT DEF Inv, TypeOK, Typed, StateType, ReaderIntegrity, ReaderState, AcquirePin, Phases
 <1>2. \A q \in Readers : Inv /\ AcquirePin(q) => RootIntegrity'
   BY RootedPin, RootIdDomains, SMT
   DEF RootedPin, RootIdDomains, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, Rooted, AcquirePin
 <1>3. \A q \in Readers : Inv /\ AcquirePin(q) => PlanIntegrity'
   BY SMT DEF Inv, TypeOK, Typed, StateType, PlanIntegrity, PlanState, AcquirePin
 <1> QED BY <1>1, <1>2, <1>3 DEF Inv
THEOREM ReadPreserves == \A q \in Readers : Inv /\ ReadPage(q) => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            ReaderState, PlanIntegrity, ReadPage
THEOREM CancelPreserves == \A q \in Readers : Inv /\ Cancel(q) => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            ReaderState, PlanIntegrity, Cancel, Phases
THEOREM TeardownPreserves == \A q \in Readers : Inv /\ CompleteTeardown(q) => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            ReaderState, PlanIntegrity, CompleteTeardown
THEOREM ReleasePreserves == \A q \in Readers : Inv /\ ReleasePin(q) => Inv'
 <1>1. \A q \in Readers : Inv /\ ReleasePin(q) => TypeOK' /\ ReaderIntegrity'
   BY SMT DEF Inv, TypeOK, Typed, StateType, ReaderIntegrity, ReaderState, ReleasePin, Phases
 <1>2. \A q \in Readers : Inv /\ ReleasePin(q) => RootIntegrity'
   BY RootedUnpin, SMT DEF RootedUnpin, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReleasePin
 <1>3. \A q \in Readers : Inv /\ ReleasePin(q) => PlanIntegrity'
   BY SMT DEF Inv, TypeOK, Typed, StateType, PlanIntegrity, PlanState, ReleasePin
 <1> QED BY <1>1, <1>2, <1>3 DEF Inv
THEOREM RetainPreserves == \A k \in RootIds, os \in SUBSET Objects : Inv /\ RetainRoot(k, os) => Inv'
 BY RootedRetain, RootIdDomains, SMT
 DEF RootedRetain, RootIdDomains, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReaderIntegrity,
     PlanIntegrity, PlanState, RetainRoot
THEOREM RetirePreserves == \A k \in RootIds, os \in SUBSET Objects : Inv /\ RetireRoot(k, os) => Inv'
 BY RootedRetire, RootIdDomains, SMT
 DEF RootedRetire, RootIdDomains, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReaderIntegrity,
     PlanIntegrity, PlanState, RetireRoot
THEOREM PublishPreserves == \A g \in Releases : Inv /\ PublishCAS(g) => Inv'
 BY RootedRetain, RootIdDomains, SMT
 DEF RootedRetain, RootIdDomains, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, Rooted, ReaderIntegrity,
     PlanIntegrity, PlanState, PublishCAS
THEOREM WritePreserves == \A g \in Releases : Inv /\ WriteObjects(g) => Inv'
 BY ClosureBound, RootedGrow, SMT
 DEF ClosureBound, RootedGrow, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReaderIntegrity,
     PlanIntegrity, WriteObjects
THEOREM PlanPreserves == Inv /\ PlanGC => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            PlanIntegrity, PlanState, RootObjects, PlanGC
THEOREM DeletePreserves == Inv /\ DeletePlan => Inv'
 BY RootedDelete, SMT
 DEF RootedDelete, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReaderIntegrity,
     PlanIntegrity, PlanState, RootObjects, DeletePlan
THEOREM PartialDeletePreserves == \A os \in SUBSET Objects : Inv /\ PartialDeleteFailure(os) => Inv'
 BY RootedDelete, SMT
 DEF RootedDelete, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, ReaderIntegrity,
     PlanIntegrity, PlanState, RootObjects, PartialDeleteFailure
THEOREM RejectPreserves == Inv /\ RejectStalePlan => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            PlanIntegrity, PlanState, RejectStalePlan
THEOREM FailurePreserves == Inv /\ FailedIO => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            PlanIntegrity, FailedIO
THEOREM CrashPreserves == Inv /\ Crash => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            ReaderState, PlanIntegrity, Crash, Phases
THEOREM RestartPreserves == Inv /\ Restart => Inv'
 BY SMT DEF Inv, TypeOK, Typed, StateType, RootIntegrity, ReaderIntegrity,
            PlanIntegrity, Restart
THEOREM StutterPreserves == Inv /\ UNCHANGED vars => Inv'
 BY SMT DEF vars, Inv, TypeOK, RootIntegrity, ReaderIntegrity, PlanIntegrity
THEOREM StepPreserves == Environment /\ Inv /\ [Next]_vars => Inv'
 BY AcquirePreserves, ReadPreserves, CancelPreserves, TeardownPreserves,
    ReleasePreserves, RetainPreserves, RetirePreserves, PublishPreserves,
    WritePreserves, PlanPreserves, DeletePreserves, PartialDeletePreserves, RejectPreserves,
    FailurePreserves, CrashPreserves, RestartPreserves, StutterPreserves
 DEF AcquirePreserves, ReadPreserves, CancelPreserves, TeardownPreserves, ReleasePreserves, RetainPreserves, RetirePreserves, PublishPreserves, WritePreserves, PlanPreserves, DeletePreserves, PartialDeletePreserves, RejectPreserves, FailurePreserves, CrashPreserves, RestartPreserves, StutterPreserves, Next, ReaderStep, RootStep, OtherStep
THEOREM InvariantImpliesSafety == Inv => Safety
 BY SingletonClosure, RootIdDomains, SMT
 DEF SingletonClosure, RootIdDomains, Inv, Safety, RootIntegrity, RootState, Rooted, ReaderIntegrity, ReaderState
THEOREM CancellationFrame == \A q \in Readers : TypeOK /\ Cancel(q) =>
 UNCHANGED <<r.pins, r.present, r.roots, r.tornDown, r.epoch>>
 BY SMT DEF Cancel, TypeOK, Typed, StateType
THEOREM RetireKeepsOtherRoot == \A owner, other \in RootIds, os \in SUBSET Objects :
 TypeOK /\ owner # other /\ RetireRoot(owner, os) =>
 r'.roots[other] = r.roots[other]
 BY SMT DEF TypeOK, Typed, StateType, RetireRoot
THEOREM RootOwnershipSurvivesGC == Inv /\ DeletePlan =>
 (\A owner \in RootIds : r.roots[owner] \subseteq r'.present)
 BY RootedDelete, SMT
 DEF RootedDelete, Inv, TypeOK, Typed, StateType, RootIntegrity, RootState, Rooted,
     PlanIntegrity, PlanState, RootObjects, DeletePlan
THEOREM RootOwnershipSurvivesPartialGC == \A os \in SUBSET Objects :
 Inv /\ PartialDeleteFailure(os) =>
 (\A owner \in RootIds : r.roots[owner] \subseteq r'.present)
 <1>1. \A os \in SUBSET Objects : Inv /\ PartialDeleteFailure(os) =>
   Rooted(r'.roots, r'.pins, r'.present)
   BY PartialDeletePreserves DEF PartialDeletePreserves, Inv, RootIntegrity, RootState
 <1>2. \A os \in SUBSET Objects : TypeOK /\ PartialDeleteFailure(os) =>
   UNCHANGED r.roots
   BY SMT DEF TypeOK, Typed, StateType, PartialDeleteFailure
 <1> QED BY <1>1, <1>2 DEF Inv, Rooted

(* Finite causal workloads. work is a finite-only driver, NOT part of vars,
   unrestricted Init/Next/Inv, or runtime authority. Each immutable candidate:
   pending root -> one closure write -> prepared root -> exact-base publication
   or conflict -> prepared cleanup -> pending cleanup. Failure may abort at any
   boundary; crashes, restart and failed IO never erase the driver's durable
   milestones. A failed candidate retry requires a fresh identity in this finite
   workload; unrestricted Next still permits arbitrary retries.

   Auxiliary identities retain in deterministic identity order before writers
   start. They retire independently after publication/all-writer termination.
   overlap profiles require an actual GC delete between the first and remaining
   auxiliary retirements. The driver never asserts Inv/Safety as a guard.
   One GC worker only plans when no plan is active. Failed deletion represents
   zero, the deterministic first candidate, or all planned objects; retry starts
   a new plan. This is a finite failure representative, NOT deterministic real
   IO. Unrestricted PartialDeleteFailure still permits every subset.
   backup: both old readers admitted first, then candidates publish sequentially
   against newly captured bases, reaching three completed generations at base.
   transfer: reader admission races publication/GC, including late old-pin mutant.
   history/explicit: overlapping same-kind roots, GC between retirements.
   correction: all competing candidates prepared before one CAS; losers conflict.
   Reader cancellation/physical teardown and all failure actions remain enabled
   at every boundary. Physical slots are not these immutable incarnation IDs.
   Control-only conflict/empty-cleanup steps project to stuttering, while each
   protocol effect projects to an unrestricted named Next action. *)
AuxiliaryKinds == {"backup", "transfer", "history", "correction", "explicit"}
Candidates == Releases \ {InitialRelease}
ProfileIds(aux) == {"pending", "prepared", aux} \X Candidates
ProfilePayload(id) ==
 IF id[1] = "pending" THEN {"native", "capture"} \X {id[2]}
 ELSE IF id[1] = "prepared" THEN Closure(id[2]) \cup {<<"capture", InitialRelease>>}
 ELSE Closure(InitialRelease)
WorkPhases == {"new", "pending", "written", "prepared", "published",
               "conflict", "failed", "dropPrepared", "done"}
WorkType ==
 work \in [phase : [Candidates -> WorkPhases], base : [Candidates -> Releases],
           overlapChecked : BOOLEAN, readerAfterClose : BOOLEAN]
WorkInit ==
 work = [phase |-> [g \in Candidates |-> "new"],
         base |-> [g \in Candidates |-> InitialRelease],
         overlapChecked |-> FALSE, readerAfterClose |-> FALSE]
FiniteEnvironment == Environment /\ IsFiniteSet(Readers) /\ IsFiniteSet(Releases)
                     /\ Cardinality(Readers) >= 2 /\ Cardinality(Releases) >= 3
FiniteInit == Init /\ WorkInit
finiteVars == <<r, work>>
WorkMode(aux) ==
 CASE aux = "backup" -> "oldReaders"
   [] aux = "transfer" -> "admissionRace"
   [] aux = "correction" -> "competition"
   [] OTHER -> "overlap"
UnretainedAux(aux) == {g \in Candidates : <<aux, g>> \notin r.retainedRootIds}
ActiveAux(aux) == {g \in Candidates : r.roots[<<aux, g>>] # {}}
WritersDone == \A g \in Candidates : work.phase[g] = "done"
OverlapCheckpoint(aux) ==
 /\ ActiveAux(aux) # {} /\ ActiveAux(aux) # Candidates
 /\ ~ (Closure(InitialRelease) \subseteq r.roots[PublishedId])
 /\ (\A q \in Readers : r.pins[q] = {})
 /\ (\A g \in Candidates : r.roots[<<"prepared", g>>] = {})
BeginAllowed(aux) ==
 /\ UnretainedAux(aux) = {}
 /\ (WorkMode(aux) = "oldReaders" => r.acquiredReaders = Readers)
SequentialTurn(aux, g) ==
 WorkMode(aux) # "oldReaders" \/
 g = CHOOSE next \in {h \in Candidates : work.phase[h] # "done"} : TRUE
WriteAllowed(aux) ==
 WorkMode(aux) # "competition" \/ (\A g \in Candidates : work.phase[g] # "new")
PublishAllowed(aux) ==
 WorkMode(aux) # "competition" \/ (\A g \in Candidates : work.phase[g] = "prepared")
AuxRetireAllowed(aux) ==
 /\ UnretainedAux(aux) = {}
 /\ (r.head # InitialRelease \/ WritersDone)
 /\ (WorkMode(aux) # "overlap" \/ ActiveAux(aux) = Candidates \/ work.overlapChecked
     \/ (WritersDone /\ r.head = InitialRelease))
ReadAfterCloseContext(q) ==
 /\ r.generation[q] = InitialRelease
 /\ (\A g \in Candidates : Closure(g) \subseteq r.roots[PublishedId])
 /\ ~ (Closure(InitialRelease) \subseteq r.roots[PublishedId])
 /\ (\E other \in Readers :
       other # q /\ other \in r.acquiredReaders /\ r.phase[other] = "idle"
       /\ other \in r.tornDown)
RecordReadEvent(q) ==
 work' = [work EXCEPT !.readerAfterClose = @ \/ ReadAfterCloseContext(q)]
FiniteReaderStep ==
 \E q \in Readers :
   \/ (/\ ((q \notin r.acquiredReaders /\ AcquirePin(q))
           \/ Cancel(q) \/ CompleteTeardown(q) \/ ReleasePin(q))
       /\ UNCHANGED work)
   \/ (ReadPage(q) /\ RecordReadEvent(q))
AuxiliaryStep(aux) ==
 \/ (\E g \in UnretainedAux(aux) :
       /\ g = CHOOSE next \in UnretainedAux(aux) : TRUE
       /\ RetainRoot(<<aux, g>>, ProfilePayload(<<aux, g>>))
       /\ UNCHANGED work)
 \/ (\E g \in ActiveAux(aux) :
       /\ AuxRetireAllowed(aux)
       /\ g = CHOOSE next \in ActiveAux(aux) : TRUE
       /\ RetireRoot(<<aux, g>>, r.roots[<<aux, g>>])
       /\ UNCHANGED work)
CandidateStep(aux, g) ==
 \/ (/\ work.phase[g] = "new" /\ BeginAllowed(aux) /\ SequentialTurn(aux, g)
     /\ RetainRoot(<<"pending", g>>, ProfilePayload(<<"pending", g>>))
     /\ work' = [work EXCEPT !.phase[g] = "pending", !.base[g] = r.head])
 \/ (/\ work.phase[g] = "pending" /\ WriteAllowed(aux)
     /\ WriteObjects(g)
     /\ work' = [work EXCEPT !.phase[g] = "written"])
 \/ (/\ work.phase[g] = "written"
     /\ RetainRoot(<<"prepared", g>>, ProfilePayload(<<"prepared", g>>))
     /\ work' = [work EXCEPT !.phase[g] = "prepared"])
 \/ (/\ work.phase[g] = "prepared" /\ r.head = work.base[g] /\ PublishAllowed(aux)
     /\ PublishCAS(g)
     /\ work' = [work EXCEPT !.phase[g] = "published"])
 \/ (/\ work.phase[g] \in {"published", "conflict", "failed"}
     /\ r.roots[<<"prepared", g>>] # {}
     /\ RetireRoot(<<"prepared", g>>, r.roots[<<"prepared", g>>])
     /\ work' = [work EXCEPT !.phase[g] = "dropPrepared"])
 \/ (/\ work.phase[g] = "dropPrepared" /\ r.roots[<<"pending", g>>] # {}
     /\ RetireRoot(<<"pending", g>>, r.roots[<<"pending", g>>])
     /\ work' = [work EXCEPT !.phase[g] = "done"])
FiniteRootStep(aux) ==
 AuxiliaryStep(aux) \/ (\E g \in Candidates : CandidateStep(aux, g))
 \/ (/\ Closure(InitialRelease) \subseteq r.roots[PublishedId]
     /\ RetireRoot(PublishedId, Closure(InitialRelease))
     /\ UNCHANGED work)
FiniteControlStep ==
 /\ r.up
 /\ (\E g \in Candidates :
       \/ (/\ work.phase[g] = "prepared" /\ r.head # work.base[g]
           /\ work' = [work EXCEPT !.phase[g] = "conflict"])
       \/ (/\ work.phase[g] \in {"published", "conflict", "failed"}
           /\ r.roots[<<"prepared", g>>] = {}
           /\ work' = [work EXCEPT !.phase[g] = "dropPrepared"])
       \/ (/\ work.phase[g] = "dropPrepared" /\ r.roots[<<"pending", g>>] = {}
           /\ work' = [work EXCEPT !.phase[g] = "done"]))
 /\ UNCHANGED r
PartialPayloads ==
 {{}, r.candidates} \cup
 (IF r.candidates = {} THEN {} ELSE {{CHOOSE obj \in r.candidates : TRUE}})
FiniteOtherStep(aux) ==
 \/ (/\ ~r.planValid /\ PlanGC /\ UNCHANGED work)
 \/ (/\ DeletePlan
     /\ work' = [work EXCEPT !.overlapChecked = @ \/ OverlapCheckpoint(aux)])
 \/ (/\ (\E os \in PartialPayloads : os \subseteq Objects /\ PartialDeleteFailure(os))
     /\ UNCHANGED work)
 \/ (/\ (RejectStalePlan \/ FailedIO \/ Crash \/ Restart) /\ UNCHANGED work)
 \/ (\E g \in Candidates :
       /\ work.phase[g] \in {"new", "pending", "written", "prepared"}
       /\ FailedIO
       /\ work' = [work EXCEPT !.phase[g] = "failed"])
FiniteNext(aux) ==
 FiniteReaderStep \/ FiniteRootStep(aux) \/ FiniteOtherStep(aux) \/ FiniteControlStep
BackupSpec == FiniteInit /\ [][FiniteNext("backup")]_finiteVars
TransferSpec == FiniteInit /\ [][FiniteNext("transfer")]_finiteVars
HistorySpec == FiniteInit /\ [][FiniteNext("history")]_finiteVars
CorrectionSpec == FiniteInit /\ [][FiniteNext("correction")]_finiteVars
ExplicitSpec == FiniteInit /\ [][FiniteNext("explicit")]_finiteVars
(* Each immutable reader acquires/releases once; each profile root retains/retires
   once; each candidate publishes at most once; the old policy root retires once.
   Thus the epoch bound is derived from event identities (19 base / 28 wide).
   It is checked as an invariant, NEVER used to stop a workload phase. *)
ProfileEpochBound(aux) ==
 r.epoch <= 2 * Cardinality(Readers) + 2 * Cardinality(ProfileIds(aux))
            + Cardinality(Candidates) + 1
BackupEpochBound == ProfileEpochBound("backup")
TransferEpochBound == ProfileEpochBound("transfer")
HistoryEpochBound == ProfileEpochBound("history")
CorrectionEpochBound == ProfileEpochBound("correction")
ExplicitEpochBound == ProfileEpochBound("explicit")
(* Reachability probes are separate expected-failure configurations. Their
   absence predicates must fail on a witness, not on an unrelated safety check. *)
OldReaderIsolationReached ==
 /\ work.readerAfterClose
 /\ (\A g \in Candidates : Closure(g) \subseteq r.roots[PublishedId])
 /\ ~ (Closure(InitialRelease) \subseteq r.roots[PublishedId])
 /\ (\E closed, live \in Readers :
       /\ closed # live /\ closed \in r.acquiredReaders
       /\ r.phase[closed] = "idle" /\ closed \in r.tornDown
       /\ r.phase[live] = "reading" /\ r.generation[live] = InitialRelease
       /\ InitialRelease \in r.seen[live])
OverlapGCReached(aux) ==
 work.overlapChecked /\ OverlapCheckpoint(aux)
CompetitionReached ==
 \E winner, loser \in Candidates :
   /\ winner # loser /\ work.phase[winner] = "published" /\ work.phase[loser] = "conflict"
   /\ work.base[winner] = work.base[loser]
   /\ r.roots[<<"prepared", winner>>] # {} /\ r.roots[<<"prepared", loser>>] # {}
OldReaderWitnessAbsent == ~OldReaderIsolationReached
OverlapWitnessAbsent == ~OverlapGCReached("history")
CompetitionWitnessAbsent == ~CompetitionReached
THEOREM ProfilePayloadTyped == \A aux \in AuxiliaryKinds : \A id \in ProfileIds(aux) :
 FiniteEnvironment => id \in RootIds /\ ProfilePayload(id) \subseteq Objects
 BY SMT DEF FiniteEnvironment, Environment, ProfilePayload, ProfileIds, AuxiliaryKinds,
            Candidates, Closure, Objects, ObjectKinds, RootIds, Kinds
THEOREM FiniteRootRefines == \A aux \in AuxiliaryKinds :
 FiniteEnvironment /\ TypeOK /\ FiniteRootStep(aux) => RootStep
 BY ProfilePayloadTyped, ClosureBound, RootIdDomains, SMT
 DEF ProfilePayloadTyped, ClosureBound, RootIdDomains, FiniteEnvironment, Environment, FiniteRootStep, AuxiliaryStep, CandidateStep,
     RootStep, UnretainedAux, ActiveAux, ProfileIds, Candidates, TypeOK, Typed, StateType
THEOREM FiniteOtherRefines == \A aux \in AuxiliaryKinds :
 FiniteOtherStep(aux) => OtherStep
 BY SMT DEF FiniteOtherStep, OtherStep
THEOREM FiniteReaderRefines == FiniteReaderStep => ReaderStep
 BY DEF FiniteReaderStep, ReaderStep
THEOREM FiniteControlFrame == FiniteControlStep => UNCHANGED vars
 BY DEF FiniteControlStep, vars
THEOREM FiniteInitRefines == FiniteInit => Init
 BY DEF FiniteInit
THEOREM FiniteStepRefines == \A aux \in AuxiliaryKinds :
 FiniteEnvironment /\ TypeOK /\ FiniteNext(aux) => [Next]_vars
 BY FiniteRootRefines, FiniteOtherRefines, FiniteReaderRefines, FiniteControlFrame
 DEF FiniteRootRefines, FiniteOtherRefines, FiniteReaderRefines, FiniteControlFrame, FiniteNext, Next
THEOREM FiniteStepPreserves == \A aux \in AuxiliaryKinds :
 FiniteEnvironment /\ Inv /\ [FiniteNext(aux)]_finiteVars => Inv'
 BY FiniteStepRefines, StepPreserves, StutterPreserves
 DEF FiniteStepRefines, StepPreserves, StutterPreserves, FiniteEnvironment, Inv, finiteVars, vars
THEOREM WorkInitTyped == FiniteEnvironment /\ WorkInit => WorkType
 BY SMT DEF FiniteEnvironment, Environment, WorkInit, WorkType, WorkPhases
THEOREM WorkStepTyped == \A aux \in AuxiliaryKinds :
 TypeOK /\ WorkType /\ [FiniteNext(aux)]_finiteVars => WorkType'
 BY SMT DEF TypeOK, Typed, StateType, WorkType, WorkPhases, finiteVars,
            FiniteNext, FiniteReaderStep, FiniteRootStep, AuxiliaryStep, CandidateStep,
            FiniteOtherStep, FiniteControlStep, RecordReadEvent

(***************************************************************************
Semantic negative controls; use the matching .cfg, expect invariant failure.
PinRace: pin an earlier unlocked resolution without revalidation/epoch change.
StaleEpoch: delete a planned set even after a root-epoch change.
EarlyCancellation: physically drop the pin on request cancellation, before
certified teardown. Not parser mutations; each is an executable action change.
***************************************************************************)
PinRace(q, g) ==
 /\ r.up /\ r.phase[q] = "idle" /\ Closure(g) \subseteq r.present
 /\ g = r.generation[q]
 /\ r' = [r EXCEPT !.pins[q] = {g}, !.generation[q] = g,
          !.phase[q] = "reading", !.seen[q] = {}, !.tornDown = @ \ {q},
          !.acquiredReaders = @ \cup {q}]
StaleEpochDelete ==
 /\ r.up /\ r.planValid
 /\ r' = [r EXCEPT !.present = @ \ r.candidates, !.planValid = FALSE]
EarlyCancellation(q) ==
 /\ r.up /\ r.phase[q] = "reading"
 /\ r' = [r EXCEPT !.phase[q] = "draining", !.pins[q] = {}]
PinRaceNext == FiniteNext("transfer") \/
 (/\ (\E q \in Readers, g \in Releases : q \notin r.acquiredReaders /\ PinRace(q, g))
  /\ UNCHANGED work)
StaleEpochNext == FiniteNext("transfer") \/ (StaleEpochDelete /\ UNCHANGED work)
EarlyCancellationNext == FiniteNext("backup") \/
 ((\E q \in Readers : EarlyCancellation(q)) /\ UNCHANGED work)
PinRaceSpec == FiniteInit /\ [][PinRaceNext]_finiteVars
StaleEpochSpec == FiniteInit /\ [][StaleEpochNext]_finiteVars
EarlyCancellationSpec == FiniteInit /\ [][EarlyCancellationNext]_finiteVars
=============================================================================
