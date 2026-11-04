import Kctx.Engineering.Actions
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
set_option maxRecDepth 2048
namespace Kctx.Engineering

theorem replay_domain_witness {entries : List (Ledger.Record Action HistoricalPremises)}
    {record : Ledger.Record Action HistoricalPremises} {result : OperationalState}
    (h : replay entries record = some result) :
    ∃ next : State,
      domain record.historical.contracts (historicalState entries record) record.payload = some next ∧
      next.toOperationalState = result := by
  cases executed : domain record.historical.contracts (historicalState entries record) record.payload with
  | none => simp [replay, executed] at h
  | some next =>
      simp only [replay, executed, Option.map_some, Option.some.injEq] at h
      exact ⟨next, rfl, h⟩

/-- The historical interpreter used by append has exactly the current operational
inputs. Only its ledger is reconstructed from the selected stream prefix. -/
theorem append_domain_witness {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected id action) = some t) :
    ∃ next, domain contracts
      (historicalState (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream id action)) action = some next ∧
      t.toOperationalState = next.toOperationalState := by
  simp only [step] at h
  split at h
  · cases replayed : replay (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream id action) with
    | none => simp [replayed] at h
    | some operational =>
        obtain ⟨next, executed, operationalEq⟩ := replay_domain_witness replayed
        simp only [replayed, bind, Option.bind] at h
        cases committed : Ledger.append historicalValid s.ledger stream expected
            (recordFor contracts s stream id action) operational.retained with
        | none => simp [committed] at h
        | some ledger =>
            simp only [committed, bind, Option.bind, Option.some.injEq] at h
            subst t
            exact ⟨next, executed, operationalEq.symm⟩
  · contradiction

theorem publish_domain_witness {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head}
    (h : publishSelected contracts s stream expected = some t) :
    ∃ rooted, Ledger.retainRelease s.ledger s.retained = some rooted ∧
      domain contracts { s with ledger := rooted } .publishAdmission = some t := by
  cases admissionFound : s.admission with
  | none => simp [publishSelected, admissionFound] at h
  | some admission =>
      simp only [publishSelected, admissionFound, bind, Option.bind] at h
      split at h
      · cases retained : Ledger.retainRelease s.ledger s.retained with
        | none => simp [retained] at h
        | some rooted =>
            simp only [retained, bind, Option.bind] at h
            exact ⟨rooted, rfl, h⟩
      · contradiction

/-- A ledger commit never needs an invariant postfilter: any property of the
operational state preserved by the actual reducer and local transitions lifts
through replay, publication CAS, physical dispatch and ledger append. -/
theorem step_preserves_operational_property {contracts : Contracts}
    {P : OperationalState → Prop}
    (domainPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → domain contracts s action = some t → P t.toOperationalState)
    (localPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → localAction contracts s action = some t → P t.toOperationalState)
    {s t : State} {request : Request} (before : P s.toOperationalState)
    (h : step contracts s request = some t) : P t.toOperationalState := by
  cases request with
  | append stream expected id action =>
      obtain ⟨next, executed, same⟩ := append_domain_witness h
      rw [same]
      exact domainPreserves
        (s := historicalState (Ledger.currentPrefix s.ledger stream)
          (recordFor contracts s stream id action)) before executed
  | publish stream expected =>
      obtain ⟨rooted, _, executed⟩ := publish_domain_witness h
      exact domainPreserves (s := { s with ledger := rooted }) before executed
  | dispatch stream expected effect =>
      simp only [step, dispatchSelected] at h
      split at h
      · exact domainPreserves before h
      · contradiction
  | «local» action => exact localPreserves before h

theorem transition_preserves_operational_property {contracts : Contracts}
    {P : OperationalState → Prop}
    (domainPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → domain contracts s action = some t → P t.toOperationalState)
    (localPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → localAction contracts s action = some t → P t.toOperationalState)
    (externalPreserves : ∀ {s t : State} {action : Environment},
      P s.toOperationalState → environment contracts s action = some t → P t.toOperationalState)
    {s t : State} (before : P s.toOperationalState) (h : Transition contracts s t) :
    P t.toOperationalState := by
  cases h with
  | operational executed => exact step_preserves_operational_property domainPreserves localPreserves before executed
  | external executed => exact externalPreserves before executed
  | refused => exact before

theorem reachable_operational_property {contracts : Contracts} {initial s : State}
    {P : OperationalState → Prop}
    (domainPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → domain contracts s action = some t → P t.toOperationalState)
    (localPreserves : ∀ {s t : State} {action : Action},
      P s.toOperationalState → localAction contracts s action = some t → P t.toOperationalState)
    (externalPreserves : ∀ {s t : State} {action : Environment},
      P s.toOperationalState → environment contracts s action = some t → P t.toOperationalState)
    (base : P initial.toOperationalState) (h : Reachable contracts initial s) :
    P s.toOperationalState := by
  induction h with
  | initial => exact base
  | next _ edge ih =>
      exact transition_preserves_operational_property domainPreserves localPreserves externalPreserves ih edge

/-- Relational edge properties lift too: the append source has the same exact
operational inputs, rather than merely an isomorphic or current-policy replay. -/
theorem step_operational_relation {contracts : Contracts}
    {R : OperationalState → OperationalState → Prop}
    (domainChecks : ∀ {s t : State} {action : Action},
      domain contracts s action = some t → R s.toOperationalState t.toOperationalState)
    (localChecks : ∀ {s t : State} {action : Action},
      localAction contracts s action = some t → R s.toOperationalState t.toOperationalState)
    {s t : State} {request : Request} (h : step contracts s request = some t) :
    R s.toOperationalState t.toOperationalState := by
  cases request with
  | append stream expected id action =>
      obtain ⟨next, executed, same⟩ := append_domain_witness h
      rw [same]
      exact domainChecks
        (s := historicalState (Ledger.currentPrefix s.ledger stream)
          (recordFor contracts s stream id action)) executed
  | publish stream expected =>
      obtain ⟨rooted, _, executed⟩ := publish_domain_witness h
      exact domainChecks (s := { s with ledger := rooted }) executed
  | dispatch stream expected effect =>
      simp only [step, dispatchSelected] at h
      split at h
      · exact domainChecks h
      · contradiction
  | «local» action => exact localChecks h

end Kctx.Engineering
