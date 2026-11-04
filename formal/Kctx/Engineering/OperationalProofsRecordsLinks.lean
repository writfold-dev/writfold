import Kctx.Engineering.OperationalProofsRecordsEffectLinks
import Kctx.Engineering.OperationalProofsRecordsEvidenceLinks

set_option autoImplicit false
namespace Kctx.Engineering

/-- Each record family follows its actual writers. There is no recursive search
that can invent empty list prefixes or repeatedly reapply a transport lemma. -/
theorem operationalLinks_domainBody {contracts : Contracts} {s t : State} {action : Action}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domainBody contracts s action = some t) :
    OperationalLinks contracts (frameDomain s action t).toOperationalState :=
  ⟨records_domainBody_effects wf h, records_domainBody_ownership wf h,
    records_domainBody_receipts wf h, records_domainBody_returned wf h⟩

theorem operationalWellFormed_domain_links {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalLinks contracts t.toOperationalState := by
  unfold domain at h
  cases body : domainBody contracts s action with
  | none => simp [body] at h
  | some next =>
    simp only [body, Option.map_some, Option.some.injEq] at h
    subst t
    exact operationalLinks_domainBody (t := next) wf body

end Kctx.Engineering
