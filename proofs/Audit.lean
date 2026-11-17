import Lean
import Writfold

/-!
# Axiom audit (docs/writfold/aeneas.md)

Every theorem under `Writfold.Core` is audited: its transitive axioms must
be a subset of Lean's standard three. Anything else — `sorryAx`, a project
axiom, an Aeneas `axiom` standing in for an unmodelled external — fails the
build. The headline claims are also required to exist, so a deleted or
renamed theorem cannot pass silently.
-/
open Lean Elab Command

/-- Lean's standard axioms. -/
def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

/-- The required claims: every theorem a mutant in `proofs/mutants.py` names
(`mutants.py` refuses to run when one is missing here), and the headline
statements of each kernel (docs/writfold/aeneas.md). -/
def claims : List Name := [
  ``Writfold.Core.same_spec,
  ``Writfold.Core.same_loop_spec,
  -- the run graph (writfold_core::plan)
  ``Writfold.Core.Plan.closed_spec,
  ``Writfold.Core.Plan.current_loop_spec,
  ``Writfold.Core.Plan.current_spec,
  ``Writfold.Core.Plan.current_least,
  ``Writfold.Core.Plan.first_owed_loop_spec,
  ``Writfold.Core.Plan.advance_check_spec,
  ``Writfold.Core.Plan.advance_ok_iff,
  ``Writfold.Core.Plan.stale_spec,
  -- skipping and closing (policy spec 3.3)
  ``Writfold.Core.Plan.skip_check_spec,
  ``Writfold.Core.Plan.skip_ok_iff,
  ``Writfold.Core.Plan.skipped_spec,
  ``Writfold.Core.Plan.locked_skip_never_closes,
  ``Writfold.Core.Plan.locked_stays_current,
  ``Writfold.Core.Plan.close_check_spec,
  -- a team's policy (writfold_core::policy, policy spec 3.1, 3.4, 3.5)
  ``Writfold.Core.Policy.met_spec,
  ``Writfold.Core.Policy.met_default,
  ``Writfold.Core.Policy.met_narrows,
  ``Writfold.Core.Policy.waiver_rejected_never_met,
  ``Writfold.Core.Policy.pin_one_spec,
  ``Writfold.Core.Policy.pin_one_same_iff,
  ``Writfold.Core.Policy.pin_one_unpinned_iff,
  ``Writfold.Core.Policy.pin_all_spec,
  ``Writfold.Core.Policy.any_moved_loop_spec,
  ``Writfold.Core.Policy.pin_all_moved_iff,
  ``Writfold.Core.Policy.pin_all_same_iff,
  ``Writfold.Core.Policy.close_spec,
  ``Writfold.Core.Policy.close_passed_iff,
  ``Writfold.Core.Policy.close_waived_iff,
  ``Writfold.Core.Policy.close_open_iff,
  ``Writfold.Core.Policy.summary_spec,
  ``Writfold.Core.Policy.summary_complete_iff,
  ``Writfold.Core.Policy.moved_never_complete,
  ``Writfold.Core.Policy.open_never_complete,
  ``Writfold.Core.Policy.exception_never_clean,
  ``Writfold.Core.Policy.empty_never_complete,
  -- the splice model: proved for `plan::splice`; the product places spliced
  -- steps in the fold (`writfold-kb` plan/fold.rs), tested against this model
  ``Writfold.Core.Plan.splice_loop0_spec,
  ``Writfold.Core.Plan.splice_loop1_spec,
  ``Writfold.Core.Plan.splice_loop2_spec,
  ``Writfold.Core.Plan.splice_spec,
  ``Writfold.Core.Plan.splice_keeps_existing,
  ``Writfold.Core.Plan.splice_keeps_prefix,
  ``Writfold.Core.Plan.splice_new_open,
  ``Writfold.Core.Plan.splice_current,
  ``Writfold.Core.Plan.contains_loop_spec,
  ``Writfold.Core.Plan.splice_check_spec,
  ``Writfold.Core.Plan.splice_check_already_iff,
  ``Writfold.Core.Plan.splice_idempotent,
  ``Writfold.Core.Plan.splice_check_depth,
  ``Writfold.Core.Plan.splice_admitted,
  -- gate obligations (writfold_core::gate)
  ``Writfold.Core.Gate.outcome_spec,
  ``Writfold.Core.Gate.outcome_pass_iff,
  ``Writfold.Core.Gate.outcome_fail_iff,
  ``Writfold.Core.Gate.state_loop_spec,
  ``Writfold.Core.Gate.state_spec,
  ``Writfold.Core.Gate.state_satisfied_iff,
  ``Writfold.Core.Gate.state_other_subject,
  -- evidence bound to its run and subject (policy spec 3.6)
  ``Writfold.Core.Gate.bound_spec,
  ``Writfold.Core.Gate.current_spec,
  ``Writfold.Core.Gate.newest_bound_loop_spec,
  ``Writfold.Core.Gate.newest_bound_spec,
  ``Writfold.Core.Gate.state_unbound_never_satisfies,
  ``Writfold.Core.Gate.state_ignores_unbound,
  ``Writfold.Core.Gate.attests_spec,
  ``Writfold.Core.Gate.standing_spec,
  ``Writfold.Core.Gate.standing_satisfied_iff,
  ``Writfold.Core.Gate.standing_attested_iff,
  ``Writfold.Core.Gate.standing_waived_iff,
  ``Writfold.Core.Gate.met_spec,
  ``Writfold.Core.Gate.standing_met_iff,
  ``Writfold.Core.Gate.satisfied_only_by_current_pass,
  -- about the current subject: the digest comparisons and the advance on
  -- the gates as found (writfold_core::gate, writfold_core::plan)
  ``Writfold.Core.Gate.about_spec,
  ``Writfold.Core.Gate.evidence_spec,
  ``Writfold.Core.Gate.judge_loop_spec,
  ``Writfold.Core.Gate.judge_spec,
  ``Writfold.Core.Gate.judge_satisfied_iff,
  ``Writfold.Core.Gate.judge_other_bytes,
  ``Writfold.Core.Gate.judge_empty_content,
  ``Writfold.Core.Gate.stands_spec,
  ``Writfold.Core.Gate.statements_loop_spec,
  ``Writfold.Core.Gate.statements_spec,
  ``Writfold.Core.Gate.met_only_when_current,
  ``Writfold.Core.Plan.fact_met_spec,
  ``Writfold.Core.Plan.met_all_loop_spec,
  ``Writfold.Core.Plan.met_all_spec,
  ``Writfold.Core.Plan.advance_facts_spec,
  ``Writfold.Core.Plan.advance_facts_ok_iff,
  ``Writfold.Core.Plan.advance_undefined_refused,
  ``Writfold.Core.Plan.done_only_when_met,
  -- checklist gates (writfold_core::checklist)
  ``Writfold.Core.Checklist.item_spec,
  ``Writfold.Core.Checklist.covered_under_spec,
  ``Writfold.Core.Checklist.not_applicable_rejected_never_covers,
  ``Writfold.Core.Checklist.item_supported_iff,
  ``Writfold.Core.Checklist.item_none_found_iff,
  ``Writfold.Core.Checklist.item_covered_iff,
  ``Writfold.Core.Checklist.owes_spec,
  ``Writfold.Core.Checklist.first_owed_loop_spec,
  ``Writfold.Core.Checklist.first_owed_spec,
  ``Writfold.Core.Checklist.stale_spec,
  ``Writfold.Core.Checklist.any_stale_loop_spec,
  ``Writfold.Core.Checklist.any_stale_spec,
  ``Writfold.Core.Checklist.rollup_under_spec,
  ``Writfold.Core.Checklist.rollup_covered_iff,
  ``Writfold.Core.Checklist.rollup_optional_irrelevant,
  ``Writfold.Core.Checklist.state_spec,
  ``Writfold.Core.Checklist.standing_spec,
  ``Writfold.Core.Checklist.standing_attested_iff,
  ``Writfold.Core.Checklist.standing_never_satisfied,
  ``Writfold.Core.Checklist.standing_met_iff,
  ``Writfold.Core.Checklist.checklist_met_iff,
  -- deliveries and the delivery guard (writfold_core::delivery)
  ``Writfold.Core.Delivery.deciding_loop_spec,
  ``Writfold.Core.Delivery.deciding_spec,
  ``Writfold.Core.Delivery.unmet_spec,
  ``Writfold.Core.Delivery.is_waived_spec,
  ``Writfold.Core.Delivery.is_attested_spec,
  ``Writfold.Core.Delivery.standing_loop_spec,
  ``Writfold.Core.Delivery.standing_spec,
  ``Writfold.Core.Delivery.standing_met_iff,
  ``Writfold.Core.Delivery.standing_satisfied_iff,
  ``Writfold.Core.Delivery.standing_first_unmet,
  ``Writfold.Core.Delivery.ready_iff,
  ``Writfold.Core.Delivery.empty_never_ready,
  ``Writfold.Core.Delivery.rechecked_spec,
  ``Writfold.Core.Delivery.guard_allow_iff,
  -- guard rules (writfold_core::guard)
  ``Writfold.Core.Guard.met_spec,
  ``Writfold.Core.Guard.all_met_loop_spec,
  ``Writfold.Core.Guard.all_met_spec,
  ``Writfold.Core.Guard.verdict_spec,
  ``Writfold.Core.Guard.decide_spec,
  ``Writfold.Core.Guard.decide_pass_iff,
  ``Writfold.Core.Guard.unknown_never_passes,
  ``Writfold.Core.Guard.combine_spec,
  ``Writfold.Core.Guard.combine_deny_iff,
  ``Writfold.Core.Guard.combine_pass_iff,
  ``Writfold.Core.Guard.combineOf_comm,
  ``Writfold.Core.Guard.combineOf_assoc,
  ``Writfold.Core.Guard.combineOf_pass_left,
  ``Writfold.Core.Guard.effect_spec,
  ``Writfold.Core.Guard.effect_audit_passes,
  ``Writfold.Core.Guard.effect_enforce,
  ``Writfold.Core.Guard.ruled_spec,
  ``Writfold.Core.Guard.decide_all_loop_spec,
  ``Writfold.Core.Guard.decide_all_pass_iff,
  ``Writfold.Core.Guard.decide_all_deny_iff,
  ``Writfold.Core.Guard.logged_iff,
  ``Writfold.Core.Guard.gates_fact_met_iff,
  ``Writfold.Core.Guard.gates_fact_never_unknown,
  ``Writfold.Core.Guard.run_current_met_iff,
  ``Writfold.Core.Guard.step_fact_met_iff,
  ``Writfold.Core.Guard.step_fact_unknown_iff,
  ``Writfold.Core.Guard.gate_fact_met_iff,
  ``Writfold.Core.Guard.gate_fact_unknown_iff,
  ``Writfold.Core.Guard.gates_read_met_iff,
  ``Writfold.Core.Guard.nudge_spec,
  ``Writfold.Core.Guard.nudge_with_iff,
  -- what grounds evidence (writfold_core::ground, writfold_core::observe)
  ``Writfold.Core.Ground.of_gate_spec,
  ``Writfold.Core.Ground.of_gate_grounded_iff,
  ``Writfold.Core.Ground.of_gate_attested_iff,
  ``Writfold.Core.Ground.waiver_never_grounds,
  ``Writfold.Core.Ground.of_receipt_grounded_iff,
  ``Writfold.Core.Ground.of_receipt_never_attested,
  ``Writfold.Core.Ground.of_citations_grounded_iff,
  ``Writfold.Core.Ground.of_citations_never_attested,
  ``Writfold.Core.Ground.of_token_grounded_iff,
  ``Writfold.Core.Ground.embeds_iff,
  ``Writfold.Core.Ground.grounds_iff,
  ``Writfold.Core.Ground.of_observation_grounded_iff,
  ``Writfold.Core.Ground.of_statement_spec,
  ``Writfold.Core.Ground.moved_iff,
  -- the Weft assessed gate's basis (writfold_core::assess)
  ``Writfold.Core.Assess.assumption_loop_spec,
  ``Writfold.Core.Assess.assumption_spec,
  ``Writfold.Core.Assess.basis_loop_spec,
  ``Writfold.Core.Assess.basis_spec,
  ``Writfold.Core.Assess.met_spec,
  ``Writfold.Core.Assess.outcome_loop_spec,
  ``Writfold.Core.Assess.outcome_spec,
  ``Writfold.Core.Assess.outcome_pass_iff,
  ``Writfold.Core.Assess.hypothetical_never_passes,
]

