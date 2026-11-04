import Kctx.Engineering.OperationalProofsRecordsCatalog

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Evidence qualification is immutable; evaluation and review do not replace
candidate material or create a fresh observation time. -/
def CandidateRepresentation (contracts : Contracts) (retained : List ObjectId)
    (candidate : Candidate) : Prop :=
  (∀ object ∈ candidate.evidence, object ∈ retained) ∧
  (candidate.evaluated = true → candidate.toCandidateMaterial ∈ contracts.evaluations) ∧
  (candidate.reviewed = true → candidate.evaluated = true ∧
    candidate.toCandidateMaterial ∈ contracts.reviews)

structure OperationalMaterials (contracts : Contracts) (s : OperationalState) : Prop where
  candidates : ∀ candidate ∈ s.candidates, CandidateRepresentation contracts s.retained candidate
  captures : ∀ capture ∈ s.captures,
    capture ∈ contracts.materialObservations ∧ capture.realm = s.realm ∧
    capture.capturedAt ≤ s.now ∧ ∀ object ∈ capture.objects, object ∈ s.retained

theorem OperationalWellFormed.materialFields {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) : OperationalMaterials contracts s :=
  ⟨wf.candidates, wf.captures⟩

theorem records_candidate_evaluate {contracts : Contracts} {s : OperationalState}
    {id : Id .candidate} {selected : Candidate}
    (wf : OperationalWellFormed contracts s)
    (found : s.candidates.find? (·.id == id) = some selected)
    (qualified : selected.toCandidateMaterial ∈ contracts.evaluations) :
    ∀ candidate ∈ s.candidates.map (fun c => if c.id == id then
      { c with evaluated := true } else c), CandidateRepresentation contracts s.retained candidate := by
  have selectedMember := List.mem_of_find?_eq_some found
  have selectedId : selected.id = id := by simpa using List.find?_some found
  intro candidate member
  obtain ⟨old, oldMember, rfl⟩ := List.mem_map.mp member
  split
  next same =>
    have oldId : old.id = id := by simpa using same
    have sameId : old.id = selected.id := oldId.trans selectedId.symm
    have equal := member_eq_of_unique_key wf.candidates_unique oldMember selectedMember sameId
    subst old
    obtain ⟨evidence, _, review⟩ := wf.candidates selected selectedMember
    exact ⟨evidence, fun _ => qualified, fun reviewed => ⟨rfl, (review reviewed).2⟩⟩
  next => exact wf.candidates old oldMember

theorem records_candidate_review {contracts : Contracts} {s : OperationalState}
    {id : Id .candidate} {selected : Candidate}
    (wf : OperationalWellFormed contracts s)
    (found : s.candidates.find? (·.id == id) = some selected)
    (evaluated : selected.evaluated = true)
    (qualified : selected.toCandidateMaterial ∈ contracts.reviews) :
    ∀ candidate ∈ s.candidates.map (fun c => if c.id == id then
      { c with reviewed := true } else c), CandidateRepresentation contracts s.retained candidate := by
  have selectedMember := List.mem_of_find?_eq_some found
  have selectedId : selected.id = id := by simpa using List.find?_some found
  intro candidate member
  obtain ⟨old, oldMember, rfl⟩ := List.mem_map.mp member
  split
  next same =>
    have oldId : old.id = id := by simpa using same
    have sameId : old.id = selected.id := oldId.trans selectedId.symm
    have equal := member_eq_of_unique_key wf.candidates_unique oldMember selectedMember sameId
    subst old
    obtain ⟨evidence, evaluation, _⟩ := wf.candidates selected selectedMember
    exact ⟨evidence, evaluation, fun _ => ⟨evaluated, qualified⟩⟩
  next => exact wf.candidates old oldMember

theorem records_candidate_retention {contracts : Contracts} {old new : List ObjectId}
    {candidate : Candidate} (extension : ∀ object ∈ old, object ∈ new)
    (before : CandidateRepresentation contracts old candidate) :
    CandidateRepresentation contracts new candidate :=
  ⟨fun object member => extension object (before.1 object member), before.2⟩

/-- Proposal adds only the checked, still unevaluated material. -/
theorem records_candidate_propose {contracts : Contracts} {s : OperationalState}
    {candidate : Candidate} (before : OperationalMaterials contracts s)
    (evidence : ∀ object ∈ candidate.evidence, object ∈ s.retained)
    (unevaluated : candidate.evaluated = false)
    (unreviewed : candidate.reviewed = false) :
    ∀ value ∈ candidate :: s.candidates,
      CandidateRepresentation contracts s.retained value := by
  intro value member
  rcases List.mem_cons.mp member with equal | oldMember
  · subst value
    exact ⟨evidence, by simp [unevaluated], by simp [unreviewed]⟩
  · exact before.candidates value oldMember

