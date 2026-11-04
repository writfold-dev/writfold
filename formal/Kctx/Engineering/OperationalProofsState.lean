import Kctx.Engineering.Actions
import Kctx.Engineering.ControlProofs

set_option autoImplicit false
namespace Kctx.Engineering

/-- The actual debit changes only the shared budget list. Success extraction
keeps the Option boundary intact instead of splitting inside a guard. -/
theorem charge_exact_state {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) :
    t = { s with budgets := s.budgets.map (fun old =>
      if old.root == root then { old with spent := old.spent + cost } else old) } := by
  unfold charge at h
  obtain ⟨account, _, checked⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, result⟩ := Option.ite_none_right_eq_some.mp checked
  exact Option.some.inj result.symm

end Kctx.Engineering
