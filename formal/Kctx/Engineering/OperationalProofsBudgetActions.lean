import Kctx.Engineering.OperationalProofsBudget
import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
namespace Kctx.Engineering

/-- An actual internal debit, with the exact final budget list. This witness is
extracted from successful reducers, rather than supplied as a trace premise. -/
def ActualCharge (s t : State) (root : RunId) (cost : Nat) : Prop :=
  ∃ charged, charge s root cost = some charged ∧
    t.budgets = charged.budgets ∧ 0 < cost

private theorem actualCharge_of_charge {s t charged : State} {root : RunId} {cost : Nat}
    (debit : charge s root cost = some charged)
    (budgets : t.budgets = charged.budgets) : ActualCharge s t root cost := by
  obtain ⟨_, _, positive, _, _⟩ := charge_has_bounded_positive_cost debit
  exact ⟨charged, debit, budgets, positive⟩

/-- Every extracted debit strictly increases the selected root's spent counter,
retaining its identity and limit, and stays within that same account's limit. -/
theorem ActualCharge.account_lookup {s t : State} {root : RunId} {cost : Nat}
    (actual : ActualCharge s t root cost) :
    ∃ account, s.budgets.find? (·.root == root) = some account ∧
      account.root = root ∧ 0 < cost ∧ account.spent + cost ≤ account.limit ∧
      t.budgets.find? (·.root == root) =
        some { account with spent := account.spent + cost } := by
  obtain ⟨charged, debit, budgets, _⟩ := actual
  obtain ⟨account, found, identity, positive, bound, after⟩ :=
    charge_account_lookup debit
  exact ⟨account, found, identity, positive, bound, by rw [budgets]; exact after⟩

/-- Loading charges the selected run's shared root exactly one unit. -/
theorem loadResource_actual_charge {s t : State} {activation : ActivationId} {object : ObjectId}
    (h : loadResource s activation object = some t) :
    ∃ r, findRun s activation = some r ∧ ActualCharge s t r.id 1 := by
  unfold loadResource at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨r, found, actualCharge_of_charge debit rfl⟩

/-- Prepare charges exactly the selected effect node's cost, before dispatch. -/
theorem prepare_actual_charge {contracts : Contracts} {s t : State} {intent : Intent}
    {generation : Nat} (h : prepare contracts s intent generation = some t) :
    ∃ r n, findRun s intent.invocation = some r ∧
      findNode r.procedure intent.node = some n ∧ ActualCharge s t r.id n.cost := by
  unfold prepare at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨r, n, found, nodeFound, actualCharge_of_charge debit rfl⟩

/-- Authenticated non-success still consumes the selected node's positive cost. -/
theorem failNode_actual_charge {contracts : Contracts} {s t : State}
    {activation : ActivationId} {node : NodeId} {receipt : Receipt}
    (h : failNode contracts s activation node receipt = some t) :
    ∃ r n, findRun s activation = some r ∧ findNode r.procedure node = some n ∧
      ActualCharge s t r.id n.cost := by
  unfold failNode at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨r, n, found, nodeFound, actualCharge_of_charge debit rfl⟩

/-- Selecting a route charges its actual selection node, not the destination. -/
theorem selectRoute_actual_charge {contracts : Contracts} {s t : State}
    {activation : ActivationId} {node route : NodeId} {value : TypedValue}
    (h : selectRoute contracts s activation node route value = some t) :
    ∃ r n, findRun s activation = some r ∧ findNode r.procedure node = some n ∧
      ActualCharge s t r.id n.cost := by
  unfold selectRoute at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
  refine ⟨r, n, found, nodeFound, ?_⟩
  split at h <;> try contradiction
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact actualCharge_of_charge debit rfl

/-- A successful call debits the parent's shared root at the call node's cost;
constructing the child does not create a copied allowance. -/
theorem call_actual_charge {contracts : Contracts} {s t : State}
    {parent invocation : ActivationId} {node : NodeId} {callee : Procedure} {binding : Binding}
    (h : call contracts s parent node invocation callee binding = some t) :
    ∃ r n, findRun s parent = some r ∧ findNode r.procedure node = some n ∧
      ActualCharge s t r.id n.cost := by
  unfold call at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨grant, _, h⟩ := Option.bind_eq_some_iff.mp h
  refine ⟨r, n, found, nodeFound, ?_⟩
  split at h <;> try contradiction
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact actualCharge_of_charge debit rfl

/-- Rebinding retains the root and charges it exactly one unit. -/
theorem rebind_actual_charge {contracts : Contracts} {s t : State}
    {activation invocation : ActivationId} {procedure : Procedure} {binding : Binding}
    (h : rebind contracts s activation procedure binding invocation = some t) :
    ∃ r, findRun s activation = some r ∧ ActualCharge s t r.id 1 := by
  unfold rebind at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨grant, _, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨r, found, actualCharge_of_charge debit rfl⟩

