import Extracted.WritfoldCore

/-!
# `find`'s prefix tier

Theorems over the Aeneas translation of `writfold_core::find`'s prefix
match (the text plumbing `norm` and `tokens` stays tested).

- `query_ok_spec`: a query may match by prefix only with two tokens or
  more, or one of four characters or more.
- `starts_spec`: a token begins a name token exactly when it is a list
  prefix of it.
- `prefixed_spec`: a query matches a name exactly when it may match by
  prefix and its tokens begin distinct name tokens in order, gaps allowed
  (`Emb`); the answer is then the count of name tokens left unmatched.
  Taking the first name token each query token begins never misses a
  match (`emb_first`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Find

/-- The query's tokens begin distinct name tokens, in order, gaps allowed. -/
inductive Emb : List (alloc.vec.Vec U32) → List (alloc.vec.Vec U32) → Prop
  | nil {ns} : Emb [] ns
  | skip {qs n ns} : Emb qs ns → Emb qs (n :: ns)
  | take {q qs n ns} : q.val <+: n.val → Emb qs ns → Emb (q :: qs) (n :: ns)

/-- A query that may match by prefix: two tokens or more, or one of four
characters or more. -/
def QueryOk (q : List (alloc.vec.Vec U32)) : Prop :=
  q.length ≥ 2 ∨ ∃ t, q = [t] ∧ t.val.length ≥ 4

theorem emb_tail {q : alloc.vec.Vec U32} {qs ns : List (alloc.vec.Vec U32)}
    (h : Emb (q :: qs) ns) : Emb qs ns := by
  induction ns with
  | nil => cases h
  | cons n ns ih =>
    cases h with
    | skip h => exact .skip (ih h)
    | take _ h => exact .skip h

theorem drop_some {γ : Type} {l : List γ} {i : Nat} {x : γ} (h : l[i]? = some x) :
    l.drop i = x :: l.drop (i + 1) := by
  obtain ⟨hi, rfl⟩ := List.getElem?_eq_some_iff.mp h
  exact List.drop_eq_getElem_cons hi

/-- Taking the first name token from `at` that `q` begins loses nothing. -/
theorem emb_first {q : alloc.vec.Vec U32} {qs ns : List (alloc.vec.Vec U32)} {a0 j : Nat}
    (hj : j < ns.length) (hle : a0 ≤ j) (hpre : q.val <+: ns[j].val)
    (hnone : ∀ k (hk : k < ns.length), a0 ≤ k → k < j → ¬ q.val <+: ns[k].val) :
    Emb (q :: qs) (ns.drop a0) ↔ Emb qs (ns.drop (j + 1)) := by
  induction h : j - a0 generalizing a0 with
  | zero =>
    have : a0 = j := by omega
    subst this
    rw [drop_some (List.getElem?_eq_getElem hj)]
    constructor
    · intro he
      cases he with
      | skip he => exact emb_tail he
      | take _ he => exact he
    · intro he; exact .take hpre he
  | succ d ih =>
    have hlt : a0 < j := by omega
    have hat : a0 < ns.length := by omega
    rw [drop_some (List.getElem?_eq_getElem hat)]
    have hno := hnone a0 hat (le_refl _) hlt
    constructor
    · intro he
      cases he with
      | skip he => exact (ih (by omega) (fun k hk h1 h2 => hnone k hk (by omega) h2) (by omega)).mp he
      | take hp _ => exact absurd hp hno
    · intro he
      exact .skip ((ih (by omega) (fun k hk h1 h2 => hnone k hk (by omega) h2) (by omega)).mpr he)

theorem emb_none {q : alloc.vec.Vec U32} {qs ns : List (alloc.vec.Vec U32)} {a0 : Nat}
    (hnone : ∀ k (hk : k < ns.length), a0 ≤ k → ¬ q.val <+: ns[k].val) :
    ¬ Emb (q :: qs) (ns.drop a0) := by
  induction h : ns.length - a0 generalizing a0 with
  | zero =>
    rw [List.drop_eq_nil_of_le (by omega)]
    intro he; cases he
  | succ d ih =>
    have hat : a0 < ns.length := by omega
    rw [drop_some (List.getElem?_eq_getElem hat)]
    intro he
    cases he with
    | skip he => exact ih (fun k hk h1 => hnone k hk (by omega)) (by omega) he
    | take hp _ => exact hnone a0 hat (le_refl _) hp

theorem deref_val {γ : Type} (v : alloc.vec.Vec γ) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref, Slice.from_val]

