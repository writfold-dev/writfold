---------------------- MODULE M01ReleasePublication ----------------------
EXTENDS Naturals, FiniteSets, TLAPS

(***************************************************************************
Finite identifiers denote exact canonical byte identities, NOT runtime integer
limits. Input(i) includes the complete ordered context family, approval bytes,
expected predecessor, schema/domain, source selections and material identity.
OpOf(1) = OpOf(2), but Input(1) # Input(2). No context pooling is modeled.

Trusted effect premises: immutable content-addressed objects; verified crypto
and kernel observations faithfully represented by auth; a successful durable
intent/native/manifest result includes the qualified fsync/retention contract;
completed Git ref CAS is atomic and conditional. Process crashes preserve
acknowledged durable state. Host power-loss durability is NOT inferred.
Observe invalidates final validation; an adapter must serialize final checks
and CAS or implement an equivalent version/lock check. Checks at publication
are historical evidence, not a claim that future policy cannot revoke trust.
***************************************************************************)
CONSTANTS WriterCount, InputCount, ContextCount
Bounds == /\ WriterCount \in Nat /\ WriterCount >= 2
          /\ InputCount \in Nat /\ InputCount >= 4
          /\ ContextCount \in Nat /\ ContextCount >= 2
ASSUME ParameterBounds == Bounds
USE ParameterBounds DEF Bounds
Writers == 1..WriterCount
Inputs == 1..InputCount
Releases == 0..InputCount
Ops == 1..(InputCount - 1)
Contexts == 1..ContextCount
Checks == {"policy", "signature", "kernel", "source"}
FullChecks == Checks \X Contexts
Objects == {"git", "native", "capture"} \X Releases
OpOf(i) == IF i <= 2 THEN 1 ELSE i - 1
(* The wide fixture adds input 4 as a distinct-operation contender for base 0:
   inputs 1 and 2 may fail different context checks while input 4 stays valid.
   Independently, inputs 1 then 3 still reach three completed generations. *)
BaseOf(i) == IF i <= 2 \/ i = 4 THEN 0 ELSE i - 2
(* Release 0 is independently authenticated, read-only schema-2 history.
   The last candidate is also schema 2, so new-write rejection is exercised. *)
SchemaOf(i) == IF i = 0 \/ i = InputCount THEN 2 ELSE 3
Family(i) == [c \in Contexts |->
  [context |-> c, snapshot |-> i, source |-> <<i,c>>,
   policy |-> <<c,i>>, intendedUse |-> c]]
Input(i) == [canonicalBytes |-> i, operation |-> OpOf(i),
  predecessor |-> BaseOf(i), schema |-> SchemaOf(i),
  domain |-> IF SchemaOf(i) = 3 THEN "kctx-operation-v3" ELSE "kctx-operation-v2",
  family |-> Family(i), orderedAssignments |-> <<i,Family(i)>>,
  retirements |-> <<i>>, approvalBytes |-> <<"approval",i>>,
  material |-> <<"material",i>>, checker |-> "semantic-2.0.0"]
Closure(i) == {<<"git",i>>, <<"native",i>>, <<"capture",i>>}

(* Independently authenticated completed schema-2 bytes, never re-encoded as
   schema 3. This exact historical record is part of the durable baseline,
   not a caller-supplied approval Boolean or a mutable recovery phase hint. *)
(* A distinct finite identity, comparable to request IDs by TLC; not a new write. *)
LegacyId == InputCount + 1
LegacyInput == [identity |-> LegacyId, operation |-> "legacy-operation",
  schema |-> 2, domain |-> "kctx-operation-v2",
  canonicalBytes |-> <<"legacy-canonical-bytes",0>>,
  approvalBytes |-> <<"legacy-approval-bytes",0>>,
  context |-> [identity |-> "legacy-context", source |-> <<"legacy-source",0>>,
               policy |-> <<"legacy-policy",0>>]]
LegacyHistory == [
  input |-> LegacyInput,
  manifest |-> [release |-> 0, input |-> LegacyInput, closure |-> Closure(0)],
  receipt |-> [release |-> 0, operation |-> LegacyInput.operation,
               input |-> LegacyInput, outcome |-> "applied"],
  authentication |-> [signedBytes |-> LegacyInput.canonicalBytes,
    approvalBytes |-> LegacyInput.approvalBytes, domain |-> LegacyInput.domain,
    policy |-> LegacyInput.context.policy, source |-> LegacyInput.context.source]]
Phases == {"idle", "planned", "admitted", "intent", "native", "manifest",
           "validated", "cas", "done", "failed", "conflict", "down"}
Outcomes == {"none", "pending", "complete", "rejected", "conflict", "unknown"}
VARIABLE p
vars == <<p>>

