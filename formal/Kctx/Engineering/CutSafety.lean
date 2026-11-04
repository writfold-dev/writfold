import Kctx.Engineering.HistorySafetyRetention

set_option autoImplicit false
namespace Kctx.Engineering

/-- The original portfolio interpretation and the complete exact manifest. This
persists after release, but readCut additionally requires an unreleased reader. -/
structure CutInterpretation (s : State) (id : Id .cut) (capturedContext : ContextCut)
    (capturedManifest : Ledger.Cut Action HistoricalPremises) : Prop where
  context : s.cuts.find? (·.id == id) = some capturedContext
  manifest : s.ledger.cuts.find? (·.id == id) = some capturedManifest
  closed : Ledger.CutClosed historicalValid s.ledger capturedManifest

theorem readCut_certificate {s : State} {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (h : readCut s id = some (context, manifest)) :
    s.ledger.releasedCuts.contains id = false ∧ CutInterpretation s id context manifest := by
  unfold readCut at h
  split at h
  · contradiction
  next active =>
    cases hc : s.cuts.find? (·.id == id) with
    | none => simp [hc] at h
    | some foundContext =>
        simp only [hc, bind, Option.bind] at h
        cases hm : s.ledger.cuts.find? (·.id == id) with
        | none => simp [hm] at h
        | some foundManifest =>
            simp only [hm, bind, Option.bind] at h
            split at h
            next closed =>
              cases h
              exact ⟨by simpa using active, ⟨hc, hm, closed⟩⟩
            next => contradiction

theorem cutInterpretation_read {s : State} {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (interpretation : CutInterpretation s id context manifest)
    (active : s.ledger.releasedCuts.contains id = false) :
    readCut s id = some (context, manifest) := by
  have activeMember : id ∉ s.ledger.releasedCuts := by
    intro member
    have impossible : false = true := active.symm.trans (List.contains_iff_mem.mpr member)
    cases impossible
  simp [readCut, activeMember, interpretation.context, interpretation.manifest, interpretation.closed]

theorem storage_preserves_cutInterpretation {contracts : Contracts} {s t : State}
    (evolution : StorageEvolution contracts s t) {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (old : CutInterpretation s id context manifest) : CutInterpretation t id context manifest := by
  cases evolution with
  | frame ledger contexts =>
      exact ⟨by simpa only [contexts] using old.context,
        by simpa only [ledger] using old.manifest, by simpa only [ledger] using old.closed⟩
  | append stream expected record action next _ appended contexts =>
      obtain ⟨_, same⟩ := Ledger.append_success historicalValid _ _ stream expected
        (recordFor contracts s stream record action) next.retained appended
      refine ⟨by simpa only [contexts] using old.context, ?_, ?_⟩
      · simpa only [same] using old.manifest
      · rw [same]
        exact Ledger.cutClosed_commit historicalValid _ _ _ _ _ old.closed
  | pin cut pinned contexts freshContext =>
      obtain ⟨ready, same⟩ := Ledger.pin_success historicalValid _ _ s.realm cut pinned
      have manifestId : manifest.id = id := by simpa using List.find?_some old.manifest
      have fresh : cut.id ≠ id := by
        intro eq
        exact ready.2.2.2.2.2 manifest (List.mem_of_find?_eq_some old.manifest) (manifestId.trans eq.symm)
      have freshBool : (cut.id == id) = false := by
        cases value : cut.id == id with
        | false => rfl
        | true => exact False.elim (fresh (eq_of_beq value))
      refine ⟨?_, ?_, ?_⟩
      · simp only [contexts, List.find?_cons]
        simpa only [freshBool] using old.context
      · simp only [same, List.find?_cons]
        simpa only [freshBool] using old.manifest
      · simpa only [same] using old.closed
  | release released releasedAt contexts =>
      unfold Ledger.releaseCut at releasedAt
      split at releasedAt
      · have same := (Option.some.inj releasedAt).symm
        exact ⟨by simpa only [contexts] using old.context,
          by simpa only [same] using old.manifest, by simpa only [same] using old.closed⟩
      · contradiction
  | publication rooted contexts =>
      unfold Ledger.retainRelease at rooted
      split at rooted
      · have same := (Option.some.inj rooted).symm
        exact ⟨by simpa only [contexts] using old.context,
          by simpa only [same] using old.manifest, by simpa only [same] using old.closed⟩
      · contradiction

theorem transition_preserves_cutInterpretation {contracts : Contracts} {s t : State}
    (transition : Transition contracts s t) {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (old : CutInterpretation s id context manifest) : CutInterpretation t id context manifest := by
  cases transition with
  | operational h => exact storage_preserves_cutInterpretation (step_storageEvolution h) old
  | external h =>
      obtain ⟨ledger, contexts, _⟩ := environment_storage_frame h
      exact storage_preserves_cutInterpretation (StorageEvolution.frame (contracts := contracts) ledger contexts) old
  | refused => exact old

theorem reachable_preserves_cutInterpretation {contracts : Contracts} {s t : State}
    (reachable : Reachable contracts s t) {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (old : CutInterpretation s id context manifest) : CutInterpretation t id context manifest := by
  induction reachable with
  | initial => exact old
  | next _ transition ih => exact transition_preserves_cutInterpretation transition ih

/-- E09: every surviving reader keeps the exact corpus AND activity meaning
through every Request and Environment case. Current HEAD is never consulted. -/
theorem E09_surviving_cut_exact {contracts : Contracts} {s t : State}
    (reachable : Reachable contracts s t) {id : Id .cut} {context : ContextCut}
    {manifest : Ledger.Cut Action HistoricalPremises}
    (read : readCut s id = some (context, manifest))
    (surviving : t.ledger.releasedCuts.contains id = false) :
    readCut t id = some (context, manifest) :=
  cutInterpretation_read (reachable_preserves_cutInterpretation reachable (readCut_certificate read).2) surviving

theorem E09_surviving_stream_exact {contracts : Contracts} {s t : State}
    (reachable : Reachable contracts s t) {id : Id .cut} {stream : StreamId}
    {entries : List (Ledger.Record Action HistoricalPremises)}
    (read : readStreamAtCut s id stream = some entries)
    (surviving : t.ledger.releasedCuts.contains id = false) :
    readStreamAtCut t id stream = some entries := by
  cases hr : readCut s id with
  | none => simp [readStreamAtCut, hr] at read
  | some result =>
      rcases result with ⟨context, manifest⟩
      have old := (readCut_certificate hr).2
      have next := reachable_preserves_cutInterpretation reachable old
      have exactRead := cutInterpretation_read next surviving
      have contents : (manifest.watermarks.find? (·.stream == stream)).map Ledger.Selection.entries = some entries := by
        simpa [readStreamAtCut, hr, Ledger.readPinned, old.manifest, old.closed] using read
      simpa [readStreamAtCut, exactRead, Ledger.readPinned, next.manifest, next.closed] using contents

/-- Projection updates only qualified observation fields, never operational
capabilities, budgets, provider state, corpus interpretation or selected heads. -/
theorem project_non_authority {contracts : Contracts} {s t : State} {pin : Ledger.NativePin}
    {heads : List Ledger.Head} {epoch : Nat} (h : project contracts s pin heads epoch = some t) :
    t = { s with activityPin := pin, projectedHeads := heads, projectionInventory := epoch } := by
  unfold project at h
  split at h
  · exact (Option.some.inj h).symm
  · contradiction

/-- Canonical ordering is complete over all selected streams, not just the
streams known to the caller or an ordinal watermark. -/
theorem acquireCut_certificate {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : acquireCut contracts s id = some t) :
    Ledger.CutReady historicalValid s.ledger s.realm (snapshotCut s id) ∧
    s.projectedHeads = Ledger.orderedHeads s.ledger ∧
    s.projectionInventory = s.ledger.inventoryEpoch ∧
    contracts.corpusBindings.contains (s.knowledge.kb.snapshot, s.corpusPin) = true ∧
    t.ledger = { s.ledger with cuts := snapshotCut s id :: s.ledger.cuts } ∧
    t.cuts = ⟨id, s.knowledge⟩ :: s.cuts := by
  unfold acquireCut at h
  split at h
  next guard =>
    have checks : contracts.corpusBindings.contains (s.knowledge.kb.snapshot, s.corpusPin) = true ∧
        s.projectedHeads = Ledger.orderedHeads s.ledger ∧
        s.projectionInventory = s.ledger.inventoryEpoch ∧ (!s.cuts.any (·.id == id)) = true := by
      simpa only [Bool.and_eq_true, beq_iff_eq, and_assoc] using guard
    change (Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id)).bind
      (fun ledger => some { s with ledger := ledger, cuts := ⟨id, s.knowledge⟩ :: s.cuts }) = some t at h
    cases hp : Ledger.pin historicalValid s.ledger s.realm (snapshotCut s id) with
    | none => simp [hp] at h
    | some ledger =>
        simp only [hp, bind, Option.bind, Option.some.injEq] at h
        subst t
        obtain ⟨ready, same⟩ := Ledger.pin_success historicalValid _ _ s.realm (snapshotCut s id) hp
        exact ⟨ready, checks.2.1, checks.2.2.1, checks.1, same, rfl⟩
  next => contradiction

theorem acquireCut_inventory_complete {contracts : Contracts} {s t : State} {id : Id .cut}
    (h : acquireCut contracts s id = some t) :
    (snapshotCut s id).inventory.Pairwise (fun a b => a.value ≤ b.value) ∧
    (∀ stream, stream ∈ (snapshotCut s id).inventory ↔
      ∃ selection ∈ s.ledger.selections, selection.stream = stream) ∧
    Ledger.NativePinsValid (snapshotCut s id) ∧
    (∀ object ∈ Ledger.cutRoots (snapshotCut s id), object ∈ s.ledger.material) := by
  have ready := (acquireCut_certificate h).1
  exact ⟨Ledger.orderedInventory_sorted s.ledger, Ledger.orderedInventory_mem s.ledger,
    ready.2.2.2.1.1.2, ready.2.2.2.1.2.2⟩

theorem released_cut_requires_refusal (s : State) (id : Id .cut)
    (released : s.ledger.releasedCuts.contains id = true) : readCut s id = none := by
  have releasedMember : id ∈ s.ledger.releasedCuts := List.contains_iff_mem.mp released
  simp [readCut, releasedMember]

theorem missing_cut_requires_refusal (s : State) (id : Id .cut)
    (missing : s.ledger.cuts.find? (·.id == id) = none) : readCut s id = none := by
  unfold readCut
  split
  · rfl
  · cases hc : s.cuts.find? (·.id == id) <;> simp [hc, missing]

theorem lagging_projection_requires_refusal (contracts : Contracts) (s : State) (id : Id .cut)
    (lagging : s.projectedHeads ≠ Ledger.orderedHeads s.ledger) : acquireCut contracts s id = none := by
  simp [acquireCut, lagging]

theorem stale_inventory_requires_refusal (contracts : Contracts) (s : State) (id : Id .cut)
    (stale : s.projectionInventory ≠ s.ledger.inventoryEpoch) : acquireCut contracts s id = none := by
  simp [acquireCut, stale]

theorem released_stream_requires_refusal (s : State) (id : Id .cut) (stream : StreamId)
    (released : s.ledger.releasedCuts.contains id = true) : readStreamAtCut s id stream = none := by
  simp [readStreamAtCut, released_cut_requires_refusal s id released]

theorem missing_watermark_has_no_head_fallback {s : State} {id : Id .cut}
    {stream : StreamId} {context : ContextCut} {manifest : Ledger.Cut Action HistoricalPremises}
    (read : readCut s id = some (context, manifest))
    (missing : manifest.watermarks.find? (·.stream == stream) = none) :
    readStreamAtCut s id stream = none := by
  have interpretation := (readCut_certificate read).2
  simp [readStreamAtCut, read, Ledger.readPinned, interpretation.manifest, interpretation.closed, missing]

theorem missing_native_root_requires_refusal (contracts : Contracts) (s : State) (id : Id .cut)
    (object : ObjectId) (root : object ∈ Ledger.cutRoots (snapshotCut s id))
    (missing : object ∉ s.ledger.material) : acquireCut contracts s id = none := by
  cases h : acquireCut contracts s id with
  | none => rfl
  | some t =>
      exact False.elim (missing ((acquireCut_certificate h).1.2.2.2.1.2.2 object root))

theorem wrong_activity_family_requires_refusal (contracts : Contracts) (s : State) (id : Id .cut)
    (wrong : s.activityPin.family ≠ .activity) : acquireCut contracts s id = none := by
  cases h : acquireCut contracts s id with
  | none => rfl
  | some t => exact False.elim (wrong (acquireCut_certificate h).1.2.2.2.1.1.2.2.1)

end Kctx.Engineering
