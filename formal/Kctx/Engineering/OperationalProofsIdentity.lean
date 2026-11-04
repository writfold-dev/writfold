import Kctx.Engineering.OperationalProofsLists
import Kctx.Engineering.OperationalProofsReducer
import Lean.Elab.Tactic

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- The identity and root-account component of operational representation.
Domain actions may extend these histories, but never alias their keys. -/
structure OperationalIdentities (s : OperationalState) : Prop where
  grants_unique : unique (s.authority.grants.map Grant.id) = true
  catalog_unique : (s.catalog.map Procedure.revision).Nodup
  activations_unique : (s.runs.map Run.invocation).Nodup
  accounts_unique : (s.budgets.map RootBudget.root).Nodup
  accounts_bounded : ∀ budget ∈ s.budgets, budget.spent ≤ budget.limit
  effects_unique : (s.effects.map (fun effect => effect.intent.effect)).Nodup
  ownership_unique : (s.ownership.map Reservation.resource).Nodup
  candidates_unique : (s.candidates.map (fun candidate : Candidate => candidate.id)).Nodup
  captures_unique : (s.captures.map RetainedObservation.id).Nodup

theorem OperationalWellFormed.identities {contracts : Contracts} {s : OperationalState}
    (h : OperationalWellFormed contracts s) : OperationalIdentities s :=
  ⟨h.grants_unique, h.catalog_unique, h.activations_unique, h.accounts_unique,
    h.accounts_bounded, h.effects_unique, h.ownership_unique, h.candidates_unique,
    h.captures_unique⟩

/-- Key-preserving updates need not preserve the rest of a historical record. -/
theorem identities_map_keys {α β : Type} (key : α → β) (update : α → α)
    (xs : List α) (same : ∀ x, key (update x) = key x) :
    (xs.map update).map key = xs.map key := by
  simp only [List.map_map]
  congr 1
  funext x
  exact same x

/-- These two transports inspect only keys, not the updated records or guards. -/
theorem identities_map_unique {α β : Type} {key : α → β} {update : α → α}
    {xs : List α} (same : ∀ x, key (update x) = key x)
    (old : (xs.map key).Nodup) : ((xs.map update).map key).Nodup := by
  rw [identities_map_keys key update xs same]
  exact old

theorem identities_map_fresh {α β : Type} {key : α → β} {update : α → α}
    {xs : List α} {id : β} (same : ∀ x, key (update x) = key x)
    (fresh : id ∉ xs.map key) : id ∉ (xs.map update).map key := by
  rw [identities_map_keys key update xs same]
  exact fresh

@[simp] theorem identities_replace_key {α : Type} [BEq α] [LawfulBEq α]
    (a b : α) : (if a == b then b else a) = a := by
  split <;> simp_all

@[simp] theorem identities_replace_key_eq {α : Type} (a b : α) [Decidable (a = b)] :
    (if a = b then b else a) = a := by
  split <;> simp_all

theorem identities_key_injective {α β : Type} {key : α → β} {xs : List α}
    (unique : (xs.map key).Nodup) {a b : α}
    (ha : a ∈ xs) (hb : b ∈ xs) (same : key a = key b) : a = b := by
  revert unique ha hb
  induction xs with
  | nil => intro _ ha; simp at ha
  | cons x xs ih =>
    intro unique ha hb
    simp only [List.map_cons, List.nodup_cons] at unique
    simp only [List.mem_cons] at ha hb
    rcases ha with rfl | ha
    · rcases hb with rfl | hb
      · rfl
      · exact False.elim (unique.1 (List.mem_map.mpr ⟨b, hb, same.symm⟩))
    · rcases hb with rfl | hb
      · exact False.elim (unique.1 (List.mem_map.mpr ⟨a, ha, same⟩))
      · exact ih unique.2 ha hb


theorem identities_fresh_key {α β : Type} [BEq β] [LawfulBEq β]
    {xs : List α} {key : α → β} {id : β}
    (fresh : xs.any (fun x => key x == id) = false) : id ∉ xs.map key := by
  simp only [List.any_eq_false] at fresh
  intro member
  obtain ⟨x, hx, eq⟩ := List.mem_map.mp member
  have := fresh x hx
  simp [eq] at this

