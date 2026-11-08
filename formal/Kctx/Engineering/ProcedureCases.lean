import Lean
import Kctx.Engineering.Procedure

set_option autoImplicit false
namespace Kctx.Engineering.ProcedureCases
open Lean

/-! Shared executable procedure cases. Each case is evaluated by the actual
`validateProcedure`; the exported verdict is Lean's, never a hand-written label.
Rust consumes the JSON to check that every Lean refusal is also a Rust refusal
and every Rust admission is Lean-admitted. This is differential evidence, not a
refinement proof. -/

def nid (n : Nat) : NodeId := ⟨n⟩
def pid (n : Nat) : PredicateId := ⟨n⟩
def oid (n : Nat) : ObjectId := ⟨n⟩

def registry : List PredicateSignature :=
  [⟨pid 1, .artifact, .evidence⟩, ⟨pid 2, .unit, .evidence⟩]

def node (n : Nat) (form : NodeForm) (input output : ValueType) : Node :=
  { id := nid n, form, input, output, dependencies := [], prerequisites := [],
    preconditions := [], postconditions := [], context := ⟨1⟩, requiredContext := [],
    scope := [], resources := [], ceiling := [.read, .check, .write], floor := [.review],
    failure := none, cancellation := none, cost := 1 }

def leaf : Procedure :=
  { id := ⟨2⟩, revision := ⟨10⟩, package := ⟨1⟩, closure := [oid 4],
    inputType := .unit, outputType := .artifact, ceiling := [.read], floor := [.review],
    rank := 0, entry := nid 1,
    nodes := [{ node 1 (.read (oid 4)) .unit .artifact with ceiling := [.read] },
              { node 2 .«end» .artifact .artifact with
                ceiling := [.read], dependencies := [⟨nid 1, .artifact⟩] }],
    preconditions := [pid 2], postconditions := [] }

def target : CallTarget := ⟨⟨2⟩, ⟨10⟩, .unit, .artifact, [.read]⟩

def rootNodes : List Node :=
  [{ node 1 (.read (oid 4)) .unit .artifact with requiredContext := [oid 4] },
   { node 2 (.judge (pid 1)) .artifact .evidence with
     dependencies := [⟨nid 1, .artifact⟩], failure := some (nid 6) },
   node 3 (.call target) .unit .artifact,
   { node 4 (.join [nid 3]) .evidence .evidence with prerequisites := [nid 2] },
   { node 5 (.effect .write) .evidence .artifact with
     resources := [⟨1⟩], prerequisites := [nid 4] },
   node 6 .«end» .unit .artifact]

def root : Procedure :=
  { id := ⟨1⟩, revision := ⟨11⟩, package := ⟨1⟩, closure := [oid 4],
    inputType := .unit, outputType := .artifact, ceiling := [.read, .check, .write],
    floor := [.review], rank := 1, entry := nid 1, nodes := rootNodes,
    preconditions := [pid 2], postconditions := [] }

/-- Replace the root node with identity `n`. -/
def withNode (n : Nat) (f : Node → Node) (p : Procedure := root) : Procedure :=
  { p with nodes := p.nodes.map (fun x => if x.id = nid n then f x else x) }

structure Case where
  name : String
  procedure : Procedure
  catalog : List Procedure

