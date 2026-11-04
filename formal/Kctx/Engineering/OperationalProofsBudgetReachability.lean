import Kctx.Engineering.OperationalProofsPreservation
import Kctx.Engineering.OperationalProofsBudgetEvolution
import Kctx.Engineering.OperationalProofsBudgetActions

set_option autoImplicit false
namespace Kctx.Engineering

private theorem rootBudget_eq {a b : RootBudget}
    (root : a.root = b.root) (limit : a.limit = b.limit) (spent : a.spent = b.spent) : a = b := by
  cases a
  cases b
  simp_all

private theorem account_lookup_of_member {accounts : List RootBudget}
    (unique : (accounts.map RootBudget.root).Nodup) {account : RootBudget}
    (member : account ∈ accounts) : accounts.find? (·.root == account.root) = some account := by
  cases found : accounts.find? (·.root == account.root) with
  | none =>
      have absent := List.find?_eq_none.mp found account member
      simp at absent
  | some selected =>
      have selectedMember := List.mem_of_find?_eq_some found
      have sameRoot : selected.root = account.root := by simpa using List.find?_some found
      have same := member_eq_of_unique_key unique selectedMember member sameRoot
      simp_all only [same]

/-- Every actual transition preserves the selected existing account's identity
and limit and can only increase its spent amount. No edge classification is
supplied by the caller. -/
theorem transition_account_evolution {contracts : Contracts} {s t : State} {root : RunId}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (edge : Transition contracts s t) {account : RootBudget}
    (found : s.budgets.find? (·.root == root) = some account) :
    ∃ next, t.budgets.find? (·.root == root) = some next ∧
      next.root = account.root ∧ next.limit = account.limit ∧ account.spent ≤ next.spent := by
  have after := transition_preserves_operationalWellFormed wf edge
  obtain ⟨next, member, sameRoot, sameLimit, increasing⟩ :=
    transition_budget_evolution edge account (List.mem_of_find?_eq_some found)
  have rootEq : account.root = root := by simpa using List.find?_some found
  have lookup := account_lookup_of_member after.accounts_unique member
  exact ⟨next, by simpa only [sameRoot, rootEq] using lookup, sameRoot, sameLimit, increasing⟩

theorem step_account_evolution {contracts : Contracts} {s t : State} {root : RunId}
    {request : Request} (wf : OperationalWellFormed contracts s.toOperationalState)
    (executed : step contracts s request = some t) {account : RootBudget}
    (found : s.budgets.find? (·.root == root) = some account) :
    ∃ next, t.budgets.find? (·.root == root) = some next ∧
      next.root = account.root ∧ next.limit = account.limit ∧ account.spent ≤ next.spent :=
  transition_account_evolution wf (.operational executed) found

theorem environment_account_evolution {contracts : Contracts} {s t : State} {root : RunId}
    {action : Environment} (wf : OperationalWellFormed contracts s.toOperationalState)
    (executed : environment contracts s action = some t) {account : RootBudget}
    (found : s.budgets.find? (·.root == root) = some account) :
    ∃ next, t.budgets.find? (·.root == root) = some next ∧
      next.root = account.root ∧ next.limit = account.limit ∧ account.spent ≤ next.spent :=
  transition_account_evolution wf (.external executed) found

/-- Strict increases are represented by the actual `charge` function applied
to their positive spent difference. Its guard follows from actual successor
account bounds, not an extra interpreter guard or a caller's arithmetic premise. -/
theorem transition_account_classified {contracts : Contracts} {s t : State} {root : RunId}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (edge : Transition contracts s t) {account : RootBudget}
    (found : s.budgets.find? (·.root == root) = some account) :
    (t.budgets.find? (·.root == root) = s.budgets.find? (·.root == root)) ∨
    ∃ cost charged, charge s root cost = some charged ∧
      t.budgets.find? (·.root == root) = charged.budgets.find? (·.root == root) := by
  obtain ⟨next, selected, rootEq, limitEq, increasing⟩ := transition_account_evolution wf edge found
  by_cases sameSpent : next.spent = account.spent
  · have same := rootBudget_eq rootEq limitEq sameSpent
    exact Or.inl ((selected.trans (congrArg some same)).trans found.symm)
  · have after := transition_preserves_operationalWellFormed wf edge
    have bound := after.accounts_bounded next (List.mem_of_find?_eq_some selected)
    let cost := next.spent - account.spent
    have positive : 0 < cost := by dsimp [cost]; omega
    have spentEq : account.spent + cost = next.spent := by dsimp [cost]; omega
    have allowed : account.spent + cost ≤ account.limit := by omega
    let charged : State := { s with budgets := s.budgets.map (fun old =>
      if old.root == root then { old with spent := old.spent + cost } else old) }
    have debited : charge s root cost = some charged := by
      simp [charge, found, positive, allowed, charged]
    obtain ⟨before, beforeFound, _, _, _, afterFound⟩ := charge_account_lookup debited
    have sameBefore : before = account := Option.some.inj (beforeFound.symm.trans found)
    subst before
    have sameAfter : next = { account with spent := account.spent + cost } :=
      rootBudget_eq rootEq limitEq spentEq.symm
    exact Or.inr ⟨cost, charged, debited,
      (selected.trans (congrArg some sameAfter)).trans afterFound.symm⟩

