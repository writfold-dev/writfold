import Writfold.Conform.Words
import Writfold.Conform.Links
import Writfold.Catalog
import Writfold.Checklist
import Writfold.Units
import Writfold.Inventory
import Writfold.Packet
import Writfold.Runs
import Writfold.Graph
import Writfold.Walk
import Calculus.Account

/-!
# Links: the universal words (#211, law U)

Every shipped word that claims all of something (`complete`, `covered`,
`clean`, "nothing affected") is shown to be `Calculus.whole`,
`Calculus.universal` or the absence side of `Calculus.search` over an
account, each derived from the kernel's existing spec theorem. The laws
that were proved per kernel (`truncated_never_complete`,
`no_manifest_incomplete`, `partial_never_patches`, ...) then hold of each
kernel by transfer from `remainder_never_whole`, `empty_never_yes` and
`whole_counts`; the per-kernel theorems stay as they were.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-! ## A universe with nothing left over -/

theorem mem_judgedOf (ms : List (Calculus.Member × Calculus.Tri)) (t : Calculus.Tri) :
    t ∈ Calculus.judgedOf ms ↔ (Calculus.Member.judged, t) ∈ ms := by
  unfold Calculus.judgedOf
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨⟨m, t'⟩, hm, h⟩
    by_cases hj : m = .judged
    · subst hj; simp at h; subst h; exact hm
    · simp [hj] at h
  · intro h; exact ⟨_, h, by simp⟩

/-- Over a universe in which every member was judged or declared out and the
empty universe is allowed, the universal word holds exactly when every
judged member holds. -/
theorem universal_allow_yes (ms : List (Calculus.Member × Calculus.Tri))
    (hw : ∀ m ∈ ms, m.1 ≠ .remainder) :
    Calculus.universal true ms = .yes ↔ ∀ t, (Calculus.Member.judged, t) ∈ ms → t = .yes := by
  rw [Calculus.universal_yes_iff]
  have hwhole : Calculus.whole (ms.map (·.1)) = true := by
    rw [Calculus.whole_iff_no_remainder]
    intro h
    obtain ⟨m, hm, he⟩ := List.mem_map.mp h
    exact hw m hm he
  simp only [hwhole, true_or, and_true, true_and]
  constructor
  · intro h t ht; exact h t ((mem_judgedOf ms t).mpr ht)
  · intro h t ht; exact h t ((mem_judgedOf ms t).mp ht)

/-! ## Catalogs: a plan is clean only over the whole input (#84) -/

/-- A catalog plan's verdict: clean is `yes`, drift (some slot would change)
`no`, invalid (input read in part, or a conflict) `unknown`. -/
def catalogTri : catalog.Verdict → Calculus.Tri
  | .Clean => .yes
  | .Drift => .no
  | .Invalid => .unknown

theorem catalogTri_onto (t : Calculus.Tri) : ∃ v, catalogTri v = t := by
  cases t
  · exact ⟨.Clean, rfl⟩
  · exact ⟨.Drift, rfl⟩
  · exact ⟨.Invalid, rfl⟩

/-- The catalog's account: its input as one member, judged when it was read
in full with no slot in conflict and a remainder otherwise; it holds unless
some slot would change. -/
def catalogAccount (complete conflict changed : Bool) : List (Calculus.Member × Calculus.Tri) :=
  [(if complete && !conflict then .judged else .remainder, if changed then .no else .yes)]

theorem catalog_table (c k ch : Bool) :
    catalogTri (Core.Catalog.verdictOf c k ch) = Calculus.universal true (catalogAccount c k ch) := by
  cases c <;> cases k <;> cases ch <;> decide

/-- `catalog::verdict` is `Calculus.universal` over the catalog's account. -/
theorem catalog_verdict_implements (c k ch : Bool) :
    catalog.verdict c k ch ⦃ v =>
      catalogTri v = Calculus.universal true (catalogAccount c k ch) ⦄ := by
  apply WP.spec_mono (Core.Catalog.verdict_spec c k ch)
  intro v hv; rw [hv]; exact catalog_table c k ch

theorem decide_any {α : Type} (l : List α) (p : α → Bool) :
    decide (∃ s ∈ l, p s = true) = l.any p := by
  cases h : l.any p <;> simp_all [List.any_eq_true, List.any_eq_false]

