import Kctx.Engineering.FixtureCalculus

set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 8000000
namespace Kctx.Engineering.Examples

/-- Separate original envelopes bind the first grant and its later renewal. -/
def installedGrantAuthorization : GrantAuthorization :=
  ⟨⟨1⟩, ⟨1⟩, { grant 4 1 with validFrom := 10 }, ⟨40⟩⟩
def renewedGrantAuthorization : GrantAuthorization :=
  ⟨⟨1⟩, ⟨1⟩, { grant 5 1 with validFrom := 10, budget := 200 }, ⟨41⟩⟩
def grantInputCapture : RetainedObservation := { capture with objects := [⟨40⟩, ⟨41⟩] }
def grantInstallationContracts : Contracts := { contracts with
  grantAuthorizations := [installedGrantAuthorization, renewedGrantAuthorization]
  materialObservations := [grantInputCapture] }
def initiallyUngrantedInputs : InitialInputs := { inputs with
  authority := { inputs.authority with grants := [] } }

/-- One fixed contract from checked initialization; actual capture and ledger
append retain the original inputs before the external authority transition. -/
def grantReadyWith (c : Contracts) : Option State := do
  let s ← init c initiallyUngrantedInputs
  trace c s [.resolve proc, .observeMaterial grantInputCapture]
def grantReady : Option State := grantReadyWith grantInstallationContracts

def installedGrantClaimed : Option State := do
  let s ← grantReady
  let authorized ← environment grantInstallationContracts s
    (.installGrant installedGrantAuthorization)
  let prepared ← trace grantInstallationContracts authorized
    [.activate run 30, .reserve ⟨1⟩ [⟨1⟩] 1, .recordReceipt receipt, .prepare intent 1]
  claim grantInstallationContracts prepared

def installedGrantIssued : Option State := installedGrantClaimed.bind (fun s =>
  submit grantInstallationContracts s (.issueEffect ⟨1⟩))

def postInitGrantDispatch : Bool :=
  grantReady.any (fun s => s.authority.grants.isEmpty &&
    (submit grantInstallationContracts s (.activate run 30)).isNone) &&
  installedGrantIssued.any (fun s =>
    (findEffect s ⟨1⟩).any (fun e => e.dispatch.any (fun d =>
      d.issued && d.grant == installedGrantAuthorization.grant.id)))

/-- The bad authorization is genuinely qualified in this fixture: scope/time
and retained-input guards must still reject it, not merely membership lookup. -/
def grantInstallationRefuses (authorization : GrantAuthorization) : Bool :=
  let c := { grantInstallationContracts with grantAuthorizations := [authorization] }
  (grantReadyWith c).any (fun s => (environment c s (.installGrant authorization)).isNone)

def unqualifiedGrantInstallationRefused : Bool :=
  let c := { grantInstallationContracts with grantAuthorizations := [] }
  (grantReadyWith c).any (fun s =>
    (environment c s (.installGrant installedGrantAuthorization)).isNone) &&
  grantReady.any (fun s => (environment grantInstallationContracts s
    (.installGrant { installedGrantAuthorization with
      grant := { installedGrantAuthorization.grant with resources := [⟨3⟩] } })).isNone)

def foreignGrantRealmRefused : Bool :=
  grantInstallationRefuses { installedGrantAuthorization with realm := ⟨2⟩ } &&
  grantInstallationRefuses { installedGrantAuthorization with
    grant := { installedGrantAuthorization.grant with
      binding := { binding with realm := ⟨2⟩ } } }

def staleGrantPolicyRefused : Bool :=
  grantInstallationRefuses { installedGrantAuthorization with policy := ⟨2⟩ }

def missingGrantEnvelopeRefused : Bool :=
  grantInstallationRefuses { installedGrantAuthorization with envelope := ⟨42⟩ }

def invalidGrantIntervalRefused : Bool :=
  grantInstallationRefuses { installedGrantAuthorization with
    grant := { installedGrantAuthorization.grant with validFrom := 11 } } &&
  grantInstallationRefuses { installedGrantAuthorization with
    grant := { installedGrantAuthorization.grant with expires := 10 } } &&
  grantInstallationRefuses { installedGrantAuthorization with
    grant := { installedGrantAuthorization.grant with revoked := true } }

def revokedGrantIdCannotBeReinstalled : Bool := installedGrantClaimed.any (fun s =>
  (environment grantInstallationContracts s (.installGrant installedGrantAuthorization)).isNone &&
  (environment grantInstallationContracts s
    (.revokeGrant installedGrantAuthorization.grant.id)).any (fun t =>
      (environment grantInstallationContracts t
        (.installGrant installedGrantAuthorization)).isNone))

def renewGrantForClaim (s : State) : Option State := do
  let revoked ← environment grantInstallationContracts s
    (.revokeGrant installedGrantAuthorization.grant.id)
  environment grantInstallationContracts revoked (.installGrant renewedGrantAuthorization)
def renewedGrantClaimed : Option State := installedGrantClaimed.bind renewGrantForClaim

def renewedGrantCannotRefreshClaim : Bool := installedGrantClaimed.any (fun before =>
  before.budgets.any (fun account => account.spent > 0) &&
  (renewGrantForClaim before).any (fun after =>
    after.budgets == before.budgets && after.runs == before.runs &&
    after.claims == before.claims && after.effects == before.effects &&
    (findRun after ⟨1⟩).any (fun r =>
      (currentGrant after r).any (·.id == renewedGrantAuthorization.grant.id)) &&
    (submit grantInstallationContracts after (.issueEffect ⟨1⟩)).isNone))

theorem post_init_grant_nonvacuous : postInitGrantDispatch = true := by decide
theorem unqualified_grant_installation_nonvacuous :
    unqualifiedGrantInstallationRefused = true := by decide
theorem foreign_grant_realm_nonvacuous : foreignGrantRealmRefused = true := by decide
theorem stale_grant_policy_nonvacuous : staleGrantPolicyRefused = true := by decide
theorem missing_grant_envelope_nonvacuous : missingGrantEnvelopeRefused = true := by decide
theorem invalid_grant_interval_nonvacuous : invalidGrantIntervalRefused = true := by decide
theorem revoked_grant_id_nonvacuous : revokedGrantIdCannotBeReinstalled = true := by decide
theorem renewed_grant_claim_nonvacuous : renewedGrantCannotRefreshClaim = true := by decide

end Kctx.Engineering.Examples
