import Kctx.Engineering.EffectsSafetyStep
import Kctx.Engineering.HistorySafetyFrames
import Kctx.Engineering.OperationalCongruence

set_option autoImplicit false
set_option maxRecDepth 4096
namespace Kctx.Engineering

/-- Consumer guards read operational inputs, not a reconstructed ledger or a
projection cut. This projection supplies those same inputs to existing guards. -/
def consumerSource (s : OperationalState) : State :=
  { toOperationalState := s, ledger := {}, cuts := [] }

theorem consumer_ancestryLive (s : State) (fuel : Nat) (run : Run) :
    ancestryLive (consumerSource s.toOperationalState) fuel run = ancestryLive s fuel run :=
  ancestryLive_operational (consumerSource s.toOperationalState) s rfl fuel run

theorem consumer_live (s : State) (run : Run) :
    live (consumerSource s.toOperationalState) run = live s run :=
  live_operational (consumerSource s.toOperationalState) s rfl run

theorem consumer_enabled (s : State) (run : Run) (node : Node) :
    enabled (consumerSource s.toOperationalState) run node = enabled s run node := by
  unfold enabled
  rw [consumer_live]
  cases s
  rfl

theorem consumer_receiptValid (s : State) (run : Run) (receipt : Receipt) :
    receiptValid (consumerSource s.toOperationalState) run receipt = receiptValid s run receipt :=
  receiptValid_operational (consumerSource s.toOperationalState) s rfl run receipt

def QualifiedPredicates (contracts : Contracts) (s : OperationalState) (r : Run)
    (predicates : List PredicateId) (value : TypedValue) : Prop :=
  ∀ predicate ∈ predicates,
    PredicateSignature.mk predicate value.type .evidence ∈ contracts.predicates ∧
    ∃ evidence ∈ contracts.typedEvidence,
      evidenceApplicable (consumerSource s) r evidence = true ∧
      evidence.predicate = predicate ∧ evidence.input = value ∧ evidence.output.type = .evidence

theorem predicatesHold_qualified {contracts : Contracts} {s : State} {r : Run}
    {predicates : List PredicateId} {value : TypedValue}
    (checked : predicatesHold contracts s r predicates value = true) :
    QualifiedPredicates contracts s.toOperationalState r predicates value := by
  intro predicate member
  have entry := List.all_eq_true.mp checked predicate member
  simp only [Bool.and_eq_true] at entry
  have signature := entry.1
  have evidence := entry.2
  refine ⟨by simpa using signature, ?_⟩
  obtain ⟨receipt, qualified, facts⟩ := List.any_eq_true.mp evidence
  refine ⟨receipt, qualified, ?_⟩
  simpa only [Bool.and_eq_true, beq_iff_eq, and_assoc] using facts

/-- Separate role judgments prevent a preauthorization gate receipt from being
used as an effect result, and forbid advancing a Call without Return. -/
def AdvanceConsumer (contracts : Contracts) (s : OperationalState) (r : Run)
    (n : Node) (value : TypedValue) : Prop :=
  match n.form with
  | .read object => object ∈ r.loaded ∧ value.objects = [object]
  | .judge predicate | .check predicate =>
      ∃ evidence ∈ contracts.typedEvidence,
        evidenceApplicable (consumerSource s) r evidence = true ∧
        evidence.predicate = predicate ∧ evidence.input = nodeInput r n ∧ evidence.output = value
  | .join calls => ∀ node ∈ calls, ∃ result ∈ s.returned,
      result.parent = r.invocation ∧ result.node = node ∧ result.binding = r.binding ∧
      result.receipt.outcome = .success ∧
      ∃ child, findRun (consumerSource s) result.child = some child ∧
        child.id = r.id ∧ child.parent = some r.invocation ∧ child.status = .completed ∧
        child.binding = r.binding ∧ child.procedure.revision = result.procedure ∧ child.input = result.input
  | .«end» => r.control.pending = [] ∧
      s.effects.any (fun e => e.intent.invocation == r.invocation &&
        (e.outcome == .prepared || e.outcome == .claimed || e.outcome == .unknown || e.outcome == .partial)) = false ∧
      QualifiedPredicates contracts s r r.procedure.postconditions value
  | .effect capability => ∃ effect ∈ s.effects,
      effect.intent.invocation = r.invocation ∧ effect.intent.node = n.id ∧
      effect.intent.binding = r.binding ∧ effect.intent.capability = capability ∧
      effect.outcome = .confirmed ∧ ∃ receipt ∈ s.receipts,
        receipt.role = .effectResult ∧ receipt.effect = effect.intent.effect ∧
        receipt.invocation = r.invocation ∧ receipt.node = n.id ∧
        receiptValid (consumerSource s) r receipt = true ∧
        receipt.outcome = .success ∧ receipt.output = value
  | .select _ | .call _ => False

