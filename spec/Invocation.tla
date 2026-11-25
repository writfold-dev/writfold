------------------------------ MODULE Invocation ------------------------------
EXTENDS Naturals, FiniteSets, TLC

(***************************************************************************
An agent invocation and the report bound to it (#145; RFC
docs/rfcs/2026-11-25-agent-vocabulary.md, calculus §6). It does not change
spec/KbWrite.tla or spec/RunGraph.tla; it adds what is new.

`agent prepare` records an invocation at a plan step with the digest of
the profile's closure as found. The worker is the environment: at any time
it may deliver any payload as a candidate naming any invocation and any
step. `invocation.bound` (the check) passes for a candidate only when its
invocation was prepared in this clone, at the candidate's step, with the
closure still the one prepared, and no other live candidate names the same
invocation with another payload (the single-report rule). `plan advance`
decides outside the lock and publishes under it, re-checking readiness
(`Recheck`). A crash drops a decided advance. Merge copies one clone's
records into another as a set union; definitions are edited in one clone
only (an edit on both sides is Git's conflict, left to the user).

Ready and stale are derived on every read, never stored.

Mutants (constants, one per configuration): Single (the single-report
rule), Recheck (advance re-checks under the lock), SameStep (the bound
step is compared), AdvanceOnDeliver (delivery writes a done record).
***************************************************************************)
CONSTANTS Single, Recheck, SameStep, AdvanceOnDeliver, MaxCrashes, MaxDeliveries

Clones == {"a", "b"}
Steps == {"s1", "s2"}
Invs == {"i1"}
Payloads == {"p", "q"}
Home == [i \in Invs |-> "a"]
Digests == {0, 1}
Cands == [inv : Invs, step : Steps, payload : Payloads]
None == [step |-> "none", digest |-> 0]

VARIABLES
  prep,      \* clone -> invocation -> the step and closure digest prepared, or None
  closure,   \* clone -> the digest of the closure as the definitions stand
  live,      \* clone -> delivered candidates (a set; withdrawal removes from it)
  gone,      \* clone -> withdrawn candidates (kept, so a merge cannot revive one)
  checked,   \* clone -> candidates whose `invocation.bound` check passed
  done,      \* clone -> candidates a done record names
  pending,   \* clone -> the candidate an advance decided on ({} when idle)
  bad,       \* ghost: a done record was written when its candidate was not ready
  last,      \* ghost: the last action's verb
  crashes,
  sent       \* the worker's deliveries so far (bounds the state space)

vars == <<prep, closure, live, gone, checked, done, pending, bad, last, crashes, sent>>

Live(c) == live[c] \ gone[c]

Prepared(c, i) == prep[c][i] /= None

\* Whether the check would pass now (the product's rule, with mutants).
Bound(c, x) ==
  /\ Prepared(c, x.inv)
  /\ (SameStep => prep[c][x.inv].step = x.step)
  /\ prep[c][x.inv].digest = closure[c]
  /\ (Single => \A y \in Live(c) : y.inv = x.inv => y.payload = x.payload)

Ready(c, x) == x \in Live(c) /\ x \in checked[c] /\ Bound(c, x)

Conflict(c, i) == \E x, y \in Live(c) : x.inv = i /\ y.inv = i /\ x.payload /= y.payload

Init ==
  /\ prep = [c \in Clones |-> [i \in Invs |-> None]]
  /\ closure = [c \in Clones |-> 0]
  /\ live = [c \in Clones |-> {}]
  /\ gone = [c \in Clones |-> {}]
  /\ checked = [c \in Clones |-> {}]
  /\ done = [c \in Clones |-> {}]
  /\ pending = [c \in Clones |-> {}]
  /\ bad = FALSE
  /\ last = "init"
  /\ crashes = 0
  /\ sent = 0

Prepare(i, s) ==
  LET c == Home[i] IN
  /\ ~Prepared(c, i)
  /\ prep' = [prep EXCEPT ![c][i] = [step |-> s, digest |-> closure[c]]]
  /\ last' = "prepare"
  /\ UNCHANGED <<closure, live, gone, checked, done, pending, bad, crashes, sent>>

\* The worker: any payload, any invocation, any step, in any clone.
Deliver(c, x) ==
  /\ sent < MaxDeliveries
  /\ sent' = sent + 1
  /\ x \notin live[c]
  /\ live' = [live EXCEPT ![c] = @ \union {x}]
  /\ IF AdvanceOnDeliver
       THEN done' = [done EXCEPT ![c] = @ \union {y \in Live(c) : y.inv = x.inv /\ Ready(c, y)}]
       ELSE UNCHANGED done
  /\ last' = "deliver"
  /\ UNCHANGED <<prep, closure, gone, checked, pending, bad, crashes>>

Check(c, x) ==
  /\ x \in Live(c)
  /\ Bound(c, x)
  /\ checked' = [checked EXCEPT ![c] = @ \union {x}]
  /\ last' = "check"
  /\ UNCHANGED <<prep, closure, live, gone, done, pending, bad, crashes, sent>>

EditClosure ==
  /\ closure["a"] = 0
  /\ closure' = [closure EXCEPT !["a"] = 1]
  /\ last' = "edit"
  /\ UNCHANGED <<prep, live, gone, checked, done, pending, bad, crashes, sent>>

Withdraw(c, x) ==
  /\ x \in Live(c)
  /\ gone' = [gone EXCEPT ![c] = @ \union {x}]
  /\ last' = "withdraw"
  /\ UNCHANGED <<prep, closure, live, checked, done, pending, bad, crashes, sent>>

\* Outside the lock: read readiness and decide.
Decide(c, x) ==
  /\ pending[c] = {}
  /\ Ready(c, x)
  /\ x \notin done[c]
  /\ pending' = [pending EXCEPT ![c] = {x}]
  /\ last' = "decide"
  /\ UNCHANGED <<prep, closure, live, gone, checked, done, bad, crashes, sent>>

\* Under the lock: the re-check, then the done record.
Publish(c) ==
  \E x \in pending[c] :
  /\ IF Recheck => Ready(c, x)
       THEN /\ done' = [done EXCEPT ![c] = @ \union {x}]
            /\ bad' = (bad \/ ~Ready(c, x))
       ELSE UNCHANGED <<done, bad>>
  /\ pending' = [pending EXCEPT ![c] = {}]
  /\ last' = "advance"
  /\ UNCHANGED <<prep, closure, live, gone, checked, crashes, sent>>

Crash(c) ==
  /\ crashes < MaxCrashes
  /\ crashes' = crashes + 1
  /\ pending' = [pending EXCEPT ![c] = {}]
  /\ last' = "crash"
  /\ UNCHANGED <<prep, closure, live, gone, checked, done, bad, sent>>

Merge(from, into) ==
  /\ from /= into
  /\ pending[into] = {}
  /\ prep' = [prep EXCEPT ![into] =
       [i \in Invs |-> IF prep[into][i] = None THEN prep[from][i] ELSE prep[into][i]]]
  /\ closure' = [closure EXCEPT ![into] =
       IF closure[from] > closure[into] THEN closure[from] ELSE closure[into]]
  /\ live' = [live EXCEPT ![into] = @ \union live[from]]
  /\ gone' = [gone EXCEPT ![into] = @ \union gone[from]]
  /\ checked' = [checked EXCEPT ![into] = @ \union checked[from]]
  /\ done' = [done EXCEPT ![into] = @ \union done[from]]
  /\ last' = "merge"
  /\ UNCHANGED <<pending, bad, crashes, sent>>

