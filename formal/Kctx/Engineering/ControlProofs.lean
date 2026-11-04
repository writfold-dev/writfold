import Kctx.Engineering.Learning

set_option autoImplicit false
namespace Kctx.Engineering

/-- Structural validator consequences are about the admitted graph, not an
assumed post-state invariant in the interpreter. -/
theorem validated_node_typed {catalog : List Procedure} {registry : List PredicateSignature}
    {p : Procedure} {n : Node} (h : validateProcedure catalog registry p = true)
    (member : n ∈ p.nodes) : NodeTyped catalog registry p n :=
  (validateProcedure_sound h).2.2.2.2.2 n member

theorem validated_nodes_unique {catalog : List Procedure} {registry : List PredicateSignature}
    {p : Procedure} (h : validateProcedure catalog registry p = true) :
    (p.nodes.map Node.id).Nodup := (validateProcedure_sound h).1

theorem typed_call_rank {catalog : List Procedure} {registry : List PredicateSignature}
    {p : Procedure} {n : Node} {target : CallTarget}
    (typed : FormTyped catalog registry p n) (form : n.form = .call target) :
    ∃ callee ∈ catalog, callee.id = target.procedure ∧ callee.revision = target.revision ∧
      callee.rank < p.rank := by
  simp only [FormTyped, form] at typed
  rcases typed with ⟨callee, member, id, revision, _, _, _, _, rank, _⟩
  exact ⟨callee, member, id, revision, rank⟩

/-- A path of actual strict call ranks has length bounded by its first rank. -/
inductive RankedCallPath : Nat → Nat → Prop where
  | nil (rank : Nat) : RankedCallPath rank 0
  | cons {parent child length : Nat} : child < parent → RankedCallPath child length →
      RankedCallPath parent (length + 1)

theorem ranked_call_path_bounded {rank length : Nat} (path : RankedCallPath rank length) :
    length ≤ rank := by
  induction path with
  | nil => exact Nat.zero_le _
  | cons smaller _ ih => omega

theorem call_capabilities_bounded {grant parent delegation nodeCeiling calleeCeiling : List Capability}
    {cap : Capability}
    (h : cap ∈ childCapabilities grant
      (childCapabilities parent delegation nodeCeiling) calleeCeiling) :
    cap ∈ grant ∧ cap ∈ parent ∧ cap ∈ delegation ∧ cap ∈ nodeCeiling ∧ cap ∈ calleeCeiling := by
  have memberships : cap ∈ delegation ∧ (cap ∈ grant ∧ cap ∈ calleeCeiling) ∧
      cap ∈ parent ∧ cap ∈ nodeCeiling := by
    simpa [childCapabilities, List.mem_filter, Bool.and_eq_true] using h
  rcases memberships with ⟨delegated, ⟨granted, callee⟩, parental, node⟩
  exact ⟨granted, parental, delegated, node, callee⟩

theorem inherited_policy_floor {parent callee policy : List Gate} {gate : Gate}
    (h : gate ∈ policy) : gate ∈ inheritedFloor parent callee policy := by
  simp [inheritedFloor, h]

theorem charge_has_bounded_positive_cost {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) :
    ∃ account, s.budgets.find? (·.root == root) = some account ∧
      0 < cost ∧ account.spent + cost ≤ account.limit ∧
      t.budgets = s.budgets.map (fun old =>
        if old.root == root then { old with spent := old.spent + cost } else old) := by
  cases ha : s.budgets.find? (·.root == root) with
  | none => simp [charge, ha] at h
  | some account =>
      simp only [charge, ha, bind, Option.bind] at h
      split at h
      next guard =>
        simp only [Option.some.injEq] at h
        subst t
        have bounds : 0 < cost ∧ account.spent + cost ≤ account.limit := by simpa using guard
        exact ⟨account, rfl, bounds.1, bounds.2, rfl⟩
      next => contradiction

theorem charge_does_not_duplicate_accounts {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) : t.budgets.length = s.budgets.length := by
  obtain ⟨_, _, _, _, accounts⟩ := charge_has_bounded_positive_cost h
  simp [accounts]

theorem charge_preserves_history {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) : t.ledger = s.ledger ∧ t.receipts = s.receipts := by
  cases ha : s.budgets.find? (·.root == root) with
  | none => simp [charge, ha] at h
  | some account =>
      simp only [charge, ha, bind, Option.bind] at h
      split at h
      · cases h
        exact ⟨rfl, rfl⟩
      · contradiction

