------------------------- MODULE M05RemoteClosure -------------------------
EXTENDS Naturals, FiniteSets, TLAPS
CONSTANTS WriterCount, InputCount, ContextCount, ClientCount
VARIABLES p, q
P == INSTANCE M01ReleasePublication
Clients == 1..ClientCount
Inputs == P!Inputs
Releases == P!Releases
Objects == P!Objects
Contexts == P!Contexts
Input(i) == P!Input(i)
Closure(i) == P!Closure(i)
BaseOf(i) == P!BaseOf(i)
ASSUME RemoteParameterBounds == P!Bounds /\ ClientCount \in Nat /\ ClientCount >= 2
USE RemoteParameterBounds DEF P!Bounds
vars == <<p,q>>

(***************************************************************************
Runtime map: p is the ACTUAL M01 publication state, not an assumed set of
valid releases. TransferGit/Native/Capture are separate immutable transport
acknowledgments. ReceiverVerify means an independent receiver read validates
bytes, native reconstruction, exact signature/domain and the entire named
context family. Acknowledge persists that exact verification result; a lost
reply never rolls it back. ClientVerify independently checks the durable
receipt and exact closure, not a pooled context or an operation-ID match.
Advertise is the conditional completed-ref update, never a Git/Dolt transaction.
StageObject copies to an empty operation-owned staging destination; StageVerify
reads that destination independently, even after SourceDisappear. SelectRestore
only installs the verified staged closure, retaining earlier selected data.
ReplicaRead is observation only: no initialization, recovery, capture or signing.

Trust: content-addressed immutable objects, retention after durable receiver
acknowledgment, qualified atomic ref CAS, exact canonical byte interning and
crypto/kernel validation, and isolated owned staging with path/KB validation.
Fault observations are typed refusal results, NOT destructive writes to an
already acknowledged store. Such destruction violates the retention premise;
no fallback-to-an-older-current action is supplied. Process crashes preserve
durable receiver/staging data; no host power-loss or enterprise endpoint
qualification is asserted. No fairness/eventual-progress theorem is asserted.

Schema 2 is completed bootstrap history only. Its exact historical bytes and
domain reside in M01's p.legacy = LegacyHistory, not in a newly authored plan.
ReadFamily projects LegacyInput.context under the explicit legacy name; it
does not re-sign or pool it. M01 rejects new schema-2 publication.
***************************************************************************)
Kinds == {"git","native","capture"}
Errors == {"unknown","corrupt","identity","unavailable","permission"}
Results == Errors \cup {"none","pending","complete","conflict","read"}
Phases == {"idle","transfer","receiver","ack","verified","advertised",
           "restore","staged","selected","failed","conflict","down"}
ReadContexts(i) == IF i = 0 THEN {"legacy"} ELSE Contexts
ReadFamily(i) == IF i = 0 THEN [legacy |-> P!LegacyInput.context] ELSE Input(i).family
ExactRecord(i) == <<Input(i),Input(i).family,Input(i).approvalBytes>>

(* Fixed representative failure profiles, not a global failure-count cap.
   Multiple inputs can fail independently and simultaneously. Unrestricted
   Init allows every error assignment; finite fixtures choose these profiles.
   Profiles do not constrain any theorem or hide concurrent clients/phases. *)
FailureProfiles ==
  { [i \in Releases |-> "ok"],
    [i \in Releases |-> IF i % 2 = 0 THEN "corrupt" ELSE "identity"],
    [i \in Releases |-> IF i % 2 = 0 THEN "unavailable" ELSE "unknown"],
    [i \in Releases |-> IF i = 1 THEN "permission" ELSE "ok"] }
AuthorityProfiles == {P!FullChecks,
  P!FullChecks \ {<<"policy",1>>},
  P!FullChecks \ {<<"signature",ContextCount>>},
  P!FullChecks \ {<<"policy",1>>,<<"signature",ContextCount>>}}

RemoteTypeOK == q \in [received : SUBSET Objects,
  receiverVerified : SUBSET Releases, acknowledged : SUBSET Releases,
  advertised : Releases, history : SUBSET Releases,
  advertisementEvidence : SUBSET Releases, stagedEvidence : SUBSET Releases,
  target : [Clients -> Releases], expected : [Clients -> Releases],
  pc : [Clients -> Phases], result : [Clients -> Results],
  checked : [Clients -> BOOLEAN], stage : [Clients -> SUBSET Objects],
  stageChecked : [Clients -> BOOLEAN], destination : SUBSET Objects,
  restored : SUBSET Releases, selected : [Clients -> Releases],
  sourceAvailable : BOOLEAN, fault : [Releases -> Errors \cup {"ok"}]]
TypeOK == P!TypeOK /\ RemoteTypeOK
TransferRoots ==
  /\ (\A o \in q.received : o[2] \in p.published)
  /\ q.received \subseteq p.objects
ReceiverSound ==
  /\ q.acknowledged \subseteq q.receiverVerified
  /\ (\A i \in q.receiverVerified :
       i \in p.published /\ Closure(i) \subseteq q.received)
