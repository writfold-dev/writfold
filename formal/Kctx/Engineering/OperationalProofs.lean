import Kctx.Engineering.OperationalInvariant
import Kctx.Engineering.Actions
import Kctx.Engineering.ControlProofs

set_option autoImplicit false
namespace Kctx.Engineering

/-- The trace induction principle does not conflate refused requests with a
successful reducer call. It covers operational and explicit environment steps. -/
theorem reachable_preserves_property {contracts : Contracts} {initial s : State}
    {P : State → Prop} (base : P initial)
    (operational : ∀ {s t : State} {request : Request},
      P s → step contracts s request = some t → P t)
    (external : ∀ {s t : State} {action : Environment},
      P s → environment contracts s action = some t → P t)
    (reach : Reachable contracts initial s) : P s := by
  induction reach with
  | initial => exact base
  | next _ transition ih =>
      cases transition with
      | operational h => exact operational ih h
      | external h => exact external ih h
      | refused => exact ih

theorem member_eq_of_unique_key {α β : Type} {key : α → β} {xs : List α}
    (unique : (xs.map key).Nodup) {x y : α}
    (hx : x ∈ xs) (hy : y ∈ xs) (same : key x = key y) : x = y := by
  revert unique hx hy
  induction xs with
  | nil => intro _ hx; simp at hx
  | cons a rest ih =>
      intro unique hx hy
      have parts := List.nodup_cons.mp unique
      rcases List.mem_cons.mp hx with rfl | hxRest
      · rcases List.mem_cons.mp hy with rfl | hyRest
        · rfl
        · exact False.elim (parts.1 (List.mem_map.mpr ⟨y, hyRest, same.symm⟩))
      · rcases List.mem_cons.mp hy with rfl | hyRest
        · exact False.elim (parts.1 (List.mem_map.mpr ⟨x, hxRest, same⟩))
        · exact ih parts.2 hxRest hyRest

/-- Only actual parent links and list membership form a path; capability,
rank and gate inequalities are conclusions, not constructors. -/
inductive InvocationPath (s : OperationalState) : Run → Run → Nat → Prop where
  | root {run : Run} : run ∈ s.runs → InvocationPath s run run 0
  | child {root parent child : Run} {depth : Nat} :
      InvocationPath s root parent depth → child ∈ s.runs →
      child.parent = some parent.invocation → InvocationPath s root child (depth + 1)

theorem invocation_path_members {s : OperationalState} {root leaf : Run} {depth : Nat}
    (path : InvocationPath s root leaf depth) : root ∈ s.runs ∧ leaf ∈ s.runs := by
  induction path with
  | root member => exact ⟨member, member⟩
  | child _ member _ ih => exact ⟨ih.1, member⟩

