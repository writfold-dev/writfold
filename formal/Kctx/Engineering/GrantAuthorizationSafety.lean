import Kctx.Engineering.Environment

set_option autoImplicit false
namespace Kctx.Engineering

/-- The exact authorization is a qualified input: membership does not implement
signature verification or establish the signer's authority. The integration
boundary must authenticate the operator, bind the complete realm/policy/grant
payload to the retained envelope, and populate `grantAuthorizations` soundly.
The interpreter independently checks current scope, time and fresh identity. -/
structure GrantInstallationChecked (contracts : Contracts) (s : State)
    (authorization : GrantAuthorization) : Prop where
  qualified : authorization ∈ contracts.grantAuthorizations
  realm : authorization.realm = s.realm
  binding_realm : authorization.grant.binding.realm = s.realm
  policy : authorization.policy = s.authority.policy
  retained : authorization.envelope ∈ s.retained
  fresh : authorization.grant.id ∉ s.authority.grants.map Grant.id
  unrevoked : authorization.grant.revoked = false
  validFrom : authorization.grant.validFrom ≤ s.now
  expires : s.now < authorization.grant.expires

/-- Successful admission prepends the exact authorized grant, with no other
state mutation. In particular, historical grants are not replaced or revived. -/
theorem installGrant_body_checked {contracts : Contracts} {s t : State}
    {authorization : GrantAuthorization}
    (h : environmentBody contracts s (.installGrant authorization) = some t) :
    GrantInstallationChecked contracts s authorization ∧
      t = { s with authority := { s.authority with
        grants := authorization.grant :: s.authority.grants } } := by
  simp only [environmentBody] at h
  split at h
  next guard =>
    cases h
    simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem,
      Bool.not_eq_true, List.contains_eq_mem, decide_eq_false_iff_not,
      decide_eq_true_eq, and_assoc] at guard
    rcases guard with ⟨qualified, realm, binding, policy, retained, fresh,
      unrevoked, validFrom, expires⟩
    have freshId : authorization.grant.id ∉ s.authority.grants.map Grant.id := by
      intro member
      simp [member] at fresh
    have isUnrevoked : authorization.grant.revoked = false := by
      cases value : authorization.grant.revoked
      · rfl
      · simp [value] at unrevoked
    exact ⟨⟨qualified, realm, binding, policy, retained, freshId, isUnrevoked,
      validFrom, expires⟩, rfl⟩
  next => contradiction

/-- Ordinary environment framing does not weaken the admission checks or
change the exact installation result. -/
theorem installGrant_checked {contracts : Contracts} {s t : State}
    {authorization : GrantAuthorization}
    (h : environment contracts s (.installGrant authorization) = some t) :
    GrantInstallationChecked contracts s authorization ∧
      t = { s with authority := { s.authority with
        grants := authorization.grant :: s.authority.grants } } := by
  cases body : environmentBody contracts s (.installGrant authorization) with
  | none => simp [environment, body] at h
  | some next =>
      obtain ⟨checked, same⟩ := installGrant_body_checked body
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst next
      cases h
      exact ⟨checked, rfl⟩

/-- Every old grant survives verbatim, including revoked entries and its entire
capability/resource scope; no fresh identity can alias a historical grant. -/
theorem installGrant_preserves_existing_grants {contracts : Contracts} {s t : State}
    {authorization : GrantAuthorization} {old : Grant}
    (h : environment contracts s (.installGrant authorization) = some t)
    (member : old ∈ s.authority.grants) :
    old ∈ t.authority.grants ∧ old.id ≠ authorization.grant.id := by
  obtain ⟨checked, rfl⟩ := installGrant_checked h
  refine ⟨List.mem_cons_of_mem _ member, ?_⟩
  intro same
  exact checked.fresh (List.mem_map.mpr ⟨old, member, same⟩)

/-- Admission neither resets root spending nor creates an activation, resource
reservation, effect or dispatch claim. Existing ledger and retained evidence
are also unchanged; retaining an envelope is a prerequisite, not a side effect. -/
theorem installGrant_history_frame {contracts : Contracts} {s t : State}
    {authorization : GrantAuthorization}
    (h : environment contracts s (.installGrant authorization) = some t) :
    t.budgets = s.budgets ∧ t.runs = s.runs ∧ t.ownership = s.ownership ∧
      t.effects = s.effects ∧ t.claims = s.claims ∧ t.ledger = s.ledger ∧
      t.retained = s.retained := by
  obtain ⟨_, rfl⟩ := installGrant_checked h
  exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

end Kctx.Engineering
