import Kctx.Engineering.LedgerCuts

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

theorem orderedSelections_mem {P H : Type} (s : Store P H) (selection : Selection P H) :
    selection ∈ orderedSelections s ↔ selection ∈ s.selections := by
  exact List.mem_mergeSort

theorem orderedSelections_sorted {P H : Type} (s : Store P H) :
    (orderedSelections s).Pairwise (fun a b => a.stream.value ≤ b.stream.value) := by
  have trans : ∀ a b c : Selection P H,
      decide (a.stream.value ≤ b.stream.value) →
      decide (b.stream.value ≤ c.stream.value) →
      decide (a.stream.value ≤ c.stream.value) := by
    intro a b c hab hbc
    exact decide_eq_true (Nat.le_trans (of_decide_eq_true hab) (of_decide_eq_true hbc))
  have total : ∀ a b : Selection P H,
      decide (a.stream.value ≤ b.stream.value) || decide (b.stream.value ≤ a.stream.value) := by
    intro a b
    rcases Nat.le_total a.stream.value b.stream.value with h | h <;> simp [h]
  simpa only [orderedSelections, decide_eq_true_eq] using
    (List.sorted_mergeSort trans total s.selections)

theorem orderedInventory_mem {P H : Type} (s : Store P H) (stream : StreamId) :
    stream ∈ orderedInventory s ↔ ∃ selection ∈ s.selections, selection.stream = stream := by
  simp only [orderedInventory, List.mem_map, orderedSelections_mem]

theorem orderedInventory_sorted {P H : Type} (s : Store P H) :
    (orderedInventory s).Pairwise (fun a b => a.value ≤ b.value) := by
  exact List.pairwise_map.mpr (orderedSelections_sorted s)

theorem pin_success {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (h : pin validate s realm cut = some t) :
    CutReady validate s realm cut ∧ t = { s with cuts := cut :: s.cuts } := by
  unfold pin at h
  split at h
  · exact ⟨‹_›, (Option.some.inj h).symm⟩
  · contradiction

theorem pin_preserves_heads_records {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (h : pin validate s realm cut = some t) :
    t.selections = s.selections ∧ t.records = s.records ∧ t.material = s.material := by
  obtain ⟨_, rfl⟩ := pin_success validate s t realm cut h
  exact ⟨rfl, rfl, rfl⟩

theorem pin_preserves_wellFormed {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (wf : WellFormed validate s) (h : pin validate s realm cut = some t) :
    WellFormed validate t := by
  obtain ⟨_, rfl⟩ := pin_success validate s t realm cut h
  exact wf

theorem pin_preserves_evidence {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (evidence : EvidenceClosed validate s) (h : pin validate s realm cut = some t) :
    EvidenceClosed validate t := by
  obtain ⟨ready, rfl⟩ := pin_success validate s t realm cut h
  refine ⟨?_, evidence.2⟩
  intro c hc
  simp only [List.mem_cons] at hc
  rcases hc with rfl | hc
  · exact ready.2.2.2.1
  · exact evidence.1 c hc

theorem pin_captures_complete_inventory {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (h : pin validate s realm cut = some t) :
    cut.inventoryEpoch = s.inventoryEpoch ∧
    cut.inventory = orderedInventory s ∧
    cut.watermarks.map Selection.stream = cut.inventory := by
  have ready := (pin_success validate s t realm cut h).1
  exact ⟨ready.2.1, ready.2.2.1, ready.2.2.2.1.1.1⟩

theorem pin_preserves_old_read {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (cutId : Id .cut) (stream : StreamId) (entries : List (Record P H))
    (hp : pin validate s realm cut = some t)
    (hr : readPinned validate s cutId stream = some entries) :
    readPinned validate t cutId stream = some entries := by
  obtain ⟨ready, rfl⟩ := pin_success validate s t realm cut hp
  cases hc : s.cuts.find? (fun c => c.id == cutId) with
  | none => simp [readPinned, hc] at hr
  | some old =>
      have oldmem := List.mem_of_find?_eq_some hc
      have oldid : old.id = cutId := by simpa using List.find?_some hc
      have fresh : cut.id ≠ cutId := by
        intro eqid
        exact ready.2.2.2.2.2 old oldmem (oldid.trans eqid.symm)
      have skip : (cut.id == cutId) = false := beq_eq_false_iff_ne.mpr fresh
      have lookup : (({ s with cuts := cut :: s.cuts } : Store P H).cuts.find?
          (fun c => c.id == cutId)) = some old := by
        simpa only [List.find?_cons, skip] using hc
      simpa only [readPinned, lookup, hc] using hr

theorem release_preserves_evidence {P H : Type} (validate : Validator P H)
    (s t : Store P H) (cutId : Id .cut) (evidence : EvidenceClosed validate s)
    (h : releaseCut s cutId = some t) :
    EvidenceClosed validate t ∧ t.cuts = s.cuts ∧ t.material = s.material := by
  unfold releaseCut at h
  split at h
  · cases Option.some.inj h
    exact ⟨evidence, rfl, rfl⟩
  · contradiction

theorem release_preserves_read {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (released cutId : Id .cut)
    (stream : StreamId) (h : releaseCut s released = some t) :
    readPinned validate t cutId stream = readPinned validate s cutId stream := by
  unfold releaseCut at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem release_roots_closed {P H : Type} (validate : Validator P H)
    (s t : Store P H) (roots : List ObjectId) (evidence : EvidenceClosed validate s)
    (h : retainRelease s roots = some t) : EvidenceClosed validate t := by
  unfold retainRelease at h
  split at h
  · rename_i retained
    cases Option.some.inj h
    refine ⟨evidence.1, ?_⟩
    intro object ho
    simp only [List.mem_append] at ho
    exact ho.elim (retained object) (evidence.2 object)
  · contradiction

/-- Selection of another stream has an exact frame; a shared coordinator
ordinal is neither consulted nor installed as the stream's record identity. -/
theorem commit_other_stream {P H : Type} (s : Store P H) (stream other : StreamId)
    (r : Record P H) (available : List ObjectId) (hne : stream ≠ other) :
    selected (commit s stream r available) other = selected s other := by
  have skip : (stream == other) = false := beq_eq_false_iff_ne.mpr hne
  simp only [selected, commit, select, retain, List.find?_cons, skip]
  rw [List.find?_filter]
  congr 1
  funext old
  by_cases h : old.stream = other
  · simp [h, Ne.symm hne]
  · simp [h]

theorem pin_native_pins_valid {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (realm : Id .realm) (cut : Cut P H)
    (h : pin validate s realm cut = some t) :
    cut.realm = realm ∧ NativePinsValid cut := by
  have ready := (pin_success validate s t realm cut h).1
  exact ⟨ready.1, ready.2.2.2.1.1.2⟩

theorem wrong_native_family_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (realm : Id .realm) (cut : Cut P H)
    (bad : cut.activityPin.family ≠ .activity) : pin validate s realm cut = none := by
  unfold pin
  split
  · exact False.elim (bad ‹CutReady validate s realm cut›.2.2.2.1.1.2.2.1)
  · rfl

end Kctx.Engineering.Ledger
