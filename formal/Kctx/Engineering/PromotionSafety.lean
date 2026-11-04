import Kctx.Engineering.HistorySafety

set_option autoImplicit false
namespace Kctx.Engineering

/-- All live checks of the real publication reducer. Evaluation, review and
approval name the entire material, not merely the candidate's nominal ID. -/
def PublicationChecks (contracts : Contracts) (s : State) (a : Admission) : Prop :=
  s.alive = true ∧ a.expected = s.knowledge.kb.snapshot ∧
  a.policy = s.authority.policy ∧ a.correctionGeneration = s.authority.correctionGeneration ∧
  s.candidates.contains a.candidate = true ∧ a.candidate.evaluated = true ∧
  a.candidate.reviewed = true ∧ contracts.evaluations.contains a.candidate.toCandidateMaterial = true ∧
  contracts.reviews.contains a.candidate.toCandidateMaterial = true ∧
  learningAuthorized contracts s a.candidate = true ∧
  portfolioPlanValid s.knowledge a.candidate.plan = true ∧
  retained s (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials) = true ∧
  contracts.corpusBindings.contains (a.candidate.plan.candidate.snapshot, a.candidate.corpusPin) = true ∧
  a.candidate.corpusPin.realm = s.realm ∧ a.candidate.corpusPin.family = .corpus ∧
  s.retained.contains a.candidate.corpusPin.envelope = true ∧
  (match a.candidate.procedure with
   | none => true
   | some p => contracts.packages.contains p && retained s p.closure &&
       contracts.packageReleases.contains (a.candidate.plan.candidate.snapshot, p) &&
       validateProcedure s.catalog contracts.predicates p &&
       !s.authority.withdrawn.contains p.revision && subset s.authority.floor p.floor) = true

structure PublicationCertificate (contracts : Contracts) (s t : State)
    (stream : StreamId) (expected : Option Ledger.Head) (a : Admission) : Prop where
  admission : s.admission = some a
  streamBound : stream = a.candidate.stream
  predecessor : Ledger.currentHead s.ledger stream = expected
  prepared : preparedAdmissionRecorded a (Ledger.currentPrefix s.ledger stream) = true
  checked : PublicationChecks contracts s a
  rooted : Ledger.retainRelease s.ledger s.retained = some t.ledger
  applied : (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).outcome = .applied
  knowledge : t.knowledge = (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).state
  journal : t.knowledgeJournal = (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).journal
  nativePin : t.corpusPin = a.candidate.corpusPin
  cleared : t.admission = none
  acknowledgement : t.acknowledged = s.acknowledged

theorem publishAdmission_certificate {contracts : Contracts} {s t : State}
    (h : publishAdmission contracts s = some t) :
    ∃ a, s.admission = some a ∧ PublicationChecks contracts s a ∧
      (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).outcome = .applied ∧
      t.knowledge = (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).state ∧
      t.knowledgeJournal = (applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals).journal ∧
      t.corpusPin = a.candidate.corpusPin ∧ t.admission = none ∧ t.acknowledged = s.acknowledged := by
  unfold publishAdmission at h
  obtain ⟨a, selected, guarded⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨guard, applying⟩ := Option.ite_none_right_eq_some.mp guarded
  obtain ⟨applied, published⟩ := Option.ite_none_right_eq_some.mp applying
  cases published
  refine ⟨a, selected, ?_, ?_, rfl, rfl, rfl, rfl, rfl⟩
  · simpa only [PublicationChecks, Bool.and_eq_true, beq_iff_eq, and_assoc] using guard
  · simpa only [beq_iff_eq] using applied

/-- E01 consumes the exact applyPortfolio state AND journal fields here. E08
additionally consumes the pre-selection roots and exact historical preparation. -/
theorem publishSelected_certificate {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head}
    (h : publishSelected contracts s stream expected = some t) :
    ∃ a, PublicationCertificate contracts s t stream expected a := by
  cases ha : s.admission with
  | none => simp [publishSelected, ha] at h
  | some a =>
      simp only [publishSelected, ha, bind, Option.bind] at h
      split at h
      next guard =>
        have selected : stream = a.candidate.stream ∧ Ledger.currentHead s.ledger stream = expected ∧
            preparedAdmissionRecorded a (Ledger.currentPrefix s.ledger stream) = true := by
          simpa only [Bool.and_eq_true, beq_iff_eq, and_assoc] using guard
        cases hr : Ledger.retainRelease s.ledger s.retained with
        | none => simp [hr] at h
        | some ledger =>
            simp only [hr, bind, Option.bind] at h
            cases hb : domainBody contracts { s with ledger := ledger } .publishAdmission with
            | none => simp [domain, hb] at h
            | some result =>
                simp only [domain, hb, Option.map_some, Option.some.injEq] at h
                subst t
                change publishAdmission contracts { s with ledger := ledger } = some result at hb
                obtain ⟨found, foundAt, checks, applied, knowledge, journal, pin, clear, ack⟩ :=
                  publishAdmission_certificate hb
                have eq : found = a := by
                  simpa only [ha, Option.some.injEq] using foundAt.symm
                subst found
                refine ⟨a, ⟨ha, selected.1, selected.2.1, selected.2.2, checks, hr,
                  applied, ?_, ?_, pin, clear, ack⟩⟩
                · simpa only [frameDomain, ↓reduceIte] using knowledge
                · simpa only [frameDomain, ↓reduceIte] using journal
      next => contradiction

