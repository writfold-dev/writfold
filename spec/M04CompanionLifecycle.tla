----------------------- MODULE M04CompanionLifecycle -----------------------
EXTENDS Naturals, FiniteSets, TLAPS
CONSTANTS Incarnations, Pids, Children, Owner, Instance, KnowledgeBase
VARIABLE c
vars == <<c>>
Tools == {1, 2}
Formats == {1, 2}
Executable(t) == <<"admitted-executable-digest", t>>
Compatible(t, f) == f <= t
Identity(i, p, t) == [owner |-> Owner, instance |-> Instance,
                      bootStart |-> i, pid |-> p, executable |-> Executable(t)]
Phases == {"idle", "starting", "listening", "ready", "draining", "killing",
           "failed", "uncertain", "upgrading"}
Outcomes == {"ok", "installFailed", "startFailed", "queryFailed", "ioFailed",
             "upgradeFailed", "uncertain", "identityConflict", "restarted"}
Environment == /\ Incarnations # {} /\ Pids # {}
               /\ Incarnations \cap {"none", "foreign"} = {}
(***************************************************************************
Owner = supervisor/session owner; Instance = local instance, NOT logical KB.
bootStart is an opaque non-reused boot/start incarnation, never a request
number or PID. Identity also binds the admitted executable. Children denotes
owned descendant INCARNATIONS, not a list of reusable numeric PIDs. The adapter
must retain the complete descendant identity/cgroup/pidfd ownership evidence.

Tools 1/2 are finite immutable package/version/platform/provenance/executable
identities, not a numeric version parser. Compatible is the finite verified
compatibility table; production supplies explicit declared/read-tested format
compatibility. Data formats are distinct from tool versions. Installed slots
are immutable: Install only adds an admitted identity. FailedInstall never
makes partial bytes executable. No untrusted package is admitted by this model.

Spawn = observed exec of immutable admitted bytes with owned pidfd/start/boot,
explicit environment/cwd/socket/auth and instance lock. Listening proves no
readiness. QueryReady requires the adapter's successful identity/version/DB
query, bound to exact incarnation and KnowledgeBase; it is not a port probe.
ParentExit/ChildExit are certified observations, not assumed timeout deaths.
PidReuse is an external process replacing a DEAD owned parent at the same PID.
SignalParent requires identity-safe delivery (for example a qualified pidfd),
not a racy inspect-PID-then-kill sequence. Descendant escalation likewise targets
exact owned Children identities. Request generation is deliberately not ownership.
Cancel blocks result admission but retains capacity; ObserveQuiescent and Reap
require all owned descendants dead and final wait/reap evidence. Crash/restart
retain ownership, charges and uncertainty. A failed/unknown start is not idle.

VerifyBackup abstracts independent restore verification with source inaccessible.
PlanUpgrade binds old tool/format, new tool/format and verified backup; changing
format requires that backup. BeginUpgrade reserves exclusive capacity. Failed
upgrade leaves original format; interrupted upgrade reconciles OLD or NEW from
observed storage evidence, never guesses. Rollback selects an old executable
ONLY if readable at CURRENT format; no action downgrades live data. Restoring a
separate backup is intentionally a distinct protocol, not an implicit rollback.

Liveness is conditional: eventually stable supervisor/IO, eventual child exit
under qualified bounded escalation, fair readiness/teardown/reap/reconciliation,
and no endless crash/install failure. Finite incarnation exhaustion blocks spawn
rather than reusing identity. No temporal progress theorem or adapter is claimed.
***************************************************************************)
(* Exact record typing, decomposed for proof search rather than weakened.
   The reconstruction clause is essential: field/domain facts alone do not
   establish that an arbitrary TLA+ value is a function. *)
LifecycleFields ==
 {"admitted", "selected", "format", "backups", "plan", "phase", "inc", "pid",
  "tool", "used", "actual", "launched", "signals", "alive", "descendants",
  "childSignals", "charged", "reaped", "teardownKnown", "admitResults", "up",
  "outcome", "readyKB"}
StateShape(s) == s = [field \in LifecycleFields |-> s[field]]
ToolTypes(s) ==
 /\ s.admitted \subseteq Tools /\ s.selected \in Tools /\ s.format \in Formats
 /\ s.backups \subseteq Formats /\ s.tool \in Tools
 /\ s.plan \in [valid : BOOLEAN, oldTool : Tools, newTool : Tools,
               oldFormat : Formats, newFormat : Formats]
ProcessTypes(s) ==
 /\ s.inc \in Incarnations /\ s.pid \in Pids /\ s.used \subseteq Incarnations
 /\ s.actual \in [Pids -> Incarnations \cup {"none", "foreign"}]
 /\ s.launched \subseteq (Pids \X Incarnations)
 /\ s.signals \subseteq (Pids \X (Incarnations \cup {"foreign", "none"}))
 /\ s.descendants \subseteq Children /\ s.childSignals \subseteq Children
ControlTypes(s) ==
 /\ s.phase \in Phases /\ s.alive \in BOOLEAN /\ s.charged \in BOOLEAN
 /\ s.reaped \in BOOLEAN /\ s.teardownKnown \in BOOLEAN
 /\ s.admitResults \in BOOLEAN /\ s.up \in BOOLEAN
 /\ s.outcome \in Outcomes /\ s.readyKB \in {KnowledgeBase, "none"}
LifecycleType(s) ==
 StateShape(s) /\ ToolTypes(s) /\ ProcessTypes(s) /\ ControlTypes(s)
TypeOK == LifecycleType(c)
(* Keep exact reconstruction abstract at action sites. These proved function
   laws expose only the domain/update/select facts an action needs; repeatedly
   unfolding the self-reconstruction equality causes unrelated SMT instantiation. *)
THEOREM ExactShapeDomain == \A s : StateShape(s) => DOMAIN s = LifecycleFields
 BY SMT DEF StateShape
THEOREM ExactShapeUpdate == \A s, field, value :
 StateShape(s) /\ field \in LifecycleFields =>
 StateShape([s EXCEPT ![field] = value])
 BY SMT DEF StateShape
THEOREM ExactShapeSelect == \A s, field, value, selected :
 StateShape(s) /\ field \in LifecycleFields /\ selected \in LifecycleFields =>
 [s EXCEPT ![field] = value][selected] =
   (IF selected = field THEN value ELSE s[selected])
 BY SMT DEF StateShape
THEOREM ExactNestedUpdate == \A s, field, inner, value :
 StateShape(s) /\ field \in LifecycleFields =>
 [s EXCEPT ![field][inner] = value] =
 [s EXCEPT ![field] = [s[field] EXCEPT ![inner] = value]]
 BY SMT DEF StateShape
THEOREM ExactShapeReconstruct == \A s, t :
 StateShape(s) /\ StateShape(t) => t = [field \in DOMAIN s |-> t[field]]
 BY ExactShapeDomain, SMT DEF ExactShapeDomain, StateShape
USE ExactShapeDomain, ExactShapeUpdate, ExactShapeSelect, ExactNestedUpdate, ExactShapeReconstruct
USE DEF ExactShapeDomain, ExactShapeUpdate, ExactShapeSelect, ExactNestedUpdate, ExactShapeReconstruct,
        LifecycleType, LifecycleFields, ToolTypes, ProcessTypes, ControlTypes
THEOREM TypeShape == \A s : LifecycleType(s) => StateShape(s)
 BY SMT DEF LifecycleType
THEOREM TypeTools == \A s : LifecycleType(s) => ToolTypes(s)
 BY SMT DEF LifecycleType
THEOREM TypeProcesses == \A s : LifecycleType(s) => ProcessTypes(s)
 BY SMT DEF LifecycleType
THEOREM TypeControls == \A s : LifecycleType(s) => ControlTypes(s)
 BY SMT DEF LifecycleType
THEOREM TypeConstruct == \A s :
 StateShape(s) /\ ToolTypes(s) /\ ProcessTypes(s) /\ ControlTypes(s) => LifecycleType(s)
 BY SMT DEF LifecycleType
Admission == /\ c.selected \in c.admitted /\ Compatible(c.selected, c.format)
             /\ c.tool \in c.admitted