AdvertisementChain ==
  /\ 0 \in q.history
  /\ q.advertised \in q.history
  /\ q.history \subseteq q.acknowledged
  /\ q.history \subseteq q.advertisementEvidence
  /\ q.advertisementEvidence \subseteq q.acknowledged
  /\ (\A i \in q.history : i <= q.advertised)
  /\ (\A i \in q.history \ {0} :
       BaseOf(i) \in q.history /\ Input(i).schema = 3)
  /\ (\A i,j \in q.history \ {0} : BaseOf(i) = BaseOf(j) => i = j)
ClientSound ==
  /\ (\A c \in Clients :
       /\ (q.checked[c] => q.target[c] \in q.acknowledged)
       /\ (q.stageChecked[c] =>
            Closure(q.target[c]) \subseteq q.stage[c] /\ q.target[c] \in q.history
            /\ q.target[c] \in q.stagedEvidence)
       /\ (\A o \in q.stage[c] : o[2] = q.target[c] /\ o[2] \in q.history))
  /\ q.restored \subseteq q.history
  /\ q.restored \subseteq q.stagedEvidence
  /\ q.stagedEvidence \subseteq q.history
  /\ (\A i \in q.restored : Closure(i) \subseteq q.destination)
  /\ (\A c \in Clients : q.selected[c] \in q.restored)
Inv == TypeOK /\ P!Inv /\ TransferRoots /\ ReceiverSound
       /\ AdvertisementChain /\ ClientSound
Safety ==
  /\ P!Safety
  /\ (\A i \in q.history :
       /\ i \in p.published
       /\ Closure(i) \subseteq q.received
       /\ i \in q.acknowledged
       /\ i \in q.advertisementEvidence
       /\ (i # 0 => Input(i).schema = 3 /\ BaseOf(i) \in q.history))
  /\ (\A i \in q.history : i <= q.advertised)
  /\ (\A i,j \in q.history \ {0} :
       Input(i).predecessor = Input(j).predecessor => ExactRecord(i) = ExactRecord(j))
  /\ (\A c \in Clients :
       Closure(q.selected[c]) \subseteq q.destination /\ q.selected[c] \in q.stagedEvidence)

RemoteInit == q \in {
  [received |-> Closure(0), receiverVerified |-> {0}, acknowledged |-> {0},
   advertised |-> 0, history |-> {0}, advertisementEvidence |-> {0},
   stagedEvidence |-> {0}, target |-> [c \in Clients |-> 0],
   expected |-> [c \in Clients |-> 0], pc |-> [c \in Clients |-> "idle"],
   result |-> [c \in Clients |-> "none"], checked |-> [c \in Clients |-> FALSE],
   stage |-> [c \in Clients |-> {}], stageChecked |-> [c \in Clients |-> FALSE],
   destination |-> Closure(0), restored |-> {0}, selected |-> [c \in Clients |-> 0],
   sourceAvailable |-> TRUE, fault |-> f] : f \in [Releases -> Errors \cup {"ok"}]}
Init == P!Init /\ RemoteInit
FiniteInit == Init /\ q.fault \in FailureProfiles
LocalPublication == P!Next /\ UNCHANGED q
StartTransfer(c,i) ==
  /\ q.pc[c] \in {"idle","selected","advertised","failed","conflict"}
  /\ i \in p.published \ {0}
  /\ q' = [q EXCEPT !.target[c] = i, !.expected[c] = BaseOf(i),
       !.pc[c] = "transfer", !.result[c] = "pending", !.checked[c] = FALSE,
       !.stage[c] = {}, !.stageChecked[c] = FALSE]
  /\ UNCHANGED p
TransferObject(c,k) == LET i == q.target[c] IN
  /\ q.pc[c] = "transfer" /\ i \in p.published
  /\ <<k,i>> \in p.objects
  /\ q' = [q EXCEPT !.received = @ \cup {<<k,i>>}]
  /\ UNCHANGED p
TransferGit(c) == TransferObject(c,"git")
TransferNative(c) == TransferObject(c,"native")
TransferCapture(c) == TransferObject(c,"capture")
ReceiverVerify(c) == LET i == q.target[c] IN
  /\ q.pc[c] = "transfer" /\ i \in p.published
  /\ Closure(i) \subseteq q.received /\ q.fault[i] = "ok"
  /\ q' = [q EXCEPT !.receiverVerified = @ \cup {i}, !.pc[c] = "receiver"]
  /\ UNCHANGED p
Acknowledge(c) == LET i == q.target[c] IN
  /\ q.pc[c] = "receiver" /\ i \in q.receiverVerified
  /\ q' = [q EXCEPT !.acknowledged = @ \cup {i}, !.pc[c] = "ack"]
  /\ UNCHANGED p
ClientVerify(c) == LET i == q.target[c] IN
  /\ q.pc[c] = "ack" /\ i \in q.acknowledged
  /\ Closure(i) \subseteq q.received /\ q.fault[i] = "ok"
  /\ q' = [q EXCEPT !.checked[c] = TRUE, !.pc[c] = "verified"]
  /\ UNCHANGED p
AdvertiseCore(c,closureGuard,predecessorGuard) == LET i == q.target[c] IN
  /\ i \in p.published \ {0}
  /\ q.pc[c] \in {"transfer","receiver","ack","verified"}
  /\ (closureGuard => q.pc[c] = "verified" /\ q.checked[c]
       /\ i \in q.acknowledged /\ Closure(i) \subseteq q.received)
  /\ (predecessorGuard => q.advertised = q.expected[c]
       /\ q.expected[c] = BaseOf(i))
  /\ q' = [q EXCEPT !.advertised = i, !.history = @ \cup {i},
       !.advertisementEvidence = @ \cup (IF q.checked[c] THEN {i} ELSE {}),
       !.pc[c] = "advertised", !.result[c] = "complete"]
  /\ UNCHANGED p
