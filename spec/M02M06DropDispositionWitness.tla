------------------- MODULE M02M06DropDispositionWitness -------------------
EXTENDS M02M06Bridge

(* Focused negative reachability schedule, not a canonical exhaustive model.
   Init is the original bridge mutant's complete canonical initial predicate;
   only the witness counter is added. Every scheduled transition calls an
   unchanged bridge action and is conjoined with the original mutated Next,
   preserving both component guards and the publication corruption guards.

   Writer 1 publishes input 1 through every synchronized publication phase.
   Its odd input identity binds inventory 0, exactly the canonical inventory;
   no inventory or validator observation is replaced. At the applied phase,
   the original mutant removes the accepted-flag accounting disposition.
   The schedule ends immediately after that negative witness. *)
VARIABLE witnessStep
WitnessVars == <<p,r,m,witnessStep>>
WitnessInit == Init /\ witnessStep = 0
WitnessNext ==
  /\ DropDispositionNext
  /\ witnessStep' = witnessStep + 1
  /\ \/ (witnessStep = 0 /\ Start(1,1))
     \/ (witnessStep = 1 /\ Admit(1))
     \/ (witnessStep = 2 /\ RetainIntent(1))
     \/ (witnessStep = 3 /\ Stage(1))
     \/ (witnessStep = 4 /\ Manifest(1))
     \/ (witnessStep = 5 /\ Verify(1))
     \/ (witnessStep = 6 /\ Apply(1))
     \/ (witnessStep = 7 /\ MutantDropDisposition(1))
WitnessSpec == WitnessInit /\ [][WitnessNext]_WitnessVars
=============================================================================
