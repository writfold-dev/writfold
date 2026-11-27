import Extracted.WritfoldCore.Target
import Extracted.WritfoldCore.Types

/-!
# The commit a citation is judged against (#179, RFC 2026-11-27-citation-target)

Theorems over the Aeneas translation of `writfold_core::target`, the Git
provider's resolver.

- `choose_spec`: `choose` is the table `chooseOf`, for every input: a
  resolving pin decides and a missing one is ambiguous; no remotes gives
  `HEAD`; only remotes at the strongest naming level present are
  candidates; a candidate without a default branch, or candidates at
  different commits, are ambiguous.
- `other_never_chosen`: a chosen remote names the cited repository (its
  level is above 0) and has a tip. This is the fork: `origin` naming
  another repository is never the target.
- `pin_decides`: with a pin, the remotes do not matter.
- `disagreement_is_ambiguous`: two candidates at the deciding level with
  different tips never give a remote.
- `one_remote_unchanged`: one remote that names the repository and has a
  tip gives that remote, and no remote gives `HEAD`: today's answer for a
  clone with only `origin`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Target

/-- A naming level: 4 for `Url` down to 1 for `Sole`; 0 is never a candidate. -/
def rankN : target.Names → Nat
  | .Url => 4
  | .Path => 3
  | .Base => 2
  | .Sole => 1
  | .Other => 0
  | .Unread => 0

/-- The commit a tip is at, if any. -/
def tipN : target.Tip → Option Nat
  | .At k => some k.val
  | .NoHead => none

/-- The strongest naming level among the remotes. -/
def bestL : List target.Remote → Nat
  | [] => 0
  | r :: rs => max (rankN r.names) (bestL rs)

/-- Some remote at `lv` has no default branch. -/
def headlessL (lv : Nat) : List target.Remote → Bool
  | [] => false
  | r :: rs => (rankN r.names == lv && (tipN r.tip).isNone) || headlessL lv rs

/-- The first remote at `lv`, numbered from `n`, and its tip; `none` when
there is none or it has no default branch. -/
def firstL (lv : Nat) : Nat → List target.Remote → Option (Nat × Nat)
  | _, [] => none
  | n, r :: rs =>
    if rankN r.names = lv then (tipN r.tip).map (fun k => (n, k)) else firstL lv (n + 1) rs

/-- Every remote at `lv` is at commit `k`. -/
def agreeL (lv k : Nat) : List target.Remote → Bool
  | [] => true
  | r :: rs => (rankN r.names != lv || tipN r.tip == some k) && agreeL lv k rs

/-- `target::Choice` with a natural-number index. -/
inductive ChoiceN where
  | pinned
  | remote (i : Nat)
  | head
  | ambiguous (w : target.Why)

def toN : target.Choice → ChoiceN
  | .Pinned => .pinned
  | .Remote i => .remote i.val
  | .Head => .head
  | .Ambiguous w => .ambiguous w

/-- The table. -/
def chooseOf (p : target.Pinned) (l : List target.Remote) : ChoiceN :=
  match p with
  | .Resolves => .pinned
  | .Missing => .ambiguous .PinMissing
  | .No =>
    if l.isEmpty then .head
    else if bestL l = 0 then .ambiguous .NoRemoteNames
    else if headlessL (bestL l) l then .ambiguous .NoDefaultBranch
    else match firstL (bestL l) 0 l with
      | none => .ambiguous .NoDefaultBranch
      | some (i, k) => if agreeL (bestL l) k l then .remote i else .ambiguous .Disagree

theorem rank_spec (n : target.Names) : target.rank n ⦃ r => r.val = rankN n ⦄ := by
  unfold target.rank; cases n <;> simp [rankN]

theorem tip_commit_spec (t : target.Tip) :
    target.tip_commit t ⦃ o => o.map (·.val) = tipN t ⦄ := by
  unfold target.tip_commit; cases t <;> simp [tipN]

theorem tip_is_spec (t : target.Tip) (k : U32) :
    target.tip_is t k ⦃ b => b = (tipN t == some k.val) ⦄ := by
  unfold target.tip_is
  cases t with
  | At j =>
    simp only [WP.spec_ok, tipN, beq_iff_eq, Option.some.injEq]
    by_cases h : j = k
    · subst h; simp
    · have : j.val ≠ k.val := fun e => h (UScalar.eq_of_val_eq e)
      simp [h, this]
  | NoHead => simp [tipN]

theorem bestL_bound (l : List target.Remote) : bestL l ≤ 4 := by
  induction l with
  | nil => simp [bestL]
  | cons r rs ih => simp only [bestL]; cases r.names <;> simp [rankN] <;> omega

