-------------------------- MODULE TrialEditStream --------------------------
EXTENDS Naturals, Sequences, FiniteSets

(***************************************************************************
One run of `writfold-trial-worktree/1` (docs/writfold/trial-profile.md): the
ledger stream after `Prepared` and `Approved`, the worktree file it edits, and
the actors that touch them.

Actors: independent `writfold trial apply` invocations (`Applies`, model
values), one `writfold trial recover`, and an external writer that may
overwrite the
worktree file at any step boundary. Every apply and recover invocation may
crash between any two of its steps (the `Claimed` append, the preimage
observation, `stage_replacement`, `install` and the outcome append); a crash
releases the writer lease and the file lock, and a later invocation starts
afresh from the durable stream and file.

Trusted effect premises, as in M01: a ledger append is a completed Git ref
CAS against the expected old head, atomic and conditional, and the edit-flow
validator (writfold_core::flow, proved in proofs/Writfold/Flow.lean) admits
only Prepared, Approved, Claimed, then one of Applied/Abandoned. The writer
lease (a held flock) is exclusive while its holder lives. `install` re-checks
that the original still matches the observed preimage and renames, as one
step: the external writer is excluded from that window (the staged primitive
assumes no uncoordinated writer inside it; it is not a hostile-race proof).
The external writer writes bytes distinct from both source and candidate
(ExternalValues); an ABA writer that restores either is outside the trial
profile and is exercised only by the ABA probe.

Authority is checked at `authorize`, immediately before the claim, against a
clock. A claim made under valid authority may complete its install after the
approval expires: a claim is never re-dispatched, so the install that follows
is the one dispatch the authority admitted (explicit decision; the
InstallAfterExpiry probe shows it is reachable). The claim is the commitment
point: authority is not re-checked at install or at the outcome append, and
`recover` needs none because it never writes the file. Revocation after the
claim likewise does not stop that one install.

Not modelled: Prepared/Approved themselves (the run starts approved), the
candidate binding beyond the single prepared candidate (proved over the Rust
validator), stage-file cleanup, power loss and hostile same-user writers.
***************************************************************************)
CONSTANTS Applies, ExternalValues, MaxTime, Expiry
CONSTANT None  \* a model value: absent
Recoverer == "recover"
Actors == Applies \cup {Recoverer}
Contents == {"source", "candidate"} \cup ExternalValues
Outcomes == {"Applied", "Abandoned"}
Kinds == {"Prepared", "Approved", "Claimed"} \cup Outcomes
Phases == {"idle", "authorized", "claimed", "observed", "staged", "installed", "done"}
RPhases == {"idle", "applied", "abandoned", "unknown", "done"}

