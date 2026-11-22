import Aeneas

/-!
# Wildcard matching with one backtrack point

The definition both scope matchers are proved against, over any pattern
alphabet with a wildcard predicate `S` and a one-to-one match `L`, and the
loop invariant of the two-index matcher with one backtrack point that
`writfold_core::scope::char_step` and `path_step` both run: from any
cursor that satisfies `Inv`, each step either keeps `Inv` with a smaller
measure or stops with the answer of `WMatch`.

`WMatch S L p t`: the pattern `p` matches the whole text `t`. A wildcard
(`S a`) matches any run of text, none included; any other pattern element
`a` matches exactly one text element `b` with `L a b`.
-/

open Aeneas Aeneas.Std Result

namespace Writfold.Core.Wild

variable {α β : Type}

/-- The pattern matches the whole text. -/
inductive WMatch (S : α → Prop) (L : α → β → Prop) : List α → List β → Prop
  /-- The empty pattern matches the empty text. -/
  | nil : WMatch S L [] []
  /-- A non-wildcard element matches one text element. -/
  | one {a b p t} : ¬ S a → L a b → WMatch S L p t → WMatch S L (a :: p) (b :: t)
  /-- A wildcard matches nothing. -/
  | none {a p t} : S a → WMatch S L p t → WMatch S L (a :: p) t
  /-- A wildcard stretches over one more text element. -/
  | more {a b p t} : S a → WMatch S L (a :: p) t → WMatch S L (a :: p) (b :: t)

variable {S : α → Prop} {L : α → β → Prop}

theorem nil_left {t : List β} : WMatch S L [] t ↔ t = [] := by
  constructor
  · intro h; cases h; rfl
  · rintro rfl; exact .nil

theorem nil_right {p : List α} : WMatch S L p [] ↔ ∀ a ∈ p, S a := by
  induction p with
  | nil => simp [WMatch.nil]
  | cons a p ih =>
    constructor
    · intro h
      cases h with
      | none hs h =>
        intro x hx
        rcases List.mem_cons.mp hx with rfl | hx
        · exact hs
        · exact ih.mp h x hx
    · intro h
      exact .none (h a (by simp)) (ih.mpr (fun x hx => h x (by simp [hx])))

theorem star_iff {a : α} {p : List α} {t : List β} (hs : S a) :
    WMatch S L (a :: p) t ↔ ∃ k, k ≤ t.length ∧ WMatch S L p (t.drop k) := by
  induction t with
  | nil =>
    constructor
    · intro h
      cases h with
      | none _ h => exact ⟨0, le_refl _, h⟩
    · rintro ⟨k, hk, h⟩
      simp at hk; subst hk
      exact .none hs (by simpa using h)
  | cons b t ih =>
    constructor
    · intro h
      cases h with
      | one hn => exact absurd hs hn
      | none _ h => exact ⟨0, by simp, h⟩
      | more _ h =>
        obtain ⟨k, hk, h'⟩ := ih.mp h
        exact ⟨k + 1, by simp; omega, by simpa using h'⟩
    · rintro ⟨k, hk, h⟩
      cases k with
      | zero => exact .none hs (by simpa using h)
      | succ k => exact .more hs (ih.mpr ⟨k, by simp at hk; omega, by simpa using h⟩)

theorem lit_iff {a : α} {p : List α} {t : List β} (hs : ¬ S a) :
    WMatch S L (a :: p) t ↔ ∃ b t', t = b :: t' ∧ L a b ∧ WMatch S L p t' := by
  constructor
  · intro h
    cases h with
    | one _ hl h => exact ⟨_, _, rfl, hl, h⟩
    | none h' => exact absurd h' hs
    | more h' => exact absurd h' hs
  · rintro ⟨b, t', rfl, hl, h⟩
    exact .one hs hl h

theorem drop_some {γ : Type} {l : List γ} {i : Nat} {x : γ} (h : l[i]? = some x) :
    l.drop i = x :: l.drop (i + 1) := by
  obtain ⟨hi, rfl⟩ := List.getElem?_eq_some_iff.mp h
  exact List.drop_eq_getElem_cons hi

