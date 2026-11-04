import Kctx.Engineering.OperationalProofs
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Historical typing depends on the existence of pinned callees, not on the
current release head. Adding a package cannot invalidate an old call target;
the form equation exposes the pinned callee in the call case. -/
theorem records_formTyped_mono {old new : List Procedure}
    {registry : List PredicateSignature} {p : Procedure} {n : Node}
    (extension : ∀ q ∈ old, q ∈ new) (typed : FormTyped old registry p n) :
    FormTyped new registry p n := by
  unfold FormTyped at typed ⊢
  cases form : n.form <;> simp only [form] at typed ⊢ <;> try exact typed
  obtain ⟨callee, member, rest⟩ := typed
  exact ⟨callee, extension callee member, rest⟩

theorem records_procedureTyped_mono {old new : List Procedure}
    {registry : List PredicateSignature} {p : Procedure}
    (extension : ∀ q ∈ old, q ∈ new) (typed : ProcedureTyped old registry p) :
    ProcedureTyped new registry p := by
  rcases typed with ⟨unique, entry, last, pre, post, nodes⟩
  refine ⟨unique, entry, last, pre, post, ?_⟩
  intro node member
  rcases nodes node member with ⟨cost, deps, prereqs, pre, post, caps, floor, routes, context, form⟩
  exact ⟨cost, deps, prereqs, pre, post, caps, floor, routes, context,
    records_formTyped_mono extension form⟩

/-- The corpus and catalog component has no current-liveness requirements. -/
structure OperationalCatalog (contracts : Contracts) (s : OperationalState) : Prop where
  portfolio : portfolioStateValid s.knowledge = true
  governing_floor : ∀ gate ∈ contracts.kernelFloor, gate ∈ s.authority.floor
  corpus_binding : (s.knowledge.kb.snapshot, s.corpusPin) ∈ contracts.corpusBindings
  corpus_realm : s.corpusPin.realm = s.realm
  corpus_family : s.corpusPin.family = .corpus
  activity_realm : s.activityPin.realm = s.realm
  activity_family : s.activityPin.family = .activity
  envelopes_retained : ∀ object ∈ [s.corpusPin.envelope, s.activityPin.envelope,
    s.projectionEnvelope, s.visibilityEnvelope], object ∈ s.retained
  catalog_release : s.catalogRelease = s.knowledge.kb.snapshot
  catalog_authority : ∀ procedure ∈ s.catalog,
    procedure ∈ contracts.packages ∧
    (∃ release, (release, procedure) ∈ contracts.packageReleases) ∧
    ProcedureTyped s.catalog contracts.predicates procedure ∧
    (∀ object ∈ procedure.closure, object ∈ s.retained)

theorem OperationalWellFormed.catalogFields {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) : OperationalCatalog contracts s :=
  ⟨wf.portfolio, wf.governing_floor, wf.corpus_binding, wf.corpus_realm,
    wf.corpus_family, wf.activity_realm, wf.activity_family, wf.envelopes_retained,
    wf.catalog_release, wf.catalog_authority⟩

