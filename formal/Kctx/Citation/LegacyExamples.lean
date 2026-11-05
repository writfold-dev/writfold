import Kctx.Citation.Conservation
import Kctx.Citation.ResolutionExamples
import Kctx.Citation.HandleExamples
import Kctx.NonVacuity

set_option autoImplicit false
set_option maxRecDepth 10000
set_option maxHeartbeats 800000
namespace Kctx.Citation.LegacyExamples

open ResolutionExamples in
def reference : RepresentationRef :=
  { originalRef with
    repository := NonVacuity.observation.source
    revision := NonVacuity.observation.revision
    content := NonVacuity.observation.content }

def binding : Binding :=
  { ResolutionExamples.original with
    representation := reference
    release := NonVacuity.kb.snapshot }

def object : Representation := ⟨reference, ResolutionExamples.lineBytes⟩
def retained : Store := [object]
def access : Access := fun _ => true

def validation : Option BindingValidation := do
  let derivation ← NonVacuity.kb.derivations.head?
  let checked ← checkedDerivation NonVacuity.kb NonVacuity.context derivation
  let receipt : Receipt := {
    id := ⟨"citation-validation", by decide⟩
    claim := NonVacuity.claim.ref
    context := NonVacuity.context
    witness := derivation.witness
    dependencies := checked.dependencies
    passed := true
    expiresAt := 30 }
  some ⟨receipt, binding.context, [binding]⟩

def withValidation (check : BindingValidation → Bool) : Bool :=
  match validation with
  | none => false
  | some receipt => check receipt

def matchingReuse : Bool := withValidation (fun receipt =>
  citationReusable retained access NonVacuity.kb NonVacuity.context binding.context
    NonVacuity.claim.ref [binding] receipt)

def changedBindingRefuses : Bool := withValidation (fun receipt =>
  !citationReusable retained access NonVacuity.kb NonVacuity.context binding.context
    NonVacuity.claim.ref [{ binding with selector := ⟨0, 0, .byte⟩, expected := [] }] receipt)

def changedRoleRefusesReuse : Bool := withValidation (fun receipt =>
  !citationReusable retained access NonVacuity.kb NonVacuity.context binding.context
    NonVacuity.claim.ref [{ binding with role := ⟨2⟩ }] receipt)

def unrelatedSourceRefuses : Bool := withValidation (fun receipt =>
  !citationReusable ResolutionExamples.store access NonVacuity.kb NonVacuity.context
    binding.context NonVacuity.claim.ref [ResolutionExamples.original]
    { receipt with bindings := [ResolutionExamples.original] })

def foreignContextRefuses : Bool := withValidation (fun receipt =>
  !citationReusable retained access NonVacuity.kb NonVacuity.context
    ⟨"other-context", by decide⟩ NonVacuity.claim.ref [binding] receipt)

def revokedSourceRefuses : Bool := withValidation (fun receipt =>
  !citationReusable retained (fun _ => false) NonVacuity.kb NonVacuity.context
    binding.context NonVacuity.claim.ref [binding] receipt)

def missingSourceRefuses : Bool := withValidation (fun receipt =>
  !citationReusable [] access NonVacuity.kb NonVacuity.context
    binding.context NonVacuity.claim.ref [binding] receipt)

def changedFootprintRefuses : Bool := withValidation (fun receipt =>
  let changed := { NonVacuity.kb with observations :=
    [{ NonVacuity.observation with content := ⟨"changed-bytes", by decide⟩ }] }
  !citationReusable retained access changed NonVacuity.context
    binding.context NonVacuity.claim.ref [binding] receipt)

def unrelatedRetentionPreserves : Bool := withValidation (fun receipt =>
  citationReusable (retained ++ [ResolutionExamples.copiedObject]) access
    NonVacuity.kb NonVacuity.context binding.context NonVacuity.claim.ref [binding] receipt)

def expiredReceiptRefuses : Bool := withValidation (fun receipt =>
  !citationReusable retained access NonVacuity.kb { NonVacuity.context with now := 31 }
    binding.context NonVacuity.claim.ref [binding] receipt)

def initial : State := {
  engineering := { (default : Engineering.State) with
    knowledge := { (default : PortfolioState) with kb := NonVacuity.kb }, now := 20, alive := true }
  citations := { representations := retained, aliases := [], acquisitionHints := [] } }

