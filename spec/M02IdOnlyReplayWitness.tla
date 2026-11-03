---------------------- MODULE M02IdOnlyReplayWitness ----------------------
EXTENDS M02RecoveryReplay

(* Focused negative reachability search, not an exhaustive model campaign.
   The canonical initial publication/recovery state is unchanged. The only
   extra state is a scheduling counter. Every scheduled edge is also an edge
   of the canonical FiniteRecoveryNext with the configuration's unchanged
   RepairCompletion <- MutantIdOnlyReplay substitution.

   Publish exact input 1, then let worker 2 inspect colliding input 2. Its
   operation ID already names published input 1, but input 2 was never applied.
   The mutant must expose its false durable receipt through RecoveryInv.
   No hint-erased VIEW is permitted for this negative calibration. *)
VARIABLE witnessStep
WitnessVars == <<p,r,witnessStep>>
WitnessInit == RecoveryInit /\ witnessStep=0
WitnessSchedule ==
  \/ (witnessStep=0 /\ Start(1,1))
  \/ (witnessStep=1 /\ Admit(1))
  \/ (witnessStep=2 /\ RetainIntent(1))
  \/ (witnessStep=3 /\ NativeCommit(1))
  \/ (witnessStep=4 /\ RetainManifest(1))
  \/ (witnessStep=5 /\ FinalRevalidate(1))
  \/ (witnessStep=6 /\ DoCAS(1))
  \/ (witnessStep=7 /\ RecoverStart(2,2))
  \/ (witnessStep=8 /\ RepairCompletion(2))
WitnessNext == /\ FiniteRecoveryNext
               /\ WitnessSchedule
               /\ witnessStep'=witnessStep+1
=============================================================================