theorem records_candidates_retained {contracts : Contracts} {s : OperationalState}
    (before : OperationalMaterials contracts s) (added : List ObjectId) :
    ∀ candidate ∈ s.candidates, CandidateRepresentation contracts
      (added.filter (fun object => !s.retained.contains object) ++ s.retained) candidate := by
  intro candidate member
  exact records_candidate_retention (records_retained_extension _ _)
    (before.candidates candidate member)

theorem records_captures_retained {contracts : Contracts} {s : OperationalState}
    (before : OperationalMaterials contracts s) (added : List ObjectId) :
    ∀ capture ∈ s.captures,
      capture ∈ contracts.materialObservations ∧ capture.realm = s.realm ∧
      capture.capturedAt ≤ s.now ∧ ∀ object ∈ capture.objects,
        object ∈ added.filter (fun object => !s.retained.contains object) ++ s.retained := by
  intro capture member
  obtain ⟨qualified, realm, time, objects⟩ := before.captures capture member
  exact ⟨qualified, realm, time,
    fun object objectMember => records_retained_extension _ _ object (objects object objectMember)⟩

/-- A capture keeps its checked timestamp and qualifies precisely its own objects. -/
theorem records_capture_observe {contracts : Contracts} {s : OperationalState}
    {observation : RetainedObservation} (before : OperationalMaterials contracts s)
    (qualified : observation ∈ contracts.materialObservations)
    (realm : observation.realm = s.realm) (time : observation.capturedAt ≤ s.now) :
    ∀ capture ∈ observation :: s.captures,
      capture ∈ contracts.materialObservations ∧ capture.realm = s.realm ∧
      capture.capturedAt ≤ s.now ∧ ∀ object ∈ capture.objects,
        object ∈ observation.objects.filter (fun object => !s.retained.contains object) ++
          s.retained := by
  intro capture member
  rcases List.mem_cons.mp member with equal | oldMember
  · subst capture
    exact ⟨qualified, realm, time, records_retained_added _ _⟩
  · exact records_captures_retained before observation.objects capture oldMember

/-- The actual observation guard supplies qualification, realm and capture time;
retention grows by the observed objects, never by manufacturing evidence. -/
theorem operationalMaterials_domainBody {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    OperationalMaterials contracts (frameDomain s action t).toOperationalState := by
  have before := wf.materialFields
  cases action with
  | propose candidate =>
      simp only [domainBody, propose] at h
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp h
      simp only [Bool.and_eq_true, Bool.not_eq_true'] at checked
      obtain ⟨⟨⟨⟨⟨⟨⟨_, unevaluated⟩, unreviewed⟩, evidence⟩, _⟩, _⟩, _⟩, _⟩ := checked
      have retainedEvidence : ∀ object ∈ candidate.evidence, object ∈ s.retained := by
        simpa only [retained, subset, List.all_eq_true, List.contains_iff_mem] using evidence
      cases result
      exact ⟨records_candidate_propose before retainedEvidence unevaluated unreviewed,
        before.captures⟩
  | evaluate id =>
      simp only [domainBody, evaluate] at h
      obtain ⟨selected, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp continuation
      simp only [Bool.and_eq_true, List.contains_iff_mem] at checked
      cases result
      exact ⟨records_candidate_evaluate wf found checked.2, before.captures⟩
  | review id =>
      simp only [domainBody, review] at h
      obtain ⟨selected, found, continuation⟩ := Option.bind_eq_some_iff.mp h
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp continuation
      simp only [Bool.and_eq_true, List.contains_iff_mem] at checked
      cases result
      exact ⟨records_candidate_review wf found checked.1 checked.2, before.captures⟩
  | observeMaterial observation =>
      simp only [domainBody, observeMaterial] at h
      obtain ⟨checked, result⟩ := Option.ite_none_right_eq_some.mp h
      simp only [Bool.and_eq_true, List.contains_iff_mem, beq_iff_eq, decide_eq_true_eq] at checked
      obtain ⟨⟨⟨⟨qualified, realm⟩, time⟩, _⟩, _⟩ := checked
      cases result
      exact ⟨records_candidates_retained before observation.objects,
        records_capture_observe before qualified realm time⟩
  | _ =>
      simp only [domainBody, resolve, activate, loadResource, advanceNode, failNode,
        selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, observeEffect,
        reconcile, reuse, returnChild, rebind, resume, reopen, cancel, quiesce,
        prepareAdmission, publishAdmission, acknowledgeAdmission, retire] at h
      operational_success h
      all_goals exact ⟨before.candidates, before.captures⟩

theorem operationalWellFormed_domain_materials {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalMaterials contracts t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    exact operationalMaterials_domainBody wf body

end Kctx.Engineering
