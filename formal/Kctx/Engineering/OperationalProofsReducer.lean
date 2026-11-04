import Kctx.Engineering.OperationalProofsState

set_option autoImplicit false
namespace Kctx.Engineering

/-- Decompose the successful path of an already unfolded reducer. Guards are
extracted at the Option boundary before any match inside them can be split.
State-valued selections are resolved before their continuation: in particular,
`charge` stays opaque until its exact-state theorem substitutes the debit.
Lookup equations for runs, nodes and other records remain available to callers.
Every recursive step consumes a bind, a guard or an actual match; the terminal
some equation is consumed, so callers must not eliminate it a second time. -/
syntax "operational_success " ident : tactic

macro_rules
  | `(tactic| operational_success $h:ident) => `(tactic| (
      first
      | contradiction
      | (have exactState := charge_exact_state $h
         obtain ⟨account, accountFound, positiveCost, debitBound, _⟩ :=
           charge_has_bounded_positive_cost $h
         clear $h
         cases exactState)
      | (obtain ⟨guard, continuation⟩ := Option.ite_none_right_eq_some.mp $h
         clear $h
         operational_success continuation)
      | (obtain ⟨selected, selectedBy, continuation⟩ := Option.bind_eq_some_iff.mp $h
         clear $h
         first
         | (have selectedState : State := selected
            clear selectedState
            operational_success selectedBy)
         | skip
         all_goals operational_success continuation)
      | (split at $h:ident <;> operational_success $h)
      | cases $h:ident
    ))

end Kctx.Engineering