/-- `catalog::plan` is `Calculus.universal` over the account of its slots. -/
theorem catalog_plan_implements (c : Bool) (xs : Slice catalog.Slot) :
    catalog.plan c xs ⦃ v =>
      catalogTri v = Calculus.universal true (catalogAccount c
        (xs.val.any Core.Catalog.conflictB) (xs.val.any Core.Catalog.changeB)) ⦄ := by
  apply WP.spec_mono (Core.Catalog.plan_spec c xs)
  intro v hv
  rw [hv, catalog_table, decide_any, decide_any]

/-- Law U at the catalog, transferred from `Calculus.remainder_never_whole`:
an input read in part never plans clean. -/
theorem catalog_partial_never_clean (xs : Slice catalog.Slot) :
    catalog.plan false xs ⦃ v => v ≠ .Clean ⦄ := by
  apply WP.spec_mono (catalog_plan_implements false xs)
  intro v hv hc
  subst hc
  exact Calculus.remainder_never_whole true _ (by simp [catalogAccount]) hv.symm

/-! ## Checklists: covered is universal over the required items -/

/-- A checklist entry as a member: a required item is judged, `yes` when it
is covered under its policy; an optional item is declared out. -/
def entryMember (e : checklist.Entry) : Calculus.Member × Calculus.Tri :=
  if e.required then (.judged, if Core.Checklist.coveredE e then .yes else .no)
  else (.declaredOut, .yes)

/-- `checklist::rollup_under`'s `covered` is `Calculus.universal` over the
entries, optional ones declared out. -/
theorem rollup_under_implements (es : Slice checklist.Entry) :
    checklist.rollup_under es ⦃ r =>
      r.covered = (Calculus.universal true (es.val.map entryMember) == .yes) ⦄ := by
  apply WP.spec_mono (Core.Checklist.rollup_covered_iff es)
  intro r hr
  have hw : ∀ m ∈ es.val.map entryMember, m.1 ≠ .remainder := by
    intro m hm
    obtain ⟨e, _, rfl⟩ := List.mem_map.mp hm
    unfold entryMember; split <;> simp
  rw [Bool.eq_iff_iff, hr, beq_iff_eq, universal_allow_yes _ hw]
  constructor
  · intro h t ht
    obtain ⟨e, he, hm⟩ := List.mem_map.mp ht
    unfold entryMember at hm
    split at hm
    · rename_i hreq
      simp only [Prod.mk.injEq, true_and] at hm
      rw [← hm, h e he hreq]; rfl
    · simp at hm
  · intro h e he hreq
    have := h (if Core.Checklist.coveredE e then .yes else .no)
      (List.mem_map.mpr ⟨e, he, by simp [entryMember, hreq]⟩)
    cases hc : Core.Checklist.coveredE e <;> simp_all

/-! ## Generated checklists: an absent manifest is a remainder (#165) -/

/-- A generated checklist's account: an absent universe is a remainder, each
unit a judged member, and the units' roll-up a failing member when it is not
covered. -/
def generatedAccount (present : Bool) (count : Nat) (covered : Bool) :
    List (Calculus.Member × Calculus.Tri) :=
  (if present then [] else [(.remainder, .unknown)]) ++
    List.replicate count (.judged, .yes) ++ (if covered then [] else [(.judged, .no)])

theorem judgedOf_append (xs ys : List (Calculus.Member × Calculus.Tri)) :
    Calculus.judgedOf (xs ++ ys) = Calculus.judgedOf xs ++ Calculus.judgedOf ys := by
  simp [Calculus.judgedOf, List.filterMap_append]

theorem judgedOf_replicate (n : Nat) (t : Calculus.Tri) :
    Calculus.judgedOf (List.replicate n (.judged, t)) = List.replicate n t := by
  induction n with
  | zero => rfl
  | succ n ih => simp_all [List.replicate_succ, Calculus.judgedOf]

theorem generated_table (present allow covered : Bool) (count : Nat) :
    Calculus.universal allow (generatedAccount present count covered) = .yes ↔
      present = true ∧ (0 < count ∨ allow = true) ∧ covered = true := by
  rw [Calculus.universal_yes_iff]
  simp only [generatedAccount, judgedOf_append, judgedOf_replicate, List.map_append,
    Calculus.whole_append, Bool.and_eq_true]
  cases present <;> cases covered <;> cases allow <;>
    simp [Calculus.whole, Calculus.remainder, Calculus.judgedOf, List.countP_replicate,
      List.mem_replicate] <;> omega