Advertise(c) == AdvertiseCore(c,TRUE,TRUE)
MutantEarlyAdvertisement(c) == AdvertiseCore(c,FALSE,TRUE)
MutantStalePointer(c) == AdvertiseCore(c,TRUE,FALSE)
PointerConflict(c) ==
  /\ q.pc[c] = "verified" /\ q.advertised # q.expected[c]
  /\ q' = [q EXCEPT !.pc[c] = "conflict", !.result[c] = "conflict"]
  /\ UNCHANGED p
BeginRestore(c,i) ==
  /\ q.pc[c] \in {"idle","selected","advertised","failed","conflict"}
  /\ i \in q.history /\ q.sourceAvailable
  /\ q' = [q EXCEPT !.target[c] = i, !.stage[c] = {},
       !.stageChecked[c] = FALSE, !.checked[c] = FALSE,
       !.pc[c] = "restore", !.result[c] = "pending"]
  /\ UNCHANGED p
StageObject(c,k) == LET i == q.target[c] IN
  /\ q.pc[c] = "restore" /\ q.sourceAvailable /\ i \in q.history
  /\ <<k,i>> \in q.received
  /\ q' = [q EXCEPT !.stage[c] = @ \cup {<<k,i>>}]
  /\ UNCHANGED p
SourceDisappear == /\ q.sourceAvailable
  /\ q' = [q EXCEPT !.sourceAvailable = FALSE] /\ UNCHANGED p
StageVerify(c) == LET i == q.target[c] IN
  /\ q.pc[c] = "restore" /\ i \in q.history
  /\ Closure(i) \subseteq q.stage[c] /\ q.fault[i] = "ok"
  /\ q' = [q EXCEPT !.stageChecked[c] = TRUE, !.stagedEvidence = @ \cup {i}, !.pc[c] = "staged"]
  /\ UNCHANGED p
SelectRestore(c) == LET i == q.target[c] IN
  /\ q.pc[c] = "staged" /\ q.stageChecked[c]
  /\ q' = [q EXCEPT !.selected[c] = i, !.destination = @ \cup q.stage[c],
       !.restored = @ \cup {i}, !.pc[c] = "selected", !.result[c] = "complete"]
  /\ UNCHANGED p
ReplicaRead(c,i,context) ==
  /\ q.pc[c] # "down" /\ i \in q.history /\ context \in ReadContexts(i)
  /\ q.fault[i] = "ok" /\ Closure(i) \subseteq q.received
  /\ q' = [q EXCEPT !.result[c] = "read"] /\ UNCHANGED p
ReplicaReadFailure(c,i,e) ==
  /\ q.pc[c] # "down" /\ i \in q.history /\ e \in Errors /\ q.fault[i] = e
  /\ q' = [q EXCEPT !.result[c] = e] /\ UNCHANGED p
TypedFailure(c,e) ==
  /\ q.pc[c] \in {"transfer","receiver","ack","verified","restore","staged"}
  /\ e \in Errors /\ q.fault[q.target[c]] = e
  /\ q' = [q EXCEPT !.pc[c] = "failed", !.result[c] = e] /\ UNCHANGED p
NetworkInterrupt(c) ==
  /\ q.pc[c] \notin {"idle","down"}
  /\ q' = [q EXCEPT !.pc[c] = "failed", !.result[c] = "unknown"] /\ UNCHANGED p
Crash(c) == /\ q.pc[c] # "down"
  /\ q' = [q EXCEPT !.pc[c] = "down", !.result[c] = "unknown",
       !.checked[c] = FALSE, !.stageChecked[c] = FALSE] /\ UNCHANGED p
Restart(c) == /\ q.pc[c] = "down"
  /\ q' = [q EXCEPT !.pc[c] = "idle", !.result[c] = "none"] /\ UNCHANGED p
Stutter == UNCHANGED vars
RemoteNext ==
  \/ \E c \in Clients, i \in Releases : StartTransfer(c,i) \/ BeginRestore(c,i)
  \/ \E c \in Clients : TransferGit(c) \/ TransferNative(c) \/ TransferCapture(c)
       \/ ReceiverVerify(c) \/ Acknowledge(c) \/ ClientVerify(c) \/ Advertise(c)
       \/ PointerConflict(c) \/ StageVerify(c) \/ SelectRestore(c)
       \/ NetworkInterrupt(c) \/ Crash(c) \/ Restart(c)
  \/ \E c \in Clients, k \in Kinds : StageObject(c,k)
  \/ \E c \in Clients, e \in Errors : TypedFailure(c,e)
  \/ \E c \in Clients, i \in Releases : \E context \in ReadContexts(i) : ReplicaRead(c,i,context)
  \/ \E c \in Clients, i \in Releases, e \in Errors : ReplicaReadFailure(c,i,e)
  \/ SourceDisappear \/ Stutter
Next == LocalPublication \/ RemoteNext
Spec == Init /\ [][Next]_vars