/-- Positions `a0 .. a0+n` of the pattern are non-wildcards, each matching
the text at `f .. f+n`. -/
def Blk (S : α → Prop) (L : α → β → Prop) (P : List α) (T : List β) (a0 f n : Nat) : Prop :=
  ∀ m, m < n → ∃ a b, P[a0 + m]? = some a ∧ T[f + m]? = some b ∧ ¬ S a ∧ L a b

theorem blk_iff {P : List α} {T : List β} {a0 f n : Nat} (h : Blk S L P T a0 f n) :
    WMatch S L (P.drop a0) (T.drop f) ↔ WMatch S L (P.drop (a0 + n)) (T.drop (f + n)) := by
  induction n generalizing a0 f with
  | zero => simp
  | succ n ih =>
    obtain ⟨a, b, ha, hb, hs, hl⟩ := h 0 (by omega)
    simp only [Nat.add_zero] at ha hb
    rw [drop_some ha, drop_some hb, lit_iff hs]
    have hshift : Blk S L P T (a0 + 1) (f + 1) n := by
      intro m hm
      obtain ⟨a', b', ha', hb', hs', hl'⟩ := h (m + 1) (by omega)
      exact ⟨a', b', by rw [← ha']; congr 1; omega, by rw [← hb']; congr 1; omega, hs', hl'⟩
    rw [show a0 + (n + 1) = a0 + 1 + n by omega, show f + (n + 1) = f + 1 + n by omega,
      ← ih hshift]
    constructor
    · rintro ⟨b', t', he, _, hw⟩
      cases he; exact hw
    · intro hw; exact ⟨b, _, rfl, hl, hw⟩

theorem blk_len {P : List α} {a0 n : Nat} {w : List β}
    (hns : ∀ m, m < n → ∃ a, P[a0 + m]? = some a ∧ ¬ S a)
    (h : WMatch S L (P.drop a0) w) :
    n ≤ w.length ∧ WMatch S L (P.drop (a0 + n)) (w.drop n) := by
  induction n generalizing a0 w with
  | zero => simpa using h
  | succ n ih =>
    obtain ⟨a, ha, hs⟩ := hns 0 (by omega)
    simp only [Nat.add_zero] at ha
    rw [drop_some ha, lit_iff hs] at h
    obtain ⟨b, w', rfl, _, hw⟩ := h
    have hshift : ∀ m, m < n → ∃ a, P[a0 + 1 + m]? = some a ∧ ¬ S a := by
      intro m hm
      obtain ⟨a', ha', hs'⟩ := hns (m + 1) (by omega)
      exact ⟨a', by rw [← ha']; congr 1; omega, hs'⟩
    obtain ⟨hlen, hrest⟩ := ih hshift hw
    refine ⟨by simp; omega, ?_⟩
    rw [show a0 + (n + 1) = a0 + 1 + n by omega]
    simpa using hrest

theorem blk_ns {P : List α} {T : List β} {a0 f n : Nat} (h : Blk S L P T a0 f n) :
    ∀ m, m < n → ∃ a, P[a0 + m]? = some a ∧ ¬ S a := by
  intro m hm
  obtain ⟨a, _, ha, _, hs, _⟩ := h m hm
  exact ⟨a, ha, hs⟩

/-- What a matcher's cursor knows: positions `i` in the pattern and `j` in
the text. Without a wildcard seen, the whole match is the match of what
is left. With one at `w0` whose stretch ends at `f`, the elements after it
up to `i` matched the text from `f` to `j` one by one, and the whole match
is a match of everything after the wildcard from some point at or past
`f`. -/
def Inv (S : α → Prop) (L : α → β → Prop) (P : List α) (T : List β)
    (i j : Nat) (st : Bool) (w0 f : Nat) : Prop :=
  i ≤ P.length ∧ j ≤ T.length ∧ f ≤ j ∧
  (st = false → (WMatch S L P T ↔ WMatch S L (P.drop i) (T.drop j))) ∧
  (st = true → ∃ n, i = w0 + 1 + n ∧ j = f + n ∧ Blk S L P T (w0 + 1) f n ∧
    (WMatch S L P T ↔ ∃ k, f ≤ k ∧ k ≤ T.length ∧ WMatch S L (P.drop (w0 + 1)) (T.drop k)))

