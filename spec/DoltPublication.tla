--------------------------- MODULE DoltPublication ---------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC

(***************************************************************************
The publication order of `writfold-dolt-integration/1` (DINT02 first slice,
docs/writfold/dolt-integration.md#publication-order-and-recovery) over the
Dolt conditional writer, and its read-only reconciliation.

Actors: `publish` and `complete` invocations of signed operations (`Table`),
and a direct writer that commits to Dolt `main` outside publication. One
invocation runs at a time: it holds the realm's exclusive Git writer lease and
the Dolt writer lease for its whole call, and a direct write happens only
while no invocation holds them. Any invocation may be killed between any two
of its steps; a kill releases both leases (the supervisor reaps the group)
and keeps every durable Git and Dolt effect.

publish: (1) Git expected-old check (`Admit`, writfold_core::publication::admit),
(2) retain the candidate blob, (3) Dolt conditional commit from the signed
base pin, which may be killed after DOLT_COMMIT and before the retention
branch, (4) retain the new head, read every change back at it and write the
prepared commit naming it, (5) Git expected-old CAS of the completed ref, the
only visibility point.

observe is read-only under the same leases, so it is a pure function of the
state whenever no invocation holds them: `Resolve` mirrors
writfold_core::publication::resolve over the facts the Rust `observed`
gathers. complete re-observes, accepts only Interrupted, and issues the
remaining Git steps: retain the blob, then (if no prepared commit) retain the
head, read back and prepare, then the CAS. It never writes Dolt.

Identities: a release is named by its operation (G0 is the fixture-owned
genesis release). A Dolt commit is named by its writer (an operation, or a
direct write); the genesis commit is P0. A signed base pin names the commit
the signer expects. The Dolt write's request identity (`writfold-dolt-write/1`
marker) is the expected-old pin and the candidate changes; the operation is
NOT part of it, exactly as in writfold-store's `marker`. The `applied` probe
finds the commit whose parent is the expected pin and whose marker matches.

Trusted premises: Git ref updates are atomic and conditional on their
expected old value; one Dolt process commits atomically under the native
lock; the leases are exclusive while held; nothing bypasses them except the
modelled direct writer, which never runs inside a held lease. Not modelled:
power loss, Git reference-lock files (cleared by complete), the candidate's
row-level contents (a candidate is one opaque value; read-back compares it),
policy/target admission and signature checks, bootstrap.
***************************************************************************)
CONSTANTS Table, MaxDirect
CONSTANT None  \* a model value: absent
G0 == "g0"            \* genesis release
P0 == "p0"            \* genesis Dolt commit, named by G0's manifest
DirectNames == <<"w1", "w2", "w3">>
Ops == DOMAIN Table
Base(o) == Table[o].base     \* signed base release (Git expected old)
Pin(o) == Table[o].pin       \* signed base pin (Dolt expected old)
Cand(o) == Table[o].cand     \* the candidate (not signed)

VARIABLES
  gitRef,     \* the completed ref refs/kctx/knowledge
  prepared,   \* each operation's prepared commit: the pin its manifest names
  blobs,      \* retained candidate blobs
  dolt,       \* Dolt main after P0, oldest first: [parent, mb, cand, by]
  retained,   \* Dolt commits with a wf-retained root
  direct,     \* direct writes so far
  inv,        \* the invocation holding the leases, or None
  \* history variables (observation only, never read by guards):
  casCount,   \* completed-ref CASes per operation
  replayed,   \* an invocation was accepted for an already-published operation
  refused,    \* an accepted invocation was refused after its first write
  doltByComplete \* complete issued a Dolt write
vars == <<gitRef, prepared, blobs, dolt, retained, direct, inv,
          casCount, replayed, refused, doltByComplete>>

Tip == Len(dolt)
Id(k) == IF k = 0 THEN P0 ELSE dolt[k].by
HeadId == Id(Tip)
Bound == Cardinality(Ops) + MaxDirect + 1