(* Finite profiles restrict only observation choices; no transition/phase,
   competing actor, generation, or failure-count bound is imposed. *)
FiniteLocalNext ==
  \/ \E w \in P!Writers, i \in Inputs : P!Start(w,i)
  \/ \E w \in P!Writers : P!Admit(w) \/ P!RetainIntent(w) \/ P!InputConflict(w)
       \/ P!NativeCommit(w) \/ P!RetainManifest(w) \/ P!FinalRevalidate(w)
       \/ P!DoCAS(w) \/ P!CASMismatch(w) \/ P!Finish(w) \/ P!RejectAdmission(w)
       \/ P!FailIO(w) \/ P!LostResponse(w) \/ P!Crash(w) \/ P!Restart(w)
  \/ \E i \in Inputs : P!RecordReceipt(i)
  \/ \E i \in Inputs, a \in AuthorityProfiles : P!Observe(i,a)
  \/ P!Stutter
FiniteNext == (FiniteLocalNext /\ UNCHANGED q) \/ RemoteNext
EarlyAdvertisementNext == FiniteNext \/ (\E c \in Clients : MutantEarlyAdvertisement(c))
StalePointerNext == FiniteNext \/ (\E c \in Clients : MutantStalePointer(c))

(* Source obligations only: TLC/TLAPS execution and backend qualification are
   deliberately unrun. Each action, including every failure and stutter, has
   its own preservation obligation; no blanket imported safety claim. *)
(* Instance theorem statements are expanded explicitly: their identifiers
   alone are opaque formulas to the backend. Parameter assumptions above are
   exactly the pre-existing structural bounds, not proof-specific premises. *)
USE DEF P!InitPreserves, P!NextPreserves, P!StutterPreserves,
      P!InvImpliesSafety, P!InputShape, P!InputDomainFacts

THEOREM PublicationFrame == P!Inv /\ UNCHANGED p => P!Inv'
BY SMT, P!StutterPreserves DEF P!Stutter, P!StutterPreserves
USE DEF PublicationFrame

THEOREM RemoteDomainFacts ==
  /\ 0 \in Releases /\ Inputs = Releases \ {0}
  /\ Releases \subseteq Nat
  /\ Closure(0) \subseteq Objects
  /\ \A i \in Inputs : BaseOf(i) \in Releases /\ BaseOf(i) < i
BY SMT, RemoteParameterBounds, P!InputShape
  DEF Inputs, Releases, Objects, Closure, BaseOf, P!Inputs, P!Releases,
      P!Objects, P!Closure, P!Bounds, RemoteParameterBounds, P!InputShape
USE DEF RemoteDomainFacts

(* Publication changes are split per reducer action. This avoids expanding
   every record update and every invariant in a single SMT query. *)
THEOREM PublicationStartRetention == \A w \in P!Writers, i \in Inputs :
  P!Inv /\ P!Start(w,i) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Start, P!Inv, P!TypeOK, Inputs
