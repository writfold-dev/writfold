-------------------------- MODULE M01M05Bridge --------------------------
EXTENDS M05RemoteClosure

(***************************************************************************
This bridge shares the actual variable p with P == INSTANCE
M01ReleasePublication. It does not assume a separately maintained 'valid'
release set. Roots come from p.published, bytes from p.objects, and exact
operation/approval/context records from P!Input; same operation ID is NOT
identity. q.received contains transport-owned copies with those exact immutable
object identities. Rooting every received object in a completed local release
excludes pending intents/native commits/manifests despite their local retention.

The bridge is an interleaving contract: publication changes p and frames q;
transfer, verification, receiver acknowledgment, conditional advertisement,
restore and failure actions frame p. No unrelated Git/native atomic transaction
is presumed. New remote history retains exact schema-3 Input records. The
bootstrap root 0 separately retains actual p.legacy signed history, projected
by HistoricalProjection below, never reinterpreted as a new Input(0) plan.
Legacy presentation grants no writer capability. Crypto, retention and CAS
are the explicit M01/M05 trusted adapter premises, not conclusions about an
enterprise endpoint or a shipping Rust implementation. These source proofs
have not been run. No general temporal/refinement theorem is claimed.
***************************************************************************)
LocalRoots == p.published
LocalRecords == [i \in LocalRoots |-> P!Input(i)]
AdmissibleObjects == UNION {P!Closure(i) : i \in LocalRoots}
PublicationProjection == [head |-> p.head, roots |-> LocalRoots,
  objects |-> p.objects, inputs |-> LocalRecords]
TransportProjection == [objects |-> q.received, acknowledged |-> q.acknowledged,
  head |-> q.advertised, roots |-> q.history,
  inputs |-> [i \in q.history |-> Input(i)]]
HistoricalProjection == [input |-> p.legacy.input, manifest |-> p.legacy.manifest,
  receipt |-> p.legacy.receipt, authentication |-> p.legacy.authentication]
ExactRootAgreement ==
  /\ q.received \subseteq AdmissibleObjects \cap p.objects
  /\ q.history \subseteq LocalRoots
  /\ \A i \in q.history : TransportProjection.inputs[i] = LocalRecords[i]
  /\ \A i \in q.history \ {0} :
       /\ LocalRecords[i] = P!Input(p.intent[P!OpOf(i)])
       /\ LocalRecords[i].family = P!Input(i).family
       /\ LocalRecords[i].approvalBytes = P!Input(i).approvalBytes
       /\ LocalRecords[i].schema = 3 /\ p.evidence[i] = P!FullChecks
RemoteClosure == \A i \in q.history :
  P!Closure(i) \subseteq q.received /\ P!Closure(i) \subseteq p.objects
BridgeInv == Inv /\ ExactRootAgreement /\ RemoteClosure
BridgeSafety == Safety /\ ExactRootAgreement /\ RemoteClosure
BridgeInit == Init
BridgeNext == Next
BridgeSpec == BridgeInit /\ [][BridgeNext]_vars
FiniteBridgeInit == FiniteInit
FiniteBridgeNext == FiniteNext

(* Two-state obligations expose durable order and the publication/ref boundary;
   these are stronger than repeating state invariant theorem names. *)