theorem publication_uses_legacy_approval {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (certificate : PublicationCertificate contracts s t stream expected a) :
    ∃ approval ∈ contracts.approvals, approval.plan = a.candidate.plan := by
  have approved := (portfolio_applied_details certificate.applied).2.1
  obtain ⟨approval, member, exactPlan⟩ := List.any_eq_true.mp approved
  exact ⟨approval, member, by simpa only [beq_iff_eq] using exactPlan⟩

theorem publication_current_engineering_approval {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (certificate : PublicationCertificate contracts s t stream expected a) :
    ∃ approval ∈ contracts.learningApprovals,
      approval.material = a.candidate.toCandidateMaterial ∧ approval.policy = s.authority.policy ∧
      approval.validFrom ≤ s.now ∧ s.now < approval.expires := by
  have authorized := certificate.checked.2.2.2.2.2.2.2.2.2.1
  obtain ⟨approval, member, valid⟩ := List.any_eq_true.mp authorized
  exact ⟨approval, member, by
    simpa only [learningApprovalValid, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq, and_assoc] using valid⟩

/-- A successful selection roots evidence and the exact corpus native envelope
before domain publication, without waiting for an acknowledgement event. -/
theorem publication_roots_before_selection {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (certificate : PublicationCertificate contracts s t stream expected a) :
    (∀ object ∈ s.retained, object ∈ s.ledger.material) ∧
    t.ledger.releaseRoots = s.retained ++ s.ledger.releaseRoots ∧
    t.ledger.selections = s.ledger.selections ∧ t.ledger.records = s.ledger.records := by
  have rooted := certificate.rooted
  unfold Ledger.retainRelease at rooted
  split at rooted
  next retained =>
    have same := (Option.some.inj rooted).symm
    exact ⟨retained, by simp only [same], by simp only [same], by simp only [same]⟩
  next => contradiction

/-- A witness identifies an exact historical suffix, record, captured premises
and successful reducer result. No present-policy replay is used. -/
theorem preparedAdmissionRecorded_witness (a : Admission)
    (entries : List (Ledger.Record Action HistoricalPremises))
    (h : preparedAdmissionRecorded a entries = true) :
    ∃ record historicalEntries after, record ∈ entries ∧
      record.payload = .prepareAdmission a.candidate.id ∧
      replay historicalEntries record = some after ∧ after.admission = some a ∧
      ∃ newer, entries = newer ++ record :: historicalEntries := by
  revert h
  induction entries with
  | nil => simp [preparedAdmissionRecorded]
  | cons record rest ih =>
      intro h
      simp only [preparedAdmissionRecorded, Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at h
      rcases h with ⟨payload, replayed⟩ | old
      · cases hr : replay rest record with
        | none => simp [hr] at replayed
        | some after =>
            have admission : after.admission = some a := by simpa [hr] using replayed
            exact ⟨record, rest, after, List.mem_cons_self, payload, hr, admission, [], rfl⟩
      · obtain ⟨oldRecord, historicalEntries, after, member, payload, replayed, admission, newer, suffix⟩ := ih old
        exact ⟨oldRecord, historicalEntries, after, List.mem_cons_of_mem _ member,
          payload, replayed, admission, record :: newer, by simp only [List.cons_append, suffix]⟩

theorem unprepared_publication_requires_refusal (contracts : Contracts) (s : State)
    (stream : StreamId) (expected : Option Ledger.Head)
    (missing : ∀ a, s.admission = some a →
      preparedAdmissionRecorded a (Ledger.currentPrefix s.ledger stream) = false) :
    publishSelected contracts s stream expected = none := by
  cases ha : s.admission with
  | none => simp [publishSelected, ha]
  | some a => simp [publishSelected, ha, missing a ha]

/-- A changed candidate cannot borrow the old candidate's evaluations. This
counterexample requirement names exact material, rather than an evaluated flag. -/
theorem changed_material_requires_refusal {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (admission : s.admission = some a)
    (missing : contracts.evaluations.contains a.candidate.toCandidateMaterial = false) :
    publishSelected contracts s stream expected ≠ some t := by
  intro h
  obtain ⟨found, certificate⟩ := publishSelected_certificate h
  have same : found = a := Option.some.inj (certificate.admission.symm.trans admission)
  subst found
  have evaluated : contracts.evaluations.contains a.candidate.toCandidateMaterial = true :=
    certificate.checked.2.2.2.2.2.2.2.1
  have impossible : false = true := missing.symm.trans evaluated
  cases impossible

end Kctx.Engineering