/-- Expose each actual action reducer, then extract its successful Option path.
The shared extractor resolves charged intermediate states before any record
frame is projected; it never unfolds a debit into an opaque state equation. -/
macro "records_split_domain" h:ident action:ident : tactic => `(tactic| (
  cases $action:ident <;>
    simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
      selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, observeEffect,
      observeMaterial, reconcile, reuse, returnChild, rebind, resume, reopen, cancel, quiesce,
      propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
      retire] at $h:ident
  all_goals operational_success $h:ident
  all_goals dsimp only [putRun, putEffect, recordResult, recordFailure, frameDomain]
))

theorem records_retained_extension (old added : List ObjectId) :
    ∀ object ∈ old, object ∈ added.filter (fun object => !old.contains object) ++ old := by
  intro object member
  exact List.mem_append_right _ member

theorem records_retained_added (old added : List ObjectId) :
    ∀ object ∈ added, object ∈ added.filter (fun object => !old.contains object) ++ old := by
  intro object member
  by_cases present : object ∈ old
  · exact List.mem_append_right _ present
  · apply List.mem_append_left
    simp [member, present]

theorem records_catalog_cons {contracts : Contracts} {s : OperationalState} {p : Procedure}
    (before : OperationalCatalog contracts s)
    (package : p ∈ contracts.packages)
    (release : (s.catalogRelease, p) ∈ contracts.packageReleases)
    (typed : validateProcedure s.catalog contracts.predicates p = true)
    (closure : ∀ object ∈ p.closure, object ∈ s.retained) :
    ∀ procedure ∈ p :: s.catalog,
      procedure ∈ contracts.packages ∧
      (∃ release, (release, procedure) ∈ contracts.packageReleases) ∧
      ProcedureTyped (p :: s.catalog) contracts.predicates procedure ∧
      (∀ object ∈ procedure.closure, object ∈ s.retained) := by
  intro procedure member
  have extension : ∀ q ∈ s.catalog, q ∈ p :: s.catalog := fun q h => List.mem_cons_of_mem p h
  rcases List.mem_cons.mp member with rfl | member
  · exact ⟨package, ⟨s.catalogRelease, release⟩,
      records_procedureTyped_mono extension (validateProcedure_sound typed), closure⟩
  · obtain ⟨package, release, typed, closure⟩ := before.catalog_authority procedure member
    exact ⟨package, release, records_procedureTyped_mono extension typed, closure⟩

theorem records_portfolio_snapshot {s : PortfolioState} {journal : List PortfolioJournalEntry}
    {plan : PortfolioPlan} {trusted : List VerifiedPortfolioApproval}
    (h : (applyPortfolio s journal plan trusted).outcome = .applied) :
    (applyPortfolio s journal plan trusted).state.kb.snapshot = plan.candidate.snapshot := by
  rw [(portfolio_applied_details h).2.2.2]
  rfl

/-- Retention only grows; neither envelope pins nor historical typing require
the newly retained objects to have been present in the old state. -/
theorem records_envelopes_mono {contracts : Contracts} {s : OperationalState}
    {objects : List ObjectId} (before : OperationalCatalog contracts s)
    (extension : ∀ object ∈ s.retained, object ∈ objects) :
    ∀ object ∈ [s.corpusPin.envelope, s.activityPin.envelope,
      s.projectionEnvelope, s.visibilityEnvelope], object ∈ objects := by
  intro object member
  exact extension object (before.envelopes_retained object member)

theorem records_catalog_retained_mono {contracts : Contracts} {s : OperationalState}
    {objects : List ObjectId} (before : OperationalCatalog contracts s)
    (extension : ∀ object ∈ s.retained, object ∈ objects) :
    ∀ procedure ∈ s.catalog,
      procedure ∈ contracts.packages ∧
      (∃ release, (release, procedure) ∈ contracts.packageReleases) ∧
      ProcedureTyped s.catalog contracts.predicates procedure ∧
      (∀ object ∈ procedure.closure, object ∈ objects) := by
  intro procedure member
  obtain ⟨package, release, typed, closure⟩ := before.catalog_authority procedure member
  exact ⟨package, release, typed, fun object member => extension object (closure object member)⟩

/-- Publication replaces only the corpus envelope. Its new envelope is checked
by the reducer; the other three retain their old witnesses. -/
theorem records_published_envelopes {contracts : Contracts} {s : OperationalState}
    {envelope : ObjectId} (before : OperationalCatalog contracts s)
    (retained : envelope ∈ s.retained) :
    ∀ object ∈ [envelope, s.activityPin.envelope,
      s.projectionEnvelope, s.visibilityEnvelope], object ∈ s.retained := by
  intro object member
  rcases List.mem_cons.mp member with rfl | member
  · exact retained
  · exact before.envelopes_retained object (List.mem_cons_of_mem _ member)

/-- Corpus publication uses the existing signed portfolio reducer. Resolution
extends typing monotonically; historical release witnesses remain immutable. -/
theorem operationalCatalog_domainBody {contracts : Contracts} {s t : State}
    {action : Action} (before : OperationalCatalog contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    OperationalCatalog contracts (frameDomain s action t).toOperationalState := by
  records_split_domain h action
  all_goals constructor
  -- Project the exact frames first. In particular, publication does not add
  -- its optional procedure to the catalog, so historical catalog validity is
  -- precisely the old witness, not a newly assumed publication conclusion.
  all_goals first
    | exact before.portfolio
    | exact before.governing_floor
    | exact before.corpus_binding
    | exact before.corpus_realm
    | exact before.corpus_family
    | exact before.activity_realm
    | exact before.activity_family
    | exact before.envelopes_retained
    | exact before.catalog_release
    | exact before.catalog_authority
    | rfl
    | skip
  -- Only resolve, retained-material observation and publication have any
  -- remaining fields. Normalize their Boolean checks, without expanding a
  -- State record or asking proof search to reconstruct its representation.
  all_goals simp_all only [Bool.and_eq_true, Bool.not_eq_true, Bool.not_eq_false,
    beq_iff_eq, retained, subset, List.all_eq_true, List.contains_iff_mem, ite_true, ite_false]
  all_goals first
    | (apply records_catalog_cons before <;> simp_all only [])
    | exact records_envelopes_mono before (records_retained_extension _ _)
    | exact records_catalog_retained_mono before (records_retained_extension _ _)
    | exact (portfolio_applied_preserves (by assumption)).1
    | (rw [records_portfolio_snapshot (by assumption)]
       simp_all only [])
    | (apply records_published_envelopes before
       simp_all only [])
  case resolve.intro.refl.catalog_authority.release =>
    rename_i procedure guard
    rw [← before.catalog_release]
    exact guard.1.1.1.1.1.2
  case resolve.intro.refl.catalog_authority.closure =>
    intro object member
    trivial

theorem operationalWellFormed_domain_catalog {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalCatalog contracts t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    exact operationalCatalog_domainBody wf.catalogFields body

end Kctx.Engineering