BridgeEffects ==
  /\ p.published \subseteq p'.published /\ p.objects \subseteq p'.objects
  /\ q.received \subseteq q'.received /\ q.history \subseteq q'.history
  /\ q.acknowledged \subseteq q'.acknowledged
  /\ q'.received \ q.received \subseteq AdmissibleObjects \cap p.objects
  /\ q'.acknowledged \ q.acknowledged \subseteq q.receiverVerified
  /\ q'.history \ q.history \subseteq q.acknowledged
  /\ (q'.advertised # q.advertised =>
       /\ q.advertised = P!Input(q'.advertised).predecessor
       /\ q'.advertised \in p.published /\ q'.advertised \in q.acknowledged
       /\ P!Closure(q'.advertised) \subseteq q.received
       /\ \E c \in Clients : q.checked[c] /\ q.target[c] = q'.advertised)
  /\ (p' # p => UNCHANGED q)
  /\ (q' # q => UNCHANGED p)

(* Explicitly expose referenced theorem statements; preserve all original
   bridge state/effect contracts and every unrestricted transition. *)
USE DEF InitPreserves, LocalPublicationPreserves, LocalPublicationRetention,
      NextPreserves, InvImpliesSafety, FailureFrames, P!CASLinearization

THEOREM RootedObjectSets == \A S \in SUBSET Releases, O \in SUBSET Objects :
  (\A o \in O : o[2] \in S) => O \subseteq UNION {P!Closure(i) : i \in S}
<1>1. ASSUME NEW S \in SUBSET Releases, NEW O \in SUBSET Objects,
            \A o \in O : o[2] \in S
      PROVE O \subseteq UNION {P!Closure(i) : i \in S}
  <2> USE (\A o \in O : o[2] \in S)
  <2>1. ASSUME NEW o \in O PROVE o \in UNION {P!Closure(i) : i \in S}
    <3>1. o[2] \in S /\ o \in P!Closure(o[2])
      BY SMT, <1>1, <2>1 DEF Objects, P!Objects, P!Closure
    <3>2. QED BY SMT, <3>1
  <2>2. QED BY <2>1
<1>2. QED BY <1>1
USE DEF RootedObjectSets

THEOREM RootsFromActualPublication == Inv => ExactRootAgreement /\ RemoteClosure
<1>1. ASSUME Inv PROVE ExactRootAgreement /\ RemoteClosure
  <2> USE Inv
  <2>1. q.received \subseteq AdmissibleObjects \cap p.objects
    <3>1. p.published \in SUBSET Releases /\ q.received \in SUBSET Objects
      BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Releases, Objects
    <3>2. \A o \in q.received : o[2] \in p.published
      BY SMT, <1>1 DEF Inv, TransferRoots
    <3>3. q.received \subseteq AdmissibleObjects
      BY SMT, <3>1, <3>2, RootedObjectSets
        DEF RootedObjectSets, AdmissibleObjects, LocalRoots
    <3>4. QED BY SMT, <1>1, <3>3 DEF Inv, TransferRoots
  <2>2. q.history \subseteq LocalRoots
    BY SMT, <1>1 DEF Inv, AdvertisementChain, ReceiverSound, LocalRoots
  <2>3. \A i \in q.history : TransportProjection.inputs[i] = LocalRecords[i]
    BY SMT, <2>2 DEF TransportProjection, LocalRecords, Input
  <2>4. \A i \in q.history \ {0} :
    /\ LocalRecords[i] = P!Input(p.intent[P!OpOf(i)])
    /\ LocalRecords[i].family = P!Input(i).family
    /\ LocalRecords[i].approvalBytes = P!Input(i).approvalBytes
    /\ LocalRecords[i].schema = 3 /\ p.evidence[i] = P!FullChecks
    <3>1. \A i \in q.history \ {0} :
      p.intent[P!OpOf(i)] = i /\ P!SchemaOf(i) = 3 /\ p.evidence[i] = P!FullChecks
      BY SMT, <1>1, <2>2 DEF Inv, P!Inv, P!PublicationChain,
        P!Prepared, P!BoundIntent, LocalRoots
    <3>2. \A i \in q.history : LocalRecords[i] = P!Input(i)
      BY SMT, <2>2 DEF LocalRecords
    <3>3. QED BY SMT, <3>1, <3>2 DEF P!Input
  <2>5. RemoteClosure
    BY SMT, <1>1 DEF Inv, TransferRoots, ReceiverSound,
      AdvertisementChain, RemoteClosure, Closure
  <2>6. QED BY <2>1, <2>2, <2>3, <2>4, <2>5 DEF ExactRootAgreement
<1>2. QED BY <1>1
USE DEF RootsFromActualPublication

(* A separate post-state lemma exposes primed records to first-order SMT.
   It is proved directly, not inferred from an opaque unprimed theorem name. *)
THEOREM PostStateBridgeLift == Inv' => BridgeInv'
<1>1. ASSUME Inv' PROVE BridgeInv'
  <2> USE Inv'
  <2>1. q'.received \subseteq AdmissibleObjects' \cap p'.objects
    <3>1. p'.published \in SUBSET Releases /\ q'.received \in SUBSET Objects
      BY SMT, <1>1 DEF Inv, TypeOK, RemoteTypeOK, P!TypeOK, Releases, Objects
    <3>2. \A o \in q'.received : o[2] \in p'.published
      BY SMT, <1>1 DEF Inv, TransferRoots
    <3>3. q'.received \subseteq AdmissibleObjects'
      BY SMT, <3>1, <3>2, RootedObjectSets
        DEF RootedObjectSets, AdmissibleObjects, LocalRoots
    <3>4. QED BY SMT, <1>1, <3>3 DEF Inv, TransferRoots
  <2>2. q'.history \subseteq LocalRoots'
    BY SMT, <1>1 DEF Inv, AdvertisementChain, ReceiverSound, LocalRoots
  <2>3. \A i \in q'.history : (TransportProjection').inputs[i] = (LocalRecords')[i]
    BY SMT, <2>2 DEF TransportProjection, LocalRecords, Input
  <2>4. \A i \in q'.history \ {0} :
    /\ (LocalRecords')[i] = P!Input(p'.intent[P!OpOf(i)])
    /\ (LocalRecords')[i].family = P!Input(i).family
    /\ (LocalRecords')[i].approvalBytes = P!Input(i).approvalBytes
    /\ (LocalRecords')[i].schema = 3 /\ p'.evidence[i] = P!FullChecks
    <3>1. \A i \in q'.history \ {0} :
      p'.intent[P!OpOf(i)] = i /\ P!SchemaOf(i) = 3 /\ p'.evidence[i] = P!FullChecks
      BY SMT, <1>1, <2>2 DEF Inv, P!Inv, P!PublicationChain,
        P!Prepared, P!BoundIntent, LocalRoots
    <3>2. \A i \in q'.history : (LocalRecords')[i] = P!Input(i)
      BY SMT, <2>2 DEF LocalRecords
    <3>3. QED BY SMT, <3>1, <3>2 DEF P!Input
  <2>5. RemoteClosure'
    BY SMT, <1>1 DEF Inv, TransferRoots, ReceiverSound,
      AdvertisementChain, RemoteClosure, Closure
  <2>6. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, <2>5
    DEF BridgeInv, ExactRootAgreement
<1>2. QED BY <1>1
USE DEF PostStateBridgeLift

THEOREM BridgeInitPreserves == BridgeInit => BridgeInv
BY SMT, InitPreserves, RootsFromActualPublication DEF BridgeInit, BridgeInv, InitPreserves, RootsFromActualPublication
USE DEF BridgeInitPreserves
THEOREM PublicationEffectContract ==
  p.published \subseteq p'.published /\ p.objects \subseteq p'.objects /\
  UNCHANGED q => BridgeEffects
BY SMT DEF BridgeEffects
THEOREM PublicationPreservesBridge == BridgeInv /\ LocalPublication =>
  /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED TransportProjection
<1>1. ASSUME BridgeInv, LocalPublication
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED TransportProjection
  <2> USE BridgeInv, LocalPublication
  <2>1. Inv' BY SMT, <1>1, LocalPublicationPreserves DEF BridgeInv, LocalPublicationPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. p.published \subseteq p'.published /\ p.objects \subseteq p'.objects /\
      UNCHANGED q
      BY <1>1, LocalPublicationRetention DEF BridgeInv, Inv, LocalPublicationRetention
    <3>2. QED BY <3>1, PublicationEffectContract DEF PublicationEffectContract
  <2>4. UNCHANGED TransportProjection
    BY SMT, <1>1 DEF LocalPublication, TransportProjection
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF PublicationPreservesBridge

(* These contracts expose only the fields relevant to transport ordering.
   Action-specific record updates are proved separately from set/ancestry
   reasoning, preventing unrelated prior theorem contexts from dominating SMT. *)
THEOREM FramedRemoteEffects ==
  UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>> => BridgeEffects
BY SMT DEF BridgeEffects
THEOREM TransferEffectContract == \A o \in Objects :
  (UNCHANGED <<p,q.acknowledged,q.history,q.advertised>> /\
   q'.received = q.received \cup {o} /\ o \in AdmissibleObjects \cap p.objects)
  => BridgeEffects
BY SMT DEF BridgeEffects
THEOREM AcknowledgmentEffectContract == \A i \in Releases :
  (UNCHANGED <<p,q.received,q.history,q.advertised>> /\
   q'.acknowledged = q.acknowledged \cup {i} /\ i \in q.receiverVerified)
  => BridgeEffects
BY SMT DEF BridgeEffects
THEOREM AdvertisementEffectContract == \A c \in Clients, i \in Releases :
  (UNCHANGED <<p,q.received,q.acknowledged>> /\
   q'.history = q.history \cup {i} /\ q'.advertised = i /\
   q.advertised = P!Input(i).predecessor /\ i \in p.published /\
   i \in q.acknowledged /\ P!Closure(i) \subseteq q.received /\
   q.checked[c] /\ q.target[c] = i) => BridgeEffects
BY SMT DEF BridgeEffects

USE DEF StartTransferPreserves
THEOREM StartTransferBridgePreserves == \A c \in Clients, i \in Releases :
  BridgeInv /\ StartTransfer(c,i) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, BridgeInv, StartTransfer(c,i)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, StartTransfer(c,i)
  <2>1. Inv' BY SMT, <1>1, StartTransferPreserves DEF BridgeInv, StartTransferPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, StartTransfer
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF StartTransfer, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF StartTransferBridgePreserves
USE DEF TransferGitPreserves
THEOREM TransferGitBridgePreserves == \A c \in Clients :
  BridgeInv /\ TransferGit(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, TransferGit(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, TransferGit(c)
  <2>1. Inv' BY SMT, <1>1, TransferGitPreserves DEF BridgeInv, TransferGitPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferGit, TransferObject
    <3>2. q'.received = q.received \cup {<<"git",q.target[c]>>}
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferGit, TransferObject
    <3>3. <<"git",q.target[c]>> \in AdmissibleObjects \cap p.objects
      BY SMT, <1>1 DEF TransferGit, TransferObject, AdmissibleObjects, LocalRoots, P!Closure
    <3>4. <<"git",q.target[c]>> \in Objects
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, P!TypeOK, TransferGit, TransferObject, Objects
    <3>5. QED BY SMT, <3>1, <3>2, <3>3, <3>4, TransferEffectContract
      DEF TransferEffectContract
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF TransferGit, TransferObject, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF TransferGitBridgePreserves
USE DEF TransferNativePreserves
THEOREM TransferNativeBridgePreserves == \A c \in Clients :
  BridgeInv /\ TransferNative(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, TransferNative(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, TransferNative(c)
  <2>1. Inv' BY SMT, <1>1, TransferNativePreserves DEF BridgeInv, TransferNativePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferNative, TransferObject
    <3>2. q'.received = q.received \cup {<<"native",q.target[c]>>}
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferNative, TransferObject
    <3>3. <<"native",q.target[c]>> \in AdmissibleObjects \cap p.objects
      BY SMT, <1>1 DEF TransferNative, TransferObject, AdmissibleObjects, LocalRoots, P!Closure
    <3>4. <<"native",q.target[c]>> \in Objects
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, P!TypeOK, TransferNative, TransferObject, Objects
    <3>5. QED BY SMT, <3>1, <3>2, <3>3, <3>4, TransferEffectContract
      DEF TransferEffectContract
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF TransferNative, TransferObject, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF TransferNativeBridgePreserves
USE DEF TransferCapturePreserves
THEOREM TransferCaptureBridgePreserves == \A c \in Clients :
  BridgeInv /\ TransferCapture(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, TransferCapture(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, TransferCapture(c)
  <2>1. Inv' BY SMT, <1>1, TransferCapturePreserves DEF BridgeInv, TransferCapturePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferCapture, TransferObject
    <3>2. q'.received = q.received \cup {<<"capture",q.target[c]>>}
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TransferCapture, TransferObject
    <3>3. <<"capture",q.target[c]>> \in AdmissibleObjects \cap p.objects
      BY SMT, <1>1 DEF TransferCapture, TransferObject, AdmissibleObjects, LocalRoots, P!Closure
    <3>4. <<"capture",q.target[c]>> \in Objects
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, P!TypeOK, TransferCapture, TransferObject, Objects
    <3>5. QED BY SMT, <3>1, <3>2, <3>3, <3>4, TransferEffectContract
      DEF TransferEffectContract
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF TransferCapture, TransferObject, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF TransferCaptureBridgePreserves
USE DEF ReceiverVerifyPreserves
THEOREM ReceiverVerifyBridgePreserves == \A c \in Clients :
  BridgeInv /\ ReceiverVerify(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, ReceiverVerify(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, ReceiverVerify(c)
  <2>1. Inv' BY SMT, <1>1, ReceiverVerifyPreserves DEF BridgeInv, ReceiverVerifyPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, ReceiverVerify
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF ReceiverVerify, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF ReceiverVerifyBridgePreserves
USE DEF AcknowledgePreserves
THEOREM AcknowledgeBridgePreserves == \A c \in Clients :
  BridgeInv /\ Acknowledge(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, Acknowledge(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, Acknowledge(c)
  <2>1. Inv' BY SMT, <1>1, AcknowledgePreserves DEF BridgeInv, AcknowledgePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Acknowledge
    <3>2. q'.acknowledged = q.acknowledged \cup {q.target[c]}
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Acknowledge
    <3>3. q.target[c] \in Releases /\ q.target[c] \in q.receiverVerified
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Acknowledge
    <3>4. QED BY SMT, <3>1, <3>2, <3>3, AcknowledgmentEffectContract
      DEF AcknowledgmentEffectContract
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF Acknowledge, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF AcknowledgeBridgePreserves
USE DEF ClientVerifyPreserves
THEOREM ClientVerifyBridgePreserves == \A c \in Clients :
  BridgeInv /\ ClientVerify(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, ClientVerify(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, ClientVerify(c)
  <2>1. Inv' BY SMT, <1>1, ClientVerifyPreserves DEF BridgeInv, ClientVerifyPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, ClientVerify
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF ClientVerify, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF ClientVerifyBridgePreserves
USE DEF AdvertisePreserves
THEOREM AdvertiseBridgePreserves == \A c \in Clients :
  BridgeInv /\ Advertise(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, Advertise(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, Advertise(c)
  <2>1. Inv' BY SMT, <1>1, AdvertisePreserves DEF BridgeInv, AdvertisePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Advertise, AdvertiseCore
    <3>2. q'.history = q.history \cup {q.target[c]} /\ q'.advertised = q.target[c]
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Advertise, AdvertiseCore
    <3>3. q.target[c] \in Releases /\ q.checked[c]
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Advertise, AdvertiseCore
    <3>4. q.advertised = P!Input(q.target[c]).predecessor /\
      q.target[c] \in p.published /\ q.target[c] \in q.acknowledged /\
      P!Closure(q.target[c]) \subseteq q.received
      BY SMT, <1>1 DEF Advertise, AdvertiseCore, BaseOf, P!Input, Closure
    <3>5. QED BY SMT, <3>1, <3>2, <3>3, <3>4, AdvertisementEffectContract
      DEF AdvertisementEffectContract
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF Advertise, AdvertiseCore, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF AdvertiseBridgePreserves
USE DEF PointerConflictPreserves
THEOREM PointerConflictBridgePreserves == \A c \in Clients :
  BridgeInv /\ PointerConflict(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, PointerConflict(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, PointerConflict(c)
  <2>1. Inv' BY SMT, <1>1, PointerConflictPreserves DEF BridgeInv, PointerConflictPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, PointerConflict
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF PointerConflict, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF PointerConflictBridgePreserves
USE DEF BeginRestorePreserves
THEOREM BeginRestoreBridgePreserves == \A c \in Clients, i \in Releases :
  BridgeInv /\ BeginRestore(c,i) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, BridgeInv, BeginRestore(c,i)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, BeginRestore(c,i)
  <2>1. Inv' BY SMT, <1>1, BeginRestorePreserves DEF BridgeInv, BeginRestorePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, BeginRestore
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF BeginRestore, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF BeginRestoreBridgePreserves
USE DEF StageObjectPreserves
THEOREM StageObjectBridgePreserves == \A c \in Clients, k \in Kinds :
  BridgeInv /\ StageObject(c,k) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW k \in Kinds, BridgeInv, StageObject(c,k)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, StageObject(c,k)
  <2>1. Inv' BY SMT, <1>1, StageObjectPreserves DEF BridgeInv, StageObjectPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, StageObject, Kinds
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF StageObject, Kinds, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF StageObjectBridgePreserves
USE DEF StageVerifyPreserves
THEOREM StageVerifyBridgePreserves == \A c \in Clients :
  BridgeInv /\ StageVerify(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, StageVerify(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, StageVerify(c)
  <2>1. Inv' BY SMT, <1>1, StageVerifyPreserves DEF BridgeInv, StageVerifyPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, StageVerify
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF StageVerify, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF StageVerifyBridgePreserves
USE DEF SelectRestorePreserves
THEOREM SelectRestoreBridgePreserves == \A c \in Clients :
  BridgeInv /\ SelectRestore(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, SelectRestore(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, SelectRestore(c)
  <2>1. Inv' BY SMT, <1>1, SelectRestorePreserves DEF BridgeInv, SelectRestorePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, SelectRestore
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF SelectRestore, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF SelectRestoreBridgePreserves
USE DEF ReplicaReadPreserves
THEOREM ReplicaReadBridgePreserves == \A c \in Clients, i \in Releases : \A context \in ReadContexts(i) :
  BridgeInv /\ ReplicaRead(c,i,context) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, NEW context \in ReadContexts(i), BridgeInv, ReplicaRead(c,i,context)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, ReplicaRead(c,i,context)
  <2>1. Inv' BY SMT, <1>1, ReplicaReadPreserves DEF BridgeInv, ReplicaReadPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, ReplicaRead, ReadContexts
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF ReplicaRead, ReadContexts, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF ReplicaReadBridgePreserves
USE DEF ReplicaReadFailurePreserves
THEOREM ReplicaReadFailureBridgePreserves == \A c \in Clients, i \in Releases, e \in Errors :
  BridgeInv /\ ReplicaReadFailure(c,i,e) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW i \in Releases, NEW e \in Errors, BridgeInv, ReplicaReadFailure(c,i,e)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, ReplicaReadFailure(c,i,e)
  <2>1. Inv' BY SMT, <1>1, ReplicaReadFailurePreserves DEF BridgeInv, ReplicaReadFailurePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, ReplicaReadFailure
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF ReplicaReadFailure, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF ReplicaReadFailureBridgePreserves
USE DEF TypedFailurePreserves
THEOREM TypedFailureBridgePreserves == \A c \in Clients, e \in Errors :
  BridgeInv /\ TypedFailure(c,e) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, NEW e \in Errors, BridgeInv, TypedFailure(c,e)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, TypedFailure(c,e)
  <2>1. Inv' BY SMT, <1>1, TypedFailurePreserves DEF BridgeInv, TypedFailurePreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, TypedFailure
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF TypedFailure, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF TypedFailureBridgePreserves
USE DEF NetworkInterruptPreserves
THEOREM NetworkInterruptBridgePreserves == \A c \in Clients :
  BridgeInv /\ NetworkInterrupt(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, NetworkInterrupt(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, NetworkInterrupt(c)
  <2>1. Inv' BY SMT, <1>1, NetworkInterruptPreserves DEF BridgeInv, NetworkInterruptPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, NetworkInterrupt
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF NetworkInterrupt, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF NetworkInterruptBridgePreserves
USE DEF CrashPreserves
THEOREM CrashBridgePreserves == \A c \in Clients :
  BridgeInv /\ Crash(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, Crash(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, Crash(c)
  <2>1. Inv' BY SMT, <1>1, CrashPreserves DEF BridgeInv, CrashPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Crash
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF Crash, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF CrashBridgePreserves
USE DEF RestartPreserves
THEOREM RestartBridgePreserves == \A c \in Clients :
  BridgeInv /\ Restart(c) =>
    /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, Restart(c)
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, Restart(c)
  <2>1. Inv' BY SMT, <1>1, RestartPreserves DEF BridgeInv, RestartPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Restart
    <3>2. QED BY SMT, <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF Restart, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF RestartBridgePreserves
USE DEF SourceDisappearPreserves, StutterPreserves
THEOREM SourceDisappearBridgePreserves == BridgeInv /\ SourceDisappear =>
  /\ BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
<1>1. ASSUME BridgeInv, SourceDisappear
      PROVE BridgeInv' /\ BridgeEffects /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, SourceDisappear
  <2>1. Inv' BY SMT, <1>1, SourceDisappearPreserves DEF BridgeInv, SourceDisappearPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    <3>1. UNCHANGED <<p,q.received,q.acknowledged,q.history,q.advertised>>
      BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, SourceDisappear
    <3>2. QED BY <3>1, FramedRemoteEffects DEF FramedRemoteEffects
  <2>4. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF SourceDisappear, PublicationProjection, LocalRoots, LocalRecords
  <2>5. QED BY <2>2, <2>3, <2>4
<1>2. QED BY <1>1
USE DEF SourceDisappearBridgePreserves
THEOREM StutterBridgePreserves == BridgeInv /\ Stutter => BridgeInv' /\ BridgeEffects
<1>1. ASSUME BridgeInv, Stutter PROVE BridgeInv' /\ BridgeEffects
  <2> USE BridgeInv, Stutter
  <2>1. Inv' BY SMT, <1>1, StutterPreserves DEF BridgeInv, StutterPreserves
  <2>2. BridgeInv' BY SMT, <2>1, PostStateBridgeLift DEF PostStateBridgeLift
  <2>3. BridgeEffects
    BY <1>1, FramedRemoteEffects DEF FramedRemoteEffects, Stutter, vars
  <2>4. QED BY <2>2, <2>3
<1>2. QED BY <1>1
USE DEF StutterBridgePreserves
THEOREM BridgeNextPreserves == BridgeInv /\ BridgeNext => BridgeInv' /\ BridgeEffects
BY PublicationPreservesBridge, StartTransferBridgePreserves, TransferGitBridgePreserves, TransferNativeBridgePreserves, TransferCaptureBridgePreserves, ReceiverVerifyBridgePreserves, AcknowledgeBridgePreserves, ClientVerifyBridgePreserves, AdvertiseBridgePreserves, PointerConflictBridgePreserves, BeginRestoreBridgePreserves, StageObjectBridgePreserves, StageVerifyBridgePreserves, SelectRestoreBridgePreserves, ReplicaReadBridgePreserves, ReplicaReadFailureBridgePreserves, TypedFailureBridgePreserves, NetworkInterruptBridgePreserves, CrashBridgePreserves, RestartBridgePreserves,
  SourceDisappearBridgePreserves, StutterBridgePreserves
  DEF BridgeNext, Next, RemoteNext, PublicationPreservesBridge, StartTransferBridgePreserves, TransferGitBridgePreserves, TransferNativeBridgePreserves, TransferCaptureBridgePreserves, ReceiverVerifyBridgePreserves, AcknowledgeBridgePreserves, ClientVerifyBridgePreserves, AdvertiseBridgePreserves, PointerConflictBridgePreserves, BeginRestoreBridgePreserves, StageObjectBridgePreserves, StageVerifyBridgePreserves, SelectRestoreBridgePreserves, ReplicaReadBridgePreserves, ReplicaReadFailureBridgePreserves, TypedFailureBridgePreserves, NetworkInterruptBridgePreserves, CrashBridgePreserves, RestartBridgePreserves, SourceDisappearBridgePreserves, StutterBridgePreserves
USE DEF BridgeNextPreserves
THEOREM BridgeInvImpliesSafety == BridgeInv => BridgeSafety
BY SMT, InvImpliesSafety DEF BridgeInv, BridgeSafety, InvImpliesSafety
THEOREM PublicationCASDoesNotAdvertise == \A w \in P!Writers :
  BridgeInv /\ P!DoCAS(w) /\ UNCHANGED q =>
    /\ p'.head = p.request[w]
    /\ p.head = P!Input(p.request[w]).predecessor
    /\ UNCHANGED TransportProjection
<1>1. \A w \in P!Writers : BridgeInv /\ P!DoCAS(w) =>
  p'.head = p.request[w] /\ p.head = P!Input(p.request[w]).predecessor
  <2>1. \A w \in P!Writers : BridgeInv /\ P!DoCAS(w) =>
    p'.head=p.request[w]
    BY SMT DEF BridgeInv, Inv, TypeOK, P!TypeOK, P!DoCAS, P!CASCore
  <2>2. \A w \in P!Writers : P!DoCAS(w) =>
    p.head=P!Input(p.request[w]).predecessor
    BY SMT DEF P!DoCAS, P!CASCore, P!Input
  <2>3. QED BY <2>1, <2>2
<1>2. UNCHANGED q => UNCHANGED TransportProjection
  BY DEF TransportProjection
<1>3. QED BY <1>1, <1>2
THEOREM TransferUsesCompletedExactRoot == \A c \in Clients, k \in Kinds :
  BridgeInv /\ TransferObject(c,k) =>
    /\ q.target[c] \in PublicationProjection.roots
    /\ <<k,q.target[c]>> \in PublicationProjection.objects
    /\ PublicationProjection.inputs[q.target[c]] = P!Input(q.target[c])
    /\ UNCHANGED PublicationProjection
BY SMT DEF TransferObject, PublicationProjection, LocalRoots, LocalRecords
THEOREM AdvertisementUsesExactPublishedApproval == \A c \in Clients :
  BridgeInv /\ Advertise(c) =>
    /\ P!Input(q'.advertised) = P!Input(p.intent[P!OpOf(q'.advertised)])
    /\ P!Input(q'.advertised).family = LocalRecords[q'.advertised].family
    /\ P!Input(q'.advertised).approvalBytes = LocalRecords[q'.advertised].approvalBytes
    /\ P!Closure(q'.advertised) \subseteq q.received
    /\ q.advertised = P!Input(q'.advertised).predecessor
    /\ UNCHANGED PublicationProjection
<1>1. ASSUME NEW c \in Clients, BridgeInv, Advertise(c)
      PROVE /\ P!Input(q'.advertised) = P!Input(p.intent[P!OpOf(q'.advertised)])
            /\ P!Input(q'.advertised).family = LocalRecords[q'.advertised].family
            /\ P!Input(q'.advertised).approvalBytes = LocalRecords[q'.advertised].approvalBytes
            /\ P!Closure(q'.advertised) \subseteq q.received
            /\ q.advertised = P!Input(q'.advertised).predecessor
            /\ UNCHANGED PublicationProjection
  <2> USE BridgeInv, Advertise(c)
  <2>1. q'.advertised = q.target[c] /\ q.target[c] \in p.published \ {0}
    BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, RemoteTypeOK, Advertise, AdvertiseCore
  <2>2. p.intent[P!OpOf(q'.advertised)] = q'.advertised
    BY SMT, <1>1, <2>1 DEF BridgeInv, Inv, P!Inv,
      P!PublicationChain, P!Prepared, P!BoundIntent
  <2>3. LocalRecords[q'.advertised] = P!Input(q'.advertised)
    BY SMT, <2>1 DEF LocalRecords, LocalRoots
  <2>4. P!Closure(q'.advertised) \subseteq q.received /\
    q.advertised = P!Input(q'.advertised).predecessor
    BY SMT, <1>1, <2>1 DEF Advertise, AdvertiseCore, Closure, BaseOf, P!Input
  <2>5. UNCHANGED PublicationProjection
    BY SMT, <1>1 DEF Advertise, AdvertiseCore, PublicationProjection, LocalRoots, LocalRecords
  <2>6. QED BY SMT, <2>2, <2>3, <2>4, <2>5
<1>2. QED BY <1>1
THEOREM FailureLeavesPriorAdvertisementUsable == \A c \in Clients, e \in Errors :
  BridgeInv /\ (TypedFailure(c,e) \/ NetworkInterrupt(c) \/ Crash(c)
    \/ Restart(c) \/ PointerConflict(c)) =>
    /\ q'.advertised = q.advertised
    /\ P!Closure(q'.advertised) \subseteq q'.received
    /\ UNCHANGED <<PublicationProjection,q.history,q.acknowledged>>
BY SMT, FailureFrames DEF BridgeInv, Inv, AdvertisementChain, RemoteClosure,
  RemoteDurable, PublicationProjection, LocalRoots, LocalRecords, FailureFrames
THEOREM SchemaTwoRemainsHistorical == BridgeInv /\ BridgeNext =>
  /\ 0 \in q'.history /\ Input(0).schema = 2
  /\ Input(0).domain = "kctx-operation-v2"
  /\ \A i \in q'.history \ q.history : Input(i).schema = 3
BY SMT, BridgeNextPreserves DEF BridgeInv, Inv, AdvertisementChain,
  Input, P!Input, P!SchemaOf, BridgeNextPreserves
THEOREM HistoricalReadUsesActualBytes == BridgeInv =>
  /\ HistoricalProjection.input = P!LegacyInput
  /\ HistoricalProjection.manifest.input = HistoricalProjection.input
  /\ HistoricalProjection.receipt.input = HistoricalProjection.input
  /\ ReadFamily(0).legacy = HistoricalProjection.input.context
  /\ HistoricalProjection.authentication.signedBytes = P!LegacyInput.canonicalBytes
  /\ HistoricalProjection.authentication.approvalBytes = P!LegacyInput.approvalBytes
  /\ HistoricalProjection.authentication.domain = "kctx-operation-v2"
<1>1. ASSUME BridgeInv PROVE
  /\ HistoricalProjection.input = P!LegacyInput
  /\ HistoricalProjection.manifest.input = HistoricalProjection.input
  /\ HistoricalProjection.receipt.input = HistoricalProjection.input
  /\ ReadFamily(0).legacy = HistoricalProjection.input.context
  /\ HistoricalProjection.authentication.signedBytes = P!LegacyInput.canonicalBytes
  /\ HistoricalProjection.authentication.approvalBytes = P!LegacyInput.approvalBytes
  /\ HistoricalProjection.authentication.domain = "kctx-operation-v2"
  <2> USE BridgeInv
  <2>1. p.legacy = P!LegacyHistory
    BY SMT, <1>1 DEF BridgeInv, Inv, TypeOK, P!TypeOK
  <2>2. HistoricalProjection.input = P!LegacyInput
    BY SMT, <2>1 DEF HistoricalProjection, P!LegacyHistory
  <2>3. HistoricalProjection.manifest.input = HistoricalProjection.input /\
    HistoricalProjection.receipt.input = HistoricalProjection.input
    BY SMT, <2>1 DEF HistoricalProjection, P!LegacyHistory
  <2>4. ReadFamily(0).legacy = HistoricalProjection.input.context
    BY SMT, <2>2 DEF ReadFamily
  <2>5. HistoricalProjection.authentication.signedBytes = P!LegacyInput.canonicalBytes /\
    HistoricalProjection.authentication.approvalBytes = P!LegacyInput.approvalBytes
    BY SMT, <2>1 DEF HistoricalProjection, P!LegacyHistory
  <2>6. P!LegacyInput.domain = "kctx-operation-v2"
    BY SMT DEF P!LegacyInput
  <2>7. HistoricalProjection.authentication.domain = "kctx-operation-v2"
    BY SMT, <2>1, <2>6 DEF HistoricalProjection, P!LegacyHistory
  <2>8. QED BY <2>2, <2>3, <2>4, <2>5, <2>7
<1>2. QED BY <1>1
=============================================================================
