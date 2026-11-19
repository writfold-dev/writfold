import Lean
import Writfold

/-!
# Total theorems (tauten adapter; docs/writfold/tauten-adoption.md)

Not part of the `proofs` gate (`proofs/gate.sh`); `proofs/tauten_gate.py`
runs it after the gate passes, for `cargo tauten proofs`.

A theorem under `Writfold.Core` of the shape
`∀ (x₁ : α₁) … (xₙ : αₙ), f x₁ … xₙ ⦃ _ ⦄`, where no `αᵢ` is a proposition
(no hypotheses) and `f` is applied to exactly those variables, states that
`f` never fails (no panic, no overflow, no out-of-bounds access, terminates)
for every input. Each is printed as `TOTAL <theorem> <f>`; every other
Hoare triple over a function, as `SPEC <theorem> <f>`. This is the check
of tauten's proofs template (`Audit.lean`, `totalOver`), over writfold's
theorem namespace instead of a `Proofs` library.
-/

open Lean Elab Command Meta

/-- Every theorem under `Writfold.Core`, sorted by name. -/
def kernelTheorems (env : Environment) : Array Name :=
  let audited : Name := `Writfold.Core
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if audited.isPrefixOf decl && decl.isInternal = false && info.isThm then acc.push decl
    else acc).qsort (fun a b => a.toString < b.toString)

/-- The function a theorem states never fails for every input, if it has
that shape. -/
def totalOver (type : Expr) : MetaM (Option Name) :=
  forallTelescope type fun xs body => do
    for x in xs do
      if ← isProp (← inferType x) then return none
    unless body.isAppOfArity ``Aeneas.Std.WP.spec 3 do return none
    let program := body.getAppArgs[1]!
    let fn := program.getAppFn
    unless fn.isConst do return none
    let args := program.getAppArgs
    unless args.size == xs.size do return none
    for (arg, x) in args.zip xs do
      unless arg == x do return none
    return some fn.constName!

/-- The function a theorem is a Hoare triple about, with or without
hypotheses (`SPEC` lines; `proofs/tauten_claims.py` reads them to group
theorems by function). -/
def specOver (type : Expr) : MetaM (Option Name) :=
  forallTelescope type fun _ body => do
    unless body.isAppOfArity ``Aeneas.Std.WP.spec 3 do return none
    let fn := body.getAppArgs[1]!.getAppFn
    unless fn.isConst do return none
    return some fn.constName!

def reportTotal : CommandElabM Unit := do
  let env ← getEnv
  let mut count : Nat := 0
  for decl in kernelTheorems env do
    if let some info := env.find? decl then
      if let some fn ← liftTermElabM (totalOver info.type) then
        logInfo m!"TOTAL {decl} {fn}"
        count := count + 1
      else if let some fn ← liftTermElabM (specOver info.type) then
        logInfo m!"SPEC {decl} {fn}"
  logInfo m!"TOTAL_CHECK ok: {count} total theorems"

run_cmd reportTotal