/-- A genuinely successful debit cannot disappear into the unchanged branch
of the automatically constructed account trace. -/
theorem charged_transition_changes_account {s t charged : State} {root : RunId} {cost : Nat}
    (debited : charge s root cost = some charged)
    (selected : t.budgets.find? (·.root == root) = charged.budgets.find? (·.root == root)) :
    t.budgets.find? (·.root == root) ≠ s.budgets.find? (·.root == root) := by
  obtain ⟨account, found, _, positive, _, afterFound⟩ := charge_account_lookup debited
  intro unchanged
  have same : ({ account with spent := account.spent + cost } : RootBudget) = account :=
    Option.some.inj (afterFound.symm.trans (selected.symm.trans (unchanged.trans found)))
  have spent := congrArg RootBudget.spent same
  simp only at spent
  omega

/-- Replay reconstructs a ledger, not a different account. A debit extracted
from its actual reducer therefore transports to the originating Request. -/
theorem ActualCharge.rebase {s t source target : State} {root : RunId} {cost : Nat}
    (actual : ActualCharge s t root cost) (before : source.budgets = s.budgets)
    (after : target.budgets = t.budgets) : ActualCharge source target root cost := by
  obtain ⟨oldCharged, oldDebit, oldBudgets, _⟩ := actual
  obtain ⟨account, found, positive, bound, mapped⟩ := charge_has_bounded_positive_cost oldDebit
  let charged : State := { source with budgets := source.budgets.map (fun old =>
    if old.root == root then { old with spent := old.spent + cost } else old) }
  have debit : charge source root cost = some charged := by
    simp [charge, before, found, positive, bound, charged]
  refine ⟨charged, debit, ?_, positive⟩
  change target.budgets = source.budgets.map _
  rw [after, oldBudgets, mapped, before]

theorem ActualCharge.counted_transition {contracts : Contracts} {s t : State}
    {root : RunId} {cost : Nat} (actual : ActualCharge s t root cost)
    (edge : Transition contracts s t) :
    SharedAccountTrace contracts root s 1 t ∧
      t.budgets.find? (·.root == root) ≠ s.budgets.find? (·.root == root) := by
  obtain ⟨charged, debit, budgets, _⟩ := actual
  have selected : t.budgets.find? (·.root == root) =
      charged.budgets.find? (·.root == root) := by rw [budgets]
  exact ⟨.charge edge debit selected (.nil t), charged_transition_changes_account debit selected⟩

