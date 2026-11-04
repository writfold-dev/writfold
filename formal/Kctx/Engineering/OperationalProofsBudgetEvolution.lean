import Kctx.Engineering.OperationalProofsBudgetActions
import Kctx.Engineering.OperationalProofsState
import Kctx.Engineering.EffectsSafetyStep
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Every existing account survives with its nominal root and limit unchanged.
This member-level relation does not require unique accounts or well-formedness;
new root activation may extend the account list, and debits only increase spent. -/
def BudgetEvolution (before after : OperationalState) : Prop :=
  ∀ account ∈ before.budgets, ∃ next ∈ after.budgets,
    next.root = account.root ∧ next.limit = account.limit ∧ account.spent ≤ next.spent

theorem BudgetEvolution.of_same {before after : OperationalState}
    (same : after.budgets = before.budgets) : BudgetEvolution before after := by
  intro account member
  exact ⟨account, same.symm ▸ member, rfl, rfl, Nat.le_refl _⟩

theorem BudgetEvolution.refl (state : OperationalState) : BudgetEvolution state state :=
  .of_same rfl

/-- Root activation prepends a new account without replacing any older member. -/
theorem BudgetEvolution.of_cons {before after : OperationalState} {fresh : RootBudget}
    (same : after.budgets = fresh :: before.budgets) : BudgetEvolution before after := by
  intro account member
  refine ⟨account, ?_, rfl, rfl, Nat.le_refl _⟩
  rw [same]
  exact List.mem_cons_of_mem fresh member

/-- The actual charge map preserves even duplicate-root members individually;
uniqueness is needed only when subsequently identifying an account by lookup. -/
theorem BudgetEvolution.of_debit {before after : OperationalState}
    {root : RunId} {cost : Nat}
    (same : after.budgets = before.budgets.map (fun old =>
      if old.root == root then { old with spent := old.spent + cost } else old)) :
    BudgetEvolution before after := by
  intro account member
  rw [same]
  refine ⟨_, List.mem_map.mpr ⟨account, member, rfl⟩, ?_⟩
  split <;> simp


/-- The source of the map is fixed by the successful charge equation, not by
an inferred intermediate state's budget projection. -/
private theorem actualCharge_budget_evolution {s t : State} {root : RunId} {cost : Nat}
    (actual : ActualCharge s t root cost) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  obtain ⟨charged, debit, budgets, _⟩ := actual
  have state := charge_exact_state debit
  subst charged
  exact BudgetEvolution.of_debit budgets

/-- Exhaust the actual reducer, including resume, retry, and the uncharged
effect-completion arm of `advanceNode`. Successful results either retain,
prepend, or debit budgets; no caller-supplied classification is used. -/
theorem domainBody_budget_evolution {contracts : Contracts} {s t : State}
    {action : Action} (h : domainBody contracts s action = some t) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  cases action with
  | activate run budget =>
      exact BudgetEvolution.of_cons (activation_initializes_one_account h).1
  | loadResource activation object =>
      obtain ⟨_, _, actual⟩ := loadResource_actual_charge h
      exact actualCharge_budget_evolution actual
  | failNode activation node receipt =>
      obtain ⟨_, _, _, _, actual⟩ := failNode_actual_charge h
      exact actualCharge_budget_evolution actual
  | selectRoute activation node route value =>
      obtain ⟨_, _, _, _, actual⟩ := selectRoute_actual_charge h
      exact actualCharge_budget_evolution actual
  | call parent node child callee binding =>
      obtain ⟨_, _, _, _, actual⟩ := call_actual_charge h
      exact actualCharge_budget_evolution actual
  | prepare intent generation =>
      obtain ⟨_, _, _, _, actual⟩ := prepare_actual_charge h
      exact actualCharge_budget_evolution actual
  | retryClaim effect claim expected record =>
      obtain ⟨_, _, _, _, _, _, actual⟩ := retryClaim_actual_charge h
      exact actualCharge_budget_evolution actual
  | rebind activation procedure binding invocation =>
      obtain ⟨_, _, actual⟩ := rebind_actual_charge h
      exact actualCharge_budget_evolution actual
  | resume activation =>
      obtain ⟨_, _, actual⟩ := resume_actual_charge h
      exact actualCharge_budget_evolution actual
  | advanceNode activation node value =>
      have executed : advanceNode contracts s activation node value = some t := h
      change advanceNode contracts s activation node value = some t at h
      unfold advanceNode at h
      obtain ⟨r, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨n, nodeFound, _⟩ := Option.bind_eq_some_iff.mp continuation
      cases form : n.form
      case effect cap =>
        exact BudgetEvolution.of_same
          (advanceNode_effect_budgets found nodeFound form executed)
      all_goals
        exact actualCharge_budget_evolution
          (advanceNode_actual_charge found nodeFound
            (by intro cap; simp [form]) executed)
  | _ =>
      simp only [domainBody, resolve, reserve, claimDispatch, issueEffect, observeEffect,
        observeMaterial, reconcile, reuse, returnChild, reopen, cancel, quiesce,
        propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
        retire] at h
      operational_success h
      all_goals exact BudgetEvolution.of_same rfl

/-- The domain writer boundary leaves the reducer's account list untouched. -/
theorem domain_budget_evolution {contracts : Contracts} {s t : State}
    {action : Action} (h : domain contracts s action = some t) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
      simp only [body, Option.map_some, Option.some.injEq] at h
      subst t
      exact domainBody_budget_evolution (t := next) body

/-- Projection, cuts, crash and recovery do not mutate shared root accounts. -/
theorem localAction_budget_evolution {contracts : Contracts} {s t : State}
    {action : Action} (h : localAction contracts s action = some t) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  cases action <;>
    simp only [localAction, project, acquireCut, releaseReadCut, crash, recover] at h
  all_goals operational_success h
  all_goals exact BudgetEvolution.of_same rfl

/-- External changes, including lost replies and provider completion, retain
all account members independently of coordinator liveness. -/
theorem environment_budget_evolution {contracts : Contracts} {s t : State}
    {action : Environment} (h : environment contracts s action = some t) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  unfold environment at h
  obtain ⟨next, body, result⟩ := Option.bind_eq_some_iff.mp h
  have budgets : next.budgets = s.budgets := by
    cases action <;> simp only [environmentBody, completeExternalEffect] at body
    all_goals operational_success body
    all_goals rfl
  cases result
  exact BudgetEvolution.of_same budgets

/-- Append replays the exact current operational inputs. Publication and dispatch
use the same domain reducer, and local requests use their dedicated reducer. -/
theorem step_budget_evolution {contracts : Contracts} {s t : State}
    {request : Request} (h : step contracts s request = some t) :
    BudgetEvolution s.toOperationalState t.toOperationalState :=
  step_operational_relation domain_budget_evolution localAction_budget_evolution h

/-- Every actual transition evolves existing accounts monotonically, including
external transitions and explicit refused self-steps, without a WF premise. -/
theorem transition_budget_evolution {contracts : Contracts} {s t : State}
    (h : Transition contracts s t) :
    BudgetEvolution s.toOperationalState t.toOperationalState := by
  cases h with
  | operational executed => exact step_budget_evolution executed
  | external executed => exact environment_budget_evolution executed
  | refused => exact BudgetEvolution.refl s.toOperationalState

end Kctx.Engineering
