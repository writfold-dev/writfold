------------------------ MODULE M02RecoveryReplay ------------------------
EXTENDS M01ReleasePublication
USE ParameterBounds DEF Bounds
USE DEF NonObservationNext

(***************************************************************************
Recovery shares M01's durable p, exact Input/OpOf and CAS, not a reconstructed
"equivalent" plan. Hint input and phase are attacker-controlled observations;
no action below reads them as authority. r.corrupt records observed integrity
failure, not permission to delete an acknowledged object from the abstract
store. It covers missing bytes, digest/row/manifest disagreement and corrupt
current state. External destructive writes are outside M01's immutable-store
premise; DetectCorruption deliberately converts that boundary failure into an
explicit refusal. There is NO fallback-to-an-older-current transition.

A repair checks exact retained input/current authority and completed ancestry.
Already-applied input repairs its receipt without another CAS/native apply.
A non-applied input resumes from independently verified actual durable objects,
never from a journal phase. Refusal retains all capacity/data and the current
ref. A runtime adapter must locate historical applied input in the retained
completed chain, not just compare the current tip or an operation ID.
***************************************************************************)
HintPhases == {"none","intent","native","manifest","published"}
RecoveryPhases == {"idle","inspect","resumed","done","down"}
RecoveryOutcomes == {"none","pending","complete","already_applied","conflict","refused","unknown"}
ReplayInputs == Inputs \cup {LegacyId}
VARIABLE r
recoveryVars == <<p,r>>
RecoveryTypeOK == r \in [target : [Writers -> Releases \cup {LegacyId}],
  pc : [Writers -> RecoveryPhases], result : [Writers -> RecoveryOutcomes],
  hint : [Ops -> [input : Releases, phase : HintPhases]],
  corrupt : SUBSET Releases, checked : [Writers -> Releases \cup {LegacyId}]]
HistoricalAuthentic ==
  /\ p.legacy = LegacyHistory
  /\ p.legacy.input = LegacyInput
  /\ p.legacy.input.schema = 2
  /\ p.legacy.input.domain = "kctx-operation-v2"
  /\ p.legacy.manifest.input = p.legacy.input
  /\ p.legacy.receipt.input = p.legacy.input
  /\ p.legacy.receipt.operation = p.legacy.input.operation
  /\ p.legacy.receipt.release = p.legacy.manifest.release
  /\ p.legacy.receipt.outcome = "applied"
  /\ p.legacy.authentication.signedBytes = p.legacy.input.canonicalBytes
  /\ p.legacy.authentication.approvalBytes = p.legacy.input.approvalBytes
  /\ p.legacy.authentication.domain = p.legacy.input.domain
  /\ p.legacy.authentication.policy = p.legacy.input.context.policy
  /\ p.legacy.authentication.source = p.legacy.input.context.source
  /\ p.legacy.manifest.release = 0
  /\ 0 \in p.published
  /\ p.legacy.manifest.closure = Closure(0)
  /\ Closure(0) \subseteq p.objects
RecoveryEvidence ==
  /\ (\A w \in Writers : r.result[w] = "complete" =>
       /\ r.target[w] \in Inputs
       /\ r.checked[w] = r.target[w]
       /\ r.target[w] \in p.receipt
       /\ BoundIntent(r.target[w]))
  /\ (\A w \in Writers : r.result[w] = "already_applied" =>
       /\ r.target[w] = LegacyId
       /\ r.checked[w] = LegacyId
       /\ HistoricalAuthentic)
  /\ (\A w \in Writers : r.pc[w] = "resumed" =>
       /\ r.target[w] \in Inputs
       /\ SchemaOf(r.target[w]) = 3)
RecoveryInv == Inv /\ RecoveryTypeOK /\ RecoveryEvidence
RecoverySafety ==
  /\ Safety
  /\ (\A w \in Writers : r.result[w] = "complete" =>
       /\ r.target[w] \in p.published
       /\ Input(r.checked[w]) = Input(r.target[w])
       /\ p.intent[OpOf(r.target[w])] = r.target[w])
  /\ (\A w \in Writers : r.result[w] = "already_applied" =>
       /\ r.target[w] = p.legacy.input.identity
       /\ p.legacy.receipt.input = LegacyInput
       /\ p.legacy.input.schema = 2
       /\ p.legacy.input.domain = "kctx-operation-v2"
       /\ HistoricalAuthentic)
  /\ (\A w \in Writers : r.target[w] = InputCount => r.pc[w] # "resumed")
RecoveryInit == Init /\ r = [target |-> [w \in Writers |-> 0],
  pc |-> [w \in Writers |-> "idle"], result |-> [w \in Writers |-> "none"],
  hint |-> [o \in Ops |-> [input |-> 0, phase |-> "none"]],
  corrupt |-> {}, checked |-> [w \in Writers |-> 0]]

Inspecting(w,i) == /\ r.pc[w] = "inspect" /\ r.target[w] = i /\ i \in ReplayInputs
Intact(i) == p.head \notin r.corrupt /\
  (IF i = LegacyId THEN 0 \notin r.corrupt ELSE i \notin r.corrupt)
RecoverStart(w,i) == /\ r.pc[w] \in {"idle","done"}
  /\ p.pc[w] \in {"idle","failed","conflict","done","down"}
  /\ r' = [r EXCEPT !.target[w] = i, !.pc[w] = "inspect",
       !.result[w] = "pending", !.checked[w] = 0]
  /\ UNCHANGED p
MisleadHint(o,i,h) ==
  /\ r' = [r EXCEPT !.hint[o] = [input |-> i, phase |-> h]] /\ UNCHANGED p
DetectCorruption(i) == /\ r' = [r EXCEPT !.corrupt = @ \cup {i}] /\ UNCHANGED p
RefuseCorruption(w) == LET i == r.target[w] IN
  /\ Inspecting(w,i) /\ ~Intact(i)
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "refused"] /\ UNCHANGED p
ReplayConflict(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ p.intent[OpOf(i)] \notin {0,i}
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "conflict"] /\ UNCHANGED p
RefuseAuthority(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ ~Valid(i)
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "refused"] /\ UNCHANGED p
PredecessorConflict(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ i \notin p.published
  /\ p.intent[OpOf(i)] \in {0,i} /\ p.head # BaseOf(i)
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "conflict"] /\ UNCHANGED p
ResumeExact(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ Valid(i)
  /\ i \notin p.published /\ p.intent[OpOf(i)] \in {0,i}
  /\ p.head = BaseOf(i)
  /\ p' = [p EXCEPT !.request[w] = i, !.outcome[w] = "pending",
       !.pc[w] = IF i \in p.manifest THEN "manifest"
                 ELSE IF i \in p.native THEN "native"
                 ELSE IF BoundIntent(i) THEN "intent" ELSE "planned"]
  /\ r' = [r EXCEPT !.pc[w] = "resumed"]