Ownership ==
 /\ c.signals \subseteq c.launched
 /\ (c.alive =>
       /\ c.actual[c.pid] = c.inc
       /\ <<c.pid, c.inc>> \in c.launched
       /\ c.inc \in c.used)
 /\ (c.alive \/ c.descendants # {} => c.charged /\ ~c.reaped)
 /\ (c.teardownKnown => ~c.alive /\ c.descendants = {})
 /\ (c.reaped => ~c.charged /\ ~c.alive /\ c.descendants = {})
 /\ (~c.charged => c.phase = "idle")
 /\ (c.phase = "idle" => c.reaped /\ ~c.charged)
Readiness ==
 /\ (c.phase = "ready" =>
       /\ c.charged
       /\ c.alive
       /\ c.readyKB = KnowledgeBase
       /\ Compatible(c.tool, c.format))
 /\ (c.admitResults => c.phase = "ready")
UpgradeIntegrity ==
 /\ (c.phase = "upgrading" =>
       /\ c.plan.valid
       /\ ~c.alive
       /\ c.descendants = {}
       /\ c.charged
       /\ c.plan.oldTool = c.selected
       /\ c.plan.oldFormat = c.format)
 /\ (c.plan.valid =>
       /\ c.plan.newTool \in c.admitted
       /\ c.plan.oldTool = c.selected
       /\ c.plan.oldFormat = c.format
       /\ c.phase \in {"idle", "upgrading", "uncertain"}
       /\ ~c.alive
       /\ c.descendants = {}
       /\ Compatible(c.plan.newTool, c.plan.newFormat)
       /\ c.plan.oldFormat <= c.plan.newFormat
       /\ (c.plan.newFormat # c.plan.oldFormat => c.plan.oldFormat \in c.backups))
Inv == TypeOK /\ Admission /\ Ownership /\ Readiness /\ UpgradeIntegrity
Safety ==
 /\ c.signals \subseteq c.launched
 /\ (c.alive \/ c.descendants # {} => c.charged)
 /\ (c.alive => c.tool \in c.admitted)
 /\ (c.phase = "ready" => c.readyKB = KnowledgeBase /\ Compatible(c.tool, c.format))
 /\ Compatible(c.selected, c.format)
InitialPlan == [valid |-> FALSE, oldTool |-> 1, newTool |-> 1, oldFormat |-> 1, newFormat |-> 1]
Init ==
 c = [admitted |-> {1}, selected |-> 1, format |-> 1, backups |-> {},
      plan |-> InitialPlan,
      phase |-> "idle", inc |-> CHOOSE i \in Incarnations : TRUE,
      pid |-> CHOOSE p \in Pids : TRUE, tool |-> 1, used |-> {},
      actual |-> [p \in Pids |-> "none"], launched |-> {}, signals |-> {},
      alive |-> FALSE, descendants |-> {}, childSignals |-> {}, charged |-> FALSE, reaped |-> TRUE,
      teardownKnown |-> TRUE, admitResults |-> FALSE, up |-> TRUE,
      outcome |-> "ok", readyKB |-> "none"]
Install(t) == /\ c.up /\ c' = [c EXCEPT !.admitted = @ \cup {t}, !.outcome = "ok"]
FailedInstall == /\ c.up /\ c' = [c EXCEPT !.outcome = "installFailed"]
Spawn(i, p) ==
 /\ c.up /\ c.phase = "idle" /\ ~c.plan.valid /\ i \notin c.used /\ c.actual[p] = "none"
 /\ c' = [c EXCEPT !.phase = "starting", !.inc = i, !.pid = p,
          !.tool = c.selected, !.used = @ \cup {i}, !.actual[p] = i,
          !.launched = @ \cup {<<p, i>>}, !.alive = TRUE,
          !.descendants = Children, !.charged = TRUE, !.reaped = FALSE,
          !.teardownKnown = FALSE, !.readyKB = "none", !.outcome = "ok"]
Listening == /\ c.up /\ c.phase = "starting"
             /\ c' = [c EXCEPT !.phase = "listening"]
QueryReady ==
 /\ c.up /\ c.phase = "listening" /\ c.alive /\ c.actual[c.pid] = c.inc
 /\ Compatible(c.tool, c.format)
 /\ c' = [c EXCEPT !.phase = "ready", !.readyKB = KnowledgeBase, !.admitResults = TRUE]
StartFailed ==
 /\ c.up /\ c.phase \in {"starting", "listening"}
 /\ c' = [c EXCEPT !.phase = "failed", !.outcome = "startFailed", !.admitResults = FALSE]
QueryFailed ==
 /\ c.up /\ c.phase = "ready"
 /\ c' = [c EXCEPT !.phase = "draining", !.outcome = "queryFailed", !.admitResults = FALSE]
Stop ==
 /\ c.up /\ c.charged /\ c.phase # "upgrading" /\ ~c.plan.valid
 /\ c' = [c EXCEPT !.phase = "draining", !.admitResults = FALSE]
Cancel == Stop
TimeoutEscalation ==
 /\ c.up /\ c.phase \in {"draining", "failed", "uncertain"}
 /\ ~c.plan.valid
 /\ c' = [c EXCEPT !.phase = "killing", !.admitResults = FALSE]
SignalParent ==
 /\ c.up /\ c.phase \in {"draining", "killing"}
 /\ c.actual[c.pid] = c.inc /\ <<c.pid, c.inc>> \in c.launched
 /\ c' = [c EXCEPT !.signals = @ \cup {<<c.pid, c.actual[c.pid]>>}]
SignalDescendant(d) ==
 /\ c.up /\ c.phase \in {"draining", "killing"} /\ d \in c.descendants
 /\ c' = [c EXCEPT !.childSignals = @ \cup {d}]
RejectWrongIncarnation ==
 /\ c.up /\ c.charged /\ c.actual[c.pid] # c.inc
 /\ c' = [c EXCEPT !.outcome = "identityConflict"]
ParentExit ==
 /\ c.alive
 /\ c' = [c EXCEPT !.alive = FALSE, !.actual[c.pid] = "none",
          !.phase = "draining", !.admitResults = FALSE]
ChildExit(d) == /\ d \in c.descendants
                /\ c' = [c EXCEPT !.descendants = @ \ {d}]
PidReuse == /\ ~c.alive /\ c.charged /\ c.phase # "upgrading"
           /\ c' = [c EXCEPT !.actual[c.pid] = "foreign"]
ObserveQuiescent ==
 /\ c.up /\ c.charged /\ ~c.alive /\ c.descendants = {}
 /\ c.phase # "upgrading" /\ ~c.plan.valid
 /\ c' = [c EXCEPT !.teardownKnown = TRUE]
Reap ==
 /\ c.up /\ c.charged /\ c.teardownKnown /\ ~c.alive /\ c.descendants = {}
 /\ c.phase # "upgrading" /\ ~c.plan.valid
 /\ c' = [c EXCEPT !.phase = "idle", !.charged = FALSE, !.reaped = TRUE,
          !.admitResults = FALSE, !.readyKB = "none"]
VerifyBackup == /\ c.up /\ c.phase = "idle"
                /\ c' = [c EXCEPT !.backups = @ \cup {c.format}]
PlanUpgrade(t, f) ==
 /\ c.up /\ c.phase = "idle" /\ t \in c.admitted
 /\ Compatible(t, f) /\ c.format <= f /\ (f # c.format => c.format \in c.backups)
 /\ c' = [c EXCEPT !.plan = [valid |-> TRUE, oldTool |-> c.selected,
                newTool |-> t, oldFormat |-> c.format, newFormat |-> f]]
BeginUpgrade ==
 /\ c.up /\ c.phase = "idle" /\ c.plan.valid
 /\ c.plan.oldTool = c.selected /\ c.plan.oldFormat = c.format
 /\ c' = [c EXCEPT !.phase = "upgrading", !.charged = TRUE,
          !.reaped = FALSE, !.teardownKnown = FALSE]
CompleteUpgrade ==
 /\ c.up /\ c.phase = "upgrading"
 /\ c' = [c EXCEPT !.selected = c.plan.newTool, !.format = c.plan.newFormat,
          !.plan.valid = FALSE, !.phase = "idle", !.charged = FALSE,
          !.reaped = TRUE, !.teardownKnown = TRUE, !.outcome = "ok"]
FailedUpgrade ==
 /\ c.up /\ c.phase = "upgrading"
 /\ c' = [c EXCEPT !.plan.valid = FALSE, !.phase = "idle", !.charged = FALSE,
          !.reaped = TRUE, !.teardownKnown = TRUE, !.outcome = "upgradeFailed"]
ReconcileUpgrade(applied) ==
 /\ c.up /\ c.phase = "uncertain" /\ c.plan.valid /\ ~c.alive /\ c.descendants = {}
 /\ c' = [c EXCEPT !.selected = IF applied THEN c.plan.newTool ELSE c.selected,
          !.format = IF applied THEN c.plan.newFormat ELSE c.format,
          !.plan.valid = FALSE, !.phase = "idle", !.charged = FALSE,
          !.reaped = TRUE, !.teardownKnown = TRUE, !.outcome = "ok"]
Rollback(t) ==
 /\ c.up /\ c.phase = "idle" /\ t \in c.admitted /\ Compatible(t, c.format)
 /\ c' = [c EXCEPT !.selected = t, !.plan.valid = FALSE]
FailedIO == /\ c.up /\ c' = [c EXCEPT !.outcome = "ioFailed"]
Crash ==
 /\ c.up
 /\ c' = [c EXCEPT !.up = FALSE, !.outcome = "uncertain", !.admitResults = FALSE,
          !.phase = IF c.charged THEN "uncertain" ELSE "idle"]
