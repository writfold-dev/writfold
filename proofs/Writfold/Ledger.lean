import Writfold.Bytes

/-!
# Validated ledger history (E05)

Theorems over the Aeneas translation of `writfold_core::ledger`, stated
against spec predicates that mirror the calculus's `historyCheck` and
`AppendReady` (`formal/Kctx/Engineering/LedgerHistory.lean`,
`LedgerAppend.lean`). The validator is any trait instance; the only
assumption about it is that it terminates (`Total`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core

variable {P V : Type}

/-- The validator never fails or diverges: it returns some verdict. -/
def Total (inst : ledger.Validator V P) (v : V) : Prop :=
  ∀ h u e, ∃ b, inst.validate v h u e = ok b

/-- An entry's link: the first names no predecessor, every later one names
exactly the id of the entry before it. -/
def Linked (entries : List (ledger.Entry P)) (i : Nat) (e : ledger.Entry P) : Prop :=
  if i = 0 then e.predecessor = none
  else ∃ p prev, e.predecessor = some p ∧ entries[i - 1]? = some prev ∧ p.val = prev.id.val

/-- Lean `historyCheck`'s per-entry conjunct: on the stream, numbered
`i + 1`, linked, and valid against exactly the entries before it. -/
def EntryOk (inst : ledger.Validator V P) (v : V) (stream : Slice U8)
    (entries : Slice (ledger.Entry P)) (i : Usize) : Prop :=
  ∃ e, entries.val[i.val]? = some e ∧ e.stream.val = stream.val ∧
    e.sequence.val = i.val + 1 ∧ Linked entries.val i.val e ∧
    inst.validate v entries i e = ok true

/-- Lean `historyCheck`. -/
def HistoryOk (inst : ledger.Validator V P) (v : V) (stream : Slice U8)
    (entries : Slice (ledger.Entry P)) : Prop :=
  ∀ i : Usize, i.val < entries.val.length → EntryOk inst v stream entries i

theorem entry_ok_spec (inst : ledger.Validator V P) (v : V) (htotal : Total inst v)
    (stream : Slice U8) (entries : Slice (ledger.Entry P)) (i : Usize)
    (hi : i.val < entries.val.length) :
    ledger.entry_ok inst v stream entries i ⦃ r => r ↔ EntryOk inst v stream entries i ⦄ := by
  unfold ledger.entry_ok
  step as ⟨ e, he ⟩
  step with same_spec as ⟨ b, hb ⟩
  have hget : entries.val[i.val]? = some e := by rw [List.getElem?_eq_getElem hi, he]
  split
  · rename_i hs
    apply WP.spec_bind (Usize.add_spec (by scalar_tac))
    intro n hn
    split
    · rename_i hseq
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨e', he', _, hsq, _⟩
      rw [hget] at he'; cases he'
      rw [bne_iff_ne] at hseq; apply hseq
      apply UScalar.eq_of_val_eq; scalar_tac
    · rename_i hseq
      rw [bne_iff_ne, not_not] at hseq
      have hsq : e.sequence.val = i.val + 1 := by rw [hseq]; scalar_tac
      have hst : e.stream.val = stream.val := by
        simpa [alloc.vec.Vec.deref, Slice.from_val] using hb.mp hs
      have hentry : ({ e with predecessor := e.predecessor } : ledger.Entry P) = e := by
        cases e; rfl
      by_cases h0 : i = 0#usize
      · have h0' : i.val = 0 := by rw [h0]; rfl
        rw [if_pos h0]
        step with same_option_spec as ⟨ l, hl ⟩
        rw [hentry]
        split
        · rename_i hlk
          obtain ⟨b', hb'⟩ := htotal entries i e
          rw [hb']
          simp only [WP.spec_ok]
          constructor
          · intro hbt; subst hbt
            exact ⟨e, hget, hst, hsq, by simp [Linked, h0', hl.mp hlk], hb'⟩
          · rintro ⟨e', he', _, _, _, hv⟩
            rw [hget] at he'; cases he'; rw [hb'] at hv; simpa using hv
        · rename_i hlk
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
          rintro ⟨e', he', _, _, hlink, _⟩
          rw [hget] at he'; cases he'
          simp only [Linked, h0', ite_true] at hlink
          exact hlk (hl.mpr hlink)
      · have h0' : i.val ≠ 0 := by
          intro h; apply h0; apply UScalar.eq_of_val_eq; simpa using h
        rw [if_neg h0]
        cases hp : e.predecessor with
        | none =>
          dsimp only
          simp
          rintro ⟨e', he', _, _, hlink, _⟩
          rw [hget] at he'; cases he'
          simp [Linked, h0', hp] at hlink
        | some p =>
          dsimp only
          step as ⟨ j, hj ⟩
          step as ⟨ prev, hprev ⟩
          step with same_spec as ⟨ l, hl ⟩
          have hprev' : entries.val[i.val - 1]? = some prev := by
            rw [List.getElem?_eq_getElem (by scalar_tac), hprev]; congr 2; scalar_tac
          have hent : ({ e with predecessor := some p } : ledger.Entry P) = e := by
            cases e; simp_all
          rw [hent]
          split
          · rename_i hlk
            obtain ⟨b', hb'⟩ := htotal entries i e
            rw [hb']
            simp only [WP.spec_ok]
            constructor
            · intro hbt; subst hbt
              refine ⟨e, hget, hst, hsq, ?_, hb'⟩
              simp only [Linked, h0', ite_false]
              exact ⟨p, prev, hp, hprev',
                by simpa [alloc.vec.Vec.deref, Slice.from_val] using hl.mp hlk⟩
            · rintro ⟨e', he', _, _, _, hv⟩
              rw [hget] at he'; cases he'; rw [hb'] at hv; simpa using hv
          · rename_i hlk
            simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
            rintro ⟨e', he', _, _, hlink, _⟩
            rw [hget] at he'; cases he'
            simp only [Linked, h0', ite_false] at hlink
            obtain ⟨p', prev', hp', hprev'', hpv⟩ := hlink
            rw [hp] at hp'; cases hp'
            rw [hprev'] at hprev''; cases hprev''
            exact hlk (hl.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using hpv))

  · rename_i hs
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    rintro ⟨e', he', hst, _⟩
    rw [hget] at he'; cases he'
    exact hs (hb.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using hst))

theorem history_check_loop_spec (inst : ledger.Validator V P) (v : V) (htotal : Total inst v)
    (stream : Slice U8) (entries : Slice (ledger.Entry P)) (i : Usize)
    (hi : i.val ≤ entries.val.length)
    (hdone : ∀ j : Usize, j.val < i.val → EntryOk inst v stream entries j) :
    ledger.history_check_loop inst v stream entries i ⦃ r => r ↔ HistoryOk inst v stream entries ⦄ := by
  unfold ledger.history_check_loop
  apply loop.spec_decr_nat (fun i : Usize => entries.val.length - i.val)
    (fun i : Usize => i.val ≤ entries.val.length ∧
      ∀ j : Usize, j.val < i.val → EntryOk inst v stream entries j)
  · intro i ⟨hi, hdone⟩
    unfold ledger.history_check_loop.body
    dsimp only
    split
    · rename_i hlt
      step with entry_ok_spec inst v htotal stream entries i (by scalar_tac) as ⟨ b, hb ⟩
      split
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro j hj
        by_cases hji : j.val < i.val
        · exact hdone j hji
        · have : j = i := by apply UScalar.eq_of_val_eq; scalar_tac
          subst this
          exact hb.mp hbt
      · rename_i hbf
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        intro hall
        exact hbf (hb.mpr (hall i (by scalar_tac)))
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro j hj
      exact hdone j (by scalar_tac)
  · exact ⟨hi, hdone⟩

/-- `history_check` is Lean `historyCheck`: every entry is on the stream,
numbered from 1, linked to its predecessor and valid against the entries
before it. -/
theorem history_check_spec (inst : ledger.Validator V P) (v : V) (htotal : Total inst v)
    (stream : Slice U8) (entries : Slice (ledger.Entry P)) :
    ledger.history_check inst v stream entries ⦃ r => r ↔ HistoryOk inst v stream entries ⦄ := by
  unfold ledger.history_check
  apply history_check_loop_spec inst v htotal
  · simp
  · intro j hj; simp at hj

/-- Some entry of this selection has the id. -/
def ContainsId (entries : Slice (ledger.Entry P)) (id : Slice U8) : Prop :=
  ∃ e ∈ entries.val, e.id.val = id.val

theorem entries_contain_loop_spec (entries : Slice (ledger.Entry P)) (id : Slice U8) (j : Usize)
    (hj : j.val ≤ entries.val.length)
    (hmiss : ∀ k, k < j.val → ∀ e, entries.val[k]? = some e → e.id.val ≠ id.val) :
    ledger.entries_contain_loop entries id j ⦃ r => r ↔ ContainsId entries id ⦄ := by
  unfold ledger.entries_contain_loop
  apply loop.spec_decr_nat (fun j : Usize => entries.val.length - j.val)
    (fun j : Usize => j.val ≤ entries.val.length ∧
      ∀ k, k < j.val → ∀ e, entries.val[k]? = some e → e.id.val ≠ id.val)
  · intro j ⟨hj, hmiss⟩
    unfold ledger.entries_contain_loop.body
    dsimp only
    split
    · step as ⟨ e, he ⟩
      step with same_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨e, ?_, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hb.mp hbt⟩
        rw [he]; exact List.getElem_mem _
      · rename_i hbf
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro j' hj'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk e' he'
        by_cases hkj : k < j.val
        · exact hmiss k hkj e' he'
        · have : k = j.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at he'
          simp only [Option.some.injEq] at he'
          rw [← he', ← he]
          intro heq
          exact hbf (hb.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using heq))
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨e, hmem, hid⟩
      obtain ⟨k, hk, hke⟩ := List.getElem_of_mem hmem
      exact hmiss k (by scalar_tac) e (by rw [List.getElem?_eq_getElem hk, hke]) hid
  · exact ⟨hj, hmiss⟩

theorem entries_contain_spec (entries : Slice (ledger.Entry P)) (id : Slice U8) :
    ledger.entries_contain entries id ⦃ r => r ↔ ContainsId entries id ⦄ := by
  unfold ledger.entries_contain
  apply entries_contain_loop_spec
  · simp
  · intro k hk; simp at hk

/-- Some selection, on any stream, holds a record with the id. -/
def RetainsId (selections : Slice (ledger.Selection P)) (id : Slice U8) : Prop :=
  ∃ s ∈ selections.val, ∃ e ∈ s.entries.val, e.id.val = id.val

theorem retains_record_loop_spec (selections : Slice (ledger.Selection P)) (id : Slice U8) (i : Usize)
    (hi : i.val ≤ selections.val.length)
    (hmiss : ∀ k, k < i.val → ∀ s, selections.val[k]? = some s → ∀ e ∈ s.entries.val, e.id.val ≠ id.val) :
    ledger.retains_record_loop selections id i ⦃ r => r ↔ RetainsId selections id ⦄ := by
  unfold ledger.retains_record_loop
  apply loop.spec_decr_nat (fun i : Usize => selections.val.length - i.val)
    (fun i : Usize => i.val ≤ selections.val.length ∧
      ∀ k, k < i.val → ∀ s, selections.val[k]? = some s → ∀ e ∈ s.entries.val, e.id.val ≠ id.val)
  · intro i ⟨hi, hmiss⟩
    unfold ledger.retains_record_loop.body
    dsimp only
    split
    · step as ⟨ s, hs ⟩
      step with entries_contain_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        obtain ⟨e, hmem, hid⟩ := hb.mp hbt
        refine ⟨s, ?_, e, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hmem, hid⟩
        rw [hs]; exact List.getElem_mem _
      · rename_i hbf
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk s' hs' e hmem
        by_cases hki : k < i.val
        · exact hmiss k hki s' hs' e hmem
        · have : k = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at hs'
          simp only [Option.some.injEq] at hs'
          rw [← hs', ← hs] at hmem
          intro heq
          exact hbf (hb.mpr ⟨e, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hmem, heq⟩)
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨s, hmem, e, he, hid⟩
      obtain ⟨k, hk, hks⟩ := List.getElem_of_mem hmem
      exact hmiss k (by scalar_tac) s (by rw [List.getElem?_eq_getElem hk, hks]) e he hid
  · exact ⟨hi, hmiss⟩

/-- `retains_record` finds a record id in any stream's selection. -/
theorem retains_record_spec (selections : Slice (ledger.Selection P)) (id : Slice U8) :
    ledger.retains_record selections id ⦃ r => r ↔ RetainsId selections id ⦄ := by
  unfold ledger.retains_record
  apply retains_record_loop_spec
  · simp
  · intro k hk; simp at hk

/-- Lean `AppendReady`'s expected-head conjunct: the caller's expected head is
exactly the selection's current head (none for an empty selection). -/
def HeadOk (entries : Slice (ledger.Entry P)) (expected : Option ledger.Head) : Prop :=
  match expected with
  | none => entries.val = []
  | some h => ∃ last, entries.val.getLast? = some last ∧ h.stream.val = last.stream.val ∧
      h.sequence = last.sequence ∧ h.record.val = last.id.val

theorem last_of_nonempty (entries : Slice (ledger.Entry P)) (n : Usize)
    (hn : n.val = entries.val.length - 1) (hne : entries.val ≠ []) :
    entries.val.getLast? = some (entries.val[n.val]'(by
      have := List.length_pos_of_ne_nil hne; omega)) := by
  have hpos := List.length_pos_of_ne_nil hne
  rw [List.getLast?_eq_getElem?, List.getElem?_eq_getElem (by omega)]
  simp [hn]

theorem head_matches_spec (entries : Slice (ledger.Entry P)) (expected : Option ledger.Head) :
    ledger.head_matches entries expected ⦃ r => r ↔ HeadOk entries expected ⦄ := by
  unfold ledger.head_matches
  cases expected with
  | none =>
    simp only [HeadOk]
    step as ⟨ b, hb ⟩
    simp_all [Slice.length, List.length_eq_zero_iff]
  | some h =>
    simp only [HeadOk]
    step as ⟨ b, hb ⟩
    split
    · rename_i hbt
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨last, hl, _⟩
      have : entries.val = [] := by simp_all [Slice.length, List.length_eq_zero_iff]
      simp [this] at hl
    · rename_i hbf
      have hne : entries.val ≠ [] := by
        intro h0; apply hbf; simp_all [Slice.length, List.length_eq_zero_iff]
      have hpos := List.length_pos_of_ne_nil hne
      step as ⟨ n, hn ⟩
      step as ⟨ last, hlast ⟩
      have hgl : entries.val.getLast? = some last := by
        rw [hlast]; exact last_of_nonempty entries n (by scalar_tac) hne
      step with same_spec as ⟨ b1, hb1 ⟩
      split
      · rename_i hs
        split
        · rename_i hq
          step with same_spec as ⟨ b2, hb2 ⟩
          constructor
          · intro hb2t
            exact ⟨last, hgl, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hb1.mp hs, hq,
              by simpa [alloc.vec.Vec.deref, Slice.from_val] using hb2.mp hb2t⟩
          · rintro ⟨l', hl', _, _, hr⟩
            rw [hgl] at hl'; simp only [Option.some.injEq] at hl'; subst hl'
            exact hb2.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using hr)
        · rename_i hq
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
          rintro ⟨l', hl', _, hsq, _⟩
          rw [hgl] at hl'; simp only [Option.some.injEq] at hl'; subst hl'
          exact hq hsq
      · rename_i hs
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        rintro ⟨l', hl', hst, _⟩
        rw [hgl] at hl'; simp only [Option.some.injEq] at hl'; subst hl'
        exact hs (hb1.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using hst))

/-- The new record names exactly the current head's id, or none on an empty selection. -/
def PredOk (entries : Slice (ledger.Entry P)) (record : ledger.Entry P) : Prop :=
  match entries.val.getLast? with
  | none => record.predecessor = none
  | some last => ∃ p, record.predecessor = some p ∧ p.val = last.id.val

theorem predecessor_ok_spec (entries : Slice (ledger.Entry P)) (record : ledger.Entry P) :
    ledger.predecessor_ok entries record ⦃ r => r ↔ PredOk entries record ⦄ := by
  unfold ledger.predecessor_ok
  step as ⟨ b, hb ⟩
  split
  · rename_i hbt
    have h0 : entries.val = [] := by simp_all [Slice.length]
    step with same_option_spec as ⟨ l, hl ⟩
    simp [PredOk, h0, hl]
  · rename_i hbf
    have hne : entries.val ≠ [] := by intro h0; apply hbf; simp_all [Slice.length]
    have hpos := List.length_pos_of_ne_nil hne
    cases hp : record.predecessor with
    | none =>
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, PredOk]
      obtain ⟨last, hl⟩ : ∃ last, entries.val.getLast? = some last := by
        cases h : entries.val.getLast? with
        | none => simp [List.getLast?_eq_none_iff] at h; exact absurd h hne
        | some l => exact ⟨l, rfl⟩
      rw [hl]; simp [hp]
    | some p =>
      dsimp only
      step as ⟨ n, hn ⟩
      step as ⟨ last, hlast ⟩
      have hgl : entries.val.getLast? = some last := by
        rw [hlast]; exact last_of_nonempty entries n (by scalar_tac) hne
      step with same_spec as ⟨ l, hl ⟩
      simp only [PredOk, hgl]
      constructor
      · intro hlt
        exact ⟨p, hp, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hl.mp hlt⟩
      · rintro ⟨p', hp', hpv⟩
        rw [hp] at hp'; simp only [Option.some.injEq] at hp'; subst hp'
        exact hl.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using hpv)

/-- Every closure object is available now or already retained. -/
def MaterialOk (closure available material : Slice Digest) : Prop :=
  ∀ d ∈ closure.val, d ∈ available.val ∨ d ∈ material.val

theorem material_ok_loop_spec (closure available material : Slice Digest) (i : Usize)
    (hi : i.val ≤ closure.val.length)
    (hdone : ∀ k, k < i.val → ∀ d, closure.val[k]? = some d → d ∈ available.val ∨ d ∈ material.val) :
    ledger.material_ok_loop closure available material i ⦃ r => r ↔ MaterialOk closure available material ⦄ := by
  unfold ledger.material_ok_loop
  apply loop.spec_decr_nat (fun i : Usize => closure.val.length - i.val)
    (fun i : Usize => i.val ≤ closure.val.length ∧
      ∀ k, k < i.val → ∀ d, closure.val[k]? = some d → d ∈ available.val ∨ d ∈ material.val)
  · intro i ⟨hi, hdone⟩
    unfold ledger.material_ok_loop.body
    dsimp only
    split
    · step as ⟨ d, hd ⟩
      have hstep : ∀ i' : Usize, i'.val = i.val + 1 → (d ∈ available.val ∨ d ∈ material.val) →
          ∀ k, k < i'.val → ∀ d', closure.val[k]? = some d' → d' ∈ available.val ∨ d' ∈ material.val := by
        intro i' hi' hdin k hk d' hd'
        by_cases hki : k < i.val
        · exact hdone k hki d' hd'
        · have : k = i.val := by omega
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at hd'
          simp only [Option.some.injEq] at hd'
          rw [← hd', ← hd]; exact hdin
      step with contains_digest_spec as ⟨ a, ha ⟩
      split
      · rename_i hat
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        exact hstep i' (by scalar_tac) (Or.inl (ha.mp hat))
      · rename_i haf
        step with contains_digest_spec as ⟨ m, hm ⟩
        split
        · rename_i hmt
          apply WP.spec_bind (Usize.add_spec (by scalar_tac))
          intro i' hi'
          simp only [WP.spec_ok]
          and_intros
          all_goals try scalar_tac
          exact hstep i' (by scalar_tac) (Or.inr (hm.mp hmt))
        · rename_i hmf
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
          intro hall
          rcases hall d (by rw [hd]; exact List.getElem_mem _) with h | h
          · exact haf (ha.mpr h)
          · exact hmf (hm.mpr h)
    · simp only [WP.spec_ok, true_iff]
      intro d hmem
      obtain ⟨k, hk, hkd⟩ := List.getElem_of_mem hmem
      exact hdone k (by scalar_tac) d (by rw [List.getElem?_eq_getElem hk, hkd])
  · exact ⟨hi, hdone⟩

theorem material_ok_spec (closure available material : Slice Digest) :
    ledger.material_ok closure available material ⦃ r => r ↔ MaterialOk closure available material ⦄ := by
  unfold ledger.material_ok
  apply material_ok_loop_spec
  · simp
  · intro k hk; simp at hk

open Classical in
/-- Lean `AppendReady`, conjunct by conjunct and in the calculus's order: the
first failing conjunct names the refusal; `Ok` only when all hold. -/
noncomputable def readyOutcome (inst : ledger.Validator V P) (v : V)
    (selections : Slice (ledger.Selection P)) (material : Slice Digest)
    (entries : Slice (ledger.Entry P)) (stream : Slice U8) (expected : Option ledger.Head)
    (record : ledger.Entry P) (available : Slice Digest) : core.result.Result Unit ledger.Refusal :=
  if ¬ HeadOk entries expected then .Err .StaleHead
  else if ¬ HistoryOk inst v stream entries then .Err .UnlinkedPrefix
  else if record.stream.val ≠ stream.val then .Err .ForeignStream
  else if record.sequence.val ≠ entries.val.length + 1 then .Err .Sequence
  else if ¬ PredOk entries record then .Err .Predecessor
  else if RetainsId selections (alloc.vec.Vec.deref record.id) then .Err .DuplicateRecord
  else if ¬ MaterialOk (alloc.vec.Vec.deref record.closure) available material then .Err .MissingMaterial
  else if inst.validate v entries (Slice.len entries) record ≠ ok true then .Err .InvalidRecord
  else .Ok ()

/-- `ready_against` is exactly Lean `AppendReady` with its refusal order. -/
theorem ready_against_spec (inst : ledger.Validator V P) (v : V) (htotal : Total inst v)
    (selections : Slice (ledger.Selection P)) (material : Slice Digest)
    (entries : Slice (ledger.Entry P)) (stream : Slice U8) (expected : Option ledger.Head)
    (record : ledger.Entry P) (available : Slice Digest) :
    ledger.ready_against inst v selections material entries stream expected record available
      ⦃ res => res = readyOutcome inst v selections material entries stream expected record available ⦄ := by
  unfold ledger.ready_against
  step with head_matches_spec as ⟨ b, hb ⟩
  split
  · rename_i hh
    step with history_check_spec inst v htotal as ⟨ b1, hb1 ⟩
    split
    · rename_i hc
      step with same_spec as ⟨ b2, hb2 ⟩
      have hst_iff : b2 = true ↔ record.stream.val = stream.val := by
        simpa [alloc.vec.Vec.deref, Slice.from_val] using hb2
      split
      · rename_i hs
        split
        · rename_i h0
          simp only [WP.spec_ok, readyOutcome]
          have hne : record.sequence.val ≠ entries.val.length + 1 := by
            rw [h0]; simp
          simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, hne]
        · rename_i h0
          have h0' : record.sequence.val ≠ 0 := by
            intro h; apply h0; apply UScalar.eq_of_val_eq; simpa using h
          apply WP.spec_bind (Usize.sub_spec (by scalar_tac))
          intro n hn
          split
          · rename_i hseq
            rw [bne_iff_ne] at hseq
            simp only [WP.spec_ok, readyOutcome]
            have hne : record.sequence.val ≠ entries.val.length + 1 := by
              intro h; apply hseq; apply UScalar.eq_of_val_eq; simp [Slice.len] at *; omega
            simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, hne]
          · rename_i hseq
            rw [bne_iff_ne, not_not] at hseq
            have heq : record.sequence.val = entries.val.length + 1 := by
              have := congrArg (fun x : Usize => x.val) hseq
              simp [Slice.len] at *; omega
            step with predecessor_ok_spec as ⟨ b3, hb3 ⟩
            split
            · rename_i hp
              step with retains_record_spec as ⟨ b4, hb4 ⟩
              split
              · rename_i hr
                simp only [WP.spec_ok, readyOutcome]
                simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, heq, hb3.mp hp, hb4.mp hr]
              · rename_i hr
                step with material_ok_spec as ⟨ b5, hb5 ⟩
                split
                · rename_i hm
                  obtain ⟨b6, hb6⟩ := htotal entries (Slice.len entries) record
                  rw [hb6]
                  simp only [bind_ok]
                  have hr' : ¬ RetainsId selections (alloc.vec.Vec.deref record.id) := fun h => hr (hb4.mpr h)
                  split
                  · rename_i hv
                    simp only [WP.spec_ok, readyOutcome]
                    simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, heq, hb3.mp hp, hr', hb5.mp hm, hb6, hv]
                  · rename_i hv
                    simp only [WP.spec_ok, readyOutcome]
                    simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, heq, hb3.mp hp, hr', hb5.mp hm, hb6, hv]
                · rename_i hm
                  have hr' : ¬ RetainsId selections (alloc.vec.Vec.deref record.id) := fun h => hr (hb4.mpr h)
                  have hm' : ¬ MaterialOk (alloc.vec.Vec.deref record.closure) available material :=
                    fun h => hm (hb5.mpr h)
                  simp only [WP.spec_ok, readyOutcome]
                  simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, heq, hb3.mp hp, hr', hm']
            · rename_i hp
              have hp' : ¬ PredOk entries record := fun h => hp (hb3.mpr h)
              simp only [WP.spec_ok, readyOutcome]
              simp [hb.mp hh, hb1.mp hc, hst_iff.mp hs, heq, hp']
      · rename_i hs
        have hs' : record.stream.val ≠ stream.val := fun h => hs (hst_iff.mpr h)
        simp only [WP.spec_ok, readyOutcome]
        simp [hb.mp hh, hb1.mp hc, hs']
    · rename_i hc
      have hc' : ¬ HistoryOk inst v stream entries := fun h => hc (hb1.mpr h)
      simp only [WP.spec_ok, readyOutcome]
      simp [hb.mp hh, hc']
  · rename_i hh
    have hh' : ¬ HeadOk entries expected := fun h => hh (hb.mpr h)
    simp only [WP.spec_ok, readyOutcome]
    simp [hh']

/-- Corollary: an append is admitted exactly when every `AppendReady`
conjunct holds. -/
theorem ready_ok_iff (inst : ledger.Validator V P) (v : V)
    (selections : Slice (ledger.Selection P)) (material : Slice Digest)
    (entries : Slice (ledger.Entry P)) (stream : Slice U8) (expected : Option ledger.Head)
    (record : ledger.Entry P) (available : Slice Digest) :
    readyOutcome inst v selections material entries stream expected record available = .Ok () ↔
      HeadOk entries expected ∧ HistoryOk inst v stream entries ∧
      record.stream.val = stream.val ∧ record.sequence.val = entries.val.length + 1 ∧
      PredOk entries record ∧ ¬ RetainsId selections (alloc.vec.Vec.deref record.id) ∧
      MaterialOk (alloc.vec.Vec.deref record.closure) available material ∧
      inst.validate v entries (Slice.len entries) record = ok true := by
  unfold readyOutcome
  split_ifs <;> simp_all

/-- What `find` returns: the first selection on the stream, or none. -/
def FindOk (selections : Slice (ledger.Selection P)) (stream : Slice U8) : Option Usize → Prop
  | none => ∀ s ∈ selections.val, s.stream.val ≠ stream.val
  | some k => ∃ h : k.val < selections.val.length,
      (selections.val[k.val]'h).stream.val = stream.val ∧
      ∀ j (hj : j < k.val), (selections.val[j]'(by omega)).stream.val ≠ stream.val

theorem find_loop_spec (selections : Slice (ledger.Selection P)) (stream : Slice U8) (i : Usize)
    (hi : i.val ≤ selections.val.length)
    (hmiss : ∀ j (hj : j < i.val), (selections.val[j]'(by omega)).stream.val ≠ stream.val) :
    ledger.find_loop selections stream i ⦃ o => FindOk selections stream o ⦄ := by
  unfold ledger.find_loop
  apply loop.spec_decr_nat (fun i : Usize => selections.val.length - i.val)
    (fun i : Usize => ∃ hi : i.val ≤ selections.val.length,
      ∀ j (hj : j < i.val), (selections.val[j]'(by omega)).stream.val ≠ stream.val)
  · intro i ⟨hi, hmiss⟩
    unfold ledger.find_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ s, hs ⟩
      step with same_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, FindOk]
        refine ⟨by scalar_tac, ?_, hmiss⟩
        rw [← hs]; simpa [alloc.vec.Vec.deref, Slice.from_val] using hb.mp hbt
      · rename_i hbf
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨⟨by scalar_tac, ?_⟩, by scalar_tac⟩
        intro j hj
        by_cases hji : j < i.val
        · exact hmiss j hji
        · have : j = i.val := by scalar_tac
          subst this
          intro heq
          apply hbf; apply hb.mpr
          rw [hs]; simpa [alloc.vec.Vec.deref, Slice.from_val] using heq
    · simp only [WP.spec_ok, FindOk]
      intro s hmem heq
      obtain ⟨j, hj, hjs⟩ := List.getElem_of_mem hmem
      exact hmiss j (by scalar_tac) (by rw [hjs]; exact heq)
  · exact ⟨hi, hmiss⟩

theorem find_spec (selections : Slice (ledger.Selection P)) (stream : Slice U8) :
    ledger.find selections stream ⦃ o => FindOk selections stream o ⦄ := by
  unfold ledger.find
  apply find_loop_spec
  all_goals simp

/-- The entries currently selected for `stream` (none for a new stream). -/
def streamEntries (self : ledger.Store P) (stream : Slice U8) : alloc.vec.Vec (ledger.Entry P) :=
  match self.selections.val.find? (fun s => s.stream.val == stream.val) with
  | none => alloc.vec.Vec.new (ledger.Entry P)
  | some s => s.entries

theorem streamEntries_of_find (self : ledger.Store P) (stream : Slice U8) (o : Option Usize)
    (h : FindOk (alloc.vec.Vec.deref self.selections) stream o) :
    streamEntries self stream = (match o with
      | none => alloc.vec.Vec.new (ledger.Entry P)
      | some k => (self.selections.val[k.val]?.map (·.entries)).getD (alloc.vec.Vec.new _)) := by
  unfold streamEntries
  cases o with
  | none =>
    simp only [FindOk, alloc.vec.Vec.deref, Slice.from_val] at h
    rw [List.find?_eq_none.mpr (by intro s hs; simpa using h s hs)]
  | some k =>
    obtain ⟨hk, hks, hbefore⟩ := h
    simp only [alloc.vec.Vec.deref, Slice.from_val] at hk hks hbefore
    have : self.selections.val.find? (fun s => s.stream.val == stream.val) =
        some (self.selections.val[k.val]'hk) := by
      rw [List.find?_eq_some_iff_getElem]
      refine ⟨by simpa using hks, k.val, hk, rfl, ?_⟩
      intro j hj; simpa using hbefore j hj
    rw [this]
    simp [List.getElem?_eq_getElem hk]

/-- `Store::ready` is `AppendReady` over the stream's current selection. -/
theorem Store.ready_spec (inst : ledger.Validator V P) (v : V) (htotal : Total inst v)
    (self : ledger.Store P) (stream : Slice U8) (expected : Option ledger.Head)
    (record : ledger.Entry P) (available : Slice Digest) :
    ledger.Store.ready inst self v stream expected record available ⦃ res =>
      res = readyOutcome inst v (alloc.vec.Vec.deref self.selections)
        (alloc.vec.Vec.deref self.material) (alloc.vec.Vec.deref (streamEntries self stream))
        stream expected record available ⦄ := by
  unfold ledger.Store.ready
  step with find_spec as ⟨ o, ho ⟩
  have hE := streamEntries_of_find self stream o ho
  cases o with
  | none =>
    simp only [bind_ok]
    rw [hE]
    exact ready_against_spec inst v htotal _ _ _ stream expected record available
  | some k =>
    obtain ⟨hk, _, _⟩ := ho
    simp only [alloc.vec.Vec.deref, Slice.from_val] at hk
    step as ⟨ sel, hsel ⟩
    rw [hE]
    simp only [List.getElem?_eq_getElem hk, Option.map_some, Option.getD_some]
    rw [← hsel]
    exact ready_against_spec inst v htotal _ _ _ stream expected record available

/-- Lean `append` changes nothing on refusal: when `ready` refuses, `append`
returns that refusal and the store unchanged. -/
theorem Store.append_refused (inst : ledger.Validator V P) (self : ledger.Store P) (v : V)
    (stream : Slice U8) (expected : Option ledger.Head) (record : ledger.Entry P)
    (available : Slice Digest) (r : ledger.Refusal)
    (h : ledger.Store.ready inst self v stream expected record available = ok (.Err r)) :
    ledger.Store.append inst self v stream expected record available = ok (.Err r, self) := by
  unfold ledger.Store.append
  rw [h]
  simp [core.result.Result.Insts.CoreOpsTry.branch,
    core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]

end Writfold.Core
