-------------------------------- MODULE Loop --------------------------------
EXTENDS Naturals, FiniteSets, TLC

(***************************************************************************
`plan again` (#77; docs/rfcs/2026-11-21-loops.md;
crates/writfold-kb/src/plan/again.rs; writfold_core::plan::round). A loop
step's rounds are splice files under it, named by ordinal
(`splice-<parent>~<body>.<n>.json`) and published create-new, exactly as
spec/RunGraph.tla models any splice; this model adds what is new: the bound
`Max` on how many rounds `plan again` may write.

Sessions decide outside the clone's write lock (they count the rounds in
the clone and ask the kernel: again only while rounds < Max), then publish
under the lock: with `Recheck` they first re-check that no round was added
since they counted (the existing splice re-check); the file is created new,
and a name already taken is a conflict (nothing written). A crash kills a
clone's sessions between any two steps. With two clones, Merge copies one
clone's files into the other as a set union, refused when the two hold
different bytes under one name (Git's add/add conflict, left to the user).
A file's bytes carry their writer, so two sessions never write equal bytes.

Mutants (constants, one per configuration): AgainChecksMax (the kernel's
rounds < Max), Recheck.
***************************************************************************)
CONSTANTS Max, Topology, AgainChecksMax, Recheck, MaxCrashes

Sessions == {"s1", "s2"}
Clones == IF Topology = "separate" THEN {"c1", "c2"} ELSE {"c1"}
Home == [s \in Sessions |-> IF Topology = "same" \/ s = "s1" THEN "c1" ELSE "c2"]
Ordinals == 1..(Max + 2)

VARIABLES
  files,    \* clone -> ordinal -> writer, or "none"
  pc,       \* session -> "idle" | "decided"
  want,     \* session -> the ordinal it decided to write (0: none)
  seen,     \* session -> the round count it decided on
  crashes

vars == <<files, pc, want, seen, crashes>>

Rounds(c) == Cardinality({n \in Ordinals : files[c][n] /= "none"})

Init ==
  \* Round 1 comes with the plan, in every clone.
  /\ files = [c \in Clones |-> [n \in Ordinals |-> IF n = 1 THEN "plan" ELSE "none"]]
  /\ pc = [s \in Sessions |-> "idle"]
  /\ want = [s \in Sessions |-> 0]
  /\ seen = [s \in Sessions |-> 0]
  /\ crashes = 0

\* Outside the lock: count the rounds and ask the kernel.
Decide(s) ==
  LET r == Rounds(Home[s]) IN
  /\ pc[s] = "idle"
  /\ (~AgainChecksMax \/ r < Max)
  /\ r + 1 \in Ordinals
  /\ pc' = [pc EXCEPT ![s] = "decided"]
  /\ want' = [want EXCEPT ![s] = r + 1]
  /\ seen' = [seen EXCEPT ![s] = r]
  /\ UNCHANGED <<files, crashes>>

\* Under the lock: the re-check, then create-new.
Publish(s) ==
  LET c == Home[s] IN
  /\ pc[s] = "decided"
  /\ IF (Recheck => Rounds(c) = seen[s]) /\ files[c][want[s]] = "none"
       THEN files' = [files EXCEPT ![c][want[s]] = s]
       ELSE UNCHANGED files
  /\ pc' = [pc EXCEPT ![s] = "idle"]
  /\ UNCHANGED <<want, seen, crashes>>

Crash(c) ==
  /\ crashes < MaxCrashes
  /\ crashes' = crashes + 1
  /\ pc' = [s \in Sessions |-> IF Home[s] = c THEN "idle" ELSE pc[s]]
  /\ UNCHANGED <<files, want, seen>>

\* Pull `from`'s files into `into`, between invocations there.
Merge(from, into) ==
  /\ from /= into
  /\ \A s \in Sessions : Home[s] = into => pc[s] = "idle"
  /\ \A n \in Ordinals : files[from][n] = "none" \/ files[into][n] = "none"
                          \/ files[from][n] = files[into][n]
  /\ files' = [files EXCEPT ![into] =
       [n \in Ordinals |-> IF files[into][n] = "none" THEN files[from][n] ELSE files[into][n]]]
  /\ UNCHANGED <<pc, want, seen, crashes>>

Next ==
  \/ \E s \in Sessions : Decide(s) \/ Publish(s)
  \/ \E c \in Clones : Crash(c)
  \/ \E a, b \in Clones : Merge(a, b)

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* Rounds written in one clone never exceed the loop's max.
RoundsWithinMaxLocal == \A c \in Clones : Rounds(c) <= Max

\* Rounds are numbered without gaps: round n exists only after round n-1,
\* so the count is the newest round's number.
RoundsContiguous ==
  \A c \in Clones : \A n \in Ordinals :
    (n > 1 /\ files[c][n] /= "none") => files[c][n - 1] /= "none"
=============================================================================
