import Kctx.Engineering.FixtureCalculus

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- The requested callee/delegation ceilings contain publish, but neither the
parent's actual authority nor its grant does. A successful Call cannot add it. -/
def broadCallee : Procedure := { leaf with
  id := ⟨30⟩, revision := ⟨30⟩, ceiling := [.read, .check, .publish] }
def broadCaller : Procedure := { caller with
  id := ⟨40⟩, revision := ⟨40⟩, ceiling := [.read, .check, .publish]
  nodes := caller.nodes.map (fun n => if n.id == ⟨2⟩ then
    { n with
      form := .call { target broadCallee with delegation := [.check, .publish] }
      ceiling := [.read, .check, .publish] } else n) }
def broadCallContracts : Contracts := { contracts with
  packages := broadCallee :: broadCaller :: contracts.packages
  packageReleases := [(PortfolioExamples.base.snapshot, broadCallee),
    (PortfolioExamples.base.snapshot, broadCaller)] ++ contracts.packageReleases }
def broadCalled : Option State := do
  let s ← init broadCallContracts inputs
  trace broadCallContracts s [.resolve broadCallee, .resolve broadCaller,
    .activate { run with procedure := broadCaller } 30,
    .selectRoute ⟨1⟩ ⟨1⟩ ⟨2⟩ unitValue,
    .call ⟨1⟩ ⟨2⟩ ⟨3⟩ broadCallee binding]
def childPublishRefused : Bool := broadCalled.any (fun s =>
  broadCallee.ceiling.contains .publish &&
  (findRun s ⟨1⟩).any (fun parent => !parent.capabilities.contains .publish) &&
  (findRun s ⟨3⟩).any (fun child =>
    child.parent == some ⟨1⟩ && child.capabilities == [.check] &&
    !child.capabilities.contains .publish &&
    (currentGrant s child).any (fun grant => !grant.capabilities.contains .publish)))

/-- Grant expiry occurs strictly before receipt expiry. The same selected claim
can send before that boundary and must refuse after it, with evidence still fresh. -/
def shortGrantClaimed : Option State := do
  let input := { inputs with authority := { inputs.authority with
    grants := inputs.authority.grants.map (fun g =>
      if g.id == ⟨1⟩ then { g with expires := 20 } else g) } }
  let s ← init contracts input
  let prepared ← trace contracts s [.resolve proc, .activate run 30,
    .reserve ⟨1⟩ [⟨1⟩] 1, .recordReceipt receipt, .prepare intent 1]
  claim contracts prepared
def expiredGrantRefused : Bool := shortGrantClaimed.any (fun s =>
  (submit contracts s (.issueEffect ⟨1⟩)).isSome &&
  (environment contracts s (.advanceTime 21)).any (fun t =>
    t.now < receipt.expires && t.receipts.contains receipt &&
    (findRun t ⟨1⟩).any (fun r => (currentGrant t r).isNone && !live t r) &&
    (submit contracts t (.issueEffect ⟨1⟩)).isNone))

def withdrawnProcedureRefused : Bool := claimed.any (fun s =>
  (submit contracts s (.issueEffect ⟨1⟩)).isSome &&
  (environment contracts s (.withdrawProcedure proc.revision)).any (fun t =>
    t.now < receipt.expires && t.receipts.contains receipt &&
    (findRun t ⟨1⟩).any (fun r => (currentGrant t r).isSome && !live t r) &&
    (submit contracts t (.issueEffect ⟨1⟩)).isNone))

def correctedContinuationRefused : Bool := claimed.any (fun s =>
  (environment contracts s .correctKnowledge).any (fun t =>
    t.authority.correctionGeneration == 1 &&
    (findRun t ⟨1⟩).any (fun r => r.correctionGeneration == 0 &&
      (currentGrant t r).isSome && !live t r) &&
    (submit contracts t (.issueEffect ⟨1⟩)).isNone))

def effectInputsReady : Option State := do
  let s ← started
  trace contracts s [.reserve ⟨1⟩ [⟨1⟩] 1, .recordReceipt receipt]
def changedProviderBindingRefused : Bool := effectInputsReady.any (fun s =>
  (submit contracts s (.prepare intent 1)).isSome &&
  (environment contracts s (.advanceUpstream ⟨2⟩)).any (fun t =>
    (submit contracts t (.prepare intent 1)).isNone &&
    (submit contracts t (.prepare { intent with expectedProvider := ⟨2⟩ } 1)).isNone))

theorem child_publish_nonvacuous : childPublishRefused = true := by decide
theorem expired_grant_nonvacuous : expiredGrantRefused = true := by decide
theorem withdrawn_procedure_nonvacuous : withdrawnProcedureRefused = true := by decide
theorem corrected_continuation_nonvacuous : correctedContinuationRefused = true := by decide
theorem changed_provider_binding_nonvacuous : changedProviderBindingRefused = true := by decide

end Kctx.Engineering.Examples
