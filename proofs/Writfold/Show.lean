import Writfold.Walk

/-!
# `find`'s rows: tier order, once, within the limit

Theorems over the Aeneas translation of `writfold_core::find::show`,
`among` and `exact_stops`. `show` takes every tier's hits in tier order
and keeps the positions `admit` takes; `show_spec` says it is the list
model `Shown` below, and the model's theorems are the claims:

- `shown_le_limit`: at most `limit` rows.
- `shown_once`: no record is shown twice.
- `shown_in_order`: rows keep the order of the hits, so of the tiers.
- `shown_passes`: every row's hit passes the filter.
- `shown_first`: a record is shown under its first hit that passes the
  filter: no earlier hit of the same record passes. So it is shown under
  the first tier that found it.
- `limit_prefix`: the rows under a limit `n ≤ m` are the first `n` rows
  under `m`. `find --limit N` only shortens the answer.
- `stop_is_prefix`: the rows of the first tiers' hits alone are the first
  rows of all the tiers' hits. Stopping after the `exact` tier
  (`exact_stops_spec`: a record's own ID, exactly one record) only
  shortens the answer.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Show

/-- The rows so far: each hit's position and its record. -/
abbrev St := List (Nat × U32)

/-- One hit walked: kept when fewer than `L` rows are shown, its record is
not shown yet and it passes the filter (`admit`). -/
def step (L : Nat) (st : St) (i : Nat) (h : find.Hit) : St :=
  if st.length < L ∧ h.record ∉ st.map Prod.snd ∧ h.passes = true then st ++ [(i, h.record)]
  else st

/-- Walk the hits, the first one at position `k`. -/
def go (L : Nat) : List find.Hit → Nat → St → St
  | [], _, st => st
  | h :: hs, k, st => go L hs (k + 1) (step L st k h)

/-- The rows `find` shows for these hits under limit `L`. -/
def Shown (L : Nat) (hits : List find.Hit) : St := go L hits 0 []

theorem go_append (L : Nat) : ∀ (l1 l2 : List find.Hit) (k : Nat) (st : St),
    go L (l1 ++ l2) k st = go L l2 (k + l1.length) (go L l1 k st)
  | [], l2, k, st => by simp [go]
  | h :: t, l2, k, st => by
    rw [List.cons_append, go, go, go_append L t l2 (k + 1)]
    congr 1
    simp only [List.length_cons]
    omega

theorem step_prefix (L : Nat) (st : St) (i : Nat) (h : find.Hit) : st <+: step L st i h := by
  unfold step
  split
  · exact List.prefix_append _ _
  · exact List.prefix_refl _

theorem go_prefix (L : Nat) : ∀ (l : List find.Hit) (k : Nat) (st : St), st <+: go L l k st
  | [], _, _ => List.prefix_refl _
  | h :: t, k, st => (step_prefix L st k h).trans (go_prefix L t (k + 1) _)

theorem go_full (L : Nat) : ∀ (l : List find.Hit) (k : Nat) (st : St), L ≤ st.length →
    go L l k st = st
  | [], _, _, _ => rfl
  | h :: t, k, st, hl => by
    have hs : step L st k h = st := by
      unfold step
      split
      · rename_i hc; omega
      · rfl
    rw [go, hs, go_full L t (k + 1) st hl]

theorem step_len (L : Nat) (st : St) (i : Nat) (h : find.Hit) :
    (step L st i h).length ≤ st.length + 1 := by
  unfold step
  split <;> simp

theorem step_le_limit (L : Nat) (st : St) (i : Nat) (h : find.Hit) (hl : st.length ≤ L) :
    (step L st i h).length ≤ L := by
  unfold step
  split
  · rename_i hc; simp; omega
  · exact hl

theorem go_len (L : Nat) : ∀ (l : List find.Hit) (k : Nat) (st : St),
    (go L l k st).length ≤ st.length + l.length
  | [], _, _ => by simp [go]
  | h :: t, k, st => by
    have := go_len L t (k + 1) (step L st k h)
    have := step_len L st k h
    simp only [go, List.length_cons]
    omega

theorem go_take_succ (L : Nat) (hits : List find.Hit) (i : Nat) (hi : i < hits.length) :
    go L (hits.take (i + 1)) 0 [] = step L (go L (hits.take i) 0 []) i hits[i] := by
  rw [List.take_add_one, List.getElem?_eq_getElem hi]
  simp only [Option.toList]
  rw [go_append]
  simp [go, List.length_take, Nat.min_eq_left hi.le]

/-! ## The limit -/

theorem limit_prefix_go (n m : Nat) (hnm : n ≤ m) : ∀ (l : List find.Hit) (k : Nat) (st : St),
    st.length ≤ n → go n l k st = (go m l k st).take n
  | [], k, st, hs => by simp [go, List.take_of_length_le hs]
  | h :: t, k, st, hs => by
    by_cases hlt : st.length < n
    · have heq : step n st k h = step m st k h := by
        unfold step
        by_cases hA : h.record ∉ st.map Prod.snd ∧ h.passes = true
        · simp only [hlt, lt_of_lt_of_le hlt hnm, hA]
        · have hn : ¬ (st.length < n ∧ h.record ∉ st.map Prod.snd ∧ h.passes = true) :=
            fun ⟨_, hb⟩ => hA hb
          have hm : ¬ (st.length < m ∧ h.record ∉ st.map Prod.snd ∧ h.passes = true) :=
            fun ⟨_, hb⟩ => hA hb
          rw [if_neg hn, if_neg hm]
      rw [go, go, heq]
      apply limit_prefix_go n m hnm t (k + 1)
      rw [← heq]
      exact step_le_limit n st k h (by omega)
    · have hn : st.length = n := by omega
      rw [go_full n (h :: t) k st (by omega)]
      obtain ⟨d, hd⟩ := go_prefix m (h :: t) k st
      rw [← hd, List.take_append_of_le_length (by omega), List.take_of_length_le (by omega)]

/-- The rows under a limit `n ≤ m` are the first `n` rows under `m`. -/
theorem limit_prefix (n m : Nat) (hnm : n ≤ m) (hits : List find.Hit) :
    Shown n hits = (Shown m hits).take n :=
  limit_prefix_go n m hnm hits 0 [] (by simp)

/-- The rows of the first `k` hits are the first rows of all the hits. -/
theorem stop_is_prefix (L k : Nat) (hits : List find.Hit) :
    Shown L (hits.take k) <+: Shown L hits := by
  unfold Shown
  conv => rhs; rw [← List.take_append_drop k hits]
  rw [go_append]
  exact go_prefix L _ _ _

/-! ## Each row -/

/-- What holds of the rows after the first `i` hits. -/
def Good (L : Nat) (hits : List find.Hit) (i : Nat) (st : St) : Prop :=
  st.length ≤ L ∧ (st.map Prod.snd).Nodup ∧ (st.map Prod.fst).Pairwise (· < ·) ∧
  (∀ p ∈ st, ∃ hp : p.1 < hits.length, p.1 < i ∧ hits[p.1].record = p.2 ∧
    hits[p.1].passes = true ∧
    ∀ j (hj : j < hits.length), j < p.1 → hits[j].record = p.2 → hits[j].passes = false) ∧
  (∀ j (hj : j < hits.length), j < i → hits[j].passes = true →
    hits[j].record ∈ st.map Prod.snd ∨ L ≤ st.length)

theorem good_zero (L : Nat) (hits : List find.Hit) : Good L hits 0 [] := by
  refine ⟨by simp, by simp, by simp, by simp, ?_⟩
  intro j _ hj; omega

theorem good_step (L : Nat) (hits : List find.Hit) (i : Nat) (hi : i < hits.length) (st : St)
    (hg : Good L hits i st) : Good L hits (i + 1) (step L st i hits[i]) := by
  obtain ⟨hlen, hnd, hpw, hel, hseen⟩ := hg
  unfold step
  split
  · rename_i hc
    obtain ⟨hcl, hcn, hcp⟩ := hc
    refine ⟨by simp; omega, ?_, ?_, ?_, ?_⟩
    · rw [List.map_append, List.nodup_append]
      refine ⟨hnd, by simp, ?_⟩
      intro a ha b hb
      simp only [List.map_cons, List.map_nil, List.mem_singleton] at hb
      subst hb
      intro he; subst he; exact hcn ha
    · rw [List.map_append, List.pairwise_append]
      refine ⟨hpw, by simp, ?_⟩
      intro a ha b hb
      simp only [List.map_cons, List.map_nil, List.mem_singleton] at hb
      subst hb
      obtain ⟨p, hp, rfl⟩ := List.mem_map.mp ha
      obtain ⟨_, hpi, _⟩ := hel p hp
      exact hpi
    · intro p hp
      rcases List.mem_append.mp hp with hp | hp
      · obtain ⟨h1, h2, h3⟩ := hel p hp
        exact ⟨h1, by omega, h3⟩
      · simp only [List.mem_singleton] at hp
        subst hp
        refine ⟨hi, by simp, rfl, hcp, ?_⟩
        intro j hj hji hjr
        cases hpj : hits[j].passes
        · rfl
        · exfalso
          rcases hseen j hj hji hpj with hm | hm
          · simp only at hjr; rw [hjr] at hm; exact hcn hm
          · omega
    · intro j hj hji hpj
      by_cases hje : j = i
      · subst hje; left; simp
      · rcases hseen j hj (by omega) hpj with hm | hm
        · left; simp [List.map_append]; exact Or.inl (by simpa using hm)
        · right; simp; omega
  · rename_i hc
    refine ⟨hlen, hnd, hpw, ?_, ?_⟩
    · intro p hp
      obtain ⟨h1, h2, h3⟩ := hel p hp
      exact ⟨h1, by omega, h3⟩
    · intro j hj hji hpj
      by_cases hje : j = i
      · subst hje
        by_cases hL : st.length < L
        · left
          by_contra hm
          exact hc ⟨hL, hm, hpj⟩
        · right; omega
      · exact hseen j hj (by omega) hpj

theorem good_take (L : Nat) (hits : List find.Hit) :
    ∀ i, i ≤ hits.length → Good L hits i (go L (hits.take i) 0 [])
  | 0, _ => by simpa [go] using good_zero L hits
  | i + 1, hi => by
    rw [go_take_succ L hits i (by omega)]
    exact good_step L hits i (by omega) _ (good_take L hits i (by omega))

theorem good_shown (L : Nat) (hits : List find.Hit) :
    Good L hits hits.length (Shown L hits) := by
  have := good_take L hits hits.length (le_refl _)
  rwa [List.take_length] at this

/-- At most `L` rows. -/
theorem shown_le_limit (L : Nat) (hits : List find.Hit) : (Shown L hits).length ≤ L :=
  (good_shown L hits).1

/-- No record is shown twice. -/
theorem shown_once (L : Nat) (hits : List find.Hit) : ((Shown L hits).map Prod.snd).Nodup :=
  (good_shown L hits).2.1

/-- Rows keep the order of the hits. -/
theorem shown_in_order (L : Nat) (hits : List find.Hit) :
    ((Shown L hits).map Prod.fst).Pairwise (· < ·) :=
  (good_shown L hits).2.2.1

/-- Every row is a hit that passes the filter, with that hit's record. -/
theorem shown_passes (L : Nat) (hits : List find.Hit) :
    ∀ p ∈ Shown L hits, ∃ hp : p.1 < hits.length,
      hits[p.1].record = p.2 ∧ hits[p.1].passes = true := by
  intro p hp
  obtain ⟨h1, _, h3, h4, _⟩ := (good_shown L hits).2.2.2.1 p hp
  exact ⟨h1, h3, h4⟩

/-- A record is shown under its first hit that passes the filter. -/
theorem shown_first (L : Nat) (hits : List find.Hit) :
    ∀ p ∈ Shown L hits, ∀ j (hj : j < hits.length), j < p.1 →
      hits[j].record = p.2 → hits[j].passes = false := by
  intro p hp
  obtain ⟨_, _, _, _, h5⟩ := (good_shown L hits).2.2.2.1 p hp
  exact h5

/-! ## The Rust -/

theorem among_loop_spec (rs : Slice U32) (r : U32) (i : Usize) (hi : i.val ≤ rs.val.length)
    (hno : ∀ j (hj : j < rs.val.length), j < i.val → rs.val[j] ≠ r) :
    find.among_loop rs r i ⦃ b => b = true ↔ r ∈ rs.val ⦄ := by
  unfold find.among_loop
  apply loop.spec_decr_nat (fun i : Usize => rs.val.length - i.val)
    (fun i : Usize => i.val ≤ rs.val.length ∧
      ∀ j (hj : j < rs.val.length), j < i.val → rs.val[j] ≠ r)
  · intro i ⟨hi, hno⟩
    unfold find.among_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      split
      · rename_i heq
        simp only [WP.spec_ok, true_iff]
        rw [← heq, hx]
        exact List.getElem_mem _
      · rename_i hne
        step as ⟨ i3, hi3 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hji
        by_cases hje : j = i.val
        · subst hje; rw [← hx]; exact hne
        · exact hno j hj (by scalar_tac)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro hm
      obtain ⟨j, hj, he⟩ := List.getElem_of_mem hm
      exact hno j hj (by scalar_tac) he
  · exact ⟨hi, hno⟩

theorem among_spec (rs : Slice U32) (r : U32) :
    find.among rs r ⦃ b => b = true ↔ r ∈ rs.val ⦄ := by
  unfold find.among
  exact among_loop_spec rs r 0#usize (by simp) (fun j _ hj => by simp at hj)

theorem show_loop_spec (hits : Slice find.Hit) (limit : Usize)
    (shown : alloc.vec.Vec Usize) (records : alloc.vec.Vec U32) (i : Usize)
    (hi : i.val ≤ hits.val.length)
    (hsh : shown.val.map (·.val) = (go limit.val (hits.val.take i.val) 0 []).map Prod.fst)
    (hrs : records.val = (go limit.val (hits.val.take i.val) 0 []).map Prod.snd) :
    find.show_loop hits limit shown records i
      ⦃ r => r.val.map (·.val) = (Shown limit.val hits.val).map Prod.fst ⦄ := by
  unfold find.show_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec Usize × alloc.vec.Vec U32 × Usize => hits.val.length - p.2.2.val)
    (fun p : alloc.vec.Vec Usize × alloc.vec.Vec U32 × Usize => p.2.2.val ≤ hits.val.length ∧
      p.1.val.map (·.val) = (go limit.val (hits.val.take p.2.2.val) 0 []).map Prod.fst ∧
      p.2.1.val = (go limit.val (hits.val.take p.2.2.val) 0 []).map Prod.snd)
  · rintro ⟨shown, records, i⟩ ⟨hi, hsh, hrs⟩
    unfold find.show_loop.body
    dsimp only at hi hsh hrs ⊢
    split
    · rename_i hlt
      have hil : i.val < hits.val.length := by scalar_tac
      step as ⟨ hit, hhit ⟩
      step with among_spec as ⟨ b, hb ⟩
      step with Writfold.Core.Walk.admit_iff as ⟨ b1, hb1 ⟩
      have hgl := go_len limit.val (hits.val.take i.val) 0 []
      simp only [List.length_take, List.length_nil, Nat.zero_add] at hgl
      have hsl : shown.val.length = (go limit.val (hits.val.take i.val) 0 []).length := by
        have := congrArg List.length hsh; simpa using this
      have hrl : records.val.length = (go limit.val (hits.val.take i.val) 0 []).length := by
        rw [hrs]; simp
      have hmem : (b = true) ↔ hits.val[i.val].record ∈
          (go limit.val (hits.val.take i.val) 0 []).map Prod.snd := by
        rw [hb, alloc.vec.Vec.deref, hhit]
        simp [Slice.from_val, hrs]
      split
      · rename_i hb1t
        have hc := hb1.mp hb1t
        have hlen : shown.val.length < Usize.max := by scalar_tac
        have hlen2 : records.val.length < Usize.max := by scalar_tac
        step as ⟨ shown2, hshown2 ⟩
        step as ⟨ records2, hrecords2 ⟩
        step as ⟨ i', hi' ⟩
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        have hcond : (go limit.val (hits.val.take i.val) 0 []).length < limit.val ∧
            hits.val[i.val].record ∉ (go limit.val (hits.val.take i.val) 0 []).map Prod.snd ∧
            hits.val[i.val].passes = true := by
          obtain ⟨h1, h2, h3⟩ := hc
          refine ⟨by scalar_tac, ?_, by rw [← hhit]; exact h3⟩
          intro hm; rw [← hmem] at hm; rw [hm] at h2; simp at h2
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · rw [hi1, go_take_succ limit.val hits.val i.val hil, step, if_pos hcond, hshown2]
          simp [hsh]
        · rw [hi1, go_take_succ limit.val hits.val i.val hil, step, if_pos hcond, hrecords2,
            hrs, hhit]
          simp
      · rename_i hb1f
        step as ⟨ i', hi' ⟩
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        have hcond : ¬ ((go limit.val (hits.val.take i.val) 0 []).length < limit.val ∧
            hits.val[i.val].record ∉ (go limit.val (hits.val.take i.val) 0 []).map Prod.snd ∧
            hits.val[i.val].passes = true) := by
          rintro ⟨h1, h2, h3⟩
          apply hb1f
          apply hb1.mpr
          refine ⟨by scalar_tac, ?_, by rw [hhit]; exact h3⟩
          cases hbb : b
          · rfl
          · exact absurd (hmem.mp hbb) h2
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · rw [hi1, go_take_succ limit.val hits.val i.val hil, step, if_neg hcond]; exact hsh
        · rw [hi1, go_take_succ limit.val hits.val i.val hil, step, if_neg hcond]; exact hrs
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = hits.val.length := by scalar_tac
      rw [hsh, this, List.take_length]
      rfl
  · exact ⟨hi, hsh, hrs⟩

/-- `show` is the model `Shown`: the positions it returns are the model's
rows, in order. -/
theorem show_spec (hits : Slice find.Hit) (limit : Usize) :
    find.show hits limit ⦃ r => r.val.map (·.val) = (Shown limit.val hits.val).map Prod.fst ⦄ := by
  unfold find.show
  exact show_loop_spec hits limit _ _ 0#usize (by simp) (by simp [go]) (by simp [go])

/-- `find` stops after the `exact` tier exactly when the phrase is a
record's own ID and exactly one record has it. -/
theorem exact_stops_spec (idShaped : Bool) (n : Usize) :
    find.exact_stops idShaped n ⦃ b => b = true ↔ idShaped = true ∧ n.val = 1 ⦄ := by
  unfold find.exact_stops
  split
  · rename_i h
    simp only [WP.spec_ok, decide_eq_true_eq, h, true_and]
    constructor
    · intro e; simp [e]
    · intro e; exact UScalar.eq_of_val_eq (by simpa using e)
  · rename_i h
    simp [h]

end Writfold.Core.Show