structure AdvanceCertificate (contracts : Contracts) (source : OperationalState)
    (afterRuns : List Run) (activation : ActivationId) (id : NodeId)
    (value : TypedValue) (run : Run) (node : Node) : Prop where
  invocation : findRun (consumerSource source) activation = some run
  selected : findNode run.procedure id = some node
  typed : NodeTyped source.catalog contracts.predicates run.procedure node
  enabled : enabled (consumerSource source) run node = true
  context : subset node.requiredContext run.loaded = true
  dependencies : ∀ dependency ∈ node.dependencies, ∃ result ∈ run.control.results,
    result.node = dependency.node ∧ result.value.type = dependency.type ∧ result.outcome = .success
  preconditions : QualifiedPredicates contracts source run node.preconditions (nodeInput run node)
  output_type : value.type = node.output
  output_retained : retained (consumerSource source) value.objects = true
  postconditions : QualifiedPredicates contracts source run node.postconditions value
  consumer : AdvanceConsumer contracts source run node value
  qualified_receipts : ∀ receipt ∈ source.receipts, receipt ∈ contracts.receipts
  recorded : afterRuns = source.runs.map (fun old => if old.invocation == run.invocation then
    (if node.form == .«end» && run.parent.isNone then
      { (recordResult run node value .success) with status := .completed }
    else recordResult run node value .success) else old)

structure ReturnCertificate (contracts : Contracts) (source : OperationalState)
    (afterRuns : List Run) (afterReturned : List ChildReturn)
    (parent child : ActivationId) (receipt : Receipt)
    (caller callee : Run) (pending : PendingCall) (node : Node) : Prop where
  caller_found : findRun (consumerSource source) parent = some caller
  callee_found : findRun (consumerSource source) child = some callee
  pending_found : caller.control.pending.find? (·.child == child) = some pending
  node_found : findNode caller.procedure pending.node = some node
  caller_live : live (consumerSource source) caller = true
  callee_live : live (consumerSource source) callee = true
  root : callee.id = caller.id
  parent_match : callee.parent = some caller.invocation
  binding : callee.binding = caller.binding
  pending_binding : pending.binding = caller.binding
  input : pending.input = callee.input
  caller_input : pending.input = nodeInput caller node
  procedure : pending.target.procedure = callee.procedure.id
  revision : pending.target.revision = callee.procedure.revision
  call_node : node.form = .call pending.target
  children_settled : callee.control.pending = []
  terminal : ∃ last ∈ callee.procedure.nodes,
    last.form = .«end» ∧ last.id = receipt.node ∧ ∃ result ∈ callee.control.results,
      result.node = last.id ∧ result.outcome = receipt.outcome ∧ result.value = receipt.output
  receipt_role : receipt.role = .procedureResult
  receipt_invocation : receipt.invocation = callee.invocation
  receipt_input : receipt.rootInput = callee.input
  terminal_input : ∃ terminalNode, findNode callee.procedure receipt.node = some terminalNode ∧
    receipt.input = nodeInput callee terminalNode
  output_type : receipt.output.type = pending.target.output
  receipt_retained : receipt ∈ source.receipts
  receipt_qualified : receipt ∈ contracts.receipts
  receipt_current : receiptValid (consumerSource source) callee receipt = true
  postconditions : receipt.outcome = .success →
    QualifiedPredicates contracts source callee callee.procedure.postconditions receipt.output ∧
    QualifiedPredicates contracts source caller node.postconditions receipt.output
  effects_settled : source.effects.any (fun e => e.intent.invocation == callee.invocation &&
    (e.outcome == .prepared || e.outcome == .claimed || e.outcome == .unknown || e.outcome == .partial)) = false
  recorded_runs : afterRuns =
    ((source.runs.map (fun old => if old.invocation == callee.invocation then
      { callee with status := .completed } else old)).map (fun old =>
        if old.invocation == caller.invocation then
          let result := if receipt.outcome == .success then recordResult caller node receipt.output .success
            else recordFailure caller node receipt
          { result with control := { result.control with
            pending := caller.control.pending.filter (·.child != child) } }
        else old))
  recorded_return : afterReturned =
    ⟨caller.invocation, callee.invocation, node.id, callee.binding, callee.procedure.revision,
      callee.input, receipt.output, receipt⟩ :: source.returned

