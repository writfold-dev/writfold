import Kctx.Engineering.HistorySafetyFrames
import Kctx.Engineering.OperationalProofsRunsFrames
import Kctx.Engineering.OperationalCongruence

set_option autoImplicit false
set_option maxHeartbeats 2400000
namespace Kctx.Engineering

/-- Reuse is a checked reference to an existing observation, not an execution.
The complete original receipt, including failure/success and its original time,
is preserved; applicable current guards are separate fields of this witness. -/
structure ReuseChecked (s t : State) (activation : ActivationId) (receipt : Receipt) where
  run : Run
  node : Node
  found : findRun s activation = some run
  stored : receipt ∈ s.receipts
  node_found : findNode run.procedure receipt.node = some node
  live : live s run = true
  binding : receipt.binding = run.binding
  procedure : receipt.procedure = run.procedure.revision
  root_input : receipt.rootInput = run.input
  node_input : receipt.input = nodeInput run node
  floor : ∀ gate ∈ effectiveNodeFloor s run node, gate ∈ receipt.gates
  role : receiptRoleValid s run node receipt = true
  complete : receipt.complete = true
  retained : ∀ object ∈ receipt.objects, object ∈ s.retained
  observed : receipt.observedAt ≤ s.now
  fresh : s.now < receipt.expires
  policy : receipt.policy = s.authority.policy
  correction : receipt.correctionGeneration = s.authority.correctionGeneration
  original : t.reused = ⟨receipt, run.invocation⟩ :: s.reused

theorem reuse_checked {s t : State} {activation : ActivationId} {receipt : Receipt}
    (h : reuse s activation receipt = some t) : Nonempty (ReuseChecked s t activation receipt) := by
  cases found : findRun s activation with
  | none => simp [reuse, found] at h
  | some run =>
      simp only [reuse, found, bind, Option.bind] at h
      split at h
      next guard =>
        simp only [Bool.and_eq_true] at guard
        have stored : receipt ∈ s.receipts := by simpa using guard.1
        have valid := guard.2
        cases nodeFound : findNode run.procedure receipt.node with
        | none => simp [receiptValid, nodeFound] at valid
        | some node =>
            simp only [receiptValid, nodeFound, Option.any_some, Bool.and_eq_true,
              beq_iff_eq, decide_eq_true_eq, and_assoc] at valid
            obtain ⟨live, binding, procedure, complete, retained, input, floor, role,
              observed, fresh, rootInput, policy, _, correction⟩ := valid
            simp only [Option.some.injEq] at h
            subst t
            exact ⟨{
              run := run
              node := node
              found := found
              stored := stored
              node_found := nodeFound
              live := live
              binding := binding
              procedure := procedure
              root_input := rootInput
              node_input := input
              floor := runs_subset floor
              role := role
              complete := complete
              retained := runs_retained retained
              observed := observed
              fresh := fresh
              policy := policy
              correction := correction
              original := rfl }⟩
      next refused => contradiction

theorem domain_reuse_checked {contracts : Contracts} {s t : State}
    {activation : ActivationId} {receipt : Receipt}
    (h : domain contracts s (.reuse activation receipt) = some t) :
    Nonempty (ReuseChecked s t activation receipt) := by
  cases hr : reuse s activation receipt with
  | none => simp [domain, domainBody, hr] at h
  | some next =>
      obtain ⟨checked⟩ := reuse_checked hr
      simp only [domain, domainBody, hr, Option.map_some, Option.some.injEq] at h
      subst t
      exact ⟨{ checked with original := checked.original }⟩

/-- E04 for the admitted Request path, not merely its leaf checker. -/
theorem E04_request_reuse_integrity {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId}
    {activation : ActivationId} {receipt : Receipt}
    (h : step contracts s (.append stream expected id (.reuse activation receipt)) = some t) :
    Nonempty (ReuseChecked s t activation receipt) := by
  have replayed := (step_append_exact h).1
  let record := recordFor contracts s stream id (.reuse activation receipt)
  let before := historicalState (Ledger.currentPrefix s.ledger stream) record
  change (domain contracts before (.reuse activation receipt)).map State.toOperationalState =
    some t.toOperationalState at replayed
  cases hd : domain contracts before (.reuse activation receipt) with
  | none => simp [hd] at replayed
  | some next =>
      obtain ⟨checked⟩ := domain_reuse_checked hd
      simp only [hd, Option.map_some, Option.some.injEq] at replayed
      have original : t.reused = ⟨receipt, checked.run.invocation⟩ :: s.reused := by
        change t.toOperationalState.reused = _
        rw [← replayed]
        exact checked.original
      exact ⟨{
        run := checked.run
        node := checked.node
        found := checked.found
        stored := checked.stored
        node_found := checked.node_found
        live := by
          rw [← live_operational before s rfl checked.run]
          exact checked.live
        binding := checked.binding
        procedure := checked.procedure
        root_input := checked.root_input
        node_input := checked.node_input
        floor := checked.floor
        role := checked.role
        complete := checked.complete
        retained := checked.retained
        observed := checked.observed
        fresh := checked.fresh
        policy := checked.policy
        correction := checked.correction
        original := original }⟩

/-- No fabricated fresh timestamp or success is introduced by reuse. -/
theorem E04_reuse_keeps_observation {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId}
    {activation : ActivationId} {receipt : Receipt}
    (h : step contracts s (.append stream expected id (.reuse activation receipt)) = some t) :
    ∃ reused ∈ t.reused, reused.original = receipt ∧
      reused.original.observedAt = receipt.observedAt ∧ reused.original.outcome = receipt.outcome := by
  obtain ⟨checked⟩ := E04_request_reuse_integrity h
  refine ⟨⟨receipt, checked.run.invocation⟩, ?_, rfl, rfl, rfl⟩
  simp [checked.original]

end Kctx.Engineering