VARIABLES
  log,        \* the run's ledger stream (record kinds, oldest first)
  file,       \* the worktree file's bytes
  lease,      \* holder of the exclusive writer lease, or None
  pc,         \* each apply invocation's step
  seen,       \* each apply invocation's observed preimage
  authAt,     \* clock value at each apply invocation's authorization
  rpc,        \* the recover invocation's step
  rseen,      \* what recover observed
  now,        \* the clock
  \* history variables (observation only, never read by actions' guards):
  dispatches, \* how many times an invocation passed the claim into the effect path
  claimAuth,  \* authorization time of the invocation whose claim was appended
  installed,  \* the candidate was installed by rename
  installAt,  \* clock value at that install
  replaced,   \* the bytes that install replaced
  writers     \* which actors ever wrote the file
vars == <<log, file, lease, pc, seen, authAt, rpc, rseen, now,
          dispatches, claimAuth, installed, installAt, replaced, writers>>

TypeOK ==
  /\ log \in Seq(Kinds) /\ Len(log) <= 4
  /\ file \in Contents
  /\ lease \in Actors \cup {None}
  /\ pc \in [Applies -> Phases]
  /\ seen \in [Applies -> Contents \cup {None}]
  /\ authAt \in [Applies -> 0..MaxTime]
  /\ rpc \in RPhases
  /\ rseen \in Contents \cup {None}
  /\ now \in 0..MaxTime
  /\ dispatches \in 0..2
  /\ claimAuth \in (0..MaxTime) \cup {None}
  /\ installed \in BOOLEAN
  /\ installAt \in (0..MaxTime) \cup {None}
  /\ replaced \in Contents \cup {None}
  /\ writers \subseteq Actors \cup {"external"}

(* The proved edit-flow validator, on kinds (the candidate binding is the
   single prepared candidate here): the record that may follow `l`. *)
FlowOk(l, k) ==
  \/ Len(l) = 0 /\ k = "Prepared"
  \/ Len(l) = 1 /\ k = "Approved"
  \/ Len(l) = 2 /\ k = "Claimed"
  \/ Len(l) = 3 /\ k \in Outcomes
(* A completed Git ref CAS: appends exactly when the head is the expected
   old head and the validator admits the record; otherwise nothing changes. *)
CanAppend(expected, k) == Len(log) = expected /\ FlowOk(log, k)
AppendRecord(k) == log' = Append(log, k)

Init ==
  /\ log = <<"Prepared", "Approved">>
  /\ file = "source"
  /\ lease = None
  /\ pc = [a \in Applies |-> "idle"]
  /\ seen = [a \in Applies |-> None]
  /\ authAt = [a \in Applies |-> 0]
  /\ rpc = "idle"
  /\ rseen = None
  /\ now = 0
  /\ dispatches = 0
  /\ claimAuth = None
  /\ installed = FALSE
  /\ installAt = None
  /\ replaced = None
  /\ writers = {}

Bump(n) == IF n < 2 THEN n + 1 ELSE 2
Release == lease' = None
Finish(a) == /\ pc' = [pc EXCEPT ![a] = "done"] /\ seen' = [seen EXCEPT ![a] = None]
             /\ Release

(* apply: take the lease, load the stream, authorize. A stream of exactly
   Prepared, Approved under valid authority proceeds; a three-record stream
   (claimed, no outcome) refuses and points at `recover`. With
   `resumeClaimed` (calibrated negative) apply instead resumes a claimed run
   and dispatches the effect again. *)
StartCore(a, resumeClaimed) ==
  /\ pc[a] = "idle" /\ lease = None
  /\ lease' = a
  /\ \/ /\ Len(log) = 2 /\ now < Expiry
        /\ pc' = [pc EXCEPT ![a] = "authorized"]
        /\ authAt' = [authAt EXCEPT ![a] = now]
        /\ UNCHANGED <<dispatches, claimAuth>>
     \/ /\ resumeClaimed /\ Len(log) = 3
        /\ pc' = [pc EXCEPT ![a] = "claimed"]
        /\ dispatches' = Bump(dispatches)
        /\ UNCHANGED <<authAt, claimAuth>>
  /\ UNCHANGED <<log, file, seen, rpc, rseen, now, installed, installAt, replaced, writers>>
Start(a) == StartCore(a, FALSE)
MutantResumeClaimed(a) == StartCore(a, TRUE)

(* Refusal at start: anything else (expired, claimed, finished). *)
Refuse(a) ==
  /\ pc[a] = "idle" /\ lease = None
  /\ ~(Len(log) = 2 /\ now < Expiry)
  /\ pc' = [pc EXCEPT ![a] = "done"]
  /\ UNCHANGED <<log, file, lease, seen, authAt, rpc, rseen, now,
                 dispatches, claimAuth, installed, installAt, replaced, writers>>

(* The durable claim: Claimed appended against expected head 2. *)
Claim(a) ==
  /\ pc[a] = "authorized"
  /\ IF CanAppend(2, "Claimed")
       THEN /\ AppendRecord("Claimed")
            /\ pc' = [pc EXCEPT ![a] = "claimed"]
            /\ dispatches' = Bump(dispatches)
            /\ claimAuth' = authAt[a]
            /\ UNCHANGED <<lease, seen>>
       ELSE /\ Finish(a) /\ UNCHANGED <<log, dispatches, claimAuth>>
  /\ UNCHANGED <<file, authAt, rpc, rseen, now, installed, installAt, replaced, writers>>

(* Observe the file under its lock; the preimage guard requires the exact
   source bytes, otherwise the original was never replaced: Abandoned. *)
ObserveCore(a, guard) ==
  /\ pc[a] = "claimed"
  /\ IF guard => file = "source"
       THEN /\ pc' = [pc EXCEPT ![a] = "observed"]
            /\ seen' = [seen EXCEPT ![a] = file]
            /\ UNCHANGED <<log, lease>>
       ELSE /\ IF CanAppend(3, "Abandoned") THEN AppendRecord("Abandoned") ELSE UNCHANGED log
            /\ Finish(a)
  /\ UNCHANGED <<file, authAt, rpc, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
Observe(a) == ObserveCore(a, TRUE)
MutantNoPreimageGuard(a) == ObserveCore(a, FALSE)

(* stage_replacement: success stages the candidate; a refused stage leaves
   the original untouched and records Abandoned. *)
Stage(a) ==
  /\ pc[a] = "observed"
  /\ pc' = [pc EXCEPT ![a] = "staged"]
  /\ UNCHANGED <<log, file, lease, seen, authAt, rpc, rseen, now,
                 dispatches, claimAuth, installed, installAt, replaced, writers>>
StageFails(a) ==
  /\ pc[a] = "observed"
  /\ IF CanAppend(3, "Abandoned") THEN AppendRecord("Abandoned") ELSE UNCHANGED log
  /\ Finish(a)
  /\ UNCHANGED <<file, authAt, rpc, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>

(* install: re-check that the original matches the observed preimage, then
   rename the stage over it (one step). A failed re-check is an error with no
   outcome recorded: the run is left for `recover`. *)
Install(a) ==
  /\ pc[a] = "staged"
  /\ IF file = seen[a]
       THEN /\ file' = "candidate"
            /\ pc' = [pc EXCEPT ![a] = "installed"]
            /\ installed' = TRUE
            /\ installAt' = now
            /\ replaced' = file
            /\ writers' = writers \cup {a}
            /\ UNCHANGED <<lease, seen>>
       ELSE /\ Finish(a) /\ UNCHANGED <<file, installed, installAt, replaced, writers>>
  /\ UNCHANGED <<log, authAt, rpc, rseen, now, dispatches, claimAuth>>

(* The outcome: Applied against expected head 3. *)
RecordApplied(a) ==
  /\ pc[a] = "installed"
  /\ IF CanAppend(3, "Applied") THEN AppendRecord("Applied") ELSE UNCHANGED log
  /\ Finish(a)
  /\ UNCHANGED <<file, authAt, rpc, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>

(* A crash anywhere after start: local state is lost, the lease and file
   lock are released, durable stream and file are kept. *)
CrashApply(a) ==
  /\ pc[a] \notin {"idle", "done"}
  /\ pc' = [pc EXCEPT ![a] = "idle"]
  /\ seen' = [seen EXCEPT ![a] = None]
  /\ lease' = IF lease = a THEN None ELSE lease
  /\ UNCHANGED <<log, file, authAt, rpc, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
(* A finished invocation may be run again by the operator. *)
Rerun(a) ==
  /\ pc[a] = "done"
  /\ pc' = [pc EXCEPT ![a] = "idle"]
  /\ UNCHANGED <<log, file, lease, seen, authAt, rpc, rseen, now,
                 dispatches, claimAuth, installed, installAt, replaced, writers>>

(* recover: under the lease, for a claim with no outcome, observe the file
   (never write it) and decide by exact bytes (writfold_core::trial::recovery). *)
RecoverObserve ==
  /\ rpc = "idle" /\ lease = None /\ Len(log) = 3
  /\ lease' = Recoverer
  /\ rseen' = file
  /\ rpc' = CASE file = "candidate" -> "applied"
              [] file = "source" -> "abandoned"
              [] OTHER -> "unknown"
  /\ UNCHANGED <<log, file, pc, seen, authAt, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
RecoverRecord ==
  /\ rpc \in {"applied", "abandoned"}
  /\ LET k == IF rpc = "applied" THEN "Applied" ELSE "Abandoned" IN
       IF CanAppend(3, k) THEN AppendRecord(k) ELSE UNCHANGED log
  /\ rpc' = "done" /\ Release
  /\ UNCHANGED <<file, pc, seen, authAt, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
(* Unknown refuses: nothing is appended and a person decides. *)
RecoverRefuse ==
  /\ rpc = "unknown"
  /\ rpc' = "done" /\ Release
  /\ UNCHANGED <<log, file, pc, seen, authAt, rseen, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
CrashRecover ==
  /\ rpc \in {"applied", "abandoned", "unknown"}
  /\ rpc' = "idle" /\ rseen' = None
  /\ lease' = IF lease = Recoverer THEN None ELSE lease
  /\ UNCHANGED <<log, file, pc, seen, authAt, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>
RerunRecover ==
  /\ rpc = "done"
  /\ rpc' = "idle" /\ rseen' = None
  /\ UNCHANGED <<log, file, lease, pc, seen, authAt, now, dispatches, claimAuth,
                 installed, installAt, replaced, writers>>

(* The external writer: any time, outside install's re-check/rename step. *)
ExternalWrite ==
  /\ \E v \in ExternalValues : file' = v
  /\ writers' = writers \cup {"external"}
  /\ UNCHANGED <<log, lease, pc, seen, authAt, rpc, rseen, now,
                 dispatches, claimAuth, installed, installAt, replaced>>

Tick ==
  /\ now < MaxTime
  /\ now' = now + 1
  /\ UNCHANGED <<log, file, lease, pc, seen, authAt, rpc, rseen,
                 dispatches, claimAuth, installed, installAt, replaced, writers>>

Next ==
  \/ \E a \in Applies :
       \/ Start(a) \/ Refuse(a) \/ Claim(a) \/ Observe(a) \/ Stage(a)
       \/ StageFails(a) \/ Install(a) \/ RecordApplied(a) \/ CrashApply(a) \/ Rerun(a)
  \/ RecoverObserve \/ RecoverRecord \/ RecoverRefuse \/ CrashRecover \/ RerunRecover
  \/ ExternalWrite \/ Tick

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
Has(k) == \E i \in 1..Len(log) : log[i] = k

(* At most one outcome record, and it is the fourth. *)
OneOutcome ==
  /\ Cardinality({i \in 1..Len(log) : log[i] \in Outcomes}) <= 1
  /\ \A i \in 1..Len(log) : log[i] \in Outcomes => i = 4
(* Applied implies the file held the candidate at install. *)
AppliedMeansInstalled == Has("Applied") => installed
(* Abandoned implies the original was never replaced. *)
AbandonedMeansUntouched == Has("Abandoned") => ~installed
(* No second dispatch after Claimed. *)
NoSecondDispatch == dispatches <= 1 /\ (dispatches = 1 <=> Has("Claimed"))
(* Install only ever replaces the exact source bytes. *)
NoClobber == replaced \in {None, "source"}
(* Recover decides Unknown only on bytes that are neither source nor candidate. *)
UnknownOnlyOnNeither == rpc = "unknown" => rseen \notin {"source", "candidate"}
(* Recover never writes the file. *)
RecoverNeverWrites == Recoverer \notin writers
(* The claim was made under valid authority. *)
ClaimUnderAuthority == Has("Claimed") => claimAuth # None /\ claimAuth < Expiry
(* The stream is always a validator-admitted prefix. *)
StreamValid == \A i \in 1..Len(log) : FlowOk(SubSeq(log, 1, i - 1), log[i])

Safety ==
  /\ OneOutcome /\ AppliedMeansInstalled /\ AbandonedMeansUntouched
  /\ NoSecondDispatch /\ NoClobber /\ UnknownOnlyOnNeither
  /\ RecoverNeverWrites /\ ClaimUnderAuthority /\ StreamValid

(* Probe (expected violation, documenting the authority-timing decision):
   an install may happen after the approval expired. *)
NoInstallAfterExpiry == installed => installAt < Expiry
=============================================================================
