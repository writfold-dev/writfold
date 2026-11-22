import Extracted.WritfoldCore.Types
import Extracted.WritfoldCore.Verdict
import Writfold.Get

/-!
# Check receipts: what may be shared, how old it is, which one is used

Theorems over the Aeneas translation of `writfold_core::verdict::recorded`.

- `recordable_iff`: only what a checker said about the token itself
  (fresh, moved, changed, gone) is written to a check receipt.
- `recorded_age_spec`: a recorded verdict dated more than an hour after
  `now` is unusable; otherwise its age is `now - checked_at`, never below
  zero and saturated at the largest `i64`, over the integers.
- `before_spec`: one statement comes before another when newer, then
  when from a receipt, then with the worse verdict (`Before`).
- `best_recorded_spec`, `newest_spec`: the pick is a usable statement
  (never `unverifiable`, never a receipt without a `max_age`) ahead of
  every other usable one: by `Before`, then a receipt before the local
  statement, then the first receipt (`Ahead`). `none` exactly when no
  statement is usable. `newest_never_unverifiable` follows.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Recorded

/-- Only what a checker said about the token itself may be shared. -/
theorem recordable_iff (c : verdict.Checker) :
    verdict.recorded.recordable c ⦃ b => b = true ↔
      ∃ s, c = .Said s ∧ s ≠ .Unverifiable ⦄ := by
  unfold verdict.recorded.recordable
  rcases c with _ | _ | _ | s | _
  all_goals try simp
  cases s <;> simp

theorem skew_val : verdict.recorded.CLOCK_SKEW.val = 3600 := by
  simp [verdict.recorded.CLOCK_SKEW]

/-- The age of a recorded verdict: unusable when dated more than an hour
ahead; otherwise `now - checked_at`, clamped to `0 ..= i64::MAX`. -/
theorem recorded_age_spec (now ca : I64) :
    verdict.recorded.recorded_age now ca ⦃ r =>
      (r = none ↔ ca.val > now.val + 3600) ∧
      ∀ a, r = some a → a.val = max 0 (min (2^63 - 1) (now.val - ca.val)) ⦄ := by
  unfold verdict.recorded.recorded_age
  have hmax : core.num.I64.MAX.val = 2^63 - 1 := by simp [core.num.I64.MAX]; rfl
  have hk := skew_val
  step as ⟨ i, hi ⟩
  split
  · rename_i hn
    step as ⟨ i1, hi1 ⟩
    split
    · rename_i h; simp; scalar_tac
    · rename_i h
      split
      · rename_i h2; simp; constructor <;> scalar_tac
      · split
        · rename_i h3
          step as ⟨ i2, hi2 ⟩
          split
          · simp; constructor <;> scalar_tac
          · step as ⟨ i3, hi3 ⟩
            simp; constructor <;> scalar_tac
        · step as ⟨ i2, hi2 ⟩
          simp; constructor <;> scalar_tac
  · rename_i hn
    split
    · simp; constructor <;> scalar_tac
    · split
      · step as ⟨ i1, hi1 ⟩
        split
        · simp; constructor <;> scalar_tac
        · step as ⟨ i2, hi2 ⟩
          simp; constructor <;> scalar_tac
      · step as ⟨ i1, hi1 ⟩
        simp; constructor <;> scalar_tac

/-! ## Which statement a reader uses -/

/-- Worst first: gone, changed, moved, fresh, then unverifiable. -/
def worstOf : verdict.Said → Nat
  | .Gone => 0
  | .Changed => 1
  | .Moved => 2
  | .Fresh => 3
  | .Unverifiable => 4

/-- A statement that may be picked: never `unverifiable`. -/
def Usable (s : verdict.recorded.Statement) : Prop := s.said ≠ .Unverifiable

/-- `a` comes strictly before `b`: newer (smaller age), then from a
receipt, then the worse verdict. -/
def Before (a b : verdict.recorded.Statement) : Prop :=
  a.age.val < b.age.val ∨ (a.age.val = b.age.val ∧
    ((a.recorded = true ∧ b.recorded = false) ∨
     (a.recorded = b.recorded ∧ worstOf a.said < worstOf b.said)))