RECURSIVE Chain(_)
(* A release and its predecessors: a prepared commit's parent is its
   operation's signed base release. *)
Chain(r) == IF r = G0 THEN {G0} ELSE {r} \cup Chain(Base(r))
OnChain(o) == o \in Chain(gitRef)
(* The pin a release's manifest names. *)
PinOf(r) == IF r = G0 THEN P0 ELSE prepared[r]

(* The commit carrying exactly o's request: parent and marker match. *)
AppliedAt(o) == {k \in 1..Tip : /\ dolt[k].parent = Pin(o)
                                 /\ dolt[k].mb = Pin(o)
                                 /\ dolt[k].cand = Cand(o)}
Applied(o) == IF AppliedAt(o) = {} THEN None ELSE Id(CHOOSE k \in AppliedAt(o) : TRUE)
(* Ground truth: the commits o's own Dolt write produced. *)
WritesBy(o) == {k \in 1..Tip : dolt[k].by = o}
WroteBy(o) == IF WritesBy(o) = {} THEN None ELSE o
(* Read-back: the commit named p exists and carries o's candidate. *)
ReadsBack(p, o) == \E k \in 1..Tip : dolt[k].by = p /\ dolt[k].cand = Cand(o)

-----------------------------------------------------------------------------
(* writfold_core::publication::admit over the facts `publish` gathers. The
   flags drop a guard (calibrated negatives). Without the Git guard the pin
   is checked against the signed base release's manifest. *)
AdmitCore(o, gitGuard, preparedGuard) ==
  IF gitGuard /\ gitRef # Base(o) THEN "Conflict"
  ELSE IF PinOf(IF gitGuard THEN gitRef ELSE Base(o)) # Pin(o) THEN "Misinterpreted"
  ELSE IF preparedGuard /\ prepared[o] # None THEN "AlreadyPrepared"
  ELSE "Proceed"
Admit(o) == AdmitCore(o, TRUE, TRUE)
AdmitNoGitGuard(o) == AdmitCore(o, FALSE, TRUE)
AdmitNoPreparedGuard(o) == AdmitCore(o, TRUE, FALSE)
AdmitNoReplayGuards(o) == AdmitCore(o, FALSE, FALSE)

(* writfold_core::publication::resolve over the facts `observed` gathers.
   `unprepared`: an applied, unprepared attempt is Interrupted(false);
   `needHead`: Interrupted requires the Dolt head to be the applied commit. *)
ResolveCore(o, unprepared, needHead) ==
  LET prep == prepared[o] # None
      published == prep /\ OnChain(o)
      applied == Applied(o) # None
      names == prep /\ applied /\ prepared[o] = Applied(o)
      headIs == applied /\ Applied(o) = HeadId
      isBase == gitRef = Base(o)
  IN IF published THEN (IF applied /\ prep /\ names THEN "Published" ELSE "Diverged")
     ELSE IF ~applied THEN (IF prep THEN "Diverged" ELSE "NotPublished")
     ELSE IF isBase /\ (headIs \/ ~needHead)
       THEN (IF prep THEN (IF names THEN "Interrupted" ELSE "Diverged")
             ELSE IF unprepared THEN "Interrupted" ELSE "NotPublished")
     ELSE "Diverged"
Resolve(o) == ResolveCore(o, TRUE, TRUE)
ResolveForgetsUnprepared(o) == ResolveCore(o, FALSE, TRUE)
ResolveIgnoresHead(o) == ResolveCore(o, TRUE, FALSE)

-----------------------------------------------------------------------------
Init ==
  /\ gitRef = G0
  /\ prepared = [o \in Ops |-> None]
  /\ blobs = {}
  /\ dolt = <<>>
  /\ retained = {P0}
  /\ direct = 0
  /\ inv = None
  /\ casCount = [o \in Ops |-> 0]
  /\ replayed = FALSE
  /\ refused = FALSE
  /\ doltByComplete = FALSE

At(o, s) == inv # None /\ inv.op = o /\ inv.step = s
Go(s) == inv' = [inv EXCEPT !.step = s]
Stop == inv' = None
History == <<casCount, replayed, refused, doltByComplete>>

(* Step 1, atomic with taking the leases: a refusal writes nothing, so only
   Proceed is a state change. *)
PublishStart(o) ==
  /\ inv = None /\ Admit(o) = "Proceed"
  /\ inv' = [op |-> o, kind |-> "publish", step |-> "checked", pin |-> None]
  /\ replayed' = (replayed \/ casCount[o] > 0)
  /\ UNCHANGED <<gitRef, prepared, blobs, dolt, retained, direct,
                 casCount, refused, doltByComplete>>

(* Step 2: the candidate blob (invisible, idempotent). The mutant also
   writes the per-operation prepared ref here, before the Dolt write. *)
RetainCore(o, early) ==
  /\ At(o, "checked")
  /\ blobs' = blobs \cup {Cand(o)}
  /\ prepared' = IF early THEN [prepared EXCEPT ![o] = P0] ELSE prepared
  /\ Go("retained")
  /\ UNCHANGED <<gitRef, dolt, retained, direct, History>>
Retain(o) == RetainCore(o, FALSE)
RetainAsPrepared(o) == RetainCore(o, TRUE)

(* Step 3: the conditional commit. Committed, or killed after DOLT_COMMIT
   before the retention branch (Unknown), or Stale with nothing written. *)
DoltWriteCore(o, guard) ==
  /\ At(o, "retained")
  /\ IF guard => HeadId = Pin(o)
       THEN /\ Tip < Bound
            /\ dolt' = Append(dolt, [parent |-> HeadId, mb |-> Pin(o),
                                     cand |-> Cand(o), by |-> o])
            /\ \/ /\ retained' = retained \cup {o}
                  /\ inv' = [inv EXCEPT !.step = "committed", !.pin = o]
               \/ /\ UNCHANGED retained
                  /\ Stop
       ELSE /\ Stop
            /\ UNCHANGED <<dolt, retained>>
  /\ doltByComplete' = (doltByComplete \/ inv.kind = "complete")
  /\ UNCHANGED <<gitRef, prepared, blobs, direct, casCount, replayed, refused>>
DoltWrite(o) == DoltWriteCore(o, TRUE)
DoltWriteNoGuard(o) == DoltWriteCore(o, FALSE)

(* Calibrated negative: Git first. The prepared commit names the commit the
   write is about to make and the completed ref moves, then the Dolt write. *)
DoltWriteGitFirst(o) ==
  /\ At(o, "retained")
  /\ prepared' = [prepared EXCEPT ![o] = o]
  /\ IF gitRef = Base(o)
       THEN gitRef' = o /\ casCount' = [casCount EXCEPT ![o] = @ + 1]
       ELSE UNCHANGED <<gitRef, casCount>>
  /\ Go("late")
  /\ UNCHANGED <<blobs, dolt, retained, direct, replayed, refused, doltByComplete>>
LateDolt(o) ==
  /\ At(o, "late")
  /\ IF HeadId = Pin(o) /\ Tip < Bound
       THEN /\ dolt' = Append(dolt, [parent |-> HeadId, mb |-> Pin(o),
                                     cand |-> Cand(o), by |-> o])
            /\ retained' = retained \cup {o}
       ELSE UNCHANGED <<dolt, retained>>
  /\ Stop
  /\ UNCHANGED <<gitRef, prepared, blobs, direct, History>>

(* Step 4a: retain the head (guarded on the head being the pin). *)
RetainHeadCore(o, doIt) ==
  /\ At(o, "committed")
  /\ IF HeadId = inv.pin
       THEN /\ retained' = IF doIt THEN retained \cup {inv.pin} ELSE retained
            /\ Go("headRetained") /\ UNCHANGED refused
       ELSE /\ refused' = TRUE /\ Stop /\ UNCHANGED retained
  /\ UNCHANGED <<gitRef, prepared, blobs, dolt, direct,
                 casCount, replayed, doltByComplete>>
RetainHead(o) == RetainHeadCore(o, TRUE)
RetainHeadSkipped(o) == RetainHeadCore(o, FALSE)

(* Step 4b: read back at the pin, then the prepared commit (refused if one
   exists for the operation). *)
Prepare(o) ==
  /\ At(o, "headRetained")
  /\ IF ReadsBack(inv.pin, o) /\ prepared[o] = None
       THEN /\ prepared' = [prepared EXCEPT ![o] = inv.pin]
            /\ Go("prepared") /\ UNCHANGED refused
       ELSE /\ refused' = TRUE /\ Stop /\ UNCHANGED prepared
  /\ UNCHANGED <<gitRef, blobs, dolt, retained, direct,
                 casCount, replayed, doltByComplete>>

(* Step 5: the Git expected-old CAS, the only visibility point. *)
Cas(o) ==
  /\ At(o, "prepared")
  /\ IF gitRef = Base(o)
       THEN /\ gitRef' = o /\ casCount' = [casCount EXCEPT ![o] = @ + 1]
            /\ UNCHANGED refused
       ELSE /\ refused' = TRUE /\ UNCHANGED <<gitRef, casCount>>
  /\ Stop
  /\ UNCHANGED <<prepared, blobs, dolt, retained, direct, replayed, doltByComplete>>

(* complete: re-observe under the leases; only Interrupted is accepted. *)
CompleteStartCore(o, accepted) ==
  /\ inv = None /\ Resolve(o) \in accepted
  /\ inv' = [op |-> o, kind |-> "complete", step |-> "c_checked", pin |-> Applied(o)]
  /\ replayed' = (replayed \/ casCount[o] > 0)
  /\ UNCHANGED <<gitRef, prepared, blobs, dolt, retained, direct,
                 casCount, refused, doltByComplete>>
CompleteStart(o) == CompleteStartCore(o, {"Interrupted"})
(* Calibrated negative: complete also accepts a Published operation. *)
CompleteAcceptsPublished(o) == CompleteStartCore(o, {"Interrupted", "Published"})
(* Retain the blob, then continue at 4a, or at 5 if already prepared. The
   mutant instead re-enters the order before the Dolt write. *)
CompleteRetainCore(o, reissue) ==
  /\ At(o, "c_checked")
  /\ blobs' = blobs \cup {Cand(o)}
  /\ Go(IF reissue THEN "retained"
        ELSE IF prepared[o] = None THEN "committed" ELSE "prepared")
  /\ UNCHANGED <<gitRef, prepared, dolt, retained, direct, History>>
CompleteRetain(o) == CompleteRetainCore(o, FALSE)
CompleteReissuesDolt(o) == CompleteRetainCore(o, TRUE)

(* A kill between any two steps: leases released, durable effects kept. *)
Crash ==
  /\ inv # None /\ Stop
  /\ UNCHANGED <<gitRef, prepared, blobs, dolt, retained, direct, History>>

(* A direct commit on Dolt main outside publication (never inside a lease). *)
DirectWrite ==
  /\ inv = None /\ direct < MaxDirect /\ Tip < Bound
  /\ dolt' = Append(dolt, [parent |-> HeadId, mb |-> HeadId,
                           cand |-> "direct", by |-> DirectNames[direct + 1]])
  /\ direct' = direct + 1
  /\ UNCHANGED <<gitRef, prepared, blobs, retained, inv, History>>