Restart == /\ ~c.up /\ c' = [c EXCEPT !.up = TRUE, !.outcome = "restarted"]
ToolStep == (\E t \in Tools : Install(t) \/ Rollback(t)) \/ FailedInstall \/ VerifyBackup
            \/ (\E t \in Tools, f \in Formats : PlanUpgrade(t, f))
            \/ BeginUpgrade \/ CompleteUpgrade \/ FailedUpgrade
            \/ (\E applied \in BOOLEAN : ReconcileUpgrade(applied))
ProcessStep == (\E i \in Incarnations, p \in Pids : Spawn(i, p))
               \/ Listening \/ QueryReady \/ StartFailed \/ QueryFailed \/ Stop \/ Cancel
               \/ TimeoutEscalation \/ SignalParent \/ RejectWrongIncarnation
               \/ ParentExit \/ (\E d \in Children : ChildExit(d) \/ SignalDescendant(d)) \/ PidReuse
               \/ ObserveQuiescent \/ Reap
Next == ToolStep \/ ProcessStep \/ FailedIO \/ Crash \/ Restart
Spec == Init /\ [][Next]_vars

(* Proof-only effect projections. These are derived from the actual actions;
   none is an alternative transition, environment assumption or safety guard. *)
SemanticFields ==
 <<c.admitted, c.selected, c.format, c.tool, c.signals, c.launched,
   c.alive, c.actual, c.pid, c.inc, c.used, c.descendants, c.charged,
   c.reaped, c.teardownKnown, c.phase, c.readyKB, c.admitResults,
   c.plan, c.backups>>
CoreInvariant == Admission /\ Ownership /\ Readiness /\ UpgradeIntegrity
THEOREM CoreFrame ==
 CoreInvariant /\ UNCHANGED SemanticFields => CoreInvariant'
 BY SMT DEF CoreInvariant, SemanticFields, Admission, Ownership, Readiness, UpgradeIntegrity
DiagnosticStep == FailedIO \/ Restart
THEOREM DiagnosticTyped == TypeOK /\ DiagnosticStep => TypeOK'
 BY SMT DEF TypeOK, DiagnosticStep, FailedIO, Restart, Outcomes
THEOREM DiagnosticFrame == TypeOK /\ DiagnosticStep => UNCHANGED SemanticFields
 BY SMT DEF TypeOK, DiagnosticStep, FailedIO, Restart, SemanticFields
THEOREM DiagnosticPreserves == Inv /\ DiagnosticStep => Inv'
 BY DiagnosticTyped, DiagnosticFrame, CoreFrame DEF DiagnosticTyped, DiagnosticFrame, CoreFrame, Inv, CoreInvariant

(* Factor the actual EXCEPT chain for proof search only. Spawn itself and its
   guards are unchanged; each projection traverses at most one update stage. *)
SpawnIdentityState(i,p) ==
 [c EXCEPT !.phase="starting", !.inc=i, !.pid=p, !.tool=c.selected,
   !.used=@ \cup {i}, !.actual[p]=i, !.launched=@ \cup {<<p,i>>}]
SpawnResourceState(s) ==
 [s EXCEPT !.alive=TRUE, !.descendants=Children, !.charged=TRUE,
   !.reaped=FALSE, !.teardownKnown=FALSE, !.readyKB="none", !.outcome="ok"]
THEOREM SpawnFactor == \A i,p :
 Spawn(i,p) => c'=SpawnResourceState(SpawnIdentityState(i,p))
 BY ONLY DEF Spawn, SpawnIdentityState, SpawnResourceState
THEOREM SpawnGuard == \A i,p : Spawn(i,p) => c.phase="idle" /\ ~c.plan.valid
 BY ONLY DEF Spawn
THEOREM SpawnIdentityProjection == \A i,p : StateShape(c) =>
 LET s == SpawnIdentityState(i,p) IN
 /\ StateShape(s)
 /\ s.phase="starting" /\ s.inc=i /\ s.pid=p /\ s.tool=c.selected
 /\ s.used=c.used \cup {i} /\ s.actual=[c.actual EXCEPT ![p]=i]
 /\ s.launched=c.launched \cup {<<p,i>>}
 /\ <<s.admitted,s.selected,s.format,s.backups,s.plan,s.signals,
      s.childSignals,s.admitResults,s.up>> =
    <<c.admitted,c.selected,c.format,c.backups,c.plan,c.signals,
      c.childSignals,c.admitResults,c.up>>
 BY ONLY SMT, ExactShapeUpdate, ExactShapeSelect, ExactNestedUpdate
 DEF ExactShapeUpdate, ExactShapeSelect, ExactNestedUpdate, LifecycleFields, SpawnIdentityState
THEOREM SpawnResourceProjection == \A s : StateShape(s) =>
 LET t == SpawnResourceState(s) IN
 /\ StateShape(t)
 /\ t.alive /\ t.descendants=Children /\ t.charged /\ ~t.reaped
 /\ ~t.teardownKnown /\ t.readyKB="none" /\ t.outcome="ok"
 /\ <<t.phase,t.inc,t.pid,t.tool,t.used,t.actual,t.launched>> =
    <<s.phase,s.inc,s.pid,s.tool,s.used,s.actual,s.launched>>
 /\ <<t.admitted,t.selected,t.format,t.backups,t.plan,t.signals,
      t.childSignals,t.admitResults,t.up>> =
    <<s.admitted,s.selected,s.format,s.backups,s.plan,s.signals,
      s.childSignals,s.admitResults,s.up>>
 BY ONLY SMT, ExactShapeUpdate, ExactShapeSelect
 DEF ExactShapeUpdate, ExactShapeSelect, LifecycleFields, SpawnResourceState
SpawnChanged(i, p) ==
 /\ c.phase = "idle" /\ ~c.plan.valid
 /\ c'.phase = "starting" /\ c'.inc = i /\ c'.pid = p
 /\ c'.tool = c.selected /\ c'.used = c.used \cup {i}
 /\ c'.actual = [c.actual EXCEPT ![p] = i]
 /\ c'.launched = c.launched \cup {<<p, i>>}
 /\ c'.alive = TRUE /\ c'.descendants = Children
 /\ c'.charged = TRUE /\ c'.reaped = FALSE /\ c'.teardownKnown = FALSE
 /\ c'.readyKB = "none" /\ c'.outcome = "ok"
SpawnStable ==
 /\ c' = [field \in DOMAIN c |-> c'[field]]
 /\ UNCHANGED <<c.admitted, c.selected, c.format, c.backups, c.plan,
               c.signals, c.childSignals, c.admitResults, c.up>>
THEOREM SpawnChanges == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Spawn(i, p) => SpawnChanged(i, p)
 <1>1. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) =>
   /\ c.phase = "idle" /\ ~c.plan.valid /\ c'.phase = "starting"
   /\ c'.inc = i /\ c'.pid = p /\ c'.tool = c.selected
   BY SpawnGuard, SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, SMT
   DEF SpawnGuard, SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, TypeOK
 <1>2. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) =>
   /\ c'.used = c.used \cup {i}
   /\ c'.actual = [c.actual EXCEPT ![p] = i]
   /\ c'.launched = c.launched \cup {<<p, i>>}
   BY SMT DEF TypeOK, Spawn
 <1>3. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) =>
   /\ c'.alive = TRUE /\ c'.descendants = Children /\ c'.charged = TRUE
   /\ c'.reaped = FALSE /\ c'.teardownKnown = FALSE
   /\ c'.readyKB = "none" /\ c'.outcome = "ok"
   BY SMT DEF TypeOK, Spawn
 <1> QED BY <1>1, <1>2, <1>3 DEF SpawnChanged
THEOREM SpawnFrame == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Spawn(i, p) => SpawnStable
 <1>1. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) => StateShape(c')
   BY SMT DEF TypeOK, Spawn
 <1>2. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) =>
   UNCHANGED <<c.admitted, c.selected, c.format, c.backups, c.plan,
               c.signals, c.childSignals, c.admitResults, c.up>>
   <2>1. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i,p) =>
     UNCHANGED <<c.admitted, c.selected, c.format>>
     BY SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, SMT
     DEF SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, TypeOK
   <2>2. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i,p) =>
     UNCHANGED <<c.backups, c.plan, c.signals>>
     BY SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, SMT
     DEF SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, TypeOK
   <2>3. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i,p) =>
     UNCHANGED <<c.childSignals, c.admitResults, c.up>>
     BY SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, SMT
     DEF SpawnFactor, SpawnIdentityProjection, SpawnResourceProjection, TypeOK
   <2> QED BY <2>1, <2>2, <2>3
 <1> QED BY <1>1, <1>2, ExactShapeReconstruct DEF ExactShapeReconstruct, TypeOK, SpawnStable