def cases : List Case :=
  [ ⟨"base", root, [leaf]⟩,
    ⟨"leaf-alone", leaf, []⟩,
    ⟨"duplicate-node", { root with nodes := root.nodes ++ [node 3 .«end» .unit .artifact] }, [leaf]⟩,
    ⟨"entry-missing", { root with entry := nid 9 }, [leaf]⟩,
    ⟨"entry-input", { root with entry := nid 2 }, [leaf]⟩,
    ⟨"missing-end", withNode 2 (fun n => { n with failure := none })
        { root with nodes := root.nodes.filter (fun n => n.form != .«end») }, [leaf]⟩,
    ⟨"precondition-type", { root with preconditions := [pid 1] }, [leaf]⟩,
    ⟨"postcondition-type", { root with postconditions := [pid 2] }, [leaf]⟩,
    ⟨"self-dependency", withNode 2 (fun n => { n with dependencies := [⟨nid 2, .artifact⟩] }), [leaf]⟩,
    ⟨"forward-dependency", withNode 2 (fun n => { n with dependencies := [⟨nid 4, .artifact⟩] }), [leaf]⟩,
    ⟨"dependency-type", withNode 2 (fun n => { n with dependencies := [⟨nid 1, .evidence⟩] }), [leaf]⟩,
    ⟨"forward-prerequisite", withNode 4 (fun n => { n with prerequisites := [nid 5] }), [leaf]⟩,
    ⟨"node-ceiling", withNode 1 (fun n => { n with ceiling := [.read, .publish] }), [leaf]⟩,
    ⟨"node-floor", withNode 1 (fun n => { n with floor := [] }), [leaf]⟩,
    ⟨"backward-route", withNode 2 (fun n => { n with failure := some (nid 1) }), [leaf]⟩,
    ⟨"self-route", withNode 2 (fun n => { n with failure := some (nid 2) }), [leaf]⟩,
    ⟨"self-prerequisite", withNode 4 (fun n => { n with prerequisites := [nid 4] }), [leaf]⟩,
    ⟨"select-self", withNode 1 (fun n => { n with form := .select [nid 1] }), [leaf]⟩,
    ⟨"unknown-route", withNode 2 (fun n => { n with cancellation := some (nid 9) }), [leaf]⟩,
    ⟨"required-context", withNode 1 (fun n => { n with requiredContext := [oid 7] }), [leaf]⟩,
    ⟨"zero-cost", withNode 1 (fun n => { n with cost := 0 }), [leaf]⟩,
    ⟨"read-object", withNode 1 (fun n => { n with form := .read (oid 7) }), [leaf]⟩,
    ⟨"read-output", withNode 1 (fun n => { n with output := .evidence }), [leaf]⟩,
    ⟨"select-empty", withNode 1 (fun n => { n with form := .select [] }), [leaf]⟩,
    ⟨"select-duplicate", withNode 1 (fun n => { n with form := .select [nid 2, nid 2] }), [leaf]⟩,
    ⟨"select-backward", withNode 2 (fun n => { n with form := .select [nid 1] }), [leaf]⟩,
    ⟨"judge-unregistered", withNode 2 (fun n => { n with form := .judge (pid 9) }), [leaf]⟩,
    ⟨"check-type", withNode 2 (fun n => { n with form := .check (pid 2) }), [leaf]⟩,
    ⟨"call-unknown", withNode 3 (fun n => { n with form := .call { target with revision := ⟨12⟩ } }), [leaf]⟩,
    ⟨"call-rank", { root with rank := 0 }, [leaf]⟩,
    ⟨"call-delegation", withNode 3 (fun n => { n with ceiling := [.check] }), [leaf]⟩,
    ⟨"call-signature", withNode 3 (fun n => { n with form := .call { target with output := .evidence } }), [leaf]⟩,
    ⟨"effect-ceiling", withNode 5 (fun n => { n with ceiling := [.read] }), [leaf]⟩,
    ⟨"effect-no-resource", withNode 5 (fun n => { n with resources := [] }), [leaf]⟩,
    ⟨"effect-duplicate-resource", withNode 5 (fun n => { n with resources := [⟨1⟩, ⟨1⟩] }), [leaf]⟩,
    ⟨"join-empty", withNode 4 (fun n => { n with form := .join [] }), [leaf]⟩,
    ⟨"join-noncall", withNode 4 (fun n => { n with form := .join [nid 2] }), [leaf]⟩,
    ⟨"join-forward", withNode 2 (fun n => { n with form := .join [nid 3] }), [leaf]⟩,
    ⟨"end-output", withNode 6 (fun n => { n with output := .evidence }), [leaf]⟩ ]

def vt : ValueType → Json
  | .unit => "unit" | .artifact => "artifact" | .evidence => "evidence"