theorem worst_spec (s : verdict.Said) :
    verdict.recorded.worst s ⦃ r => r.val = worstOf s ⦄ := by
  unfold verdict.recorded.worst
  cases s <;> simp [worstOf]

theorem usable_spec (s : verdict.recorded.Statement) :
    verdict.recorded.usable s ⦃ b => b = true ↔ Usable s ⦄ := by
  unfold verdict.recorded.usable
  rcases s with ⟨said, age, rec⟩
  cases said <;> simp [Usable]

theorem before_spec (a b : verdict.recorded.Statement) :
    verdict.recorded.before a b ⦃ r => r = true ↔ Before a b ⦄ := by
  unfold verdict.recorded.before
  split
  · rename_i hne
    simp only [WP.spec_ok, decide_eq_true_eq, Before]
    have : a.age.val ≠ b.age.val := by
      intro e; have := IScalar.eq_of_val_eq e; simp [this] at hne
    constructor
    · intro h; left; scalar_tac
    · rintro (h | ⟨h, _⟩)
      · scalar_tac
      · exact absurd h this
  · rename_i heq
    have hage : a.age.val = b.age.val := by
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at heq; rw [heq]
    split
    · rename_i hr
      simp only [WP.spec_ok, Before, hage, lt_self_iff_false, true_and, false_or]
      cases ha : a.recorded <;> cases hb : b.recorded <;> simp_all
    · rename_i hr
      have hrr : a.recorded = b.recorded := by
        simpa using hr
      step with worst_spec as ⟨ x, hx ⟩
      step with worst_spec as ⟨ y, hy ⟩
      simp only [decide_eq_true_eq, Before, hage, lt_self_iff_false, true_and,
        false_or, hrr]
      cases b.recorded <;> simp <;> scalar_tac

theorem before_asymm {a b : verdict.recorded.Statement} (h : Before a b) : ¬ Before b a := by
  unfold Before at *
  rcases ha : a.recorded <;> rcases hb : b.recorded <;> simp_all <;> omega

/-- Strictly before, then not after: strictly before. -/
theorem before_of_le {a b c : verdict.recorded.Statement}
    (hab : Before a b) (hcb : ¬ Before c b) : Before a c := by
  unfold Before at *
  rcases ha : a.recorded <;> rcases hb : b.recorded <;> rcases hc : c.recorded <;>
    simp_all <;> omega

/-- The best of the first `n` recorded statements: none usable, or a
usable one that every other usable one does not beat, the first on a tie. -/
def BestUpTo (rs : List verdict.recorded.Statement) (n : Nat) : Option Usize → Prop
  | none => ∀ j (hj : j < rs.length), j < n → ¬ Usable rs[j]
  | some b => ∃ hb : b.val < rs.length, b.val < n ∧ Usable rs[b.val] ∧
      ∀ j (hj : j < rs.length), j < n → j ≠ b.val → Usable rs[j] →
        Before rs[b.val] rs[j] ∨ (¬ Before rs[j] rs[b.val] ∧ b.val < j)

