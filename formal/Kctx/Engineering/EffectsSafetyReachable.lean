import Kctx.Engineering.OperationalProofsPreservation
import Kctx.Engineering.EffectsSafetyOwnershipTransitions

set_option autoImplicit false
namespace Kctx.Engineering

/-- End-to-end exclusive ownership starts at the checked initializer. The
caller supplies no representation invariant: E02 discharges it over the actual
Request/Environment/refusal trace. Equality fixes both ownership generations. -/
theorem E06_reachable_exclusive_conflicting_ownership {contracts : Contracts}
    {input : InitialInputs} {initial s : State}
    (initialized : init contracts input = some initial) (reachable : Reachable contracts initial s)
    {left right : Reservation} (leftOwned : left ∈ s.ownership) (rightOwned : right ∈ s.ownership)
    (sameResource : left.resource = right.resource) : left = right :=
  E06_exclusive_conflicting_ownership (E02_representation_preserved initialized reachable)
    leftOwned rightOwned sameResource

end Kctx.Engineering