Next ==
  \/ \E o \in Ops :
       \/ PublishStart(o) \/ Retain(o) \/ DoltWrite(o) \/ LateDolt(o)
       \/ RetainHead(o) \/ Prepare(o) \/ Cas(o)
       \/ CompleteStart(o) \/ CompleteRetain(o)
  \/ Crash \/ DirectWrite

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
Commits == {P0} \cup Ops \cup {DirectNames[i] : i \in 1..3}
TypeOK ==
  /\ gitRef \in {G0} \cup Ops
  /\ prepared \in [Ops -> Commits \cup {None}]
  /\ blobs \subseteq {Cand(o) : o \in Ops}
  /\ Tip <= Bound
  /\ \A k \in 1..Tip : dolt[k].by \in Commits /\ dolt[k].parent = Id(k - 1)
  /\ retained \subseteq Commits
  /\ direct \in 0..MaxDirect
  /\ inv = None \/ (inv.op \in Ops /\ inv.kind \in {"publish", "complete"})
  /\ casCount \in [Ops -> Nat]
  /\ replayed \in BOOLEAN /\ refused \in BOOLEAN /\ doltByComplete \in BOOLEAN

(* At most one release CAS and one Dolt commit per operation, and no Dolt
   commit is named by two releases on the completed history. *)
