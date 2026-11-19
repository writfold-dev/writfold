import Writfold.Verdict
import Writfold.Get

/-!
# Idle runs

Theorems over the Aeneas translation of `writfold_core::idle`.

- `newest_spec`: the newest stamp is the first greatest one; `none`
  exactly when there is no stamp.
- `since_spec`: `now - last` over the integers, held inside `i64`.
- `idle_spec`: a run is idle exactly when it has a stamp and its newest
  stamp lies more than `age` seconds before `now`, over the integers (no
  clock value overflows it); the seconds shown are `since`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Idle

/-- The newest of the first `n` stamps: the first greatest one. -/
def NewestUpTo (l : List I64) (n : Nat) : Option (Usize × I64) → Prop
  | none => n = 0
  | some (i, v) => ∃ h : i.val < l.length, i.val < n ∧ l[i.val] = v ∧
      (∀ j (hj : j < l.length), j < n → l[j].val ≤ v.val) ∧
      (∀ j (hj : j < l.length), j < i.val → l[j].val < v.val)

theorem newer_spec (l : Slice I64) (best : Option (Usize × I64)) (i : Usize)
    (hi : i.val < l.val.length) :
    idle.newer l best i ⦃ b => b = true ↔ match best with
      | none => True
      | some (_, a0) => l.val[i.val].val > a0.val ⦄ := by
  unfold idle.newer
  simp only [slice_get_ok, List.getElem?_eq_getElem hi, bind_ok]
  rcases best with _ | ⟨k, a0⟩ <;> simp

/-- `newer` never fails, at any index (tauten's total shape,
docs/writfold/tauten-adoption.md): a position past the end has no stamp
and is never newer. -/
theorem newer_total (l : Slice I64) (best : Option (Usize × I64)) (i : Usize) :
    idle.newer l best i ⦃ _ => True ⦄ := by
  unfold idle.newer
  simp only [slice_get_ok, bind_ok]
  rcases best with _ | ⟨k, a0⟩ <;> split <;> simp

/-- Past the end, `newer` is `false`: no stamp there to be newer. -/
theorem newer_past_end (l : Slice I64) (best : Option (Usize × I64)) (i : Usize)
    (hi : l.val.length ≤ i.val) :
    idle.newer l best i ⦃ b => b = false ⦄ := by
  unfold idle.newer
  simp [slice_get_ok, List.getElem?_eq_none hi]

theorem newest_loop_spec (l : Slice I64) (best : Option (Usize × I64)) (i : Usize)
    (hi : i.val ≤ l.val.length) (hb : NewestUpTo l.val i.val best) :
    idle.newest_loop l best i ⦃ r => NewestUpTo l.val l.val.length r ⦄ := by
  unfold idle.newest_loop
  apply loop.spec_decr_nat (fun (x : Option (Usize × I64) × Usize) => l.val.length - x.2.val)
    (fun (x : Option (Usize × I64) × Usize) => x.2.val ≤ l.val.length ∧
      NewestUpTo l.val x.2.val x.1)
  · rintro ⟨best, i⟩ ⟨hi, hb⟩
    dsimp only at hi hb
    unfold idle.newest_loop.body
    dsimp only
    split
    · rename_i hlt
      have hil : i.val < l.val.length := by scalar_tac
      step with newer_spec as ⟨ b, hbn ⟩
      split
      · rename_i hbt
        step as ⟨ x, hx ⟩
        step as ⟨ i2, hi2 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        refine ⟨hil, by scalar_tac, hx.symm, ?_, ?_⟩
        · intro j hj hjn
          by_cases hji : j < i.val
          · rcases best with _ | ⟨k, a0⟩
            · simp [NewestUpTo] at hb; omega
            · obtain ⟨_, _, hk, hall, _⟩ := hb
              have := hall j hj hji
              have hgt := hbn.mp hbt
              simp only at hgt; rw [hx]; scalar_tac
          · have : j = i.val := by scalar_tac
            subst this; rw [hx]
        · intro j hj hji
          rcases best with _ | ⟨k, a0⟩
          · simp [NewestUpTo] at hb; omega
          · obtain ⟨_, _, hk, hall, _⟩ := hb
            have := hall j hj hji
            have hgt := hbn.mp hbt
            simp only at hgt; rw [hx]; scalar_tac
      · rename_i hbt
        step as ⟨ i2, hi2 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rcases best with _ | ⟨k, a0⟩
        · exact absurd (hbn.mpr trivial) hbt
        · obtain ⟨hkl, hkn, hk, hall, hfirst⟩ := hb
          have hle : l.val[i.val].val ≤ a0.val := by
            by_contra hc; exact hbt (hbn.mpr (by simp only; omega))
          refine ⟨hkl, by scalar_tac, hk, ?_, hfirst⟩
          intro j hj hjn
          by_cases hji : j < i.val
          · exact hall j hj hji
          · have : j = i.val := by scalar_tac
            subst this; exact hle
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = l.val.length := by scalar_tac
      rw [← this]; exact hb
  · exact ⟨hi, hb⟩