RepairModernCompletion(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ Valid(i) /\ BoundIntent(i)
  /\ i \in p.published
  /\ p' = [p EXCEPT !.receipt = @ \cup {i}]
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]
(* Historical replay uses the independently authenticated completed record,
   not current new-write authority, hints, or a schema/domain conversion. *)
RepairLegacyCompletion(w) ==
  /\ Inspecting(w,LegacyId) /\ Intact(LegacyId) /\ HistoricalAuthentic
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "already_applied",
       !.checked[w] = LegacyId]
  /\ UNCHANGED p
RepairCompletion(w) == RepairModernCompletion(w) \/ RepairLegacyCompletion(w)
RefusePendingLegacy(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ SchemaOf(i) = 2
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "refused"]
  /\ UNCHANGED p
ObserveCompletion(w) == LET i == r.target[w] IN
  /\ r.pc[w] = "resumed" /\ i \in Inputs /\ Intact(i) /\ Valid(i)
  /\ i \in p.receipt /\ BoundIntent(i)
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]
  /\ UNCHANGED p
RecoveryIOFailure(w) == /\ r.pc[w] \in {"inspect","resumed"}
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "unknown"] /\ UNCHANGED p
RecoveryCrash(w) == /\ Crash(w)
  /\ r' = [r EXCEPT !.pc[w] = "down", !.result[w] = "unknown"]
RecoveryRestart(w) == /\ Restart(w)
  /\ r' = [r EXCEPT !.pc[w] = "idle", !.target[w] = 0,
       !.result[w] = "none", !.checked[w] = 0]
PublicationStep == /\ Next /\ p.head \notin r.corrupt
                   /\ p'.head \notin r.corrupt /\ UNCHANGED r
RecoveryStutter == UNCHANGED recoveryVars
LocalRecoveryNext ==
  \/ (\E w \in Writers, i \in ReplayInputs : RecoverStart(w,i))
  \/ (\E o \in Ops, i \in Releases, h \in HintPhases : MisleadHint(o,i,h))
  \/ (\E i \in Releases : DetectCorruption(i))
  \/ (\E w \in Writers :
       \/ RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
       \/ PredecessorConflict(w) \/ ResumeExact(w) \/ RepairCompletion(w)
       \/ ObserveCompletion(w) \/ RecoveryIOFailure(w) \/ RecoveryCrash(w)
       \/ RecoveryRestart(w) \/ RefusePendingLegacy(w))
  \/ RecoveryStutter
RecoveryNext == PublicationStep \/ LocalRecoveryNext

(* The unrestricted LocalRecoveryNext remains unchanged. This shared finite
   inventory removes only hint observation; the decomposition theorem below
   checks that no request, crash, failure, or recovery phase was omitted. *)
NonHintRecoveryNext ==
  \/ (\E w \in Writers, i \in ReplayInputs : RecoverStart(w,i))
  \/ (\E i \in Releases : DetectCorruption(i))
  \/ (\E w \in Writers :
       \/ RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
       \/ PredecessorConflict(w) \/ ResumeExact(w) \/ RepairCompletion(w)
       \/ ObserveCompletion(w) \/ RecoveryIOFailure(w) \/ RecoveryCrash(w)
       \/ RecoveryRestart(w) \/ RefusePendingLegacy(w))
  \/ RecoveryStutter
UnrestrictedHintStep ==
  \E o \in Ops, i \in Releases, h \in HintPhases : MisleadHint(o,i,h)

(* Baseline: only collision operation 1 varies, through absent, exact,
   colliding, and pending-schema-2 identities and EVERY journal phase.
   Other hint slots retain their initial empty records. Recovery targets are
   selected separately by the declared workload profile; publication requests
   remain unrestricted. There are twenty finite hint maps. *)
FiniteHintStep ==
  \E i \in {0,1,2,InputCount}, h \in HintPhases : MisleadHint(1,i,h)

(* Independent cross-operation forgery: operation 1 claims input 3 (owned by
   operation 2), while operation 2 claims input 1. Each slot independently
   selects any phase or returns to its empty record: 6*6 hint maps.
   Exact IDs stay distinct; neither forged slot grants authority. *)
CrossHintStep ==
  \/ (\E o \in {1,2}, h \in HintPhases :
       MisleadHint(o,(IF o = 1 THEN 3 ELSE 1),h))
  \/ (\E o \in {1,2} : MisleadHint(o,0,"none"))

RecoveryObservation ==
  <<p,r.target,r.pc,r.result,r.checked,r.corrupt>>
(* Candidate positive-only TLC VIEW. Do not activate in configurations until
   the hint-replacement certificate in M01M02Bridge has been checked.
   Hint-reading semantic mutants MUST retain the full state. *)
HintErasedView == RecoveryObservation

(* Fixed recovery workload for each run; publication writers retain every
   input. Odd/even worker assignments extend the same campaign to 3 writers.
   0: collision 1/2. 1: same exact input 1. 2: successor/base 3/1.
   3: completed LegacyId/pending-schema-2 InputCount.
   Corruption remains an independent arbitrary subset of Releases in EVERY
   profile: current, retained ancestry, and target faults may coexist. *)
WorkloadTarget(w,workload) ==
  CASE workload = 0 -> IF w % 2 = 1 THEN 1 ELSE 2
    [] workload = 1 -> 1
    [] workload = 2 -> IF w % 2 = 1 THEN 3 ELSE 1
    [] OTHER -> IF w % 2 = 1 THEN LegacyId ELSE InputCount
RecoveryServiceNext ==
  \/ (\E i \in Releases : DetectCorruption(i))
  \/ (\E w \in Writers :
       \/ RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
       \/ PredecessorConflict(w) \/ ResumeExact(w) \/ RepairCompletion(w)
       \/ ObserveCompletion(w) \/ RecoveryIOFailure(w) \/ RecoveryCrash(w)
       \/ RecoveryRestart(w) \/ RefusePendingLegacy(w))
  \/ RecoveryStutter
WorkloadLocalNext(workload) ==
  RecoveryServiceNext \/
  (\E w \in Writers : RecoverStart(w,WorkloadTarget(w,workload)))
FinitePublicationStep ==
  /\ FiniteNext
  /\ p.head \notin r.corrupt
  /\ p'.head \notin r.corrupt
  /\ UNCHANGED r
WorkloadRecoveryNext(workload) ==
  FinitePublicationStep \/ WorkloadLocalNext(workload) \/ FiniteHintStep
FiniteRecoveryNext == WorkloadRecoveryNext(0)
USE DEF LocalRecoveryNext
RecoverySpec == RecoveryInit /\ [][RecoveryNext]_recoveryVars
FiniteRecoverySpec == RecoveryInit /\ [][FiniteRecoveryNext]_recoveryVars
WorkloadRecoverySpec(workload) ==
  RecoveryInit /\ [][WorkloadRecoveryNext(workload)]_recoveryVars
