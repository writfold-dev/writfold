import Kctx.Engineering.PromotionSafetyEvidence
import Kctx.Engineering.OperationalProofsPreservation

set_option autoImplicit false
namespace Kctx.Engineering

/-- Full E08 over initialized reachable interpreter states. Neither historical
closure nor the governing policy floor is assumed at publication: both follow
from the actual init/Request/Environment trace. The two approval authorities
remain distinct, and engineering approval names the exact candidate material
under the current policy and current time window. Authenticated approvals and
native retention remain the explicit external-contract boundary. -/
theorem E08_publication_safe_reachable {contracts : Contracts} {input : InitialInputs}
    {initial s t : State} {stream : StreamId} {expected : Option Ledger.Head}
    (initialized : init contracts input = some initial)
    (reachable : Reachable contracts initial s)
    (published : step contracts s (.publish stream expected) = some t) :
    ∃ a, PublicationCertificate contracts s t stream expected a ∧
      (∃ record historicalEntries after, record ∈ s.ledger.records ∧
        record.payload = .prepareAdmission a.candidate.id ∧ historicalValid historicalEntries record = true ∧
        replay historicalEntries record = some after ∧ after.admission = some a ∧
        ∃ newer, Ledger.currentPrefix s.ledger stream = newer ++ record :: historicalEntries) ∧
      (∃ approval ∈ contracts.approvals, approval.plan = a.candidate.plan) ∧
      (∃ approval ∈ contracts.learningApprovals,
        approval.material = a.candidate.toCandidateMaterial ∧
        approval.policy = s.authority.policy ∧
        approval.validFrom ≤ s.now ∧ s.now < approval.expires) ∧
      (∀ object ∈ a.candidate.corpusPin.envelope ::
          (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials),
        object ∈ s.ledger.material ∧ object ∈ t.ledger.releaseRoots) ∧
      (∀ p, a.candidate.procedure = some p →
        ∀ gate ∈ contracts.kernelFloor, gate ∈ p.floor) ∧
      (∀ p, a.candidate.procedure = some p →
        contracts.packageReleases.contains (a.candidate.plan.candidate.snapshot, p) = true) ∧
      HistorySafe t := by
  have history := (E05_E09_history initialized reachable).1
  have representation := E02_representation_preserved initialized reachable
  obtain ⟨a, certificate, prepared, legacy, roots, floor⟩ :=
    E08_publication_safe history representation published
  exact ⟨a, certificate, prepared, legacy,
    publication_current_engineering_approval certificate, roots, floor,
    fun _ procedure => publication_target_release_package certificate procedure,
    step_historySafe history published⟩

end Kctx.Engineering
