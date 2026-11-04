import Kctx.Engineering.HistorySafetyFrames

set_option autoImplicit false
namespace Kctx.Engineering

/-- E05 and the permanent evidence component shared by E08/E09. -/
def HistorySafe (s : State) : Prop :=
  Ledger.WellFormed historicalValid s.ledger ∧ Ledger.EvidenceClosed historicalValid s.ledger

/-- Every retained record, not merely the newest selected ordinal, has exact
historical premises under the interpreter's explicit external contracts. -/
def RetainedHistory (contracts : Contracts) (s : State) : Prop :=
  ∀ record ∈ s.ledger.records, ∃ historicalEntries,
    historicalValid historicalEntries record = true ∧ record.historical.contracts = contracts

theorem storage_historySafe {contracts : Contracts} {s t : State}
    (evolution : StorageEvolution contracts s t) (safe : HistorySafe s) : HistorySafe t := by
  rcases safe with ⟨wf, evidence⟩
  cases evolution with
  | frame same _ => simpa only [HistorySafe, same] using And.intro wf evidence
  | append stream expected id action next _ appended _ =>
      exact ⟨Ledger.append_preserves historicalValid _ _ stream expected _ next.retained wf appended,
        Ledger.append_preserves_evidence historicalValid _ _ stream expected _ next.retained evidence appended⟩
  | pin cut pinned _ _ =>
      exact ⟨Ledger.pin_preserves_wellFormed historicalValid _ _ s.realm cut wf pinned,
        Ledger.pin_preserves_evidence historicalValid _ _ s.realm cut evidence pinned⟩
  | release id released _ =>
      unfold Ledger.releaseCut at released
      split at released
      · have same := (Option.some.inj released).symm
        rw [HistorySafe, same]
        exact ⟨wf, evidence⟩
      · contradiction
  | publication rooted _ =>
      have closed := Ledger.release_roots_closed historicalValid _ _ s.retained evidence rooted
      unfold Ledger.retainRelease at rooted
      split at rooted
      · have same := (Option.some.inj rooted).symm
        exact ⟨by simpa only [same] using wf, closed⟩
      · contradiction

theorem storage_retainedHistory {contracts : Contracts} {s t : State}
    (evolution : StorageEvolution contracts s t) (old : RetainedHistory contracts s) :
    RetainedHistory contracts t := by
  intro record member
  cases evolution with
  | frame same _ => exact old record (by simpa only [same] using member)
  | append stream expected id action next _ appended _ =>
      obtain ⟨ready, same⟩ := Ledger.append_success historicalValid _ _ stream expected
        (recordFor contracts s stream id action) next.retained appended
      rw [same] at member
      change record ∈ recordFor contracts s stream id action :: s.ledger.records at member
      rcases List.mem_cons.mp member with rfl | member
      · exact ⟨Ledger.currentPrefix s.ledger stream, ready.2.2.2.2.2.2.2.2.2, rfl⟩
      · exact old record member
  | pin cut pinned _ _ =>
      have same := (Ledger.pin_preserves_heads_records historicalValid _ _ s.realm cut pinned).2.1
      exact old record (by simpa only [same] using member)
  | release id released _ =>
      unfold Ledger.releaseCut at released
      split at released
      · have same := (Option.some.inj released).symm
        exact old record (by simpa only [same] using member)
      · contradiction
  | publication rooted _ =>
      unfold Ledger.retainRelease at rooted
      split at rooted
      · have same := (Option.some.inj rooted).symm
        exact old record (by simpa only [same] using member)
      · contradiction

theorem init_historySafe {contracts : Contracts} {input : InitialInputs} {s : State}
    (h : init contracts input = some s) : HistorySafe s ∧ RetainedHistory contracts s := by
  unfold init at h
  split at h
  · cases h
    simp [HistorySafe, RetainedHistory, Ledger.WellFormed, Ledger.UniqueRecords,
      Ledger.UniqueHeads, Ledger.EvidenceClosed]
  · contradiction

theorem step_historySafe {contracts : Contracts} {s t : State} {request : Request}
    (safe : HistorySafe s) (h : step contracts s request = some t) : HistorySafe t :=
  storage_historySafe (step_storageEvolution h) safe

theorem environment_historySafe {contracts : Contracts} {s t : State} {action : Environment}
    (safe : HistorySafe s) (h : environment contracts s action = some t) : HistorySafe t := by
  simpa only [HistorySafe, (environment_storage_frame h).1] using safe

theorem transition_historySafe {contracts : Contracts} {s t : State}
    (safe : HistorySafe s) (h : Transition contracts s t) : HistorySafe t := by
  cases h with
  | operational h => exact step_historySafe safe h
  | external h => exact environment_historySafe safe h
  | refused => exact safe

theorem reachable_historySafe {contracts : Contracts} {initial s : State}
    (safe : HistorySafe initial) (h : Reachable contracts initial s) : HistorySafe s := by
  induction h with
  | initial => exact safe
  | next _ transition ih => exact transition_historySafe ih transition

theorem reachable_retainedHistory {contracts : Contracts} {initial s : State}
    (old : RetainedHistory contracts initial) (h : Reachable contracts initial s) :
    RetainedHistory contracts s := by
  induction h with
  | initial => exact old
  | next _ transition ih =>
      cases transition with
      | operational h => exact storage_retainedHistory (step_storageEvolution h) ih
      | external h => simpa only [RetainedHistory, (environment_storage_frame h).1] using ih
      | refused => exact ih

