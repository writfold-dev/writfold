import Kctx.Engineering.Procedure
import Kctx.Engineering.Ledger
import Kctx.Portfolio
import Kctx.DesignWorkflow

set_option autoImplicit false
namespace Kctx.Engineering

inductive EffectOutcome where
  | prepared | claimed | unknown | confirmed | failed | «partial» | cancelledBeforeDispatch
  deriving DecidableEq, Repr, Inhabited
inductive ExternalOutcome where
  | applied | rejected | «partial»
  deriving DecidableEq, Repr, Inhabited
inductive RunStatus where
  | active | cancelled | completed | retired
  deriving DecidableEq, Repr, Inhabited

structure Binding where
  realm : Id .realm
  subject : Id .subject
  dependencies : List (Id .dependency)
  revision : BindingRevision
  expectedProvider : ProviderRevision
  inputType : ValueType
  targetSource : SourceId
  targetRevision : TargetRevision
  deriving DecidableEq, Repr, Inhabited

structure TypedValue where
  type : ValueType
  objects : List ObjectId
  deriving DecidableEq, Repr, Inhabited

inductive Completion where
  | success | failed | blocked | cancelled | unknownEffect
  deriving DecidableEq, Repr, Inhabited

structure NodeResult where
  node : NodeId
  value : TypedValue
  outcome : Completion
  deriving DecidableEq, Repr, Inhabited

structure PendingCall where
  node : NodeId
  child : ActivationId
  target : CallTarget
  binding : Binding
  input : TypedValue
  deriving DecidableEq, Repr, Inhabited

structure ControlState where
  results : List NodeResult
  routes : List (NodeId × NodeId)
  pending : List PendingCall
  deriving DecidableEq, Repr, Inhabited

structure RootBudget where
  root : RunId
  limit : Nat
  spent : Nat
  deriving DecidableEq, Repr, Inhabited

/-- Membership is supplied by a qualified predicate evaluator, not a script exit
code. Every use rechecks this exact invocation, data, and current observation. -/
structure TypedEvidence where
  predicate : PredicateId
  invocation : ActivationId
  binding : Binding
  input : TypedValue
  output : TypedValue
  objects : List ObjectId
  policy : Id .policy
  correctionGeneration : Nat
  expires : Nat
  deriving DecidableEq, Repr, Inhabited

structure Grant where
  id : Id .grant
  run : RunId
  binding : Binding
  resources : List (Id .resource)
  capabilities : List Capability
  budget : Nat
  validFrom : Nat
  expires : Nat
  revoked : Bool
  deriving DecidableEq, Repr, Inhabited

/-- Qualified operator authorization of these exact grant bytes under one realm
and policy. Membership is supplied by the authenticated authority boundary, never
by an agent assertion; the original envelope must already be retained. -/
structure GrantAuthorization where
  realm : Id .realm
  policy : Id .policy
  grant : Grant
  envelope : ObjectId
  deriving DecidableEq, Repr, Inhabited

structure Authority where
  grants : List Grant
  policy : Id .policy
  floor : List Gate
  withdrawn : List ProcedureRevision
  correctionGeneration : Nat
  deriving DecidableEq, Repr, Inhabited

/-- Immutable call-time delegation, retained when current capabilities narrow or
gate floors grow on recovery. It is not itself current authority. -/
structure DelegationSnapshot where
  parent : ActivationId
  capabilities : List Capability
  floor : List Gate
  deriving DecidableEq, Repr, Inhabited

structure Run where
  id : RunId
  stream : Id .stream
  invocation : ActivationId
  incarnation : Nat
  parent : Option ActivationId
  delegation : Option DelegationSnapshot
  procedure : Procedure
  binding : Binding
  capabilities : List Capability
  floor : List Gate
  status : RunStatus
  design : Option DesignProposal
  correctionGeneration : Nat
  policy : Id .policy
  loaded : List ObjectId
  input : TypedValue
  control : ControlState
  deriving DecidableEq, Repr, Inhabited

structure Reservation where
  resource : Id .resource
  owner : RunId
  holder : ActivationId
  generation : Nat
  processGeneration : Nat
  children : Nat
  deriving DecidableEq, Repr, Inhabited

structure Intent where
  effect : Id .effect
  run : RunId
  invocation : ActivationId
  node : NodeId
  binding : Binding
  procedure : ProcedureRevision
  capability : Capability
  resources : List (Id .resource)
  expectedProvider : ProviderRevision
  closure : List ObjectId
  deriving DecidableEq, Repr, Inhabited

inductive RetryKind where
  | providerIdempotent | provenNoEffect
  deriving DecidableEq, Repr, Inhabited

structure RetryAuthorization where
  intent : Intent
  previousClaim : Option (Id .claim)
  incarnation : Nat
  policy : Id .policy
  validFrom : Nat
  expires : Nat
  kind : RetryKind
  deriving DecidableEq, Repr, Inhabited

