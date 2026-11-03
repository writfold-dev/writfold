--------------------- MODULE M02M06RestoreOldWitness ---------------------
EXTENDS M02M06Bridge

(* Focused negative reachability schedule, not a canonical exhaustive model.
   Init is the original bridge mutant's complete canonical initial predicate;
   only the witness counter is added. Every scheduled transition calls an
   unchanged bridge action and is conjoined with the original mutated Next,
   preserving both component guards and the publication corruption guards.

   Writer 1 publishes the exact chain 0 -> 1 -> 3 -> 5: migration, correction,
   then forward selection. All three odd inputs bind inventory 0, exactly the
   canonical inventory, so no inventory change or weakened exactness guard is
   needed. Publication of correction 3 retains correction {3}; selection 5
   carries that correction and current contexts forward before the mutation.

   Between generations, Receipt records the publication receipt before
   Complete synchronizes publication Finish and migration RecordReceipt.
   Complete leaves the publication writer done, not idle: the original bridge
   Crash and Restart reset all three component controls before its next Start.
   After Apply for input 5, the original mutant copies record 0's corrections
   and contexts into record 5. The schedule ends after this negative witness. *)
VARIABLE witnessStep
WitnessVars == <<p,r,m,witnessStep>>
WitnessInit == Init /\ witnessStep = 0
WitnessNext ==
  /\ RestoreOldNext
  /\ witnessStep' = witnessStep + 1
  /\ \/ (witnessStep = 0 /\ Start(1,1))
     \/ (witnessStep = 1 /\ Admit(1))
     \/ (witnessStep = 2 /\ RetainIntent(1))
     \/ (witnessStep = 3 /\ Stage(1))
     \/ (witnessStep = 4 /\ Manifest(1))
     \/ (witnessStep = 5 /\ Verify(1))
     \/ (witnessStep = 6 /\ Apply(1))
     \/ (witnessStep = 7 /\ Receipt(1))
     \/ (witnessStep = 8 /\ Complete(1))
     \/ (witnessStep = 9 /\ Crash(1))
     \/ (witnessStep = 10 /\ Restart(1))
     \/ (witnessStep = 11 /\ Start(1,3))
     \/ (witnessStep = 12 /\ Admit(1))
     \/ (witnessStep = 13 /\ RetainIntent(1))
     \/ (witnessStep = 14 /\ Stage(1))
     \/ (witnessStep = 15 /\ Manifest(1))
     \/ (witnessStep = 16 /\ Verify(1))
     \/ (witnessStep = 17 /\ Apply(1))
     \/ (witnessStep = 18 /\ Receipt(3))
     \/ (witnessStep = 19 /\ Complete(1))
     \/ (witnessStep = 20 /\ Crash(1))
     \/ (witnessStep = 21 /\ Restart(1))
     \/ (witnessStep = 22 /\ Start(1,5))
     \/ (witnessStep = 23 /\ Admit(1))
     \/ (witnessStep = 24 /\ RetainIntent(1))
     \/ (witnessStep = 25 /\ Stage(1))
     \/ (witnessStep = 26 /\ Manifest(1))
     \/ (witnessStep = 27 /\ Verify(1))
     \/ (witnessStep = 28 /\ Apply(1))
     \/ (witnessStep = 29 /\ MutantRestoreOld(1))
WitnessSpec == WitnessInit /\ [][WitnessNext]_WitnessVars
=============================================================================