def hinted : State := { initial with citations :=
  { initial.citations with acquisitionHints := [(reference, 999999)] } }

theorem hint_step : localStep access initial (.acquisitionHint reference 999999) = some hinted := by
  rfl

theorem hint_trace : CitationTrace access initial hinted :=
  CitationTrace.local CitationTrace.initial hint_step

def hintDoesNotRefresh : Bool :=
  let context := { NonVacuity.context with now := 31 }
  let before := assess initial.engineering.knowledge.kb context NonVacuity.claim.ref
  let after := assess hinted.engineering.knowledge.kb context NonVacuity.claim.ref
  decide (before = after ∧ after.freshness = .stale ∧ after.usable = false)

def inclusiveExpiry : Bool := decide
  ((assess NonVacuity.kb { NonVacuity.context with now := 30 } NonVacuity.claim.ref).freshness = .fresh)

def afterExpiry : Bool := decide
  ((assess NonVacuity.kb { NonVacuity.context with now := 31 } NonVacuity.claim.ref).freshness = .stale)

def missingPolicyUnknown : Bool := decide
  ((assess NonVacuity.kb { NonVacuity.context with policy :=
    { NonVacuity.context.policy with ttl := .missing } } NonVacuity.claim.ref).freshness = .unknown)

def futureObservationUnknown : Bool := decide
  (assessFresh .noExpiry 21 20 = .unknown)

def queryDoesNotAdmit : Bool :=
  decide (inspect hinted access (.resolve binding) =
    .resolution (.exact binding.expected reference)) &&
  !supported hinted.engineering.knowledge.kb NonVacuity.context NonVacuity.unsupported.ref .positive

def refusedReadPreservesKnowledge : Bool :=
  decide (inspect initial (fun _ => false) (.resolve binding) = .resolution .denied) &&
  usableFor initial.engineering.knowledge.kb NonVacuity.context NonVacuity.claim.ref

def hintPreservesJournal : Bool := decide
  (Engineering.legacyView hinted.engineering = Engineering.legacyView initial.engineering)

def unavailableCoverage : Bool := decide
  (Coverage.unavailable.complete = false ∧ Coverage.unavailable.total = none ∧
    (Coverage.measured 0 0).complete = true)

def allCases : List (String × String × String × Bool) :=
  [("matching_reuse", "CIT05", "positive", matchingReuse),
   ("changed_binding", "CIT05", "negative", changedBindingRefuses),
   ("changed_role", "CIT05", "negative", changedRoleRefusesReuse),
   ("unrelated_source", "CIT05", "negative", unrelatedSourceRefuses),
   ("foreign_context", "CIT05", "negative", foreignContextRefuses),
   ("revoked_source", "CIT05", "negative", revokedSourceRefuses),
   ("missing_source", "CIT05", "negative", missingSourceRefuses),
   ("changed_footprint", "CIT05", "negative", changedFootprintRefuses),
   ("unrelated_retention", "CIT05", "positive", unrelatedRetentionPreserves),
   ("expired_receipt", "CIT05", "negative", expiredReceiptRefuses),
   ("hint_does_not_refresh", "CIT06", "negative", hintDoesNotRefresh),
   ("inclusive_expiry", "CIT06", "positive", inclusiveExpiry),
   ("after_expiry", "CIT06", "negative", afterExpiry),
   ("missing_policy", "CIT06", "negative", missingPolicyUnknown),
   ("future_observation", "CIT06", "negative", futureObservationUnknown),
   ("query_does_not_admit", "CIT08", "negative", queryDoesNotAdmit),
   ("refused_read_preserves_knowledge", "CIT08", "positive", refusedReadPreservesKnowledge),
   ("hint_preserves_journal", "CIT08", "positive", hintPreservesJournal),
   ("unavailable_coverage", "CIT04", "negative", unavailableCoverage)]

theorem allCases_nonvacuous : allCases.all (fun row => row.2.2.2) = true := by decide

theorem hint_preserves_assessment (context : Context) (claim : Ref) :
    assess hinted.engineering.knowledge.kb context claim =
      assess initial.engineering.knowledge.kb context claim :=
  CIT06_assessment_unchanged hint_trace context claim

end Kctx.Citation.LegacyExamples