(* TLC fixture bound only, NOT an assumption of the inductive proofs.
   Explore each missing check/context independently; simultaneous missing
   observations are outside these finite fixtures, but inside Next/Inv. *)
AbsentObservations == {t \in Inputs \X Checks \X Contexts :
                       <<t[2],t[3]>> \notin p.auth[t[1]]}
FiniteObservationBound == Cardinality(AbsentObservations) <= 1

Valid(i) == /\ SchemaOf(i) = 3
            /\ \A c \in Contexts : \A k \in Checks : <<k,c>> \in p.auth[i]
BoundIntent(i) == p.intent[OpOf(i)] = i
Prepared(i) == /\ BoundIntent(i) /\ i \in p.native /\ i \in p.manifest
               /\ Closure(i) \subseteq p.objects
Fresh(i) == i \in p.validated /\ Valid(i)
Durable == <<p.head, p.published, p.intent, p.native, p.manifest,
             p.objects, p.evidence, p.receipt, p.legacy>>
Visible == <<p.head, p.published>>

TypeOK == p \in [head : Releases, published : SUBSET Releases,
  intent : [Ops -> Releases], native : SUBSET Inputs, manifest : SUBSET Inputs,
  objects : SUBSET Objects, evidence : [Inputs -> SUBSET FullChecks],
  receipt : SUBSET Inputs, auth : [Inputs -> SUBSET FullChecks],
  validated : SUBSET Inputs, request : [Writers -> Releases],
  pc : [Writers -> Phases], outcome : [Writers -> Outcomes], legacy : {LegacyHistory}]
Retention ==
  /\ Closure(0) \subseteq p.objects
  /\ (\A i \in p.native : BoundIntent(i) /\ <<"native",i>> \in p.objects)
  /\ (\A i \in p.manifest : i \in p.native /\ Closure(i) \subseteq p.objects)
IntentIdentity == \A o \in Ops : p.intent[o] # 0 =>
  /\ OpOf(p.intent[o]) = o
  /\ SchemaOf(p.intent[o]) = 3
PublicationChain ==
  /\ 0 \in p.published
  /\ p.head \in p.published
  /\ (\A i \in p.published : i <= p.head)
  /\ (\A i \in p.published \ {0} :
       /\ BaseOf(i) \in p.published
       /\ BaseOf(i) < i
       /\ Prepared(i)
       /\ SchemaOf(i) = 3
       /\ p.evidence[i] = FullChecks)
  /\ (\A i,j \in p.published \ {0} : BaseOf(i) = BaseOf(j) => i = j)
ReceiptSound == p.receipt \subseteq p.published \ {0}
ValidationSound == \A i \in p.validated : Prepared(i) /\ Valid(i)
Inv == TypeOK /\ IntentIdentity /\ Retention /\ PublicationChain
       /\ ReceiptSound /\ ValidationSound
       /\ (\A w \in Writers : p.outcome[w] = "complete" => p.request[w] \in p.receipt)

Safety ==
  /\ (\A i \in p.published \ {0} :
       /\ Closure(i) \subseteq p.objects
       /\ i \in p.native
       /\ p.intent[OpOf(i)] = i
       /\ Input(i).schema = 3
       /\ p.evidence[i] = FullChecks
       /\ Input(i).predecessor \in p.published)
  /\ (\A i,j \in p.published \ {0} :
       Input(i).predecessor = Input(j).predecessor => i = j)
  /\ (\A i \in p.receipt : i \in p.published)
  /\ (\A w \in Writers : p.outcome[w] = "complete" => p.request[w] \in p.receipt)
  /\ p.legacy = LegacyHistory
  /\ (\A i \in p.native : SchemaOf(i) = 3)

Init == p = [head |-> 0, published |-> {0}, intent |-> [o \in Ops |-> 0],
  native |-> {}, manifest |-> {}, objects |-> Closure(0),
  evidence |-> [i \in Inputs |-> {}], receipt |-> {},
  auth |-> [i \in Inputs |-> FullChecks], validated |-> {},
  request |-> [w \in Writers |-> 0], pc |-> [w \in Writers |-> "idle"],
  outcome |-> [w \in Writers |-> "none"], legacy |-> LegacyHistory]

Start(w,i) == /\ p.pc[w] = "idle" /\ i \notin p.published
  /\ p' = [p EXCEPT !.request[w] = i, !.pc[w] = "planned", !.outcome[w] = "pending"]
Admit(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] = "planned" /\ Valid(i)
  /\ p' = [p EXCEPT !.pc[w] = "admitted"]
RetainIntent(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] = "admitted" /\ Valid(i)
  /\ p.intent[OpOf(i)] \in {0,i}
  /\ p' = [p EXCEPT !.intent[OpOf(i)] = i, !.pc[w] = "intent"]