/-- Resuming an existing activation charges its unchanged shared root one unit. -/
theorem resume_actual_charge {contracts : Contracts} {s t : State}
    {activation : ActivationId} (h : resume contracts s activation = some t) :
    ∃ r, findRun s activation = some r ∧ ActualCharge s t r.id 1 := by
  unfold resume at h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨grant, _, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨r, found, actualCharge_of_charge debit rfl⟩

/-- A fresh retry dispatch pays the immutable intent's selected node cost
against the owning activation's shared root before recording the claim. -/
theorem retryClaim_actual_charge {contracts : Contracts} {s t : State}
    {effect : Id .effect} {claim : Id .claim} {expected : Option Ledger.Head}
    {record : LedgerRecordId}
    (h : retryClaim contracts s effect claim expected record = some t) :
    ∃ e r n, findEffect s effect = some e ∧
      findRun s e.intent.invocation = some r ∧
      findNode r.procedure e.intent.node = some n ∧ ActualCharge s t r.id n.cost := by
  unfold retryClaim at h
  obtain ⟨e, effectFound, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨r, found, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨grant, _, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨n, nodeFound, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨authorization, _, h⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp h
  cases result
  exact ⟨e, r, n, effectFound, found, nodeFound, actualCharge_of_charge debit rfl⟩

/-- Read, judge, check, join and end advancement all debit the actual selected
node. Effect advancement is excluded: its debit already occurred in prepare. -/
theorem advanceNode_actual_charge {contracts : Contracts} {s t : State}
    {activation : ActivationId} {node : NodeId} {value : TypedValue} {r : Run} {n : Node}
    (found : findRun s activation = some r) (nodeFound : findNode r.procedure node = some n)
    (nonEffect : ∀ cap, n.form ≠ .effect cap)
    (h : advanceNode contracts s activation node value = some t) :
    ActualCharge s t r.id n.cost := by
  cases form : n.form
  all_goals first
    | exact False.elim (nonEffect _ form)
    | skip
  all_goals simp only [advanceNode, found, nodeFound, form, bind, Option.bind] at h
  all_goals obtain ⟨_, checked⟩ := Option.ite_none_right_eq_some.mp h
  all_goals first
    | contradiction
    | (obtain ⟨_, executed⟩ := Option.ite_none_right_eq_some.mp checked
       obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp executed
       cases result
       exact actualCharge_of_charge debit rfl)

/-- Effect completion does not debit again: successful prepare already paid for
this node before the external operation could be dispatched. -/
theorem advanceNode_effect_budgets {contracts : Contracts} {s t : State}
    {activation : ActivationId} {node : NodeId} {value : TypedValue}
    {r : Run} {n : Node} {cap : Capability}
    (found : findRun s activation = some r) (nodeFound : findNode r.procedure node = some n)
    (effect : n.form = .effect cap)
    (h : advanceNode contracts s activation node value = some t) :
    t.budgets = s.budgets := by
  simp only [advanceNode, found, nodeFound, effect, bind, Option.bind] at h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨_, h⟩ := Option.ite_none_right_eq_some.mp h
  cases h
  rfl

/-- Successful append supplies an actual domain execution with precisely the
source operational state, even though replay reconstructs its ledger prefix. -/
theorem append_actual_domain {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected record action) = some t) :
    ∃ before after, before.toOperationalState = s.toOperationalState ∧
      domain contracts before action = some after ∧
      t.toOperationalState = after.toOperationalState := by
  obtain ⟨after, executed, same⟩ := append_domain_witness h
  exact ⟨historicalState (Ledger.currentPrefix s.ledger stream)
    (recordFor contracts s stream record action), after, rfl, executed, same⟩

/-- The frame writer preserves budgets, so append also exposes the underlying
action reducer and its exact budget result. -/
theorem append_actual_domainBody {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected record action) = some t) :
    ∃ before after, before.toOperationalState = s.toOperationalState ∧
      domainBody contracts before action = some after ∧ t.budgets = after.budgets := by
  obtain ⟨before, framed, source, executed, final⟩ := append_actual_domain h
  cases body : domainBody contracts before action with
  | none => simp [domain, body] at executed
  | some after =>
    simp only [domain, body, Option.map_some, Option.some.injEq] at executed
    subst framed
    have budgets := congrArg OperationalState.budgets final
    exact ⟨before, after, source, body, budgets⟩

/-- A real append activation initializes the account consumed by all later
nested work; no caller-provided account classification is required. -/
theorem append_activation_account {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {run : Run} {budget : Nat}
    (h : step contracts s (.append stream expected record (.activate run budget)) = some t) :
    t.budgets.find? (·.root == run.id) = some ⟨run.id, budget, 0⟩ := by
  obtain ⟨before, after, _, executed, finalBudgets⟩ := append_actual_domainBody h
  have activated : activate contracts before run budget = some after := executed
  have budgets := (activation_initializes_one_account activated).1
  rw [finalBudgets, budgets]
  simp

end Kctx.Engineering