theorem query_ok_spec (q : Slice (alloc.vec.Vec U32)) :
    find.query_ok q ⦃ b => b = true ↔ QueryOk q.val ⦄ := by
  unfold find.query_ok
  dsimp only
  split
  · rename_i h; simp only [WP.spec_ok, true_iff, QueryOk]; left; scalar_tac
  · rename_i h
    split
    · rename_i h1
      have hl : q.val.length = 1 := by scalar_tac
      step as ⟨ v, hv ⟩
      simp only [decide_eq_true_eq, QueryOk]
      obtain ⟨t, ht⟩ := List.length_eq_one_iff.mp hl
      simp only [ht, List.getElem_cons_zero] at hv
      subst hv
      simp [ht, alloc.vec.Vec.len]
    · rename_i h1
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, QueryOk, not_or, not_exists, not_and]
      refine ⟨by scalar_tac, fun t ht => ?_⟩
      exfalso; apply h1
      have : q.val.length = 1 := by simp [ht]
      exact UScalar.eq_of_val_eq (by simpa [Slice.len] using this)

theorem starts_loop_spec (w s : Slice U32) (i : Usize) (hle : s.val.length ≤ w.val.length)
    (hi : i.val ≤ s.val.length)
    (heq : ∀ j (h1 : j < s.val.length) (h2 : j < w.val.length), j < i.val → w.val[j] = s.val[j]) :
    find.starts_loop w s i ⦃ b => b = true ↔ s.val <+: w.val ⦄ := by
  unfold find.starts_loop
  apply loop.spec_decr_nat (fun i : Usize => s.val.length - i.val)
    (fun i : Usize => i.val ≤ s.val.length ∧
      ∀ j (h1 : j < s.val.length) (h2 : j < w.val.length), j < i.val → w.val[j] = s.val[j])
  · intro i ⟨hi, heq⟩
    unfold find.starts_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ a, ha ⟩
      step as ⟨ b, hb ⟩
      split
      · rename_i hne
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        intro hp
        obtain ⟨t, ht⟩ := hp
        have : w.val[i.val]? = s.val[i.val]? := by
          rw [← ht]; simp [List.getElem?_append_left (by scalar_tac : i.val < s.val.length)]
        rw [List.getElem?_eq_getElem (by scalar_tac), List.getElem?_eq_getElem (by scalar_tac)] at this
        simp only [bne_iff_ne, ne_eq] at hne
        apply hne
        rw [ha, hb]; simpa using this
      · rename_i hne
        step as ⟨ i4, hi4 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j h1 h2 hj
        by_cases hji : j < i.val
        · exact heq j h1 h2 hji
        · have : j = i.val := by scalar_tac
          subst this
          simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
          rw [← ha, ← hb]; exact hne
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      have hall : i.val = s.val.length := by scalar_tac
      refine ⟨w.val.drop s.val.length, ?_⟩
      apply List.ext_getElem
      · simp; omega
      · intro n h1 h2
        by_cases hn : n < s.val.length
        · rw [List.getElem_append_left hn]; exact (heq n hn h2 (by omega)).symm
        · rw [List.getElem_append_right (by omega)]; simp; congr 1; omega
  · exact ⟨hi, heq⟩

theorem starts_spec (w s : Slice U32) :
    find.starts w s ⦃ b => b = true ↔ s.val <+: w.val ⦄ := by
  unfold find.starts
  dsimp only
  split
  · rename_i h
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro hp; have := hp.length_le; scalar_tac
  · exact starts_loop_spec w s 0#usize (by scalar_tac) (by simp) (fun j _ _ hj => absurd hj (by simp))

/-- The first name token a0 or after `a0` that `q` begins. -/
def FirstFrom (ns : List (alloc.vec.Vec U32)) (q : List U32) (a0 : Nat) : Option Usize → Prop
  | some j => ∃ h : j.val < ns.length, a0 ≤ j.val ∧ q <+: ns[j.val].val ∧
      ∀ k (hk : k < ns.length), a0 ≤ k → k < j.val → ¬ q <+: ns[k].val
  | none => ∀ k (hk : k < ns.length), a0 ≤ k → ¬ q <+: ns[k].val

theorem next_from_loop_spec (ns : Slice (alloc.vec.Vec U32)) (q : Slice U32) (a0 j : Usize)
    (hj : j.val ≤ ns.val.length) (hat : a0.val ≤ j.val)
    (hno : ∀ k (hk : k < ns.val.length), a0.val ≤ k → k < j.val → ¬ q.val <+: ns.val[k].val) :
    find.next_from_loop ns q j ⦃ r => FirstFrom ns.val q.val a0.val r ⦄ := by
  unfold find.next_from_loop
  apply loop.spec_decr_nat (fun j : Usize => ns.val.length - j.val)
    (fun j : Usize => j.val ≤ ns.val.length ∧ a0.val ≤ j.val ∧
      ∀ k (hk : k < ns.val.length), a0.val ≤ k → k < j.val → ¬ q.val <+: ns.val[k].val)
  · intro j ⟨hj, hat, hno⟩
    unfold find.next_from_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ v, hv ⟩
      step with starts_spec as ⟨ b, hb ⟩
      rw [deref_val] at hb
      split
      · rename_i hbt
        simp only [WP.spec_ok, FirstFrom]
        exact ⟨by scalar_tac, hat, by rw [← hv]; exact hb.mp hbt, hno⟩
      · rename_i hbt
        step as ⟨ j1, hj1 ⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        intro k hk h1 h2
        by_cases hkj : k < j.val
        · exact hno k hk h1 hkj
        · have : k = j.val := by scalar_tac
          subst this
          rw [← hv]; exact fun h => hbt (hb.mpr h)
    · rename_i hge
      simp only [WP.spec_ok, FirstFrom]
      intro k hk h1
      exact hno k hk h1 (by scalar_tac)
  · exact ⟨hj, hat, hno⟩