theorem parent_representation {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {parent child : Run}
    (parentMember : parent ∈ s.runs) (childMember : child ∈ s.runs)
    (edge : child.parent = some parent.invocation) :
    parent.id = child.id ∧ parent.stream = child.stream ∧ parent.binding = child.binding ∧
    child.procedure.rank < parent.procedure.rank ∧
    ∃ delegation, child.delegation = some delegation ∧ delegation.parent = parent.invocation ∧
      (∀ cap ∈ child.capabilities, cap ∈ delegation.capabilities) ∧
      (∀ gate ∈ delegation.floor, gate ∈ child.floor) := by
  obtain ⟨p, member, identity, rest⟩ := (wf.runs child childMember).parent _ edge
  have same := member_eq_of_unique_key wf.activations_unique member parentMember identity
  subst p
  exact rest

/-- Historical composition remains acyclic after an in-place resume changes
current authority. This statement needs no current-liveness premise. -/
theorem invocation_path_rank_bound {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s root leaf depth) :
    leaf.procedure.rank + depth ≤ root.procedure.rank := by
  induction path with
  | root _ => omega
  | child path member edge ih =>
      have rank := (parent_representation wf (invocation_path_members path).2 member edge).2.2.2.1
      omega

theorem ancestryLive_monotone {s : State} {run : Run} {small large : Nat}
    (fuel : small ≤ large) (checked : ancestryLive s small run = true) :
    ancestryLive s large run = true := by
  induction small generalizing large run with
  | zero => simp [ancestryLive] at checked
  | succ small ih =>
      cases large with
      | zero => omega
      | succ large =>
          simp only [ancestryLive, Bool.and_eq_true] at checked ⊢
          refine ⟨checked.1, ?_⟩
          cases parent : run.parent with
          | none => simp [parent]
          | some id =>
              have ancestors := checked.2
              simp only [parent, Option.any_eq_true] at ancestors ⊢
              obtain ⟨p, found, guards⟩ := ancestors
              refine ⟨p, found, ?_⟩
              simp only [Bool.and_eq_true] at guards ⊢
              exact ⟨guards.1, ih (by omega) guards.2⟩

/-- Current applicability is checked on every ancestry edge. Immutable saved
delegation validates history; it does not authorize a child after its parent
has resumed under narrower capabilities or a stronger gate floor. -/
theorem live_parent_bounds {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) {parent child : Run}
    (member : parent ∈ s.runs) (edge : child.parent = some parent.invocation)
    (checked : live s child = true) :
    live s parent = true ∧
      (∀ cap ∈ child.capabilities, cap ∈ parent.capabilities) ∧
      (∀ gate ∈ parent.floor, gate ∈ child.floor) := by
  simp only [live, ancestryLive, edge, Bool.and_eq_true, Option.any_eq_true] at checked
  obtain ⟨_, p, found, guards⟩ := checked
  have identity : p.invocation = parent.invocation := by simpa using List.find?_some found
  have same := member_eq_of_unique_key wf.activations_unique
    (List.mem_of_find?_eq_some found) member identity
  subst p
  refine ⟨ancestryLive_monotone (Nat.le_succ _) guards.2, ?_, ?_⟩
  · simpa only [subset, List.all_eq_true, List.contains_iff_mem] using guards.1.1.2
  · simpa only [subset, List.all_eq_true, List.contains_iff_mem] using guards.1.2

/-- E04/E10: arbitrary currently applicable nesting cannot expand current
ancestor capabilities or drop ancestor gates. Rank bounds remain historical. -/
theorem invocation_path_bounded {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s.toOperationalState root leaf depth) (leafLive : live s leaf = true) :
    (∀ cap ∈ leaf.capabilities, cap ∈ root.capabilities) ∧
    (∀ gate ∈ root.floor, gate ∈ leaf.floor) ∧
    leaf.procedure.rank + depth ≤ root.procedure.rank := by
  have rank := invocation_path_rank_bound wf path
  suffices bounds : (∀ cap ∈ leaf.capabilities, cap ∈ root.capabilities) ∧
      (∀ gate ∈ root.floor, gate ∈ leaf.floor) from ⟨bounds.1, bounds.2, rank⟩
  clear rank
  revert leafLive
  induction path with
  | root _ => intro _; exact ⟨fun _ h => h, fun _ h => h⟩
  | child path _ edge ih =>
      intro leafLive
      obtain ⟨parentLive, caps, floor⟩ :=
        live_parent_bounds wf (invocation_path_members path).2 edge leafLive
      have previous := ih parentLive
      exact ⟨fun cap h => previous.1 cap (caps cap h),
        fun gate h => floor gate (previous.2 gate h)⟩

theorem invocation_path_shared_root {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s root leaf depth) : root.id = leaf.id := by
  induction path with
  | root _ => rfl
  | child path member edge ih =>
      exact ih.trans (parent_representation wf (invocation_path_members path).2 member edge).1

theorem invocation_path_unique_account {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s root leaf depth) :
    ∃ account ∈ s.budgets, account.root = root.id ∧ account.root = leaf.id ∧
      account.spent ≤ account.limit ∧
      ∀ other ∈ s.budgets, other.root = leaf.id → other = account := by
  obtain ⟨account, member, rootEq⟩ := (wf.runs root (invocation_path_members path).1).account
  have leafEq := rootEq.trans (invocation_path_shared_root wf path)
  refine ⟨account, member, rootEq, leafEq, wf.accounts_bounded account member, ?_⟩
  intro other otherMember otherRoot
  exact member_eq_of_unique_key wf.accounts_unique otherMember member (otherRoot.trans leafEq.symm)

theorem invocation_path_kernel_floor {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {root leaf : Run} {depth : Nat}
    (path : InvocationPath s root leaf depth) {gate : Gate} (member : gate ∈ contracts.kernelFloor) :
    gate ∈ leaf.floor :=
  (wf.runs leaf (invocation_path_members path).2).kernel_floor gate member

theorem parent_call_version_pinned {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {parent child : Run}
    (parentMember : parent ∈ s.runs) (childMember : child ∈ s.runs)
    (edge : child.parent = some parent.invocation) :
    ∃ node ∈ parent.procedure.nodes, ∃ target,
      node.form = .call target ∧ target.procedure = child.procedure.id ∧
      target.revision = child.procedure.revision ∧ target.input = child.procedure.inputType ∧
      target.output = child.procedure.outputType := by
  obtain ⟨p, member, identity, origin⟩ := (wf.runs child childMember).call_origin _ edge
  have same := member_eq_of_unique_key wf.activations_unique member parentMember identity
  subst p
  obtain ⟨node, nodeMember, target, form, procedure, revision, input, output, _⟩ := origin
  exact ⟨node, nodeMember, target, form, procedure, revision, input, output⟩

/-- Unlike current parent capability sets, declared callsite attenuation is
immutable and remains a valid historical bound across resumption. -/
theorem parent_call_attenuation {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {parent child : Run}
    (parentMember : parent ∈ s.runs) (childMember : child ∈ s.runs)
    (edge : child.parent = some parent.invocation) :
    ∃ node ∈ parent.procedure.nodes, ∃ target, node.form = .call target ∧
      (∀ cap ∈ child.capabilities, cap ∈ target.delegation ∧ cap ∈ node.ceiling) := by
  obtain ⟨p, member, identity, origin⟩ := (wf.runs child childMember).call_origin _ edge
  have same := member_eq_of_unique_key wf.activations_unique member parentMember identity
  subst p
  obtain ⟨node, nodeMember, target, form, _, _, _, _, bound⟩ := origin
  exact ⟨node, nodeMember, target, form, bound⟩

theorem invocation_cycles_impossible {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {run : Run} {depth : Nat}
    (path : InvocationPath s run run depth) : depth = 0 := by
  have bound := invocation_path_rank_bound wf path
  omega

theorem operational_effective_floor {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) (run : Run) (node : Node)
    {gate : Gate} (member : gate ∈ contracts.kernelFloor) :
    gate ∈ effectiveNodeFloor s run node :=
  effective_floor_preserves_policy s run node (wf.governing_floor gate member)

/-- E03: a retained pending call identifies one invocation, not merely a
procedure name or a structurally similar binding. -/
theorem pending_call_exact_child {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {parent : Run} (member : parent ∈ s.runs)
    {pending : PendingCall} (waiting : pending ∈ parent.control.pending) :
    ∃ child ∈ s.runs, child.invocation = pending.child ∧
      child.parent = some parent.invocation ∧ child.id = parent.id ∧
      child.procedure.id = pending.target.procedure ∧
      child.procedure.revision = pending.target.revision ∧ child.input = pending.input ∧
      ∀ other ∈ s.runs, other.invocation = pending.child → other = child := by
  obtain ⟨_, _, _, _, _, child, childMember, identity, rest⟩ :=
    (wf.runs parent member).pending pending waiting
  refine ⟨child, childMember, identity, rest.1, rest.2.1, rest.2.2.1,
    rest.2.2.2.1, rest.2.2.2.2, ?_⟩
  intro other otherMember otherId
  exact member_eq_of_unique_key wf.activations_unique otherMember childMember
    (otherId.trans identity.symm)

end Kctx.Engineering
