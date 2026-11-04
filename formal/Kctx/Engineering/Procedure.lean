import Kctx.Engineering.Identity

set_option autoImplicit false
namespace Kctx.Engineering

structure PredicateSignature where
  id : PredicateId
  input : ValueType
  output : ValueType
  deriving DecidableEq, Repr, Inhabited

structure Dependency where
  node : NodeId
  type : ValueType
  deriving DecidableEq, Repr, Inhabited

structure CallTarget where
  procedure : Id .procedure
  revision : ProcedureRevision
  input : ValueType
  output : ValueType
  delegation : List Capability
  deriving DecidableEq, Repr, Inhabited

inductive NodeForm where
  | read (object : ObjectId)
  | select (routes : List NodeId)
  | judge (predicate : PredicateId)
  | call (target : CallTarget)
  | effect (capability : Capability)
  | check (predicate : PredicateId)
  | join (calls : List NodeId)
  | «end»
  deriving DecidableEq, Repr, Inhabited

structure Node where
  id : NodeId
  form : NodeForm
  input : ValueType
  output : ValueType
  dependencies : List Dependency
  /-- Every dependency and route is exact, never a mutable name lookup. -/
  prerequisites : List NodeId
  preconditions : List PredicateId
  postconditions : List PredicateId
  context : Id .subject
  requiredContext : List ObjectId
  scope : List (Id .dependency)
  resources : List (Id .resource)
  ceiling : List Capability
  floor : List Gate
  failure : Option NodeId
  cancellation : Option NodeId
  cost : Nat
  deriving DecidableEq, Repr, Inhabited

structure Procedure where
  id : Id .procedure
  revision : ProcedureRevision
  package : Id .package
  closure : List ObjectId
  inputType : ValueType
  outputType : ValueType
  ceiling : List Capability
  floor : List Gate
  /-- Calls strictly decrease this admitted rank: recursive executable calls are impossible. -/
  rank : Nat
  entry : NodeId
  nodes : List Node
  preconditions : List PredicateId
  postconditions : List PredicateId
  deriving DecidableEq, Repr, Inhabited

def findNode (p : Procedure) (id : NodeId) : Option Node :=
  p.nodes.find? (·.id == id)

/-- Structural judgments say what must exist in the graph, independently of the
validator's success bit. Ordering is a finite DAG certificate, not runtime fuel. -/
def EdgeTyped (p : Procedure) (n : Node) (d : Dependency) : Prop :=
  ∃ source ∈ p.nodes, source.id = d.node ∧ source.output = d.type ∧
    d.type = n.input ∧ source.id.value < n.id.value

instance (p : Procedure) (n : Node) (d : Dependency) : Decidable (EdgeTyped p n d) :=
  inferInstanceAs (Decidable (∃ source ∈ p.nodes, source.id = d.node ∧
    source.output = d.type ∧ d.type = n.input ∧ source.id.value < n.id.value))

def ForwardRoute (p : Procedure) (n : Node) (target : NodeId) : Prop :=
  ∃ next ∈ p.nodes, next.id = target ∧ n.id.value < next.id.value

instance (p : Procedure) (n : Node) (target : NodeId) :
    Decidable (ForwardRoute p n target) :=
  inferInstanceAs (Decidable (∃ next ∈ p.nodes, next.id = target ∧ n.id.value < next.id.value))

def PredicateTyped (registry : List PredicateSignature) (input output : ValueType)
    (id : PredicateId) : Prop := ⟨id, input, output⟩ ∈ registry

instance (registry : List PredicateSignature) (input output : ValueType) (id : PredicateId) :
    Decidable (PredicateTyped registry input output id) := inferInstanceAs
  (Decidable (PredicateSignature.mk id input output ∈ registry))