theorem append_loadResource_counted {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {activation : ActivationId} {object : ObjectId}
    (h : step contracts s (.append stream expected record (.loadResource activation object)) = some t) :
    ∃ run, findRun s activation = some run ∧
      SharedAccountTrace contracts run.id s 1 t ∧
      t.budgets.find? (·.root == run.id) ≠ s.budgets.find? (·.root == run.id) := by
  obtain ⟨before, after, source, executed, budgets⟩ := append_actual_domainBody h
  obtain ⟨run, found, actual⟩ := loadResource_actual_charge executed
  have sourceBudgets := congrArg OperationalState.budgets source
  have sourceRuns := congrArg OperationalState.runs source
  have actualRequest := actual.rebase sourceBudgets.symm budgets
  exact ⟨run, by simpa only [findRun, sourceRuns] using found,
    actualRequest.counted_transition (.operational h)⟩

theorem SharedAccountTrace.append {contracts : Contracts} {root : RunId}
    {s middle t : State} {left right : Nat}
    (first : SharedAccountTrace contracts root s left middle)
    (second : SharedAccountTrace contracts root middle right t) :
    SharedAccountTrace contracts root s (left + right) t := by
  induction first generalizing right t with
  | nil => simpa using second
  | unchanged edge selected _ ih => exact .unchanged edge selected (ih second)
  | charge edge debited selected _ ih =>
      have joined := SharedAccountTrace.charge edge debited selected (ih second)
      simpa only [Nat.add_assoc, Nat.add_comm, Nat.add_left_comm] using joined

/-- Every Reachable suffix starting with an existing account admits a complete
shared-account trace. Refusal, environment actions, other-root activity and
physical-send boundaries are included, not omitted from the induction. -/
theorem reachable_constructs_shared_account_trace {contracts : Contracts} {s t : State}
    {root : RunId} (wf : OperationalWellFormed contracts s.toOperationalState)
    {account : RootBudget} (found : s.budgets.find? (·.root == root) = some account)
    (reach : Reachable contracts s t) :
    ∃ count finalAccount, SharedAccountTrace contracts root s count t ∧
      t.budgets.find? (·.root == root) = some finalAccount ∧
      finalAccount.root = account.root ∧ finalAccount.limit = account.limit ∧
      account.spent + count ≤ finalAccount.spent ∧ finalAccount.spent ≤ account.limit := by
  induction reach with
  | initial =>
      exact ⟨0, account, .nil s, found, rfl, rfl, by omega,
        wf.accounts_bounded account (List.mem_of_find?_eq_some found)⟩
  | @next before after beforeReach edge ih =>
      obtain ⟨count, current, trace, currentFound, currentRoot, currentLimit, _, _⟩ := ih
      have beforeWF := reachable_operational_wellFormed wf beforeReach
      have classified := transition_account_classified beforeWF edge currentFound
      have extension : ∃ total, SharedAccountTrace contracts root s total after := by
        rcases classified with unchanged | ⟨cost, charged, debited, selected⟩
        · exact ⟨count, by simpa using trace.append (.unchanged edge unchanged (.nil after))⟩
        · exact ⟨count + 1, trace.append (.charge edge debited selected (.nil after))⟩
      obtain ⟨total, complete⟩ := extension
      obtain ⟨final, selected, finalRoot, finalLimit, consumed⟩ :=
        shared_account_trace_consumption complete found
          (wf.accounts_bounded account (List.mem_of_find?_eq_some found))
      have bounds := budget_consumption_bounded consumed
      exact ⟨total, final, complete, selected, finalRoot, finalLimit, bounds.1, bounds.2⟩

/-- E10 now quantifies over every actual suffix after activation, with no
caller-supplied SharedAccountTrace or debit/unchanged classification. -/
theorem E10_every_reachable_suffix_bounded {contracts : Contracts} {s t : State}
    {root : RunId} (wf : OperationalWellFormed contracts s.toOperationalState)
    {account : RootBudget} (found : s.budgets.find? (·.root == root) = some account)
    (reach : Reachable contracts s t) :
    ∃ count, SharedAccountTrace contracts root s count t ∧ account.spent + count ≤ account.limit := by
  obtain ⟨count, _, trace, _, _, _, lower, upper⟩ :=
    reachable_constructs_shared_account_trace wf found reach
  exact ⟨count, trace, by omega⟩

/-- Activation creates the sole zero-spent root account. Every later actual
transition is automatically included in the traversal bounded by that limit. -/
theorem E10_after_activation_bounded {contracts : Contracts} {before active final : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {run : Run} {budget : Nat} (wf : OperationalWellFormed contracts before.toOperationalState)
    (activated : step contracts before (.append stream expected record (.activate run budget)) = some active)
    (reach : Reachable contracts active final) :
    ∃ count, SharedAccountTrace contracts run.id active count final ∧ count ≤ budget := by
  have activeWF := step_preserves_operationalWellFormed wf activated
  have selected := append_activation_account activated
  obtain ⟨count, trace, bound⟩ := E10_every_reachable_suffix_bounded activeWF selected reach
  exact ⟨count, trace, by simpa using bound⟩

end Kctx.Engineering
