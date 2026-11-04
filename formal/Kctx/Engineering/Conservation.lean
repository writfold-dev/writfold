import Kctx.Engineering.PromotionSafety

set_option autoImplicit false
set_option maxHeartbeats 2400000
namespace Kctx.Engineering

abbrev LegacyState := PortfolioState × List PortfolioJournalEntry

def legacyView (s : State) : LegacyState := (s.knowledge, s.knowledgeJournal)
def operationalLegacyView (s : OperationalState) : LegacyState := (s.knowledge, s.knowledgeJournal)

/-- The journal is threaded through the original operation, not existentially
replaced at each edge. Replay/input-conflict semantics are therefore conserved too. -/
inductive LegacyTrace (contracts : Contracts) (initial : LegacyState) : LegacyState → Prop where
  | initial : LegacyTrace contracts initial initial
  | admitted {before : LegacyState} (plan : PortfolioPlan) :
      LegacyTrace contracts initial before →
      (applyPortfolio before.1 before.2 plan contracts.approvals).outcome = .applied →
      LegacyTrace contracts initial
        ((applyPortfolio before.1 before.2 plan contracts.approvals).state,
         (applyPortfolio before.1 before.2 plan contracts.approvals).journal)

theorem domain_nonpublication_legacy {contracts : Contracts} {s t : State} {action : Action}
    (different : action ≠ .publishAdmission) (h : domain contracts s action = some t) :
    legacyView t = legacyView s := by
  cases hb : domainBody contracts s action with
  | none => simp [domain, hb] at h
  | some next =>
      simp only [domain, hb, Option.map_some, Option.some.injEq] at h
      subst t
      simp [legacyView, frameDomain, different]

theorem environment_legacy_frame {contracts : Contracts} {s t : State} {action : Environment}
    (h : environment contracts s action = some t) : legacyView t = legacyView s := by
  cases hb : environmentBody contracts s action with
  | none => simp [environment, hb] at h
  | some next =>
      simp only [environment, hb, bind, Option.bind, Option.some.injEq] at h
      subst t
      rfl

theorem project_legacy_frame {contracts : Contracts} {s t : State} {pin : Ledger.NativePin}
    {heads : List Ledger.Head} {epoch : Nat}
    (h : project contracts s pin heads epoch = some t) : legacyView t = legacyView s := by
  unfold project at h
  split at h
  · cases h; rfl
  · contradiction

theorem acquireCut_legacy_frame {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : acquireCut contracts s id = some t) : legacyView t = legacyView s := by
  unfold acquireCut at h
  obtain ⟨_, selected⟩ := Option.ite_none_right_eq_some.mp h
  obtain ⟨ledger, _, result⟩ := Option.bind_eq_some_iff.mp selected
  simp only [Option.some.injEq] at result
  subst t
  rfl

theorem releaseCut_legacy_frame {s t : State} {id : Id .cut}
    (h : releaseReadCut s id = some t) : legacyView t = legacyView s := by
  cases hl : Ledger.releaseCut s.ledger id with
  | none => simp [releaseReadCut, hl] at h
  | some ledger =>
      simp only [releaseReadCut, hl, bind, Option.bind, Option.some.injEq] at h
      subst t
      rfl

theorem localAction_legacy_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) : legacyView t = legacyView s := by
  cases action <;> simp only [localAction] at h
  all_goals first
    | contradiction
    | exact project_legacy_frame h
    | exact acquireCut_legacy_frame h
    | exact releaseCut_legacy_frame h
    | (cases h; rfl)
    | (unfold recover at h; split at h
       · cases h; rfl
       · contradiction)

/-- Ledgered actions do not publish the corpus; their historical snapshots
project to exactly the caller's current legacy state and journal. -/
theorem append_legacy_frame {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected id action) = some t) :
    legacyView t = legacyView s := by
  have durable : action.durable = true := by
    simp only [step] at h
    split at h
    next guard =>
      simp only [Bool.and_eq_true, and_assoc] at guard
      exact guard.1
    next refused => contradiction
  have different : action ≠ .publishAdmission := by
    intro same
    subst action
    simp [Action.durable] at durable
  have replayed := (step_append_exact h).1
  let record := recordFor contracts s stream id action
  let before := historicalState (Ledger.currentPrefix s.ledger stream) record
  change (domain contracts before action).map State.toOperationalState = some t.toOperationalState at replayed
  cases hd : domain contracts before action with
  | none => simp [hd] at replayed
  | some next =>
      have framed := domain_nonpublication_legacy different hd
      simp only [hd, Option.map_some, Option.some.injEq] at replayed
      change operationalLegacyView t.toOperationalState = legacyView s
      rw [← replayed]
      exact framed

theorem step_legacy_trace {contracts : Contracts} {initial : LegacyState}
    {s t : State} {request : Request}
    (before : LegacyTrace contracts initial (legacyView s))
    (h : step contracts s request = some t) : LegacyTrace contracts initial (legacyView t) := by
  cases request with
  | append stream expected id action =>
      rw [append_legacy_frame h]
      exact before
  | publish stream expected =>
      obtain ⟨admission, certificate⟩ := publishSelected_certificate h
      rw [legacyView, certificate.knowledge, certificate.journal]
      exact LegacyTrace.admitted admission.candidate.plan before certificate.applied
  | dispatch stream expected effect =>
      simp only [step, dispatchSelected] at h
      split at h
      · have frame := domain_nonpublication_legacy (by simp) h
        rw [frame]
        exact before
      · contradiction
  | «local» action =>
      rw [localAction_legacy_frame h]
      exact before

/-- E01: every finite operational/environment/refusal trace has an exact
legacy-admission trace with the same final knowledge AND operation journal.
There is no new witness constructor or skill-text inference rule. -/
theorem E01_conservative_extension {contracts : Contracts} {initial s : State}
    (h : Reachable contracts initial s) :
    LegacyTrace contracts (legacyView initial) (legacyView s) := by
  induction h with
  | initial => exact LegacyTrace.initial
  | next _ transition ih =>
      cases transition with
      | operational h => exact step_legacy_trace ih h
      | external h => rw [environment_legacy_frame h]; exact ih
      | refused => exact ih

theorem E01_nonpublication_derivation {contracts : Contracts} {s t : State} {action : Action}
    (different : action ≠ .publishAdmission) (h : domain contracts s action = some t)
    (context : Context) (witness : Witness) (checked : Checked) :
    Derives t.knowledge.kb context witness checked ↔ Derives s.knowledge.kb context witness checked := by
  have same := congrArg Prod.fst (domain_nonpublication_legacy different h)
  change t.knowledge = s.knowledge at same
  rw [same]

end Kctx.Engineering