structure Dispatch where
  id : Id .claim
  intent : Intent
  expectedHead : Option Ledger.Head
  record : LedgerRecordId
  generation : Nat
  incarnation : Nat
  grant : Id .grant
  policy : Id .policy
  retry : Option RetryKind
  retryAuthorization : Option RetryAuthorization
  issued : Bool
  deriving DecidableEq, Repr, Inhabited

structure Effect where
  intent : Intent
  generation : Nat
  outcome : EffectOutcome
  dispatch : Option Dispatch
  deriving DecidableEq, Repr, Inhabited

structure ExternalEvent where
  effect : Id .effect
  claim : Option (Id .claim)
  binding : Binding
  expected : ProviderRevision
  result : ExternalOutcome
  providerAfter : ProviderRevision
  deriving DecidableEq, Repr, Inhabited

inductive ReceiptRole where
  | gate | effectResult | procedureResult
  deriving DecidableEq, Repr, Inhabited

structure Receipt where
  role : ReceiptRole
  effect : Id .effect
  binding : Binding
  procedure : ProcedureRevision
  invocation : ActivationId
  node : NodeId
  gates : List Gate
  objects : List ObjectId
  observedAt : Nat
  expires : Nat
  policy : Id .policy
  correctionGeneration : Nat
  complete : Bool
  outputType : ValueType
  rootInput : TypedValue
  input : TypedValue
  output : TypedValue
  outcome : Completion
  deriving DecidableEq, Repr, Inhabited

structure Reuse where
  original : Receipt
  invocation : ActivationId
  deriving DecidableEq, Repr, Inhabited

structure ChildReturn where
  parent : ActivationId
  child : ActivationId
  node : NodeId
  binding : Binding
  procedure : ProcedureRevision
  input : TypedValue
  output : TypedValue
  receipt : Receipt
  deriving DecidableEq, Repr, Inhabited

structure CandidateMaterial where
  id : Id .candidate
  stream : StreamId
  plan : PortfolioPlan
  corpusPin : Ledger.NativePin
  evidence : List ObjectId
  procedure : Option Procedure
  rubric : ObjectId
  trials : List ObjectId
  deriving DecidableEq, Repr, Inhabited

structure Candidate extends CandidateMaterial where
  evaluated : Bool
  reviewed : Bool
  deriving DecidableEq, Repr, Inhabited

/-- Qualified engineering-domain approval over the exact delta. Legacy corpus
approvals remain separate and are still required for the underlying KB plan. -/
structure LearningApproval where
  material : CandidateMaterial
  policy : Id .policy
  validFrom : Nat
  expires : Nat
  deriving DecidableEq, Repr, Inhabited

structure Admission where
  candidate : Candidate
  expected : SnapshotId
  policy : Id .policy
  correctionGeneration : Nat
  deriving DecidableEq, Repr, Inhabited


structure PendingEffect where
  intent : Intent
  claim : Id .claim
  deriving DecidableEq, Repr, Inhabited

structure LogicalApplication where
  intent : Intent
  result : ExternalOutcome
  providerAfter : ProviderRevision
  deriving DecidableEq, Repr, Inhabited

structure World where
  provider : ProviderRevision
  producer : Nat
  consumer : Nat
  pending : List PendingEffect
  applications : List LogicalApplication
  events : List ExternalEvent
  replies : List ExternalEvent
  deriving DecidableEq, Repr, Inhabited

/-- Trusted inputs are explicit values supplied by a qualified boundary, never axioms.
The executable reducer still checks exact identities and legacy admission rules. -/
structure MigrationObservation where
  root : RunId
  oldBinding : Binding
  newBinding : Binding
  incarnation : Nat
  provider : ProviderRevision
  deriving DecidableEq, Repr, Inhabited

structure ResumeAuthorization where
  activation : ActivationId
  root : RunId
  procedure : ProcedureRevision
  binding : Binding
  input : TypedValue
  control : ControlState
  previousIncarnation : Nat
  incarnation : Nat
  policy : Id .policy
  correctionGeneration : Nat
  provider : ProviderRevision
  validFrom : Nat
  expires : Nat
  deriving DecidableEq, Repr, Inhabited

/-- This is an observed successful durable capture, not an attempted write or
source-text assertion. Acquisition time does not renew any enclosed evidence. -/
structure RetainedObservation where
  id : Id .capture
  realm : Id .realm
  objects : List ObjectId
  capturedAt : Nat
  deriving DecidableEq, Repr, Inhabited

structure Contracts where
  kernelFloor : List Gate
  packages : List Procedure
  predicates : List PredicateSignature
  typedEvidence : List TypedEvidence
  packageReleases : List (SnapshotId × Procedure)
  corpusBindings : List (SnapshotId × Ledger.NativePin)
  projections : List (Ledger.NativePin × List Ledger.Head × Nat)
  approvals : List VerifiedPortfolioApproval
  evaluations : List CandidateMaterial
  reviews : List CandidateMaterial
  learningApprovals : List LearningApproval
  observations : List ExternalEvent
  receipts : List Receipt
  quiescent : List (Id .resource × Nat × Nat)
  compatibility : List (ProviderRevision × Nat × Nat)
  effectAuthorizations : List Intent
  migrations : List MigrationObservation
  retryAuthorizations : List RetryAuthorization
  resumptions : List ResumeAuthorization
  materialObservations : List RetainedObservation
  grantAuthorizations : List GrantAuthorization
  deriving DecidableEq, Repr, Inhabited

