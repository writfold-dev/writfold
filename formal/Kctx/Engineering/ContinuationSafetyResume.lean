import Kctx.Engineering.ContinuationSafety

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Resumption is a qualified full-state migration, never a digest-only refresh
of an incarnation. The authorization includes the old control and exact input. -/
theorem E12_resume_has_exact_authorization {contracts : Contracts} {s t : State} {activation : ActivationId}
    (h : resume contracts s activation = some t) :
    ∃ old authorization, findRun s activation = some old ∧ authorization ∈ contracts.resumptions ∧
      authorization.activation = old.invocation ∧ authorization.root = old.id ∧
      authorization.procedure = old.procedure.revision ∧ authorization.binding = old.binding ∧
      authorization.input = old.input ∧ authorization.control = old.control ∧
      authorization.previousIncarnation = old.incarnation ∧ authorization.incarnation = s.incarnation ∧
      authorization.policy = s.authority.policy ∧
      authorization.correctionGeneration = s.authority.correctionGeneration ∧
      authorization.provider = s.world.provider ∧ authorization.validFrom ≤ s.now ∧
      s.now < authorization.expires := by
  have guarded : ContinuationChecked contracts s (.resume activation) :=
    domain_body_checks_continuation h
  obtain ⟨old, grant, found, _, _, _, _, qualified, _, _, _⟩ := guarded
  obtain ⟨authorization, member, valid⟩ := List.any_eq_true.mp qualified
  refine ⟨old, authorization, found, member, ?_⟩
  simpa [resumeAuthorizationValid, Bool.and_eq_true, and_assoc] using valid

theorem E12_append_resume_has_exact_authorization {contracts : Contracts} {s t : State}
    {activation : ActivationId} {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    (h : step contracts s (.append stream expected record (.resume activation)) = some t) :
    ∃ old authorization, findRun s activation = some old ∧ authorization ∈ contracts.resumptions ∧
      authorization.activation = old.invocation ∧ authorization.root = old.id ∧
      authorization.procedure = old.procedure.revision ∧ authorization.binding = old.binding ∧
      authorization.input = old.input ∧ authorization.control = old.control ∧
      authorization.previousIncarnation = old.incarnation ∧ authorization.incarnation = s.incarnation ∧
      authorization.policy = s.authority.policy ∧
      authorization.correctionGeneration = s.authority.correctionGeneration ∧
      authorization.provider = s.world.provider ∧ authorization.validFrom ≤ s.now ∧
      s.now < authorization.expires := by
  obtain ⟨next, executed, _⟩ := append_domain_witness h
  cases body : domainBody contracts
      (historicalState (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream record (.resume activation))) (.resume activation) with
  | none => simp [domain, body] at executed
  | some result =>
      have authorized := E12_resume_has_exact_authorization body
      exact authorized

/-- The actual refreshed run retains invocation, procedure, binding, input and
control. Its current authority is checked and context is deliberately unloaded. -/
theorem E12_resume_preserves_control_and_clears_context {contracts : Contracts} {s t : State}
    {activation : ActivationId} (h : resume contracts s activation = some t) :
    ∃ old next charged, findRun s activation = some old ∧ charge s old.id 1 = some charged ∧
      t = putRun charged next ∧ next.id = old.id ∧ next.invocation = old.invocation ∧
      next.parent = old.parent ∧ next.procedure = old.procedure ∧ next.binding = old.binding ∧
      next.input = old.input ∧ next.control = old.control ∧ next.loaded = [] ∧ live s next = true := by
  cases found : findRun s activation with
  | none => simp [resume, found] at h
  | some old =>
      cases grantFound : currentGrant s old with
      | none => simp [resume, found, grantFound] at h
      | some grant =>
          let parent := old.parent.bind (findRun s)
          let next : Run := { old with
            incarnation := s.incarnation
            capabilities := old.capabilities.filter (fun cap =>
              grant.capabilities.contains cap && old.procedure.ceiling.contains cap &&
                ((parent.map Run.capabilities).getD old.capabilities).contains cap)
            floor := inheritedFloor old.floor old.procedure.floor
              (s.authority.floor ++ (parent.map Run.floor).getD [])
            correctionGeneration := s.authority.correctionGeneration
            policy := s.authority.policy
            loaded := [] }
          simp only [resume, found, grantFound, bind, Option.bind_some] at h
          obtain ⟨guard, continuation⟩ := Option.ite_none_right_eq_some.mp h
          have liveNext : live s next = true := by
            simp only [Bool.and_eq_true] at guard
            exact guard.1.2
          obtain ⟨charged, debit, result⟩ := Option.bind_eq_some_iff.mp continuation
          have resultEq : t = putRun charged next := Option.some.inj result.symm
          exact ⟨old, next, charged, rfl, debit, resultEq, rfl, rfl, rfl,
            rfl, rfl, rfl, rfl, rfl, liveNext⟩

/-- Preserved control is not permission to execute without reloading a required
context. nodeReady is used by the actual advance/call/prepare/dispatch guards. -/
theorem E12_cleared_required_context_blocks_node {contracts : Contracts} {s : State}
    {run : Run} {node : Node} {object : ObjectId} (cleared : run.loaded = [])
    (required : object ∈ node.requiredContext) : nodeReady contracts s run node = false := by
  have absent : subset node.requiredContext [] = false := by
    cases checked : subset node.requiredContext [] with
    | false => rfl
    | true =>
        have present := List.all_eq_true.mp checked object required
        simp at present
  simp [nodeReady, cleared, absent]

/-- Resumption cannot free physical reservations or reset effect/claim history. -/
theorem E12_resume_preserves_effect_history {contracts : Contracts} {s t : State} {activation : ActivationId}
    (h : resume contracts s activation = some t) :
    t.effects = s.effects ∧ t.claims = s.claims ∧
      t.ownership = s.ownership ∧ t.receipts = s.receipts ∧ t.ledger = s.ledger := by
  simp only [resume, putRun] at h
  operational_success h
  all_goals exact ⟨rfl, rfl, rfl, rfl, rfl⟩

end Kctx.Engineering