NoDoublePublication ==
  /\ \A o \in Ops : casCount[o] <= 1 /\ Cardinality(WritesBy(o)) <= 1
  /\ \A r1, r2 \in Chain(gitRef) : r1 # r2 => PinOf(r1) # PinOf(r2)

(* What readers can see: the pins named on the completed history. *)
VisibleSet == {PinOf(r) : r \in Chain(gitRef)}
(* Calibrated negative: readers follow the Dolt head. *)
VisibleViaHead == {PinOf(r) : r \in Chain(gitRef)} \cup {HeadId}
(* Every visible commit beyond genesis was made visible by a completed CAS. *)
VisibleOnlyViaCas ==
  \A p \in VisibleSet : p = P0 \/ \E o \in Ops : casCount[o] > 0 /\ prepared[o] = p

(* A published release names exactly the commit its own Dolt write made,
   which is retained, and its candidate blob is retained. *)
PublishedPinIsCommittedHead ==
  \A o \in Chain(gitRef) \ {G0} :
    /\ WroteBy(o) = o /\ prepared[o] = o
    /\ o \in retained /\ Cand(o) \in blobs

(* observe agrees with ground truth whenever it can run (leases free), and
   refuses Diverged only after a direct write moved the head past an
   interrupted attempt. *)
ObserveAgrees ==
  inv = None =>
    \A o \in Ops :
      LET r == Resolve(o) IN
      /\ r = "Published" => OnChain(o) /\ WroteBy(o) = o /\ prepared[o] = o
      /\ r = "NotPublished" => WroteBy(o) = None /\ ~OnChain(o) /\ prepared[o] = None
      /\ r = "Interrupted" => WroteBy(o) = o /\ ~OnChain(o) /\ Applied(o) = o
      /\ r = "Diverged" => WroteBy(o) = o /\ ~OnChain(o) /\ HeadId # o
      /\ OnChain(o) => r = "Published"
      /\ (WroteBy(o) = None /\ ~OnChain(o)) => r = "NotPublished"