USE DEF PublicationStartRetention
THEOREM PublicationAdmitRetention == \A w \in P!Writers :
  P!Inv /\ P!Admit(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Admit, P!Inv, P!TypeOK, Inputs
USE DEF PublicationAdmitRetention
THEOREM PublicationRetainIntentRetention == \A w \in P!Writers :
  P!Inv /\ P!RetainIntent(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!RetainIntent, P!Inv, P!TypeOK, Inputs
USE DEF PublicationRetainIntentRetention
THEOREM PublicationInputConflictRetention == \A w \in P!Writers :
  P!Inv /\ P!InputConflict(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!InputConflict, P!Inv, P!TypeOK, Inputs
USE DEF PublicationInputConflictRetention
THEOREM PublicationNativeCommitRetention == \A w \in P!Writers :
  P!Inv /\ P!NativeCommit(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!NativeCommit, P!Inv, P!TypeOK, Inputs
USE DEF PublicationNativeCommitRetention
THEOREM PublicationRetainManifestRetention == \A w \in P!Writers :
  P!Inv /\ P!RetainManifest(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!RetainManifest, P!Inv, P!TypeOK, Inputs
USE DEF PublicationRetainManifestRetention
THEOREM PublicationFinalRevalidateRetention == \A w \in P!Writers :
  P!Inv /\ P!FinalRevalidate(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!FinalRevalidate, P!Inv, P!TypeOK, Inputs
USE DEF PublicationFinalRevalidateRetention
THEOREM PublicationDoCASRetention == \A w \in P!Writers :
  P!Inv /\ P!DoCAS(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!DoCAS, P!CASCore, P!Inv, P!TypeOK, Inputs
USE DEF PublicationDoCASRetention
THEOREM PublicationCASMismatchRetention == \A w \in P!Writers :
  P!Inv /\ P!CASMismatch(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!CASMismatch, P!Inv, P!TypeOK, Inputs
USE DEF PublicationCASMismatchRetention
THEOREM PublicationRecordReceiptRetention == \A i \in Inputs :
  P!Inv /\ P!RecordReceipt(i) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!RecordReceipt, P!Inv, P!TypeOK, Inputs
USE DEF PublicationRecordReceiptRetention
THEOREM PublicationFinishRetention == \A w \in P!Writers :
  P!Inv /\ P!Finish(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Finish, P!Inv, P!TypeOK, Inputs
USE DEF PublicationFinishRetention
THEOREM PublicationRejectAdmissionRetention == \A w \in P!Writers :
  P!Inv /\ P!RejectAdmission(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!RejectAdmission, P!Inv, P!TypeOK, Inputs
USE DEF PublicationRejectAdmissionRetention
THEOREM PublicationFailIORetention == \A w \in P!Writers :
  P!Inv /\ P!FailIO(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!FailIO, P!Inv, P!TypeOK, Inputs
USE DEF PublicationFailIORetention
THEOREM PublicationLostResponseRetention == \A w \in P!Writers :
  P!Inv /\ P!LostResponse(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!LostResponse, P!Inv, P!TypeOK, Inputs
USE DEF PublicationLostResponseRetention
THEOREM PublicationCrashRetention == \A w \in P!Writers :
  P!Inv /\ P!Crash(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Crash, P!Inv, P!TypeOK, Inputs
USE DEF PublicationCrashRetention
THEOREM PublicationRestartRetention == \A w \in P!Writers :
  P!Inv /\ P!Restart(w) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Restart, P!Inv, P!TypeOK, Inputs
USE DEF PublicationRestartRetention
THEOREM PublicationObserveRetention == \A i \in Inputs, a \in SUBSET P!FullChecks :
  P!Inv /\ P!Observe(i,a) =>
    p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
BY SMT DEF P!Observe, P!Inv, P!TypeOK, Inputs
USE DEF PublicationObserveRetention
THEOREM LocalPublicationRetention == P!Inv /\ LocalPublication =>
  /\ p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
  /\ UNCHANGED q
BY SMT, PublicationStartRetention, PublicationAdmitRetention, PublicationRetainIntentRetention, PublicationInputConflictRetention, PublicationNativeCommitRetention, PublicationRetainManifestRetention, PublicationFinalRevalidateRetention, PublicationDoCASRetention, PublicationCASMismatchRetention, PublicationRecordReceiptRetention, PublicationFinishRetention, PublicationRejectAdmissionRetention, PublicationFailIORetention, PublicationLostResponseRetention, PublicationCrashRetention, PublicationRestartRetention, PublicationObserveRetention
  DEF LocalPublication, P!Next, P!NonObservationNext, P!Stutter, Inputs, PublicationStartRetention, PublicationAdmitRetention, PublicationRetainIntentRetention, PublicationInputConflictRetention, PublicationNativeCommitRetention, PublicationRetainManifestRetention, PublicationFinalRevalidateRetention, PublicationDoCASRetention, PublicationCASMismatchRetention, PublicationRecordReceiptRetention, PublicationFinishRetention, PublicationRejectAdmissionRetention, PublicationFailIORetention, PublicationLostResponseRetention, PublicationCrashRetention, PublicationRestartRetention, PublicationObserveRetention
USE DEF LocalPublicationRetention

THEOREM InitPreserves == Init => Inv
<1>1. ASSUME Init PROVE Inv
  <2> USE Init
  <2>1. P!Inv BY SMT, <1>1, P!InitPreserves DEF Init, P!InitPreserves
  <2>2. RemoteTypeOK
    BY SMT, <1>1, RemoteDomainFacts
      DEF Init, RemoteInit, RemoteTypeOK, Phases, Results, Errors, RemoteDomainFacts
  <2>3. TransferRoots /\ ReceiverSound
    BY SMT, <1>1 DEF Init, RemoteInit, P!Init, TransferRoots,
      ReceiverSound, Closure, P!Closure
  <2>4. AdvertisementChain /\ ClientSound
    BY SMT, <1>1 DEF Init, RemoteInit, AdvertisementChain, ClientSound
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF InitPreserves

THEOREM LocalPublicationPreserves == Inv /\ LocalPublication => Inv'
<1>1. ASSUME Inv, LocalPublication PROVE Inv'
  <2> USE Inv, LocalPublication
  <2>1. P!Inv' BY SMT, <1>1, P!NextPreserves DEF Inv, LocalPublication, P!NextPreserves
  <2>2. RemoteTypeOK'
    BY SMT, <1>1 DEF LocalPublication, Inv, TypeOK, RemoteTypeOK
  <2>3. TransferRoots' /\ ReceiverSound'
    BY SMT, <1>1, LocalPublicationRetention
      DEF Inv, TransferRoots, ReceiverSound, LocalPublication, LocalPublicationRetention
  <2>4. AdvertisementChain' /\ ClientSound'
    BY SMT, <1>1 DEF Inv, AdvertisementChain, ClientSound, LocalPublication
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF LocalPublicationPreserves

(* Each component is separately preserved. In particular, the advertisement
   argument uses the already authenticated local chain rather than assuming
   that received objects or a client acknowledgment establish ancestry. *)
THEOREM StartTransferPreserves == \A c \in Clients, i \in Releases :
  Inv /\ StartTransfer(c,i) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, Inv, StartTransfer(c,i) PROVE Inv'
  <2> USE Inv, StartTransfer(c,i)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, StartTransfer, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, StartTransfer,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, StartTransfer
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, StartTransfer
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, StartTransfer
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, StartTransfer
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF StartTransferPreserves
THEOREM TransferGitPreserves == \A c \in Clients :
  Inv /\ TransferGit(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, TransferGit(c) PROVE Inv'
  <2> USE Inv, TransferGit(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, TransferGit, TransferObject, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, TransferGit, TransferObject,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, TransferGit, TransferObject
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, TransferGit, TransferObject
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, TransferGit, TransferObject
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, TransferGit, TransferObject
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF TransferGitPreserves
THEOREM TransferNativePreserves == \A c \in Clients :
  Inv /\ TransferNative(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, TransferNative(c) PROVE Inv'
  <2> USE Inv, TransferNative(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, TransferNative, TransferObject, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, TransferNative, TransferObject,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, TransferNative, TransferObject
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, TransferNative, TransferObject
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, TransferNative, TransferObject
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, TransferNative, TransferObject
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF TransferNativePreserves
THEOREM TransferCapturePreserves == \A c \in Clients :
  Inv /\ TransferCapture(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, TransferCapture(c) PROVE Inv'
  <2> USE Inv, TransferCapture(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, TransferCapture, TransferObject, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, TransferCapture, TransferObject,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, TransferCapture, TransferObject
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, TransferCapture, TransferObject
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, TransferCapture, TransferObject
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, TransferCapture, TransferObject
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF TransferCapturePreserves
THEOREM ReceiverVerifyPreserves == \A c \in Clients :
  Inv /\ ReceiverVerify(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, ReceiverVerify(c) PROVE Inv'
  <2> USE Inv, ReceiverVerify(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, ReceiverVerify, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, ReceiverVerify,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, ReceiverVerify
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, ReceiverVerify
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, ReceiverVerify
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, ReceiverVerify
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF ReceiverVerifyPreserves
THEOREM AcknowledgePreserves == \A c \in Clients :
  Inv /\ Acknowledge(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, Acknowledge(c) PROVE Inv'
  <2> USE Inv, Acknowledge(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, Acknowledge, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Acknowledge,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, Acknowledge
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, Acknowledge
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, Acknowledge
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, Acknowledge
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF AcknowledgePreserves
THEOREM ClientVerifyPreserves == \A c \in Clients :
  Inv /\ ClientVerify(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, ClientVerify(c) PROVE Inv'
  <2> USE Inv, ClientVerify(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, ClientVerify, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, ClientVerify,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, ClientVerify
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, ClientVerify
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, ClientVerify
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, ClientVerify
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF ClientVerifyPreserves
THEOREM AdvertisePreserves == \A c \in Clients :
  Inv /\ Advertise(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, Advertise(c) PROVE Inv'
  <2> USE Inv, Advertise(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, Advertise, AdvertiseCore, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Advertise, AdvertiseCore,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, Advertise, AdvertiseCore
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, Advertise, AdvertiseCore
  <2>5. AdvertisementChain'
    <3>1. q.target[c] \in Inputs /\ BaseOf(q.target[c]) = q.advertised /\
          q.advertised < q.target[c] /\ Input(q.target[c]).schema = 3
      BY SMT, <1>1, RemoteDomainFacts
        DEF Advertise, AdvertiseCore, Inv, TypeOK, P!Inv, P!TypeOK,
          P!PublicationChain, Input, P!Input, BaseOf, Releases, RemoteDomainFacts
    <3>2. \A i \in q.history \cup {q.target[c]} : i <= q.target[c]
      BY SMT, <1>1, <3>1 DEF Inv, TypeOK, RemoteTypeOK,
        AdvertisementChain, Releases, P!Releases
    <3>3. q.history \cup {q.target[c]} \subseteq p.published
      BY SMT, <1>1 DEF Advertise, AdvertiseCore, Inv,
        AdvertisementChain, ReceiverSound
    <3>4. \A i,j \in (q.history \cup {q.target[c]}) \ {0} :
      BaseOf(i) = BaseOf(j) => i = j
      BY SMT, <1>1, <3>3 DEF Inv, P!Inv, P!PublicationChain, BaseOf
    <3>5. QED BY SMT, <1>1, <3>1, <3>2, <3>4
      DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, Advertise, AdvertiseCore
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, Advertise, AdvertiseCore
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF AdvertisePreserves
THEOREM PointerConflictPreserves == \A c \in Clients :
  Inv /\ PointerConflict(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, PointerConflict(c) PROVE Inv'
  <2> USE Inv, PointerConflict(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, PointerConflict, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, PointerConflict,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, PointerConflict
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, PointerConflict
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, PointerConflict
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, PointerConflict
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF PointerConflictPreserves
THEOREM BeginRestorePreserves == \A c \in Clients, i \in Releases :
  Inv /\ BeginRestore(c,i) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, Inv, BeginRestore(c,i) PROVE Inv'
  <2> USE Inv, BeginRestore(c,i)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, BeginRestore, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, BeginRestore,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, BeginRestore
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, BeginRestore
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, BeginRestore
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, BeginRestore, AdvertisementChain, ReceiverSound
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF BeginRestorePreserves
THEOREM StageObjectPreserves == \A c \in Clients, k \in Kinds :
  Inv /\ StageObject(c,k) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW k \in Kinds, Inv, StageObject(c,k) PROVE Inv'
  <2> USE Inv, StageObject(c,k)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, StageObject, Kinds, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, StageObject, Kinds,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, StageObject, Kinds
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, StageObject, Kinds
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, StageObject, Kinds
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, StageObject, Kinds
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF StageObjectPreserves
THEOREM StageVerifyPreserves == \A c \in Clients :
  Inv /\ StageVerify(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, StageVerify(c) PROVE Inv'
  <2> USE Inv, StageVerify(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, StageVerify, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, StageVerify,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, StageVerify
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, StageVerify
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, StageVerify
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, StageVerify
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF StageVerifyPreserves
THEOREM SelectRestorePreserves == \A c \in Clients :
  Inv /\ SelectRestore(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, SelectRestore(c) PROVE Inv'
  <2> USE Inv, SelectRestore(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, SelectRestore, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, SelectRestore,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, SelectRestore
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, SelectRestore
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, SelectRestore
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, SelectRestore
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF SelectRestorePreserves
THEOREM ReplicaReadPreserves == \A c \in Clients, i \in Releases : \A context \in ReadContexts(i) :
  Inv /\ ReplicaRead(c,i,context) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, NEW context \in ReadContexts(i), Inv, ReplicaRead(c,i,context) PROVE Inv'
  <2> USE Inv, ReplicaRead(c,i,context)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, ReplicaRead, ReadContexts, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, ReplicaRead, ReadContexts,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, ReplicaRead, ReadContexts
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, ReplicaRead, ReadContexts
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, ReplicaRead, ReadContexts
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, ReplicaRead, ReadContexts
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF ReplicaReadPreserves
THEOREM ReplicaReadFailurePreserves == \A c \in Clients, i \in Releases, e \in Errors :
  Inv /\ ReplicaReadFailure(c,i,e) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, NEW e \in Errors, Inv, ReplicaReadFailure(c,i,e) PROVE Inv'
  <2> USE Inv, ReplicaReadFailure(c,i,e)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, ReplicaReadFailure, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, ReplicaReadFailure,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, ReplicaReadFailure
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, ReplicaReadFailure
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, ReplicaReadFailure
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, ReplicaReadFailure
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF ReplicaReadFailurePreserves
THEOREM TypedFailurePreserves == \A c \in Clients, e \in Errors :
  Inv /\ TypedFailure(c,e) => Inv'
<1>1. ASSUME NEW c \in Clients, NEW e \in Errors, Inv, TypedFailure(c,e) PROVE Inv'
  <2> USE Inv, TypedFailure(c,e)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, TypedFailure, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, TypedFailure,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, TypedFailure
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, TypedFailure
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, TypedFailure
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, TypedFailure
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF TypedFailurePreserves
THEOREM NetworkInterruptPreserves == \A c \in Clients :
  Inv /\ NetworkInterrupt(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, NetworkInterrupt(c) PROVE Inv'
  <2> USE Inv, NetworkInterrupt(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, NetworkInterrupt, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, NetworkInterrupt,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, NetworkInterrupt
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, NetworkInterrupt
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, NetworkInterrupt
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, NetworkInterrupt
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF NetworkInterruptPreserves
THEOREM CrashPreserves == \A c \in Clients :
  Inv /\ Crash(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, Crash(c) PROVE Inv'
  <2> USE Inv, Crash(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, Crash, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Crash,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, Crash
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, Crash
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, Crash
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, Crash
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF CrashPreserves
THEOREM RestartPreserves == \A c \in Clients :
  Inv /\ Restart(c) => Inv'
<1>1. ASSUME NEW c \in Clients, Inv, Restart(c) PROVE Inv'
  <2> USE Inv, Restart(c)
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, Restart, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1, RemoteDomainFacts
      DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Restart,
        Phases, Results, Errors, Inputs, Releases, Objects, BaseOf, RemoteDomainFacts
  <2>3. TransferRoots'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, TransferRoots, Restart
  <2>4. ReceiverSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ReceiverSound, Restart
  <2>5. AdvertisementChain'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, AdvertisementChain, Restart
  <2>6. ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, ClientSound, Restart
  <2>7. QED BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF RestartPreserves
THEOREM SourceDisappearPreserves == Inv /\ SourceDisappear => Inv'
<1>1. ASSUME Inv, SourceDisappear PROVE Inv'
  <2> USE Inv, SourceDisappear
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, SourceDisappear, PublicationFrame
  <2>2. RemoteTypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, SourceDisappear
  <2>3. TransferRoots' /\ ReceiverSound' /\ AdvertisementChain' /\ ClientSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, SourceDisappear,
      TransferRoots, ReceiverSound, AdvertisementChain, ClientSound
  <2>4. QED BY <2>1, <2>2, <2>3 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF SourceDisappearPreserves
THEOREM StutterPreserves == Inv /\ Stutter => Inv'
<1>1. ASSUME Inv, Stutter PROVE Inv'
  <2> USE Inv, Stutter
  <2>1. P!Inv' BY SMT, <1>1, PublicationFrame DEF Inv, Stutter, vars, PublicationFrame
  <2>2. RemoteTypeOK' /\ TransferRoots' /\ ReceiverSound' /\ AdvertisementChain' /\ ClientSound'
    BY SMT, <1>1 DEF Stutter, vars, Inv, TypeOK, RemoteTypeOK,
      TransferRoots, ReceiverSound, AdvertisementChain, ClientSound
  <2>3. QED BY <2>1, <2>2 DEF Inv, TypeOK, P!Inv
<1>2. QED BY <1>1
USE DEF StutterPreserves
THEOREM NextPreserves == Inv /\ Next => Inv'
BY LocalPublicationPreserves, StartTransferPreserves, TransferGitPreserves, TransferNativePreserves, TransferCapturePreserves, ReceiverVerifyPreserves, AcknowledgePreserves, ClientVerifyPreserves, AdvertisePreserves, PointerConflictPreserves, BeginRestorePreserves, StageObjectPreserves, StageVerifyPreserves, SelectRestorePreserves, ReplicaReadPreserves, ReplicaReadFailurePreserves, TypedFailurePreserves, NetworkInterruptPreserves, CrashPreserves, RestartPreserves,
  SourceDisappearPreserves, StutterPreserves DEF Next, RemoteNext, LocalPublicationPreserves, StartTransferPreserves, TransferGitPreserves, TransferNativePreserves, TransferCapturePreserves, ReceiverVerifyPreserves, AcknowledgePreserves, ClientVerifyPreserves, AdvertisePreserves, PointerConflictPreserves, BeginRestorePreserves, StageObjectPreserves, StageVerifyPreserves, SelectRestorePreserves, ReplicaReadPreserves, ReplicaReadFailurePreserves, TypedFailurePreserves, NetworkInterruptPreserves, CrashPreserves, RestartPreserves, SourceDisappearPreserves, StutterPreserves
USE DEF NextPreserves
THEOREM InvImpliesSafety == Inv => Safety
<1>1. ASSUME Inv PROVE Safety
  <2> USE Inv
  <2>1. P!Safety BY SMT, <1>1, P!InvImpliesSafety DEF Inv, P!InvImpliesSafety
  <2>2. \A i \in q.history :
    /\ i \in p.published /\ Closure(i) \subseteq q.received
    /\ i \in q.acknowledged /\ i \in q.advertisementEvidence
    /\ (i # 0 => Input(i).schema = 3 /\ BaseOf(i) \in q.history)
    BY SMT, <1>1 DEF Inv, ReceiverSound, AdvertisementChain
  <2>3. \A i,j \in q.history \ {0} :
    Input(i).predecessor = Input(j).predecessor => ExactRecord(i) = ExactRecord(j)
    BY SMT, <1>1 DEF Inv, AdvertisementChain, Input, P!Input, BaseOf
  <2>4. \A c \in Clients :
    Closure(q.selected[c]) \subseteq q.destination /\ q.selected[c] \in q.stagedEvidence
    BY SMT, <1>1 DEF Inv, ClientSound
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4 DEF Safety, Inv, AdvertisementChain
<1>2. QED BY <1>1
USE DEF InvImpliesSafety
THEOREM FiniteInitialIncluded == FiniteInit => Init
BY DEF FiniteInit
THEOREM AuthorityProfileBound == AuthorityProfiles \subseteq SUBSET P!FullChecks
BY SMT DEF AuthorityProfiles
THEOREM FiniteActionsIncluded == FiniteNext => Next
BY SMT, AuthorityProfileBound DEF AuthorityProfileBound, FiniteNext, Next,
  LocalPublication, FiniteLocalNext, P!Next, P!NonObservationNext, Inputs

RemoteDurable == <<q.received,q.receiverVerified,q.acknowledged,q.advertised,
  q.history,q.advertisementEvidence,q.destination,q.restored,q.selected,q.stagedEvidence>>
THEOREM FailureFrames == \A c \in Clients, e \in Errors :
  TypeOK /\ (TypedFailure(c,e) \/ NetworkInterrupt(c) \/ Crash(c) \/ Restart(c)
   \/ PointerConflict(c)) => UNCHANGED <<p,RemoteDurable>>
BY SMT DEF TypeOK, RemoteTypeOK, TypedFailure, NetworkInterrupt, Crash, Restart, PointerConflict, RemoteDurable
THEOREM ReadOnlyAuthorityFrame == \A c \in Clients, i \in Releases, e \in Errors :
  \A context \in ReadContexts(i) :
  TypeOK /\ (ReplicaRead(c,i,context) \/ ReplicaReadFailure(c,i,e)) =>
  UNCHANGED <<p,RemoteDurable,q.stage,q.stageChecked,q.checked,q.target,q.pc>>
BY SMT DEF TypeOK, RemoteTypeOK, ReplicaRead, ReplicaReadFailure, RemoteDurable
THEOREM RestoreSourceIndependence == \A c \in Clients :
  Inv /\ ~q.sourceAvailable /\ SelectRestore(c) =>
    /\ Closure(q'.selected[c]) \subseteq q'.destination
    /\ q'.selected[c] \in q'.stagedEvidence /\ UNCHANGED p
BY SMT DEF SelectRestore, Inv, TypeOK, RemoteTypeOK, ClientSound
THEOREM ExactAdvertisementCAS == \A c \in Clients : Inv /\ Advertise(c) =>
  /\ q.advertised = Input(q.target[c]).predecessor
  /\ q'.advertised = q.target[c] /\ q.target[c] \in p.published
  /\ q.target[c] \in q.acknowledged /\ Closure(q.target[c]) \subseteq q.received
  /\ q.history \subseteq q'.history /\ UNCHANGED p
BY SMT DEF Advertise, AdvertiseCore, Inv, TypeOK, RemoteTypeOK, Input, BaseOf, P!Input
=============================================================================
