import Kctx.Engineering.FixtureCalculus

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- The exact evaluated/reviewed material still cannot weaken the governing floor. -/
def learningProcedureContracts (p : Procedure) : Contracts :=
  let candidate := { lesson with procedure := some p }
  { contracts with
    packages := p :: contracts.packages
    packageReleases := (PortfolioExamples.candidate.snapshot, p) :: contracts.packageReleases
    evaluations := [candidate.toCandidateMaterial]
    reviews := [candidate.toCandidateMaterial]
    learningApprovals := [⟨candidate.toCandidateMaterial, ⟨1⟩, 0, 100⟩] }

def learningProcedureReviewed (p : Procedure) : Option State := do
  let c := learningProcedureContracts p
  let candidate := { lesson with procedure := some p }
  let s ← init c inputs
  trace c s [.propose candidate, .evaluate candidate.id, .review candidate.id]

def learnedFloorRefused : Bool :=
  let weak := { procV2 with floor := [] }
  (learningProcedureReviewed procV2).any (fun s =>
    (submit (learningProcedureContracts procV2) s (.prepareAdmission ⟨1⟩)).isSome) &&
  (learningProcedureReviewed weak).any (fun s =>
    s.candidates.any (fun candidate => candidate.evaluated && candidate.reviewed) &&
    (submit (learningProcedureContracts weak) s (.prepareAdmission ⟨1⟩)).isNone)

/-- The receipt remains retained and the grant remains live after receipt expiry. -/
def staleReuseRefused : Bool := prepared.any (fun s =>
  (environment contracts s (.advanceTime 51)).any (fun t =>
    t.receipts.contains receipt &&
    (findRun t ⟨1⟩).any (fun r => live t r) &&
    (submit contracts t (.reuse ⟨1⟩ receipt)).isNone))

def otherBindingReady (b : Binding) : Option State := do
  let input := { inputs with authority := { inputs.authority with
    grants := { grant 4 2 with binding := b } :: inputs.authority.grants } }
  let s ← init contracts input
  trace contracts s [.resolve proc, .activate run 30, .recordReceipt receipt,
    .activate { runTwo with binding := b } 20]

/-- Same-qualified historical evidence is reusable; changed dependency identity is not. -/
def changedDependencyReuseRefused : Bool :=
  (otherBindingReady binding).any (fun s =>
    (submit contracts s (.reuse ⟨20⟩ receipt)).isSome) &&
  (otherBindingReady { binding with dependencies := [⟨2⟩] }).any (fun s =>
    s.receipts.contains receipt && (findRun s ⟨20⟩).any (fun r => live s r) &&
    (submit contracts s (.reuse ⟨20⟩ receipt)).isNone)

def migrationReturnContracts : Contracts := { contracts with
  typedEvidence := predicateEvidence 9 :: contracts.typedEvidence }

def newActivationPending : Option State := do
  let c := migrationReturnContracts
  let initial ← init c inputs
  let s ← trace c initial [.resolve leaf, .resolve caller,
    .activate { run with procedure := caller } 30,
    .selectRoute ⟨1⟩ ⟨1⟩ ⟨2⟩ unitValue, .call ⟨1⟩ ⟨2⟩ ⟨3⟩ leaf binding,
    .advanceNode ⟨3⟩ ⟨1⟩ evidenceValue, .advanceNode ⟨3⟩ ⟨2⟩ evidenceValue,
    .recordReceipt childReceipt, .returnChild ⟨1⟩ ⟨3⟩ childReceipt]
  trace c s [.cancel ⟨1⟩, .crash, .recover,
    .rebind ⟨1⟩ caller binding ⟨9⟩,
    .selectRoute ⟨9⟩ ⟨1⟩ ⟨2⟩ unitValue,
    .call ⟨9⟩ ⟨2⟩ ⟨10⟩ leaf binding]

/-- A completed old child is not the new activation's pending invocation. -/
def oldChildReturnRefused : Bool := newActivationPending.any (fun s =>
  s.receipts.contains childReceipt &&
  (findRun s ⟨3⟩).any (fun r => r.parent == some ⟨1⟩ && r.status == .completed) &&
  (findRun s ⟨10⟩).any (fun r => r.parent == some ⟨9⟩ && r.status == .active) &&
  (findRun s ⟨9⟩).any (fun r => r.control.pending.any (fun pending => pending.child == ⟨10⟩)) &&
  (submit migrationReturnContracts s (.returnChild ⟨9⟩ ⟨3⟩ childReceipt)).isNone)

/-- Missing physical closure is an explicit fault input, not an admitted GC writer.
The intact publication roots evidence before CAS; the damaged store cannot publish. -/
def publicationRetentionRefused : Bool := admissionReady.any (fun s =>
  (submit contracts s .publishAdmission).any (fun t =>
    t.knowledge == PortfolioExamples.admitted && t.ledger.releaseRoots == s.retained) &&
  let damaged : State := { s with ledger := { s.ledger with
    material := s.ledger.material.filter (· != (⟨1⟩ : ObjectId)) } }
  s.admission.isSome && s.ledger.releaseRoots == [] &&
  damaged.knowledge == PortfolioExamples.initial &&
  (submit contracts damaged .publishAdmission).isNone)

theorem learned_floor_nonvacuous : learnedFloorRefused = true := by decide
theorem stale_reuse_nonvacuous : staleReuseRefused = true := by decide
theorem changed_dependency_reuse_nonvacuous : changedDependencyReuseRefused = true := by decide
theorem old_child_return_nonvacuous : oldChildReturnRefused = true := by decide
theorem publication_retention_nonvacuous : publicationRetentionRefused = true := by decide

end Kctx.Engineering.Examples