/-- Every theorem under `Writfold.Core`, sorted by name. -/
def kernelTheorems (env : Environment) : Array Name :=
  let audited : Name := `Writfold.Core
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if audited.isPrefixOf decl && decl.isInternal = false && info.isThm then acc.push decl
    else acc).qsort (fun a b => a.toString < b.toString)

-- `name` is a reserved token once the Aeneas library is imported (its attribute
-- syntax), so declarations are called `decl` here.
def auditAxioms : CommandElabM Unit := do
  let env ← getEnv
  let sorted := kernelTheorems env
  if sorted.isEmpty then
    throwError "axiom audit: no theorems under Writfold.Core"
  for decl in sorted do
    let axioms := (← collectAxioms decl).qsort (fun a b => a.toString < b.toString)
    logInfo m!"AXIOMS {decl}: {axioms.toList}"
    for ax in axioms do
      unless allowedAxioms.contains ax do
        throwError "axiom audit: {decl} depends on {ax}, which is not a standard Lean axiom"
  for claim in claims do
    unless sorted.contains claim do
      throwError "axiom audit: required claim {claim} is missing or is not a theorem"
  logInfo m!"AXIOM_AUDIT ok: {sorted.size} theorems, {claims.length} required claims, axioms ⊆ {allowedAxioms}"

run_cmd auditAxioms