theorem drop_cons {α} (l : List α) (i : Nat) (h : i < l.length) :
    l.drop i = l[i] :: l.drop (i + 1) := List.drop_eq_getElem_cons h

theorem best_loop_spec (rs : Slice target.Remote) (top : U8) (i : Usize)
    (hi : i.val ≤ rs.val.length) :
    target.best_loop rs top i ⦃ r => r.val = max top.val (bestL (rs.val.drop i.val)) ⦄ := by
  unfold target.best_loop
  apply loop.spec_decr_nat (fun p : U8 × Usize => rs.val.length - p.2.val)
    (fun p : U8 × Usize => p.2.val ≤ rs.val.length ∧
      max p.1.val (bestL (rs.val.drop p.2.val)) = max top.val (bestL (rs.val.drop i.val)))
  · rintro ⟨t, j⟩ ⟨hj, hinv⟩
    unfold target.best_loop.body
    dsimp only at hj hinv ⊢
    split
    · rename_i hlt
      have hjl : j.val < rs.val.length := by scalar_tac
      step as ⟨ r, hr ⟩
      step with rank_spec as ⟨ lv, hlv ⟩
      rw [drop_cons _ _ hjl] at hinv
      simp only [bestL] at hinv
      split
      · rename_i hgt
        step as ⟨ j', hj' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show j'.val = j.val + 1 by scalar_tac, ← hinv, ← hr, ← hlv]
        have : t.val < lv.val := by scalar_tac
        omega
      · rename_i hgt
        step as ⟨ j', hj' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show j'.val = j.val + 1 by scalar_tac, ← hinv, ← hr, ← hlv]
        have : lv.val ≤ t.val := by scalar_tac
        omega
    · rename_i hge
      simp only [WP.spec_ok]
      rw [← hinv, List.drop_of_length_le (by scalar_tac)]
      simp [bestL]
  · exact ⟨hi, rfl⟩

