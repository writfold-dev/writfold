import Kctx.Engineering.LedgerAppend

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

theorem commit_unique_records {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) (unique : UniqueRecords s)
    (fresh : ∀ old ∈ s.records, old.id ≠ r.id) :
    UniqueRecords (commit s stream r available) := by
  intro a ha b hb eqid
  change a ∈ r :: s.records at ha
  change b ∈ r :: s.records at hb
  simp only [List.mem_cons] at ha hb
  rcases ha with rfl | ha
  · rcases hb with rfl | hb
    · rfl
    · exact False.elim (fresh b hb eqid.symm)
  · rcases hb with rfl | hb
    · exact False.elim (fresh a ha eqid)
    · exact unique a ha b hb eqid

theorem commit_unique_heads {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) (unique : UniqueHeads s) :
    UniqueHeads (commit s stream r available) := by
  intro a ha b hb eqstream
  change a ∈ (⟨stream, r.head, r :: currentPrefix s stream⟩ : Selection P H) ::
    s.selections.filter (fun old => old.stream != stream) at ha
  change b ∈ (⟨stream, r.head, r :: currentPrefix s stream⟩ : Selection P H) ::
    s.selections.filter (fun old => old.stream != stream) at hb
  simp only [List.mem_cons, List.mem_filter, bne_iff_ne] at ha hb
  rcases ha with rfl | ⟨ha, hna⟩
  · rcases hb with rfl | ⟨hb, hnb⟩
    · rfl
    · exact False.elim (hnb eqstream.symm)
  · rcases hb with rfl | ⟨hb, hnb⟩
    · exact False.elim (hna eqstream)
    · exact unique a ha b hb eqstream

theorem commit_retained {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) (entries : List (Record P H))
    (old : Retained s entries) : Retained (commit s stream r available) entries := by
  intro prior hp
  obtain ⟨hr, hm⟩ := old prior hp
  exact ⟨commit_retains_records s stream r prior available hr,
    fun o ho => commit_retains_material s stream r available o (hm o ho)⟩

theorem commit_wellFormed {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (expected : Option Head)
    (r : Record P H) (available : List ObjectId)
    (wf : WellFormed validate s)
    (ready : AppendReady validate s stream expected r available) :
    WellFormed validate (commit s stream r available) := by
  rcases ready with ⟨hexpected, hhead, hlinked, hretained, hstream,
    hsequence, hpredecessor, hfresh, hclosure, hvalidate⟩
  refine ⟨commit_unique_records s stream r available wf.1 hfresh,
    commit_unique_heads s stream r available wf.2.1, ?_⟩
  intro selection hs
  change selection ∈ (⟨stream, r.head, r :: currentPrefix s stream⟩ : Selection P H) ::
    s.selections.filter (fun old => old.stream != stream) at hs
  simp only [List.mem_cons, List.mem_filter] at hs
  rcases hs with rfl | ⟨hs, _⟩
  · refine ⟨rfl, hstream, ?_, ?_⟩
    · exact linked_cons validate stream r (currentPrefix s stream)
        hlinked hstream hsequence hpredecessor hvalidate
    · intro prior hp
      simp only [List.mem_cons] at hp
      rcases hp with rfl | hp
      · exact ⟨List.mem_cons_self, hclosure⟩
      · exact commit_retained s stream r available _ hretained prior hp
  · obtain ⟨hh, hstream', hl, hr⟩ := wf.2.2 selection hs
    exact ⟨hh, hstream', hl, commit_retained s stream r available _ hr⟩

theorem append_preserves {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (wf : WellFormed validate s)
    (h : append validate s stream expected r available = some t) :
    WellFormed validate t := by
  obtain ⟨ready, rfl⟩ := append_success validate s t stream expected r available h
  exact commit_wellFormed validate s stream expected r available wf ready

theorem append_retains_exact_record {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r old : Record P H) (available : List ObjectId)
    (h : append validate s stream expected r available = some t)
    (retained : old ∈ s.records) : old ∈ t.records := by
  obtain ⟨_, rfl⟩ := append_success validate s t stream expected r available h
  exact commit_retains_records s stream r old available retained

theorem selected_head_changes {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (expected : Option Head)
    (r : Record P H) (available : List ObjectId)
    (ready : AppendReady validate s stream expected r available) :
    expected ≠ some r.head := by
  intro heq
  have hh := ready.2.1
  have hc : currentHead s stream = some r.head := ready.1.symm.trans heq
  rw [hc] at hh
  cases hp : currentPrefix s stream with
  | nil => simp [hp] at hh
  | cons first rest =>
      have heads : first.head = r.head := by simpa [hp] using hh
      have ids : first.id = r.id := congrArg Head.record heads
      have retained := ready.2.2.2.1 first (by simp [hp])
      exact ready.2.2.2.2.2.2.2.1 first retained.1 ids

/-- The same predecessor cannot select twice, including a competing payload
with a different fresh record ID. This is a sequential conditional-head theorem;
a storage implementation must refine the coordinator's atomic selection. -/
theorem competing_append_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (first second : Record P H)
    (availableFirst availableSecond : List ObjectId)
    (hfirst : append validate s stream expected first availableFirst = some t) :
    append validate t stream expected second availableSecond = none := by
  obtain ⟨ready, rfl⟩ := append_success validate s t stream expected first availableFirst hfirst
  apply stale_head_refuses
  rw [commit_currentHead]
  exact selected_head_changes validate s stream expected first availableFirst ready

/-- A successful append cannot change any exact value already bound to an ID. -/
theorem immutable_identity {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r old later : Record P H) (available : List ObjectId)
    (wf : WellFormed validate s)
    (h : append validate s stream expected r available = some t)
    (ho : old ∈ s.records) (hl : later ∈ t.records) (hid : old.id = later.id) :
    old = later := by
  have wt := append_preserves validate s t stream expected r available wf h
  exact wt.1 old (append_retains_exact_record validate s t stream expected r old available h ho)
    later hl hid

end Kctx.Engineering.Ledger