SameInputRecoverySpec == WorkloadRecoverySpec(1)
SuccessorRecoverySpec == WorkloadRecoverySpec(2)
LegacyWorkloadRecoverySpec == WorkloadRecoverySpec(3)
WidePublicationStep(profile) ==
  /\ WideNext(profile)
  /\ p.head \notin r.corrupt
  /\ p'.head \notin r.corrupt
  /\ UNCHANGED r
WideRecoveryNext(profile) ==
  WidePublicationStep(profile) \/ WorkloadLocalNext(0) \/ FiniteHintStep
WideRecoverySpec(profile) == RecoveryInit /\ [][WideRecoveryNext(profile)]_recoveryVars
WideRecoverySpec0 == WideRecoverySpec(0)
WideRecoverySpec1 == WideRecoverySpec(1)
WideRecoverySpec2 == WideRecoverySpec(2)
WideRecoverySpec3 == WideRecoverySpec(3)

(* Separate positive profile: admission profile 0, with independently forged
   hints for operations 1 and 2. No participant or phase bounds are reduced. *)
CrossHintRecoveryNext ==
  WidePublicationStep(0) \/ WorkloadLocalNext(2) \/ CrossHintStep
CrossHintRecoverySpec == RecoveryInit /\ [][CrossHintRecoveryNext]_recoveryVars

(* Reachability probes deliberately choose short executions of the real
   actions. Their negated-target invariant is EXPECTED to fail, yielding a
   witness, not a safety qualification or a replacement for positive runs. *)
CompletedLegacyReplayReached ==
  HistoricalAuthentic /\ (\E w \in Writers :
    /\ r.target[w] = LegacyId
    /\ r.checked[w] = LegacyId
    /\ r.result[w] = "already_applied")
NoCompletedLegacyReplay == ~CompletedLegacyReplayReached
LegacyReplayProbeNext ==
  RecoverStart(1,LegacyId) \/ RepairLegacyCompletion(1) \/ RecoveryStutter
LegacyReplayProbeSpec == RecoveryInit /\ [][LegacyReplayProbeNext]_recoveryVars

ModernPreparedRecoveryReached == \E w \in Writers :
  /\ r.pc[w] = "resumed"
  /\ r.target[w] \in Inputs
  /\ r.target[w] \in p.manifest
  /\ r.target[w] \notin p.published
  /\ p.request[w] = r.target[w]
  /\ p.pc[w] = "manifest"
  /\ BoundIntent(r.target[w])
NoModernPreparedRecovery == ~ModernPreparedRecoveryReached
PreparedProbePublicationStep ==
  /\ ((1 \notin p.manifest /\ Start(1,1)) \/ Admit(1) \/ RetainIntent(1)
      \/ NativeCommit(1) \/ RetainManifest(1))
  /\ p.head \notin r.corrupt
  /\ p'.head \notin r.corrupt
  /\ UNCHANGED r
PreparedRecoveryProbeNext ==
  \/ PreparedProbePublicationStep
  \/ (p.pc[1] = "manifest" /\ RecoveryCrash(1))
  \/ RecoveryRestart(1)
  \/ (1 \in p.manifest /\ p.pc[1] = "idle" /\ RecoverStart(1,1))
  \/ ResumeExact(1)
  \/ RecoveryStutter
PreparedRecoveryProbeSpec == RecoveryInit /\ [][PreparedRecoveryProbeNext]_recoveryVars

(* Semantic mutant: one action substitutes journal assertion for exact chain,
   authority and identity checks. A forged "published" hint can create a receipt
   for an unapplied input; this must fail ReceiptSound, not parsing/type checks. *)
MutantTrustHint(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ r.hint[OpOf(i)].phase = "published"
  /\ p' = [p EXCEPT !.receipt = @ \cup {i}]
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]
(* Input-only mutation preserves replay shape while confusing operation ID with
   complete input equality. Its false receipt for the colliding input is visible. *)
MutantIdOnlyReplay(w) == LET i == r.target[w] IN
  /\ i \in Inputs /\ Inspecting(w,i) /\ Intact(i) /\ Valid(i)
  /\ p.intent[OpOf(i)] \in p.published \ {0}
  /\ p' = [p EXCEPT !.receipt = @ \cup {i}]
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "complete", !.checked[w] = i]

(* Substitute the pending schema-2 request for the authenticated historical
   identity guard. This must violate exact legacy receipt binding. *)
MutantPendingLegacyAsApplied(w) ==
  /\ Inspecting(w,InputCount) /\ SchemaOf(InputCount) = 2
  /\ r' = [r EXCEPT !.pc[w] = "done", !.result[w] = "already_applied",
       !.checked[w] = LegacyId]
  /\ UNCHANGED p

(* Proof-only views keep constant historical bytes out of action obligations.
   HistoricalAuthentic and RecoveryEvidence remain the public predicates. *)
HistoricalRecord(h) ==
  /\ h.input = LegacyInput
  /\ h.input.schema = 2
  /\ h.input.domain = "kctx-operation-v2"
  /\ h.manifest.input = h.input
  /\ h.receipt.input = h.input
  /\ h.receipt.operation = h.input.operation
  /\ h.receipt.release = h.manifest.release
  /\ h.receipt.outcome = "applied"
  /\ h.authentication.signedBytes = h.input.canonicalBytes
  /\ h.authentication.approvalBytes = h.input.approvalBytes
  /\ h.authentication.domain = h.input.domain
  /\ h.authentication.policy = h.input.context.policy
  /\ h.authentication.source = h.input.context.source
  /\ h.manifest.release = 0
  /\ h.manifest.closure = Closure(0)
THEOREM LegacyRecordFacts ==
  HistoricalRecord(LegacyHistory) /\ LegacyInput.identity = LegacyId
<1>1. LegacyInput.schema = 2 /\ LegacyInput.domain = "kctx-operation-v2"
      /\ LegacyInput.identity = LegacyId
  BY SMT DEF LegacyInput
<1>2. QED BY SMT, <1>1 DEF HistoricalRecord, LegacyHistory
THEOREM HistoricalRecordEquivalence ==
  HistoricalAuthentic <=>
    (p.legacy = LegacyHistory /\ HistoricalRecord(p.legacy)
     /\ 0 \in p.published /\ Closure(0) \subseteq p.objects)
BY SMT DEF HistoricalAuthentic, HistoricalRecord
THEOREM InvHistoricalAuthentic == Inv => HistoricalAuthentic
<1>1. Inv => p.legacy = LegacyHistory
  BY SMT DEF Inv, TypeOK
<1>2. Inv => 0 \in p.published /\ Closure(0) \subseteq p.objects
  BY SMT DEF Inv, PublicationChain, Retention
<1>3. QED BY SMT, <1>1, <1>2, LegacyRecordFacts, HistoricalRecordEquivalence
  DEF LegacyRecordFacts, HistoricalRecordEquivalence