theorem inv_init (P : List α) (T : List β) : Inv S L P T 0 0 false 0 0 := by
  refine ⟨by omega, by omega, le_refl _, fun _ => by simp, (fun h => by cases h)⟩

/-- After a wildcard at `i`, from any text position `x`. -/
theorem star_from {P : List α} {T : List β} {i x : Nat} {a : α}
    (ha : P[i]? = some a) (hs : S a) (hx : x ≤ T.length) :
    WMatch S L (P.drop i) (T.drop x) ↔
      ∃ k, x ≤ k ∧ k ≤ T.length ∧ WMatch S L (P.drop (i + 1)) (T.drop k) := by
  rw [drop_some ha, star_iff hs]
  constructor
  · rintro ⟨k, hk, hw⟩
    refine ⟨x + k, by omega, by simp at hk; omega, by simpa [List.drop_drop] using hw⟩
  · rintro ⟨k, hk1, hk2, hw⟩
    refine ⟨k - x, by simp; omega, ?_⟩
    rw [List.drop_drop, show x + (k - x) = k by omega]
    exact hw

/-- A wildcard is recorded: its stretch starts empty, at `j`. -/
theorem inv_star {P : List α} {T : List β} {i j w0 f : Nat} {st : Bool} {a : α}
    (h : Inv S L P T i j st w0 f) (ha : P[i]? = some a) (hs : S a) :
    Inv S L P T (i + 1) j true i j := by
  obtain ⟨hi, hj, hf, hfalse, htrue⟩ := h
  have hlt : i < P.length := (List.getElem?_eq_some_iff.mp ha).1
  refine ⟨by omega, hj, le_refl _, (fun h => by cases h), fun _ => ⟨0, by omega, by omega,
    fun m hm => absurd hm (by omega), ?_⟩⟩
  cases st with
  | false => rw [hfalse rfl, star_from ha hs hj]
  | true =>
    obtain ⟨n, rfl, rfl, hblk, hw⟩ := htrue rfl
    rw [hw]
    constructor
    · rintro ⟨k, hk1, hk2, hk⟩
      obtain ⟨hlen, hk'⟩ := blk_len (blk_ns hblk) hk
      simp only [List.drop_drop, List.length_drop] at hk' hlen
      obtain ⟨k2, hk2a, hk2b, hk2c⟩ := (star_from ha hs (by omega)).mp hk'
      exact ⟨k2, by omega, hk2b, hk2c⟩
    · intro hk
      exact ⟨f, le_refl _, by omega, (blk_iff hblk).mpr ((star_from ha hs hj).mpr hk)⟩

/-- A non-wildcard matches the next text element. -/
theorem inv_lit {P : List α} {T : List β} {i j w0 f : Nat} {st : Bool} {a : α} {b : β}
    (h : Inv S L P T i j st w0 f) (ha : P[i]? = some a) (hb : T[j]? = some b)
    (hs : ¬ S a) (hl : L a b) :
    Inv S L P T (i + 1) (j + 1) st w0 f := by
  obtain ⟨hi, hj, hf, hfalse, htrue⟩ := h
  have hlt : i < P.length := (List.getElem?_eq_some_iff.mp ha).1
  have hlt' : j < T.length := (List.getElem?_eq_some_iff.mp hb).1
  refine ⟨by omega, by omega, by omega, fun hst => ?_, fun hst => ?_⟩
  · rw [hfalse hst, drop_some ha, drop_some hb, lit_iff hs]
    constructor
    · rintro ⟨b', t', he, _, hw⟩
      cases he; exact hw
    · intro hw; exact ⟨b, _, rfl, hl, hw⟩
  · obtain ⟨n, rfl, rfl, hblk, hw⟩ := htrue hst
    refine ⟨n + 1, by omega, by omega, ?_, hw⟩
    intro m hm
    by_cases hmn : m < n
    · exact hblk m hmn
    · have : m = n := by omega
      subst this
      exact ⟨a, b, ha, hb, hs, hl⟩

/-- No element of the pattern at `i` can take the text element at `j`:
the pattern ended, or it is a non-wildcard that does not match. -/
def Stuck (S : α → Prop) (L : α → β → Prop) (P : List α) (T : List β) (i j : Nat) : Prop :=
  ∀ a, P[i]? = some a → ¬ S a ∧ ∀ b, T[j]? = some b → ¬ L a b