theorem consumer_if_some {α : Type} {condition : Bool} {branch : Option α} {result : α}
    (h : (if condition then branch else none) = some result) :
    condition = true ∧ branch = some result := by
  cases condition <;> simp_all

theorem consumer_enabled_dependencies {s : State} {run : Run} {node : Node}
    (ready : enabled s run node = true) :
    ∀ dependency ∈ node.dependencies, ∃ result ∈ run.control.results,
      result.node = dependency.node ∧ result.value.type = dependency.type ∧ result.outcome = .success := by
  have dependencies : node.dependencies.all (fun dependency =>
      run.control.results.any (fun result => result.node == dependency.node &&
        result.value.type == dependency.type && result.outcome == .success)) = true := by
    simp only [enabled, Bool.and_eq_true] at ready
    grind only
  intro dependency member
  have checked := List.all_eq_true.mp dependencies dependency member
  simpa only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, and_assoc] using checked

theorem consumer_charge_runs {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) : t.runs = s.runs := by
  cases account : s.budgets.find? (·.root == root) with
  | none => simp [charge, account] at h
  | some budget =>
      simp only [charge, account, bind, Option.bind] at h
      obtain ⟨_, result⟩ := consumer_if_some h
      cases result
      rfl

/-- The source validity premise is extracted from the actual successful append,
not assumed for the consumer theorem and never checked on a proposed successor. -/
theorem consumer_append_source_valid {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected record action) = some t) :
    OperationalWellFormed contracts s.toOperationalState := by
  have appended := (step_append_exact h).2.1
  obtain ⟨ready, _⟩ := Ledger.append_success historicalValid s.ledger t.ledger stream expected
    (recordFor contracts s stream record action) t.retained appended
  obtain ⟨_, _, _, _, _, _, _, _, _, valid⟩ := ready
  have source : operationalValid contracts s.toOperationalState = true := by
    simp only [historicalValid, recordFor, Bool.and_eq_true] at valid
    grind only
  exact (operationalValid_iff contracts s.toOperationalState).mp source

theorem consumer_append_body {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {record : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected record action) = some t) :
    ∃ before after : State, before.toOperationalState = s.toOperationalState ∧
      domainBody contracts before action = some after ∧
      t.runs = after.runs ∧ t.returned = after.returned := by
  obtain ⟨next, executed, final⟩ := append_domain_witness h
  let before := historicalState (Ledger.currentPrefix s.ledger stream)
    (recordFor contracts s stream record action)
  change domain contracts before action = some next at executed
  cases body : domainBody contracts before action with
  | none => simp [domain, body] at executed
  | some after =>
      have result : frameDomain before action after = next := by
        simpa only [domain, body, Option.map_some, Option.some.injEq] using executed
      subst next
      refine ⟨before, after, rfl, body, ?_, ?_⟩
      · simpa only [frameDomain] using congrArg OperationalState.runs final
      · simpa only [frameDomain] using congrArg OperationalState.returned final

end Kctx.Engineering