theorem better_spec (rs : Slice verdict.recorded.Statement) (best : Option Usize) (i : Usize)
    (hi : i.val < rs.val.length) (hb : BestUpTo rs.val i.val best) :
    verdict.recorded.better rs best i ⦃ r => BestUpTo rs.val (i.val + 1) r ⦄ := by
  unfold verdict.recorded.better
  simp only [slice_get_ok, List.getElem?_eq_getElem hi, bind_ok]
  step with usable_spec as ⟨ u, hu ⟩
  split
  · rename_i hut
    have hus : Usable rs.val[i.val] := hu.mp hut
    rcases best with _ | b
    · simp only [WP.spec_ok, BestUpTo]
      refine ⟨hi, by omega, hus, ?_⟩
      intro j hj hjn hji hju
      exact absurd hju (hb j hj (by omega))
    · obtain ⟨hbl, hbn, hbu, hball⟩ := hb
      simp only [List.getElem?_eq_getElem hbl]
      step with before_spec as ⟨ c, hc ⟩
      split
      · rename_i hct
        have hbef : Before rs.val[i.val] rs.val[b.val] := hc.mp hct
        simp only [WP.spec_ok, BestUpTo]
        refine ⟨hi, by omega, hus, ?_⟩
        intro j hj hjn hji hju
        left
        by_cases hjb : j = b.val
        · subst hjb; exact hbef
        · have hjlt : j < i.val := by omega
          rcases hball j hj hjlt hjb hju with h | ⟨h, _⟩
          · exact before_of_le hbef (before_asymm h)
          · exact before_of_le hbef h
      · rename_i hct
        have hnot : ¬ Before rs.val[i.val] rs.val[b.val] := fun h => hct (hc.mpr h)
        simp only [WP.spec_ok, BestUpTo]
        refine ⟨hbl, by omega, hbu, ?_⟩
        intro j hj hjn hjb hju
        by_cases hji : j = i.val
        · subst hji; right; exact ⟨hnot, hbn⟩
        · exact hball j hj (by omega) hjb hju
  · rename_i hut
    have hnu : ¬ Usable rs.val[i.val] := fun h => hut (hu.mpr h)
    simp only [WP.spec_ok]
    rcases best with _ | b
    · simp only [BestUpTo] at hb ⊢
      intro j hj hjn
      by_cases hji : j = i.val
      · subst hji; exact hnu
      · exact hb j hj (by omega)
    · obtain ⟨hbl, hbn, hbu, hball⟩ := hb
      refine ⟨hbl, by omega, hbu, ?_⟩
      intro j hj hjn hjb hju
      by_cases hji : j = i.val
      · subst hji; exact absurd hju hnu
      · exact hball j hj (by omega) hjb hju

/-- `better` never fails, at any index and from any best so far (tauten's
total shape, docs/writfold/tauten-adoption.md). -/
theorem better_total (rs : Slice verdict.recorded.Statement) (best : Option Usize) (i : Usize) :
    verdict.recorded.better rs best i ⦃ _ => True ⦄ := by
  unfold verdict.recorded.better
  simp only [slice_get_ok, bind_ok]
  split
  · simp
  · step with usable_spec as ⟨ u, hu ⟩
    split
    · split
      · simp
      · split
        · simp
        · step with before_spec as ⟨ c, hc ⟩
          split <;> simp
    · simp

/-- Past the end there is no statement: an index there keeps the best so
far. -/
theorem better_past_end (rs : Slice verdict.recorded.Statement) (best : Option Usize) (i : Usize)
    (hi : rs.val.length ≤ i.val) :
    verdict.recorded.better rs best i ⦃ r => r = best ⦄ := by
  unfold verdict.recorded.better
  simp [slice_get_ok, List.getElem?_eq_none hi]

theorem best_recorded_loop_spec (rs : Slice verdict.recorded.Statement) (best : Option Usize)
    (i : Usize) (hi : i.val ≤ rs.val.length) (hb : BestUpTo rs.val i.val best) :
    verdict.recorded.best_recorded_loop rs best i ⦃ r => BestUpTo rs.val rs.val.length r ⦄ := by
  unfold verdict.recorded.best_recorded_loop
  apply loop.spec_decr_nat (fun (x : Option Usize × Usize) => rs.val.length - x.2.val)
    (fun (x : Option Usize × Usize) => x.2.val ≤ rs.val.length ∧ BestUpTo rs.val x.2.val x.1)
  · rintro ⟨best, i⟩ ⟨hi, hb⟩
    dsimp only at hi hb
    unfold verdict.recorded.best_recorded_loop.body
    dsimp only
    split
    · rename_i hlt
      step with better_spec as ⟨ b1, hb1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨by scalar_tac, by rw [hi1]; exact hb1, by scalar_tac⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = rs.val.length := by scalar_tac
      rw [← this]; exact hb
  · exact ⟨hi, hb⟩

