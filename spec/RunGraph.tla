------------------------------ MODULE RunGraph ------------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC

(***************************************************************************
The run graph (docs/writfold/mission.md, "The run graph, hooks and order of
work"; docs/writfold/kb-v1-design.md sections 21, 24 and 26;
crates/writfold-kb/src/plan/ and crates/writfold-core/src/plan.rs): a run's
plan is an append-only set of event files under runs/<run>/plan/, and the
steps, the current step and what is done are derived from that set on
every read. Model evidence about the design, not a refinement of the Rust.

Events. A template snapshot (named by the template's digest), a splice
(named by parent, template and ordinal; it also carries the digest), a
`done-` record (named by its node; it names, per gate, the evidence and the
content it was judged on) and a `skip-` record (named by its node). Every
event is published create-new and atomically: that is the premise
spec/KbWrite.tla checks, so the file protocol and checkpoints are not
re-modelled here. A file's content carries its writer (the real bytes carry
a `recorded` time and the run's records), so two sessions never write equal
bytes under one name, except snapshots, whose bytes are the template's.

Actors. Two sessions, each working in one clone (`Home`; `Topology`
"separate" puts them in two clones that merge, "same" in one clone). A
session reads the fold outside the write lock and decides, then, under the
clone's lock,
re-checks (`Recheck`) and publishes its files one at a time; a crash kills
every session of a clone between any two of those steps. The environment
moves a clone's subject bytes (a gate goes stale), records a passing
receipt for the present bytes, and edits a template (version 1 to 2, a new
digest). Merge pulls one clone's files and receipts into another: a set
union of files, refused (left to the user, outside Writfold) when the two
hold different bytes under one name, as Git's add/add conflict. Premise: a
merge into a clone runs between Writfold invocations there, not while one
of its sessions is between its decision and its last publish.

Verbs, as built (section 24). `plan set <t>` (and `plan use <t>` on a run
without a plan): the snapshot, then the root splice at ordinal 1; nothing
is written when the run already has a plan. `plan use <t>`: a splice under
the current node N whose steps go before N as its children, at the next
ordinal under N (ordinals count per parent, not per template); it writes
nothing when the key (template, digest) is already spliced under N, an
ancestor of N or the root; `--again` skips that check. `plan advance`: the
current node's gates judged now; a done record naming the receipt that met
each gate and the content judged. `plan skip`: a skip record.

The fold. Splices are expanded from their snapshots in key order
(parent, ordinal, template), post-order: a node's children come before it.
A splice without its snapshot is invalid. The current node is the first
neither done nor skipped (writfold_core::plan::current).

Constants. Splicing: whether `plan use` exists. Recheck: what is
re-checked under the lock before publishing: "none" is the Rust as built
(create-new only: a name taken is a conflict); "all" re-checks the decision
(plan set: no splice exists; advance and skip: N is still the current
node; use: N is still current and no splice was added under it, so its
ordinal is still the next), which this model proposes. Mutant: "none", or
one deviation, each failing one property (see the .cfg files).

Model bounds: two templates of one or two steps, two versions each, one
gate; splices nested two levels (the Rust allows 4); ordinals up to 2;
MaxWrites decisions per session; MaxDrift subject moves per clone;
MaxCrashes crashes in all.

Not modelled: the write protocol and power loss (KbWrite.tla), `use=`
includes and their closure (one more snapshot per included template, the
same ordering), required start and end templates (more root splices at
ordinals 2, 3), `!template-moved` (a flag only; the run is unchanged), gate
definitions changing, attested and waived gates (to the decision they are
met or not met, like a receipt), run-record windows, a closed run, a
hand-edited event (read as invalid and counted for nothing).
***************************************************************************)
CONSTANTS Topology, MaxDrift, MaxCrashes, MaxWrites, Splicing, Recheck, Mutant

\* "separate": session s1 in clone c1, s2 in c2; "same": both in c1.
Sessions == {"s1", "s2"}
Clones == IF Topology = "separate" THEN {"c1", "c2"} ELSE {"c1"}
Home == [s \in Sessions |-> IF Topology = "same" \/ s = "s1" THEN "c1" ELSE "c2"]

Templates == {"chg", "sk"}
KeyOrder == <<<<1, "chg">>, <<1, "sk">>, <<2, "chg">>, <<2, "sk">>>>
MaxOrd == 2
\* Version 2 of chg drops step b: only the snapshot keeps a run's steps.
Steps(t, v) ==
  CASE t = "chg" /\ v = 1 -> <<"a", "b">>
    [] t = "chg" /\ v = 2 -> <<"a", "c">>
    [] t = "sk" /\ v = 1 -> <<"k">>
    [] t = "sk" /\ v = 2 -> <<"k", "m">>
\* Steps a and k owe the one gate (on the subject's bytes); the others none.
Gated(id) == id \in {"a", "k"}

Max(S) == CHOOSE x \in S : \A y \in S : y <= x
Range(f) == {f[i] : i \in DOMAIN f}
RECURSIVE Cat(_)
Cat(seqs) == IF seqs = <<>> THEN <<>> ELSE Head(seqs) \o Cat(Tail(seqs))

\* A node is a path: a sequence of <<template, ordinal, step>>.
StepOf(n) == n[Len(n)][3]

Ev(k, node, t, v, o, ev, w) ==
  [k |-> k, node |-> node, t |-> t, v |-> v, ord |-> o, ev |-> ev, w |-> w]
SnapEv(t, v) == Ev("snap", <<>>, t, v, 0, {}, "-")
SpliceEv(p, t, v, o, w) == Ev("splice", p, t, v, o, {}, w)
DoneEv(n, ev, w) == Ev("done", n, "-", 0, 0, ev, w)
SkipEv(n, w) == Ev("skip", n, "-", 0, 0, {}, w)

\* The file name an event must have. Mutant TimeNamedDone: a done file
\* named by its writer (as a time-stamped name would be), not its node.
Name(e) ==
  CASE e.k = "snap" -> <<"snap", <<>>, e.t, e.v, 0, "-">>
    [] e.k = "splice" -> <<"splice", e.node, e.t, 0, e.ord, "-">>
    [] e.k = "done" /\ Mutant = "TimeNamedDone" -> <<"done", e.node, "-", 0, 0, e.w>>
    [] OTHER -> <<e.k, e.node, "-", 0, 0, "-">>

VARIABLES
  files,    \* clone -> its plan files in listing (arrival) order
  rcpt,     \* clone -> subject versions with a passing receipt
  subj,     \* clone -> the subject's present bytes (a version)
  tver,     \* template -> its present version
  lock,     \* clone -> the session holding its write lock, or "none"
  pc,       \* session -> "idle" | "planned" | "locked"
  want,     \* session -> what it decided: [op, node]
  pending,  \* session -> files still to publish, in order
  writes,   \* session -> decisions made
  crashes,
  againReq, \* history: <<parent, template, ordinal>> asked for with --again
  view      \* derived, never read by a guard's own decision: each clone's fold
vars == <<files, rcpt, subj, tver, lock, pc, want, pending, writes, crashes, againReq, view>>

FS(c) == Range(files[c])

----------------------------------------------------------------------------
(* The fold, over a world W = [files, tver, subj, rcpt]. It is computed once
   per step into the derived variable `view` (a function of the other
   variables, so it adds no states): TLC would otherwise fold the same files
   dozens of times per state. *)

Now == [files |-> files, tver |-> tver, subj |-> subj, rcpt |-> rcpt]
Nxt == [files |-> files', tver |-> tver', subj |-> subj', rcpt |-> rcpt']

WS(W, c) == Range(W.files[c])
HasSnapW(W, c, e) == SnapEv(e.t, e.v) \in WS(W, c)
StepsOf(W, e) == IF Mutant = "FoldFromLiveTemplate" THEN Steps(e.t, W.tver[e.t])
                 ELSE Steps(e.t, e.v)
Valid(W, c, e) == e.k = "splice" /\ (HasSnapW(W, c, e) \/ Mutant = "FoldFromLiveTemplate")

RECURSIVE Pick(_, _, _, _)
Pick(W, c, p, i) ==
  IF i > Len(KeyOrder) THEN <<>>
  ELSE LET hits == {e \in WS(W, c) : Valid(W, c, e) /\ e.node = p
                                     /\ e.ord = KeyOrder[i][1] /\ e.t = KeyOrder[i][2]}
       IN (IF hits = {} THEN <<>> ELSE <<CHOOSE e \in hits : TRUE>>) \o Pick(W, c, p, i + 1)

\* The valid splices under p: in key order, or (mutant) in listing order.
Kids(W, c, p) ==
  IF Mutant = "FoldByListingOrder"
    THEN SelectSeq(W.files[c], LAMBDA e : Valid(W, c, e) /\ e.node = p)
    ELSE Pick(W, c, p, 1)

Under(W, sp) ==
  LET st == StepsOf(W, sp)
  IN [i \in 1..Len(st) |-> Append(sp.node, <<sp.t, sp.ord, st[i]>>)]

\* Post-order: each node's children, then the node.
RECURSIVE Expand(_, _, _, _)
Expand(W, c, p, d) ==
  LET ks == Kids(W, c, p)
  IN Cat([j \in 1..Len(ks) |->
        LET ns == Under(W, ks[j])
        IN Cat([i \in 1..Len(ns) |->
              (IF d < 2 THEN Expand(W, c, ns[i], d + 1) ELSE <<>>) \o <<ns[i]>>])])

FoldW(W, c) == Expand(W, c, <<>>, 0)

DoneNodesW(W, c) == {e.node : e \in {f \in WS(W, c) : f.k = "done"}}
SkipNodesW(W, c) == {e.node : e \in {f \in WS(W, c) : f.k = "skip"}}
Pos(F, n) == CHOOSE i \in 1..Len(F) : F[i] = n

\* Mutant LastEventWins: a node done and skipped is whichever event was
\* listed last.
SkipListedAfterDone(W, c, n) ==
  LET fl == W.files[c]
  IN \E i, j \in 1..Len(fl) :
       i < j /\ fl[i].k = "done" /\ fl[j].k = "skip" /\ fl[i].node = n /\ fl[j].node = n
DoneSetOf(W, c, F, dn) ==
  {n \in Range(F) : n \in dn /\ ~(Mutant = "LastEventWins" /\ SkipListedAfterDone(W, c, n))}

\* A gate is met when a passing receipt is about the present bytes.
MetW(W, c, n) == ~Gated(StepOf(n)) \/ W.subj[c] \in W.rcpt[c]

\* The first index of F satisfying open (0 when none).
First(F, open(_)) ==
  IF \E i \in 1..Len(F) : open(F[i])
    THEN CHOOSE i \in 1..Len(F) : open(F[i]) /\ \A j \in 1..i - 1 : ~open(F[j])
    ELSE 0

\* The current node's index (0: every node closed) as computed
\* (writfold_core::plan::current), or by a mutant: CurrentMovesBack reopens
\* a done node whose gate is no longer met (stale); CurrentAfterLastDone
\* takes the node after the last done one.
CurOf(W, c, F, dn, closed) ==
  CASE Mutant = "CurrentMovesBack" ->
         First(F, LAMBDA n : n \notin closed \/ (n \in dn /\ ~MetW(W, c, n)))
    [] Mutant = "CurrentAfterLastDone" ->
         LET ds == {i \in 1..Len(F) : F[i] \in dn}
             k == IF ds = {} THEN 1 ELSE Max(ds) + 1
         IN IF k <= Len(F) THEN k ELSE 0
    [] OTHER -> First(F, LAMBDA n : n \notin closed)

ViewOf(W, c) ==
  LET F == FoldW(W, c)
      dn == DoneNodesW(W, c)
      sn == SkipNodesW(W, c)
      closed == dn \cup sn
  IN [fold |-> F, dn |-> dn, sn |-> sn, done |-> DoneSetOf(W, c, F, dn),
      cur |-> CurOf(W, c, F, dn, closed),
      first |-> First(F, LAMBDA n : n \notin closed)]   \* as specified

Views(W) == [c \in Clones |-> ViewOf(W, c)]

Fold(c) == view[c].fold
DoneNodes(c) == view[c].dn
SkipNodes(c) == view[c].sn
DoneSet(c) == view[c].done
Closed(c, n) == n \in DoneNodes(c) \cup SkipNodes(c)
Met(c, n) == MetW(Now, c, n)
FirstOpen(c) == view[c].first
Cur(c) == view[c].cur
CurNode(c) == Fold(c)[Cur(c)]

----------------------------------------------------------------------------
(* Sessions: decide outside the lock *)

Free(s) == pc[s] = "idle" /\ writes[s] < MaxWrites

Decide(s, op, node, files2) ==
  /\ pc' = [pc EXCEPT ![s] = "planned"]
  /\ want' = [want EXCEPT ![s] = [op |-> op, node |-> node,
                                   ord |-> IF op = "use" THEN files2[2].ord ELSE 0]]
  /\ pending' = [pending EXCEPT ![s] = files2]
  /\ writes' = [writes EXCEPT ![s] = @ + 1]

