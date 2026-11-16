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

/-- The headline claims: byte identity, the run graph (the current step, the
advance check's refusal order, staleness, splicing), gate obligations (only a
clean current pass satisfies; a statement never does) and the assessed gate's
basis (a hypothetical entailment never passes). -/
def claims : List Name := [
  ``Writfold.Core.same_spec,
  -- the run graph (writfold_core::plan)
  ``Writfold.Core.Plan.current_spec,
  ``Writfold.Core.Plan.current_least,
  ``Writfold.Core.Plan.advance_check_spec,
  ``Writfold.Core.Plan.advance_ok_iff,
  ``Writfold.Core.Plan.stale_spec,
  ``Writfold.Core.Plan.stale_never_current,
  ``Writfold.Core.Plan.splice_spec,
  ``Writfold.Core.Plan.splice_keeps_existing,
  ``Writfold.Core.Plan.splice_keeps_prefix,
  ``Writfold.Core.Plan.splice_new_open,
  ``Writfold.Core.Plan.splice_current,
  ``Writfold.Core.Plan.splice_check_spec,
  ``Writfold.Core.Plan.splice_check_already_iff,
  ``Writfold.Core.Plan.splice_idempotent,
  ``Writfold.Core.Plan.splice_check_depth,
  ``Writfold.Core.Plan.splice_admitted,
  -- gate obligations (writfold_core::gate)
  ``Writfold.Core.Gate.outcome_pass_iff,
  ``Writfold.Core.Gate.outcome_fail_iff,
  ``Writfold.Core.Gate.state_spec,
  ``Writfold.Core.Gate.state_satisfied_iff,
  ``Writfold.Core.Gate.state_other_subject,
  ``Writfold.Core.Gate.standing_satisfied_iff,
  ``Writfold.Core.Gate.standing_attested_iff,
  ``Writfold.Core.Gate.standing_waived_iff,
  ``Writfold.Core.Gate.met_spec,
  ``Writfold.Core.Gate.standing_met_iff,
  ``Writfold.Core.Gate.satisfied_only_by_current_pass,
  -- the Weft assessed gate's basis (writfold_core::assess)
  ``Writfold.Core.Assess.assumption_spec,
  ``Writfold.Core.Assess.basis_spec,
  ``Writfold.Core.Assess.met_spec,
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
