import Kctx.Engineering.PromotionSafety
import Kctx.Engineering.HistorySafetyRetention
import Kctx.Engineering.OperationalInvariant

set_option autoImplicit false
namespace Kctx.Engineering

theorem linked_suffix_valid (stream : StreamId)
    (newer : List (Ledger.Record Action HistoricalPremises))
    (record : Ledger.Record Action HistoricalPremises)
    (historicalEntries : List (Ledger.Record Action HistoricalPremises))
    (linked : Ledger.Linked historicalValid stream (newer ++ record :: historicalEntries)) :
    historicalValid historicalEntries record = true := by
  revert linked
  induction newer with
  | nil =>
      intro linked
      exact (Ledger.linked_cons_fields historicalValid stream record historicalEntries linked).2.2.2.2
  | cons head rest ih =>
      intro linked
      exact ih (Ledger.linked_cons_fields historicalValid stream head (rest ++ record :: historicalEntries) linked).1

/-- The selected preparation witness is retained, valid and replayed under its
own exact captured before-state/contracts, including its old policy. -/
theorem publication_prepared_history {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (safe : HistorySafe s) (certificate : PublicationCertificate contracts s t stream expected a) :
    ∃ record historicalEntries after, record ∈ s.ledger.records ∧
      record.payload = .prepareAdmission a.candidate.id ∧
      historicalValid historicalEntries record = true ∧
      replay historicalEntries record = some after ∧ after.admission = some a ∧
      ∃ newer, Ledger.currentPrefix s.ledger stream = newer ++ record :: historicalEntries := by
  obtain ⟨record, historicalEntries, after, member, payload, replayed, admission, newer, suffix⟩ :=
    preparedAdmissionRecorded_witness a _ certificate.prepared
  obtain ⟨linked, retained⟩ := every_selected_stream_replays safe stream
  have valid : historicalValid historicalEntries record = true := by
    rw [suffix] at linked
    exact linked_suffix_valid stream newer record historicalEntries linked
  exact ⟨record, historicalEntries, after, (retained record member).1, payload,
    valid, replayed, admission, newer, suffix⟩

theorem publication_evidence_and_native_roots {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (certificate : PublicationCertificate contracts s t stream expected a) :
    ∀ object ∈ a.candidate.corpusPin.envelope ::
        (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials),
      object ∈ s.ledger.material ∧ object ∈ t.ledger.releaseRoots := by
  have material := certificate.checked.2.2.2.2.2.2.2.2.2.2.2.1
  have envelope := certificate.checked.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1
  have roots := publication_roots_before_selection certificate
  intro object member
  have retainedAt : object ∈ s.retained := by
    rcases List.mem_cons.mp member with rfl | member
    · simpa using envelope
    · exact List.contains_iff_mem.mp (List.all_eq_true.mp material object member)
  exact ⟨roots.1 object retainedAt, by
    rw [roots.2.1]
    exact List.mem_append_left _ retainedAt⟩

theorem publication_procedure_policy_floor {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission} {p : Procedure}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (certificate : PublicationCertificate contracts s t stream expected a)
    (procedure : a.candidate.procedure = some p) :
    contracts.packages.contains p = true ∧ retained s p.closure = true ∧
    validateProcedure s.catalog contracts.predicates p = true ∧
    s.authority.withdrawn.contains p.revision = false ∧
    (∀ gate ∈ s.authority.floor, gate ∈ p.floor) ∧
    (∀ gate ∈ contracts.kernelFloor, gate ∈ p.floor) := by
  have checked := certificate.checked.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2
  have procedureChecked : contracts.packages.contains p = true ∧ retained s p.closure = true ∧
      contracts.packageReleases.contains (a.candidate.plan.candidate.snapshot, p) = true ∧
      validateProcedure s.catalog contracts.predicates p = true ∧
      s.authority.withdrawn.contains p.revision = false ∧ subset s.authority.floor p.floor = true := by
    simpa only [procedure, Bool.and_eq_true, Bool.not_eq_true', and_assoc] using checked
  have floor : ∀ gate ∈ s.authority.floor, gate ∈ p.floor := by
    simpa only [subset, List.all_eq_true, List.contains_iff_mem] using procedureChecked.2.2.2.2.2
  exact ⟨procedureChecked.1, procedureChecked.2.1, procedureChecked.2.2.2.1,
    procedureChecked.2.2.2.2.1, floor, fun gate member => floor gate (wf.governing_floor gate member)⟩

/-- A trusted package alone cannot be promoted into a release that does not
explicitly bind that exact package to the candidate corpus snapshot. -/
theorem publication_target_release_package {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission} {p : Procedure}
    (certificate : PublicationCertificate contracts s t stream expected a)
    (procedure : a.candidate.procedure = some p) :
    contracts.packageReleases.contains (a.candidate.plan.candidate.snapshot, p) = true := by
  have checked := certificate.checked.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2
  simp only [procedure, Bool.and_eq_true, and_assoc] at checked
  exact checked.2.2.1

/-- Release roots outlive every acknowledgement, cut release, process failure,
policy change and subsequent publication in the actual transition relation. -/
theorem published_evidence_permanent {contracts : Contracts} {s selected later : State}
    {stream : StreamId} {expected : Option Ledger.Head} {a : Admission}
    (safe : HistorySafe s)
    (published : step contracts s (.publish stream expected) = some selected)
    (certificate : PublicationCertificate contracts s selected stream expected a)
    (reachable : Reachable contracts selected later) :
    ∀ object ∈ a.candidate.corpusPin.envelope ::
        (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials),
      object ∈ later.ledger.releaseRoots ∧ object ∈ later.ledger.material := by
  intro object member
  exact reachable_release_evidence (step_historySafe safe published) reachable
    (publication_evidence_and_native_roots certificate object member).2

/-- Loss of acknowledgement cannot undo the corpus CAS or its permanent roots.
This covers every explicit environment action, including loseReply. -/
theorem environment_cannot_undo_publication {contracts : Contracts} {s t : State}
    {action : Environment} (h : environment contracts s action = some t) :
    t.knowledge = s.knowledge ∧ t.knowledgeJournal = s.knowledgeJournal ∧
    t.ledger = s.ledger := by
  cases hb : environmentBody contracts s action with
  | none => simp [environment, hb] at h
  | some next =>
      simp only [environment, hb, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact ⟨rfl, rfl, rfl⟩

theorem crash_cannot_undo_publication (s : State) :
    (crash s).knowledge = s.knowledge ∧ (crash s).knowledgeJournal = s.knowledgeJournal ∧
    (crash s).ledger = s.ledger := ⟨rfl, rfl, rfl⟩

/-- Full publication theorem over the actual selected preparation and actual
corpus writer. OperationalWellFormed supplies the independently preserved kernel
floor; native durability, authenticated approvals and retention remain explicit
external-contract boundaries rather than cryptographic claims. -/
theorem E08_publication_safe {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head}
    (history : HistorySafe s) (wf : OperationalWellFormed contracts s.toOperationalState)
    (published : step contracts s (.publish stream expected) = some t) :
    ∃ a, PublicationCertificate contracts s t stream expected a ∧
      (∃ record historicalEntries after, record ∈ s.ledger.records ∧
        record.payload = .prepareAdmission a.candidate.id ∧ historicalValid historicalEntries record = true ∧
        replay historicalEntries record = some after ∧ after.admission = some a ∧
        ∃ newer, Ledger.currentPrefix s.ledger stream = newer ++ record :: historicalEntries) ∧
      (∃ approval ∈ contracts.approvals, approval.plan = a.candidate.plan) ∧
      (∀ object ∈ a.candidate.corpusPin.envelope ::
          (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials),
        object ∈ s.ledger.material ∧ object ∈ t.ledger.releaseRoots) ∧
      (∀ p, a.candidate.procedure = some p → ∀ gate ∈ contracts.kernelFloor, gate ∈ p.floor) := by
  obtain ⟨a, certificate⟩ := publishSelected_certificate published
  refine ⟨a, certificate, publication_prepared_history history certificate,
    publication_uses_legacy_approval certificate, publication_evidence_and_native_roots certificate, ?_⟩
  intro p procedure
  exact (publication_procedure_policy_floor wf certificate procedure).2.2.2.2.2

end Kctx.Engineering