InputConflict(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \in {"planned","admitted"}
  /\ p.intent[OpOf(i)] \notin {0,i}
  /\ p' = [p EXCEPT !.pc[w] = "conflict", !.outcome[w] = "conflict"]
NativeCommit(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] = "intent" /\ BoundIntent(i)
  /\ p' = [p EXCEPT !.native = @ \cup {i},
       !.objects = @ \cup {<<"native",i>>}, !.pc[w] = "native"]
RetainManifest(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] = "native" /\ i \in p.native /\ BoundIntent(i)
  /\ p' = [p EXCEPT !.manifest = @ \cup {i}, !.objects = @ \cup Closure(i),
       !.pc[w] = "manifest"]
FinalRevalidate(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \in {"manifest","validated"} /\ Prepared(i) /\ Valid(i)
  /\ p' = [p EXCEPT !.validated = @ \cup {i}, !.pc[w] = "validated"]
Observe(i,a) == /\ a \subseteq FullChecks
  /\ p' = [p EXCEPT !.auth[i] = a, !.validated = @ \ {i}]

(* Flags retain semantic mutants without textual/parser corruption. *)
CASCore(w, predecessorGuard, retentionGuard, authorityGuard) ==
 LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \notin {"idle","down","done","failed","conflict"}
  /\ i \notin p.published
  /\ (predecessorGuard => p.head = BaseOf(i))
  /\ (retentionGuard => Prepared(i))
  /\ (authorityGuard => Fresh(i))
  /\ p' = [p EXCEPT !.head = i, !.published = @ \cup {i},
       !.evidence[i] = p.auth[i], !.pc[w] = "cas"]
DoCAS(w) == CASCore(w, TRUE, TRUE, TRUE)
MutantNoPredecessor(w) == CASCore(w, FALSE, TRUE, TRUE)
(* Fresh includes Prepared indirectly; early publication must drop the entire
   ordered durable+final-validation barrier, not an unreachable redundant guard. *)
MutantEarlyPublication(w) == CASCore(w, TRUE, FALSE, FALSE)
MutantStaleAuthority(w) == CASCore(w, TRUE, TRUE, FALSE)

CASMismatch(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \in {"manifest","validated"}
  /\ i \notin p.published /\ p.head # BaseOf(i)
  /\ p' = [p EXCEPT !.pc[w] = "conflict", !.outcome[w] = "conflict"]
RecordReceipt(i) == /\ i \in p.published \ {0}
  /\ p' = [p EXCEPT !.receipt = @ \cup {i}]
Finish(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ i \in p.receipt /\ p.pc[w] # "down"
  /\ p' = [p EXCEPT !.pc[w] = "done", !.outcome[w] = "complete"]
RejectAdmission(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \in {"planned","admitted","manifest","validated"}
  /\ ~Valid(i)
  /\ p' = [p EXCEPT !.pc[w] = "failed",
       !.outcome[w] = IF i \in p.published THEN "unknown" ELSE "rejected"]
FailIO(w) == LET i == p.request[w] IN
  /\ i \in Inputs /\ p.pc[w] \in {"admitted","intent","native","manifest","validated"}
  /\ p' = [p EXCEPT !.pc[w] = "failed",
       !.outcome[w] = IF i \in p.published THEN "unknown" ELSE "rejected"]
LostResponse(w) == /\ p.pc[w] \in {"cas","done"}
  /\ p' = [p EXCEPT !.pc[w] = "failed", !.outcome[w] = "unknown"]
Crash(w) == /\ p.pc[w] # "down"
  /\ p' = [p EXCEPT !.pc[w] = "down", !.validated = @ \ {p.request[w]},
       !.outcome[w] = IF p.request[w] = 0 THEN "none" ELSE "unknown"]
Restart(w) == /\ p.pc[w] = "down"
  /\ p' = [p EXCEPT !.pc[w] = "idle", !.request[w] = 0, !.outcome[w] = "none"]
Stutter == UNCHANGED p
NonObservationNext ==
  \/ (\E w \in Writers, i \in Inputs : Start(w,i))
  \/ (\E w \in Writers :
       \/ Admit(w) \/ RetainIntent(w) \/ InputConflict(w)
       \/ NativeCommit(w) \/ RetainManifest(w) \/ FinalRevalidate(w) \/ DoCAS(w)
       \/ CASMismatch(w) \/ Finish(w) \/ RejectAdmission(w) \/ FailIO(w)
       \/ LostResponse(w) \/ Crash(w) \/ Restart(w))
  \/ (\E i \in Inputs : RecordReceipt(i))
  \/ Stutter
Next == NonObservationNext \/
  (\E i \in Inputs, a \in SUBSET FullChecks : Observe(i,a))
