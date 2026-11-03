-------------------- MODULE M05EarlyAdvertisementWitness --------------------
EXTENDS M05RemoteClosure
VARIABLE witnessStep

(* Focused negative reachability schedule, not a canonical exhaustive model.
   Preserve every canonical FiniteInit state, including all fault profiles.
   Each scheduled successor is also an original mutant Next successor. *)
WitnessInit == FiniteInit /\ witnessStep = 0

WitnessNext ==
  /\ EarlyAdvertisementNext
  /\ witnessStep' = witnessStep + 1
  /\ \/ /\ witnessStep = 0 /\ P!Start(1,1) /\ UNCHANGED q
     \/ /\ witnessStep = 1 /\ P!Admit(1) /\ UNCHANGED q
     \/ /\ witnessStep = 2 /\ P!RetainIntent(1) /\ UNCHANGED q
     \/ /\ witnessStep = 3 /\ P!NativeCommit(1) /\ UNCHANGED q
     \/ /\ witnessStep = 4 /\ P!RetainManifest(1) /\ UNCHANGED q
     \/ /\ witnessStep = 5 /\ P!FinalRevalidate(1) /\ UNCHANGED q
     \/ /\ witnessStep = 6 /\ P!DoCAS(1) /\ UNCHANGED q
     \/ /\ witnessStep = 7 /\ StartTransfer(1,1)
     \/ /\ witnessStep = 8 /\ MutantEarlyAdvertisement(1)
=============================================================================
