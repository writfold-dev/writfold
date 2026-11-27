import Writfold.Conform.Words
import Writfold.Target
import Calculus.Referent

/-!
# Link: `target::choose_in` implements `Calculus.resolve` (#208, #212)

The Git resolver chooses the line a citation is judged at as law R says:
the strongest naming level decides alone, its candidates must agree, and
nothing falls back to a weaker level or to the first listed. The map's pin
is a candidate above every remote, and a clone with no fetched remote
offers its own `HEAD` (undeclared) or nothing (declared).
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Target

namespace Writfold.Core.Conform

/-- What a candidate offers to judge against. -/
inductive Line where
  | pin
  | head
  | commit (k : Nat)
  deriving DecidableEq

/-- A remote as a candidate: its naming level and its tip. -/
def remoteCand (d : Bool) (r : target.Remote) : Option (Calculus.Candidate Line) :=
  if rankN r.names d = 0 then none else some ⟨rankN r.names d, (tipN r.tip).map .commit⟩

/-- The candidates `choose_in` weighs. -/
def candidates (p : target.Pinned) (d : Bool) (l : List target.Remote) : List (Calculus.Candidate Line) :=
  match p with
  | .Resolves => [⟨5, some .pin⟩]
  | .Missing => [⟨5, none⟩]
  | .No => if l.isEmpty then (if d then [] else [⟨0, some .head⟩]) else l.filterMap (remoteCand d)

/-- A choice as a resolution: the chosen remote stands for its tip. -/
def resolutionOf (l : List target.Remote) : ChoiceN → Calculus.Resolution Line
  | .pinned => .one .pin
  | .head => .one .head
  | .remote i => match (l[i]?).bind (fun r => tipN r.tip) with
    | some k => .one (.commit k)
    | none => .ambiguous
  | .ambiguous .NoRemoteNames => .none
  | .ambiguous _ => .ambiguous

theorem rank_le_best (d : Bool) : ∀ (l : List target.Remote) (r : target.Remote),
    r ∈ l → rankN r.names d ≤ bestL d l := by
  intro l
  induction l with
  | nil => intro r h; simp at h
  | cons x xs ih =>
    intro r h
    simp only [bestL]
    rcases List.mem_cons.mp h with rfl | h
    · omega
    · have := ih r h; omega

theorem best_mem (d : Bool) : ∀ (l : List target.Remote), bestL d l ≠ 0 →
    ∃ r ∈ l, rankN r.names d = bestL d l := by
  intro l
  induction l with
  | nil => intro h; simp [bestL] at h
  | cons x xs ih =>
    intro h
    simp only [bestL] at h ⊢
    by_cases hx : bestL d xs ≤ rankN x.names d
    · exact ⟨x, by simp, by omega⟩
    · obtain ⟨r, hr, he⟩ := ih (by omega)
      exact ⟨r, List.mem_cons_of_mem _ hr, by omega⟩

/-- A remote as a candidate at level `lv`, if it is at that level. -/
def atLevel (d : Bool) (lv : Nat) (r : target.Remote) : Option (Calculus.Candidate Line) :=
  if rankN r.names d = lv then some ⟨lv, (tipN r.tip).map .commit⟩ else none

/-- The candidates at level `lv`, in order. -/
def topOf (d : Bool) (lv : Nat) (l : List target.Remote) : List (Calculus.Candidate Line) :=
  l.filterMap (atLevel d lv)

theorem filter_level (d : Bool) (lv : Nat) (hlv : lv ≠ 0) : ∀ l : List target.Remote,
    (l.filterMap (remoteCand d)).filter (fun c => decide (c.level = lv)) = topOf d lv l := by
  intro l
  induction l with
  | nil => rfl
  | cons x xs ih =>
    unfold topOf at ih ⊢
    by_cases h0 : rankN x.names d = 0
    · have hr : remoteCand d x = none := by simp [remoteCand, h0]
      have ht : atLevel d lv x = none := by
        have : rankN x.names d ≠ lv := by omega
        simp [atLevel, this]
      rw [List.filterMap_cons_none hr, List.filterMap_cons_none ht]; exact ih
    · have hr : remoteCand d x = some ⟨rankN x.names d, (tipN x.tip).map .commit⟩ := by
        simp [remoteCand, h0]
      rw [List.filterMap_cons_some hr, List.filter_cons]
      by_cases hl : rankN x.names d = lv
      · have ht : atLevel d lv x = some ⟨lv, (tipN x.tip).map .commit⟩ := by simp [atLevel, hl]
        rw [List.filterMap_cons_some ht, ← ih]
        simp [hl]
      · have ht : atLevel d lv x = none := by
          simp [atLevel, hl]
        rw [List.filterMap_cons_none ht, ← ih]
        simp [hl]

