import Kctx.Engineering.HistorySafety

set_option autoImplicit false
namespace Kctx.Engineering

theorem replay_realm {historicalEntries : List (Ledger.Record Action HistoricalPremises)}
    {record : Ledger.Record Action HistoricalPremises} {next : OperationalState}
    (h : replay historicalEntries record = some next) : next.realm = record.historical.before.realm := by
  cases hd : domain record.historical.contracts (historicalState historicalEntries record) record.payload with
  | none => simp [replay, hd] at h
  | some result =>
      simp only [replay, hd, Option.map_some, Option.some.injEq] at h
      subst next
      exact (domain_storage_frame hd).2.2

theorem acquireCut_realm {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : acquireCut contracts s id = some t) : t.realm = s.realm := by
  unfold acquireCut at h
  split at h
  · change (Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id)).bind
      (fun ledger => some { s with ledger := ledger, cuts := ⟨id, s.knowledge⟩ :: s.cuts }) = some t at h
    cases hp : Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id) with
    | none => simp [hp] at h
    | some ledger =>
        simp only [hp, bind, Option.bind, Option.some.injEq] at h
        subst t
        rfl
  · contradiction

theorem releaseReadCut_realm {s t : State} {id : Id .cut}
    (h : releaseReadCut s id = some t) : t.realm = s.realm := by
  cases hr : Ledger.releaseCut s.ledger id with
  | none => simp [releaseReadCut, hr] at h
  | some ledger =>
      simp only [releaseReadCut, hr, bind, Option.bind, Option.some.injEq] at h
      subst t
      rfl

theorem localAction_realm {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) : t.realm = s.realm := by
  cases action <;> simp only [localAction] at h
  all_goals first
    | contradiction
    | exact (project_storage_frame h).2.2
    | exact acquireCut_realm h
    | (cases h; rfl)
    | (unfold recover at h; split at h
       · cases h; rfl
       · contradiction)
    | exact releaseReadCut_realm h

theorem publishSelected_realm {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head}
    (h : publishSelected contracts s stream expected = some t) : t.realm = s.realm := by
  cases ha : s.admission with
  | none => simp [publishSelected, ha] at h
  | some admission =>
      simp only [publishSelected, ha, bind, Option.bind] at h
      split at h
      · cases hr : Ledger.retainRelease s.ledger s.retained with
        | none => simp [hr] at h
        | some ledger =>
            simp only [hr, bind, Option.bind] at h
            exact (domain_storage_frame h).2.2
      · contradiction

theorem step_realm {contracts : Contracts} {s t : State} {request : Request}
    (h : step contracts s request = some t) : t.realm = s.realm := by
  cases request with
  | append stream expected id action => exact replay_realm (step_append_exact h).1
  | publish stream expected => exact publishSelected_realm h
  | dispatch stream expected effect =>
      change dispatchSelected contracts s stream expected effect = some t at h
      unfold dispatchSelected at h
      split at h
      · exact (domain_storage_frame h).2.2
      · contradiction
  | «local» action => exact localAction_realm h

theorem transition_realm {contracts : Contracts} {s t : State}
    (h : Transition contracts s t) : t.realm = s.realm := by
  cases h with
  | operational h => exact step_realm h
  | external h => exact (environment_storage_frame h).2.2
  | refused => rfl

def RealmHistory (realm : Id .realm) (s : State) : Prop :=
  ∀ record ∈ s.ledger.records, record.historical.before.realm = realm

theorem storage_realmHistory {contracts : Contracts} {s t : State} {realm : Id .realm}
    (evolution : StorageEvolution contracts s t) (sameRealm : s.realm = realm)
    (old : RealmHistory realm s) : RealmHistory realm t := by
  intro record member
  cases evolution with
  | frame same _ => exact old record (by simpa only [same] using member)
  | append stream expected id action next _ appended _ =>
      obtain ⟨_, same⟩ := Ledger.append_success historicalValid _ _ stream expected
        (recordFor contracts s stream id action) next.retained appended
      rw [same] at member
      change record ∈ recordFor contracts s stream id action :: s.ledger.records at member
      rcases List.mem_cons.mp member with rfl | member
      · exact sameRealm
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

theorem reachable_realmHistory {contracts : Contracts} {initial s : State}
    (old : RealmHistory initial.realm initial) (reachable : Reachable contracts initial s) :
    s.realm = initial.realm ∧ RealmHistory initial.realm s := by
  induction reachable with
  | initial => exact ⟨rfl, old⟩
  | next _ transition ih =>
      refine ⟨(transition_realm transition).trans ih.1, ?_⟩
      cases transition with
      | operational h => exact storage_realmHistory (step_storageEvolution h) ih.1 ih.2
      | external h => simpa only [RealmHistory, (environment_storage_frame h).1] using ih.2
      | refused => exact ih.2

theorem init_realmHistory {contracts : Contracts} {input : InitialInputs} {s : State}
    (h : init contracts input = some s) : s.realm = input.realm ∧ RealmHistory s.realm s := by
  unfold init at h
  split at h
  · cases h; simp [RealmHistory]
  · contradiction

/-- E05 origin is checked for ALL retained records, including those no longer at
HEAD. Realm, source routing and captured contract are exact, not inferred from
an ordinal or borrowed from today's authorization state. -/
theorem E05_retained_origin {contracts : Contracts} {input : InitialInputs} {initial s : State}
    (initialized : init contracts input = some initial) (reachable : Reachable contracts initial s) :
    ∀ record ∈ s.ledger.records, ∃ historicalEntries,
      record.historical.before.realm = input.realm ∧ record.historical.contracts = contracts ∧
      historicalValid historicalEntries record = true ∧
      actionStream (historicalState historicalEntries record) record.payload = some record.stream ∧
      record.closure = inputClosure record.historical.before record.payload := by
  have origin := reachable_realmHistory (init_realmHistory initialized).2 reachable
  have history := reachable_retainedHistory (init_historySafe initialized).2 reachable
  intro record member
  obtain ⟨historicalEntries, valid, exactContract⟩ := history record member
  have checked := historicalValid_contract valid
  exact ⟨historicalEntries, (origin.2 record member).trans (init_realmHistory initialized).1,
    exactContract, valid, checked.2.2.2.2.2.2.1, checked.2.2.2.2.1⟩

end Kctx.Engineering
