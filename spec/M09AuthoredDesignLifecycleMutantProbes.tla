---------------- MODULE M09AuthoredDesignLifecycleMutantProbes ----------------
EXTENDS M09AuthoredDesignLifecycle

(* Directed negative witnesses, not positive exploration qualification.
   Init is the canonical Init, including every origin and fault profile.
   There are no new variables, constants, assignments or state constraints.
   Each controlled successor must also satisfy the original finite mutant
   transition used by its existing negative configuration. Specification
   stuttering remains permitted. Safety and Inv are imported unchanged.

   Stale witness: bind 1; retain reopen 7 at revision 1 and crash; another
   actor lands 3 at revision 1; recover the retained 7 and skip the final CAS.
   Requests 3 and 7 have distinct operation identities but the same base.

   Landed witness: bind 1; land 3; rebind 4 while incorrectly copying the old
   landed marker. The resulting actual file differs from RecordAt(origin,4).
   Actor provenance remains unrelated to knowledge approval. *)

StaleWitnessControl ==
  \/ (a.head=0 /\ Start(1,1))
  \/ (a.head=1 /\ Start(1,7))
  \/ Acquire(1)
  \/ RetainIntent(1)
  \/ (a.request[1]=1 /\ ConditionalWrite(1))
  \/ (a.request[1]=1 /\ Complete(1))
  \/ (a.head=1 /\ a.request[1]=7 /\ a.pc[1]="intent" /\ Crash(1))
  \/ (a.head=1 /\ a.pc[1]="down" /\ a.intent[OpOf(7)]=7 /\ Start(2,3))
  \/ Acquire(2)
  \/ RetainIntent(2)
  \/ ConditionalWrite(2)
  \/ Complete(2)
  \/ (a.head=3 /\ Restart(1))
  \/ (a.head=3 /\ RecoverStart(1,7))
  \/ (a.head=3 /\ a.request[1]=7 /\ MutantResumeStale(1))
  \/ (a.head=3 /\ a.request[1]=7 /\ WriteCore(1,TRUE,FALSE))
StaleWitnessNext == FiniteStaleNext /\ StaleWitnessControl
StaleWitnessSpec == Init /\ [][StaleWitnessNext]_vars

LandedWitnessControl ==
  \/ (a.head=0 /\ Start(1,1))
  \/ (a.head=1 /\ Start(1,3))
  \/ (a.head=3 /\ Start(1,4))
  \/ Acquire(1)
  \/ RetainIntent(1)
  \/ (a.request[1] \in {1,3} /\ ConditionalWrite(1))
  \/ Complete(1)
  \/ (a.request[1]=4 /\ WriteCore(1,FALSE,TRUE))
LandedWitnessNext == FiniteLandedNext /\ LandedWitnessControl
LandedWitnessSpec == Init /\ [][LandedWitnessNext]_vars

(* The inclusion is structural, not an assumption about controlled effects. *)
THEOREM StaleWitnessIncluded == StaleWitnessNext => StaleNext
BY ONLY DEF StaleWitnessNext, FiniteStaleNext
THEOREM LandedWitnessIncluded == LandedWitnessNext => LandedNext
BY ONLY DEF LandedWitnessNext, FiniteLandedNext
=============================================================================
