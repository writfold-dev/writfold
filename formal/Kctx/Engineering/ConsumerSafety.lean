import Kctx.Engineering.ConsumerSafetyFacts

set_option autoImplicit false
set_option maxRecDepth 4096
namespace Kctx.Engineering

theorem advanceNode_certificate {contracts : Contracts} {s t : State}
    {activation : ActivationId} {id : NodeId} {value : TypedValue}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : advanceNode contracts s activation id value = some t) :
    ∃ run node, AdvanceCertificate contracts s.toOperationalState t.runs activation id value run node := by
  cases found : findRun s activation with
  | none => simp [advanceNode, found] at h
  | some run =>
    cases selected : findNode run.procedure id with
    | none => simp [advanceNode, found, selected] at h
    | some node =>
      simp only [advanceNode, found, selected, bind, Option.bind] at h
      obtain ⟨checked, accepted⟩ := consumer_if_some h
      obtain ⟨consumer, completed⟩ := consumer_if_some accepted
      have role : AdvanceConsumer contracts s.toOperationalState run node value := by
        cases form : node.form with
        | read object =>
            simpa only [AdvanceConsumer, form, Bool.and_eq_true, beq_iff_eq,
              List.contains_iff_mem] using consumer
        | select routes => simp [form] at consumer
        | judge predicate =>
            simpa only [AdvanceConsumer, form, List.any_eq_true, Bool.and_eq_true,
              beq_iff_eq, and_assoc] using consumer
        | check predicate =>
            simpa only [AdvanceConsumer, form, List.any_eq_true, Bool.and_eq_true,
              beq_iff_eq, and_assoc] using consumer
        | call target => simp [form] at consumer
        | effect capability =>
            simpa only [AdvanceConsumer, form, List.any_eq_true, Bool.and_eq_true,
              beq_iff_eq, and_assoc, consumer_receiptValid] using consumer
        | join calls =>
            simp only [AdvanceConsumer, form]
            simp only [form, joinReady] at consumer
            intro call member
            have checkedCall := List.all_eq_true.mp consumer call member
            simpa only [List.any_eq_true, Option.any_eq_true, Bool.and_eq_true,
              beq_iff_eq, and_assoc] using checkedCall
        | «end» =>
            simp only [AdvanceConsumer, form]
            have endChecked : (run.control.pending = [] ∧
                s.effects.any (fun e => e.intent.invocation == run.invocation &&
                  (e.outcome == .prepared || e.outcome == .claimed ||
                    e.outcome == .unknown || e.outcome == .partial)) = false) ∧
                predicatesHold contracts s run run.procedure.postconditions value = true := by
              simpa only [form, Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_iff] using consumer
            exact ⟨endChecked.1.1, endChecked.1.2, predicatesHold_qualified endChecked.2⟩
      have progress : ∃ middle : State, middle.runs = s.runs ∧
          putRun middle (if node.form == .«end» && run.parent.isNone then
            { (recordResult run node value .success) with status := .completed }
          else recordResult run node value .success) = t := by
        cases form : node.form <;> simp only [form] at completed ⊢
        all_goals
          first
          | exact ⟨s, rfl, Option.some.inj completed⟩
          | cases charged : charge s run.id node.cost with
            | none => simp [charged] at completed
            | some middle =>
                simp only [charged, bind, Option.bind, Option.some.injEq] at completed
                exact ⟨middle, consumer_charge_runs charged, completed⟩
      obtain ⟨middle, unchangedRuns, actual⟩ := progress
      subst t
      simp only [Bool.and_eq_true, beq_iff_eq] at checked
      have ready := checked.1.1.1
      simp only [nodeReady, Bool.and_eq_true] at ready
      have runMember : run ∈ s.runs := List.mem_of_find?_eq_some found
      have nodeMember : node ∈ run.procedure.nodes := List.mem_of_find?_eq_some selected
      have typed := (wf.catalog_authority run.procedure (wf.runs run runMember).catalog).2.2.1
      refine ⟨run, node, {
        invocation := found
        selected := selected
        typed := typed.2.2.2.2.2 node nodeMember
        enabled := by simpa only [consumer_enabled] using ready.1.1
        context := ready.1.2
        dependencies := consumer_enabled_dependencies ready.1.1
        preconditions := predicatesHold_qualified ready.2
        output_type := checked.1.1.2
        output_retained := checked.1.2
        postconditions := predicatesHold_qualified checked.2
        consumer := role
        qualified_receipts := fun receipt member => (wf.receipts receipt member).qualified
        recorded := ?_ }⟩
      cases terminal : (node.form == .«end» && run.parent.isNone) <;>
        simp [terminal, putRun, unchangedRuns, recordResult]