theorem next_from_spec (ns : Slice (alloc.vec.Vec U32)) (q : Slice U32) (a0 : Usize) :
    find.next_from ns q a0 ⦃ r => FirstFrom ns.val q.val a0.val r ⦄ := by
  unfold find.next_from
  by_cases h : a0.val ≤ ns.val.length
  · exact next_from_loop_spec ns q a0 a0 h (le_refl _) (fun k _ h1 h2 => absurd h2 (by omega))
  · unfold find.next_from_loop
    rw [loop]
    unfold find.next_from_loop.body
    have : ¬ a0 < ns.len := by scalar_tac
    simp only [this, ↓reduceIte, bind_ok]
    simp only [WP.spec_ok, FirstFrom]
    intro k hk h1; exfalso; omega

theorem prefixed_loop_spec (qs ns : Slice (alloc.vec.Vec U32)) (a0 k : Usize)
    (hk : k.val ≤ qs.val.length) (hat : a0.val ≤ ns.val.length) (hka : k.val ≤ a0.val)
    (hemb : Emb qs.val ns.val ↔ Emb (qs.val.drop k.val) (ns.val.drop a0.val)) :
    find.prefixed_loop qs ns a0 k ⦃ r =>
      (r = none ↔ ¬ Emb qs.val ns.val) ∧
      ∀ c, r = some c → c.val = ns.val.length - qs.val.length ⦄ := by
  unfold find.prefixed_loop
  apply loop.spec_decr_nat (fun (x : Usize × Usize) => qs.val.length - x.2.val)
    (fun (x : Usize × Usize) => x.2.val ≤ qs.val.length ∧ x.1.val ≤ ns.val.length ∧
      x.2.val ≤ x.1.val ∧
      (Emb qs.val ns.val ↔ Emb (qs.val.drop x.2.val) (ns.val.drop x.1.val)))
  · rintro ⟨a0, k⟩ ⟨hk, hat, hka, hemb⟩
    dsimp only at hk hat hka hemb
    unfold find.prefixed_loop.body
    dsimp only
    split
    · rename_i hlt
      have hkq : k.val < qs.val.length := by scalar_tac
      step as ⟨ v, hv ⟩
      step with next_from_spec as ⟨ o, ho ⟩
      rw [deref_val] at ho
      have hdrop : qs.val.drop k.val = v :: qs.val.drop (k.val + 1) := by
        rw [hv]; exact List.drop_eq_getElem_cons hkq
      rcases o with _ | j
      · simp only [WP.spec_ok, true_iff, reduceCtorEq, false_implies, implies_true, and_true]
        rw [hemb, hdrop]
        exact emb_none ho
      · obtain ⟨hjl, hjat, hjp, hjno⟩ := ho
        simp only
        step as ⟨ a1, ha1 ⟩
        step as ⟨ k1, hk1 ⟩
        refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [hemb, hdrop, ha1, hk1, emb_first hjl hjat hjp hjno]
    · rename_i hge
      have hkq : k.val = qs.val.length := by scalar_tac
      step as ⟨ c, hc ⟩
      simp only [WP.spec_ok, reduceCtorEq, false_iff, not_not, Option.some.injEq]
      refine ⟨?_, fun c' hc' => by rw [← hc', hc]; simp [Slice.len]⟩
      rw [hemb, hkq, List.drop_length]
      exact .nil
  · exact ⟨hk, hat, hka, hemb⟩

/-- A query matches a name exactly when it may match by prefix and its
tokens begin distinct name tokens in order; the answer counts the name
tokens left unmatched. -/
theorem prefixed_spec (qs ns : Slice (alloc.vec.Vec U32)) :
    find.prefixed qs ns ⦃ r =>
      (r = none ↔ ¬ (QueryOk qs.val ∧ Emb qs.val ns.val)) ∧
      ∀ c, r = some c → c.val = ns.val.length - qs.val.length ⦄ := by
  unfold find.prefixed
  step with query_ok_spec as ⟨ b, hb ⟩
  split
  · rename_i hbt
    apply WP.spec_mono (prefixed_loop_spec qs ns 0#usize 0#usize (by simp) (by simp) (by simp)
      (by simp))
    intro r ⟨h1, h2⟩
    exact ⟨by rw [h1]; simp [hb.mp hbt], h2⟩
  · rename_i hbt
    simp only [WP.spec_ok, true_iff, not_and, reduceCtorEq, false_implies, implies_true,
      and_true]
    intro hq; exact absurd (hb.mpr hq) hbt

end Writfold.Core.Find