/-- Integrated init/step/environment theorem for every selected stream and every
retained payload, including released cuts and previously published release roots. -/
theorem E05_E09_history {contracts : Contracts} {input : InitialInputs} {initial s : State}
    (initialized : init contracts input = some initial) (reachable : Reachable contracts initial s) :
    HistorySafe s ∧ RetainedHistory contracts s :=
  ⟨reachable_historySafe (init_historySafe initialized).1 reachable,
    reachable_retainedHistory (init_historySafe initialized).2 reachable⟩

theorem every_selected_stream_replays {s : State} (safe : HistorySafe s) (stream : StreamId) :
    Ledger.Linked historicalValid stream (Ledger.currentPrefix s.ledger stream) ∧
    Ledger.Retained s.ledger (Ledger.currentPrefix s.ledger stream) :=
  ⟨Ledger.currentPrefix_linked historicalValid s.ledger stream safe.1,
    Ledger.currentPrefix_retained historicalValid s.ledger stream safe.1⟩

/-- Source, claim stamp and exact input closure are checked against the recorded
before-state, never against present policy or a projection row. -/
theorem historicalValid_contract {historicalEntries : List (Ledger.Record Action HistoricalPremises)}
    {record : Ledger.Record Action HistoricalPremises} (h : historicalValid historicalEntries record = true) :
    record.profile = ⟨1⟩ ∧ record.payload.durable = true ∧ record.historical.before.alive = true ∧
    operationalValid record.historical.contracts record.historical.before = true ∧
    record.closure = inputClosure record.historical.before record.payload ∧
    claimStampBound (historicalEntries.head?.map Ledger.Record.head) record.id record.payload = true ∧
    actionStream (historicalState historicalEntries record) record.payload = some record.stream ∧
    (replay historicalEntries record).isSome = true := by
  simpa only [historicalValid, Bool.and_eq_true, beq_iff_eq, and_assoc] using h

theorem historical_claim_identity {historicalEntries : List (Ledger.Record Action HistoricalPremises)}
    {record : Ledger.Record Action HistoricalPremises} {effect : Id .effect} {claim : Id .claim}
    {expected : Option Ledger.Head} {id : LedgerRecordId}
    (valid : historicalValid historicalEntries record = true)
    (payload : record.payload = .claimDispatch effect claim expected id) :
    expected = historicalEntries.head?.map Ledger.Record.head ∧ id = record.id := by
  have bound := (historicalValid_contract valid).2.2.2.2.2.1
  simpa only [payload, claimStampBound, Bool.and_eq_true, beq_iff_eq] using bound

theorem append_captures_exact_inputs (contracts : Contracts) (s : State) (stream : StreamId)
    (id : LedgerRecordId) (action : Action) :
    (recordFor contracts s stream id action).historical.before = s.toOperationalState ∧
    (recordFor contracts s stream id action).historical.contracts = contracts ∧
    (recordFor contracts s stream id action).historical.before.realm = s.realm ∧
    (recordFor contracts s stream id action).payload = action := ⟨rfl, rfl, rfl, rfl⟩

/-- Reusing a selected predecessor cannot commit a different payload or record ID. -/
theorem conflicting_head_requires_refusal {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId} {action : Action}
    (first : step contracts s (.append stream expected id action) = some t)
    (otherId : LedgerRecordId) (other : Action) :
    step contracts t (.append stream expected otherId other) = none := by
  have append := (step_append_exact first).2.1
  obtain ⟨ready, committed⟩ := Ledger.append_success historicalValid _ _ stream expected
    (recordFor contracts s stream id action) t.retained append
  have stale : expected ≠ Ledger.currentHead t.ledger stream := by
    rw [committed, Ledger.commit_currentHead]
    exact Ledger.selected_head_changes historicalValid _ _ _ _ _ ready
  simp only [step]
  split
  · cases hr : replay (Ledger.currentPrefix t.ledger stream)
        (recordFor contracts t stream otherId other) with
    | none => simp [hr]
    | some next =>
        simp [hr, Ledger.stale_head_refuses historicalValid t.ledger stream expected
          (recordFor contracts t stream otherId other) next.retained stale]
  · rfl

theorem stale_head_requires_refusal (contracts : Contracts) (s : State) (stream : StreamId)
    (expected : Option Ledger.Head) (id : LedgerRecordId) (action : Action)
    (stale : expected ≠ Ledger.currentHead s.ledger stream) :
    step contracts s (.append stream expected id action) = none := by
  simp only [step]
  split
  · cases hr : replay (Ledger.currentPrefix s.ledger stream) (recordFor contracts s stream id action) with
    | none => simp [hr]
    | some next => simp [hr, Ledger.stale_head_refuses historicalValid _ _ _ _ next.retained stale]
  · rfl

theorem malformed_head_requires_refusal (contracts : Contracts) (s : State) (stream : StreamId)
    (expected : Option Ledger.Head) (id : LedgerRecordId) (action : Action)
    (malformed : (Ledger.currentPrefix s.ledger stream).head?.map Ledger.Record.head ≠
      Ledger.currentHead s.ledger stream) :
    step contracts s (.append stream expected id action) = none := by
  simp only [step]
  split
  · cases hr : replay (Ledger.currentPrefix s.ledger stream) (recordFor contracts s stream id action) with
    | none => simp [hr]
    | some next => simp [hr, Ledger.malformed_head_refuses historicalValid _ _ _ _ next.retained malformed]
  · rfl

end Kctx.Engineering
