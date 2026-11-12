------------------------------- MODULE KbWrite -------------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC

(***************************************************************************
The KB v1 write protocol and the derived index that reads it
(docs/writfold/kb-v1-design.md sections 4.3, 8.4 and 13;
crates/writfold-kb/src/write/atomic.rs and src/index/).

Files. Two tracked KB files: F, which writers replace (a checkpoint append
or a front-matter change: new bytes computed from the bytes read), and G,
which a writer creates new (a new record). A file is a directory entry
naming an inode; an inode holds a content, a sequence of entries.

Writers. w1 and w2 append to F; w3 creates G. Each plans first, reading F
outside the lock (as the verb plans from the index and the file before
`atomic::apply`), then under the per-KB lock: writes a temporary inode,
syncs it, re-checks that F still holds the bytes it read (else a
conflict, exit 3), publishes by rename (F) or by link, which fails when the
name exists (G), syncs the directory, and unlocks. A writer's entry counts
as acknowledged once the directory sync returned.

Crash. A power loss at any moment (at most MaxCrashes): every visible
directory entry and inode content reverts to its durable state (an
unsynced temporary inode holds junk, `Partial`), every process dies and
the lock is released. The SQLite index commits atomically and durably
(trusted premise), so it survives.

Index. A refresher observes the snapshot (F and G as Git status and the
digests show them), reads the files (Phase A, possibly later), re-observes
(the pin re-check of section 13), and commits, in one transaction, the pin
it observed, the Phase A facts it read and Phase B recomputed from all
Phase A facts. A reader observes the snapshot and, when the stored pin
equals it, reports `index=fresh` with the stored rows.

Mutants (constants, each FALSE in exactly one configuration): Recheck (the
writer's digest re-check), SyncFirst (sync the temporary before publishing),
PhaseB (recompute Phase B on refresh), PinRecheck (re-observe before the
index commit), Lock (the per-KB write lock).

Trusted premises: rename and link are atomic; fsync makes an inode's
content or a directory's entries durable; SQLite transactions are atomic
and durable; the content digest identifies content. Not modelled: other
tools editing F without the lock (the re-check narrows that window, it
does not close it), a user reverting a file to earlier bytes between two
observations of one refresher (ABA), more than two files, Git commits
(HEAD is part of the observed snapshot exactly like the dirty files).
***************************************************************************)
CONSTANTS MaxCrashes, Recheck, SyncFirst, PhaseB, PinRecheck, Lock
CONSTANTS None, Partial

Appenders == {"w1", "w2"}
Creator == "w3"
Writers == Appenders \cup {Creator}
Refreshers == {"r1", "r2"}
Names == {"F", "G"}
Attempts(w) == IF w = Creator THEN 2 ELSE 1
Origin == <<"f0", 0>>  \* the inode F starts with
Inodes == {Origin} \cup {<<w, k>> : w \in Writers, k \in 1..2}
Name(w) == IF w = Creator THEN "G" ELSE "F"

VARIABLES
  dir, ddir,        \* visible and durable directory entries: name -> inode or None
  data, ddata,      \* visible and durable inode contents
  pc, base, lock,   \* writers
  attempts, acked,  \* attempts made; acknowledged entries
  rpc, robs, rread, \* refreshers: step, observed snapshot, Phase A read
  ix,               \* the stored index: [pin, a (Phase A), b (Phase B)]
  qpc, qobs, report,\* the reader and its last fresh report
  crashes

vars == <<dir, ddir, data, ddata, pc, base, lock, attempts, acked,
          rpc, robs, rread, ix, qpc, qobs, report, crashes>>

\* A writer's current entry (and the temporary inode that carries it).
Entry(w) == <<w, attempts[w]>>

\* What a snapshot shows of each file: its content, or None when absent.
Snap == [n \in Names |-> IF dir[n] = None THEN None ELSE data[dir[n]]]
\* Phase B: a function of every Phase A fact.
B(a) == <<a["F"], a["G"]>>
Initial == [F |-> <<>>, G |-> None]

Init ==
  /\ dir = [n \in Names |-> IF n = "F" THEN Origin ELSE None]
  /\ ddir = dir
  /\ data = [i \in Inodes |-> IF i = Origin THEN <<>> ELSE None]
  /\ ddata = data
  /\ pc = [w \in Writers |-> "idle"]
  /\ base = [w \in Writers |-> None]
  /\ lock = None
  /\ attempts = [w \in Writers |-> 0]
  /\ acked = {}
  /\ rpc = [r \in Refreshers |-> "idle"]
  /\ robs = [r \in Refreshers |-> None]
  /\ rread = [r \in Refreshers |-> None]
  /\ ix = [pin |-> Initial, a |-> Initial, b |-> B(Initial)]
  /\ qpc = "idle"
  /\ qobs = None
  /\ report = None
  /\ crashes = 0

Index == <<rpc, robs, rread, ix, qpc, qobs, report, crashes>>

\* Plan: read F (appenders) outside the lock.
Plan(w) ==
  /\ pc[w] = "idle" /\ attempts[w] < Attempts(w)
  /\ attempts' = [attempts EXCEPT ![w] = @ + 1]
  /\ base' = [base EXCEPT ![w] = IF w = Creator THEN None ELSE data[dir["F"]]]
  /\ pc' = [pc EXCEPT ![w] = "planned"]
  /\ UNCHANGED <<dir, ddir, data, ddata, lock, acked>> /\ UNCHANGED Index

TakeLock(w) ==
  /\ pc[w] = "planned"
  /\ IF Lock THEN lock = None /\ lock' = w ELSE UNCHANGED lock
  /\ pc' = [pc EXCEPT ![w] = "locked"]
  /\ UNCHANGED <<dir, ddir, data, ddata, base, attempts, acked>> /\ UNCHANGED Index

\* The new content in a temporary inode; not yet durable.
WriteTemp(w) ==
  /\ pc[w] = "locked"
  /\ LET t == Entry(w)
         new == IF w = Creator THEN <<Entry(w)>> ELSE Append(base[w], Entry(w))
     IN /\ data' = [data EXCEPT ![t] = new]
        /\ ddata' = [ddata EXCEPT ![t] = Partial]
  /\ pc' = [pc EXCEPT ![w] = "written"]
  /\ UNCHANGED <<dir, ddir, base, lock, attempts, acked>> /\ UNCHANGED Index

SyncTemp(w) ==
  /\ pc[w] = IF SyncFirst THEN "written" ELSE "dirsynced"
  /\ ddata' = [ddata EXCEPT ![Entry(w)] = data[Entry(w)]]
  /\ pc' = [pc EXCEPT ![w] = IF SyncFirst THEN "synced" ELSE "done"]
  /\ acked' = IF SyncFirst THEN acked ELSE acked \cup {Entry(w)}
  /\ UNCHANGED <<dir, ddir, data, base, lock, attempts>> /\ UNCHANGED Index

\* The re-check: F still holds what was read; else a conflict.
CheckBase(w) ==
  /\ pc[w] = IF SyncFirst THEN "synced" ELSE "written"
  /\ IF w = Creator \/ ~Recheck \/ data[dir["F"]] = base[w]
       THEN pc' = [pc EXCEPT ![w] = "checked"] /\ UNCHANGED lock
       ELSE pc' = [pc EXCEPT ![w] = "idle"] /\ lock' = IF Lock THEN None ELSE lock
  /\ UNCHANGED <<dir, ddir, data, ddata, base, attempts, acked>> /\ UNCHANGED Index

\* Rename over F, or link G (refused when G exists).
Publish(w) ==
  /\ pc[w] = "checked"
  /\ IF w = Creator /\ dir["G"] /= None
       THEN /\ pc' = [pc EXCEPT ![w] = "idle"]
            /\ lock' = IF Lock THEN None ELSE lock
            /\ UNCHANGED dir
       ELSE /\ dir' = [dir EXCEPT ![Name(w)] = Entry(w)]
            /\ pc' = [pc EXCEPT ![w] = "published"]
            /\ UNCHANGED lock
  /\ UNCHANGED <<ddir, data, ddata, base, attempts, acked>> /\ UNCHANGED Index

SyncDir(w) ==
  /\ pc[w] = "published"
  /\ ddir' = [ddir EXCEPT ![Name(w)] = dir[Name(w)]]
  /\ pc' = [pc EXCEPT ![w] = IF SyncFirst THEN "done" ELSE "dirsynced"]
  /\ acked' = IF SyncFirst THEN acked \cup {Entry(w)} ELSE acked
  /\ UNCHANGED <<dir, data, ddata, base, lock, attempts>> /\ UNCHANGED Index

Unlock(w) ==
  /\ pc[w] = "done"
  /\ lock' = IF Lock THEN None ELSE lock
  /\ pc' = [pc EXCEPT ![w] = "idle"]
  /\ UNCHANGED <<dir, ddir, data, ddata, base, attempts, acked>> /\ UNCHANGED Index

Files == <<dir, ddir, data, ddata, pc, base, lock, attempts, acked>>

Observe(r) ==
  /\ rpc[r] = "idle" /\ Snap /= ix.pin
  /\ robs' = [robs EXCEPT ![r] = Snap]
  /\ rpc' = [rpc EXCEPT ![r] = "observed"]
  /\ UNCHANGED <<rread, ix, qpc, qobs, report, crashes>> /\ UNCHANGED Files

ReadFiles(r) ==
  /\ rpc[r] = "observed"
  /\ rread' = [rread EXCEPT ![r] = Snap]
  /\ rpc' = [rpc EXCEPT ![r] = "read"]
  /\ UNCHANGED <<robs, ix, qpc, qobs, report, crashes>> /\ UNCHANGED Files

\* One transaction: re-observe, then store pin, Phase A and Phase B.
Commit(r) ==
  /\ rpc[r] = "read"
  /\ IF PinRecheck /\ Snap /= robs[r]
       THEN UNCHANGED ix
       ELSE ix' = [pin |-> robs[r], a |-> rread[r],
                   b |-> IF PhaseB THEN B(rread[r]) ELSE ix.b]
  /\ rpc' = [rpc EXCEPT ![r] = "idle"]
  /\ UNCHANGED <<robs, rread, qpc, qobs, report, crashes>> /\ UNCHANGED Files

