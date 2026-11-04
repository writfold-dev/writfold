import Kctx.Engineering.OperationalProofsRecordsLinkHelpers

set_option autoImplicit false

namespace Kctx.Engineering

/-- Receipt publication is the only receipt-list writer. Historical receipts are
transported through the actual run-anchor and retention extensions; the new
receipt is justified by the publication guard, not by a post-state filter. -/
theorem records_domainBody_receipts {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ receipt ∈ (frameDomain s action t).receipts,
      ReceiptRepresentation contracts (frameDomain s action t).toOperationalState receipt := by
  have executed : domain contracts s action = some (frameDomain s action t) := by
    simp [domain, h]
  have runs := records_domain_run_extension wf executed
  have retention := records_domain_retention executed
  have before : ∀ receipt ∈ s.receipts,
      ReceiptRepresentation contracts (frameDomain s action t).toOperationalState receipt :=
    fun receipt member => records_receipt_transport runs retention.1 (wf.receipts receipt member)
  cases action with
  | recordReceipt receipt =>
      simp only [domainBody] at h
      obtain ⟨run, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp continuation
      simp only [Bool.and_eq_true, List.contains_iff_mem, beq_iff_eq, and_assoc] at checked
      obtain ⟨qualified, objects, input, binding, procedure, node, output⟩ := checked
      have retainedObjects : ∀ object ∈ receipt.objects ++ receipt.rootInput.objects ++
          receipt.input.objects ++ receipt.output.objects, object ∈ s.retained := by
        simpa only [retained, subset, List.all_eq_true, List.contains_iff_mem] using objects
      have fresh : ReceiptRepresentation contracts
          (frameDomain s (.recordReceipt receipt) t).toOperationalState receipt :=
        records_receipt_new (s := s) (t := frameDomain s (.recordReceipt receipt) t)
          runs retention.1 found qualified retainedObjects input binding procedure node output
      cases result
      exact records_cons_links fresh before
  | _ =>
      simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
        selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, observeEffect,
        observeMaterial, reconcile, reuse, returnChild, rebind, resume, reopen, cancel, quiesce,
        propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
        retire] at h
      operational_success h
      all_goals exact before

/-- Child return is the only returned-record writer. Its exact parent, child,
pending call and node are extracted before the current receipt guard supplies
the new witness; old returns retain their run anchors and receipt membership. -/
theorem records_domainBody_returned {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    ∀ result ∈ (frameDomain s action t).returned,
      ReturnedRepresentation (frameDomain s action t).toOperationalState result := by
  have executed : domain contracts s action = some (frameDomain s action t) := by
    simp [domain, h]
  have runs := records_domain_run_extension wf executed
  have retention := records_domain_retention executed
  have before : ∀ result ∈ s.returned,
      ReturnedRepresentation (frameDomain s action t).toOperationalState result :=
    fun result member => records_returned_transport runs retention.2 (wf.returned result member)
  cases action with
  | returnChild parent child receipt =>
      simp only [domainBody, returnChild] at h
      obtain ⟨p, parentFound, continuation⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨c, childFound, continuation⟩ := Option.bind_eq_some_iff.mp continuation
      obtain ⟨pending, pendingFound, continuation⟩ := Option.bind_eq_some_iff.mp continuation
      obtain ⟨n, nodeFound, continuation⟩ := Option.bind_eq_some_iff.mp continuation
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp continuation
      simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem, and_assoc] at checked
      obtain ⟨_, _, root, edge, _, _, _, _, _, _, _, _, _, role, invocation,
        input, _, member, valid, _, _⟩ := checked
      have fresh : ReturnedRepresentation
          (frameDomain s (.returnChild parent child receipt) t).toOperationalState
          ⟨p.invocation, c.invocation, n.id, c.binding, c.procedure.revision,
            c.input, receipt.output, receipt⟩ :=
        records_return_new (s := s) (t := frameDomain s (.returnChild parent child receipt) t)
          runs retention.2 parentFound childFound root edge member role invocation input valid n.id
      cases result
      exact records_cons_links fresh before
  | _ =>
      simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
        selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, observeEffect,
        observeMaterial, reconcile, reuse, rebind, resume, reopen, cancel, quiesce,
        propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
        retire] at h
      operational_success h
      all_goals exact before

end Kctx.Engineering