(* An Interrupted attempt satisfies every guard of the Git-only completion,
   and no accepted invocation is ever refused after its first write. *)
InterruptedCompletable ==
  /\ inv = None =>
       \A o \in Ops : Resolve(o) = "Interrupted" =>
         /\ gitRef = Base(o)
         /\ Applied(o) # None /\ Applied(o) = HeadId /\ ReadsBack(Applied(o), o)
         /\ prepared[o] \in {None, Applied(o)}
  /\ ~refused
(* complete never issues a Dolt write, committed or not. *)
CompleteGitOnly == ~doltByComplete

(* An attempt whose Dolt write never happened leaves the approval unspent:
   no prepared commit, and whenever the Git ref is again its base with its
   pin, publish admits it. *)
NotPublishedKeepsApproval ==
  \A o \in Ops :
    (WroteBy(o) = None /\ ~OnChain(o)) =>
      /\ prepared[o] = None
      /\ (gitRef = Base(o) /\ PinOf(gitRef) = Pin(o)) => Admit(o) = "Proceed"

(* A published approval is never accepted again by publish or complete, and
   its release stays on the completed history. *)
SpentCannotRepublish ==
  /\ ~replayed
  /\ \A o \in Ops : casCount[o] > 0 => OnChain(o)

Safety ==
  /\ NoDoublePublication /\ VisibleOnlyViaCas /\ PublishedPinIsCommittedHead
  /\ ObserveAgrees /\ InterruptedCompletable /\ CompleteGitOnly
  /\ NotPublishedKeepsApproval /\ SpentCannotRepublish

(* Reachability probe (expected violation, the witness): the successor c
   publishes over a's release. *)
SuccessorNeverPublished == ~OnChain("c")

-----------------------------------------------------------------------------
(* Fixtures. a and b: rival signed operations over the genesis release with
   distinct candidates. c: a successor signed over a's release and a's
   commit, with the same candidate bytes as a (a different request, since
   the expected pin differs). *)
Rec(b, p, c) == [base |-> b, pin |-> p, cand |-> c]
PositiveTable ==
  ("a" :> Rec(G0, P0, "x")) @@ ("b" :> Rec(G0, P0, "y")) @@ ("c" :> Rec("a", "a", "x"))
(* Wider: adds m, whose signed pin is not what its base release names
   (Misinterpreted), and e, a successor over b's release. *)
WideTable ==
  PositiveTable @@ ("m" :> Rec(G0, "b", "z")) @@ ("e" :> Rec("b", "b", "x"))
(* Boundary: d is a rival of a over the same base with the same candidate
   bytes, so both make the same Dolt request. *)
SameCandidateTable ==
  ("a" :> Rec(G0, P0, "x")) @@ ("d" :> Rec(G0, P0, "x"))
=============================================================================