THEOREM InvHistoricalAuthenticNext == Inv' => HistoricalAuthentic'
<1>1. Inv' => p'.legacy = LegacyHistory
  BY SMT DEF Inv, TypeOK
<1>2. Inv' => 0 \in p'.published /\ Closure(0) \subseteq p'.objects
  BY SMT DEF Inv, PublicationChain, Retention
<1>3. QED BY SMT, <1>1, <1>2, LegacyRecordFacts
  DEF LegacyRecordFacts, HistoricalAuthentic, HistoricalRecord
THEOREM HistoricalIdentity ==
  HistoricalAuthentic => p.legacy.input.identity = LegacyId
BY SMT, LegacyRecordFacts DEF LegacyRecordFacts, HistoricalAuthentic

(* Typed record-update contracts isolate control writes from the recovery
   guard and immutable historical-record predicates. *)
THEOREM WriterControlUpdateTyped ==
  \A w \in Writers, i \in Releases, phase \in Phases, result \in Outcomes :
    TypeOK /\
    p' = [p EXCEPT !.request[w] = i, !.outcome[w] = result, !.pc[w] = phase]
    => TypeOK'
BY SMT DEF TypeOK
THEOREM RecoveryCompletionUpdateTyped ==
  \A w \in Writers, i \in Releases \cup {LegacyId},
      phase \in RecoveryPhases, result \in RecoveryOutcomes :
    RecoveryTypeOK /\
    r' = [r EXCEPT !.pc[w] = phase, !.result[w] = result, !.checked[w] = i]
    => RecoveryTypeOK'
BY SMT DEF RecoveryTypeOK
RecoveryClientEvidence(w) ==
  /\ (r.result[w] = "complete" =>
       /\ r.target[w] \in Inputs
       /\ r.checked[w] = r.target[w]
       /\ r.target[w] \in p.receipt
       /\ BoundIntent(r.target[w]))
  /\ (r.result[w] = "already_applied" =>
       /\ r.target[w] = LegacyId
       /\ r.checked[w] = LegacyId
       /\ HistoricalAuthentic)
  /\ (r.pc[w] = "resumed" =>
       /\ r.target[w] \in Inputs
       /\ SchemaOf(r.target[w]) = 3)
THEOREM RecoveryEvidencePointwise ==
  RecoveryEvidence <=> (\A w \in Writers : RecoveryClientEvidence(w))
BY SMT DEF RecoveryEvidence, RecoveryClientEvidence
THEOREM RecoveryEvidencePointwiseNext ==
  RecoveryEvidence' <=> (\A w \in Writers : RecoveryClientEvidence(w)')
BY SMT DEF RecoveryEvidence, RecoveryClientEvidence
RecoveryDurableCarry ==
  /\ p.receipt \subseteq p'.receipt
  /\ \A i \in Inputs : BoundIntent(i) => BoundIntent(i)'
THEOREM RecoveryClientFrame == \A w \in Writers :
  RecoveryClientEvidence(w) /\ RecoveryDurableCarry /\ HistoricalAuthentic'
  /\ UNCHANGED <<r.target[w],r.checked[w],r.result[w],r.pc[w]>>
  => RecoveryClientEvidence(w)'
BY SMT DEF RecoveryClientEvidence, RecoveryDurableCarry
THEOREM RecoveryDurableFrame ==
  UNCHANGED <<p.receipt,p.intent>> => RecoveryDurableCarry
BY SMT DEF RecoveryDurableCarry, BoundIntent

THEOREM PublicationReceiptMonotone ==
  TypeOK /\ Next => p.receipt \subseteq p'.receipt
<1>1. \A w \in Writers, i \in Inputs :
  TypeOK /\ (Start(w,i) \/ Admit(w) \/ RetainIntent(w) \/ InputConflict(w))
  => p'.receipt = p.receipt
  BY SMT DEF TypeOK, Start, Admit, RetainIntent, InputConflict
<1>2. \A w \in Writers :
  TypeOK /\ (NativeCommit(w) \/ RetainManifest(w) \/ FinalRevalidate(w) \/ DoCAS(w))
  => p'.receipt = p.receipt
  BY SMT DEF TypeOK, NativeCommit, RetainManifest, FinalRevalidate, DoCAS, CASCore
<1>3. \A w \in Writers :
  TypeOK /\ (CASMismatch(w) \/ Finish(w) \/ RejectAdmission(w) \/ FailIO(w))
  => p'.receipt = p.receipt
  BY SMT DEF TypeOK, CASMismatch, Finish, RejectAdmission, FailIO
<1>4. \A w \in Writers :
  TypeOK /\ (LostResponse(w) \/ Crash(w) \/ Restart(w))
  => p'.receipt = p.receipt
  BY SMT DEF TypeOK, LostResponse, Crash, Restart
<1>5. \A i \in Inputs, a \in SUBSET FullChecks :
  TypeOK /\ Observe(i,a) => p'.receipt = p.receipt
  BY SMT DEF TypeOK, Observe
<1>6. \A i \in Inputs :
  TypeOK /\ RecordReceipt(i) => p.receipt \subseteq p'.receipt
  BY SMT DEF TypeOK, RecordReceipt
<1>7. Stutter => p'.receipt = p.receipt BY SMT DEF Stutter
<1>8. 1 \in Inputs BY SMT, ParameterBounds DEF ParameterBounds, Bounds, Inputs
<1>9. QED BY SMT, <1>1, <1>2, <1>3, <1>4, <1>5, <1>6, <1>7, <1>8
  DEF Next, NonObservationNext
THEOREM PublicationIntentCarry ==
  Inv /\ Next => \A i \in Inputs : BoundIntent(i) => BoundIntent(i)'
BY SMT, IntentNeverRebound, InputShape, InputDomainFacts
  DEF IntentNeverRebound, InputShape, InputDomainFacts, BoundIntent

THEOREM RecoveryInitPreserves == RecoveryInit => RecoveryInv
<1>1. RecoveryInit => Inv
  BY SMT, InitPreserves DEF InitPreserves, RecoveryInit
<1>2. RecoveryInit => RecoveryTypeOK
  BY SMT, ParameterBounds DEF ParameterBounds, RecoveryInit, RecoveryTypeOK,
    RecoveryPhases, RecoveryOutcomes, HintPhases, Releases, Bounds
<1>3. RecoveryInit => RecoveryEvidence
  BY SMT DEF RecoveryInit, RecoveryEvidence
