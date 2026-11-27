import Lean
import Extracted.WritfoldCore
import Extracted.WritfoldCore.Compose
import Extracted.WritfoldCore.Invoke
import Extracted.WritfoldCore.Units
import Extracted.WritfoldCore.Vale
import Conform.Sample
open Lean Elab Command Meta

/-- Every translated function with the head of its result type. -/
elab "#kernel_universe" : command => do
  let env ← getEnv
  let ns : Name := `writfold_core
  let mut rows : Array String := #[]
  for (c, info) in env.constants.toList do
    if !ns.isPrefixOf c || c.isInternal then continue
    unless info matches .defnInfo _ do continue
    let s := c.toString
    if (s.splitOn "_loop").length > 1 then continue
    let head ← liftTermElabM <| forallTelescope info.type fun _ body => do
      let body ← whnfR body
      if body.isAppOf ``Aeneas.Std.Result then
        let t := body.getAppArgs[0]!
        return some (t.getAppFn.constName?.map toString |>.getD (toString (← ppExpr t)))
      else return none
    if let some h := head then rows := rows.push s!"FN {s} {h}"
  for r in rows.qsort (· < ·) do logInfo r

/-- A conformance theorem: a triple over one kernel whose postcondition
mentions a constant of the core. Prints the kernel and the core constants. -/
elab "#links" : command => do
  let env ← getEnv
  let ns : Name := `Writfold.Conform
  for (c, info) in env.constants.toList do
    if !ns.isPrefixOf c || c.isInternal then continue
    unless info matches .thmInfo _ do continue
    let r ← liftTermElabM <| forallTelescope info.type fun _ body => do
      unless body.isAppOfArity ``Aeneas.Std.WP.spec 3 do return none
      let prog := body.getAppArgs[1]!
      let kernel := prog.getAppFn.constName?
      let post := body.getAppArgs[2]!
      let core := post.getUsedConstants.filter (fun n => (`Calculus).isPrefixOf n)
      return some (kernel, core)
    if let some (some k, core) := r then
      if (`writfold_core).isPrefixOf k then
        logInfo s!"LINK {k} by {c} core={core.toList}"

#kernel_universe
#links