theorem identities_find_fresh {s : State} {id : Id .effect}
    (fresh : (findEffect s id).isSome = false) :
    id ∉ s.effects.map (fun effect => effect.intent.effect) := by
  have absent : findEffect s id = none := by
    cases found : findEffect s id with
    | none => rfl
    | some effect => simp only [found, Option.isSome, Bool.true_eq_false] at fresh
  intro member
  obtain ⟨effect, member, same⟩ := List.mem_map.mp member
  have rejected := List.find?_eq_none.mp absent effect member
  exact rejected (by simp only [same, beq_self_eq_true])

theorem identities_all_fresh {α β : Type} [BEq β]
    {xs : List α} {keys : List β} {key : α → β} {p : β → Bool}
    (checked : keys.all (fun id => !xs.any (fun x => key x == id) && p id) = true) :
    ∀ id ∈ keys, xs.any (fun x => key x == id) = false := by
  intro id member
  have guard := List.all_eq_true.mp checked id member
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at guard
  exact guard.1

theorem identities_cons_bounded {xs : List RootBudget} (root : RunId) (limit : Nat)
    (bounded : ∀ budget ∈ xs, budget.spent ≤ budget.limit) :
    ∀ budget ∈ (⟨root, limit, 0⟩ : RootBudget) :: xs, budget.spent ≤ budget.limit := by
  intro budget member
  rcases List.mem_cons.mp member with rfl | member
  · exact Nat.zero_le _
  · exact bounded budget member

theorem identities_filter_keys {α β : Type} {xs : List α} {key : α → β}
    (p : α → Bool) (h : (xs.map key).Nodup) : ((xs.filter p).map key).Nodup := by
  exact ((List.filter_sublist (p := p) (l := xs)).map key).nodup h

theorem identities_charge_bounded {s : State} {root : RunId} {cost : Nat}
    {account : RootBudget}
    (unique : (s.budgets.map RootBudget.root).Nodup)
    (bounded : ∀ budget ∈ s.budgets, budget.spent ≤ budget.limit)
    (found : s.budgets.find? (·.root == root) = some account)
    (bound : account.spent + cost ≤ account.limit) :
    ∀ budget ∈ s.budgets.map (fun old =>
      if old.root == root then { old with spent := old.spent + cost } else old),
      budget.spent ≤ budget.limit := by
  intro budget member
  obtain ⟨old, oldMember, rfl⟩ := List.mem_map.mp member
  split
  next same =>
    have selected : account.root = root := by simpa using List.find?_some found
    have eq : old = account := identities_key_injective unique oldMember
      (List.mem_of_find?_eq_some found) (by simpa using (show old.root = root from by simpa using same).trans selected.symm)
    simpa [eq] using bound
  next => exact bounded old oldMember

theorem operationalIdentities_charge {s t : State} {root : RunId} {cost : Nat}
    (before : OperationalIdentities s.toOperationalState) (h : charge s root cost = some t) :
    OperationalIdentities t.toOperationalState := by
  obtain ⟨account, found, _, bound, _⟩ := charge_has_bounded_positive_cost h
  rw [charge_exact_state h]
  refine ⟨before.grants_unique, before.catalog_unique, before.activations_unique,
    ?_, ?_, before.effects_unique, before.ownership_unique, before.candidates_unique,
    before.captures_unique⟩
  · simpa only [List.map_map, Function.comp_def, apply_ite, ite_self]
      using before.accounts_unique
  · exact identities_charge_bounded before.accounts_unique before.accounts_bounded found bound

/-- Append preserves distinct keys when the two histories are disjoint. -/
theorem identities_nodup_append {α : Type} {xs ys : List α}
    (left : xs.Nodup) (right : ys.Nodup)
    (disjoint : ∀ x ∈ xs, x ∉ ys) : (xs ++ ys).Nodup := by
  revert left disjoint
  induction xs with
  | nil => intro _ _; exact right
  | cons x xs ih =>
    intro left disjoint
    obtain ⟨fresh, unique⟩ := List.nodup_cons.mp left
    apply List.nodup_cons.mpr
    constructor
    · intro member
      rcases List.mem_append.mp member with member | member
      · exact fresh member
      · exact disjoint x (List.mem_cons_self) member
    · exact ih unique (fun y member => disjoint y (List.mem_cons_of_mem x member))

theorem identities_map_id {α : Type} (xs : List α) :
    xs.map (fun x => x) = xs := by
  induction xs with
  | nil => rfl
  | cons x xs ih => exact congrArg (List.cons x) ih

