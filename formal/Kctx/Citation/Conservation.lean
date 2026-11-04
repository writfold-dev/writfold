import Kctx.Citation.State
import Kctx.Citation.ReuseProofs

set_option autoImplicit false
namespace Kctx.Citation

theorem localStep_engineering_frame {access : Access} {before after : State}
    {command : LocalCommand} (accepted : localStep access before command = some after) :
    after.engineering = before.engineering := by
  cases changed : localUpdate access before.citations command with
  | none => simp [localStep, changed] at accepted
  | some data =>
      simp only [localStep, changed, Option.map_some, Option.some.injEq] at accepted
      subst after
      rfl

/-- Erasure threads the SAME engineering state and history through arbitrary
citation updates, successful/refused reads, environment events and core steps. -/
theorem CIT08_erasure {contracts : Engineering.Contracts} {access : Access}
    {initial final : State} (trace : Trace contracts access initial final) :
    Engineering.Reachable contracts initial.engineering final.engineering := by
  induction trace with
  | initial => exact Engineering.Reachable.initial
  | next _ transition ih =>
      cases transition with
      | «local» accepted => rw [localStep_engineering_frame accepted]; exact ih
      | inspection => exact ih
      | engineering accepted => exact Engineering.Reachable.next ih accepted
      | refused => exact ih

/-- Publication remains exactly the original admitted portfolio transition,
including its operation journal. Citation operations add no knowledge rule. -/
theorem CIT08_legacy_conservation {contracts : Engineering.Contracts} {access : Access}
    {initial final : State} (trace : Trace contracts access initial final) :
    Engineering.LegacyTrace contracts (Engineering.legacyView initial.engineering)
      (Engineering.legacyView final.engineering) :=
  Engineering.E01_conservative_extension (CIT08_erasure trace)

theorem citationTrace_frame {access : Access} {initial final : State}
    (trace : CitationTrace access initial final) : final.engineering = initial.engineering := by
  induction trace with
  | initial => rfl
  | «local» _ accepted ih => exact (localStep_engineering_frame accepted).trans ih
  | inspection _ _ _ _ ih => exact ih

/-- The theorem quantifies over actual closed citation operation traces, not
only two states assumed to have equal knowledge. -/
theorem CIT08_no_new_derivation {access : Access} {initial final : State}
    (trace : CitationTrace access initial final) (context : Context)
    (witness : Witness) (checked : Checked) :
    Derives final.engineering.knowledge.kb context witness checked ↔
      Derives initial.engineering.knowledge.kb context witness checked := by
  rw [citationTrace_frame trace]

theorem CIT06_assessment_unchanged {access : Access} {initial final : State}
    (trace : CitationTrace access initial final) (context : Context) (claim : Ref) :
    assess final.engineering.knowledge.kb context claim =
      assess initial.engineering.knowledge.kb context claim := by
  rw [citationTrace_frame trace]

theorem CIT06_observation_times_unchanged {access : Access} {initial final : State}
    (trace : CitationTrace access initial final) :
    final.engineering.knowledge.kb.observations = initial.engineering.knowledge.kb.observations := by
  rw [citationTrace_frame trace]

theorem CIT05_legacy_validation_unchanged {access : Access} {initial final : State}
    (trace : CitationTrace access initial final) (context : Context) (claim : Ref)
    (receipt : Receipt) :
    reusable final.engineering.knowledge.kb context claim receipt =
      reusable initial.engineering.knowledge.kb context claim receipt := by
  rw [citationTrace_frame trace]

end Kctx.Citation