def cap : Capability → Json
  | .read => "read" | .check => "check" | .write => "write" | .publish => "publish"
def gate : Gate → Json
  | .test => "test" | .review => "review" | .compatibility => "compatibility"
def ids {k : Namespace} (xs : List (Id k)) : Json := toJson (xs.map (·.value))
def opt {k : Namespace} : Option (Id k) → Json
  | none => .null | some x => toJson x.value

def form : NodeForm → Json
  | .read o => Json.mkObj [("kind", "read"), ("object", toJson o.value)]
  | .select r => Json.mkObj [("kind", "select"), ("routes", ids r)]
  | .judge p => Json.mkObj [("kind", "judge"), ("predicate", toJson p.value)]
  | .check p => Json.mkObj [("kind", "check"), ("predicate", toJson p.value)]
  | .call t => Json.mkObj [("kind", "call"), ("procedure", toJson t.procedure.value),
      ("revision", toJson t.revision.value), ("input", vt t.input), ("output", vt t.output),
      ("delegation", Json.arr (t.delegation.map cap).toArray)]
  | .effect c => Json.mkObj [("kind", "effect"), ("capability", cap c)]
  | .join c => Json.mkObj [("kind", "join"), ("calls", ids c)]
  | .«end» => Json.mkObj [("kind", "end")]

def nodeJson (n : Node) : Json := Json.mkObj
  [("id", toJson n.id.value), ("form", form n.form), ("input", vt n.input),
   ("output", vt n.output),
   ("dependencies", Json.arr (n.dependencies.map (fun d =>
      Json.mkObj [("node", toJson d.node.value), ("type", vt d.type)])).toArray),
   ("prerequisites", ids n.prerequisites), ("preconditions", ids n.preconditions),
   ("postconditions", ids n.postconditions), ("context", toJson n.context.value),
   ("required_context", ids n.requiredContext), ("scope", ids n.scope),
   ("resources", ids n.resources), ("ceiling", Json.arr (n.ceiling.map cap).toArray),
   ("floor", Json.arr (n.floor.map gate).toArray), ("failure", opt n.failure),
   ("cancellation", opt n.cancellation), ("cost", toJson n.cost)]

def procedureJson (p : Procedure) : Json := Json.mkObj
  [("id", toJson p.id.value), ("revision", toJson p.revision.value),
   ("package", toJson p.package.value), ("closure", ids p.closure),
   ("input", vt p.inputType), ("output", vt p.outputType),
   ("ceiling", Json.arr (p.ceiling.map cap).toArray),
   ("floor", Json.arr (p.floor.map gate).toArray), ("rank", toJson p.rank),
   ("entry", toJson p.entry.value), ("nodes", Json.arr (p.nodes.map nodeJson).toArray),
   ("preconditions", ids p.preconditions), ("postconditions", ids p.postconditions)]

def signatureJson (s : PredicateSignature) : Json := Json.mkObj
  [("id", toJson s.id.value), ("input", vt s.input), ("output", vt s.output)]

def caseJson (c : Case) : Json := Json.mkObj
  [("name", toJson c.name), ("procedure", procedureJson c.procedure),
   ("catalog", Json.arr (c.catalog.map procedureJson).toArray),
   ("registry", Json.arr (registry.map signatureJson).toArray),
   ("lean_valid", toJson (validateProcedure (c.procedure :: c.catalog) registry c.procedure &&
      c.catalog.all (fun q => validateProcedure (c.procedure :: c.catalog) registry q)))]

end Kctx.Engineering.ProcedureCases

open Kctx.Engineering.ProcedureCases in
def main : IO UInt32 := do
  let doc := Lean.Json.mkObj
    [("schema", "writfold-procedure-lean-cases/1"),
     ("verdict", "lean_valid = validateProcedure for the root and every catalog procedure against (root :: catalog)"),
     ("cases", Lean.Json.arr (cases.map caseJson).toArray)]
  IO.println doc.pretty
  return 0
