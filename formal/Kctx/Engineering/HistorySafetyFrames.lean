import Kctx.Engineering.Actions
import Kctx.Engineering.LedgerCutProofs

set_option autoImplicit false
namespace Kctx.Engineering

/-- The manifest literally assembled by acquireCut, including non-head inputs. -/
def snapshotCut (s : State) (id : Id .cut) : Ledger.Cut Action HistoricalPremises :=
  { id := id, realm := s.realm, inventoryEpoch := s.projectionInventory
    inventory := Ledger.orderedInventory s.ledger
    watermarks := Ledger.orderedSelections s.ledger
    corpusPins := [s.corpusPin], activityPin := s.activityPin
    projectionContract := s.projectionContract, projectionEnvelope := s.projectionEnvelope
    visibilityContract := s.visibilityContract, visibilityEnvelope := s.visibilityEnvelope
    assessmentTime := s.now, material := s.retained }

/-- Exhaustive storage effects of the actual interpreter. These constructors
carry successful primitive calls, not assumptions of their preservation result. -/
inductive StorageEvolution (contracts : Contracts) (s t : State) : Prop where
  | frame (ledger : t.ledger = s.ledger) (contexts : t.cuts = s.cuts)
  | append (stream : StreamId) (expected : Option Ledger.Head) (id : LedgerRecordId)
      (action : Action) (next : OperationalState)
      (replayed : replay (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream id action) = some next)
      (appended : Ledger.append historicalValid s.ledger stream expected
        (recordFor contracts s stream id action) next.retained = some t.ledger)
      (contexts : t.cuts = s.cuts)
  | pin (cut : Ledger.Cut Action HistoricalPremises)
      (pinned : Ledger.pin historicalValid s.ledger s.realm cut = some t.ledger)
      (contexts : t.cuts = ⟨cut.id, s.knowledge⟩ :: s.cuts)
      (fresh : ∀ old ∈ s.cuts, old.id ≠ cut.id)
  | release (id : Id .cut) (released : Ledger.releaseCut s.ledger id = some t.ledger)
      (contexts : t.cuts = s.cuts)
  | publication (rooted : Ledger.retainRelease s.ledger s.retained = some t.ledger)
      (contexts : t.cuts = s.cuts)

theorem domain_storage_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) :
    t.ledger = s.ledger ∧ t.cuts = s.cuts ∧ t.realm = s.realm := by
  cases hb : domainBody contracts s action with
  | none => simp [domain, hb] at h
  | some next =>
      simp only [domain, hb, Option.map_some, Option.some.injEq] at h
      subst t
      exact ⟨rfl, rfl, rfl⟩