\* Snapshot first, then the splice (mutant: the other way round).
WithSnap(snap, sp) ==
  IF Mutant = "SpliceBeforeSnapshot" THEN <<sp, snap>> ELSE <<snap, sp>>

\* plan set <t>: only when the run has no plan (the same template with the
\* same bytes again writes nothing; another is refused: no step either way).
PlanSet(s, t) ==
  LET c == Home[s]
  IN /\ Free(s)
     /\ \A e \in FS(c) : e.k /= "splice"
     /\ Decide(s, "set", <<>>, WithSnap(SnapEv(t, tver[t]), SpliceEv(<<>>, t, tver[t], 1, s)))
     /\ UNCHANGED <<files, rcpt, subj, tver, lock, crashes, againReq>>

\* plan use <t> [--again] at the current node N. The key (t, digest) is
\* looked up under N, its ancestors and the root (24.1); the ordinal is the
\* next under N. Mutant NoIdempotence: no key lookup.
Ancestors(n) == {SubSeq(n, 1, k) : k \in 0..Len(n)}
NextOrd(c, n) == Max({f.ord : f \in {g \in FS(c) : g.k = "splice" /\ g.node = n}} \cup {0}) + 1
PlanUse(s, t, again) ==
  LET c == Home[s]
      n == CurNode(c)
      o == NextOrd(c, n)
      spliced == \E f \in FS(c) : f.k = "splice" /\ f.node \in Ancestors(n)
                                  /\ f.t = t /\ f.v = tver[t]
  IN /\ Splicing /\ Free(s) /\ Cur(c) /= 0
     /\ Len(n) <= 1                      \* the model's nesting bound
     /\ again \/ ~spliced \/ Mutant = "NoIdempotence"   \* else nothing written
     /\ o <= MaxOrd
     /\ Decide(s, "use", n, WithSnap(SnapEv(t, tver[t]), SpliceEv(n, t, tver[t], o, s)))
     /\ againReq' = IF again THEN againReq \cup {<<n, t, o>>} ELSE againReq
     /\ UNCHANGED <<files, rcpt, subj, tver, lock, crashes>>

