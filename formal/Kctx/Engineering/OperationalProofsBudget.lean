import Kctx.Engineering.OperationalProofs

set_option autoImplicit false
namespace Kctx.Engineering

/-- A successful debit preserves the selected nominal account and its limit;
this is an equation of the actual `charge` implementation, not an allowance
copied into a child invocation. -/
theorem charge_account_lookup {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) :
    ∃ account, s.budgets.find? (·.root == root) = some account ∧
      account.root = root ∧ 0 < cost ∧ account.spent + cost ≤ account.limit ∧
      t.budgets.find? (·.root == root) =
        some { account with spent := account.spent + cost } := by
  obtain ⟨account, found, positive, bound, budgets⟩ := charge_has_bounded_positive_cost h
  have identity : account.root = root := by simpa using List.find?_some found
  refine ⟨account, found, identity, positive, bound, ?_⟩
  rw [budgets, List.find?_map]
  have predicate : ((fun old : RootBudget => old.root == root) ∘
      (fun old => if old.root == root then { old with spent := old.spent + cost } else old)) =
      (fun old => old.root == root) := by
    funext old
    by_cases same : old.root = root <;> simp [Function.comp_def, same]
  rw [predicate, found]
  simp [identity]

/-- A trace of counted charges of one shared account. Every edge is an actual
Request/Environment transition; the debit witness is the actual internal
`charge` call. Uncounted work must leave this account's lookup unchanged, so
other roots and arbitrary nesting do not consume a second allowance. -/
inductive SharedAccountTrace (contracts : Contracts) (root : RunId) :
    State → Nat → State → Prop where
  | nil (state : State) : SharedAccountTrace contracts root state 0 state
  | unchanged {s t final : State} {count : Nat} :
      Transition contracts s t →
      t.budgets.find? (·.root == root) = s.budgets.find? (·.root == root) →
      SharedAccountTrace contracts root t count final →
      SharedAccountTrace contracts root s count final
  | charge {s t charged final : State} {count cost : Nat} :
      Transition contracts s t → charge s root cost = some charged →
      t.budgets.find? (·.root == root) = charged.budgets.find? (·.root == root) →
      SharedAccountTrace contracts root t count final →
      SharedAccountTrace contracts root s (count + 1) final

theorem shared_account_trace_consumption {contracts : Contracts} {root : RunId}
    {s t : State} {count : Nat} (trace : SharedAccountTrace contracts root s count t)
    {account : RootBudget} (found : s.budgets.find? (·.root == root) = some account)
    (initialBound : account.spent ≤ account.limit) :
    ∃ finalAccount, t.budgets.find? (·.root == root) = some finalAccount ∧
      finalAccount.root = account.root ∧ finalAccount.limit = account.limit ∧
      BudgetConsumption account.limit account.spent count finalAccount.spent := by
  induction trace generalizing account with
  | nil state => exact ⟨account, found, rfl, rfl, .nil _ initialBound⟩
  | unchanged _ unchanged _ ih =>
      exact ih (unchanged.trans found) initialBound
  | charge _ debited selected _ ih =>
      obtain ⟨before, beforeFound, _, positive, bound, afterFound⟩ := charge_account_lookup debited
      have same : before = account := Option.some.inj (beforeFound.symm.trans found)
      subst before
      obtain ⟨finalAccount, finalFound, rootEq, limitEq, consumed⟩ :=
        ih (selected.trans afterFound) bound
      exact ⟨finalAccount, finalFound, rootEq, limitEq, .step positive bound consumed⟩

/-- E10's bound counts arbitrarily many nested traversals, rather than merely
proving an inequality for a parent and its immediate child. -/
theorem shared_account_trace_bounded {contracts : Contracts} {root : RunId}
    {s t : State} {count : Nat} (wf : OperationalWellFormed contracts s.toOperationalState)
    (trace : SharedAccountTrace contracts root s count t)
    {account : RootBudget} (found : s.budgets.find? (·.root == root) = some account) :
    account.spent + count ≤ account.limit := by
  have member := List.mem_of_find?_eq_some found
  obtain ⟨_, _, _, _, consumption⟩ :=
    shared_account_trace_consumption trace found (wf.accounts_bounded account member)
  have bounds := budget_consumption_bounded consumption
  omega

end Kctx.Engineering