<1>4. QED BY <1>1, <1>2, <1>3 DEF RecoveryInv
THEOREM PublicationStepPreserves == RecoveryInv /\ PublicationStep => RecoveryInv'
<1>1. ASSUME RecoveryInv, PublicationStep PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, NextPreserves DEF NextPreserves, RecoveryInv, PublicationStep
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1 DEF RecoveryInv, RecoveryTypeOK, PublicationStep
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1, PublicationReceiptMonotone, PublicationIntentCarry
      DEF PublicationReceiptMonotone, PublicationIntentCarry, RecoveryDurableCarry,
        RecoveryInv, Inv, PublicationStep
  <2>5. \A w \in Writers : RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv, PublicationStep
  <2>6. QED BY <2>1, <2>2, <2>5, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RecoverStartPreserves == \A w \in Writers, i \in ReplayInputs :
  RecoveryInv /\ RecoverStart(w,i) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, NEW i \in ReplayInputs, RecoveryInv, RecoverStart(w,i)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RecoverStart, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RecoverStart, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoverStart,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RecoverStart
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RecoverStart, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM MisleadHintPreserves == \A o \in Ops, i \in Releases, h \in HintPhases :
  RecoveryInv /\ MisleadHint(o,i,h) => RecoveryInv'
<1>1. ASSUME NEW o \in Ops, NEW i \in Releases, NEW h \in HintPhases, RecoveryInv, MisleadHint(o,i,h)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, MisleadHint, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, MisleadHint, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF MisleadHint,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, MisleadHint
  <2>6. QED BY <2>1, <2>2, <2>5, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM DetectCorruptionPreserves == \A i \in Releases :
  RecoveryInv /\ DetectCorruption(i) => RecoveryInv'
