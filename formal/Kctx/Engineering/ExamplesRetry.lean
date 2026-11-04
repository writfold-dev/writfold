import Kctx.Engineering.Examples

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- Exact recovery and retry attestations are fixture boundary inputs, not
conclusions inferred from a crash, a timeout, or the reducer's current state. -/
def resumedAuthorization (p : Procedure) (provider : ProviderRevision) : ResumeAuthorization :=
  { activation := ⟨1⟩, root := ⟨1⟩, procedure := p.revision, binding := binding,
    input := unitValue, control := ⟨[], [], []⟩, previousIncarnation := 0,
    incarnation := 1, policy := ⟨1⟩, correctionGeneration := 0, provider := provider,
    validFrom := 0, expires := 100 }
def retryAuthorization (i : Intent) (previous : Option (Id .claim))
    (incarnation : Nat) (kind : RetryKind) : RetryAuthorization :=
  ⟨i, previous, incarnation, ⟨1⟩, 0, 100, kind⟩
def contextProc : Procedure := { proc with
  id := ⟨8⟩, revision := ⟨8⟩,
  nodes := [{ baseNode 1 (.effect .check) with resources := [⟨1⟩], requiredContext := [⟨1⟩] },
    { baseNode 2 .«end» with prerequisites := [⟨1⟩] }] }
def contextIntent : Intent := { intent with procedure := contextProc.revision }
def contextReceipt : Receipt := { receipt with procedure := contextProc.revision }
def retryEvent : ExternalEvent := { event with claim := some ⟨2⟩ }
def retryContracts : Contracts := { contracts with
  packages := contextProc :: contracts.packages,
  packageReleases := (PortfolioExamples.base.snapshot, contextProc) :: contracts.packageReleases,
  receipts := contextReceipt :: contracts.receipts,
  effectAuthorizations := contextIntent :: contracts.effectAuthorizations,
  observations := retryEvent :: contracts.observations,
  resumptions := [resumedAuthorization proc ⟨1⟩, resumedAuthorization proc ⟨2⟩,
    resumedAuthorization contextProc ⟨1⟩],
  retryAuthorizations := [retryAuthorization intent none 1 .providerIdempotent,
    retryAuthorization intent (some ⟨1⟩) 1 .providerIdempotent,
    retryAuthorization contextIntent none 1 .providerIdempotent],
  quiescent := (⟨1⟩, 1, 2) :: contracts.quiescent }
def absenceRetryContracts : Contracts := { retryContracts with
  retryAuthorizations := [retryAuthorization intent (some ⟨1⟩) 0 .provenNoEffect] }
def retryAs (c : Contracts) (s : State) (claimId : Nat) : Option State :=
  submit c s (.retryClaim ⟨1⟩ ⟨claimId⟩ (Ledger.currentHead s.ledger ⟨1⟩)
    ⟨s.ledger.records.length + 1⟩)

def resumedUnclaimed : Option State := do
  let s ← prepared
  trace retryContracts s [.crash, .recover, .resume ⟨1⟩]
def retriedUnclaimed : Option State := do
  let s ← resumedUnclaimed
  let retry ← retryAs retryContracts s 2
  submit retryContracts retry (.issueEffect ⟨1⟩)
def retryWithoutOldClaim : Bool := resumedUnclaimed.any (fun s =>
  (findEffect s ⟨1⟩).any (fun e => e.outcome == .unknown && e.dispatch.isNone) &&
  (findRun s ⟨1⟩).any (fun r => live s r && r.incarnation == 1 && r.control == ⟨[], [], []⟩)) &&
  retriedUnclaimed.any (fun s =>
    s.world.pending == [⟨intent, ⟨2⟩⟩] && s.world.applications == [] &&
    s.claims.map (·.id) == [⟨2⟩] && s.budgets == [⟨⟨1⟩, 30, 3⟩] &&
    (findEffect s ⟨1⟩).any (fun e => e.intent == intent && e.outcome == .claimed &&
      e.dispatch.any (fun d => d.id == ⟨2⟩ && d.issued && d.retry == some .providerIdempotent)))

def resumedClaimed : Option State := do
  let s ← claimed
  trace retryContracts s [.crash, .recover, .resume ⟨1⟩]
def retriedClaimed : Option State := do
  let s ← resumedClaimed
  let retry ← retryAs retryContracts s 2
  submit retryContracts retry (.issueEffect ⟨1⟩)
def retryAfterClaimBeforeIO : Bool := resumedClaimed.any (fun s =>
  s.world.pending == [] && s.world.applications == [] &&
  (findEffect s ⟨1⟩).any (fun e => e.outcome == .unknown &&
    e.dispatch.any (fun d => d.id == ⟨1⟩ && !d.issued))) &&
  retriedClaimed.any (fun s => s.world.pending == [⟨intent, ⟨2⟩⟩] &&
    s.claims.map (fun d => (d.id, d.issued)) == [(⟨2⟩, true), (⟨1⟩, false)] &&
    s.budgets == [⟨⟨1⟩, 30, 3⟩])
