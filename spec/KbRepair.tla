------------------------------- MODULE KbRepair ------------------------------
EXTENDS Naturals, TLC

(***************************************************************************
Repairing the derived index from a shadow rebuild (#58; crates/writfold-kb/
src/index/verify.rs and src/index/mod.rs). spec/KbWrite.tla models the KB
writes and the transactional refresh; this model covers the other way an
index is published: a whole new file built beside it and renamed over it,
as `index rebuild`, a full rebuild in a refresh, and the repair of
`index verify` / `kb check` do.

KB. One file whose content is a number; a writer edits it (MaxEdits times).
A snapshot shows that content; the pin is the snapshot.

Index. The stored index holds a pin and the Phase A facts it was derived
from, and `ok`: FALSE after storage corrupted it (a bit flip, a stray
write), the case repair exists for. A healthy index is the derivation of
its pin (ok => a = pin).

Refresher. Observes the snapshot, reads the file (possibly after the writer
moved it), re-observes and commits in one transaction only when the
snapshot still holds (KbWrite's RecheckPin, kept here as the context the
verifier races with).

Verifier. Observes the snapshot S; builds a shadow labelled S from the file
as it reads it; compares the live index with the shadow; on a mismatch it
replaces the index by rename only when the source is still at S
(RepairRecheck, a re-observation after the build) and the live index is
still the very generation it compared, at S (LiveCheck: the replacing
transaction sees the same data version and pin as the comparison; a
refresh that committed in between makes the comparison void). Otherwise it keeps the index and retries.

Crash. At any moment (at most MaxCrashes): processes die; the shadow file
is lost; the rename is atomic, so the index is the old or the new file.

Mutants (constants, each FALSE in exactly one configuration):
RepairRecheck (re-observe the source before the rename), LiveCheck (replace
only an index at the observed pin).

Trusted premises: rename is atomic and durable once the directory is
synced (as in KbWrite); SQLite transactions are atomic and durable; the
comparison is exact (the verifier's canonical digests). Not modelled: the
streaming of the comparison (it changes memory, not what is compared).
***************************************************************************)
CONSTANTS MaxEdits, MaxCrashes, MaxCorruptions, RepairRecheck, LiveCheck

VARIABLES
  file,           \* the KB file's content
  ix,             \* the stored index: [pin, a, ok]
  rpc, robs, rread,
  vpc, vobs, shadow, mismatch, seen,  \* seen: the live index generation compared
  repaired,       \* every index a repair replaced: [old, new]
  edits, crashes, corruptions

vars == <<file, ix, rpc, robs, rread, vpc, vobs, shadow, mismatch, seen, repaired,
          edits, crashes, corruptions>>

Init ==
  /\ file = 0
  /\ ix = [pin |-> 0, a |-> 0, ok |-> TRUE]
  /\ rpc = "idle" /\ robs = 0 /\ rread = 0
  /\ vpc = "idle" /\ vobs = 0 /\ shadow = [pin |-> 0, a |-> 0] /\ mismatch = FALSE
  /\ seen = [pin |-> 0, a |-> 0, ok |-> TRUE]
  /\ repaired = {}
  /\ edits = 0 /\ crashes = 0 /\ corruptions = 0

Edit ==
  /\ edits < MaxEdits
  /\ file' = file + 1 /\ edits' = edits + 1
  /\ UNCHANGED <<ix, rpc, robs, rread, vpc, vobs, shadow, mismatch, seen, repaired, crashes, corruptions>>

\* Storage corrupts the index: its rows no longer derive from its pin.
Corrupt ==
  /\ corruptions < MaxCorruptions
  /\ ix' = [ix EXCEPT !.a = ix.a + 100, !.ok = FALSE]
  /\ corruptions' = corruptions + 1
  /\ UNCHANGED <<file, rpc, robs, rread, vpc, vobs, shadow, mismatch, seen, repaired, edits, crashes>>

RObserve ==
  /\ rpc = "idle" /\ file /= ix.pin
  /\ robs' = file /\ rpc' = "observed"
  /\ UNCHANGED <<file, ix, rread, vpc, vobs, shadow, mismatch, seen, repaired, edits, crashes, corruptions>>

RRead ==
  /\ rpc = "observed"
  /\ rread' = file /\ rpc' = "read"
  /\ UNCHANGED <<file, ix, robs, vpc, vobs, shadow, mismatch, seen, repaired, edits, crashes, corruptions>>

RCommit ==
  /\ rpc = "read"
  /\ ix' = IF file = robs THEN [pin |-> robs, a |-> rread, ok |-> TRUE] ELSE ix
  /\ rpc' = "idle"
  /\ UNCHANGED <<file, robs, rread, vpc, vobs, shadow, mismatch, seen, repaired, edits, crashes, corruptions>>

VObserve ==
  /\ vpc = "idle"
  /\ vobs' = file /\ vpc' = "observed"
  /\ UNCHANGED <<file, ix, rpc, robs, rread, shadow, mismatch, seen, repaired, edits, crashes, corruptions>>

\* The shadow is built from the file as it is read, labelled with the pin
\* observed before.
VBuild ==
  /\ vpc = "observed"
  /\ shadow' = [pin |-> vobs, a |-> file] /\ vpc' = "built"
  /\ UNCHANGED <<file, ix, rpc, robs, rread, vobs, mismatch, seen, repaired, edits, crashes, corruptions>>

VCompare ==
  /\ vpc = "built"
  /\ mismatch' = (ix.pin /= shadow.pin \/ ix.a /= shadow.a)
  /\ seen' = ix
  /\ vpc' = "compared"
  /\ UNCHANGED <<file, ix, rpc, robs, rread, vobs, shadow, repaired, edits, crashes, corruptions>>

\* Replace by rename only on a mismatch, and (RepairRecheck) only while the
\* source and the live index are both still at the observed pin.
VRepair ==
  /\ vpc = "compared"
  /\ LET go == /\ mismatch
               /\ (~RepairRecheck \/ file = vobs)
               /\ (~LiveCheck \/ (ix = seen /\ ix.pin = vobs))
     IN /\ ix' = IF go THEN [pin |-> shadow.pin, a |-> shadow.a, ok |-> TRUE] ELSE ix
        /\ repaired' = IF go THEN repaired \cup {[old |-> ix, new |-> shadow]} ELSE repaired
  /\ vpc' = "idle"
  /\ UNCHANGED <<file, rpc, robs, rread, vobs, shadow, mismatch, seen, edits, crashes, corruptions>>

Crash ==
  /\ crashes < MaxCrashes
  /\ crashes' = crashes + 1
  /\ rpc' = "idle" /\ vpc' = "idle" /\ mismatch' = FALSE
  /\ UNCHANGED <<file, ix, robs, rread, vobs, shadow, seen, repaired, edits, corruptions>>

Next ==
  \/ Edit \/ Corrupt
  \/ RObserve \/ RRead \/ RCommit
  \/ VObserve \/ VBuild \/ VCompare \/ VRepair
  \/ Crash

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* A healthy index is the derivation of the pin it names: neither a refresh
\* nor a repair ever publishes another (KbWrite's IndexIsDerivation, for
\* the rename path).
HealthyIsDerivation == ix.ok => ix.a = ix.pin

\* Every repair installed the derivation of its pin.
RepairIsDerivation == \A r \in repaired : r.new.a = r.new.pin

\* A repair replaces only an index that was not the derivation of its own
\* pin: drift (the KB or the index moving on) never authorises replacing a
\* healthy index.
RepairOnlyWhenBroken == \A r \in repaired : r.old.a /= r.old.pin

Safety == HealthyIsDerivation /\ RepairIsDerivation /\ RepairOnlyWhenBroken
=============================================================================