theorem filter_top (d : Bool) (l : List target.Remote) (h : bestL d l ≠ 0) :
    (l.filterMap (remoteCand d)).filter
        (Calculus.Candidate.top (l.filterMap (remoteCand d))) = topOf d (bestL d l) l := by
  have key : ∀ c ∈ l.filterMap (remoteCand d),
      Calculus.Candidate.top (l.filterMap (remoteCand d)) c = decide (c.level = bestL d l) := by
    intro c hc
    obtain ⟨r, hr, hrc⟩ := List.mem_filterMap.mp hc
    unfold remoteCand at hrc
    split at hrc
    · cases hrc
    · cases hrc
      rename_i hr0
      rw [Bool.eq_iff_iff]
      simp only [Calculus.Candidate.top, List.all_eq_true, decide_eq_true_eq]
      constructor
      · intro hall
        obtain ⟨m, hm, hme⟩ := best_mem d l h
        have hmc : (⟨rankN m.names d, (tipN m.tip).map .commit⟩ : Calculus.Candidate Line) ∈
            l.filterMap (remoteCand d) :=
          List.mem_filterMap.mpr ⟨m, hm, by unfold remoteCand; simp; omega⟩
        have h1 := hall _ hmc
        have h2 := rank_le_best d l r hr
        simp only at h1; omega
      · intro he c' hc'
        obtain ⟨r', hr', hrc'⟩ := List.mem_filterMap.mp hc'
        unfold remoteCand at hrc'
        split at hrc'
        · cases hrc'
        · cases hrc'; have := rank_le_best d l r' hr'; simp only; omega
  rw [List.filter_congr key]
  exact filter_level d _ h l

/-- Some candidate at `lv` offers nothing exactly when a remote at `lv`
has no default branch. -/
theorem headless_top (d : Bool) (lv : Nat) : ∀ l : List target.Remote,
    headlessL d lv l = true ↔ ∃ c ∈ topOf d lv l, c.referent = none := by
  intro l
  induction l with
  | nil => simp [headlessL, topOf]
  | cons x xs ih =>
    simp only [headlessL, topOf, List.filterMap_cons, atLevel] at ih ⊢
    by_cases hl : rankN x.names d = lv
    · cases ht : tipN x.tip <;> simp [hl, ht, ih]
    · simp [hl, ih]

/-- The first candidate at `lv` is the first remote at `lv`. -/
theorem first_top (d : Bool) (lv : Nat) : ∀ (l : List target.Remote) (n : Nat),
    (topOf d lv l).head? = (firstL d lv n l).map (fun p => ⟨lv, some (.commit p.2)⟩) ∨
    (∃ c ∈ (topOf d lv l).head?, c.referent = none) := by
  intro l
  induction l with
  | nil => intro n; left; simp [topOf, firstL]
  | cons x xs ih =>
    intro n
    simp only [topOf, List.filterMap_cons, firstL, atLevel] at ih ⊢
    by_cases hl : rankN x.names d = lv
    · cases ht : tipN x.tip <;> simp [hl, ht]
    · simp only [hl, ite_false]; exact ih (n + 1)

/-- Every candidate at `lv` offers commit `k` exactly when every remote at
`lv` is at `k`. -/
theorem agree_top (d : Bool) (lv k : Nat) : ∀ l : List target.Remote,
    agreeL d lv k l = (topOf d lv l).all (fun c => c.referent == some (.commit k)) := by
  intro l
  induction l with
  | nil => rfl
  | cons x xs ih =>
    unfold topOf at ih ⊢
    simp only [agreeL]
    by_cases hl : rankN x.names d = lv
    · have ht : atLevel d lv x = some ⟨lv, (tipN x.tip).map .commit⟩ := by simp [atLevel, hl]
      rw [List.filterMap_cons_some ht, List.all_cons, ← ih]
      cases h : tipN x.tip with
      | none => simp [hl]
      | some v =>
        by_cases hv : v = k
        · simp [hl, hv]
        · have h1 : (v == k) = false := beq_eq_false_iff_ne.mpr hv
          have h2 : (Line.commit v == Line.commit k) = false :=
            beq_eq_false_iff_ne.mpr (fun e => hv (Line.commit.inj e))
          simp [hl, h1, h2]
    · have ht : atLevel d lv x = none := by
        simp [atLevel, hl]
      rw [List.filterMap_cons_none ht, ← ih]
      simp [hl]

theorem cands_nil_of_best_zero (d : Bool) (l : List target.Remote) (h : bestL d l = 0) :
    l.filterMap (remoteCand d) = [] := by
  rw [List.filterMap_eq_nil_iff]
  intro r hr
  have := rank_le_best d l r hr
  simp [remoteCand]; omega

