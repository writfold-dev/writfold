import Kctx.Engineering.OperationalProofsClaimsLists

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

theorem claims_findEffect_member {s : State} {id : Id .effect} {effect : Effect}
    (found : findEffect s id = some effect) : effect ∈ s.effects :=
  List.mem_of_find?_eq_some found

theorem claims_fresh {claims : List Dispatch} {id : Id .claim}
    (fresh : claims.any (fun claim => claim.id == id) = false) :
    ∀ previous ∈ claims, previous.id ≠ id := by
  intro previous member same
  have absent := List.any_eq_false.mp fresh previous member
  simp [same] at absent

/-- Every successful reducer branch preserves the authoritative inventory.
The two claim branches use their actual freshness guard; issue transports the
issued bit in both places. Observe/reconcile/cancel change only outcomes. -/
theorem operationalClaims_domainBody {contracts : Contracts} {s t : State} {action : Action}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    OperationalClaims (frameDomain s action t).toOperationalState := by
  have before := wf.claimFields.inventory
  records_split_domain h action
  all_goals apply OperationalClaims.of_inventory
  all_goals first
    | exact before
    | exact claims_inventory_prepare before _ _
    | (apply claims_inventory_issue wf
       · exact claims_findEffect_member (by assumption)
       · assumption)
    | (apply claims_inventory_prepend before wf.effects_unique
       · exact claims_findEffect_member (by assumption)
       · rfl
       · rfl
       · apply claims_fresh
         simp only [Bool.and_eq_true, Bool.not_eq_true] at *
         grind only)
    | (apply claims_inventory_outcome before wf.effects_unique
       exact claims_findEffect_member (by assumption))
    | (apply claims_inventory_preserve_dispatch before
       · intro effect member
         dsimp only
         split <;> exact ⟨rfl, rfl⟩
       · intro effect member
         dsimp only
         split <;> rfl)

/-- Domain preservation is derived from the checked BEFORE state and the actual
interpreter transition, never from an assumed post-state inventory. -/
theorem domain_preserves_operationalClaims {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalClaims t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    exact operationalClaims_domainBody wf body

end Kctx.Engineering