/-- Reservation extends an ownership history by fresh, distinct resource keys. -/
theorem identities_reservation_keys {xs : List Reservation} {keys : List (Id .resource)}
    (owner : RunId) (holder : ActivationId) (generation : Nat)
    (old : (xs.map Reservation.resource).Nodup) (distinct : unique keys = true)
    (fresh : ∀ key ∈ keys, xs.any (·.resource == key) = false) :
    ((keys.map (fun key => (⟨key, owner, holder, generation, 0, 0⟩ : Reservation)) ++ xs).map
      Reservation.resource).Nodup := by
  simpa only [List.map_append, List.map_map, Function.comp_def, identities_map_id] using
    identities_nodup_append (unique_sound distinct) old
      (fun key member => identities_fresh_key (fresh key member))

/-- Key proofs split the update, never simplify its (possibly state-sized)
condition. A replacement is keyed by exactly the equality tested by putRun or
putEffect; all other updates leave the key definitionally unchanged. -/
macro "identities_same_key" : tactic => `(tactic| (
  intro old
  first
  | rfl
  | (split <;> first
      | rfl
      | (rename_i same; exact (beq_iff_eq.mp same).symm))
))

/- Consume only outer Boolean conjunctions and Boolean negations, then
eliminate concrete conjunction hypotheses without searching record fields. -/
open Lean Meta Elab Tactic in
elab "identities_guards" : tactic => do
  evalTactic (← `(tactic| try simp only [Bool.and_eq_true, Bool.not_eq_true', Bool.not_eq_false'] at *))
  let mut more := true
  while more do
    more := false
    let goal ← getMainGoal
    let conjunction ← goal.withContext do
      for declaration in (← getLCtx) do
        if declaration.type.isAppOfArity ``And 2 then
          return some declaration.fvarId
      return none
    if let some hypothesis := conjunction then
      let branches ← goal.cases hypothesis
      replaceMainGoal (branches.toList.map (·.mvarId))
      more := true

/-- Recursion follows the list expression: map, cons, or filter. Only the
shared-account debit and reservation append require their own bounded lemmas. -/
syntax "identities_field" : tactic
macro_rules
  | `(tactic| identities_field) => `(tactic| (
      first
      | assumption
      | (apply identities_charge_bounded <;> assumption)
      | (apply identities_cons_bounded; assumption)
      | (apply identities_map_unique
         · identities_same_key
         · identities_field)
      | (apply identities_map_fresh
         · identities_same_key
         · identities_field)
      | (apply List.nodup_cons.mpr
         constructor <;> identities_field)
      | (apply identities_filter_keys; identities_field)
      | (apply identities_fresh_key; assumption)
      | (apply identities_find_fresh; assumption)
      | (apply identities_all_fresh; assumption)
      | (apply identities_reservation_keys <;> identities_field)
    ))

macro "identities_fields" : tactic => `(tactic| (
  identities_guards
  constructor <;> identities_field
))

/-- Every successful checked action preserves identity separation and account
bounds. Success extraction retains freshness guards and the exact shared-account
debit. Field proofs recurse only through key-preserving list operations; the
observation action splits its three explicit effect-state alternatives. -/
theorem operationalIdentities_domainBody {contracts : Contracts} {s t : State}
    {action : Action} (before : OperationalIdentities s.toOperationalState)
    (h : domainBody contracts s action = some t) : OperationalIdentities t.toOperationalState := by
  rcases before with ⟨grants, catalog, runs, accounts, bounded, effects, ownership, candidates, captures⟩
  cases action
  case observe event =>
    simp only [domainBody, observeEffect] at h
    operational_success h
    all_goals identities_fields
  all_goals
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect,
      observeMaterial, reconcile, reuse, returnChild, rebind, resume, reopen, cancel, quiesce,
      propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
      retire] at h
    operational_success h
    all_goals identities_fields

theorem operationalIdentities_domain {contracts : Contracts} {s t : State}
    {action : Action} (before : OperationalIdentities s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalIdentities t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    have after := operationalIdentities_domainBody before body
    exact ⟨before.grants_unique, after.catalog_unique, after.activations_unique,
      after.accounts_unique, after.accounts_bounded, after.effects_unique,
      after.ownership_unique, after.candidates_unique, after.captures_unique⟩

theorem operationalWellFormed_domain_identities {contracts : Contracts} {s t : State}
    {action : Action} (before : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalIdentities t.toOperationalState :=
  operationalIdentities_domain before.identities h

end Kctx.Engineering