<1>1. ASSUME NEW i \in Releases, RecoveryInv, DetectCorruption(i)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, DetectCorruption, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, DetectCorruption, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF DetectCorruption,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, DetectCorruption
  <2>6. QED BY <2>1, <2>2, <2>5, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RefuseCorruptionPreserves == \A w \in Writers :
  RecoveryInv /\ RefuseCorruption(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RefuseCorruption(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RefuseCorruption, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RefuseCorruption, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RefuseCorruption,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RefuseCorruption
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RefuseCorruption, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM ReplayConflictPreserves == \A w \in Writers :
  RecoveryInv /\ ReplayConflict(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, ReplayConflict(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, ReplayConflict, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, ReplayConflict, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF ReplayConflict,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, ReplayConflict
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, ReplayConflict, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RefuseAuthorityPreserves == \A w \in Writers :
  RecoveryInv /\ RefuseAuthority(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RefuseAuthority(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RefuseAuthority, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RefuseAuthority, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RefuseAuthority,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RefuseAuthority
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RefuseAuthority, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM PredecessorConflictPreserves == \A w \in Writers :
  RecoveryInv /\ PredecessorConflict(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, PredecessorConflict(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, PredecessorConflict, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, PredecessorConflict, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF PredecessorConflict,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, PredecessorConflict
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, PredecessorConflict, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM ResumeExactPreserves == \A w \in Writers :
  RecoveryInv /\ ResumeExact(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, ResumeExact(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    <3>1. TypeOK'
      <4>1. r.target[w] \in Releases /\
        (IF r.target[w] \in p.manifest THEN "manifest"
         ELSE IF r.target[w] \in p.native THEN "native"
         ELSE IF BoundIntent(r.target[w]) THEN "intent" ELSE "planned") \in Phases
        BY SMT, <1>1, InputDomainFacts
          DEF InputDomainFacts, ResumeExact, Phases
      <4>2. QED BY SMT, <1>1, <4>1, WriterControlUpdateTyped
        DEF WriterControlUpdateTyped, RecoveryInv, Inv, ResumeExact, Outcomes
    <3>2. ReplyBound'
      BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, ResumeExact, ReplyBound
    <3>3. UNCHANGED <<Durable,p.auth,p.validated>>
      BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, ResumeExact, Durable
    <3>4. QED BY <1>1, <3>1, <3>2, <3>3, ControlFrameInv
      DEF RecoveryInv, ControlFrameInv
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, ResumeExact, RecoveryPhases, RecoveryOutcomes, ReplayInputs, Valid
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, ResumeExact,
      RecoveryDurableCarry, BoundIntent, Valid
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, ResumeExact, Valid
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, ResumeExact, Inspecting, Valid
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RepairModernCompletionPreserves == \A w \in Writers :
  RecoveryInv /\ RepairModernCompletion(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RepairModernCompletion(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    <3>1. r.target[w] \in Inputs /\ RecordReceipt(r.target[w])
      BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts,
        RepairModernCompletion, RecordReceipt
    <3>2. QED BY SMT, <1>1, <3>1, RecordReceiptPreserves
      DEF RecoveryInv, RecordReceiptPreserves
  <2>2. RecoveryTypeOK'
    <3>1. r.target[w] \in Releases \cup {LegacyId} /\
          "done" \in RecoveryPhases /\ "complete" \in RecoveryOutcomes
      BY SMT, <1>1, InputDomainFacts
        DEF InputDomainFacts, RepairModernCompletion, RecoveryPhases, RecoveryOutcomes
    <3>2. QED BY SMT, <1>1, <3>1, RecoveryCompletionUpdateTyped
      DEF RecoveryCompletionUpdateTyped, RecoveryInv, RepairModernCompletion
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, RepairModernCompletion,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RepairModernCompletion, Inv, TypeOK, BoundIntent
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RepairModernCompletion, Inspecting, Inv, TypeOK, BoundIntent
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RepairLegacyCompletionPreserves == \A w \in Writers :
  RecoveryInv /\ RepairLegacyCompletion(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RepairLegacyCompletion(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RepairLegacyCompletion, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RepairLegacyCompletion, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RepairLegacyCompletion,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RepairLegacyCompletion
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RepairLegacyCompletion, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RefusePendingLegacyPreserves == \A w \in Writers :
  RecoveryInv /\ RefusePendingLegacy(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RefusePendingLegacy(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RefusePendingLegacy, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RefusePendingLegacy, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RefusePendingLegacy,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RefusePendingLegacy
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RefusePendingLegacy, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RepairCompletionPreserves == \A w \in Writers :
  RecoveryInv /\ RepairCompletion(w) => RecoveryInv'
BY SMT, RepairModernCompletionPreserves, RepairLegacyCompletionPreserves
  DEF RepairModernCompletionPreserves, RepairLegacyCompletionPreserves, RepairCompletion
THEOREM ObserveCompletionPreserves == \A w \in Writers :
  RecoveryInv /\ ObserveCompletion(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, ObserveCompletion(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, ObserveCompletion, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, ObserveCompletion, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF ObserveCompletion,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, ObserveCompletion
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, ObserveCompletion, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RecoveryIOFailurePreserves == \A w \in Writers :
  RecoveryInv /\ RecoveryIOFailure(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RecoveryIOFailure(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, StutterPreserves DEF StutterPreserves, RecoveryInv, RecoveryIOFailure, Stutter
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RecoveryIOFailure, RecoveryPhases, RecoveryOutcomes, ReplayInputs
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoveryIOFailure,
      RecoveryDurableCarry, BoundIntent
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RecoveryIOFailure
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RecoveryIOFailure, Inspecting
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RecoveryCrashPreserves == \A w \in Writers :
  RecoveryInv /\ RecoveryCrash(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RecoveryCrash(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, CrashPreserves DEF CrashPreserves, RecoveryInv, RecoveryCrash
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RecoveryCrash, RecoveryPhases, RecoveryOutcomes, ReplayInputs, Crash
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, RecoveryCrash,
      RecoveryDurableCarry, BoundIntent, Crash
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RecoveryCrash, Crash
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RecoveryCrash, Inspecting, Crash
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1
THEOREM RecoveryRestartPreserves == \A w \in Writers :
  RecoveryInv /\ RecoveryRestart(w) => RecoveryInv'
<1>1. ASSUME NEW w \in Writers, RecoveryInv, RecoveryRestart(w)
      PROVE RecoveryInv'
  <2>1. Inv'
    BY SMT, <1>1, RestartPreserves DEF RestartPreserves, RecoveryInv, RecoveryRestart
  <2>2. RecoveryTypeOK'
    BY SMT, <1>1, InputDomainFacts DEF InputDomainFacts, RecoveryInv,
      RecoveryTypeOK, RecoveryRestart, RecoveryPhases, RecoveryOutcomes, ReplayInputs, Restart
  <2>3. HistoricalAuthentic'
    BY <2>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
  <2>4. RecoveryDurableCarry
    BY SMT, <1>1 DEF RecoveryInv, Inv, TypeOK, RecoveryRestart,
      RecoveryDurableCarry, BoundIntent, Restart
  <2>5. \A v \in Writers \ {w} : RecoveryClientEvidence(v)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryClientFrame, RecoveryEvidencePointwise
      DEF RecoveryClientFrame, RecoveryEvidencePointwise, RecoveryInv,
        RecoveryTypeOK, RecoveryRestart, Restart
  <2>6. RecoveryClientEvidence(w)'
    BY SMT, <1>1, <2>3, <2>4, RecoveryEvidencePointwise
      DEF RecoveryEvidencePointwise, RecoveryInv, RecoveryTypeOK,
        RecoveryClientEvidence, RecoveryDurableCarry, RecoveryRestart, Inspecting, Restart
  <2>7. QED BY SMT, <1>1, <2>1, <2>2, <2>5, <2>6, RecoveryEvidencePointwiseNext
    DEF RecoveryEvidencePointwiseNext, RecoveryInv
<1>2. QED BY <1>1

THEOREM RecoveryStutterPreserves == RecoveryInv /\ RecoveryStutter => RecoveryInv'
<1>1. RecoveryInv /\ RecoveryStutter => Inv'
  BY SMT, StutterPreserves DEF StutterPreserves, RecoveryInv,
    RecoveryStutter, recoveryVars, Stutter
<1>2. RecoveryInv /\ RecoveryStutter => RecoveryTypeOK'
  BY SMT DEF RecoveryInv, RecoveryStutter, recoveryVars, RecoveryTypeOK
<1>3. RecoveryInv /\ RecoveryStutter => HistoricalAuthentic'
  BY <1>1, InvHistoricalAuthenticNext DEF InvHistoricalAuthenticNext
<1>4. RecoveryInv /\ RecoveryStutter => RecoveryEvidence'
  BY SMT, <1>3, RecoveryEvidencePointwise, RecoveryEvidencePointwiseNext,
    RecoveryClientFrame, RecoveryDurableFrame
    DEF RecoveryEvidencePointwise, RecoveryEvidencePointwiseNext,
      RecoveryClientFrame, RecoveryDurableFrame, RecoveryInv, RecoveryStutter, recoveryVars
<1>5. QED BY <1>1, <1>2, <1>4 DEF RecoveryInv
THEOREM RepairDoesNotApplyAgain == \A w \in Writers : RecoveryInv /\ RepairCompletion(w) =>
  /\ UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.objects,p.evidence,p.legacy>>
  /\ p'.receipt = (IF r.target[w] = LegacyId THEN p.receipt
                  ELSE p.receipt \cup {r.target[w]})
BY SMT DEF RepairCompletion, RepairModernCompletion, RepairLegacyCompletion, Inspecting, LegacyId, Inputs
  , RecoveryInv, Inv, TypeOK, RecoveryTypeOK
THEOREM RefusalFrames == \A w \in Writers :
  RecoveryInv /\ (RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
   \/ PredecessorConflict(w) \/ RecoveryIOFailure(w) \/ RefusePendingLegacy(w)) => UNCHANGED Durable
<1>1. \A w \in Writers :
  (RefuseCorruption(w) \/ ReplayConflict(w) \/ RefuseAuthority(w)
   \/ PredecessorConflict(w) \/ RecoveryIOFailure(w) \/ RefusePendingLegacy(w))
  => UNCHANGED p
  BY DEF RefuseCorruption, ReplayConflict, RefuseAuthority,
    PredecessorConflict, RecoveryIOFailure, RefusePendingLegacy
<1>2. UNCHANGED p => UNCHANGED Durable
  BY DEF Durable
<1>3. QED BY <1>1, <1>2
THEOREM HintHasNoAuthority == \A o \in Ops, i \in Releases, h \in HintPhases :
  RecoveryInv /\ MisleadHint(o,i,h) => UNCHANGED <<p,r.target,r.pc,r.result,r.checked,r.corrupt>>
BY SMT DEF MisleadHint, RecoveryInv, RecoveryTypeOK
THEOREM CorruptCurrentRefused == \A w \in Writers :
  RecoveryInv /\ Inspecting(w,r.target[w]) /\ p.head \in r.corrupt =>
    /\ ~ResumeExact(w) /\ ~RepairCompletion(w)
    /\ (RefuseCorruption(w) => p'.head = p.head /\ r'.result[w] = "refused")
<1>1. ASSUME NEW w \in Writers, RecoveryInv,
      Inspecting(w,r.target[w]), p.head \in r.corrupt
      PROVE /\ ~ResumeExact(w) /\ ~RepairCompletion(w)
            /\ (RefuseCorruption(w) => p'.head = p.head /\ r'.result[w] = "refused")
  <2>1. ~ResumeExact(w)
    BY SMT, <1>1 DEF ResumeExact, Intact
  <2>2. ~RepairModernCompletion(w) /\ ~RepairLegacyCompletion(w)
    BY SMT, <1>1 DEF RepairModernCompletion, RepairLegacyCompletion, Intact
  <2>3. RefuseCorruption(w) => p'.head = p.head /\ r'.result[w] = "refused"
    BY SMT, <1>1 DEF RefuseCorruption, RecoveryInv, RecoveryTypeOK
  <2>4. QED BY <2>1, <2>2, <2>3 DEF RepairCompletion
<1>2. QED BY <1>1
THEOREM ChangedInputConflicts == \A w \in Writers :
  RecoveryInv /\ r.target[w] \in Inputs /\ Inspecting(w,r.target[w]) /\ Intact(r.target[w])
  /\ p.intent[OpOf(r.target[w])] \notin {0,r.target[w]} =>
    /\ ~ResumeExact(w) /\ ~RepairCompletion(w)
    /\ (ReplayConflict(w) => UNCHANGED Durable)
<1>1. ASSUME NEW w \in Writers, RecoveryInv, r.target[w] \in Inputs,
      Inspecting(w,r.target[w]), Intact(r.target[w]),
      p.intent[OpOf(r.target[w])] \notin {0,r.target[w]}
      PROVE /\ ~ResumeExact(w) /\ ~RepairCompletion(w)
            /\ (ReplayConflict(w) => UNCHANGED Durable)
  <2>1. ~ResumeExact(w) /\ ~RepairModernCompletion(w)
    BY SMT, <1>1 DEF ResumeExact, RepairModernCompletion, BoundIntent
  <2>2. r.target[w] # LegacyId
    BY SMT, <1>1 DEF Inputs, LegacyId
  <2>3. ~RepairLegacyCompletion(w)
    BY SMT, <1>1, <2>2 DEF RepairLegacyCompletion, Inspecting
  <2>4. ReplayConflict(w) => UNCHANGED Durable
    BY SMT, <1>1, RefusalFrames DEF RefusalFrames
  <2>5. QED BY <2>1, <2>3, <2>4 DEF RepairCompletion
<1>2. QED BY <1>1
THEOREM RecoveryNextPreserves == RecoveryInv /\ RecoveryNext => RecoveryInv'
BY SMT, PublicationStepPreserves, RecoverStartPreserves, MisleadHintPreserves,
  DetectCorruptionPreserves, RefuseCorruptionPreserves, ReplayConflictPreserves,
  RefuseAuthorityPreserves, PredecessorConflictPreserves, ResumeExactPreserves,
  RepairCompletionPreserves, ObserveCompletionPreserves, RecoveryIOFailurePreserves,
  RecoveryCrashPreserves, RecoveryRestartPreserves, RecoveryStutterPreserves,
  RefusePendingLegacyPreserves
  DEF PublicationStepPreserves, RecoverStartPreserves, MisleadHintPreserves,
    DetectCorruptionPreserves, RefuseCorruptionPreserves, ReplayConflictPreserves,
    RefuseAuthorityPreserves, PredecessorConflictPreserves, ResumeExactPreserves,
    RepairCompletionPreserves, ObserveCompletionPreserves, RecoveryIOFailurePreserves,
    RecoveryCrashPreserves, RecoveryRestartPreserves, RecoveryStutterPreserves,
    RefusePendingLegacyPreserves, RecoveryNext, LocalRecoveryNext
THEOREM RecoveryInvImpliesSafety == RecoveryInv => RecoverySafety
<1>1. ASSUME RecoveryInv PROVE RecoverySafety
  <2>1. Safety
    BY SMT, <1>1, InvImpliesSafety DEF InvImpliesSafety, RecoveryInv
  <2>2. \A w \in Writers : r.result[w] = "complete" =>
    /\ r.target[w] \in p.published
    /\ Input(r.checked[w]) = Input(r.target[w])
    /\ p.intent[OpOf(r.target[w])] = r.target[w]
    BY SMT, <1>1 DEF RecoveryInv, RecoveryEvidence, Inv, ReceiptSound, BoundIntent
  <2>3. \A w \in Writers : r.result[w] = "already_applied" =>
    /\ r.target[w] = p.legacy.input.identity
    /\ p.legacy.receipt.input = LegacyInput
    /\ p.legacy.input.schema = 2
    /\ p.legacy.input.domain = "kctx-operation-v2"
    /\ HistoricalAuthentic
    <3>1. HistoricalAuthentic
      BY <1>1, InvHistoricalAuthentic DEF InvHistoricalAuthentic, RecoveryInv
    <3>2. p.legacy.input.identity = LegacyId
      BY <3>1, HistoricalIdentity DEF HistoricalIdentity
    <3>3. p.legacy.receipt.input = LegacyInput /\
          p.legacy.input.schema = 2 /\
          p.legacy.input.domain = "kctx-operation-v2"
      BY SMT, <3>1 DEF HistoricalAuthentic
    <3>4. \A w \in Writers :
      r.result[w] = "already_applied" => r.target[w] = LegacyId
      BY SMT, <1>1 DEF RecoveryInv, RecoveryEvidence
    <3>5. QED BY <3>1, <3>2, <3>3, <3>4
  <2>4. \A w \in Writers : r.target[w] = InputCount => r.pc[w] # "resumed"
    BY SMT, <1>1 DEF RecoveryInv, RecoveryEvidence, SchemaOf
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF RecoverySafety
<1>2. QED BY <1>1
THEOREM LegacyReplayNeverReapplies == \A w \in Writers :
  RecoveryInv /\ RepairLegacyCompletion(w) =>
    /\ UNCHANGED Durable
    /\ r'.result[w] = "already_applied"
    /\ p.legacy.receipt.input = LegacyInput
BY SMT DEF RepairLegacyCompletion, HistoricalAuthentic, Durable, RecoveryInv, RecoveryTypeOK
THEOREM PendingLegacyNeverResumes == \A w \in Writers :
  r.target[w] \in Inputs /\ SchemaOf(r.target[w]) = 2 =>
    /\ ~ResumeExact(w)
    /\ ~RepairModernCompletion(w)
    /\ ~ObserveCompletion(w)
BY SMT DEF ResumeExact, RepairModernCompletion, ObserveCompletion, Valid
THEOREM LegacyCompletedAdmission == RecoveryInv => HistoricalAuthentic
BY InvHistoricalAuthentic DEF InvHistoricalAuthentic, RecoveryInv
THEOREM RecoveryInductiveSafety == RecoverySpec => []RecoverySafety
<1>1. RecoveryInit => RecoveryInv
  BY RecoveryInitPreserves DEF RecoveryInitPreserves
<1>2. RecoveryInv /\ [RecoveryNext]_recoveryVars => RecoveryInv'
  BY RecoveryNextPreserves, RecoveryStutterPreserves
    DEF RecoveryNextPreserves, RecoveryStutterPreserves, RecoveryStutter
<1>3. RecoverySpec => []RecoveryInv BY <1>1, <1>2, PTL DEF RecoverySpec
<1>4. QED BY <1>3, RecoveryInvImpliesSafety, PTL DEF RecoveryInvImpliesSafety

THEOREM LocalRecoveryDecomposition ==
  LocalRecoveryNext <=> (NonHintRecoveryNext \/ UnrestrictedHintStep)
BY DEF LocalRecoveryNext, NonHintRecoveryNext, UnrestrictedHintStep
THEOREM FiniteHintRefines == FiniteHintStep => UnrestrictedHintStep
BY SMT, ParameterBounds DEF ParameterBounds, FiniteHintStep, UnrestrictedHintStep,
  Ops, Releases, Bounds
THEOREM CrossHintRefines == CrossHintStep => UnrestrictedHintStep
BY SMT, ParameterBounds DEF ParameterBounds, CrossHintStep, UnrestrictedHintStep,
  Ops, Releases, HintPhases, Bounds
THEOREM HintProjectionStutters ==
  RecoveryInv /\ UnrestrictedHintStep => UNCHANGED RecoveryObservation
BY SMT, HintHasNoAuthority DEF HintHasNoAuthority, UnrestrictedHintStep, RecoveryObservation
THEOREM FiniteHintProjectionStutters ==
  RecoveryInv /\ (FiniteHintStep \/ CrossHintStep) => UNCHANGED RecoveryObservation
BY FiniteHintRefines, CrossHintRefines, HintProjectionStutters
  DEF FiniteHintRefines, CrossHintRefines, HintProjectionStutters

THEOREM WorkloadTargetsTyped == \A w \in Writers, workload \in 0..3 :
  WorkloadTarget(w,workload) \in ReplayInputs
BY SMT, ParameterBounds DEF ParameterBounds, WorkloadTarget, ReplayInputs, Inputs, Bounds
THEOREM RecoveryServiceRefines == RecoveryServiceNext => NonHintRecoveryNext
BY DEF RecoveryServiceNext, NonHintRecoveryNext
THEOREM WorkloadLocalRefines == \A workload \in 0..3 :
  WorkloadLocalNext(workload) => NonHintRecoveryNext
BY SMT, WorkloadTargetsTyped, RecoveryServiceRefines
  DEF WorkloadTargetsTyped, RecoveryServiceRefines, WorkloadLocalNext, NonHintRecoveryNext
THEOREM WorkloadRecoveryStepRefines == \A workload \in 0..3 :
  WorkloadRecoveryNext(workload) => RecoveryNext
BY SMT, FiniteStepRefines, WorkloadLocalRefines, FiniteHintRefines, LocalRecoveryDecomposition
  DEF FiniteStepRefines, WorkloadLocalRefines, FiniteHintRefines, LocalRecoveryDecomposition,
    WorkloadRecoveryNext, FinitePublicationStep, PublicationStep, RecoveryNext
THEOREM WorkloadRecoverySpecRefines == \A workload \in 0..3 :
  WorkloadRecoverySpec(workload) => RecoverySpec
<1>1. ASSUME NEW workload \in 0..3
      PROVE WorkloadRecoverySpec(workload) => RecoverySpec
  <2>1. WorkloadRecoveryNext(workload) => RecoveryNext
    BY <1>1, WorkloadRecoveryStepRefines DEF WorkloadRecoveryStepRefines
  <2>2. QED BY <2>1, PTL DEF WorkloadRecoverySpec, RecoverySpec
<1>2. QED BY <1>1

THEOREM FinitePublicationRefines == FinitePublicationStep => PublicationStep
BY SMT, FiniteStepRefines DEF FiniteStepRefines, FinitePublicationStep, PublicationStep
THEOREM FiniteRecoveryStepRefines == FiniteRecoveryNext => RecoveryNext
BY WorkloadRecoveryStepRefines DEF WorkloadRecoveryStepRefines, FiniteRecoveryNext
THEOREM FiniteRecoverySpecRefines == FiniteRecoverySpec => RecoverySpec
BY FiniteRecoveryStepRefines, PTL DEF FiniteRecoveryStepRefines, FiniteRecoverySpec, RecoverySpec

THEOREM WideRecoveryStepRefines == \A profile \in 0..3 :
  WideRecoveryNext(profile) => RecoveryNext
BY SMT, WideStepRefines, WorkloadLocalRefines, FiniteHintRefines, LocalRecoveryDecomposition
  DEF WideStepRefines, WorkloadLocalRefines, FiniteHintRefines, LocalRecoveryDecomposition,
    WideRecoveryNext, WidePublicationStep, RecoveryNext, PublicationStep
THEOREM WideRecoverySpecRefines == \A profile \in 0..3 :
  WideRecoverySpec(profile) => RecoverySpec
<1>1. ASSUME NEW profile \in 0..3
      PROVE WideRecoverySpec(profile) => RecoverySpec
  <2>1. WideRecoveryNext(profile) => RecoveryNext
    BY <1>1, WideRecoveryStepRefines DEF WideRecoveryStepRefines
  <2>2. QED BY <2>1, PTL DEF WideRecoverySpec, RecoverySpec
<1>2. QED BY <1>1

THEOREM CrossHintRecoveryStepRefines == CrossHintRecoveryNext => RecoveryNext
BY SMT, WideStepRefines, WorkloadLocalRefines, CrossHintRefines, LocalRecoveryDecomposition
  DEF WideStepRefines, WorkloadLocalRefines, CrossHintRefines, LocalRecoveryDecomposition,
    CrossHintRecoveryNext, WidePublicationStep, RecoveryNext, PublicationStep
THEOREM CrossHintRecoverySpecRefines == CrossHintRecoverySpec => RecoverySpec
BY CrossHintRecoveryStepRefines, PTL
  DEF CrossHintRecoveryStepRefines, CrossHintRecoverySpec, RecoverySpec

THEOREM LegacyReplayProbeStepRefines == LegacyReplayProbeNext => RecoveryNext
BY SMT, ParameterBounds DEF ParameterBounds, LegacyReplayProbeNext, RecoveryNext,
  LocalRecoveryNext, RepairCompletion, ReplayInputs, Writers, Bounds
THEOREM LegacyReplayProbeSpecRefines == LegacyReplayProbeSpec => RecoverySpec
BY LegacyReplayProbeStepRefines, PTL
  DEF LegacyReplayProbeStepRefines, LegacyReplayProbeSpec, RecoverySpec
THEOREM PreparedProbePublicationRefines == PreparedProbePublicationStep => PublicationStep
BY SMT, ParameterBounds DEF ParameterBounds, PreparedProbePublicationStep, PublicationStep,
  Next, NonObservationNext, Writers, Inputs, Bounds
THEOREM PreparedRecoveryProbeStepRefines == PreparedRecoveryProbeNext => RecoveryNext
BY SMT, ParameterBounds, PreparedProbePublicationRefines
  DEF ParameterBounds, PreparedProbePublicationRefines, PreparedRecoveryProbeNext,
    RecoveryNext, LocalRecoveryNext, ReplayInputs, Writers, Inputs, Bounds
THEOREM PreparedRecoveryProbeSpecRefines == PreparedRecoveryProbeSpec => RecoverySpec
BY PreparedRecoveryProbeStepRefines, PTL
  DEF PreparedRecoveryProbeStepRefines, PreparedRecoveryProbeSpec, RecoverySpec
=============================================================================