\* plan advance: the current node's gate judged now. Its evidence is the
\* newest passing receipt about the present bytes; mutants: advance anyway
\* naming an older receipt (AdvanceIgnoresGates), name none (DoneWithoutEvidence).
PlanAdvance(s) ==
  LET c == Home[s]
      n == CurNode(c)
      met == Met(c, n)
      ev == CASE ~Gated(StepOf(n)) -> {}
              [] Mutant = "DoneWithoutEvidence" -> {}
              [] met -> {<<subj[c], subj[c]>>}
              [] rcpt[c] = {} -> {}
              [] OTHER -> {<<Max(rcpt[c]), subj[c]>>}
  IN /\ Free(s) /\ Cur(c) /= 0
     /\ met \/ Mutant = "AdvanceIgnoresGates"
     /\ Decide(s, "advance", n, <<DoneEv(n, ev, s)>>)
     /\ UNCHANGED <<files, rcpt, subj, tver, lock, crashes, againReq>>

PlanSkip(s) ==
  LET c == Home[s]
  IN /\ Free(s) /\ Cur(c) /= 0
     /\ Decide(s, "skip", CurNode(c), <<SkipEv(CurNode(c), s)>>)
     /\ UNCHANGED <<files, rcpt, subj, tver, lock, crashes, againReq>>