theorem best_recorded_spec (rs : Slice verdict.recorded.Statement) :
    verdict.recorded.best_recorded rs ⦃ r => BestUpTo rs.val rs.val.length r ⦄ := by
  unfold verdict.recorded.best_recorded
  apply best_recorded_loop_spec
  · simp
  · simp [BestUpTo]

/-- A statement the reader may pick. -/
def Cand (loc : Option verdict.recorded.Statement) (rs : List verdict.recorded.Statement)
    (max : Option I64) : verdict.recorded.Pick → verdict.recorded.Statement → Prop
  | .Local, s => loc = some s ∧ Usable s
  | .Recorded i, s => max ≠ none ∧ ∃ h : i.val < rs.length, rs[i.val] = s ∧ Usable s

/-- On a full tie, a receipt comes before the local statement, and an
earlier receipt before a later one. -/
def PickLt : verdict.recorded.Pick → verdict.recorded.Pick → Prop
  | .Recorded i, .Recorded j => i.val < j.val
  | .Recorded _, .Local => True
  | .Local, _ => False

/-- `p` (saying `s`) is ahead of `q` (saying `t`). -/
def Ahead (p : verdict.recorded.Pick) (s : verdict.recorded.Statement)
    (q : verdict.recorded.Pick) (t : verdict.recorded.Statement) : Prop :=
  Before s t ∨ (¬ Before t s ∧ PickLt p q)

