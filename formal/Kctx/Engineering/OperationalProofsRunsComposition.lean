import Kctx.Engineering.OperationalProofsRunsLists

set_option autoImplicit false
namespace Kctx.Engineering

/-- A successful call preserves the waiting parent and adds the exactly pinned child. -/
theorem call_preserves_run_representation {contracts : Contracts} {s t : State}
    {parent invocation : ActivationId} {node : NodeId} {callee : Procedure} {binding : Binding}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : call contracts s parent node invocation callee binding = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have body : domainBody contracts s (.call parent node invocation callee binding) = some t := h
  have old := domainBody_transports_old_run_representation wf body
  have anchors := domainBody_preserves_run_anchors wf body
  unfold call at h
  cases hp : findRun s parent with
  | none => simp [hp] at h
  | some p =>
    simp only [hp, bind, Option.bind] at h
    cases hn : findNode p.procedure node with
    | none => simp [hn] at h
    | some n =>
      simp only [hn, bind, Option.bind] at h
      cases hg : currentGrant s p with
      | none => simp [hg] at h
      | some g =>
        simp only [hg, bind, Option.bind] at h
        cases form : n.form <;> simp only [form] at h
        all_goals try contradiction
        rename_i target
        obtain ⟨checked, accepted⟩ := Option.ite_none_right_eq_some.mp h
        obtain ⟨preconditions, accepted⟩ := Option.ite_none_right_eq_some.mp accepted
        cases chargedEq : charge s p.id n.cost with
        | none => simp [chargedEq] at accepted
        | some charged =>
          simp only [chargedEq, bind, Option.bind, Option.some.injEq] at accepted
          subst t
          have pm := runs_find_member hp
          have nm := runs_node_member hn
          have ni := runs_node_id hn
          have base := old p pm
          have sameRuns : charged.runs = s.runs := by rw [charge_exact_state chargedEq]
          have ready : enabled s p n = true := by
            simp only [nodeReady, Bool.and_eq_true] at checked
            grind only
          have fresh := runs_enabled_fresh ready
          simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem,
            Bool.not_eq_true, decide_eq_true_eq] at checked
          have bindingEq : binding = p.binding := by grind only
          have childFresh : s.runs.any (·.invocation == invocation) = false := by grind only
          have newRep := runs_called_representation (s := s) base nm form
            (catalog := by
              change callee ∈ charged.catalog
              rw [charge_exact_state chargedEq]
              grind only)
            (input := by grind only)
            (closure := by
              have retainedCallee : retained s callee.closure = true := by grind only
              simpa only [putRun, charge_exact_state chargedEq] using runs_retained retainedCallee)
            (grant := by
              simpa only [putRun, charge_exact_state chargedEq] using (runs_grant hg).1)
            (grantRoot := (runs_grant hg).2)
            (grantBinding := (runs_grant_scope hg).2.2.1)
            (rank := by grind only)
            (pins := by grind only)
            (parent := anchors p pm)
            (invocation := invocation)
          have childMember : calledRun s p n g target invocation callee ∈
              ({ (putRun charged { p with control := { p.control with
                pending := ⟨node, invocation, target, binding,
                  (calledRun s p n g target invocation callee).input⟩ :: p.control.pending } }) with
                runs := calledRun s p n g target invocation callee ::
                  (putRun charged { p with control := { p.control with
                    pending := ⟨node, invocation, target, binding,
                      (calledRun s p n g target invocation callee).input⟩ :: p.control.pending } }).runs }).runs :=
            by simp
          have waiting := base.wait nm form fresh.2 fresh.1
            (runs_pending_child_fresh (wf.runs p pm) childFresh) childMember
            rfl rfl (by change callee.id = target.procedure; grind only)
            (by change callee.revision = target.revision; grind only)
          apply runs_cons_represented newRep
          change ∀ r ∈ charged.runs.map (fun r =>
            if r.invocation == p.invocation then
              { p with control := { p.control with pending :=
                ⟨node, invocation, target, binding, nodeInput p n⟩ :: p.control.pending } }
            else r), _
          have replaced := runs_replace_represented
            (next := { p with
              control := { p.control with
                pending := ⟨node, invocation, target, binding, nodeInput p n⟩ :: p.control.pending } })
            old (by simpa only [calledRun, ni, bindingEq] using waiting)
          intro current member
          exact replaced current (by simpa only [sameRuns] using member)

/-- Returning consumes the selected waiting entry, records its typed receipt, and
completes the child without changing either activation's historical identity. -/
theorem returnChild_preserves_run_representation {contracts : Contracts} {s t : State}
    {parent child : ActivationId} {receipt : Receipt}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : returnChild contracts s parent child receipt = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  have body : domainBody contracts s (.returnChild parent child receipt) = some t := h
  have old := domainBody_transports_old_run_representation wf body
  have storage := (domainBody_run_storage body).2.1
  have catalogEq : t.catalog = s.catalog := by
    unfold returnChild at h
    operational_success h
    all_goals rfl
  unfold returnChild at h
  cases hp : findRun s parent with
  | none => simp [hp] at h
  | some p =>
    simp only [hp, bind, Option.bind] at h
    cases hc : findRun s child with
    | none => simp [hc] at h
    | some c =>
      simp only [hc, bind, Option.bind] at h
      cases hw : p.control.pending.find? (·.child == child) with
      | none => simp [hw] at h
      | some waiting =>
        simp only [hw, bind, Option.bind] at h
        cases hn : findNode p.procedure waiting.node with
        | none => simp [hn] at h
        | some n =>
          simp only [hn, bind, Option.bind] at h
          obtain ⟨checked, accepted⟩ := Option.ite_none_right_eq_some.mp h
          have pm := runs_find_member hp
          have cm := runs_find_member hc
          have nm := runs_node_member hn
          have wm := List.mem_of_find?_eq_some hw
          have wi : waiting.child = child := by simpa using List.find?_some hw
          have ni := runs_node_id hn
          simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem] at checked
          have form : n.form = .call waiting.target := by grind only
          have outputType : receipt.output.type = n.output := by
            have output := runs_typed_call_output wf pm nm form
            grind only
          have receiptMember : receipt ∈ s.receipts := by grind only
          have retainedOutput : ∀ object ∈ receipt.output.objects, object ∈ t.retained := by
            intro object member
            apply storage object
            exact (wf.receipts receipt receiptMember).retained object (by simp [member])
          have typed : NodeTyped t.catalog contracts.predicates p.procedure n := by
            rw [catalogEq]
            exact (wf.catalog_authority p.procedure (wf.runs p pm).catalog).2.2.1.2.2.2.2.2 n nm
          have parentBase := old p pm
          have childRep := (old c cm).status .completed
          split at accepted
          · rename_i success
            have parentRep := parentBase.return_result nm wm ni
              (v := receipt.output) (outcome := .success) (fun _ => outputType) retainedOutput
            simp only [Option.some.injEq] at accepted
            subst t
            apply runs_replace_represented
              (runs_replace_represented old childRep)
            simpa only [wi] using parentRep
          · rename_i failed
            have notSuccess : receipt.outcome ≠ .success := by simpa using failed
            have parentRep := parentBase.return_failure nm wm typed ni notSuccess retainedOutput
            simp only [Option.some.injEq] at accepted
            subst t
            apply runs_replace_represented
              (runs_replace_represented old childRep)
            simpa only [wi] using parentRep

end Kctx.Engineering
