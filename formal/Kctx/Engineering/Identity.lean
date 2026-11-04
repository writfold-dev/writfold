import Kctx.Ids

set_option autoImplicit false
namespace Kctx.Engineering

inductive Namespace where
  | realm | run | stream | invocation | effect | resource | procedure | package
  | subject | dependency | object | grant | policy | cut | candidate | claim
  | ledgerRecord | node | predicate | procedureRevision | providerRevision | bindingRevision
  | graph | graphCommit | schemaEpoch | query | compatibilityProfile
  | projectionContract | visibilityContract | capture
  deriving DecidableEq, Repr, Inhabited

/-- Mathematical names remain nominal across every semantic namespace. -/
structure Id (kind : Namespace) where
  value : Nat
  deriving DecidableEq, Repr, Inhabited

abbrev RunId := Id .run
abbrev ObjectId := Id .object
abbrev StreamId := Id .stream
abbrev LedgerRecordId := Id .ledgerRecord
abbrev NodeId := Id .node
abbrev PredicateId := Id .predicate
abbrev ProcedureRevision := Id .procedureRevision
abbrev ProviderRevision := Id .providerRevision
abbrev BindingRevision := Id .bindingRevision
abbrev ActivationId := Id .invocation

inductive Capability where
  | read | check | write | publish
  deriving DecidableEq, Repr, Inhabited
inductive Gate where
  | test | review | compatibility
  deriving DecidableEq, Repr, Inhabited
inductive ValueType where
  | unit | artifact | evidence
  deriving DecidableEq, Repr, Inhabited

end Kctx.Engineering