ReaderObserve ==
  /\ qpc = "idle"
  /\ qobs' = Snap /\ qpc' = "observed"
  /\ UNCHANGED <<rpc, robs, rread, ix, report, crashes>> /\ UNCHANGED Files

\* `index=fresh`: the stored pin equals the observed snapshot.
ReaderRead ==
  /\ qpc = "observed"
  /\ report' = IF ix.pin = qobs THEN [pin |-> qobs, view |-> ix] ELSE report
  /\ qpc' = "idle"
  /\ UNCHANGED <<rpc, robs, rread, ix, qobs, crashes>> /\ UNCHANGED Files

Crash ==
  /\ crashes < MaxCrashes
  /\ crashes' = crashes + 1
  /\ dir' = ddir /\ data' = ddata
  /\ pc' = [w \in Writers |-> "idle"]
  /\ lock' = None
  /\ rpc' = [r \in Refreshers |-> "idle"]
  /\ qpc' = "idle"
  /\ UNCHANGED <<ddir, ddata, base, attempts, acked, robs, rread, ix, qobs, report>>

Next ==
  \/ \E w \in Writers :
       Plan(w) \/ TakeLock(w) \/ WriteTemp(w) \/ SyncTemp(w) \/ CheckBase(w)
       \/ Publish(w) \/ SyncDir(w) \/ Unlock(w)
  \/ \E r \in Refreshers : Observe(r) \/ ReadFiles(r) \/ Commit(r)
  \/ ReaderObserve \/ ReaderRead
  \/ Crash

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
TypeOK ==
  /\ \A n \in Names : dir[n] \in Inodes \cup {None}
  /\ lock \in Writers \cup {None}
  /\ acked \subseteq Inodes
  /\ crashes \in 0..MaxCrashes

\* A published record is complete: no visible file holds junk.
PublishedComplete ==
  \A n \in Names : dir[n] /= None => data[dir[n]] /= Partial

\* No acknowledged write is lost: its entry is in the file it wrote.
AckedSurvive ==
  \A e \in acked :
    /\ dir[Name(e[1])] /= None
    /\ data[dir[Name(e[1])]] /= Partial
    /\ \E i \in 1..Len(data[dir[Name(e[1])]]) : data[dir[Name(e[1])]][i] = e

\* Create-new never overwrites: G holds exactly the first acknowledged entry.
CreateNewNeverOverwrites ==
  \A e \in acked : e[1] = Creator => data[dir["G"]] = <<e>>

\* The stored index is the derivation of the pin it names.
IndexIsDerivation == ix.a = ix.pin /\ ix.b = B(ix.a)

\* A reader that reports index=fresh at pin P sees exactly the derivation
\* of P, never a mix of generations.
FreshIsDerivation ==
  report /= None => (report.view.a = report.pin /\ report.view.b = B(report.pin))

Safety ==
  /\ PublishedComplete /\ AckedSurvive /\ CreateNewNeverOverwrites
  /\ IndexIsDerivation /\ FreshIsDerivation
=============================================================================
