import Writfold.Wild

/-!
# Scope globs

Theorems over the Aeneas translation of `writfold_core::scope`: which
paths a gate's scope covers.

- `segment_iff`: one segment pattern matches one path segment exactly when
  `SegMatch` holds: `*` matches any run of characters, none included; `?`
  matches one character; any other character matches itself.
- `segments_iff`: a pattern's segments match a path's exactly when
  `PathMatch` holds: a `**` segment matches any number of segments, none
  included; any other segment matches one path segment by `SegMatch`.
- `covers_iff`: a path is covered exactly when some pattern without `!`
  matches it and no `!` pattern does. `exclusion_wins`,
  `no_pattern_no_scope` follow.

Both matchers are the same loop (`Wild.Inv`): each step keeps the
invariant and lowers `meas`, or stops with the answer of `WMatch`.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Wild

namespace Writfold.Core.Scope

/-- `*` (code point 42). -/
def IsStar (a : U32) : Prop := a = 42#u32

/-- `?` (code point 63) matches any character; any other matches itself. -/
def SameChar (a b : U32) : Prop := a = 63#u32 ∨ a = b

/-- One segment pattern matches one path segment, by code points. -/
def SegMatch (p t : List U32) : Prop := WMatch IsStar SameChar p t

/-- A `**` segment. -/
def IsGlobstar (w : alloc.vec.Vec U32) : Prop := w.val = [42#u32, 42#u32]

/-- One segment pattern matches one path segment (as vectors). -/
def SegMatchV (w q : alloc.vec.Vec U32) : Prop := SegMatch w.val q.val

/-- A pattern's segments match a path's segments. -/
def PathMatch (w ps : List (alloc.vec.Vec U32)) : Prop := WMatch IsGlobstar SegMatchV w ps

/-! ## One step of either matcher -/


variable {α β : Type} {S : α → Prop} {L : α → β → Prop}

/-- The invariant at a cursor. -/
def CInv (S : α → Prop) (L : α → β → Prop) (P : List α) (T : List β) (c : scope.Cursor) : Prop :=
  Inv S L P T c.i.val c.j.val c.starred c.at.val c.from.val

/-- Each step lowers it: the end of the last wildcard's stretch moves on,
or else the text position or the pattern position does. -/
def meas (P : List α) (T : List β) (c : scope.Cursor) : Nat :=
  (T.length - c.from.val) * (T.length + P.length + 1) + (T.length - c.j.val) +
    (P.length - c.i.val)

/-- What a step returns: a cursor that keeps the invariant with a lower
measure, or the answer. -/
def Good (S : α → Prop) (L : α → β → Prop) (P : List α) (T : List β) (c : scope.Cursor) :
    scope.Next → Prop
  | .Go c' => CInv S L P T c' ∧ meas P T c' < meas P T c
  | .Done b => (b = true ↔ WMatch S L P T)

theorem go_star {P : List α} {T : List β} {c : scope.Cursor} {a : α} {i2 : Usize}
    (h : CInv S L P T c) (ha : P[c.i.val]? = some a) (hs : S a) (hi2 : i2.val = c.i.val + 1) :
    Good S L P T c (.Go { c with i := i2, starred := true, «at» := c.i, «from» := c.j }) := by
  have hlt : c.i.val < P.length := (List.getElem?_eq_some_iff.mp ha).1
  have hf : c.from.val ≤ c.j.val := h.2.2.1
  have hj : c.j.val ≤ T.length := h.2.1
  refine ⟨?_, ?_⟩
  · have := inv_star h ha hs
    simpa [CInv, hi2] using this
  · simp only [meas, hi2]
    have := Nat.mul_le_mul_right (T.length + P.length + 1)
      (show T.length - c.j.val ≤ T.length - c.from.val by omega)
    omega

theorem go_lit {P : List α} {T : List β} {c : scope.Cursor} {a : α} {b : β} {i5 i6 : Usize}
    (h : CInv S L P T c) (ha : P[c.i.val]? = some a) (hb : T[c.j.val]? = some b)
    (hs : ¬ S a) (hl : L a b) (hi5 : i5.val = c.i.val + 1) (hi6 : i6.val = c.j.val + 1) :
    Good S L P T c (.Go { c with i := i5, j := i6 }) := by
  have hlt : c.i.val < P.length := (List.getElem?_eq_some_iff.mp ha).1
  have hlt' : c.j.val < T.length := (List.getElem?_eq_some_iff.mp hb).1
  refine ⟨?_, ?_⟩
  · have := inv_lit h ha hb hs hl
    simpa [CInv, hi5, hi6] using this
  · simp only [meas, hi5, hi6]
    omega

theorem go_back {P : List α} {T : List β} {c : scope.Cursor} {i3 i4 : Usize}
    (h : CInv S L P T c) (hst : c.starred = true) (hj : c.j.val < T.length)
    (hno : Stuck S L P T c.i.val c.j.val)
    (hi3 : i3.val = c.at.val + 1) (hi4 : i4.val = c.from.val + 1) :
    Good S L P T c (.Go { c with i := i3, j := i4, «from» := i4 }) := by
  have hf : c.from.val ≤ c.j.val := h.2.2.1
  have hb : c.at.val + 1 ≤ c.i.val := by
    obtain ⟨n, hn, _⟩ := h.2.2.2.2 hst
    omega
  have hiP : c.i.val ≤ P.length := h.1
  refine ⟨?_, ?_⟩
  · have := inv_back (by simpa [CInv, hst] using h) hj hno
    simpa [CInv, hi3, hi4, hst] using this
  · simp only [meas, hi3, hi4]
    have hsplit : T.length - c.from.val = (T.length - (c.from.val + 1)) + 1 := by omega
    rw [hsplit, Nat.add_mul, Nat.one_mul]
    generalize (T.length - (c.from.val + 1)) * (T.length + P.length + 1) = x
    omega

theorem done_exit {P : List α} {T : List β} {c : scope.Cursor} {len : Usize}
    (h : CInv S L P T c) (hj : T.length ≤ c.j.val) (hlen : len.val = P.length)
    (hno : ∀ a, P[c.i.val]? = some a → ¬ S a) :
    Good S L P T c (.Done (decide (c.i = len))) := by
  have := inv_exit h hj hno
  simp only [Good, decide_eq_true_eq, this]
  constructor
  · rintro rfl; exact hlen
  · intro he; apply UScalar.eq_of_val_eq; omega

theorem done_fail {P : List α} {T : List β} {c : scope.Cursor}
    (h : CInv S L P T c) (hst : c.starred = false) (hj : c.j.val < T.length)
    (hno : Stuck S L P T c.i.val c.j.val) :
    Good S L P T c (.Done false) := by
  have := inv_fail (by simpa [CInv, hst] using h) hj hno
  simp [Good, this]


/-! ## Within a segment -/

theorem star_val : scope.STAR = 42#u32 := by unfold scope.STAR; rfl

theorem any_val : scope.ANY = 63#u32 := by unfold scope.ANY; rfl

theorem same_char_spec (a b : U32) :
    scope.same_char a b ⦃ r => r = true ↔ SameChar a b ⦄ := by
  unfold scope.same_char
  split
  · rename_i h; simp [SameChar, h, any_val] at *
  · rename_i h; simp only [WP.spec_ok, SameChar, decide_eq_true_eq]
    rw [any_val] at h
    constructor
    · intro e; exact Or.inr e
    · rintro (e | e)
      · exact absurd e h
      · exact e

theorem start_spec {α β : Type} {S : α → Prop} {L : α → β → Prop} (P : List α) (T : List β) :
    scope.start ⦃ c => CInv S L P T c ⦄ := by
  unfold scope.start
  simp only [WP.spec_ok, CInv]
  exact inv_init P T

theorem char_step_spec (p t : Slice U32) (c : scope.Cursor)
    (h : CInv IsStar SameChar p.val t.val c) :
    scope.char_step p t c ⦃ n => Good IsStar SameChar p.val t.val c n ⦄ := by
  have hjT : c.j.val ≤ t.val.length := h.2.1
  have hfj : c.from.val ≤ c.j.val := h.2.2.1
  have hat : c.starred = true → c.at.val + 1 ≤ c.i.val := by
    intro hst; obtain ⟨n, hn, _⟩ := h.2.2.2.2 hst; omega
  unfold scope.char_step
  dsimp only
  by_cases hlt : c.i < p.len
  · simp only [hlt, ↓reduceIte]
    step as ⟨ x, hx ⟩
    have ha : p.val[c.i.val]? = some x := by
      rw [hx]; exact List.getElem?_eq_getElem (by scalar_tac)
    by_cases hstar : x = scope.STAR
    · simp only [hstar, ↓reduceIte]
      step as ⟨ i2, hi2 ⟩
      exact go_star h (hstar ▸ ha) (by simp [IsStar, star_val]) hi2
    · simp only [hstar, ↓reduceIte]
      have hns : ¬ IsStar x := by simp [IsStar, ← star_val, hstar]
      by_cases hend : c.j ≥ t.len
      · simp only [hend, ↓reduceIte]
        rw [WP.spec_ok]
        exact done_exit (len := p.len) h (by scalar_tac) (by scalar_tac)
          (fun a ha' => by rw [ha] at ha'; cases ha'; exact hns)
      · simp only [hend, ↓reduceIte]
        step as ⟨ y, hy ⟩
        have hb : t.val[c.j.val]? = some y := by
          rw [hy]; exact List.getElem?_eq_getElem (by scalar_tac)
        step with same_char_spec as ⟨ b, hbs ⟩
        by_cases hbt : b = true
        · simp only [hbt, ↓reduceIte]
          step as ⟨ i5, hi5 ⟩
          step as ⟨ i6, hi6 ⟩
          exact go_lit h ha hb hns (hbs.mp hbt) hi5 hi6
        · simp only [hbt, Bool.false_eq_true, ↓reduceIte]
          have hno : Stuck IsStar SameChar p.val t.val c.i.val c.j.val := by
            intro a ha'
            rw [ha] at ha'; cases ha'
            refine ⟨hns, fun b' hb' => ?_⟩
            rw [hb] at hb'; cases hb'
            intro hl; exact hbt (hbs.mpr hl)
          split
          · rename_i hst
            step as ⟨ i5, hi5 ⟩
            step as ⟨ i6, hi6 ⟩
            exact go_back h hst (by scalar_tac) hno hi5 hi6
          · rename_i hst
            rw [WP.spec_ok]
            exact done_fail h (by simpa using hst) (by scalar_tac) hno
  · simp only [hlt, ↓reduceIte]
    have hnone : p.val[c.i.val]? = none := List.getElem?_eq_none (by scalar_tac)
    have hno : Stuck IsStar SameChar p.val t.val c.i.val c.j.val := by
      intro a ha'; rw [hnone] at ha'; cases ha'
    by_cases hend : c.j ≥ t.len
    · simp only [hend, ↓reduceIte]
      rw [WP.spec_ok]
      exact done_exit (len := p.len) h (by scalar_tac) (by scalar_tac)
        (fun a ha' => by rw [hnone] at ha'; cases ha')
    · simp only [hend, ↓reduceIte]
      split
      · rename_i hst
        step as ⟨ i3, hi3 ⟩
        step as ⟨ i4, hi4 ⟩
        exact go_back h hst (by scalar_tac) hno hi3 hi4
      · rename_i hst
        rw [WP.spec_ok]
        exact done_fail h (by simpa using hst) (by scalar_tac) hno

theorem segment_loop_spec (p t : Slice U32) (c : scope.Cursor)
    (h : CInv IsStar SameChar p.val t.val c) :
    scope.segment_loop c p t ⦃ b => b = true ↔ SegMatch p.val t.val ⦄ := by
  unfold scope.segment_loop
  apply loop.spec_decr_nat (fun c => meas p.val t.val c)
    (fun c => CInv IsStar SameChar p.val t.val c)
  · intro c hc
    unfold scope.segment_loop.body
    step with char_step_spec as ⟨ n, hn ⟩
    cases n with
    | Go c' => simp only [WP.spec_ok]; exact hn
    | Done b => simp only [WP.spec_ok]; exact hn
  · exact h

/-- One segment pattern matches one path segment exactly when `SegMatch`
holds. -/
theorem segment_iff (p t : Slice U32) :
    scope.segment p t ⦃ b => b = true ↔ SegMatch p.val t.val ⦄ := by
  unfold scope.segment
  step with (start_spec (S := IsStar) (L := SameChar) p.val t.val) as ⟨ c, hc ⟩
  exact segment_loop_spec p t c hc

/-! ## Across segments -/

theorem globstar_spec (s : Slice U32) :
    scope.globstar s ⦃ b => b = true ↔ s.val = [42#u32, 42#u32] ⦄ := by
  unfold scope.globstar
  dsimp only
  split
  · rename_i hlen
    have hl : s.val.length = 2 := by scalar_tac
    step as ⟨ x, hx ⟩
    split
    · rename_i hx42
      step as ⟨ y, hy ⟩
      simp only [decide_eq_true_eq, star_val]
      constructor
      · intro hy42
        apply List.ext_getElem (by simp [hl])
        intro n h1 h2
        have : n = 0 ∨ n = 1 := by omega
        rcases this with rfl | rfl
        · simp [← hx, hx42, star_val]
        · simp [← hy, hy42]
      · intro he
        rw [hy]; simp [he]
    · rename_i hx42
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      apply hx42
      rw [hx, star_val]; simp [he]
  · rename_i hlen
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro he
    apply hlen
    have : s.val.length = 2 := by simp [he]
    scalar_tac

theorem deref_val {γ : Type} (v : alloc.vec.Vec γ) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref, Slice.from_val]

theorem path_step_spec (w ps : Slice (alloc.vec.Vec U32)) (c : scope.Cursor)
    (h : CInv IsGlobstar SegMatchV w.val ps.val c) :
    scope.path_step w ps c ⦃ n => Good IsGlobstar SegMatchV w.val ps.val c n ⦄ := by
  have hjT : c.j.val ≤ ps.val.length := h.2.1
  have hfj : c.from.val ≤ c.j.val := h.2.2.1
  have hat : c.starred = true → c.at.val + 1 ≤ c.i.val := by
    intro hst; obtain ⟨n, hn, _⟩ := h.2.2.2.2 hst; omega
  unfold scope.path_step
  dsimp only
  by_cases hlt : c.i < w.len
  · simp only [hlt, ↓reduceIte]
    step as ⟨ x, hx ⟩
    have ha : w.val[c.i.val]? = some x := by
      rw [hx]; exact List.getElem?_eq_getElem (by scalar_tac)
    step with globstar_spec as ⟨ g, hg ⟩
    rw [deref_val] at hg
    by_cases hstar : g = true
    · simp only [hstar, ↓reduceIte]
      step as ⟨ i2, hi2 ⟩
      exact go_star h ha (hg.mp hstar) hi2
    · simp only [hstar, Bool.false_eq_true, ↓reduceIte]
      have hns : ¬ IsGlobstar x := fun e => hstar (hg.mpr e)
      by_cases hend : c.j ≥ ps.len
      · simp only [hend, ↓reduceIte]
        rw [WP.spec_ok]
        exact done_exit (len := w.len) h (by scalar_tac) (by scalar_tac)
          (fun a ha' => by rw [ha] at ha'; cases ha'; exact hns)
      · simp only [hend, ↓reduceIte]
        step as ⟨ y, hy ⟩
        have hb : ps.val[c.j.val]? = some y := by
          rw [hy]; exact List.getElem?_eq_getElem (by scalar_tac)
        step with segment_iff as ⟨ b, hbs ⟩
        rw [deref_val, deref_val] at hbs
        by_cases hbt : b = true
        · simp only [hbt, ↓reduceIte]
          step as ⟨ i5, hi5 ⟩
          step as ⟨ i6, hi6 ⟩
          exact go_lit h ha hb hns (hbs.mp hbt) hi5 hi6
        · simp only [hbt, Bool.false_eq_true, ↓reduceIte]
          have hno : Stuck IsGlobstar SegMatchV w.val ps.val c.i.val c.j.val := by
            intro a ha'
            rw [ha] at ha'; cases ha'
            refine ⟨hns, fun b' hb' => ?_⟩
            rw [hb] at hb'; cases hb'
            intro hl; exact hbt (hbs.mpr hl)
          split
          · rename_i hst
            step as ⟨ i5, hi5 ⟩
            step as ⟨ i6, hi6 ⟩
            exact go_back h hst (by scalar_tac) hno hi5 hi6
          · rename_i hst
            rw [WP.spec_ok]
            exact done_fail h (by simpa using hst) (by scalar_tac) hno
  · simp only [hlt, ↓reduceIte]
    have hnone : w.val[c.i.val]? = none := List.getElem?_eq_none (by scalar_tac)
    have hno : Stuck IsGlobstar SegMatchV w.val ps.val c.i.val c.j.val := by
      intro a ha'; rw [hnone] at ha'; cases ha'
    by_cases hend : c.j ≥ ps.len
    · simp only [hend, ↓reduceIte]
      rw [WP.spec_ok]
      exact done_exit (len := w.len) h (by scalar_tac) (by scalar_tac)
        (fun a ha' => by rw [hnone] at ha'; cases ha')
    · simp only [hend, ↓reduceIte]
      split
      · rename_i hst
        step as ⟨ i3, hi3 ⟩
        step as ⟨ i4, hi4 ⟩
        exact go_back h hst (by scalar_tac) hno hi3 hi4
      · rename_i hst
        rw [WP.spec_ok]
        exact done_fail h (by simpa using hst) (by scalar_tac) hno

theorem segments_loop_spec (w ps : Slice (alloc.vec.Vec U32)) (c : scope.Cursor)
    (h : CInv IsGlobstar SegMatchV w.val ps.val c) :
    scope.segments_loop c w ps ⦃ b => b = true ↔ PathMatch w.val ps.val ⦄ := by
  unfold scope.segments_loop
  apply loop.spec_decr_nat (fun c => meas w.val ps.val c)
    (fun c => CInv IsGlobstar SegMatchV w.val ps.val c)
  · intro c hc
    unfold scope.segments_loop.body
    step with path_step_spec as ⟨ n, hn ⟩
    cases n with
    | Go c' => simp only [WP.spec_ok]; exact hn
    | Done b => simp only [WP.spec_ok]; exact hn
  · exact h

/-- A pattern's segments match a path's exactly when `PathMatch` holds. -/
theorem segments_iff (w ps : Slice (alloc.vec.Vec U32)) :
    scope.segments w ps ⦃ b => b = true ↔ PathMatch w.val ps.val ⦄ := by
  unfold scope.segments
  step with (start_spec (S := IsGlobstar) (L := SegMatchV) w.val ps.val) as ⟨ c, hc ⟩
  exact segments_loop_spec w ps c hc

/-! ## A scope of patterns -/

/-- A path is covered: some pattern without `!` matches it and no `!`
pattern does. -/
def Covers (pats : List scope.Pattern) (path : List (alloc.vec.Vec U32)) : Prop :=
  (∃ p ∈ pats, p.exclude = false ∧ PathMatch p.segments.val path) ∧
  (∀ p ∈ pats, p.exclude = true → ¬ PathMatch p.segments.val path)

theorem include_spec (included exclude hit : Bool) :
    scope.include included exclude hit ⦃ r => r = (included || (!exclude && hit)) ⦄ := by
  unfold scope.include
  cases included <;> cases exclude <;> cases hit <;> simp

/-- What the patterns before `k` decided: some including pattern matched
(`inc`), and no excluding one did. -/
def Seen (pats : List scope.Pattern) (path : List (alloc.vec.Vec U32)) (inc : Bool) (k : Nat) :
    Prop :=
  (inc = true ↔ ∃ j q, j < k ∧ pats[j]? = some q ∧ q.exclude = false ∧
      PathMatch q.segments.val path) ∧
  (∀ j q, j < k → pats[j]? = some q → q.exclude = true → ¬ PathMatch q.segments.val path)

theorem covers_loop_spec (pats : Slice scope.Pattern) (path : Slice (alloc.vec.Vec U32))
    (inc : Bool) (k : Usize) (hk : k.val ≤ pats.val.length)
    (hseen : Seen pats.val path.val inc k.val) :
    scope.covers_loop pats path inc k ⦃ b => b = true ↔ Covers pats.val path.val ⦄ := by
  unfold scope.covers_loop
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => pats.val.length - x.2.val)
    (fun (x : Bool × Usize) => x.2.val ≤ pats.val.length ∧ Seen pats.val path.val x.1 x.2.val)
  · rintro ⟨inc, k⟩ ⟨hk, hinc, hexc⟩
    dsimp only at hk hinc hexc
    unfold scope.covers_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ q, hq ⟩
      have hqk : pats.val[k.val]? = some q := by
        rw [hq]; exact List.getElem?_eq_getElem _
      step with segments_iff as ⟨ hit, hhit ⟩
      rw [deref_val] at hhit
      -- the patterns up to and including `k`
      have hexc' : ¬ q.exclude = true ∨ ¬ hit = true →
          ∀ j q', j < k.val + 1 → pats.val[j]? = some q' → q'.exclude = true →
            ¬ PathMatch q'.segments.val path.val := by
        intro hor j q' hj hq' he
        by_cases hjk : j < k.val
        · exact hexc j q' hjk hq' he
        · have : j = k.val := by omega
          subst this
          rw [hqk] at hq'; cases hq'
          rcases hor with hor | hor
          · exact absurd he hor
          · intro hm; exact hor (hhit.mpr hm)
      have hinc' : (inc || (!q.exclude && hit)) = true ↔
          ∃ j q', j < k.val + 1 ∧ pats.val[j]? = some q' ∧ q'.exclude = false ∧
            PathMatch q'.segments.val path.val := by
        simp only [Bool.or_eq_true, Bool.and_eq_true, Bool.not_eq_eq_eq_not, Bool.not_true]
        rw [hinc]
        constructor
        · rintro (⟨j, q', hj, hq', he, hm⟩ | ⟨he, hht⟩)
          · exact ⟨j, q', by omega, hq', he, hm⟩
          · exact ⟨k.val, q, by omega, hqk, he, hhit.mp hht⟩
        · rintro ⟨j, q', hj, hq', he, hm⟩
          by_cases hjk : j < k.val
          · exact Or.inl ⟨j, q', hjk, hq', he, hm⟩
          · have : j = k.val := by omega
            subst this
            rw [hqk] at hq'; cases hq'
            exact Or.inr ⟨he, hhit.mpr hm⟩
      split
      · rename_i hex
        split
        · rename_i hht
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Covers, not_and]
          intro _ hall
          exact hall q (List.mem_of_getElem? hqk) hex (hhit.mp hht)
        · rename_i hht
          step with include_spec as ⟨ inc1, hinc1 ⟩
          step as ⟨ k1, hk1 ⟩
          refine ⟨by scalar_tac, ⟨?_, ?_⟩, by scalar_tac⟩
          · rw [hinc1, hk1]; exact hinc'
          · rw [hk1]; exact hexc' (Or.inr hht)
      · rename_i hex
        step with include_spec as ⟨ inc1, hinc1 ⟩
        step as ⟨ k1, hk1 ⟩
        refine ⟨by scalar_tac, ⟨?_, ?_⟩, by scalar_tac⟩
        · rw [hinc1, hk1]; exact hinc'
        · rw [hk1]; exact hexc' (Or.inl hex)
    · rename_i hge
      simp only [WP.spec_ok, Covers]
      have hall : k.val = pats.val.length := by scalar_tac
      rw [hinc]
      constructor
      · rintro ⟨j, q, hj, hq, he, hm⟩
        refine ⟨⟨q, List.mem_of_getElem? hq, he, hm⟩, ?_⟩
        intro p hp hpe
        obtain ⟨j', hj', rfl⟩ := List.getElem_of_mem hp
        exact hexc j' _ (by omega) (List.getElem?_eq_getElem hj') hpe
      · rintro ⟨⟨p, hp, hpe, hm⟩, _⟩
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hp
        exact ⟨j, _, by omega, List.getElem?_eq_getElem hj, hpe, hm⟩
  · exact ⟨hk, hseen⟩

/-- A path is covered exactly when some pattern without `!` matches it and
no `!` pattern does. -/
theorem covers_iff (pats : Slice scope.Pattern) (path : Slice (alloc.vec.Vec U32)) :
    scope.covers pats path ⦃ b => b = true ↔ Covers pats.val path.val ⦄ := by
  unfold scope.covers
  apply covers_loop_spec
  · simp
  · refine ⟨by simp, fun j q hj => absurd hj (by simp)⟩

/-- An exclusion that matches wins over every pattern that includes. -/
theorem exclusion_wins (pats : Slice scope.Pattern) (path : Slice (alloc.vec.Vec U32))
    (p : scope.Pattern) (hp : p ∈ pats.val) (he : p.exclude = true)
    (hm : PathMatch p.segments.val path.val) :
    scope.covers pats path ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (covers_iff pats path)
  intro b hb
  cases b
  · rfl
  · exact absurd (hb.mp rfl).2 (fun hall => hall p hp he hm)

/-- With no pattern that includes it, a path is never covered. -/
theorem no_pattern_no_scope (pats : Slice scope.Pattern) (path : Slice (alloc.vec.Vec U32))
    (h : ∀ p ∈ pats.val, p.exclude = false → ¬ PathMatch p.segments.val path.val) :
    scope.covers pats path ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (covers_iff pats path)
  intro b hb
  cases b
  · rfl
  · obtain ⟨⟨p, hp, he, hm⟩, _⟩ := hb.mp rfl
    exact absurd hm (h p hp he)

end Writfold.Core.Scope