def retryNeedsFreshClaim : Bool := resumedClaimed.any (fun s =>
  (retryAs retryContracts s 1).isNone && (retryAs retryContracts s 2).isSome) &&
  retriedClaimed.any (fun s => (submit retryContracts s (.issueEffect ⟨1⟩)).isNone &&
    s.world.pending == [⟨intent, ⟨2⟩⟩])

def resumedApplied : Option State := do
  let s ← replyLost
  let quiet ← environment retryContracts s (.terminateProcess ⟨1⟩ 1 1)
  trace retryContracts quiet [.crash, .recover, .resume ⟨1⟩]
def duplicateApplied : Option State := do
  let s ← resumedApplied
  let retry ← retryAs retryContracts s 2
  let sent ← submit retryContracts retry (.issueEffect ⟨1⟩)
  -- The second delivery returns the canonical first result. Its supplied
  -- producer/consumer cannot reapply or overwrite the already committed state.
  let applied ← environment retryContracts sent (.completeExternalEffect retryEvent 99 99)
  trace retryContracts applied [.observe retryEvent, .reconcile retryEvent]
def idempotentRetryOneApplication : Bool := duplicateApplied.any (fun s =>
  s.world.applications == [⟨intent, .applied, ⟨2⟩⟩] &&
  s.world.events == [retryEvent, event] && s.world.replies == [retryEvent] &&
  s.world.provider == ⟨2⟩ && s.world.producer == 1 && s.world.consumer == 0 &&
  s.world.pending == [] && s.claims.map (fun d => (d.id, d.issued)) ==
    [(⟨2⟩, true), (⟨1⟩, true)] &&
  s.ownership == [⟨⟨1⟩, ⟨1⟩, ⟨1⟩, 1, 2, 1⟩] &&
  (findEffect s ⟨1⟩).any (fun e => e.intent == intent && e.outcome == .confirmed))

def duplicateAfterOutsideChange : Option State := do
  let s ← resumedApplied
  let retry ← retryAs retryContracts s 2
  let sent ← submit retryContracts retry (.issueEffect ⟨1⟩)
  let changed ← environment retryContracts sent (.changeProviderState ⟨3⟩ 7 8)
  environment retryContracts changed (.completeExternalEffect retryEvent 99 99)

/-- An idempotent reply describes the old application, not a rollback of later
independent provider changes. All three current world coordinates survive. -/
def idempotentReplyPreservesOutsideChange : Bool := duplicateAfterOutsideChange.any (fun s =>
  s.world.applications == [⟨intent, .applied, ⟨2⟩⟩] &&
  s.world.provider == ⟨3⟩ && s.world.producer == 7 && s.world.consumer == 8 &&
  s.world.events == [retryEvent, event] && s.world.replies == [retryEvent] &&
  s.world.pending == [])

def rejectedAttempt : Option State := do
  let s ← issued
  let rejected ← environment absenceRetryContracts s (.completeExternalEffect claimedNoEffect 0 0)
  let quiet ← environment absenceRetryContracts rejected (.terminateProcess ⟨1⟩ 1 1)
  submit absenceRetryContracts quiet (.observe claimedNoEffect)
def retriedAbsence : Option State := rejectedAttempt.bind (fun s => retryAs absenceRetryContracts s 2)
def appliedAfterAbsence : Option State := do
  let s ← retriedAbsence
  let sent ← submit absenceRetryContracts s (.issueEffect ⟨1⟩)
  let applied ← environment absenceRetryContracts sent (.completeExternalEffect retryEvent 1 0)
  trace absenceRetryContracts applied [.observe retryEvent, .reconcile retryEvent]
def provenAbsenceRetryApplies : Bool := rejectedAttempt.any (fun s =>
  s.world.applications == [] && s.world.events == [claimedNoEffect] &&
  (findEffect s ⟨1⟩).any (·.outcome == .unknown)) &&
  appliedAfterAbsence.any (fun s => s.world.applications == [⟨intent, .applied, ⟨2⟩⟩] &&
    s.world.events == [retryEvent, claimedNoEffect] && s.world.pending == [] &&
    (findEffect s ⟨1⟩).any (fun e => e.outcome == .confirmed && e.dispatch.any (fun d =>
      d.id == ⟨2⟩ && d.retry == some .provenNoEffect)))
def oldRejectionCannotClassifyRetry : Bool := retriedAbsence.any (fun s =>
  s.observations == [claimedNoEffect] && s.world.pending == [] &&
  (findEffect s ⟨1⟩).any (fun e => e.outcome == .claimed && e.dispatch.any (·.id == ⟨2⟩)) &&
  (submit absenceRetryContracts s (.reconcile claimedNoEffect)).isNone &&
  (submit absenceRetryContracts s (.observe claimedNoEffect)).isNone &&
  (submit absenceRetryContracts s (.issueEffect ⟨1⟩)).isSome)