theorem top_ne_nil (d : Bool) (l : List target.Remote) (h : bestL d l ≠ 0) :
    topOf d (bestL d l) l ≠ [] := by
  obtain ⟨r, hr, he⟩ := best_mem d l h
  unfold topOf
  intro hn
  rw [List.filterMap_eq_nil_iff] at hn
  have := hn r hr
  simp [atLevel, he] at this

/-- The link (#208, #212): on every input, what `choose_in` chooses is what
`Calculus.resolve` makes of its candidates, so law R holds of it. -/
theorem choose_in_resolves (p : target.Pinned) (d : Bool) (l : List target.Remote) :
    resolutionOf l (chooseOfIn p d l) = Calculus.resolve (candidates p d l) := by
  cases p with
  | Resolves => simp [chooseOfIn, resolutionOf, candidates, Calculus.resolve, Calculus.Candidate.top]
  | Missing => simp [chooseOfIn, resolutionOf, candidates, Calculus.resolve, Calculus.Candidate.top]
  | No =>
    simp only [chooseOfIn, candidates]
    by_cases he : l.isEmpty = true
    · simp only [he, ite_true]
      cases d <;> simp [resolutionOf, Calculus.resolve, Calculus.Candidate.top]
    · simp only [he, Bool.false_eq_true, ite_false]
      by_cases h0 : bestL d l = 0
      · simp [h0, resolutionOf, cands_nil_of_best_zero d l h0, Calculus.resolve]
      · simp only [h0, ite_false]
        unfold Calculus.resolve
        rw [filter_top d l h0]
        have hne := top_ne_nil d l h0
        by_cases hh : headlessL d (bestL d l) l = true
        · simp only [hh, ite_true, resolutionOf]
          obtain ⟨x, hx, hn⟩ := (headless_top d _ l).mp hh
          cases ht : topOf d (bestL d l) l with
          | nil => exact absurd ht hne
          | cons c rest =>
            rw [ht] at hx
            cases hc : c.referent with
            | none => simp [hc]
            | some r =>
              rcases List.mem_cons.mp hx with rfl | hx'
              · rw [hc] at hn; cases hn
              · have hall : rest.all (fun c' => c'.referent == some r) = false := by
                  rw [List.all_eq_false]
                  exact ⟨x, hx', by simp [hn]⟩
                simp [hc, hall]
        · simp only [hh, Bool.false_eq_true, ite_false]
          rcases first_top d (bestL d l) l 0 with hf | ⟨c, hc, hn⟩
          · cases hq : firstL d (bestL d l) 0 l with
            | none =>
              rw [hq] at hf
              cases ht : topOf d (bestL d l) l with
              | nil => exact absurd ht hne
              | cons c rest => rw [ht] at hf; simp at hf
            | some q =>
              obtain ⟨i, k⟩ := q
              rw [hq] at hf
              obtain ⟨_, hil, _, htip⟩ := firstL_found d _ l 0 i k hq
              simp only [Nat.sub_zero] at hil htip
              cases ht : topOf d (bestL d l) l with
              | nil => exact absurd ht hne
              | cons c rest =>
                rw [ht] at hf
                simp only [List.head?_cons, Option.map_some, Option.some.injEq] at hf
                subst hf
                have hag := agree_top d (bestL d l) k l
                rw [ht] at hag
                simp only [List.all_cons, beq_self_eq_true, Bool.true_and] at hag
                simp only [hag]
                split
                · simp [resolutionOf, List.getElem?_eq_getElem hil, htip]
                · simp [resolutionOf]
          · exfalso
            apply hh
            rw [headless_top]
            cases ht : topOf d (bestL d l) l with
            | nil => rw [ht] at hc; simp at hc
            | cons c' rest =>
              rw [ht] at hc
              simp only [List.head?_cons, Option.mem_def, Option.some.injEq] at hc
              exact ⟨c, by rw [← hc]; exact List.mem_cons_self, hn⟩

/-- `target::choose_in` implements `Calculus.resolve`. -/
theorem choose_in_implements (p : target.Pinned) (d : Bool) (rs : Slice target.Remote) :
    target.choose_in p d rs ⦃ c =>
      resolutionOf rs.val (toN c) = Calculus.resolve (candidates p d rs.val) ⦄ := by
  apply WP.spec_mono (choose_in_spec p d rs)
  intro c hc; rw [hc]; exact choose_in_resolves p d rs.val

/-- `target::choose` implements `Calculus.resolve` (undeclared). -/
theorem choose_implements (p : target.Pinned) (rs : Slice target.Remote) :
    target.choose p rs ⦃ c =>
      resolutionOf rs.val (toN c) = Calculus.resolve (candidates p false rs.val) ⦄ := by
  rw [undeclared_is_choose]; exact choose_in_implements p false rs

end Writfold.Core.Conform