theorem best_spec (rs : Slice target.Remote) :
    target.best rs ⦃ r => r.val = bestL rs.val ⦄ := by
  unfold target.best
  apply WP.spec_mono (best_loop_spec rs 0#u8 0#usize (by simp))
  intro r hr; simp_all

theorem headless_at_loop_spec (rs : Slice target.Remote) (lv : U8) (i : Usize)
    (hi : i.val ≤ rs.val.length) :
    target.headless_at_loop rs lv i ⦃ b => b = headlessL lv.val (rs.val.drop i.val) ⦄ := by
  unfold target.headless_at_loop
  apply loop.spec_decr_nat (fun j : Usize => rs.val.length - j.val)
    (fun j : Usize => j.val ≤ rs.val.length ∧
      headlessL lv.val (rs.val.drop j.val) = headlessL lv.val (rs.val.drop i.val))
  · rintro j ⟨hj, hinv⟩
    unfold target.headless_at_loop.body
    dsimp only at hj hinv ⊢
    split
    · rename_i hlt
      have hjl : j.val < rs.val.length := by scalar_tac
      step as ⟨ r, hr ⟩
      step with rank_spec as ⟨ x, hx ⟩
      split
      · rename_i heq
        step with tip_commit_spec as ⟨ o, ho ⟩
        have hxv : rankN rs.val[j.val].names = lv.val := by rw [← hr, ← hx, heq]
        have hl : headlessL lv.val (rs.val.drop j.val) =
            ((tipN rs.val[j.val].tip).isNone || headlessL lv.val (rs.val.drop (j.val + 1))) := by
          rw [drop_cons _ _ hjl]; simp [headlessL, hxv]
        rw [hl] at hinv
        rw [← hr] at hinv
        rcases o with _ | k
        · simp only [Option.map_none] at ho
          dsimp only [core.option.Option.is_none, Option.isNone]
          simp only [ite_true, WP.spec_ok]
          rw [← hinv, ← ho]; simp
        · simp only [Option.map_some] at ho
          dsimp only [core.option.Option.is_none, Option.isNone]
          simp only [Bool.false_eq_true, ite_false]
          step as ⟨ j', hj' ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [show j'.val = j.val + 1 by scalar_tac, ← hinv, ← ho]; simp
      · rename_i hne
        step as ⟨ j', hj' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show j'.val = j.val + 1 by scalar_tac, ← hinv]
        have hxv : rankN rs.val[j.val].names ≠ lv.val := by
          rw [← hr, ← hx]; intro h; exact hne (UScalar.eq_of_val_eq h)
        rw [drop_cons _ _ hjl]; simp [headlessL, hxv]
    · rename_i hge
      simp only [WP.spec_ok]
      rw [← hinv, List.drop_of_length_le (by scalar_tac)]
      simp [headlessL]
  · exact ⟨hi, rfl⟩

theorem headless_at_spec (rs : Slice target.Remote) (lv : U8) :
    target.headless_at rs lv ⦃ b => b = headlessL lv.val rs.val ⦄ := by
  unfold target.headless_at
  apply WP.spec_mono (headless_at_loop_spec rs lv 0#usize (by simp))
  intro b hb; simp_all

theorem first_at_loop_spec (rs : Slice target.Remote) (lv : U8) (i : Usize)
    (hi : i.val ≤ rs.val.length) :
    target.first_at_loop rs lv i ⦃ o =>
      o.map (fun p => (p.1.val, p.2.val)) = firstL lv.val i.val (rs.val.drop i.val) ⦄ := by
  unfold target.first_at_loop
  apply loop.spec_decr_nat (fun j : Usize => rs.val.length - j.val)
    (fun j : Usize => j.val ≤ rs.val.length ∧
      firstL lv.val j.val (rs.val.drop j.val) = firstL lv.val i.val (rs.val.drop i.val))
  · rintro j ⟨hj, hinv⟩
    unfold target.first_at_loop.body
    dsimp only at hj hinv ⊢
    split
    · rename_i hlt
      have hjl : j.val < rs.val.length := by scalar_tac
      step as ⟨ r, hr ⟩
      step with rank_spec as ⟨ x, hx ⟩
      rw [drop_cons _ _ hjl] at hinv
      split
      · rename_i heq
        step with tip_commit_spec as ⟨ o, ho ⟩
        have hxv : rankN rs.val[j.val].names = lv.val := by rw [← hr, ← hx, heq]
        rw [← hinv]; simp only [firstL, hxv, ite_true]
        rw [← hr, ← ho]
        cases o <;> simp
      · rename_i hne
        step as ⟨ j', hj' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show j'.val = j.val + 1 by scalar_tac, ← hinv]
        have hxv : rankN rs.val[j.val].names ≠ lv.val := by
          rw [← hr, ← hx]; intro h; exact hne (UScalar.eq_of_val_eq h)
        simp [firstL, hxv]
    · rename_i hge
      simp only [WP.spec_ok]
      rw [← hinv, List.drop_of_length_le (by scalar_tac)]
      simp [firstL]
  · exact ⟨hi, rfl⟩

theorem first_at_spec (rs : Slice target.Remote) (lv : U8) :
    target.first_at rs lv ⦃ o => o.map (fun p => (p.1.val, p.2.val)) = firstL lv.val 0 rs.val ⦄ := by
  unfold target.first_at
  apply WP.spec_mono (first_at_loop_spec rs lv 0#usize (by simp))
  intro o ho; simp_all

theorem agree_at_loop_spec (rs : Slice target.Remote) (lv : U8) (k : U32) (i : Usize)
    (hi : i.val ≤ rs.val.length) :
    target.agree_at_loop rs lv k i ⦃ b => b = agreeL lv.val k.val (rs.val.drop i.val) ⦄ := by
  unfold target.agree_at_loop
  apply loop.spec_decr_nat (fun j : Usize => rs.val.length - j.val)
    (fun j : Usize => j.val ≤ rs.val.length ∧
      agreeL lv.val k.val (rs.val.drop j.val) = agreeL lv.val k.val (rs.val.drop i.val))
  · rintro j ⟨hj, hinv⟩
    unfold target.agree_at_loop.body
    dsimp only at hj hinv ⊢
    split
    · rename_i hlt
      have hjl : j.val < rs.val.length := by scalar_tac
      step as ⟨ r, hr ⟩
      step with rank_spec as ⟨ x, hx ⟩
      rw [drop_cons _ _ hjl] at hinv
      split
      · rename_i heq
        step with tip_is_spec as ⟨ b, hb ⟩
        have hxv : rankN rs.val[j.val].names = lv.val := by rw [← hr, ← hx, heq]
        split
        · rename_i hbt
          step as ⟨ j', hj' ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [show j'.val = j.val + 1 by scalar_tac, ← hinv]
          simp only [agreeL, hxv, bne_self_eq_false, Bool.false_or]
          rw [← hr, ← hb, hbt]; simp
        · rename_i hbt
          simp only [WP.spec_ok]
          rw [← hinv]; simp only [agreeL, hxv, bne_self_eq_false, Bool.false_or]
          rw [← hr, ← hb]; cases b <;> simp_all
      · rename_i hne
        step as ⟨ j', hj' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show j'.val = j.val + 1 by scalar_tac, ← hinv]
        have hxv : rankN rs.val[j.val].names ≠ lv.val := by
          rw [← hr, ← hx]; intro h; exact hne (UScalar.eq_of_val_eq h)
        simp [agreeL, hxv]
    · rename_i hge
      simp only [WP.spec_ok]
      rw [← hinv, List.drop_of_length_le (by scalar_tac)]
      simp [agreeL]
  · exact ⟨hi, rfl⟩

theorem agree_at_spec (rs : Slice target.Remote) (lv : U8) (k : U32) :
    target.agree_at rs lv k ⦃ b => b = agreeL lv.val k.val rs.val ⦄ := by
  unfold target.agree_at
  apply WP.spec_mono (agree_at_loop_spec rs lv k 0#usize (by simp))
  intro b hb; simp_all

theorem choose_spec (p : target.Pinned) (rs : Slice target.Remote) :
    target.choose p rs ⦃ c => toN c = chooseOf p rs.val ⦄ := by
  unfold target.choose
  cases p with
  | Resolves => simp [toN, chooseOf]
  | Missing => simp [toN, chooseOf]
  | No =>
    simp only [chooseOf]
    step as ⟨ e, he ⟩
    split
    · rename_i het
      simp_all [toN]
    · rename_i hef
      have hne : rs.val.isEmpty = false := by simp_all
      simp only [hne]
      step with best_spec as ⟨ lv, hlv ⟩
      split
      · rename_i h0
        have : bestL rs.val = 0 := by rw [← hlv, h0]; rfl
        simp [toN, this]
      · rename_i h0
        have h0' : bestL rs.val ≠ 0 := by
          rw [← hlv]; intro h; exact h0 (UScalar.eq_of_val_eq (by simpa using h))
        simp only [h0', ite_false]
        step with headless_at_spec as ⟨ b, hb ⟩
        rw [hlv] at hb
        split
        · rename_i hbt; simp_all [toN]
        · rename_i hbt
          have hbf : headlessL (bestL rs.val) rs.val = false := by simp_all
          simp only [hbf]
          step with first_at_spec as ⟨ o, ho ⟩
          rw [hlv] at ho
          cases o with
          | none =>
            simp only [Option.map_none] at ho
            rw [← ho]; simp [toN]
          | some q =>
            obtain ⟨i, k⟩ := q
            simp only [Option.map_some] at ho
            rw [← ho]
            simp only
            step with agree_at_spec as ⟨ a, ha ⟩
            rw [hlv] at ha
            split
            · rename_i hat; simp_all [toN]
            · rename_i hat; simp_all [toN]

/-! ## Consequences of the table -/

theorem firstL_found (lv : Nat) : ∀ (l : List target.Remote) (n i k : Nat),
    firstL lv n l = some (i, k) →
    n ≤ i ∧ ∃ h : i - n < l.length, rankN l[i - n].names = lv ∧ tipN l[i - n].tip = some k := by
  intro l
  induction l with
  | nil => intro n i k h; simp [firstL] at h
  | cons r rs ih =>
    intro n i k h
    simp only [firstL] at h
    split at h
    · rename_i hr
      cases ht : tipN r.tip with
      | none => simp [ht] at h
      | some k' =>
        simp only [ht, Option.map_some, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        exact ⟨le_refl _, by simp, by simpa using hr, by simpa using ht⟩
    · obtain ⟨hle, hlt, hrk, htp⟩ := ih (n + 1) i k h
      refine ⟨by omega, ?_⟩
      have he : i - n = (i - (n + 1)) + 1 := by omega
      refine ⟨by simp; omega, ?_, ?_⟩
      · simp only [he, List.getElem_cons_succ]; exact hrk
      · simp only [he, List.getElem_cons_succ]; exact htp

theorem chooseOf_remote_facts (l : List target.Remote) (i : Nat)
    (h : chooseOf .No l = .remote i) :
    ∃ k, firstL (bestL l) 0 l = some (i, k) ∧ agreeL (bestL l) k l = true := by
  simp only [chooseOf] at h
  by_cases h1 : l.isEmpty = true
  · simp [h1] at h
  by_cases h2 : bestL l = 0
  · simp [h1, h2] at h
  by_cases h3 : headlessL (bestL l) l = true
  · simp [h1, h2, h3] at h
  simp only [h1, h2, h3, Bool.false_eq_true, ite_false] at h
  rcases hq : firstL (bestL l) 0 l with _ | ⟨i', k⟩ <;> simp only [hq] at h
  · cases h
  · refine ⟨k, ?_⟩
    by_cases hag : agreeL (bestL l) k l = true
    · simp only [hag, ite_true, ChoiceN.remote.injEq] at h
      subst h; exact ⟨rfl, hag⟩
    · simp [hag] at h

theorem chooseOf_remote (l : List target.Remote) (i : Nat)
    (h : chooseOf .No l = .remote i) :
    ∃ hi : i < l.length, 0 < rankN l[i].names ∧ (tipN l[i].tip).isSome := by
  obtain ⟨k, hq, _⟩ := chooseOf_remote_facts l i h
  have hb : bestL l ≠ 0 := by
    intro h0; simp only [chooseOf] at h
    by_cases h1 : l.isEmpty = true <;> simp_all
  obtain ⟨_, hlt, hrk, htp⟩ := firstL_found _ l 0 _ _ hq
  simp only [Nat.sub_zero] at hlt hrk htp
  exact ⟨hlt, by omega, by simp [htp]⟩

theorem other_never_chosen (rs : Slice target.Remote) :
    target.choose .No rs ⦃ c => ∀ i, c = .Remote i →
      ∃ hi : i.val < rs.val.length, 0 < rankN rs.val[i.val].names ∧
        (tipN rs.val[i.val].tip).isSome ⦄ := by
  apply WP.spec_mono (choose_spec .No rs)
  intro c hc i hi; subst hi
  exact chooseOf_remote rs.val i.val (by simpa [toN] using hc.symm)

theorem pin_decides (rs : Slice target.Remote) :
    target.choose .Resolves rs ⦃ c => c = .Pinned ⦄ ∧
    target.choose .Missing rs ⦃ c => c = .Ambiguous .PinMissing ⦄ := by
  constructor <;> (unfold target.choose; simp)

theorem agreeL_false (lv k : Nat) : ∀ (l : List target.Remote) (a : Nat) (ha : a < l.length),
    rankN l[a].names = lv → tipN l[a].tip ≠ some k → agreeL lv k l = false := by
  intro l
  induction l with
  | nil => intro a ha; simp at ha
  | cons r rs ih =>
    intro a ha hr ht
    cases a with
    | zero => simp_all [agreeL]
    | succ a =>
      simp only [agreeL, Bool.and_eq_false_iff]
      right; exact ih a (by simpa using ha) (by simpa using hr) (by simpa using ht)

theorem chooseOf_disagree (l : List target.Remote) (a b : Nat) (ha : a < l.length)
    (hb : b < l.length) (hra : rankN l[a].names = bestL l) (hrb : rankN l[b].names = bestL l)
    (ka kb : Nat) (hka : tipN l[a].tip = some ka) (hkb : tipN l[b].tip = some kb)
    (hd : ka ≠ kb) : ∀ i, chooseOf .No l ≠ .remote i := by
  intro i h
  obtain ⟨k, _, hag⟩ := chooseOf_remote_facts l i h
  by_cases hk : ka = k
  · subst hk
    have := agreeL_false (bestL l) ka l b hb hrb (by rw [hkb]; simp; exact fun e => hd e.symm)
    simp_all
  · have := agreeL_false (bestL l) k l a ha hra (by rw [hka]; simp; exact hk)
    simp_all

theorem disagreement_is_ambiguous (rs : Slice target.Remote) (a b : Nat)
    (ha : a < rs.val.length) (hb : b < rs.val.length)
    (hra : rankN rs.val[a].names = bestL rs.val) (hrb : rankN rs.val[b].names = bestL rs.val)
    (ka kb : Nat) (hka : tipN rs.val[a].tip = some ka) (hkb : tipN rs.val[b].tip = some kb)
    (hd : ka ≠ kb) :
    target.choose .No rs ⦃ c => ∀ i, c ≠ .Remote i ⦄ := by
  apply WP.spec_mono (choose_spec .No rs)
  intro c hc i hi; subst hi
  exact chooseOf_disagree rs.val a b ha hb hra hrb ka kb hka hkb hd i.val
    (by simpa [toN] using hc.symm)

theorem one_remote_unchanged (r : target.Remote) (k : Nat)
    (hr : 0 < rankN r.names) (ht : tipN r.tip = some k) :
    chooseOf .No [r] = .remote 0 ∧ chooseOf .No [] = .head := by
  constructor
  · have hb : bestL [r] = rankN r.names := by simp [bestL]
    simp only [chooseOf, List.isEmpty_cons, Bool.false_eq_true, ite_false, hb]
    have h0 : rankN r.names ≠ 0 := by omega
    simp [h0, headlessL, firstL, agreeL, ht]
  · simp [chooseOf]

end Writfold.Core.Target
