import Kctx.Engineering.OperationalProofsRecordsMaterials
import Kctx.Engineering.OperationalProofsRecordsLinks

set_option autoImplicit false
namespace Kctx.Engineering

/-- All non-identity, non-run fields of the operational invariant. This package
contains conclusions of the actual reducer proofs, not additional admission
premises. Contracts remain fixed while the execution history grows. -/
structure OperationalRecords (contracts : Contracts) (s : OperationalState) : Prop
    extends OperationalCatalog contracts s, OperationalMaterials contracts s,
      OperationalLinks contracts s

theorem OperationalWellFormed.recordFields {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) : OperationalRecords contracts s :=
  ⟨wf.catalogFields, wf.materialFields, wf.linkFields⟩

/-- Domain preservation for portfolio/corpus/catalog authority, qualified
materials, effects, ownership, receipts and exact child-return subjects. The
separate run and identity families supply the remaining invariant fields. -/
theorem operationalWellFormed_domain_records {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : domain contracts s action = some t) : OperationalRecords contracts t.toOperationalState :=
  ⟨operationalWellFormed_domain_catalog wf h,
    operationalWellFormed_domain_materials wf h,
    operationalWellFormed_domain_links wf h⟩

end Kctx.Engineering