THEOREM SpawnTyped == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Spawn(i, p) => TypeOK'
 <1>1. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) => StateShape(c')
   BY SpawnFrame, TypeShape, SMT DEF SpawnFrame, TypeShape, TypeOK, SpawnStable, StateShape
 <1>2. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) => ToolTypes(c')
   BY SpawnChanges, SpawnFrame, TypeTools, SMT
   DEF SpawnChanges, SpawnFrame, TypeTools, TypeOK, SpawnChanged, SpawnStable, ToolTypes
 <1>3. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) => ProcessTypes(c')
   BY SpawnChanges, SpawnFrame, TypeProcesses, SMT
   DEF SpawnChanges, SpawnFrame, TypeProcesses, TypeOK, SpawnChanged, SpawnStable, ProcessTypes
 <1>4. \A i \in Incarnations, p \in Pids : TypeOK /\ Spawn(i, p) => ControlTypes(c')
   BY SpawnChanges, SpawnFrame, TypeControls, SMT
   DEF SpawnChanges, SpawnFrame, TypeControls, TypeOK, SpawnChanged, SpawnStable, ControlTypes, Phases, Outcomes
 <1> QED BY <1>1, <1>2, <1>3, <1>4, TypeConstruct DEF TypeConstruct, TypeOK
THEOREM SpawnAdmission == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Admission /\ Spawn(i, p) => Admission'
 BY SpawnChanges, SpawnFrame, SMT DEF SpawnChanges, SpawnFrame, SpawnChanged, SpawnStable, Admission
THEOREM SpawnOwnership == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Ownership /\ Spawn(i, p) => Ownership'
 BY SpawnChanges, SpawnFrame, SMT DEF SpawnChanges, SpawnFrame, TypeOK, SpawnChanged, SpawnStable, Ownership
THEOREM SpawnControl == \A i \in Incarnations, p \in Pids :
 TypeOK /\ Readiness /\ Spawn(i, p) => Readiness' /\ UpgradeIntegrity'
 BY SpawnChanges, SpawnFrame, SMT
 DEF SpawnChanges, SpawnFrame, TypeOK, SpawnChanged, SpawnStable, Readiness, UpgradeIntegrity

UpgradeExit == CompleteUpgrade \/ FailedUpgrade
               \/ (\E applied \in BOOLEAN : ReconcileUpgrade(applied))
ExitEffects ==
 /\ c.phase \in {"upgrading", "uncertain"}
 /\ (c.phase = "uncertain" => c.plan.valid)
 /\ c' = [field \in DOMAIN c |-> c'[field]]
 /\ c'.phase = "idle" /\ c'.charged = FALSE /\ c'.reaped = TRUE
 /\ c'.teardownKnown = TRUE
 /\ c'.plan = [c.plan EXCEPT !.valid = FALSE]
 /\ (<<c'.selected, c'.format>> = <<c.plan.newTool, c.plan.newFormat>>
     \/ <<c'.selected, c'.format>> = <<c.selected, c.format>>)
 /\ c'.outcome \in {"ok", "upgradeFailed"}
 /\ UNCHANGED <<c.admitted, c.backups, c.inc, c.pid, c.tool, c.used,
               c.actual, c.launched, c.signals, c.alive, c.descendants,
               c.childSignals, c.admitResults, c.up, c.readyKB>>
THEOREM CompletedExitEffects == TypeOK /\ CompleteUpgrade => ExitEffects
 BY SMT DEF TypeOK, CompleteUpgrade, ExitEffects
THEOREM FailedExitEffects == TypeOK /\ FailedUpgrade => ExitEffects
 BY SMT DEF TypeOK, FailedUpgrade, ExitEffects
THEOREM ReconciledExitEffects == \A applied \in BOOLEAN :
 TypeOK /\ ReconcileUpgrade(applied) => ExitEffects
 BY SMT DEF TypeOK, ReconcileUpgrade, ExitEffects
THEOREM ExitTyped == TypeOK /\ ExitEffects => TypeOK'
 <1>1. TypeOK /\ ExitEffects => StateShape(c')
   BY TypeShape, SMT DEF TypeShape, TypeOK, ExitEffects, StateShape
 <1>2. TypeOK /\ ExitEffects => ToolTypes(c')
   BY TypeTools, SMT DEF TypeTools, TypeOK, ExitEffects, ToolTypes
 <1>3. TypeOK /\ ExitEffects => ProcessTypes(c')
   BY TypeProcesses, SMT DEF TypeProcesses, TypeOK, ExitEffects, ProcessTypes
 <1>4. TypeOK /\ ExitEffects => ControlTypes(c')
   BY TypeControls, SMT DEF TypeControls, TypeOK, ExitEffects, ControlTypes, Phases, Outcomes
 <1> QED BY <1>1, <1>2, <1>3, <1>4, TypeConstruct DEF TypeConstruct, TypeOK
THEOREM ExitAdmission ==
 TypeOK /\ Admission /\ UpgradeIntegrity /\ ExitEffects => Admission'
 BY SMT DEF TypeOK, Admission, UpgradeIntegrity, ExitEffects
THEOREM ExitOwnership ==
 TypeOK /\ Ownership /\ UpgradeIntegrity /\ ExitEffects => Ownership'
 BY SMT DEF TypeOK, Ownership, UpgradeIntegrity, ExitEffects
THEOREM ExitReadiness ==
 TypeOK /\ Readiness /\ ExitEffects => Readiness' /\ UpgradeIntegrity'
 BY SMT DEF TypeOK, Readiness, UpgradeIntegrity, ExitEffects
THEOREM UpgradeExitPreserves == Inv /\ UpgradeExit => Inv'
 BY CompletedExitEffects, FailedExitEffects, ReconciledExitEffects,
    ExitTyped, ExitAdmission, ExitOwnership, ExitReadiness
 DEF CompletedExitEffects, FailedExitEffects, ReconciledExitEffects, ExitTyped, ExitAdmission, ExitOwnership, ExitReadiness, Inv, UpgradeExit
THEOREM ExitMonotone ==
 TypeOK /\ UpgradeIntegrity /\ ExitEffects => c.format <= c.format'
 BY SMT DEF TypeOK, Formats, UpgradeIntegrity, ExitEffects
THEOREM UpgradeExitMonotone == Inv /\ UpgradeExit => c.format <= c.format'
 BY CompletedExitEffects, FailedExitEffects, ReconciledExitEffects, ExitMonotone
 DEF CompletedExitEffects, FailedExitEffects, ReconciledExitEffects, ExitMonotone, Inv, UpgradeExit

THEOREM InitInvariant == Environment /\ Init => Inv
 <1>1. Environment /\ Init => StateShape(c)
   BY ONLY DEF Environment, Init, StateShape, LifecycleFields
 <1>2. Environment /\ Init => ToolTypes(c)
   <2>1. Init =>
     c.admitted={1} /\ c.selected=1 /\ c.format=1 /\ c.backups={} /\ c.tool=1
     BY ONLY DEF Init
   <2>2. Init =>
     c.plan=InitialPlan
     BY ONLY DEF Init
   <2>3. InitialPlan
     \in [valid : BOOLEAN, oldTool : Tools, newTool : Tools, oldFormat : Formats, newFormat : Formats]
     BY ONLY DEF InitialPlan, Tools, Formats
   <2> QED BY ONLY <2>1, <2>2, <2>3 DEF ToolTypes, Tools, Formats
 <1>3. Environment /\ Init => ProcessTypes(c)
   BY SMT DEF Environment, Init, ProcessTypes
 <1>4. Init => ControlTypes(c)
   BY SMT DEF Init, ControlTypes, Phases, Outcomes
 <1>5. Init => Admission
   BY SMT DEF Init, Admission, Compatible
 <1>6. Init => Ownership
   BY SMT DEF Init, Ownership
 <1>7. Init => Readiness /\ UpgradeIntegrity
   BY SMT DEF Init, InitialPlan, Readiness, UpgradeIntegrity
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5, <1>6, <1>7, TypeConstruct DEF TypeConstruct, Inv, TypeOK
THEOREM InstallPreserves == \A t \in Tools : Inv /\ Install(t) => Inv'
 <1>1. \A t \in Tools : TypeOK /\ Install(t) => TypeOK'
   <2>1. \A t \in Tools : TypeOK /\ Install(t) => StateShape(c')
     BY SMT DEF TypeOK, Install
   <2>2. \A t \in Tools : TypeOK /\ Install(t) => ToolTypes(c')
     BY SMT DEF TypeOK, Install, ToolTypes
   <2>3. \A t \in Tools : TypeOK /\ Install(t) => ProcessTypes(c')
     BY SMT DEF TypeOK, Install, ProcessTypes
   <2>4. \A t \in Tools : TypeOK /\ Install(t) => ControlTypes(c')
     BY SMT DEF TypeOK, Install, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. \A t \in Tools : TypeOK /\ Admission /\ Install(t) => Admission'
   BY SMT DEF TypeOK, Admission, Install
 <1>3. \A t \in Tools : TypeOK /\ Ownership /\ Install(t) => Ownership'
   BY SMT DEF TypeOK, Ownership, Install
 <1>4. \A t \in Tools : TypeOK /\ Readiness /\ Install(t) => Readiness'
   BY SMT DEF TypeOK, Readiness, Install
 <1>5. \A t \in Tools : TypeOK /\ UpgradeIntegrity /\ Install(t) => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Install
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM FailedInstallPreserves == Inv /\ FailedInstall => Inv'
 <1>1. TypeOK /\ FailedInstall => TypeOK'
   <2>1. TypeOK /\ FailedInstall => StateShape(c')
     BY SMT DEF TypeOK, FailedInstall
   <2>2. TypeOK /\ FailedInstall => ToolTypes(c')
     BY SMT DEF TypeOK, FailedInstall, ToolTypes
   <2>3. TypeOK /\ FailedInstall => ProcessTypes(c')
     BY SMT DEF TypeOK, FailedInstall, ProcessTypes
   <2>4. TypeOK /\ FailedInstall => ControlTypes(c')
     BY SMT DEF TypeOK, FailedInstall, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ FailedInstall => Admission'
   BY SMT DEF TypeOK, Admission, FailedInstall
 <1>3. TypeOK /\ Ownership /\ FailedInstall => Ownership'
   BY SMT DEF TypeOK, Ownership, FailedInstall
 <1>4. TypeOK /\ Readiness /\ FailedInstall => Readiness'
   BY SMT DEF TypeOK, Readiness, FailedInstall
 <1>5. TypeOK /\ UpgradeIntegrity /\ FailedInstall => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, FailedInstall
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM SpawnPreserves == \A i \in Incarnations, p \in Pids : Inv /\ Spawn(i, p) => Inv'
 BY SpawnTyped, SpawnAdmission, SpawnOwnership, SpawnControl DEF SpawnTyped, SpawnAdmission, SpawnOwnership, SpawnControl, Inv
THEOREM ListeningPreserves == Inv /\ Listening => Inv'
 <1>1. TypeOK /\ Listening => TypeOK'
   <2>1. TypeOK /\ Listening => StateShape(c')
     BY SMT DEF TypeOK, Listening
   <2>2. TypeOK /\ Listening => ToolTypes(c')
     BY SMT DEF TypeOK, Listening, ToolTypes
   <2>3. TypeOK /\ Listening => ProcessTypes(c')
     BY SMT DEF TypeOK, Listening, ProcessTypes
   <2>4. TypeOK /\ Listening => ControlTypes(c')
     BY SMT DEF TypeOK, Listening, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ Listening => Admission'
   BY SMT DEF TypeOK, Admission, Listening
 <1>3. TypeOK /\ Ownership /\ Listening => Ownership'
   BY SMT DEF TypeOK, Ownership, Listening
 <1>4. TypeOK /\ Readiness /\ Listening => Readiness'
   BY SMT DEF TypeOK, Readiness, Listening
 <1>5. TypeOK /\ UpgradeIntegrity /\ Listening => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Listening
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM QueryReadyPreserves == Inv /\ QueryReady => Inv'
 <1>1. TypeOK /\ QueryReady => TypeOK'
   <2>1. TypeOK /\ QueryReady => StateShape(c')
     BY SMT DEF TypeOK, QueryReady
   <2>2. TypeOK /\ QueryReady => ToolTypes(c')
     BY SMT DEF TypeOK, QueryReady, ToolTypes
   <2>3. TypeOK /\ QueryReady => ProcessTypes(c')
     BY SMT DEF TypeOK, QueryReady, ProcessTypes
   <2>4. TypeOK /\ QueryReady => ControlTypes(c')
     BY SMT DEF TypeOK, QueryReady, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ QueryReady => Admission'
   BY SMT DEF TypeOK, Admission, QueryReady
 <1>3. TypeOK /\ Ownership /\ QueryReady => Ownership'
   BY SMT DEF TypeOK, Ownership, QueryReady
 <1>4. TypeOK /\ Ownership /\ QueryReady => Readiness'
   BY SMT DEF TypeOK, Readiness, Ownership, QueryReady
 <1>5. TypeOK /\ UpgradeIntegrity /\ QueryReady => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, QueryReady
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM StartFailedPreserves == Inv /\ StartFailed => Inv'
 <1>1. TypeOK /\ StartFailed => TypeOK'
   <2>1. TypeOK /\ StartFailed => StateShape(c')
     BY SMT DEF TypeOK, StartFailed
   <2>2. TypeOK /\ StartFailed => ToolTypes(c')
     BY SMT DEF TypeOK, StartFailed, ToolTypes
   <2>3. TypeOK /\ StartFailed => ProcessTypes(c')
     BY SMT DEF TypeOK, StartFailed, ProcessTypes
   <2>4. TypeOK /\ StartFailed => ControlTypes(c')
     BY SMT DEF TypeOK, StartFailed, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ StartFailed => Admission'
   BY SMT DEF TypeOK, Admission, StartFailed
 <1>3. TypeOK /\ Ownership /\ StartFailed => Ownership'
   BY SMT DEF TypeOK, Ownership, StartFailed
 <1>4. TypeOK /\ StartFailed => Readiness'
   BY SMT DEF TypeOK, Readiness, StartFailed
 <1>5. TypeOK /\ UpgradeIntegrity /\ StartFailed => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, StartFailed
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM QueryFailedPreserves == Inv /\ QueryFailed => Inv'
 <1>1. TypeOK /\ QueryFailed => TypeOK'
   <2>1. TypeOK /\ QueryFailed => StateShape(c')
     BY SMT DEF TypeOK, QueryFailed
   <2>2. TypeOK /\ QueryFailed => ToolTypes(c')
     BY SMT DEF TypeOK, QueryFailed, ToolTypes
   <2>3. TypeOK /\ QueryFailed => ProcessTypes(c')
     BY SMT DEF TypeOK, QueryFailed, ProcessTypes
   <2>4. TypeOK /\ QueryFailed => ControlTypes(c')
     BY SMT DEF TypeOK, QueryFailed, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ QueryFailed => Admission'
   BY SMT DEF TypeOK, Admission, QueryFailed
 <1>3. TypeOK /\ Ownership /\ QueryFailed => Ownership'
   BY SMT DEF TypeOK, Ownership, QueryFailed
 <1>4. TypeOK /\ QueryFailed => Readiness'
   BY SMT DEF TypeOK, Readiness, QueryFailed
 <1>5. TypeOK /\ UpgradeIntegrity /\ QueryFailed => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, QueryFailed
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM StopPreserves == Inv /\ Stop => Inv'
 <1>1. TypeOK /\ Stop => TypeOK'
   <2>1. TypeOK /\ Stop => StateShape(c')
     BY SMT DEF TypeOK, Stop
   <2>2. TypeOK /\ Stop => ToolTypes(c')
     BY SMT DEF TypeOK, Stop, ToolTypes
   <2>3. TypeOK /\ Stop => ProcessTypes(c')
     BY SMT DEF TypeOK, Stop, ProcessTypes
   <2>4. TypeOK /\ Stop => ControlTypes(c')
     BY SMT DEF TypeOK, Stop, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ Stop => Admission'
   BY SMT DEF TypeOK, Admission, Stop
 <1>3. TypeOK /\ Ownership /\ Stop => Ownership'
   BY SMT DEF TypeOK, Ownership, Stop
 <1>4. TypeOK /\ Stop => Readiness'
   BY SMT DEF TypeOK, Readiness, Stop
 <1>5. TypeOK /\ Stop => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Stop
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM CancelPreserves == Inv /\ Cancel => Inv'
 BY StopPreserves DEF StopPreserves, Cancel
THEOREM TimeoutPreserves == Inv /\ TimeoutEscalation => Inv'
 <1>1. TypeOK /\ TimeoutEscalation => TypeOK'
   BY SMT DEF TypeOK, TimeoutEscalation, Phases
 <1>2. TypeOK /\ Admission /\ TimeoutEscalation => Admission'
   BY SMT DEF TypeOK, Admission, TimeoutEscalation
 <1>3. TypeOK /\ Ownership /\ TimeoutEscalation => Ownership'
   BY SMT DEF TypeOK, Ownership, TimeoutEscalation
 <1>4. TypeOK /\ TimeoutEscalation => Readiness' /\ UpgradeIntegrity'
   BY SMT DEF TypeOK, Readiness, UpgradeIntegrity, TimeoutEscalation
 <1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM SignalPreserves == Inv /\ SignalParent => Inv'
 <1>1. TypeOK /\ SignalParent => TypeOK'
   <2>1. TypeOK /\ SignalParent => StateShape(c')
     BY SMT DEF TypeOK, SignalParent
   <2>2. TypeOK /\ SignalParent => ToolTypes(c')
     BY SMT DEF TypeOK, SignalParent, ToolTypes
   <2>3. TypeOK /\ SignalParent => ProcessTypes(c')
     BY SMT DEF TypeOK, SignalParent, ProcessTypes
   <2>4. TypeOK /\ SignalParent => ControlTypes(c')
     BY SMT DEF TypeOK, SignalParent, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ SignalParent => Admission'
   BY SMT DEF TypeOK, Admission, SignalParent
 <1>3. TypeOK /\ Ownership /\ SignalParent => Ownership'
   BY SMT DEF TypeOK, Ownership, SignalParent
 <1>4. TypeOK /\ Readiness /\ SignalParent => Readiness'
   BY SMT DEF TypeOK, Readiness, SignalParent
 <1>5. TypeOK /\ UpgradeIntegrity /\ SignalParent => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, SignalParent
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM DescendantSignalPreserves == \A d \in Children : Inv /\ SignalDescendant(d) => Inv'
 <1>1. \A d \in Children : TypeOK /\ SignalDescendant(d) => TypeOK'
   <2>1. \A d \in Children : TypeOK /\ SignalDescendant(d) => StateShape(c')
     BY SMT DEF TypeOK, SignalDescendant
   <2>2. \A d \in Children : TypeOK /\ SignalDescendant(d) => ToolTypes(c')
     BY SMT DEF TypeOK, SignalDescendant, ToolTypes
   <2>3. \A d \in Children : TypeOK /\ SignalDescendant(d) => ProcessTypes(c')
     BY SMT DEF TypeOK, SignalDescendant, ProcessTypes
   <2>4. \A d \in Children : TypeOK /\ SignalDescendant(d) => ControlTypes(c')
     BY SMT DEF TypeOK, SignalDescendant, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. \A d \in Children : TypeOK /\ Admission /\ SignalDescendant(d) => Admission'
   BY SMT DEF TypeOK, Admission, SignalDescendant
 <1>3. \A d \in Children : TypeOK /\ Ownership /\ SignalDescendant(d) => Ownership'
   BY SMT DEF TypeOK, Ownership, SignalDescendant
 <1>4. \A d \in Children : TypeOK /\ Readiness /\ SignalDescendant(d) => Readiness'
   BY SMT DEF TypeOK, Readiness, SignalDescendant
 <1>5. \A d \in Children : TypeOK /\ UpgradeIntegrity /\ SignalDescendant(d) => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, SignalDescendant
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM RejectPreserves == Inv /\ RejectWrongIncarnation => Inv'
 <1>1. TypeOK /\ RejectWrongIncarnation => TypeOK'
   <2>1. TypeOK /\ RejectWrongIncarnation => StateShape(c')
     BY SMT DEF TypeOK, RejectWrongIncarnation
   <2>2. TypeOK /\ RejectWrongIncarnation => ToolTypes(c')
     BY SMT DEF TypeOK, RejectWrongIncarnation, ToolTypes
   <2>3. TypeOK /\ RejectWrongIncarnation => ProcessTypes(c')
     BY SMT DEF TypeOK, RejectWrongIncarnation, ProcessTypes
   <2>4. TypeOK /\ RejectWrongIncarnation => ControlTypes(c')
     BY SMT DEF TypeOK, RejectWrongIncarnation, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ RejectWrongIncarnation => Admission'
   BY SMT DEF TypeOK, Admission, RejectWrongIncarnation
 <1>3. TypeOK /\ Ownership /\ RejectWrongIncarnation => Ownership'
   BY SMT DEF TypeOK, Ownership, RejectWrongIncarnation
 <1>4. TypeOK /\ Readiness /\ RejectWrongIncarnation => Readiness'
   BY SMT DEF TypeOK, Readiness, RejectWrongIncarnation
 <1>5. TypeOK /\ UpgradeIntegrity /\ RejectWrongIncarnation => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, RejectWrongIncarnation
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM ParentExitPreserves == Inv /\ ParentExit => Inv'
 <1>1. TypeOK /\ ParentExit => TypeOK'
   <2>1. TypeOK /\ ParentExit => StateShape(c')
     BY SMT DEF TypeOK, ParentExit
   <2>2. TypeOK /\ ParentExit => ToolTypes(c')
     BY SMT DEF TypeOK, ParentExit, ToolTypes
   <2>3. TypeOK /\ ParentExit => ProcessTypes(c')
     BY SMT DEF TypeOK, ParentExit, ProcessTypes
   <2>4. TypeOK /\ ParentExit => ControlTypes(c')
     BY SMT DEF TypeOK, ParentExit, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ ParentExit => Admission'
   BY SMT DEF TypeOK, Admission, ParentExit
 <1>3. TypeOK /\ Ownership /\ ParentExit => Ownership'
   BY SMT DEF TypeOK, Ownership, ParentExit
 <1>4. TypeOK /\ ParentExit => Readiness'
   BY SMT DEF TypeOK, Readiness, ParentExit
 <1>5. TypeOK /\ UpgradeIntegrity /\ ParentExit => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, ParentExit
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM ChildExitPreserves == \A d \in Children : Inv /\ ChildExit(d) => Inv'
 <1>1. \A d \in Children : TypeOK /\ ChildExit(d) => TypeOK'
   <2>1. \A d \in Children : TypeOK /\ ChildExit(d) => StateShape(c')
     BY SMT DEF TypeOK, ChildExit
   <2>2. \A d \in Children : TypeOK /\ ChildExit(d) => ToolTypes(c')
     BY SMT DEF TypeOK, ChildExit, ToolTypes
   <2>3. \A d \in Children : TypeOK /\ ChildExit(d) => ProcessTypes(c')
     BY SMT DEF TypeOK, ChildExit, ProcessTypes
   <2>4. \A d \in Children : TypeOK /\ ChildExit(d) => ControlTypes(c')
     BY SMT DEF TypeOK, ChildExit, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. \A d \in Children : TypeOK /\ Admission /\ ChildExit(d) => Admission'
   BY SMT DEF TypeOK, Admission, ChildExit
 <1>3. \A d \in Children : TypeOK /\ Ownership /\ ChildExit(d) => Ownership'
   BY SMT DEF TypeOK, Ownership, ChildExit
 <1>4. \A d \in Children : TypeOK /\ Readiness /\ ChildExit(d) => Readiness'
   BY SMT DEF TypeOK, Readiness, ChildExit
 <1>5. \A d \in Children : TypeOK /\ UpgradeIntegrity /\ ChildExit(d) => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, ChildExit
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM ReusePreserves == Inv /\ PidReuse => Inv'
 <1>1. TypeOK /\ PidReuse => TypeOK'
   <2>1. TypeOK /\ PidReuse => StateShape(c')
     BY SMT DEF TypeOK, PidReuse
   <2>2. TypeOK /\ PidReuse => ToolTypes(c')
     BY SMT DEF TypeOK, PidReuse, ToolTypes
   <2>3. TypeOK /\ PidReuse => ProcessTypes(c')
     BY SMT DEF TypeOK, PidReuse, ProcessTypes
   <2>4. TypeOK /\ PidReuse => ControlTypes(c')
     BY SMT DEF TypeOK, PidReuse, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ PidReuse => Admission'
   BY SMT DEF TypeOK, Admission, PidReuse
 <1>3. TypeOK /\ Ownership /\ PidReuse => Ownership'
   BY SMT DEF TypeOK, Ownership, PidReuse
 <1>4. TypeOK /\ Readiness /\ PidReuse => Readiness'
   BY SMT DEF TypeOK, Readiness, PidReuse
 <1>5. TypeOK /\ UpgradeIntegrity /\ PidReuse => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, PidReuse
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM ObservePreserves == Inv /\ ObserveQuiescent => Inv'
 <1>1. TypeOK /\ ObserveQuiescent => TypeOK'
   <2>1. TypeOK /\ ObserveQuiescent => StateShape(c')
     BY SMT DEF TypeOK, ObserveQuiescent
   <2>2. TypeOK /\ ObserveQuiescent => ToolTypes(c')
     BY SMT DEF TypeOK, ObserveQuiescent, ToolTypes
   <2>3. TypeOK /\ ObserveQuiescent => ProcessTypes(c')
     BY SMT DEF TypeOK, ObserveQuiescent, ProcessTypes
   <2>4. TypeOK /\ ObserveQuiescent => ControlTypes(c')
     BY SMT DEF TypeOK, ObserveQuiescent, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ ObserveQuiescent => Admission'
   BY SMT DEF TypeOK, Admission, ObserveQuiescent
 <1>3. TypeOK /\ Ownership /\ ObserveQuiescent => Ownership'
   BY SMT DEF TypeOK, Ownership, ObserveQuiescent
 <1>4. TypeOK /\ Readiness /\ ObserveQuiescent => Readiness'
   BY SMT DEF TypeOK, Readiness, ObserveQuiescent
 <1>5. TypeOK /\ UpgradeIntegrity /\ ObserveQuiescent => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, ObserveQuiescent
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM ReapPreserves == Inv /\ Reap => Inv'
 <1>1. TypeOK /\ Reap => TypeOK'
   <2>1. TypeOK /\ Reap => StateShape(c')
     BY SMT DEF TypeOK, Reap
   <2>2. TypeOK /\ Reap => ToolTypes(c')
     BY SMT DEF TypeOK, Reap, ToolTypes
   <2>3. TypeOK /\ Reap => ProcessTypes(c')
     BY SMT DEF TypeOK, Reap, ProcessTypes
   <2>4. TypeOK /\ Reap => ControlTypes(c')
     BY SMT DEF TypeOK, Reap, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ Reap => Admission'
   BY SMT DEF TypeOK, Admission, Reap
 <1>3. TypeOK /\ Ownership /\ Reap => Ownership'
   BY SMT DEF TypeOK, Ownership, Reap
 <1>4. TypeOK /\ Reap => Readiness'
   BY SMT DEF TypeOK, Readiness, Reap
 <1>5. TypeOK /\ Reap => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Reap
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM BackupPreserves == Inv /\ VerifyBackup => Inv'
 <1>1. TypeOK /\ VerifyBackup => TypeOK'
   <2>1. TypeOK /\ VerifyBackup => StateShape(c')
     BY SMT DEF TypeOK, VerifyBackup
   <2>2. TypeOK /\ VerifyBackup => ToolTypes(c')
     BY SMT DEF TypeOK, VerifyBackup, ToolTypes
   <2>3. TypeOK /\ VerifyBackup => ProcessTypes(c')
     BY SMT DEF TypeOK, VerifyBackup, ProcessTypes
   <2>4. TypeOK /\ VerifyBackup => ControlTypes(c')
     BY SMT DEF TypeOK, VerifyBackup, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ VerifyBackup => Admission'
   BY SMT DEF TypeOK, Admission, VerifyBackup
 <1>3. TypeOK /\ Ownership /\ VerifyBackup => Ownership'
   BY SMT DEF TypeOK, Ownership, VerifyBackup
 <1>4. TypeOK /\ Readiness /\ VerifyBackup => Readiness'
   BY SMT DEF TypeOK, Readiness, VerifyBackup
 <1>5. TypeOK /\ UpgradeIntegrity /\ VerifyBackup => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, VerifyBackup
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM PlanPreserves == \A t \in Tools, f \in Formats : Inv /\ PlanUpgrade(t, f) => Inv'
 <1>1. \A t \in Tools, f \in Formats : TypeOK /\ PlanUpgrade(t, f) => TypeOK'
   <2>1. \A t \in Tools, f \in Formats : TypeOK /\ PlanUpgrade(t, f) => StateShape(c')
     BY TypeShape, SMT DEF TypeShape, TypeOK, PlanUpgrade, StateShape, LifecycleFields
   <2>2. \A t \in Tools, f \in Formats : TypeOK /\ PlanUpgrade(t, f) => ToolTypes(c')
     BY TypeTools, SMT DEF TypeTools, TypeOK, PlanUpgrade, ToolTypes
   <2>3. \A t \in Tools, f \in Formats : TypeOK /\ PlanUpgrade(t, f) => ProcessTypes(c')
     BY TypeProcesses, SMT DEF TypeProcesses, TypeOK, PlanUpgrade, ProcessTypes
   <2>4. \A t \in Tools, f \in Formats : TypeOK /\ PlanUpgrade(t, f) => ControlTypes(c')
     BY TypeControls, SMT DEF TypeControls, TypeOK, PlanUpgrade, ControlTypes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. \A t \in Tools, f \in Formats : TypeOK /\ Admission /\ PlanUpgrade(t, f) => Admission'
   BY SMT DEF TypeOK, Admission, PlanUpgrade
 <1>3. \A t \in Tools, f \in Formats : TypeOK /\ Ownership /\ PlanUpgrade(t, f) => Ownership'
   BY SMT DEF TypeOK, Ownership, PlanUpgrade
 <1>4. \A t \in Tools, f \in Formats : TypeOK /\ Readiness /\ PlanUpgrade(t, f) => Readiness'
   BY SMT DEF TypeOK, Readiness, PlanUpgrade
 <1>5. \A t \in Tools, f \in Formats : TypeOK /\ Ownership /\ PlanUpgrade(t, f) => UpgradeIntegrity'
   BY SMT DEF TypeOK, Ownership, UpgradeIntegrity, PlanUpgrade
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM BeginPreserves == Inv /\ BeginUpgrade => Inv'
 <1>1. TypeOK /\ BeginUpgrade => TypeOK'
   BY SMT DEF TypeOK, BeginUpgrade, Phases
 <1>2. TypeOK /\ Admission /\ BeginUpgrade => Admission'
   BY SMT DEF TypeOK, Admission, BeginUpgrade
 <1>3. TypeOK /\ Ownership /\ BeginUpgrade => Ownership'
   BY SMT DEF TypeOK, Ownership, BeginUpgrade
 <1>4. TypeOK /\ Readiness /\ BeginUpgrade => Readiness'
   BY SMT DEF TypeOK, Readiness, BeginUpgrade
 <1>5. TypeOK /\ Ownership /\ UpgradeIntegrity /\ BeginUpgrade => UpgradeIntegrity'
   BY SMT DEF TypeOK, Ownership, UpgradeIntegrity, BeginUpgrade
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM CompletePreserves == Inv /\ CompleteUpgrade => Inv'
 BY UpgradeExitPreserves DEF UpgradeExitPreserves, UpgradeExit
THEOREM UpgradeFailurePreserves == Inv /\ FailedUpgrade => Inv'
 BY UpgradeExitPreserves DEF UpgradeExitPreserves, UpgradeExit
THEOREM ReconcilePreserves == \A applied \in BOOLEAN : Inv /\ ReconcileUpgrade(applied) => Inv'
 BY UpgradeExitPreserves DEF UpgradeExitPreserves, UpgradeExit
THEOREM RollbackPreserves == \A t \in Tools : Inv /\ Rollback(t) => Inv'
 <1>1. \A t \in Tools : TypeOK /\ Rollback(t) => TypeOK'
   <2>1. \A t \in Tools : TypeOK /\ Rollback(t) => StateShape(c')
     BY SMT DEF TypeOK, Rollback
   <2>2. \A t \in Tools : TypeOK /\ Rollback(t) => ToolTypes(c')
     BY SMT DEF TypeOK, Rollback, ToolTypes
   <2>3. \A t \in Tools : TypeOK /\ Rollback(t) => ProcessTypes(c')
     BY SMT DEF TypeOK, Rollback, ProcessTypes
   <2>4. \A t \in Tools : TypeOK /\ Rollback(t) => ControlTypes(c')
     BY SMT DEF TypeOK, Rollback, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. \A t \in Tools : TypeOK /\ Admission /\ Rollback(t) => Admission'
   BY SMT DEF TypeOK, Admission, Rollback
 <1>3. \A t \in Tools : TypeOK /\ Ownership /\ Rollback(t) => Ownership'
   BY SMT DEF TypeOK, Ownership, Rollback
 <1>4. \A t \in Tools : TypeOK /\ Readiness /\ Rollback(t) => Readiness'
   BY SMT DEF TypeOK, Readiness, Rollback
 <1>5. \A t \in Tools : TypeOK /\ Rollback(t) => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Rollback
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM FailurePreserves == Inv /\ FailedIO => Inv'
 BY DiagnosticPreserves DEF DiagnosticPreserves, DiagnosticStep
THEOREM CrashPreserves == Inv /\ Crash => Inv'
 <1>1. TypeOK /\ Crash => TypeOK'
   <2>1. TypeOK /\ Crash => StateShape(c')
     BY SMT DEF TypeOK, Crash
   <2>2. TypeOK /\ Crash => ToolTypes(c')
     BY SMT DEF TypeOK, Crash, ToolTypes
   <2>3. TypeOK /\ Crash => ProcessTypes(c')
     BY SMT DEF TypeOK, Crash, ProcessTypes
   <2>4. TypeOK /\ Crash => ControlTypes(c')
     BY SMT DEF TypeOK, Crash, ControlTypes, Phases, Outcomes
   <2> QED BY <2>1, <2>2, <2>3, <2>4, TypeConstruct DEF TypeConstruct, TypeOK
 <1>2. TypeOK /\ Admission /\ Crash => Admission'
   BY SMT DEF TypeOK, Admission, Crash
 <1>3. TypeOK /\ Ownership /\ Crash => Ownership'
   BY SMT DEF TypeOK, Ownership, Crash
 <1>4. TypeOK /\ Crash => Readiness'
   BY SMT DEF TypeOK, Readiness, Crash
 <1>5. TypeOK /\ UpgradeIntegrity /\ Crash => UpgradeIntegrity'
   BY SMT DEF TypeOK, UpgradeIntegrity, Crash
 <1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv
THEOREM RestartPreserves == Inv /\ Restart => Inv'
 BY DiagnosticPreserves DEF DiagnosticPreserves, DiagnosticStep
THEOREM StutterPreserves == Inv /\ UNCHANGED vars => Inv'
 <1>1. TypeOK /\ UNCHANGED vars => TypeOK'
   BY ONLY DEF vars, TypeOK, LifecycleType
 <1>2. Inv /\ UNCHANGED vars => CoreInvariant'
   BY CoreFrame DEF CoreFrame, Inv, CoreInvariant, SemanticFields, vars
 <1> QED BY <1>1, <1>2 DEF Inv, CoreInvariant
THEOREM StepPreserves == Inv /\ [Next]_vars => Inv'
 BY InstallPreserves, FailedInstallPreserves, SpawnPreserves, ListeningPreserves,
    QueryReadyPreserves, StartFailedPreserves, QueryFailedPreserves, StopPreserves,
    CancelPreserves, TimeoutPreserves, SignalPreserves, DescendantSignalPreserves, RejectPreserves,
    ParentExitPreserves, ChildExitPreserves, ReusePreserves, ObservePreserves,
    ReapPreserves, BackupPreserves, PlanPreserves, BeginPreserves,
    CompletePreserves, UpgradeFailurePreserves, ReconcilePreserves,
    RollbackPreserves, FailurePreserves, CrashPreserves, RestartPreserves, StutterPreserves
 DEF InstallPreserves, FailedInstallPreserves, SpawnPreserves, ListeningPreserves, QueryReadyPreserves, StartFailedPreserves, QueryFailedPreserves, StopPreserves, CancelPreserves, TimeoutPreserves, SignalPreserves, DescendantSignalPreserves, RejectPreserves, ParentExitPreserves, ChildExitPreserves, ReusePreserves, ObservePreserves, ReapPreserves, BackupPreserves, PlanPreserves, BeginPreserves, CompletePreserves, UpgradeFailurePreserves, ReconcilePreserves, RollbackPreserves, FailurePreserves, CrashPreserves, RestartPreserves, StutterPreserves, Next, ToolStep, ProcessStep
THEOREM InvariantImpliesSafety == Inv => Safety
 <1>1. Admission /\ Ownership /\ Readiness => Safety
   BY ONLY DEF Admission, Ownership, Readiness, Safety
 <1> QED BY <1>1 DEF Inv
(* Incarnation sets are arbitrary under Environment; no campaign constraint
   or fairness premise enters this mathematical safety induction. *)
THEOREM InductiveSafety == Environment /\ Spec => []Safety
 <1>1. Environment /\ Init => Inv
   BY InitInvariant DEF InitInvariant
 <1>2. Inv /\ [Next]_vars => Inv'
   BY StepPreserves DEF StepPreserves
 <1>3. Environment /\ Spec => []Inv
   BY <1>1, <1>2, PTL DEF Spec
 <1> QED BY <1>3, InvariantImpliesSafety, PTL DEF InvariantImpliesSafety
THEOREM CancellationFrame == TypeOK /\ Cancel => UNCHANGED
 <<c.charged, c.alive, c.descendants, c.reaped, c.inc, c.pid, c.tool>>
 BY SMT DEF Cancel, Stop, TypeOK
THEOREM CancellationKeepsTeardownEvidence == TypeOK /\ Cancel =>
 UNCHANGED <<c.teardownKnown, c.actual, c.launched, c.used, c.childSignals>>
 BY SMT DEF Cancel, Stop, TypeOK
THEOREM SignalExactIncarnation == TypeOK /\ SignalParent =>
 /\ c.actual[c.pid] = c.inc
 /\ <<c.pid, c.inc>> \in c.launched
 /\ c'.signals = c.signals \cup {<<c.pid, c.inc>>}
 /\ UNCHANGED <<c.inc, c.pid, c.actual, c.launched, c.charged, c.reaped>>
 BY SMT DEF TypeOK, SignalParent
THEOREM ReapOwnershipEffects == TypeOK /\ Reap =>
 /\ c.teardownKnown /\ ~c.alive /\ c.descendants = {}
 /\ c'.reaped /\ ~c'.charged
 /\ UNCHANGED <<c.inc, c.pid, c.actual, c.launched, c.used>>
 BY SMT DEF TypeOK, Reap
THEOREM CrashRetainsOwnership == TypeOK /\ Crash =>
 UNCHANGED <<c.inc, c.pid, c.actual, c.launched, c.used, c.charged,
             c.alive, c.descendants, c.reaped, c.teardownKnown>>
 BY SMT DEF TypeOK, Crash
StableToolStep ==
 (\E t \in Tools : Install(t) \/ Rollback(t))
 \/ FailedInstall \/ VerifyBackup
 \/ (\E t \in Tools, f \in Formats : PlanUpgrade(t, f))
 \/ BeginUpgrade
THEOREM StableToolFormatFrame == TypeOK /\ StableToolStep => UNCHANGED c.format
 BY SMT DEF TypeOK, StableToolStep, Install, Rollback, FailedInstall,
            VerifyBackup, PlanUpgrade, BeginUpgrade
(* Narrow resident-process frames also serve the retention/lifecycle bridge.
   Spawn, reaping and upgrade completion are deliberately outside this frame. *)
ReadinessControlStep ==
 Listening \/ QueryReady \/ StartFailed \/ QueryFailed \/ Stop \/ Cancel \/ TimeoutEscalation
ObservationControlStep ==
 SignalParent \/ RejectWrongIncarnation \/ ParentExit \/ PidReuse \/ ObserveQuiescent
 \/ (\E d \in Children : ChildExit(d) \/ SignalDescendant(d))
FailureControlStep == FailedIO \/ Crash \/ Restart
ResidentControlStep == ReadinessControlStep \/ ObservationControlStep \/ FailureControlStep
ControlFrame == UNCHANGED <<c.inc, c.charged, c.reaped, c.format>>
THEOREM ReadinessControlFrame == TypeOK /\ ReadinessControlStep => ControlFrame
 BY SMT DEF TypeOK, ReadinessControlStep, ControlFrame, Listening, QueryReady,
            StartFailed, QueryFailed, Stop, Cancel, TimeoutEscalation
THEOREM ObservationControlFrame == TypeOK /\ ObservationControlStep => ControlFrame
 BY SMT DEF TypeOK, ObservationControlStep, ControlFrame, SignalParent,
            RejectWrongIncarnation, ParentExit, PidReuse, ObserveQuiescent,
            ChildExit, SignalDescendant
THEOREM FailureControlFrame == TypeOK /\ FailureControlStep => ControlFrame
 BY SMT DEF TypeOK, FailureControlStep, ControlFrame, FailedIO, Crash, Restart
THEOREM ResidentControlFrame == TypeOK /\ ResidentControlStep => ControlFrame
 BY ReadinessControlFrame, ObservationControlFrame, FailureControlFrame
 DEF ReadinessControlFrame, ObservationControlFrame, FailureControlFrame, ResidentControlStep
THEOREM ReapFormatFrame == TypeOK /\ Reap => UNCHANGED c.format
 BY SMT DEF TypeOK, Reap
THEOREM ProcessFormatFrame == TypeOK /\ ProcessStep => UNCHANGED c.format
 BY SpawnFrame, ResidentControlFrame, ReapFormatFrame
 DEF SpawnFrame, SpawnStable, ResidentControlFrame, ReapFormatFrame, ControlFrame,
     ProcessStep, ResidentControlStep, ReadinessControlStep, ObservationControlStep, FailureControlStep
THEOREM DiagnosticFormatFrame ==
 TypeOK /\ (FailedIO \/ Crash \/ Restart) => UNCHANGED c.format
 BY SMT DEF TypeOK, FailedIO, Crash, Restart
THEOREM FormatReflexive == TypeOK => c.format <= c.format
 BY SMT DEF TypeOK, Formats
THEOREM EqualFormatMonotone ==
 TypeOK /\ UNCHANGED c.format => c.format <= c.format'
 BY FormatReflexive, SMT DEF FormatReflexive
THEOREM ToolFormatMonotone == Inv /\ ToolStep => c.format <= c.format'
 BY StableToolFormatFrame, UpgradeExitMonotone, EqualFormatMonotone
 DEF StableToolFormatFrame, UpgradeExitMonotone, EqualFormatMonotone,
     Inv, ToolStep, StableToolStep, UpgradeExit
THEOREM NoDataDowngrade == Inv /\ Next => c.format <= c.format'
 BY ToolFormatMonotone, ProcessFormatFrame, DiagnosticFormatFrame, EqualFormatMonotone
 DEF ToolFormatMonotone, ProcessFormatFrame, DiagnosticFormatFrame, EqualFormatMonotone,
     Inv, Next

(* Real semantic mutants: remove exact-incarnation signal guard, or remove
   current-format compatibility from rollback. Expected failures: Safety. *)
WrongIncarnation ==
 /\ c.up /\ c.phase \in {"draining", "killing"}
 /\ c' = [c EXCEPT !.signals = @ \cup {<<c.pid, c.actual[c.pid]>>}]
IncompatibleRollback(t) ==
 /\ c.up /\ c.phase = "idle" /\ t \in c.admitted
 /\ c' = [c EXCEPT !.selected = t, !.plan.valid = FALSE]
WrongIncarnationNext == Next \/ WrongIncarnation
IncompatibleRollbackNext == Next \/ (\E t \in Tools : IncompatibleRollback(t))
WrongIncarnationSpec == Init /\ [][WrongIncarnationNext]_vars
IncompatibleRollbackSpec == Init /\ [][IncompatibleRollbackNext]_vars
=============================================================================