def contextResumed : Option State := do
  let s ← start retryContracts contextProc
  trace retryContracts s [.loadResource ⟨1⟩ ⟨1⟩, .reserve ⟨1⟩ [⟨1⟩] 1,
    .recordReceipt contextReceipt, .prepare contextIntent 1, .crash, .recover, .resume ⟨1⟩]
def contextReloaded : Option State := do
  let s ← contextResumed
  let loaded ← submit retryContracts s (.loadResource ⟨1⟩ ⟨1⟩)
  let retry ← retryAs retryContracts loaded 2
  submit retryContracts retry (.issueEffect ⟨1⟩)
def resumeRequiresContextReload : Bool := contextResumed.any (fun s =>
  (findRun s ⟨1⟩).any (fun r => live s r && r.loaded == [] && r.input == unitValue &&
    r.control == ⟨[], [], []⟩ && r.procedure == contextProc) &&
  (retryAs retryContracts s 2).isNone) && contextReloaded.any (fun s =>
    (findRun s ⟨1⟩).any (fun r => r.loaded == [⟨1⟩] && r.incarnation == 1) &&
    s.world.pending == [⟨contextIntent, ⟨2⟩⟩] && s.budgets == [⟨⟨1⟩, 30, 5⟩])
def unqualifiedResumeRefused : Bool := claimCrash.any (fun s =>
  (submit contracts s (.resume ⟨1⟩)).isNone &&
  (submit retryContracts s (.resume ⟨1⟩)).isSome)

def conflictingProc : Procedure := { proc with
  id := ⟨9⟩, revision := ⟨9⟩,
  nodes := [baseNode 1 (.select [⟨2⟩]),
    { baseNode 2 (.effect .check) with resources := [⟨1⟩], prerequisites := [⟨1⟩] },
    { baseNode 3 (.effect .check) with resources := [⟨1⟩], prerequisites := [⟨1⟩] },
    { baseNode 4 .«end» with prerequisites := [⟨2⟩, ⟨3⟩] }] }
def firstConflictIntent : Intent := { intent with procedure := conflictingProc.revision, node := ⟨2⟩ }
def secondConflictIntent : Intent := { firstConflictIntent with effect := ⟨2⟩, node := ⟨3⟩ }
def conflictReceipt (node : Nat) : Receipt := { receipt with procedure := conflictingProc.revision, node := ⟨node⟩ }
def conflictContracts : Contracts := { contracts with
  packages := conflictingProc :: contracts.packages,
  packageReleases := (PortfolioExamples.base.snapshot, conflictingProc) :: contracts.packageReleases,
  receipts := conflictReceipt 2 :: conflictReceipt 3 :: contracts.receipts }
def conflictReady : Option State := do
  let s ← start conflictContracts conflictingProc
  trace conflictContracts s [.selectRoute ⟨1⟩ ⟨1⟩ ⟨2⟩ unitValue, .reserve ⟨1⟩ [⟨1⟩] 1,
    .recordReceipt (conflictReceipt 2), .recordReceipt (conflictReceipt 3)]
def conflictPrepared : Option State := conflictReady.bind (fun s =>
  submit conflictContracts s (.prepare firstConflictIntent 1))
def sameActivationConflictRefused : Bool := conflictReady.any (fun s =>
  (submit conflictContracts s (.prepare secondConflictIntent 1)).any (fun t =>
    t.effects == [⟨secondConflictIntent, 1, .prepared, none⟩])) &&
  conflictPrepared.any (fun s => s.effects == [⟨firstConflictIntent, 1, .prepared, none⟩] &&
    (findRun s ⟨1⟩).any (fun r => (findNode r.procedure ⟨3⟩).any (fun n =>
      nodeReady conflictContracts s r n && evidenceReady s r n)) &&
    !noConflictingEffect s secondConflictIntent &&
    (submit conflictContracts s (.prepare secondConflictIntent 1)).isNone)

theorem retry_unclaimed_nonvacuous : retryWithoutOldClaim = true := by decide
theorem retry_claimed_nonvacuous : retryAfterClaimBeforeIO = true := by decide
theorem retry_fresh_claim_nonvacuous : retryNeedsFreshClaim = true := by decide
theorem retry_idempotent_nonvacuous : idempotentRetryOneApplication = true := by decide
theorem retry_outside_change_nonvacuous : idempotentReplyPreservesOutsideChange = true := by decide
theorem retry_absence_nonvacuous : provenAbsenceRetryApplies = true := by decide
theorem retry_old_rejection_nonvacuous : oldRejectionCannotClassifyRetry = true := by decide
theorem resume_context_nonvacuous : resumeRequiresContextReload = true := by decide
theorem resume_authority_nonvacuous : unqualifiedResumeRefused = true := by decide
theorem same_activation_conflict_nonvacuous : sameActivationConflictRefused = true := by decide

end Kctx.Engineering.Examples