theorem returnChild_certificate {contracts : Contracts} {s t : State}
    {parent child : ActivationId} {receipt : Receipt}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : returnChild contracts s parent child receipt = some t) :
    ∃ caller callee pending node,
      ReturnCertificate contracts s.toOperationalState t.runs t.returned
        parent child receipt caller callee pending node := by
  cases callerFound : findRun s parent with
  | none => simp [returnChild, callerFound] at h
  | some caller =>
    cases calleeFound : findRun s child with
    | none => simp [returnChild, callerFound, calleeFound] at h
    | some callee =>
      cases pendingFound : caller.control.pending.find? (·.child == child) with
      | none => simp [returnChild, callerFound, calleeFound, pendingFound] at h
      | some pending =>
        cases nodeFound : findNode caller.procedure pending.node with
        | none => simp [returnChild, callerFound, calleeFound, pendingFound, nodeFound] at h
        | some node =>
          simp only [returnChild, callerFound, calleeFound, pendingFound, nodeFound,
            bind, Option.bind] at h
          obtain ⟨checked, result⟩ := consumer_if_some h
          simp only [Option.some.injEq] at result
          subst t
          simp only [Bool.and_eq_true, beq_iff_eq, List.isEmpty_iff,
            List.contains_iff_mem, Bool.not_eq_true, and_assoc] at checked
          rcases checked with ⟨callerLive, calleeLive, root, parentMatch, binding,
            pendingBinding, input, callerInput, procedure, revision, callNode,
            childrenSettled, terminalChecked, role, invocation, rootInput, outputType,
            retainedReceipt, receiptChecked, postChecked, effectsSettled⟩
          have terminalInput : ∃ terminalNode,
              findNode callee.procedure receipt.node = some terminalNode ∧
                receipt.input = nodeInput callee terminalNode := by
            have all := receiptChecked
            simp only [receiptValid, Bool.and_eq_true, and_assoc] at all
            obtain ⟨_, _, _, _, _, nodeChecked, _⟩ := all
            obtain ⟨terminalNode, located, nodeGuards⟩ := (Option.any_eq_true _ _).mp nodeChecked
            simp only [Bool.and_eq_true, beq_iff_eq, and_assoc] at nodeGuards
            exact ⟨terminalNode, located, nodeGuards.1⟩
          have postconditions : receipt.outcome = .success →
              QualifiedPredicates contracts s.toOperationalState callee callee.procedure.postconditions receipt.output ∧
              QualifiedPredicates contracts s.toOperationalState caller node.postconditions receipt.output := by
            intro success
            have accepted : predicatesHold contracts s callee callee.procedure.postconditions receipt.output = true ∧
                predicatesHold contracts s caller node.postconditions receipt.output = true := by
              simpa [success, Bool.and_eq_true] using postChecked
            exact ⟨predicatesHold_qualified accepted.1, predicatesHold_qualified accepted.2⟩
          refine ⟨caller, callee, pending, node, {
            caller_found := callerFound
            callee_found := calleeFound
            pending_found := pendingFound
            node_found := nodeFound
            caller_live := by simpa only [consumer_live] using callerLive
            callee_live := by simpa only [consumer_live] using calleeLive
            root := root
            parent_match := parentMatch
            binding := binding
            pending_binding := pendingBinding
            input := input
            caller_input := callerInput
            procedure := procedure
            revision := revision
            call_node := callNode
            children_settled := childrenSettled
            terminal := ?_
            receipt_role := role
            receipt_invocation := invocation
            receipt_input := rootInput
            terminal_input := terminalInput
            output_type := outputType
            receipt_retained := retainedReceipt
            receipt_qualified := (wf.receipts receipt retainedReceipt).qualified
            receipt_current := by simpa only [consumer_receiptValid] using receiptChecked
            postconditions := postconditions
            effects_settled := by simpa only [Bool.not_eq_true'] using effectsSettled
            recorded_runs := by
              cases success : (receipt.outcome == .success) <;>
                simp [success, putRun, recordResult, recordFailure]
            recorded_return := rfl }⟩
          simpa only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, and_assoc] using terminalChecked

/-- Actual selected append, with exact source witnesses and exact result update.
No successor-well-formedness test or successful-step predicate is the certificate. -/
theorem E04_request_advance_integrity {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {activation : ActivationId} {node : NodeId} {value : TypedValue}
    (h : step contracts s (.append stream expected record (.advanceNode activation node value)) = some t) :
    ∃ run selected,
      AdvanceCertificate contracts s.toOperationalState t.runs activation node value run selected := by
  obtain ⟨before, after, source, executed, runs, _⟩ := consumer_append_body h
  have sourceValid : OperationalWellFormed contracts before.toOperationalState := by
    rw [source]
    exact consumer_append_source_valid h
  have advanced : advanceNode contracts before activation node value = some after := executed
  obtain ⟨run, selected, certificate⟩ := advanceNode_certificate sourceValid advanced
  refine ⟨run, selected, ?_⟩
  rw [runs, ← source]
  exact certificate

/-- Successful returns expose both callee and caller-node postcondition witnesses;
non-success returns remain explicitly non-success and cannot satisfy a Join. -/
theorem E04_request_return_integrity {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId}
    {parent child : ActivationId} {receipt : Receipt}
    (h : step contracts s (.append stream expected record (.returnChild parent child receipt)) = some t) :
    ∃ caller callee pending node,
      ReturnCertificate contracts s.toOperationalState t.runs t.returned
        parent child receipt caller callee pending node := by
  obtain ⟨before, after, source, executed, runs, returned⟩ := consumer_append_body h
  have sourceValid : OperationalWellFormed contracts before.toOperationalState := by
    rw [source]
    exact consumer_append_source_valid h
  have returnedChild : returnChild contracts before parent child receipt = some after := executed
  obtain ⟨caller, callee, pending, node, certificate⟩ := returnChild_certificate sourceValid returnedChild
  refine ⟨caller, callee, pending, node, ?_⟩
  rw [runs, returned, ← source]
  exact certificate

end Kctx.Engineering