----------------------------------------------------------------------------
(* Sessions: under the lock *)

Release(s) ==
  /\ pc' = [pc EXCEPT ![s] = "idle"]
  /\ pending' = [pending EXCEPT ![s] = <<>>]

\* No other bytes under any name this session will create.
NamesFree(c, es) == \A e \in Range(es) : e \in FS(c) \/ \A f \in FS(c) : Name(f) /= Name(e)

Rechecked(s) ==
  LET c == Home[s]
  IN CASE Recheck = "none" -> TRUE
       [] want[s].op = "set" -> \A e \in FS(c) : e.k /= "splice"
       [] want[s].op = "use" ->
            Cur(c) /= 0 /\ CurNode(c) = want[s].node
            /\ (Recheck = "current" \/ NextOrd(c, want[s].node) = want[s].ord)
       [] OTHER -> Cur(c) /= 0 /\ CurNode(c) = want[s].node

\* Take the lock and check every guard before anything is published
\* (atomic::apply_held); a failed guard is a conflict, nothing written.
TakeLock(s) ==
  LET c == Home[s]
  IN /\ pc[s] = "planned" /\ lock[c] = "none"
     /\ IF Rechecked(s) /\ NamesFree(c, pending[s])
          THEN /\ lock' = [lock EXCEPT ![c] = s]
               /\ pc' = [pc EXCEPT ![s] = "locked"]
               /\ UNCHANGED pending
          ELSE Release(s) /\ UNCHANGED lock
     /\ UNCHANGED <<files, rcpt, subj, tver, want, writes, crashes, againReq>>