def FormTyped (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) (n : Node) : Prop :=
  match n.form with
  | .read object => object ∈ p.closure ∧ n.output = .artifact
  | .select routes => routes ≠ [] ∧ routes.Nodup ∧ ∀ route ∈ routes, ForwardRoute p n route
  | .judge pred | .check pred => PredicateTyped registry n.input n.output pred ∧ n.output = .evidence
  | .call target => ∃ callee ∈ catalog,
      callee.id = target.procedure ∧ callee.revision = target.revision ∧
      callee.inputType = target.input ∧ callee.outputType = target.output ∧
      n.input = target.input ∧ n.output = target.output ∧ callee.rank < p.rank ∧
      (∀ cap ∈ target.delegation, cap ∈ n.ceiling)
  | .effect cap => cap ∈ n.ceiling ∧ n.resources ≠ [] ∧ n.resources.Nodup
  | .join calls => calls ≠ [] ∧ calls.Nodup ∧ ∀ call ∈ calls,
      ∃ source ∈ p.nodes, source.id = call ∧ source.id.value < n.id.value ∧
        (match source.form with | .call _ => True | _ => False)
  | .«end» => n.output = p.outputType

instance (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) (n : Node) : Decidable (FormTyped catalog registry p n) := by
  letI callFormDecidable : ∀ form : NodeForm,
      Decidable (match form with | .call _ => True | _ => False) := by
    intro form
    cases form <;> infer_instance
  unfold FormTyped
  cases n.form <;> infer_instance

def NodeTyped (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) (n : Node) : Prop :=
  n.cost > 0 ∧ (∀ d ∈ n.dependencies, EdgeTyped p n d) ∧
  (∀ id ∈ n.prerequisites, ∃ source ∈ p.nodes, source.id = id ∧ source.id.value < n.id.value) ∧
  (∀ pred ∈ n.preconditions, PredicateTyped registry n.input .evidence pred) ∧
  (∀ pred ∈ n.postconditions, PredicateTyped registry n.output .evidence pred) ∧
  (∀ cap ∈ n.ceiling, cap ∈ p.ceiling) ∧ (∀ gate ∈ p.floor, gate ∈ n.floor) ∧
  (∀ route ∈ n.failure.toList ++ n.cancellation.toList, ForwardRoute p n route) ∧
  (∀ object ∈ n.requiredContext, object ∈ p.closure) ∧
  FormTyped catalog registry p n

instance (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) (n : Node) : Decidable (NodeTyped catalog registry p n) := by
  unfold NodeTyped
  infer_instance

def ProcedureTyped (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) : Prop :=
  (p.nodes.map Node.id).Nodup ∧
  (∃ entry ∈ p.nodes, entry.id = p.entry ∧ entry.input = p.inputType ∧
    entry.dependencies = [] ∧ entry.prerequisites = []) ∧
  (∃ last ∈ p.nodes, last.form = .«end») ∧
  (∀ pred ∈ p.preconditions, PredicateTyped registry p.inputType .evidence pred) ∧
  (∀ pred ∈ p.postconditions, PredicateTyped registry p.outputType .evidence pred) ∧
  ∀ n ∈ p.nodes, NodeTyped catalog registry p n

instance (catalog : List Procedure) (registry : List PredicateSignature) (p : Procedure) :
    Decidable (ProcedureTyped catalog registry p) := by
  unfold ProcedureTyped
  infer_instance

/-- `decide` traverses only the finite graph, registry and pinned catalog. -/
def validateProcedure (catalog : List Procedure) (registry : List PredicateSignature)
    (p : Procedure) : Bool := decide (ProcedureTyped catalog registry p)

theorem validateProcedure_sound {catalog : List Procedure} {registry : List PredicateSignature}
    {p : Procedure} (h : validateProcedure catalog registry p = true) :
    ProcedureTyped catalog registry p := of_decide_eq_true h

theorem validateProcedure_complete {catalog : List Procedure} {registry : List PredicateSignature}
    {p : Procedure} (h : ProcedureTyped catalog registry p) :
    validateProcedure catalog registry p = true := by
  exact decide_eq_true h

theorem dependency_strictly_precedes {p : Procedure} {n : Node} {d : Dependency}
    (h : EdgeTyped p n d) : d.node.value < n.id.value := by
  rcases h with ⟨source, _, same, _, _, order⟩
  simpa [same] using order

theorem no_self_dependency {p : Procedure} {n : Node} {d : Dependency}
    (h : EdgeTyped p n d) : d.node ≠ n.id := by
  intro same
  have order := dependency_strictly_precedes h
  simp [same] at order

end Kctx.Engineering