theorem recorded_progress (r : Run) (n : Node) (value : TypedValue) (outcome : Completion) :
    (recordResult r n value outcome).control.results.length = r.control.results.length + 1 := rfl

theorem completed_node_disabled (s : State) (r : Run) (n : Node)
    (h : r.control.results.any (·.node == n.id) = true) : enabled s r n = false := by
  simp [enabled, h]

theorem progress_preserves_invocation (r : Run) (n : Node) (value : TypedValue) (outcome : Completion) :
    (recordResult r n value outcome).id = r.id ∧
    (recordResult r n value outcome).invocation = r.invocation ∧
    (recordResult r n value outcome).capabilities = r.capabilities ∧
    (recordResult r n value outcome).floor = r.floor := ⟨rfl, rfl, rfl, rfl⟩

theorem cancelled_locally_inactive (s : State) (r : Run) (h : r.status = .cancelled) :
    locallyLive s r = false := by simp [locallyLive, h]

theorem cancelled_cannot_continue (s : State) (r : Run) (h : r.status = .cancelled) :
    live s r = false := by simp [live, ancestryLive, locallyLive, h]

theorem stale_binding_evidence_refused (s : State) (r : Run) (e : TypedEvidence)
    (h : e.binding ≠ r.binding) : evidenceApplicable s r e = false := by
  simp [evidenceApplicable, h]

theorem activation_initializes_one_account {contracts : Contracts} {s t : State}
    {r : Run} {budget : Nat} (h : activate contracts s r budget = some t) :
    t.budgets = ⟨r.id, budget, 0⟩ :: s.budgets ∧ t.runs = r :: s.runs := by
  cases granted : currentGrant s r with
  | none => simp [activate, granted] at h
  | some grant =>
      simp only [activate, granted] at h
      split at h
      · cases h
        exact ⟨rfl, rfl⟩
      · contradiction

/-- Every charged operation is positive. Consequently an arbitrarily nested
sequence of operations consuming one root account has a finite length bound. -/
inductive BudgetConsumption (limit : Nat) : Nat → Nat → Nat → Prop where
  | nil (spent : Nat) : spent ≤ limit → BudgetConsumption limit spent 0 spent
  | step {spent cost count final : Nat} : 0 < cost → spent + cost ≤ limit →
      BudgetConsumption limit (spent + cost) count final →
      BudgetConsumption limit spent (count + 1) final

theorem budget_consumption_bounded {limit spent count final : Nat}
    (h : BudgetConsumption limit spent count final) :
    spent + count ≤ final ∧ final ≤ limit := by
  induction h with
  | nil spent bound => exact ⟨by omega, bound⟩
  | step positive _ _ ih => constructor <;> omega

theorem budget_traversal_bounded {limit count final : Nat}
    (h : BudgetConsumption limit 0 count final) : count ≤ limit := by
  have bounds := budget_consumption_bounded h
  omega

theorem stale_incarnation_cannot_continue (s : State) (r : Run)
    (h : r.incarnation ≠ s.incarnation) : live s r = false := by
  simp [live, ancestryLive, locallyLive, h]

theorem recovery_does_not_refresh_activations {s t : State} (h : recover s = some t) :
    t.runs = s.runs ∧ t.incarnation = s.incarnation + 1 := by
  unfold recover at h
  split at h
  · cases h
    exact ⟨rfl, rfl⟩
  · contradiction

theorem effective_floor_preserves_invocation (s : State) (r : Run) (n : Node)
    {gate : Gate} (h : gate ∈ r.floor) : gate ∈ effectiveNodeFloor s r n := by
  simp [effectiveNodeFloor, inheritedFloor, h]

theorem effective_floor_preserves_node (s : State) (r : Run) (n : Node)
    {gate : Gate} (h : gate ∈ n.floor) : gate ∈ effectiveNodeFloor s r n := by
  simp [effectiveNodeFloor, inheritedFloor, h]

theorem effective_floor_preserves_policy (s : State) (r : Run) (n : Node)
    {gate : Gate} (h : gate ∈ s.authority.floor) : gate ∈ effectiveNodeFloor s r n := by
  simp [effectiveNodeFloor, inheritedFloor, h]

theorem changed_invocation_input_refuses_receipt (s : State) (r : Run) (receipt : Receipt)
    (h : receipt.rootInput ≠ r.input) : receiptValid s r receipt = false := by
  simp [receiptValid, h]

