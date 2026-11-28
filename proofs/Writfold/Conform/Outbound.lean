import Writfold.Conform.Site
import Writfold.Outbound

/-!
# Links: the outbound remainder is law S, folded (#187)

Each member of a call is a site's basis (`classBasis`: a local command is
no match, a routed or listed one is judged, an opaque or unclassified one
is undecided) and the `unclassified` policy is the site's posture
(`policyPosture`, onto): `delivery::outbound::contribution` is
`Calculus.Site.decide`, and `outbound` joins the members' decisions with
`Calculus.Decision.join`.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Guard (combineOf)

namespace Writfold.Core.Conform

/-- A member of a call as a site's basis. -/
def classBasis : delivery.outbound.Class → Calculus.Basis
  | .Local => .noMatch
  | .Routed d => .judged (decisionOf d)
  | .Listed d => .judged (decisionOf d)
  | .Opaque => .undecided
  | .Unclassified => .undecided

/-- The `unclassified` policy as a posture: `log` is `open` that keeps a line. -/
def policyPosture : delivery.outbound.Unclassified → Calculus.Posture
  | .Pass => .open
  | .Log => .open
  | .Ask => .ask
  | .Deny => .deny

theorem policyPosture_onto (q : Calculus.Posture) : ∃ p, policyPosture p = q := by
  cases q
  · exact ⟨.Pass, rfl⟩
  · exact ⟨.Ask, rfl⟩
  · exact ⟨.Deny, rfl⟩

theorem decisionOf_combine (a b : guard.Decision) :
    decisionOf (combineOf a b) = Calculus.Decision.join (decisionOf a) (decisionOf b) := by
  cases a <;> cases b <;> rfl

/-- What an unclassified member gets is the site's decision for an
undecided event under the policy's posture. -/
theorem outbound_left_over_implements (p : delivery.outbound.Unclassified) :
    delivery.outbound.left_over p ⦃ d =>
      decisionOf d = Calculus.Site.decide .undecided (policyPosture p) ⦄ :=
  WP.spec_mono (Core.Delivery.left_over_spec p) (fun d hd => by
    subst hd
    cases p <;> rfl)

/-- One member's contribution is the site's decision for its basis. -/
theorem outbound_contribution_implements (c : delivery.outbound.Class) (p : delivery.outbound.Unclassified) :
    delivery.outbound.contribution c p ⦃ d =>
      decisionOf d = Calculus.Site.decide (classBasis c) (policyPosture p) ⦄ :=
  WP.spec_mono (Core.Delivery.contribution_spec c p) (fun d hd => by
    subst hd
    cases c <;> cases p <;> rfl)

theorem foldl_decisionOf (acc : guard.Decision) (xs : List guard.Decision) :
    decisionOf (xs.foldl combineOf acc) = (xs.map decisionOf).foldl Calculus.Decision.join (decisionOf acc) := by
  induction xs generalizing acc with
  | nil => rfl
  | cons x xs ih => simp [ih, decisionOf_combine]

/-- A call's decision joins its members' site decisions. -/
theorem outbound_implements (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified) :
    delivery.outbound.outbound cs p ⦃ d => decisionOf d =
      (cs.val.map fun c => Calculus.Site.decide (classBasis c) (policyPosture p)).foldl
        Calculus.Decision.join .pass ⦄ :=
  WP.spec_mono (Core.Delivery.outbound_spec cs p) (fun d hd => by
    rw [hd, Core.Delivery.outboundOf, foldl_decisionOf, List.map_map]
    congr 1
    apply List.map_congr_left
    intro c _
    cases c <;> cases p <;> rfl)

end Writfold.Core.Conform