/-- `units::generated`'s `covered` is `Calculus.universal` with the
checklist's `allow_empty`. -/
theorem generated_implements (present : Bool) (count : Usize) (allow : Bool)
    (r : checklist.Rollup) :
    units.generated present count allow r ⦃ o =>
      o.covered = (Calculus.universal allow
        (generatedAccount present count.val r.covered) == .yes) ⦄ := by
  apply WP.spec_mono (Core.Units.generated_covered_iff present count allow r)
  intro o ⟨h, _⟩
  rw [Bool.eq_iff_iff, h, beq_iff_eq, generated_table]

/-- Law U at a generated checklist, transferred from
`Calculus.remainder_never_whole`: an absent universe is never covered. -/
theorem generated_absent_never_covered (count : Usize) (allow : Bool) (r : checklist.Rollup) :
    units.generated false count allow r ⦃ o => o.covered = false ⦄ := by
  apply WP.spec_mono (generated_implements false count allow r)
  intro o ho
  rw [ho]
  have := Calculus.remainder_never_whole allow (generatedAccount false count.val r.covered)
    (by simp [generatedAccount])
  simpa using this

/-- 3.5 at a generated checklist, transferred from `Calculus.empty_never_yes`:
an empty universe is covered only when allowed. -/
theorem generated_empty_needs_allow (present : Bool) (r : checklist.Rollup) :
    units.generated present 0#usize false r ⦃ o => o.covered = false ⦄ := by
  apply WP.spec_mono (generated_implements present 0#usize false r)
  intro o ho
  rw [ho]
  by_cases hc : r.covered = true
  · have := Calculus.empty_never_yes (generatedAccount present (0#usize).val r.covered)
      (by cases present <;> simp [generatedAccount, hc, Calculus.judgedOf])
    simpa using this
  · have : Calculus.universal false (generatedAccount present (0#usize).val r.covered) ≠ .yes := by
      intro h; exact hc ((generated_table _ _ _ _).mp h).2.2
    simpa using this

/-! ## The universe of a generated checklist (#165) -/

/-- What a scope's universe is read from: the manifest's members are judged
only when it was read, is a manifest, and says it is complete. -/
def scopeMembers (f : units.Scope) : List Calculus.Member :=
  [f.reads, f.manifest, f.complete].map fun b => if b then .judged else .remainder

/-- `units::universe` gives a universe only when its manifest is
`Calculus.whole` (the predicate form: the other facts refuse too). -/
theorem universe_implements (f : units.Scope) :
    units.universe f ⦃ r => r = .Ok () → Calculus.whole (scopeMembers f) = true ⦄ := by
  apply WP.spec_mono (Core.Units.universe_ok_iff f)
  intro r hr hok
  obtain ⟨_, _, hrd, hm, hc, _⟩ := hr.mp hok
  simp [scopeMembers, hrd, hm, hc, Calculus.whole, Calculus.remainder]

/-- Law U at the universe, transferred from `Calculus.whole_iff_no_remainder`:
a manifest that says `partial` gives none. -/
theorem universe_partial_never_present (f : units.Scope) (h : f.complete = false) :
    units.universe f ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (universe_implements f)
  intro r hr hok
  have := (Calculus.whole_iff_no_remainder _).mp (hr hok)
  exact this (by simp [scopeMembers, h])

/-! ## Inventories, packets, runs, walks -/

/-- `inventory::unreadable` counts a problem exactly when the inventory is
not `Calculus.whole`. -/
theorem unreadable_implements (roots : Slice inventory.Root) :
    inventory.unreadable roots ⦃ n =>
      0 < n.val ↔ Calculus.whole (roots.val.map rootMember) = false ⦄ := by
  apply WP.spec_mono (Core.Inventory.unreadable_pos_iff roots)
  intro n hn
  rw [hn, Bool.eq_false_iff, ne_eq, Calculus.whole_iff_no_remainder, not_not]
  constructor
  · rintro ⟨r, hr, rfl⟩; exact List.mem_map.mpr ⟨_, hr, rfl⟩
  · intro hm
    obtain ⟨r, hr, he⟩ := List.mem_map.mp hm
    cases r <;> simp_all [rootMember]

/-- A list of met facts as judged members. -/
def metMember (b : Bool) : Calculus.Member × Calculus.Tri := (.judged, if b then .yes else .no)

/-- `inventory::all_true` is `Calculus.universal`, the empty list allowed
(its callers require a member: `unplanned_clean`). -/
theorem all_true_implements (met : Slice Bool) :
    inventory.all_true met ⦃ b =>
      b = (Calculus.universal true (met.val.map metMember) == .yes) ⦄ := by
  apply WP.spec_mono (Core.Inventory.all_true_spec met)
  intro b hb
  have hw : ∀ m ∈ met.val.map metMember, m.1 ≠ .remainder := by
    intro m hm; obtain ⟨_, _, rfl⟩ := List.mem_map.mp hm; simp [metMember]
  rw [Bool.eq_iff_iff, hb, beq_iff_eq, universal_allow_yes _ hw]
  constructor
  · intro h t ht
    obtain ⟨x, hx, he⟩ := List.mem_map.mp ht
    simp only [metMember, Prod.mk.injEq, true_and] at he
    rw [← he, h x hx]; rfl
  · intro h x hx
    have := h _ (List.mem_map.mpr ⟨x, hx, rfl⟩)
    cases x <;> simp_all

/-- `packet::required_omitted` is `Calculus.remainder` of the required rows. -/
theorem required_omitted_implements (kept required : Usize) :
    packet.required_omitted kept required ⦃ n =>
      n.val = Calculus.remainder (Calculus.Member.counts kept.val 0 (required.val - kept.val)) ⦄ := by
  apply WP.spec_mono (Core.Packet.required_omitted_spec kept required)
  intro n hn
  rw [hn, Calculus.remainder_counts]
  omega

/-- Law U at the packet, transferred from `Calculus.remainder_counts`: none is
reported omitted exactly when every required row was kept. -/
theorem packet_omitted_counted (kept required : Usize) :
    packet.required_omitted kept required ⦃ n =>
      n.val = 0 ↔ required.val ≤ kept.val ⦄ := by
  apply WP.spec_mono (required_omitted_implements kept required)
  intro n hn
  rw [hn, Calculus.remainder_counts]
  omega

/-- Law U at the run list, transferred from `Calculus.whole_counts`. -/
theorem runs_no_manifest_incomplete (u n : Usize) (h : 0 < n.val) :
    runs.complete u n ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (runs_complete_implements u n)
  intro b hb
  rw [hb, Bool.eq_false_iff, ne_eq, Calculus.whole_counts]
  omega

/-- Law U at the run list, transferred from `Calculus.whole_counts`. -/
theorem runs_unreadable_incomplete (u n : Usize) (h : 0 < u.val) :
    runs.complete u n ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (runs_complete_implements u n)
  intro b hb
  rw [hb, Bool.eq_false_iff, ne_eq, Calculus.whole_counts]
  omega

/-- Law U at the bounded walk (#60), transferred from `Calculus.whole_counts`:
a work limit is a remainder. -/
theorem walk_truncated_never_complete (frontier skipped : Usize) :
    walk.complete_within frontier skipped true ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (walk_complete_within_implements frontier skipped true)
  intro b hb
  rw [hb, Bool.eq_false_iff, ne_eq, Calculus.whole_counts]
  simp

/-! ## The graph: "nothing affected" is an absence over every file (#65) -/

/-- `graph::plan` skips exactly when `Calculus.search` over every file says
no file is affected, and takes a delta exactly when it finds one. -/
theorem graph_plan_implements (xs : Slice Bool) :
    graph.plan xs ⦃ p =>
      (p = .Skip ↔ Calculus.search true (decide (true ∈ xs.val)) = .no) ∧
      (p = .Delta ↔ Calculus.search true (decide (true ∈ xs.val)) = .yes) ⦄ := by
  apply WP.spec_mono (Core.Graph.plan_delta_iff xs)
  intro p hp
  by_cases h : true ∈ xs.val
  · cases p <;> simp_all [Calculus.search]
  · cases p <;> simp_all [Calculus.search]

end Writfold.Core.Conform