theorem changed_node_input_refuses_receipt (s : State) (r : Run) (receipt : Receipt) (n : Node)
    (found : findNode r.procedure receipt.node = some n)
    (changed : receipt.input ≠ nodeInput r n) : receiptValid s r receipt = false := by
  simp [receiptValid, found, changed]

theorem gate_receipt_has_evidence_type (s : State) (r : Run) (n : Node) (receipt : Receipt)
    (role : receipt.role = .gate) (h : receiptRoleValid s r n receipt = true) :
    receipt.output.type = .evidence := by
  simpa [receiptRoleValid, role] using h

theorem procedure_receipt_has_terminal_subject (s : State) (r : Run) (n : Node)
    (receipt : Receipt) (role : receipt.role = .procedureResult)
    (h : receiptRoleValid s r n receipt = true) :
    n.form = .«end» ∧ receipt.output.type = r.procedure.outputType := by
  simpa [receiptRoleValid, role, Bool.and_eq_true] using h

theorem learning_flags_preserve_material (c : Candidate) (evaluated reviewed : Bool) :
    ({ c with evaluated := evaluated, reviewed := reviewed } : Candidate).toCandidateMaterial =
      c.toCandidateMaterial := rfl

theorem changed_delta_refuses_learning_approval (s : State) (c : Candidate)
    (approval : LearningApproval) (h : approval.material ≠ c.toCandidateMaterial) :
    learningApprovalValid s c approval = false := by
  simp [learningApprovalValid, h]

theorem changed_policy_refuses_learning_approval (s : State) (c : Candidate)
    (approval : LearningApproval) (h : approval.policy ≠ s.authority.policy) :
    learningApprovalValid s c approval = false := by
  simp [learningApprovalValid, h]

theorem issuing_invalidates_old_process_certificate (o : Reservation) :
    (o.resource, o.generation, o.processGeneration + 1) ≠
      (o.resource, o.generation, o.processGeneration) := by
  intro same
  have generation := congrArg (fun certificate : Id .resource × Nat × Nat => certificate.2.2) same
  simp only at generation
  omega

theorem typed_effect_has_nonempty_footprint {catalog : List Procedure}
    {registry : List PredicateSignature} {p : Procedure} {n : Node} {cap : Capability}
    (typed : FormTyped catalog registry p n) (form : n.form = .effect cap) :
    n.resources ≠ [] ∧ n.resources.Nodup := by
  simp only [FormTyped, form] at typed
  exact typed.2

theorem empty_footprint_refuses_intent (s : State) (r : Run) (i : Intent) (generation : Nat)
    (empty : i.resources = []) : intentReady s r i generation = false := by
  simp [intentReady, empty]

theorem unloaded_context_blocks_node (contracts : Contracts) (s : State) (r : Run) (n : Node)
    (missing : subset n.requiredContext r.loaded = false) :
    nodeReady contracts s r n = false := by
  simp [nodeReady, missing]

theorem changed_intent_refuses_retry_authorization (s : State) (intent : Intent)
    (authorization : RetryAuthorization) (changed : authorization.intent ≠ intent) :
    retryAuthorizationValid s intent authorization = false := by
  simp [retryAuthorizationValid, changed]

theorem stale_retry_authorization_refused (s : State) (intent : Intent)
    (authorization : RetryAuthorization) (stale : authorization.incarnation ≠ s.incarnation) :
    retryAuthorizationValid s intent authorization = false := by
  simp [retryAuthorizationValid, stale]

theorem changed_input_refuses_resumption (s : State) (r : Run) (a : ResumeAuthorization)
    (changed : a.input ≠ r.input) : resumeAuthorizationValid s r a = false := by
  simp [resumeAuthorizationValid, changed]

theorem changed_control_refuses_resumption (s : State) (r : Run) (a : ResumeAuthorization)
    (changed : a.control ≠ r.control) : resumeAuthorizationValid s r a = false := by
  simp [resumeAuthorizationValid, changed]

theorem cancelled_activation_cannot_resume (contracts : Contracts) (s : State)
    (activation : ActivationId) (r : Run) (found : findRun s activation = some r)
    (cancelled : r.status = .cancelled) : resume contracts s activation = none := by
  cases grant : currentGrant s r <;> simp [resume, found, grant, cancelled]

theorem changed_native_precondition_refuses_intent (s : State) (r : Run)
    (intent : Intent) (generation : Nat)
    (changed : intent.expectedProvider ≠ r.binding.expectedProvider) :
    intentReady s r intent generation = false := by
  simp [intentReady, changed]

end Kctx.Engineering