Next ==
  \/ \E i \in Invs, s \in Steps : Prepare(i, s)
  \/ \E c \in Clones, x \in Cands : Deliver(c, x) \/ Check(c, x) \/ Withdraw(c, x) \/ Decide(c, x)
  \/ EditClosure
  \/ \E c \in Clones : Publish(c) \/ Crash(c)
  \/ \E a, b \in Clones : Merge(a, b)

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* The check passes in a clone only for a candidate whose invocation record
\* is in that clone.
BoundOnlyWithIntent == \A c \in Clones, x \in Cands : Bound(c, x) => Prepared(c, x.inv)

\* Two live payloads for one invocation: none of them is ready.
ConflictNeverReady == \A c \in Clones, x \in Cands : Conflict(c, x.inv) => ~Ready(c, x)

\* Every done record was written where its candidate was ready at that moment.
DoneOnlyWhenReady == ~bad

\* An invocation prepared at one step never makes a candidate ready for another.
OtherStepNeverBinds ==
  \A c \in Clones, x \in Cands : Ready(c, x) => prep[c][x.inv].step = x.step

\* Delivering a report or checking it never writes a done record.
DeliveryDoesNotAdvance ==
  [][last' \in {"deliver", "check"} => done' = done]_vars

\* No action removes a done record.
StaleNeverMovesBack == [][\A c \in Clones : done[c] \subseteq done'[c]]_vars

\* Probe (expected to be violated: the trace shows the state is reachable):
\* a step done on one report, and a merge that brings a conflicting report for
\* the same invocation makes it stale while it stays done.
NoStaleDone ==
  last = "merge" =>
    \A c \in Clones, x \in Cands : x \in done[c] => ~Conflict(c, x.inv)
=============================================================================