(* Baseline representatives enumerate every individual missing check/context.
   Wide fixtures below use four fixed profiles, not their Cartesian product.
   All protocol/crash actions remain shared with unrestricted Next. *)
RepresentativeObservations ==
  {FullChecks} \cup {FullChecks \ {check} : check \in FullChecks}
FiniteNext == NonObservationNext \/
  (\E i \in Inputs, a \in RepresentativeObservations : Observe(i,a))
Spec == Init /\ [][Next]_vars
FiniteSpec == Init /\ [][FiniteNext]_vars

ProfileCheck(slot) ==
  IF slot = 0 THEN "policy" ELSE IF slot = 1 THEN "signature"
  ELSE IF slot = 2 THEN "kernel" ELSE "source"
ProfileFault(i,profile) ==
  LET slot == (5 * (i - 1) + 3 * profile) % (4 * ContextCount)
  IN <<ProfileCheck(slot % 4), 1 + (slot \div 4)>>
ProfileObservations(i,profile) == {FullChecks, FullChecks \ {ProfileFault(i,profile)}}
WideNext(profile) == NonObservationNext \/
  (\E i \in Inputs : \E a \in ProfileObservations(i,profile) : Observe(i,a))
WideSpec(profile) == Init /\ [][WideNext(profile)]_vars
WideSpec0 == WideSpec(0)
WideSpec1 == WideSpec(1)
WideSpec2 == WideSpec(2)
WideSpec3 == WideSpec(3)
(* With I=5,C=3, each profile has five independent full/missing toggles.
   Inputs 1/2 have different checks AND contexts in every profile; input 4
   may stay fully valid. Across profiles all twelve check/context pairs occur.
   Profiles are separate runs: changing profile within one run is forbidden.
   The unrestricted Next and all inductive invariants are unchanged. *)
USE DEF NonObservationNext

(* All obligations below are source obligations, not execution receipts.
   No fairness: arbitrary IO failure/crash/authority changes may prevent progress.
   Stutter is an explicit terminal convention; deadlock freedom is not liveness. *)
THEOREM IdentitySeparation == /\ OpOf(1) = OpOf(2) /\ Input(1) # Input(2)
  /\ \A i,j \in Inputs : Input(i) = Input(j) => i = j
BY SMT DEF OpOf, Input, Inputs
THEOREM InputShape == \A i \in Inputs :
  /\ OpOf(i) \in Ops /\ BaseOf(i) \in Releases /\ BaseOf(i) < i
  /\ Closure(i) \subseteq Objects
BY SMT, ParameterBounds DEF Bounds, Inputs, Ops, Releases, OpOf, BaseOf, Closure, Objects
THEOREM InputDomainFacts ==
  /\ Inputs \subseteq Releases \ {0}
  /\ 0 \in Releases
BY SMT, ParameterBounds DEF Inputs, Releases, Bounds
USE InputShape, InputDomainFacts DEF InputShape, InputDomainFacts

ReplyBound == \A w \in Writers :
  p.outcome[w] = "complete" => p.request[w] \in p.receipt

(* Structural helpers isolate typed record updates from the arithmetic and
   quantified publication-chain argument. Their premises are independently
   established below; none assumes the post-state invariant. *)
THEOREM ControlFrameInv ==
  Inv /\ TypeOK' /\ ReplyBound' /\
  UNCHANGED <<Durable,p.auth,p.validated>> => Inv'
BY SMT DEF Inv, ReplyBound, Durable, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, BoundIntent, Prepared, Valid

THEOREM ValidationFrameInv ==
  Inv /\ TypeOK' /\ ReplyBound' /\ ValidationSound' /\
  UNCHANGED <<Durable,p.auth>> => Inv'
BY SMT DEF Inv, ReplyBound, Durable, IntentIdentity, Retention,
  PublicationChain, ReceiptSound, BoundIntent, Prepared

(* Arithmetic is stated over Nat explicitly. Record typing alone leaves the
   abstract Releases symbol opaque in the small CAS preservation obligations. *)
THEOREM AppendMaximum == \A S \in SUBSET Nat, h,n \in Nat :
  (\A i \in S : i <= h) /\ h < n =>
    (\A i \in S \cup {n} : i <= n)
BY SMT

THEOREM AppendUniquePredecessor == \A S \in SUBSET Nat, h,n \in Nat :
  (/\ BaseOf(n) = h
   /\ (\A i \in S : i <= h)
   /\ (\A i \in S \ {0} : BaseOf(i) < i)
   /\ (\A i,j \in S \ {0} : BaseOf(i) = BaseOf(j) => i = j))
  => (\A i,j \in (S \cup {n}) \ {0} : BaseOf(i) = BaseOf(j) => i = j)
