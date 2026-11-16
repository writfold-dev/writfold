import Writfold.Bytes

/-! Stable fusion over the extracted Rust. Rank bounds are explicit: at most 50
records per input, zero denotes absent overlap. RRF comparisons use positive
integer denominators with k=60; `rrfWins` is their cross-multiplied inequality.
Lexical order is the supplied order, including signature priority. Therefore
an exact hit is first only when it is the leading supplied lexical hit. -/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.SemanticOrder

def Valid (x : semantic_order.Lexical) : Prop :=
  x.rank.val ≤ 50 ∧ x.semantic_rank.val ≤ 50

def rrfWins (l o s : Nat) : Prop :=
  if o = 0 then 60 + s < 60 + l
  else (120 + l + o) * (60 + s) < (60 + l) * (60 + o)

def Blocked (x : semantic_order.Lexical) (s : Nat) : Prop :=
  x.exact = true ∨ ¬ rrfWins x.rank.val x.semantic_rank.val s

theorem semantic_wins_spec (l o s : U32)
    (hl : l.val ≤ 50) (ho : o.val ≤ 50) (hs : s.val ≤ 50) :
    semantic_order.semantic_wins l o s ⦃ b => b = true ↔ rrfWins l.val o.val s.val ⦄ := by
  unfold semantic_order.semantic_wins
  split
  · scalar_tac
  split
  · scalar_tac
  split
  · scalar_tac
  split
  · rename_i hz
    simp only [WP.spec_ok]
    simp [rrfWins, show o.val = 0 by scalar_tac]
  · rename_i hn
    have hn' : o.val ≠ 0 := by scalar_tac
    step as ⟨a, ha⟩
    step as ⟨b, hb⟩
    step as ⟨c, hc⟩
    have hab : a.val * b.val ≤ 12100 :=
      Nat.mul_le_mul (by scalar_tac : a.val ≤ 110) (by scalar_tac : b.val ≤ 110)
    step as ⟨d, hd⟩
    step as ⟨e, he⟩
    have hec : e.val * c.val ≤ 24200 :=
      Nat.mul_le_mul (by scalar_tac : e.val ≤ 220) (by scalar_tac : c.val ≤ 110)
    step as ⟨f, hf⟩
    have ha' : a.val = 60 + l.val := by scalar_tac
    have hb' : b.val = 60 + o.val := by scalar_tac
    have hc' : c.val = 60 + s.val := by scalar_tac
    have he' : e.val = 120 + l.val + o.val := by scalar_tac
    simp [rrfWins, hn']
    scalar_tac

/-- Invalid bounded-rank input cannot overflow or permit semantic insertion. -/
theorem invalid_rank_refused (l o s : U32)
    (h : 50 < l.val ∨ 50 < o.val ∨ 50 < s.val) :
    semantic_order.semantic_wins l o s ⦃ b => b = false ⦄ := by
  unfold semantic_order.semantic_wins
  split
  · simp
  split
  · simp
  split
  · simp
  · scalar_tac

theorem blocks_spec (x : semantic_order.Lexical) (s : U32)
    (hx : Valid x) (hs : s.val ≤ 50) :
    semantic_order.blocks x s ⦃ b => b = true ↔ Blocked x s.val ⦄ := by
  unfold semantic_order.blocks
  cases he : x.exact
  · simp only [he, Bool.false_eq_true, ↓reduceIte]
    step with semantic_wins_spec x.rank x.semantic_rank s hx.1 hx.2 hs as ⟨b, hb⟩
    cases b <;> simp_all [Blocked]
  · simp [he, Blocked]

/-- Exact protection does not depend on numeric rank bounds. -/
theorem exact_blocks (x : semantic_order.Lexical) (s : U32) (hx : x.exact = true) :
    semantic_order.blocks x s ⦃ b => b = true ⦄ := by
  simp [semantic_order.blocks, hx]

/-- Equal lexical/semantic single-list scores keep the lexical candidate first. -/
theorem rrf_tie_lexical (r : U32) (hr : r.val ≤ 50) :
    semantic_order.semantic_wins r 0#u32 r ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (semantic_wins_spec r 0#u32 r hr (by simp) hr)
  intro b hb
  cases b <;> simp_all [rrfWins]

/-- The slot is exactly after the last blocking lexical candidate. -/
def SlotSpec (xs : List semantic_order.Lexical) (s slot : Nat) : Prop :=
  slot ≤ xs.length ∧
  (∀ j (hj : j < xs.length), Blocked xs[j] s → j < slot) ∧
  (slot = 0 ∨ ∃ j, ∃ hj : j < xs.length, slot = j + 1 ∧ Blocked xs[j] s)

theorem semantic_slot_loop_spec (xs : Slice semantic_order.Lexical) (s : U32)
    (slot i : Usize) (hs : s.val ≤ 50) (hv : ∀ x ∈ xs.val, Valid x)
    (hi : i.val ≤ xs.val.length) (hslot : slot.val ≤ i.val)
    (hpre : ∀ j (hj : j < i.val) (hj' : j < xs.val.length),
      Blocked xs.val[j] s.val → j < slot.val)
    (hlast : slot.val = 0 ∨ ∃ j, ∃ hj : j < xs.val.length,
      slot.val = j + 1 ∧ Blocked xs.val[j] s.val) :
    semantic_order.semantic_slot_loop xs s slot i ⦃ r => SlotSpec xs.val s.val r.val ⦄ := by
  unfold semantic_order.semantic_slot_loop
  apply loop.spec_decr_nat (fun p : Usize × Usize => xs.val.length - p.2.val)
    (fun p : Usize × Usize => p.2.val ≤ xs.val.length ∧ p.1.val ≤ p.2.val ∧
      (∀ j (hj : j < p.2.val) (hj' : j < xs.val.length),
        Blocked xs.val[j] s.val → j < p.1.val) ∧
      (p.1.val = 0 ∨ ∃ j, ∃ hj : j < xs.val.length,
        p.1.val = j + 1 ∧ Blocked xs.val[j] s.val))
  · rintro ⟨slot, i⟩ ⟨hi, hslot, hpre, hlast⟩
    unfold semantic_order.semantic_slot_loop.body
    dsimp only at hi hslot hpre hlast ⊢
    split
    · rename_i hlt
      have hi' : i.val < xs.val.length := by scalar_tac
      step as ⟨next, hn⟩
      have hn' : next.val = i.val + 1 := by scalar_tac
      step as ⟨x, hx⟩
      have hxv : Valid x := by rw [hx]; exact hv _ (List.getElem_mem hi')
      step with blocks_spec x s hxv hs as ⟨b, hb⟩
      split
      · rename_i hbt
        have hblocked : Blocked xs.val[i.val] s.val := by rw [← hx]; exact hb.mp hbt
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, by scalar_tac, ?_, ?_, by scalar_tac⟩
        · intro j hj hj' _; scalar_tac
        · exact Or.inr ⟨i.val, hi', hn', hblocked⟩
      · rename_i hbf
        have hnot : ¬ Blocked xs.val[i.val] s.val := by
          rw [← hx]; intro h; exact hbf (hb.mpr h)
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, by scalar_tac, ?_, hlast, by scalar_tac⟩
        intro j hj hj' hblock
        by_cases hji : j < i.val
        · exact hpre j hji hj' hblock
        · have heq : j = i.val := by scalar_tac
          subst j
          exact False.elim (hnot hblock)
    · rename_i hge
      simp only [WP.spec_ok]
      refine ⟨by scalar_tac, ?_, hlast⟩
      intro j hj hblock
      exact hpre j (by scalar_tac) hj hblock
  · exact ⟨hi, hslot, hpre, hlast⟩

theorem semantic_slot_spec (xs : Slice semantic_order.Lexical) (s : U32)
    (hs : s.val ≤ 50) (hv : ∀ x ∈ xs.val, Valid x) :
    semantic_order.semantic_slot xs s true ⦃ r => SlotSpec xs.val s.val r.val ⦄ := by
  unfold semantic_order.semantic_slot
  simp only [Bool.not_true, Bool.false_eq_true, ↓reduceIte]
  apply semantic_slot_loop_spec xs s 0#usize 0#usize hs hv
  · simp
  · simp
  · intro j hj; simp at hj
  · exact Or.inl (by simp)

theorem lexical_first (xs : Slice semantic_order.Lexical) (s : U32) :
    semantic_order.semantic_slot xs s false ⦃ r => r.val = xs.val.length ⦄ := by
  simp [semantic_order.semantic_slot, Slice.length]

def KeyLt (a b : semantic_order.Key) : Prop :=
  a.slot.val < b.slot.val ∨ (a.slot.val = b.slot.val ∧
    ((a.semantic = true ∧ b.semantic = false) ∨
      (a.semantic = b.semantic ∧ a.rank.val < b.rank.val)))

theorem before_spec (a b : semantic_order.Key) :
    semantic_order.before a b ⦃ r => r = true ↔ KeyLt a b ⦄ := by
  unfold semantic_order.before
  unfold KeyLt
  split
  · rename_i hne
    simp only [WP.spec_ok]
    scalar_tac
  · rename_i heq
    cases ha : a.semantic <;> cases hb : b.semantic <;>
      simp [ha, hb] <;> scalar_tac

/-- The actual comparator is transitive, unlike mixed ordinal/score comparators. -/
theorem key_transitive (a b c : semantic_order.Key)
    (hab : KeyLt a b) (hbc : KeyLt b c) : KeyLt a c := by
  cases ha : a.semantic <;> cases hb : b.semantic <;> cases hc : c.semantic <;>
    simp_all [KeyLt] <;> omega

/-- Keys assigned by the extracted lexical constructor preserve every ordinal. -/
theorem lexical_order_preserved (i j : Usize) (h : i.val < j.val) :
    (do let a ← semantic_order.lexical_key i
        let b ← semantic_order.lexical_key j
        semantic_order.before a b) ⦃ r => r = true ⦄ := by
  simp only [semantic_order.lexical_key, bind_ok]
  apply WP.spec_mono (before_spec _ _)
  intro r hr
  exact hr.mpr (Or.inl h)

/-- Any exact lexical hit precedes every semantic-only hit, even a later exact. -/
theorem exact_protected (xs : Slice semantic_order.Lexical) (s : U32)
    (hs : s.val ≤ 50) (hv : ∀ x ∈ xs.val, Valid x)
    (j : Usize) (hj : j.val < xs.val.length) (he : xs.val[j.val].exact = true) :
    (do let slot ← semantic_order.semantic_slot xs s true
        let a ← semantic_order.lexical_key j
        let b ← semantic_order.semantic_key slot s
        semantic_order.before a b) ⦃ r => r = true ⦄ := by
  step with semantic_slot_spec xs s hs hv as ⟨slot, hslot⟩
  simp only [semantic_order.lexical_key, semantic_order.semantic_key, bind_ok]
  apply WP.spec_mono (before_spec _ _)
  intro r hr
  exact hr.mpr (Or.inl (hslot.2.1 j.val hj (Or.inl he)))

/-- Equal slots retain the semantic input's stable rank/ID order. -/
theorem semantic_ties_stable (slot : Usize) (r t : U32) (h : r.val < t.val) :
    (do let a ← semantic_order.semantic_key slot r
        let b ← semantic_order.semantic_key slot t
        semantic_order.before a b) ⦃ b => b = true ⦄ := by
  simp only [semantic_order.semantic_key, bind_ok]
  apply WP.spec_mono (before_spec _ _)
  intro b hb
  exact hb.mpr (Or.inr ⟨rfl, Or.inr ⟨rfl, h⟩⟩)

/-- A leading exact lexical hit is ahead of all semantic insertions. Combined
with `lexical_order_preserved`, it is first in the merged stream. -/
theorem leading_exact_first (xs : Slice semantic_order.Lexical) (s : U32)
    (hs : s.val ≤ 50) (hv : ∀ x ∈ xs.val, Valid x)
    (hne : 0 < xs.val.length) (he : (xs.val[0]'hne).exact = true) :
    semantic_order.semantic_slot xs s true ⦃ slot => 0 < slot.val ⦄ := by
  apply WP.spec_mono (semantic_slot_spec xs s hs hv)
  intro slot hslot
  exact hslot.2.1 0 hne (Or.inl he)

/-- RRF genuinely permits insertion; it is not lexical-first under a new name. -/
theorem rrf_insertion_enabled :
    semantic_order.semantic_wins 2#u32 0#u32 1#u32 ⦃ b => b = true ⦄ := by
  apply WP.spec_mono (semantic_wins_spec 2#u32 0#u32 1#u32 (by simp) (by simp) (by simp))
  intro b hb
  exact hb.mpr (by simp [rrfWins])

end Writfold.Core.SemanticOrder