/-- The pick is a usable statement ahead of every other; `none` exactly
when no statement is usable. -/
theorem newest_spec (loc : Option verdict.recorded.Statement)
    (rs : Slice verdict.recorded.Statement) (max : Option I64) :
    verdict.recorded.newest loc rs max ⦃ r => match r with
      | none => ∀ p s, ¬ Cand loc rs.val max p s
      | some p => ∃ s, Cand loc rs.val max p s ∧
          ∀ q t, Cand loc rs.val max q t → q ≠ p → Ahead p s q t ⦄ := by
  unfold verdict.recorded.newest
  -- the shared statements: none without a max_age
  have hshared : (match max with
      | none => ok none
      | some _ => verdict.recorded.best_recorded rs) ⦃ sh =>
        (max = none → sh = none) ∧ (max ≠ none → BestUpTo rs.val rs.val.length sh) ⦄ := by
    rcases max with _ | m
    · simp
    · apply WP.spec_mono (best_recorded_spec rs)
      intro sh h; simp [h]
  apply WP.spec_bind hshared
  intro sh ⟨hsn, hsb⟩
  -- the local statement, when usable
  have hlocal : (match loc with
      | none => ok none
      | some s => do
        let b ← verdict.recorded.usable s
        if b = true then ok loc else ok none) ⦃ l =>
        (∀ s, l = some s ↔ loc = some s ∧ Usable s) ⦄ := by
    rcases loc with _ | s
    · simp
    · simp only
      step with usable_spec as ⟨ u, hu ⟩
      split
      · rename_i hut; simp only [WP.spec_ok]; intro t
        constructor
        · rintro ⟨⟩; exact ⟨rfl, hu.mp hut⟩
        · rintro ⟨h, _⟩; exact h
      · rename_i hut; simp only [WP.spec_ok]; intro t
        simp only [reduceCtorEq, false_iff, not_and, Option.some.injEq]
        rintro rfl h; exact hut (hu.mpr h)
  apply WP.spec_bind hlocal
  intro l hl
  -- what the shared pick says about the recorded candidates
  have hrec : ∀ j t, Cand loc rs.val max (.Recorded j) t → max ≠ none ∧
      ∃ hj : j.val < rs.val.length, rs.val[j.val] = t ∧ Usable t := fun j t h => h
  rcases l with _ | s
  · -- no usable local statement
    have hnoloc : ∀ t, ¬ Cand loc rs.val max .Local t := by
      intro t ⟨h1, h2⟩; exact absurd ((hl t).mpr ⟨h1, h2⟩) (by simp)
    rcases sh with _ | i
    · simp only [WP.spec_ok]
      intro p t hc
      rcases p with _ | j
      · exact hnoloc t hc
      · obtain ⟨hm, hj, rfl, hu⟩ := hc
        exact hsb hm j.val hj (by omega) hu
    · have hm : max ≠ none := fun h => absurd (hsn h) (by simp)
      obtain ⟨hil, -, hiu, hall⟩ := hsb hm
      simp only [WP.spec_ok]
      refine ⟨rs.val[i.val], ⟨hm, hil, rfl, hiu⟩, ?_⟩
      intro q t hq hne
      rcases q with _ | j
      · exact absurd hq (hnoloc t)
      · obtain ⟨_, hj, rfl, hju⟩ := hq
        have hji : j.val ≠ i.val := fun e => hne (by
          simp only [verdict.recorded.Pick.Recorded.injEq]; exact UScalar.eq_of_val_eq e)
        rcases hall j.val hj (by omega) hji hju with h | ⟨h, hlt⟩
        · exact Or.inl h
        · exact Or.inr ⟨h, hlt⟩
  · have hs : loc = some s ∧ Usable s := (hl s).mp rfl
    rcases sh with _ | i
    · simp only [WP.spec_ok]
      refine ⟨s, hs, ?_⟩
      intro q t hq hne
      rcases q with _ | j
      · exact absurd rfl hne
      · obtain ⟨hm, hj, rfl, hju⟩ := hq
        exact absurd hju (hsb hm j.val hj (by omega))
    · have hm : max ≠ none := fun h => absurd (hsn h) (by simp)
      obtain ⟨hil, -, hiu, hall⟩ := hsb hm
      simp only
      step as ⟨ s1, hs1 ⟩
      step with before_spec as ⟨ c, hc ⟩
      split
      · rename_i hct
        have hbef : Before s rs.val[i.val] := by rw [← hs1]; exact hc.mp hct
        simp only [WP.spec_ok]
        refine ⟨s, hs, ?_⟩
        intro q t hq hne
        rcases q with _ | j
        · exact absurd rfl hne
        · obtain ⟨_, hj, rfl, hju⟩ := hq
          left
          by_cases hji : j.val = i.val
          · simp only [hji]; exact hbef
          · rcases hall j.val hj (by omega) hji hju with h | ⟨h, _⟩
            · exact before_of_le hbef (before_asymm h)
            · exact before_of_le hbef h
      · rename_i hct
        have hnot : ¬ Before s rs.val[i.val] := by
          rw [← hs1]; exact fun h => hct (hc.mpr h)
        simp only [WP.spec_ok]
        refine ⟨rs.val[i.val], ⟨hm, hil, rfl, hiu⟩, ?_⟩
        intro q t hq hne
        rcases q with _ | j
        · obtain ⟨hq1, _⟩ := hq
          rw [hs.1] at hq1; cases hq1
          exact Or.inr ⟨hnot, trivial⟩
        · obtain ⟨_, hj, rfl, hju⟩ := hq
          have hji : j.val ≠ i.val := fun e => hne (by
            simp only [verdict.recorded.Pick.Recorded.injEq]; exact UScalar.eq_of_val_eq e)
          rcases hall j.val hj (by omega) hji hju with h | ⟨h, hlt⟩
          · exact Or.inl h
          · exact Or.inr ⟨h, hlt⟩

/-- An `unverifiable` statement is never picked, and a receipt is never
picked without a `max_age`. -/
theorem newest_never_unverifiable (loc : Option verdict.recorded.Statement)
    (rs : Slice verdict.recorded.Statement) (max : Option I64) :
    verdict.recorded.newest loc rs max ⦃ r => ∀ p, r = some p →
      ∃ s, Cand loc rs.val max p s ∧ s.said ≠ .Unverifiable ⦄ := by
  apply WP.spec_mono (newest_spec loc rs max)
  intro r hr p hp
  subst hp
  obtain ⟨s, hc, _⟩ := hr
  refine ⟨s, hc, ?_⟩
  rcases p with _ | j
  · exact hc.2
  · exact hc.2.2.2

end Writfold.Core.Recorded