BY SMT

THEOREM IntentExtensionInv ==
  Inv /\ TypeOK' /\ IntentIdentity' /\
  UNCHANGED <<p.head,p.published,p.native,p.manifest,p.objects,p.evidence,
    p.receipt,p.auth,p.validated,p.request,p.outcome,p.legacy>> /\
  (\A o \in Ops : p.intent[o] # 0 => p'.intent[o] = p.intent[o]) => Inv'
BY SMT DEF Inv, TypeOK, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, BoundIntent, Prepared, Valid

THEOREM NativeExtensionInv ==
  Inv /\ TypeOK' /\ Retention' /\
  p.native \subseteq p'.native /\ p.objects \subseteq p'.objects /\
  UNCHANGED <<p.head,p.published,p.intent,p.manifest,p.evidence,p.receipt,
    p.auth,p.validated,p.request,p.outcome,p.legacy>> => Inv'
BY SMT DEF Inv, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, BoundIntent, Prepared, Valid

THEOREM ManifestExtensionInv ==
  Inv /\ TypeOK' /\ Retention' /\
  p.manifest \subseteq p'.manifest /\ p.objects \subseteq p'.objects /\
  UNCHANGED <<p.head,p.published,p.intent,p.native,p.evidence,p.receipt,
    p.auth,p.validated,p.request,p.outcome,p.legacy>> => Inv'
BY SMT DEF Inv, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, BoundIntent, Prepared, Valid

THEOREM ObservationFrameInv ==
  Inv /\ TypeOK' /\ ValidationSound' /\
  UNCHANGED <<Durable,p.request,p.outcome>> => Inv'
BY SMT DEF Inv, Durable, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, BoundIntent, Prepared

THEOREM ReceiptFrameInv ==
  Inv /\ TypeOK' /\ ReceiptSound' /\ ReplyBound' /\
  UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.objects,
    p.evidence,p.legacy,p.auth,p.validated>> => Inv'
BY SMT DEF Inv, ReplyBound, IntentIdentity, Retention, PublicationChain,
  ValidationSound, BoundIntent, Prepared, Valid

THEOREM ValidChecks == \A i \in Inputs :
  TypeOK /\ Valid(i) => p.auth[i] = FullChecks
BY SMT DEF TypeOK, Valid, FullChecks

THEOREM NativeNewWrites == Inv => \A i \in p.native : SchemaOf(i) = 3
BY SMT DEF Inv, TypeOK, Retention, IntentIdentity, BoundIntent

THEOREM CASChainPreserved == \A w \in Writers :
  Inv /\ DoCAS(w) => PublicationChain'
<1>1. ASSUME NEW w \in Writers, Inv, DoCAS(w)
      PROVE PublicationChain'
  <2>1. p.request[w] \in Inputs /\ p.head = BaseOf(p.request[w]) /\
        p.request[w] \notin p.published /\ Prepared(p.request[w]) /\
        Valid(p.request[w])
    BY SMT, <1>1 DEF DoCAS, CASCore, Fresh
  <2>2. p.auth[p.request[w]] = FullChecks
    BY SMT, <1>1, <2>1, ValidChecks DEF Inv
  <2>3. 0 \in p'.published /\ p'.head \in p'.published /\
        (\A i \in p'.published : i <= p'.head)
    <3>1. p.published \subseteq Nat /\ p.head \in Nat /\
          p.request[w] \in Nat /\ p.head < p.request[w]
      BY SMT, <1>1, <2>1, InputShape
        DEF Inv, TypeOK, Inputs, Releases
    <3>2. p'.published = p.published \cup {p.request[w]} /\
          p'.head = p.request[w]
      BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore
    <3>3. \A i \in p'.published : i <= p'.head
      BY SMT, <1>1, <3>1, <3>2, AppendMaximum DEF Inv, PublicationChain
    <3>4. QED
      BY SMT, <1>1, <3>2, <3>3 DEF Inv, PublicationChain
  <2>4. \A i \in p'.published \ {0} :
          /\ BaseOf(i) \in p'.published
          /\ BaseOf(i) < i
          /\ Prepared(i)'
          /\ SchemaOf(i) = 3
          /\ p'.evidence[i] = FullChecks
    BY SMT, <1>1, <2>1, <2>2 DEF Inv, TypeOK, PublicationChain,
      DoCAS, CASCore, Prepared, BoundIntent, Valid
  <2>5. \A i,j \in p'.published \ {0} : BaseOf(i) = BaseOf(j) => i = j
    <3>1. p.published \subseteq Nat /\ p.head \in Nat /\ p.request[w] \in Nat /\
          p'.published = p.published \cup {p.request[w]}
      BY SMT, <1>1, <2>1 DEF Inv, TypeOK, Inputs, Releases, DoCAS, CASCore
    <3>2. QED
      BY SMT, <1>1, <2>1, <3>1, AppendUniquePredecessor DEF Inv, PublicationChain
  <2>6. QED BY <2>3, <2>4, <2>5 DEF PublicationChain
<1>2. QED BY <1>1
THEOREM InitPreserves == Init => Inv
BY SMT, ParameterBounds DEF Init, Inv, TypeOK, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, Closure, Releases, Inputs, Ops, Writers, Objects,
  FullChecks, Phases, Outcomes, Bounds
THEOREM StartPreserves == \A w \in Writers, i \in Inputs :
  Inv /\ Start(w,i) => Inv'
<1>1. ASSUME NEW w \in Writers, NEW i \in Inputs, Inv, Start(w,i)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, Start, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, Start, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, Start, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM AdmitPreserves == \A w \in Writers :
  Inv /\ Admit(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Admit(w) =>
  TypeOK' /\ ReplyBound' /\ UNCHANGED <<Durable,p.auth,p.validated>>
  BY SMT DEF Admit, Inv, TypeOK, ReplyBound, Durable, Phases
<1>2. QED BY <1>1, ControlFrameInv
THEOREM RetainIntentPreserves == \A w \in Writers :
  Inv /\ RetainIntent(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, RetainIntent(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, RetainIntent, Phases
  <2>2. IntentIdentity'
    BY SMT, <1>1 DEF Inv, TypeOK, IntentIdentity, RetainIntent, Valid
  <2>3. \A o \in Ops : p.intent[o] # 0 => p'.intent[o] = p.intent[o]
    BY SMT, <1>1 DEF Inv, TypeOK, RetainIntent
  <2>4. UNCHANGED <<p.head,p.published,p.native,p.manifest,p.objects,p.evidence,
          p.receipt,p.auth,p.validated,p.request,p.outcome,p.legacy>>
    BY SMT, <1>1 DEF Inv, TypeOK, RetainIntent
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, IntentExtensionInv
<1>2. QED BY <1>1
THEOREM InputConflictPreserves == \A w \in Writers :
  Inv /\ InputConflict(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, InputConflict(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, InputConflict, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, InputConflict, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, InputConflict, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM NativeCommitPreserves == \A w \in Writers :
  Inv /\ NativeCommit(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, NativeCommit(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, NativeCommit, Phases, Objects, Closure
  <2>2. Retention'
    BY SMT, <1>1 DEF Inv, TypeOK, Retention, NativeCommit, BoundIntent
  <2>3. p.native \subseteq p'.native /\ p.objects \subseteq p'.objects
    BY SMT, <1>1 DEF Inv, TypeOK, NativeCommit
  <2>4. UNCHANGED <<p.head,p.published,p.intent,p.manifest,p.evidence,p.receipt,
          p.auth,p.validated,p.request,p.outcome,p.legacy>>
    BY SMT, <1>1 DEF Inv, TypeOK, NativeCommit
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, NativeExtensionInv
<1>2. QED BY <1>1
THEOREM RetainManifestPreserves == \A w \in Writers :
  Inv /\ RetainManifest(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, RetainManifest(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, RetainManifest, Phases
  <2>2. Retention'
    BY SMT, <1>1 DEF Inv, TypeOK, RetainManifest, Retention, BoundIntent
  <2>3. p.manifest \subseteq p'.manifest /\ p.objects \subseteq p'.objects
    BY SMT, <1>1 DEF Inv, TypeOK, RetainManifest
  <2>4. UNCHANGED <<p.head,p.published,p.intent,p.native,p.evidence,p.receipt,
          p.auth,p.validated,p.request,p.outcome,p.legacy>>
    BY SMT, <1>1 DEF Inv, TypeOK, RetainManifest
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, ManifestExtensionInv
<1>2. QED BY <1>1
THEOREM FinalRevalidatePreserves == \A w \in Writers :
  Inv /\ FinalRevalidate(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, FinalRevalidate(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, FinalRevalidate, Phases
  <2>2. ValidationSound'
    BY SMT, <1>1 DEF Inv, TypeOK, FinalRevalidate, ValidationSound,
      Prepared, BoundIntent, Valid
  <2>3. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, FinalRevalidate, ReplyBound
  <2>4. UNCHANGED <<Durable,p.auth>>
    BY SMT, <1>1 DEF Inv, TypeOK, FinalRevalidate, Durable
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, ValidationFrameInv
<1>2. QED BY <1>1
THEOREM ObservePreserves == \A i \in Inputs, a \in SUBSET FullChecks :
  Inv /\ Observe(i,a) => Inv'
<1>1. ASSUME NEW i \in Inputs, NEW a \in SUBSET FullChecks, Inv, Observe(i,a)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, Observe
  <2>2. ValidationSound'
    BY SMT, <1>1 DEF Inv, TypeOK, Observe, ValidationSound,
      Prepared, BoundIntent, Valid
  <2>3. UNCHANGED <<Durable,p.request,p.outcome>>
    BY SMT, <1>1 DEF Inv, TypeOK, Observe, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ObservationFrameInv
<1>2. QED BY <1>1
THEOREM DoCASPreserves == \A w \in Writers :
  Inv /\ DoCAS(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, DoCAS(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore, Phases
  <2>2. PublicationChain'
    BY <1>1, CASChainPreserved
  <2>3. IntentIdentity' /\ Retention' /\ ValidationSound'
    BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore, IntentIdentity,
      Retention, ValidationSound, Prepared, BoundIntent, Valid
  <2>4. ReceiptSound'
    BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore, ReceiptSound
  <2>5. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, DoCAS, CASCore, ReplyBound
  <2>6. QED BY <2>1, <2>2, <2>3, <2>4, <2>5 DEF Inv, ReplyBound
<1>2. QED BY <1>1
THEOREM CASMismatchPreserves == \A w \in Writers :
  Inv /\ CASMismatch(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, CASMismatch(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, CASMismatch, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, CASMismatch, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, CASMismatch, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM RecordReceiptPreserves == \A i \in Inputs :
  Inv /\ RecordReceipt(i) => Inv'
<1>1. ASSUME NEW i \in Inputs, Inv, RecordReceipt(i)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, RecordReceipt
  <2>2. ReceiptSound'
    BY SMT, <1>1 DEF Inv, TypeOK, RecordReceipt, ReceiptSound
  <2>3. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, RecordReceipt, ReplyBound
  <2>4. UNCHANGED <<p.head,p.published,p.intent,p.native,p.manifest,p.objects,
          p.evidence,p.legacy,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, RecordReceipt
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, ReceiptFrameInv
<1>2. QED BY <1>1
THEOREM FinishPreserves == \A w \in Writers :
  Inv /\ Finish(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, Finish(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, Finish, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, Finish, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, Finish, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM RejectAdmissionPreserves == \A w \in Writers :
  Inv /\ RejectAdmission(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, RejectAdmission(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, RejectAdmission, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, RejectAdmission, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, RejectAdmission, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM FailIOPreserves == \A w \in Writers :
  Inv /\ FailIO(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, FailIO(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, FailIO, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, FailIO, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, FailIO, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM LostResponsePreserves == \A w \in Writers :
  Inv /\ LostResponse(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, LostResponse(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, LostResponse, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, LostResponse, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, LostResponse, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1
THEOREM CrashPreserves == \A w \in Writers :
  Inv /\ Crash(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, Crash(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, Crash, Phases, Outcomes
  <2>2. ValidationSound'
    BY SMT, <1>1 DEF Inv, TypeOK, Crash, ValidationSound, Prepared, BoundIntent, Valid
  <2>3. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, Crash, ReplyBound
  <2>4. UNCHANGED <<Durable,p.auth>>
    BY SMT, <1>1 DEF Inv, TypeOK, Crash, Durable
  <2>5. QED BY <1>1, <2>1, <2>2, <2>3, <2>4, ValidationFrameInv
<1>2. QED BY <1>1
THEOREM RestartPreserves == \A w \in Writers :
  Inv /\ Restart(w) => Inv'
<1>1. ASSUME NEW w \in Writers, Inv, Restart(w)
      PROVE Inv'
  <2>1. TypeOK'
    BY SMT, <1>1 DEF Inv, TypeOK, Restart, Phases, Outcomes
  <2>2. ReplyBound'
    BY SMT, <1>1 DEF Inv, TypeOK, Restart, ReplyBound
  <2>3. UNCHANGED <<Durable,p.auth,p.validated>>
    BY SMT, <1>1 DEF Inv, TypeOK, Restart, Durable
  <2>4. QED BY <1>1, <2>1, <2>2, <2>3, ControlFrameInv
<1>2. QED BY <1>1

THEOREM StutterPreserves == Inv /\ Stutter => Inv'
BY SMT DEF Stutter, Inv, TypeOK, IntentIdentity, Retention, PublicationChain,
  ReceiptSound, ValidationSound, Prepared, BoundIntent, Valid
THEOREM FailureFrames == \A w \in Writers :
  Inv /\ (InputConflict(w) \/ CASMismatch(w) \/ RejectAdmission(w) \/ FailIO(w)
   \/ LostResponse(w) \/ Crash(w) \/ Restart(w)) => UNCHANGED Durable
BY SMT DEF InputConflict, CASMismatch, RejectAdmission, FailIO, LostResponse,
  Crash, Restart, Durable, Inv, TypeOK
THEOREM PreparationFrames == \A w \in Writers :
  Inv /\ (RetainIntent(w) \/ NativeCommit(w) \/ RetainManifest(w) \/ FinalRevalidate(w))
  => UNCHANGED Visible
BY SMT DEF RetainIntent, NativeCommit, RetainManifest, FinalRevalidate, Visible, Inv, TypeOK
THEOREM CASLinearization == \A w \in Writers : Inv /\ DoCAS(w) =>
  /\ p'.head = p.request[w] /\ p.head = Input(p.request[w]).predecessor
  /\ p'.published = p.published \cup {p.request[w]}
  /\ p.request[w] \notin p.published /\ Prepared(p.request[w]) /\ Valid(p.request[w])
BY SMT DEF DoCAS, CASCore, Input, Fresh, Inv, TypeOK
THEOREM IntentNeverRebound == Inv /\ Next =>
  \A o \in Ops : p.intent[o] # 0 => p'.intent[o] = p.intent[o]
BY SMT DEF Next, Start, Admit, RetainIntent, InputConflict, NativeCommit,
  RetainManifest, FinalRevalidate, DoCAS, CASCore, CASMismatch, RecordReceipt,
  Finish, RejectAdmission, FailIO, LostResponse, Crash, Restart, Observe, Stutter,
  Inv, TypeOK, IntentIdentity, OpOf, Ops, Inputs, Releases
THEOREM NextPreserves == Inv /\ Next => Inv'
BY StartPreserves, AdmitPreserves, RetainIntentPreserves, InputConflictPreserves,
  NativeCommitPreserves, RetainManifestPreserves, FinalRevalidatePreserves,
  ObservePreserves, DoCASPreserves, CASMismatchPreserves, RecordReceiptPreserves,
  FinishPreserves, RejectAdmissionPreserves, FailIOPreserves, LostResponsePreserves,
  CrashPreserves, RestartPreserves, StutterPreserves DEF Next
THEOREM InvImpliesSafety == Inv => Safety
BY SMT, NativeNewWrites DEF Inv, TypeOK, Safety, PublicationChain,
  ReceiptSound, Prepared, BoundIntent, Input
THEOREM PendingLegacyNeverPrepared == Inv =>
  InputCount \notin p.native \cup p.manifest \cup (p.published \ {0})
BY SMT, InputShape DEF Inv, TypeOK, IntentIdentity, Retention, PublicationChain,
  BoundIntent, SchemaOf, Inputs, Ops, OpOf
THEOREM InductiveSafety == Spec => []Safety
<1>1. Init => Inv BY InitPreserves
<1>2. Inv /\ [Next]_vars => Inv'
  BY NextPreserves, StutterPreserves DEF vars, Stutter
<1>3. Spec => []Inv BY <1>1, <1>2, PTL DEF Spec
<1>4. QED BY <1>3, InvImpliesSafety, PTL

THEOREM RepresentativeObservationsSound ==
  \A a \in RepresentativeObservations : a \subseteq FullChecks
BY SMT DEF RepresentativeObservations
THEOREM FiniteObserveRefines == \A i \in Inputs, a \in RepresentativeObservations :
  Observe(i,a) => Next
BY SMT, RepresentativeObservationsSound DEF Next
THEOREM FiniteStepRefines == FiniteNext => Next
BY SMT, FiniteObserveRefines DEF FiniteNext, Next
THEOREM FiniteSpecRefines == FiniteSpec => Spec
BY FiniteStepRefines, PTL DEF FiniteSpec, Spec

THEOREM ProfileFaultTyped == \A i \in Inputs, profile \in 0..3 :
  ProfileFault(i,profile) \in FullChecks
BY SMT, ParameterBounds DEF ProfileFault, ProfileCheck, FullChecks, Checks,
  Contexts, Inputs, Bounds
THEOREM ProfileObservationsSound == \A i \in Inputs, profile \in 0..3 :
  \A a \in ProfileObservations(i,profile) : a \subseteq FullChecks
BY SMT DEF ProfileObservations
THEOREM WideStepRefines == \A profile \in 0..3 : WideNext(profile) => Next
BY SMT, ProfileObservationsSound DEF WideNext, Next
THEOREM WideSpecRefines == \A profile \in 0..3 : WideSpec(profile) => Spec
<1>1. ASSUME NEW profile \in 0..3
      PROVE WideSpec(profile) => Spec
  <2>1. WideNext(profile) => Next BY <1>1, WideStepRefines
  <2>2. QED BY <2>1, PTL DEF WideSpec, Spec
<1>2. QED BY <1>1
=============================================================================
