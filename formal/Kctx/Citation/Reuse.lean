import Kctx.Citation.Resolution
import Kctx.Footprint
import Kctx.Theorems2

set_option autoImplicit false
namespace Kctx.Citation

/-- A citation-bound validation keeps the original legacy receipt. It cannot
invent a new observation time, witness, approval, or validity interval. -/
structure BindingValidation where
  legacy : Receipt
  contextId : ContextId
  bindings : List Binding
  deriving DecidableEq, Repr, Inhabited

def resolvesExactly (store : Store) (access : Access) (binding : Binding) : Bool :=
  match resolve store access binding with
  | .exact _ _ => true
  | _ => false

def bindingsExact (store : Store) (access : Access) (bindings : List Binding) : Bool :=
  bindings.all (resolvesExactly store access)

/-- Association with a validated witness is exact: a readable but unrelated
source cannot be attached to somebody else's successful validation receipt. -/
def bindingInFootprint (dependencies : List Dependency) (binding : Binding) : Bool :=
  dependencies.any (fun dependency => match dependency with
    | .observation observation => decide
        (observation.source = binding.representation.repository ∧
         observation.revision = binding.representation.revision ∧
         observation.content = binding.representation.content)
    | _ => false)

def bindingsInContext (contextId : ContextId) (validation : BindingValidation)
    (bindings : List Binding) : Bool :=
  bindings.all (fun binding => decide (binding.context = contextId) &&
    bindingInFootprint validation.legacy.dependencies binding)

def citationReusable (store : Store) (access : Access) (kb : KB) (context : Context)
    (contextId : ContextId) (claim : Ref) (current : List Binding)
    (validation : BindingValidation) : Bool :=
  decide (contextId = validation.contextId) && decide (current = validation.bindings) &&
    bindingsInContext contextId validation current && bindingsExact store access current &&
    reusable kb context claim validation.legacy


end Kctx.Citation
