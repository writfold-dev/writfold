----------------------- MODULE M05StalePointerWitness -----------------------
EXTENDS M05RemoteClosure
VARIABLE witnessStep

(* Focused negative reachability schedule, not a canonical exhaustive model.
   Preserve every canonical FiniteInit state, including all fault profiles.
   Fault profiles that refuse verification may deadend without a violation.
   Each scheduled successor is also an original mutant Next successor.
   Publish 1 then 3 locally; advertise verified 3 at remote head 0, skipping 1. *)
WitnessInit == FiniteInit /\ witnessStep = 0

WitnessNext ==
  /\ StalePointerNext
  /\ witnessStep' = witnessStep + 1
  /\ \/ /\ witnessStep = 0 /\ P!Start(1,1) /\ UNCHANGED q
     \/ /\ witnessStep = 1 /\ P!Admit(1) /\ UNCHANGED q
     \/ /\ witnessStep = 2 /\ P!RetainIntent(1) /\ UNCHANGED q
     \/ /\ witnessStep = 3 /\ P!NativeCommit(1) /\ UNCHANGED q
     \/ /\ witnessStep = 4 /\ P!RetainManifest(1) /\ UNCHANGED q
     \/ /\ witnessStep = 5 /\ P!FinalRevalidate(1) /\ UNCHANGED q
     \/ /\ witnessStep = 6 /\ P!DoCAS(1) /\ UNCHANGED q
     \/ /\ witnessStep = 7 /\ P!Start(2,3) /\ UNCHANGED q
     \/ /\ witnessStep = 8 /\ P!Admit(2) /\ UNCHANGED q
     \/ /\ witnessStep = 9 /\ P!RetainIntent(2) /\ UNCHANGED q
     \/ /\ witnessStep = 10 /\ P!NativeCommit(2) /\ UNCHANGED q
     \/ /\ witnessStep = 11 /\ P!RetainManifest(2) /\ UNCHANGED q
     \/ /\ witnessStep = 12 /\ P!FinalRevalidate(2) /\ UNCHANGED q
     \/ /\ witnessStep = 13 /\ P!DoCAS(2) /\ UNCHANGED q
     \/ /\ witnessStep = 14 /\ StartTransfer(1,3)
     \/ /\ witnessStep = 15 /\ TransferGit(1)
     \/ /\ witnessStep = 16 /\ TransferNative(1)
     \/ /\ witnessStep = 17 /\ TransferCapture(1)
     \/ /\ witnessStep = 18 /\ ReceiverVerify(1)
     \/ /\ witnessStep = 19 /\ Acknowledge(1)
     \/ /\ witnessStep = 20 /\ ClientVerify(1)
     \/ /\ witnessStep = 21 /\ MutantStalePointer(1)
=============================================================================