\* Publish the next file create-new (a snapshot already there with its
\* bytes is skipped); a name taken since the check (a merge) is a conflict.
Publish(s) ==
  LET c == Home[s]
      e == Head(pending[s])
      rest == Tail(pending[s])
      clash == e \notin FS(c) /\ \E f \in FS(c) : Name(f) = Name(e)
  IN /\ pc[s] = "locked"
     /\ files' = IF e \in FS(c) \/ clash THEN files
                 ELSE [files EXCEPT ![c] = Append(@, e)]
     /\ IF clash \/ rest = <<>>
          THEN Release(s) /\ lock' = [lock EXCEPT ![c] = "none"]
          ELSE pending' = [pending EXCEPT ![s] = rest] /\ UNCHANGED <<pc, lock>>
     /\ UNCHANGED <<rcpt, subj, tver, want, writes, crashes, againReq>>

----------------------------------------------------------------------------
(* Environment *)

\* Pull d's files and receipts into c; Git stops on other bytes under one name.
\* Premise: a merge runs between Writfold invocations in c, never while one
\* of c's sessions is between its decision and its last publish.
Merge(c, d) ==
  /\ c /= d
  /\ \A s \in Sessions : Home[s] = c => pc[s] = "idle"
  /\ \A e \in FS(d), f \in FS(c) : Name(e) = Name(f) => e = f
  /\ ~(FS(d) \subseteq FS(c)) \/ ~(rcpt[d] \subseteq rcpt[c])
  /\ files' = [files EXCEPT ![c] = @ \o SelectSeq(files[d], LAMBDA e : e \notin FS(c))]
  /\ rcpt' = [rcpt EXCEPT ![c] = @ \cup rcpt[d]]
  /\ UNCHANGED <<subj, tver, lock, pc, want, pending, writes, crashes, againReq>>

\* A gate run passes on the present bytes and writes a receipt.
Receipt(c) ==
  /\ subj[c] \notin rcpt[c]
  /\ rcpt' = [rcpt EXCEPT ![c] = @ \cup {subj[c]}]
  /\ UNCHANGED <<files, subj, tver, lock, pc, want, pending, writes, crashes, againReq>>

\* The subject's bytes move: receipts about the old bytes go stale.
Drift(c) ==
  /\ subj[c] < MaxDrift
  /\ subj' = [subj EXCEPT ![c] = @ + 1]
  /\ UNCHANGED <<files, rcpt, tver, lock, pc, want, pending, writes, crashes, againReq>>