/-- The newest stamp is the first greatest one; `none` exactly when there
is no stamp. -/
theorem newest_spec (l : Slice I64) :
    idle.newest l ⦃ r => NewestUpTo l.val l.val.length r ⦄ := by
  unfold idle.newest
  exact newest_loop_spec l none 0#usize (by simp) (by simp [NewestUpTo])

/-- `now - last` over the integers, held inside `i64`. -/
theorem since_spec (now last : I64) :
    idle.since now last ⦃ s => s.val = max (-2^63) (min (2^63 - 1) (now.val - last.val)) ⦄ := by
  unfold idle.since
  have hmax : core.num.I64.MAX.val = 2^63 - 1 := by simp [core.num.I64.MAX]; rfl
  have hmin : core.num.I64.MIN.val = -2^63 := by simp [core.num.I64.MIN]; rfl
  split
  · step as ⟨ i, hi ⟩
    split
    · simp only [WP.spec_ok]; scalar_tac
    · split
      · scalar_tac
      · step as ⟨ d, hd ⟩; scalar_tac
  · split
    · step as ⟨ i, hi ⟩
      split
      · simp only [WP.spec_ok]; scalar_tac
      · step as ⟨ d, hd ⟩; scalar_tac
    · step as ⟨ d, hd ⟩; scalar_tac

/-- A run is idle exactly when it has a stamp whose newest lies more than
`age` seconds before `now`, over the integers; the seconds shown are
`since` from the newest stamp, the first of equal ones. -/
theorem idle_spec (now : I64) (l : Slice I64) (age : I64) :
    idle.idle now l age ⦃ r =>
      (r = none ↔ l.val = [] ∨ ∃ i v, NewestUpTo l.val l.val.length (some (i, v)) ∧
        now.val - v.val ≤ age.val) ∧
      ∀ i s, r = some (i, s) → ∃ v, NewestUpTo l.val l.val.length (some (i, v)) ∧
        now.val - v.val > age.val ∧
        s.val = max (-2^63) (min (2^63 - 1) (now.val - v.val)) ⦄ := by
  unfold idle.idle
  step with newest_spec as ⟨ o, ho ⟩
  rcases o with _ | ⟨a0, last⟩
  · simp only [WP.spec_ok, true_iff, reduceCtorEq, false_implies, implies_true, and_true]
    left; simp [NewestUpTo] at ho; exact ho
  · simp only
    have hne : l.val ≠ [] := by
      obtain ⟨h, _⟩ := ho; intro e; simp [e] at h
    step with Verdict.expired_spec as ⟨ b, hb ⟩
    split
    · rename_i hbt
      step with since_spec as ⟨ s, hs ⟩
      simp only [reduceCtorEq, false_iff, not_or, Option.some.injEq,
        Prod.mk.injEq]
      refine ⟨⟨hne, ?_⟩, ?_⟩
      · rintro ⟨i, v, hn, hle⟩
        obtain ⟨hi, _, hv, hall, hfirst⟩ := hn
        obtain ⟨hat, _, hlast, hall', hfirst'⟩ := ho
        have h1 := hall' i.val hi hi
        have h2 := hall a0.val hat hat
        rw [hv] at h1; rw [hlast] at h2
        have := hb.mp hbt; omega
      · rintro i s' ⟨rfl, rfl⟩
        exact ⟨last, ho, hb.mp hbt, hs⟩
    · rename_i hbt
      simp only [WP.spec_ok, true_iff, reduceCtorEq, false_implies, implies_true, and_true]
      right
      exact ⟨a0, last, ho, by by_contra hc; exact hbt (hb.mpr (by omega))⟩

end Writfold.Core.Idle