theorem environment_storage_frame {contracts : Contracts} {s t : State} {action : Environment}
    (h : environment contracts s action = some t) :
    t.ledger = s.ledger ∧ t.cuts = s.cuts ∧ t.realm = s.realm := by
  cases hb : environmentBody contracts s action with
  | none => simp [environment, hb] at h
  | some next =>
      simp only [environment, hb, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact ⟨rfl, rfl, rfl⟩

theorem project_storage_frame {contracts : Contracts} {s t : State}
    {pin : Ledger.NativePin} {heads : List Ledger.Head} {epoch : Nat}
    (h : project contracts s pin heads epoch = some t) :
    t.ledger = s.ledger ∧ t.cuts = s.cuts ∧ t.realm = s.realm := by
  unfold project at h
  split at h
  · cases h; exact ⟨rfl, rfl, rfl⟩
  · contradiction

theorem acquireCut_storageEvolution {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : acquireCut contracts s id = some t) : StorageEvolution contracts s t := by
  unfold acquireCut at h
  split at h
  next guard =>
    change (Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id)).bind
      (fun ledger => some { s with ledger := ledger, cuts := ⟨id, s.knowledge⟩ :: s.cuts }) = some t at h
    cases hp : Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id) with
    | none => simp [hp] at h
    | some ledger =>
        simp only [hp, bind, Option.bind, Option.some.injEq] at h
        subst t
        apply StorageEvolution.pin _ hp rfl
        intro old member same
        have noOld : s.cuts.any (·.id == id) = false := by
          simp only [Bool.and_eq_true] at guard
          simpa only [Bool.not_eq_true'] using guard.2
        have yesOld : s.cuts.any (·.id == id) = true := by
          exact List.any_eq_true.mpr ⟨old, member, by simpa using same⟩
        simp [noOld] at yesOld
  next => contradiction

theorem releaseReadCut_storageEvolution {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : releaseReadCut s id = some t) : StorageEvolution contracts s t := by
  cases hr : Ledger.releaseCut s.ledger id with
  | none => simp [releaseReadCut, hr] at h
  | some ledger =>
      simp only [releaseReadCut, hr, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact .release id hr rfl

theorem publishSelected_storageEvolution {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head}
    (h : publishSelected contracts s stream expected = some t) : StorageEvolution contracts s t := by
  cases ha : s.admission with
  | none => simp [publishSelected, ha] at h
  | some admission =>
      simp only [publishSelected, ha, bind, Option.bind] at h
      split at h
      · cases hr : Ledger.retainRelease s.ledger s.retained with
        | none => simp [hr] at h
        | some ledger =>
            simp only [hr, bind, Option.bind] at h
            have frame := domain_storage_frame h
            exact .publication (by simpa only [frame.1] using hr) frame.2.1
      · contradiction

theorem dispatchSelected_storageEvolution {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {effect : Id .effect}
    (h : dispatchSelected contracts s stream expected effect = some t) :
    StorageEvolution contracts s t := by
  unfold dispatchSelected at h
  split at h
  · have frame := domain_storage_frame h
    exact .frame frame.1 frame.2.1
  · contradiction

theorem localAction_storageEvolution {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) : StorageEvolution contracts s t := by
  cases action <;> simp only [localAction] at h
  all_goals first
    | contradiction
    | exact acquireCut_storageEvolution h
    | exact releaseReadCut_storageEvolution h
    | exact (StorageEvolution.frame (project_storage_frame h).1
        (project_storage_frame h).2.1)
    | (cases h; exact .frame rfl rfl)
    | (unfold recover at h; split at h
       · cases h; exact .frame rfl rfl
       · contradiction)

theorem step_storageEvolution {contracts : Contracts} {s t : State} {request : Request}
    (h : step contracts s request = some t) : StorageEvolution contracts s t := by
  cases request with
  | append stream expected id action =>
      simp only [step] at h
      split at h
      · cases hr : replay (Ledger.currentPrefix s.ledger stream)
            (recordFor contracts s stream id action) with
        | none => simp [hr] at h
        | some next =>
            simp only [hr, bind, Option.bind] at h
            cases hl : Ledger.append historicalValid s.ledger stream expected
                (recordFor contracts s stream id action) next.retained with
            | none => simp [hl] at h
            | some ledger =>
                simp only [hl, bind, Option.bind, Option.some.injEq] at h
                subst t
                exact .append stream expected id action next hr hl rfl
      · contradiction
  | publish stream expected => exact publishSelected_storageEvolution h
  | dispatch stream expected effect => exact dispatchSelected_storageEvolution h
  | «local» action => exact localAction_storageEvolution h

/-- The exact append result, including the operational payload returned by replay. -/
theorem step_append_exact {contracts : Contracts} {s t : State} {stream : StreamId}
    {expected : Option Ledger.Head} {id : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected id action) = some t) :
    replay (Ledger.currentPrefix s.ledger stream) (recordFor contracts s stream id action) =
      some t.toOperationalState ∧
    Ledger.append historicalValid s.ledger stream expected
      (recordFor contracts s stream id action) t.retained = some t.ledger ∧ t.cuts = s.cuts := by
  simp only [step] at h
  split at h
  · cases hr : replay (Ledger.currentPrefix s.ledger stream)
        (recordFor contracts s stream id action) with
    | none => simp [hr] at h
    | some next =>
        simp only [hr, bind, Option.bind] at h
        cases hl : Ledger.append historicalValid s.ledger stream expected
            (recordFor contracts s stream id action) next.retained with
        | none => simp [hl] at h
        | some ledger =>
            simp only [hl, bind, Option.bind, Option.some.injEq] at h
            subst t
            exact ⟨rfl, hl, rfl⟩
  · contradiction

end Kctx.Engineering