\* A template is edited mid-run (a new digest).
Edit(t) ==
  /\ tver[t] = 1
  /\ tver' = [tver EXCEPT ![t] = 2]
  /\ UNCHANGED <<files, rcpt, subj, lock, pc, want, pending, writes, crashes, againReq>>

\* Every session of clone c dies between two steps; published files stay.
Crash(c) ==
  /\ crashes < MaxCrashes
  /\ \E s \in Sessions : Home[s] = c /\ pc[s] /= "idle"
  /\ crashes' = crashes + 1
  /\ pc' = [s \in Sessions |-> IF Home[s] = c THEN "idle" ELSE pc[s]]
  /\ pending' = [s \in Sessions |-> IF Home[s] = c THEN <<>> ELSE pending[s]]
  /\ lock' = [lock EXCEPT ![c] = "none"]
  /\ UNCHANGED <<files, rcpt, subj, tver, want, writes, againReq>>

Init ==
  /\ files = [c \in Clones |-> <<>>]
  /\ rcpt = [c \in Clones |-> {}]
  /\ subj = [c \in Clones |-> 0]
  /\ tver = [t \in Templates |-> 1]
  /\ lock = [c \in Clones |-> "none"]
  /\ pc = [s \in Sessions |-> "idle"]
  /\ want = [s \in Sessions |-> [op |-> "none", node |-> <<>>, ord |-> 0]]
  /\ pending = [s \in Sessions |-> <<>>]
  /\ writes = [s \in Sessions |-> 0]
  /\ crashes = 0
  /\ againReq = {}
  /\ view = Views(Now)

Step ==
  \/ \E s \in Sessions :
       \/ \E t \in Templates : PlanSet(s, t) \/ PlanUse(s, t, FALSE) \/ PlanUse(s, t, TRUE)
       \/ PlanAdvance(s) \/ PlanSkip(s) \/ TakeLock(s) \/ Publish(s)
  \/ \E c, d \in Clones : Merge(c, d)
  \/ \E c \in Clones : Receipt(c) \/ Drift(c) \/ Crash(c)
  \/ \E t \in Templates : Edit(t)

Next == Step /\ view' = Views(Nxt)

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Properties *)

TypeOK ==
  /\ \A c \in Clones : subj[c] \in 0..MaxDrift /\ rcpt[c] \subseteq 0..MaxDrift
  /\ \A c \in Clones : lock[c] \in Sessions \cup {"none"}
  /\ \A s \in Sessions : pc[s] \in {"idle", "planned", "locked"}
  /\ crashes \in 0..MaxCrashes

\* 1. At most one done record per node.
OneDonePerNode ==
  \A c \in Clones : \A n \in DoneNodes(c) :
    Cardinality({e \in FS(c) : e.k = "done" /\ e.node = n}) = 1

\* 2. Every done record names, for a gated node, a receipt that is in the
\* same clone, so the record stands on files alone.
EvidenceNamed ==
  \A c \in Clones : \A e \in FS(c) :
    (e.k = "done" /\ Gated(StepOf(e.node))) =>
      (e.ev /= {} /\ \A x \in e.ev : x[1] \in rcpt[c])

\* 3. An advance was taken only when the gate was met: the receipt it names
\* is about the bytes it judged (true then; `!stale` may show it is not now).
AdvanceOnlyWhenMet ==
  \A c \in Clones : \A e \in FS(c) : e.k = "done" => \A x \in e.ev : x[1] = x[2]

\* 4. Every splice has its snapshot, so the graph folds from the run's files.
SnapshotsExist ==
  \A c \in Clones : \A e \in FS(c) : e.k = "splice" => HasSnapW(Now, c, e)

\* 5. The fold is a function of the set of files: two clones holding the
\* same set agree on the steps, the done set and the current node.
FoldOrderIndependent ==
  \A c, d \in Clones :
    FS(c) = FS(d) =>
      /\ Fold(c) = Fold(d) /\ DoneSet(c) = DoneSet(d) /\ Cur(c) = Cur(d)

\* 6. The current node is the first neither done nor skipped.
CurrentIsFirstOpen == \A c \in Clones : Cur(c) = FirstOpen(c)