theorem stuck_never {P : List α} {T : List β} {i j : Nat} (hj : j < T.length)
    (h : Stuck S L P T i j) : ¬ WMatch S L (P.drop i) (T.drop j) := by
  have hb : T[j]? = some T[j] := List.getElem?_eq_getElem hj
  rw [drop_some hb]
  cases ha : P[i]? with
  | none =>
    rw [List.drop_eq_nil_of_le (by simpa using ha), nil_left]
    simp; omega
  | some a =>
    obtain ⟨hs, hl⟩ := h a ha
    rw [drop_some ha, lit_iff hs]
    rintro ⟨b', t', he, hl', _⟩
    cases he
    exact hl T[j] hb hl'

/-- The last wildcard stretches by one: its stretch ending at `f` led
nowhere. -/
theorem inv_back {P : List α} {T : List β} {i j w0 f : Nat}
    (h : Inv S L P T i j true w0 f) (hj : j < T.length) (hno : Stuck S L P T i j) :
    Inv S L P T (w0 + 1) (f + 1) true w0 (f + 1) := by
  obtain ⟨hi, _, hf, _, htrue⟩ := h
  obtain ⟨n, rfl, rfl, hblk, hw⟩ := htrue rfl
  refine ⟨by omega, by omega, le_refl _, (fun h => by cases h), fun _ => ⟨0, by omega, by omega,
    fun m hm => absurd hm (by omega), ?_⟩⟩
  have hnot : ¬ WMatch S L (P.drop (w0 + 1)) (T.drop f) := by
    rw [blk_iff hblk]; exact stuck_never hj hno
  rw [hw]
  constructor
  · rintro ⟨k, hk1, hk2, hk⟩
    have : k ≠ f := by rintro rfl; exact hnot hk
    exact ⟨k, by omega, hk2, hk⟩
  · rintro ⟨k, hk1, hk2, hk⟩
    exact ⟨k, by omega, hk2, hk⟩

/-- With no wildcard seen and nothing to take the next text element,
nothing matches. -/
theorem inv_fail {P : List α} {T : List β} {i j w0 f : Nat}
    (h : Inv S L P T i j false w0 f) (hj : j < T.length) (hno : Stuck S L P T i j) :
    ¬ WMatch S L P T := by
  rw [h.2.2.2.1 rfl]
  exact stuck_never hj hno

/-- At the end of the text, with no wildcard left at `i`: a match exactly
when the pattern ended too. -/
theorem inv_exit {P : List α} {T : List β} {i j w0 f : Nat} {st : Bool}
    (h : Inv S L P T i j st w0 f) (hj : T.length ≤ j)
    (hno : ∀ a, P[i]? = some a → ¬ S a) :
    (WMatch S L P T ↔ i = P.length) := by
  obtain ⟨hi, hj', hf, hfalse, htrue⟩ := h
  have hend : WMatch S L (P.drop i) [] ↔ i = P.length := by
    rw [nil_right]
    constructor
    · intro hall
      by_contra hne
      have hlt : i < P.length := by omega
      exact (hno P[i] (List.getElem?_eq_getElem hlt)) (hall P[i] (by
        rw [List.mem_drop_iff_getElem]; exact ⟨0, by simp; omega, by simp⟩))
    · intro he a ha
      rw [he, List.drop_length] at ha
      simp at ha
  have hTj : T.drop j = [] := List.drop_eq_nil_of_le hj
  cases st with
  | false => rw [hfalse rfl, hTj, hend]
  | true =>
    obtain ⟨n, rfl, rfl, hblk, hw⟩ := htrue rfl
    rw [hw, ← hend]
    constructor
    · rintro ⟨k, hk1, hk2, hk⟩
      by_cases hkf : k = f
      · subst hkf
        rw [blk_iff hblk, hTj] at hk
        exact hk
      · obtain ⟨hlen, _⟩ := blk_len (blk_ns hblk) hk
        simp at hlen
        omega
    · intro hk
      refine ⟨f, le_refl _, by omega, ?_⟩
      rw [blk_iff hblk, hTj]
      exact hk

end Writfold.Core.Wild
