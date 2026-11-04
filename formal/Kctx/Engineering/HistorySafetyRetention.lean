import Kctx.Engineering.HistorySafety

set_option autoImplicit false
namespace Kctx.Engineering

theorem storage_retains_records {contracts : Contracts} {s t : State}
    (evolution : StorageEvolution contracts s t) :
    ∀ record ∈ s.ledger.records, record ∈ t.ledger.records := by
  intro record member
  cases evolution with
  | frame same _ => simpa only [same] using member
  | append stream expected id action next _ appended _ =>
      exact Ledger.append_retains_exact_record historicalValid _ _ stream expected _ record next.retained appended member
  | pin cut pinned _ _ =>
      rw [(Ledger.pin_preserves_heads_records historicalValid _ _ s.realm cut pinned).2.1]
      exact member
  | release id released _ =>
      unfold Ledger.releaseCut at released
      split at released
      · rw [(Option.some.inj released).symm]; exact member
      · contradiction
  | publication rooted _ =>
      unfold Ledger.retainRelease at rooted
      split at rooted
      · rw [(Option.some.inj rooted).symm]; exact member
      · contradiction

theorem storage_retains_release_roots {contracts : Contracts} {s t : State}
    (evolution : StorageEvolution contracts s t) :
    ∀ object ∈ s.ledger.releaseRoots, object ∈ t.ledger.releaseRoots := by
  intro object member
  cases evolution with
  | frame same _ => simpa only [same] using member
  | append stream expected id action next _ appended _ =>
      rw [(Ledger.append_preserves_roots historicalValid _ _ stream expected _ next.retained appended).2.1]
      exact member
  | pin cut pinned _ _ =>
      rw [(Ledger.pin_success historicalValid _ _ s.realm cut pinned).2]
      exact member
  | release id released _ =>
      unfold Ledger.releaseCut at released
      split at released
      · rw [(Option.some.inj released).symm]; exact member
      · contradiction
  | publication rooted _ =>
      unfold Ledger.retainRelease at rooted
      split at rooted
      · rw [(Option.some.inj rooted).symm]
        exact List.mem_append_right _ member
      · contradiction

theorem transition_retains_records {contracts : Contracts} {s t : State}
    (transition : Transition contracts s t) :
    ∀ record ∈ s.ledger.records, record ∈ t.ledger.records := by
  cases transition with
  | operational h => exact storage_retains_records (step_storageEvolution h)
  | external h => simp only [(environment_storage_frame h).1]; exact fun _ h => h
  | refused => exact fun _ h => h

theorem reachable_retains_records {contracts : Contracts} {s t : State}
    (reachable : Reachable contracts s t) :
    ∀ record ∈ s.ledger.records, record ∈ t.ledger.records := by
  induction reachable with
  | initial => exact fun _ h => h
  | next _ transition ih => exact fun record member => transition_retains_records transition record (ih record member)

theorem reachable_retains_release_roots {contracts : Contracts} {s t : State}
    (reachable : Reachable contracts s t) :
    ∀ object ∈ s.ledger.releaseRoots, object ∈ t.ledger.releaseRoots := by
  induction reachable with
  | initial => exact fun _ h => h
  | next _ transition ih =>
      intro object member
      cases transition with
      | operational h => exact storage_retains_release_roots (step_storageEvolution h) object (ih object member)
      | external h => simpa only [(environment_storage_frame h).1] using ih object member
      | refused => exact ih object member

/-- Immutable identity includes payload, closure, realm/source premises, predecessor
and the historical external contract. It is not an ordinal comparison. -/
theorem reachable_immutable_payload {contracts : Contracts} {s t : State}
    (safe : HistorySafe s) (reachable : Reachable contracts s t)
    {before after : Ledger.Record Action HistoricalPremises}
    (retained : before ∈ s.ledger.records) (later : after ∈ t.ledger.records)
    (identity : before.id = after.id) : before = after := by
  exact (reachable_historySafe safe reachable).1.1 before
    (reachable_retains_records reachable before retained) after later identity

/-- Publication evidence remains material after arbitrary appends, policy changes,
reader releases and later publications; conditional retention is still explicit. -/
theorem reachable_release_evidence {contracts : Contracts} {s t : State}
    (safe : HistorySafe s) (reachable : Reachable contracts s t)
    {object : ObjectId} (root : object ∈ s.ledger.releaseRoots) :
    object ∈ t.ledger.releaseRoots ∧ object ∈ t.ledger.material := by
  have retained := reachable_retains_release_roots reachable object root
  exact ⟨retained, (reachable_historySafe safe reachable).2.2 object retained⟩

end Kctx.Engineering