\* 7. A splice is idempotent on its key (24.1): a splice not made with
\* --again never repeats a (template, digest) already spliced under its
\* parent, one of the parent's ancestors or the root. "Already" is the
\* listing order, which is the order of writing for files written in one
\* clone. SpliceIdempotentLocal compares splices written in one clone;
\* SpliceIdempotent any two, which a merge can break (probe config).
Again(e) == <<e.node, e.t, e.ord>> \in againReq
Repeats(c, local) ==
  LET fl == files[c] IN
  \E i, j \in 1..Len(fl) :
    /\ i < j /\ fl[i].k = "splice" /\ fl[j].k = "splice"
    /\ fl[i].t = fl[j].t /\ fl[i].v = fl[j].v
    /\ fl[i].node \in Ancestors(fl[j].node)
    /\ ~Again(fl[j])
    /\ local => Home[fl[i].w] = Home[fl[j].w]
SpliceIdempotentLocal == \A c \in Clones : ~Repeats(c, TRUE)
SpliceIdempotent == \A c \in Clones : ~Repeats(c, FALSE)

\* 8. Within a clone, no node is done while an earlier node is open. Holds
\* when one clone is written; after a merge it can fail (probe configs).
DonePrefixAt(c) ==
  LET F == Fold(c) IN
  \A i, j \in 1..Len(F) : (i < j /\ F[j] \in DoneNodes(c)) => Closed(c, F[i])
DonePrefix == \A c \in Clones : DonePrefixAt(c)

\* The same, for a clone whose run has one root plan: what is left after two
\* clones each set a different plan on one run (probe-LateSplice).
OneRoot(c) == Cardinality({e \in FS(c) : e.k = "splice" /\ e.node = <<>>}) <= 1
DonePrefixOneRoot == \A c \in Clones : OneRoot(c) => DonePrefixAt(c)

Invariants ==
  /\ TypeOK /\ OneDonePerNode /\ EvidenceNamed /\ AdvanceOnlyWhenMet
  /\ SnapshotsExist /\ FoldOrderIndependent /\ CurrentIsFirstOpen /\ SpliceIdempotentLocal

\* Action properties, over every step.

\* 9. The done set only grows.
DoneOnlyGrows == [][\A c \in Clones : DoneSet(c) \subseteq view'[c].done]_vars

\* 10. Existing nodes are kept, in their order, by every step (a merge
\* included); and every step but a merge into c keeps c's fold up to its
\* current node unchanged (a splice adds only before the current node).
\* Nodes are distinct within a fold, so F is a subsequence of G when G
\* filtered to F's nodes is F.
Subsequence(F, G) ==
  /\ Range(F) \subseteq Range(G)
  /\ SelectSeq(G, LAMBDA n : n \in Range(F)) = F
Prefix(F, i) == IF i = 0 THEN F ELSE SubSeq(F, 1, i - 1)
\* A merge into c: c gains a file another clone's session wrote.
MergedInto(c) ==
  \E e \in Range(files'[c]) \ FS(c) : e.w /= "-" /\ Home[e.w] /= c
Kept(c) ==
  LET F == Fold(c)
      G == view'[c].fold
      P == Prefix(F, Cur(c))
  IN /\ Subsequence(F, G)
     /\ \/ MergedInto(c)
        \/ Len(P) <= Len(G) /\ P = SubSeq(G, 1, Len(P))
SplicePreserves == [][\A c \in Clones : Kept(c)]_vars

\* 11. The current node never moves back to a node that was already in the
\* fold before it (or at all, once complete): stale is shown, never reopened.
Back(c) ==
  LET F == Fold(c)
      i == Cur(c)
      G == view'[c].fold
      j == view'[c].cur
      n == G[j]
  IN /\ j /= 0
     /\ n \in Range(F)
     /\ (i = 0 \/ Pos(F, n) < i)
CurrentNeverBack == [][\A c \in Clones : ~Back(c)]_vars

\* Reachability witnesses (expected violated: each names a state the
\* positive configuration must reach).
NoMergeEverAgrees == ~(\E c, d \in Clones : c /= d /\ FS(c) = FS(d) /\ \E e \in FS(c) : e.k = "done")
NoNestedDone == ~(\E c \in Clones : \E n \in DoneNodes(c) : Len(n) = 2)
=============================================================================