inductive Action where
  | resolve (procedure : Procedure)
  | activate (run : Run) (budget : Nat)
  | loadResource (run : ActivationId) (object : ObjectId)
  | advanceNode (run : ActivationId) (node : NodeId) (value : TypedValue)
  | failNode (run : ActivationId) (node : NodeId) (receipt : Receipt)
  | selectRoute (run : ActivationId) (node route : NodeId) (value : TypedValue)
  | call (parent : ActivationId) (node : NodeId) (child : ActivationId)
      (callee : Procedure) (binding : Binding)
  | prepare (intent : Intent) (generation : Nat)
  | reserve (run : ActivationId) (resources : List (Id .resource)) (generation : Nat)
  | claimDispatch (effect : Id .effect) (claim : Id .claim)
      (expected : Option Ledger.Head) (record : LedgerRecordId)
  | retryClaim (effect : Id .effect) (claim : Id .claim)
      (expected : Option Ledger.Head) (record : LedgerRecordId)
  | issueEffect (effect : Id .effect)
  | observe (event : ExternalEvent)
  | observeMaterial (observation : RetainedObservation)
  | reconcile (event : ExternalEvent)
  | recordReceipt (receipt : Receipt)
  | reuse (run : ActivationId) (receipt : Receipt)
  | returnChild (parent child : ActivationId) (receipt : Receipt)
  | rebind (run : ActivationId) (procedure : Procedure) (binding : Binding) (invocation : ActivationId)
  | resume (run : ActivationId)
  | reopen (run : ActivationId) (command : DesignReopenCommand)
  | cancel (run : ActivationId)
  | quiesce (run : RunId)
  | propose (candidate : Candidate)
  | evaluate (candidate : Id .candidate)
  | review (candidate : Id .candidate)
  | prepareAdmission (candidate : Id .candidate)
  | publishAdmission
  | acknowledgeAdmission (candidate : Id .candidate)
  | project (pin : Ledger.NativePin) (heads : List Ledger.Head) (inventoryEpoch : Nat)
  | acquireReadCut (id : Id .cut)
  | releaseReadCut (id : Id .cut)
  | crash | recover
  | retire (run : ActivationId)
  deriving DecidableEq, Repr, Inhabited

/-- Sigma = K,P,J,O,A,Z,Q,L,W, with durable claims and volatile incarnation. -/
structure OperationalState where
  realm : Id .realm
  controlStream : StreamId
  knowledge : PortfolioState
  knowledgeJournal : List PortfolioJournalEntry
  catalog : List Procedure
  catalogRelease : SnapshotId
  corpusPin : Ledger.NativePin
  activityPin : Ledger.NativePin
  projectionContract : Id .projectionContract
  projectionEnvelope : ObjectId
  visibilityContract : Id .visibilityContract
  visibilityEnvelope : ObjectId
  runs : List Run
  budgets : List RootBudget
  ownership : List Reservation
  /-- Retained across release so a stale quiescence certificate cannot be reused. -/
  reservationGenerations : List (Id .resource × Nat)
  authority : Authority
  retained : List ObjectId
  captures : List RetainedObservation
  effects : List Effect
  claims : List Dispatch
  observations : List ExternalEvent
  receipts : List Receipt
  reused : List Reuse
  returned : List ChildReturn
  candidates : List Candidate
  admission : Option Admission
  acknowledged : List (Id .candidate)
  projectedHeads : List Ledger.Head
  projectionInventory : Nat
  world : World
  now : Nat
  incarnation : Nat
  alive : Bool
  deriving DecidableEq, Repr, Inhabited

structure HistoricalPremises where
  before : OperationalState
  contracts : Contracts
  deriving DecidableEq, Repr, Inhabited

structure ContextCut where
  id : Id .cut
  corpus : PortfolioState
  deriving DecidableEq, Repr, Inhabited

structure State extends OperationalState where
  ledger : Ledger.Store Action HistoricalPremises
  cuts : List ContextCut
  deriving DecidableEq, Repr, Inhabited


/-- Soundness is not implied by membership in Contracts. Each application must
supply this proposition relative to its external interpretation. -/
structure ExternalSound (c : Contracts) (truth : ExternalEvent → Prop)
    (compatible : ProviderRevision → Nat → Nat → Prop) : Prop where
  observations : ∀ e, e ∈ c.observations → truth e
  compatibility : ∀ rev p q, (rev, p, q) ∈ c.compatibility → compatible rev p q

/-- The interpreter checks binding and registered signatures; semantic validity
of an evaluator's evidence remains this explicit external assumption. -/
structure TypedEvidenceSound (c : Contracts) (truth : TypedEvidence → Prop) : Prop where
  predicates : ∀ e, e ∈ c.typedEvidence → truth e

end Kctx.Engineering
