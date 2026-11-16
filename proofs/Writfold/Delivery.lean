import Writfold.Current

/-!
# Deliveries and the delivery guard

Theorems over the Aeneas translation of `writfold_core::delivery`: which
candidate decides an open delivery step, one standing for a delivery from
its obligations' standings, whether a candidate is ready, the re-check of
a passed check whose cited evidence moved, and the delivery guard's
decision.

- `deciding_spec`: the deciding candidate is the newest one no done record
  names; `none` exactly when every candidate is bound.
- `standing_spec`: an empty set stands `open`; otherwise the first unmet
  obligation's standing; otherwise `waived` if any is, else `attested` if
  any is, else `satisfied`. `standing_met_iff`: a delivery is met exactly
  when it has obligations and each is met; `standing_satisfied_iff`: it is
  `satisfied` exactly when it has obligations and each is a passing check
  (a statement is never read as one); `standing_first_unmet`.
- `ready_iff`: a candidate is ready exactly when its mode declares at least
  one obligation and every one is met, so a mode that declares nothing is
  never ready (fixed on 16 November 2026: `deliver status` and the guard
  read such a candidate ready).
- `rechecked_spec`: a passed check whose cited evidence moved is stale.
- `guard_allow_iff`: known bytes pass only when a ready candidate holds
  them; unknown bytes only when there is a candidate and every candidate is
  ready.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Delivery

/-! ## `deciding` -/

/-- The deciding candidate: the greatest index that is not bound; `none`
when every candidate is bound. -/
def DecidingSpec (l : List Bool) : Option Usize → Prop
  | some i => ∃ h : i.val < l.length, l[i.val] = false ∧
      ∀ j (hj : j < l.length), i.val < j → l[j] = true
  | none => ∀ b ∈ l, b = true

theorem deciding_loop_spec (bs : Slice Bool) (i : Usize) (hi : i.val ≤ bs.val.length)
    (hlater : ∀ j (hj : j < bs.val.length), i.val ≤ j → bs.val[j] = true) :
    delivery.deciding_loop bs i ⦃ r => DecidingSpec bs.val r ⦄ := by
  unfold delivery.deciding_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ bs.val.length ∧
      ∀ j (hj : j < bs.val.length), i.val ≤ j → bs.val[j] = true)
  · intro i ⟨hi, hlater⟩
    unfold delivery.deciding_loop.body
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hij
        by_cases hji : i1.val = j
        · subst hji; rw [← hb, hbt]
        · exact hlater j hj (by scalar_tac)
      · rename_i hbt
        simp only [WP.spec_ok, DecidingSpec]
        refine ⟨by scalar_tac, by rw [← hb]; simpa using hbt, ?_⟩
        intro j hj hij
        exact hlater j hj (by scalar_tac)
    · rename_i hzero
      simp only [WP.spec_ok, DecidingSpec]
      intro b hb
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hb
      exact hlater j hj (by scalar_tac)
  · exact ⟨hi, hlater⟩

/-- The candidate that decides an open step is the newest one that no done
record names; `none` exactly when every candidate is bound. -/
theorem deciding_spec (bs : Slice Bool) :
    delivery.deciding bs ⦃ r => DecidingSpec bs.val r ⦄ := by
  unfold delivery.deciding
  apply deciding_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-! ## `unmet`, `is_waived`, `is_attested` -/

/-- A waiver. -/
def isW : gate.Standing → Bool
  | .Waived => true
  | _ => false

/-- An attestation. -/
def isA : gate.Standing → Bool
  | .Attested => true
  | _ => false

theorem unmet_spec (s : gate.Standing) : delivery.unmet s ⦃ b => b = !Gate.metB s ⦄ := by
  unfold delivery.unmet
  step with Gate.met_spec as ⟨ m, hm ⟩
  have := Gate.metB_iff s
  cases hmb : Gate.metB s <;> cases m <;> simp_all [Gate.Met]

theorem is_waived_spec (s : gate.Standing) : delivery.is_waived s ⦃ b => b = isW s ⦄ := by
  unfold delivery.is_waived
  rcases s with s | _ | _
  · cases s <;> simp [isW]
  · simp [isW]
  · simp [isW]

theorem is_attested_spec (s : gate.Standing) : delivery.is_attested s ⦃ b => b = isA s ⦄ := by
  unfold delivery.is_attested
  rcases s with s | _ | _
  · cases s <;> simp [isA]
  · simp [isA]
  · simp [isA]

/-! ## `standing` -/

/-- With every obligation met: waived if any is, else attested if any is,
else satisfied. -/
def allMet (l : List gate.Standing) : gate.Standing :=
  if l.any isW then .Waived else if l.any isA then .Attested else .Checked .Satisfied

/-- `standing`'s decision. -/
def StandingSpec (l : List gate.Standing) (s : gate.Standing) : Prop :=
  (l = [] ∧ s = .Checked .Open) ∨
  (∃ i, ∃ h : i < l.length, Gate.metB l[i] = false ∧
      (∀ j (hj : j < i), Gate.metB (l[j]'(Nat.lt_trans hj h)) = true) ∧ s = l[i]) ∨
  (l ≠ [] ∧ (∀ o ∈ l, Gate.metB o = true) ∧ s = allMet l)

theorem standing_loop_spec (os : Slice gate.Standing) (w a : Bool) (i : Usize)
    (hi : i.val ≤ os.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < os.val.length), Gate.metB os.val[j] = true)
    (hw : w = true ↔ ∃ j, ∃ hj : j < os.val.length, j < i.val ∧ isW os.val[j] = true)
    (ha : a = true ↔ ∃ j, ∃ hj : j < os.val.length, j < i.val ∧ isA os.val[j] = true) :
    delivery.standing_loop os w a i ⦃ s =>
      (∃ i, ∃ h : i < os.val.length, Gate.metB os.val[i] = false ∧
        (∀ j (hj : j < i), Gate.metB (os.val[j]'(Nat.lt_trans hj h)) = true) ∧ s = os.val[i]) ∨
      ((∀ o ∈ os.val, Gate.metB o = true) ∧ s = allMet os.val) ⦄ := by
  unfold delivery.standing_loop
  apply loop.spec_decr_nat (fun p : Bool × Bool × Usize => os.val.length - p.2.2.val)
    (fun p : Bool × Bool × Usize => p.2.2.val ≤ os.val.length ∧
      (∀ j (hj : j < p.2.2.val) (hj' : j < os.val.length), Gate.metB os.val[j] = true) ∧
      (p.1 = true ↔ ∃ j, ∃ hj : j < os.val.length, j < p.2.2.val ∧ isW os.val[j] = true) ∧
      (p.2.1 = true ↔ ∃ j, ∃ hj : j < os.val.length, j < p.2.2.val ∧ isA os.val[j] = true))
  · rintro ⟨w, a, i⟩ ⟨hi, hprefix, hw, ha⟩
    unfold delivery.standing_loop.body
    dsimp only at hi hprefix hw ha ⊢
    split
    · rename_i hlt
      have hil : i.val < os.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      subst hx
      step with unmet_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok]
        left
        refine ⟨i.val, hil, by simpa [hbt] using hb, fun j hj => hprefix j hj _, rfl⟩
      · rename_i hbt
        have hm : Gate.metB os.val[i.val] = true := by simpa [hbt] using hb
        have hif : ∀ (c v : Bool), (if c = true then ok true else ok v : Result Bool) = ok (c || v) := by
          intro c v; cases c <;> rfl
        step with is_waived_spec as ⟨ b1, hb1 ⟩
        simp only [hif, bind_tc_ok]
        step with is_attested_spec as ⟨ b2, hb2 ⟩
        step as ⟨ i', hi' ⟩
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, ?_, ?_, by scalar_tac⟩
        · intro j hj hj'
          by_cases hji : j < i.val
          · exact hprefix j hji hj'
          · have : j = i.val := by omega
            subst this; exact hm
        · rw [hi1, Bool.or_eq_true, hb1]
          constructor
          · rintro (h | h)
            · exact ⟨i.val, hil, by omega, h⟩
            · obtain ⟨j, hj, hji, hjw⟩ := hw.mp h
              exact ⟨j, hj, by omega, hjw⟩
          · rintro ⟨j, hj, hji, hjw⟩
            by_cases hjeq : j = i.val
            · subst hjeq; exact Or.inl hjw
            · exact Or.inr (hw.mpr ⟨j, hj, by omega, hjw⟩)
        · rw [hi1, Bool.or_eq_true, hb2]
          constructor
          · rintro (h | h)
            · exact ⟨i.val, hil, by omega, h⟩
            · obtain ⟨j, hj, hji, hja⟩ := ha.mp h
              exact ⟨j, hj, by omega, hja⟩
          · rintro ⟨j, hj, hji, hja⟩
            by_cases hjeq : j = i.val
            · subst hjeq; exact Or.inl hja
            · exact Or.inr (ha.mpr ⟨j, hj, by omega, hja⟩)
    · rename_i hge
      have hlen : i.val = os.val.length := by scalar_tac
      have hall : ∀ o ∈ os.val, Gate.metB o = true := by
        intro o ho
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ho
        exact hprefix j (by omega) hj
      have hW : os.val.any isW = w := by
        cases w
        · rw [List.any_eq_false]
          intro o ho hwo
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ho
          have := hw.mpr ⟨j, hj, by omega, hwo⟩
          cases this
        · rw [List.any_eq_true]
          obtain ⟨j, hj, _, hjw⟩ := hw.mp rfl
          exact ⟨_, List.getElem_mem hj, hjw⟩
      have hA : os.val.any isA = a := by
        cases a
        · rw [List.any_eq_false]
          intro o ho hao
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ho
          have := ha.mpr ⟨j, hj, by omega, hao⟩
          cases this
        · rw [List.any_eq_true]
          obtain ⟨j, hj, _, hja⟩ := ha.mp rfl
          exact ⟨_, List.getElem_mem hj, hja⟩
      cases w <;> cases a <;> simp only [Bool.false_eq_true, ↓reduceIte, WP.spec_ok] <;>
        exact Or.inr ⟨hall, by simp [allMet, hW, hA]⟩
  · exact ⟨hi, hprefix, hw, ha⟩

theorem standing_spec (os : Slice gate.Standing) :
    delivery.standing os ⦃ s => StandingSpec os.val s ⦄ := by
  unfold delivery.standing
  step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
  split
  · rename_i hempty
    have hnil : os.val = [] := by
      have : os.length = 0 := by simpa [hempty] using he.symm
      simpa [Slice.length] using this
    simp [StandingSpec, hnil]
  · rename_i hne
    have hnil : os.val ≠ [] := by
      intro h0; apply hne; rw [he]; simp [Slice.length, h0]
    apply WP.spec_mono (standing_loop_spec os false false 0#usize (by simp)
      (by intro j hj; simp at hj) (by simp) (by simp))
    intro s hs
    rcases hs with h1 | ⟨h1, h2⟩
    · exact Or.inr (Or.inl h1)
    · exact Or.inr (Or.inr ⟨hnil, h1, h2⟩)

/-- A delivery is met exactly when it has obligations and each is met. -/
theorem standing_met_iff (os : Slice gate.Standing) :
    (do let s ← delivery.standing os; gate.Standing.met s) ⦃ b =>
      b = true ↔ os.val ≠ [] ∧ ∀ o ∈ os.val, Gate.Met o ⦄ := by
  step with standing_spec as ⟨ s, hs ⟩
  apply WP.spec_mono (Gate.met_spec s)
  intro b hb
  rw [hb]
  change Gate.Met s ↔ _
  rcases hs with ⟨hnil, rfl⟩ | ⟨i, h, hf, _, rfl⟩ | ⟨hne, hall, rfl⟩
  · simp [Gate.Met, hnil]
  · constructor
    · intro hm
      rw [← Gate.metB_iff] at hm
      simp_all
    · rintro ⟨_, hall⟩
      have := (Gate.metB_iff _).mpr (hall _ (List.getElem_mem h))
      simp_all
  · refine ⟨fun _ => ⟨hne, fun o ho => (Gate.metB_iff o).mp (hall o ho)⟩, fun _ => ?_⟩
    unfold allMet Gate.Met
    split
    · simp
    · split <;> simp

/-- `satisfied` exactly when there are obligations and each is a passing
check: an attestation or a waiver is never read as one. -/
theorem standing_satisfied_iff (os : Slice gate.Standing) :
    delivery.standing os ⦃ s => s = .Checked .Satisfied ↔
      os.val ≠ [] ∧ ∀ o ∈ os.val, o = .Checked .Satisfied ⦄ := by
  apply WP.spec_mono (standing_spec os)
  intro s hs
  rcases hs with ⟨hnil, rfl⟩ | ⟨i, h, hf, _, rfl⟩ | ⟨hne, hall, rfl⟩
  · simp [hnil]
  · constructor
    · intro hsat
      rw [hsat] at hf
      simp [Gate.metB] at hf
    · rintro ⟨_, hall⟩
      have := hall _ (List.getElem_mem h)
      rw [this] at hf
      simp [Gate.metB] at hf
  · unfold allMet
    constructor
    · intro hsat
      refine ⟨hne, fun o ho => ?_⟩
      have hm := (Gate.metB_iff o).mp (hall o ho)
      split at hsat
      · cases hsat
      · split at hsat
        · cases hsat
        · rename_i hw ha
          rcases hm with h1 | h1 | h1
          · exact h1
          · exact absurd (List.any_eq_true.mpr ⟨o, ho, by simp [isA, h1]⟩) ha
          · exact absurd (List.any_eq_true.mpr ⟨o, ho, by simp [isW, h1]⟩) hw
    · rintro ⟨_, hsat⟩
      have hnw : os.val.any isW = false := by
        rw [List.any_eq_false]
        intro o ho; rw [hsat o ho]; simp [isW]
      have hna : os.val.any isA = false := by
        rw [List.any_eq_false]
        intro o ho; rw [hsat o ho]; simp [isA]
      simp [hnw, hna]

/-- When some obligation is not met, the delivery stands as the first unmet
obligation. -/
theorem standing_first_unmet (os : Slice gate.Standing)
    (h : ∃ o ∈ os.val, Gate.metB o = false) :
    delivery.standing os ⦃ s => ∃ i, ∃ h : i < os.val.length, Gate.metB os.val[i] = false ∧
      (∀ j (hj : j < i), Gate.metB (os.val[j]'(Nat.lt_trans hj h)) = true) ∧ s = os.val[i] ⦄ := by
  apply WP.spec_mono (standing_spec os)
  intro s hs
  obtain ⟨o, ho, hof⟩ := h
  rcases hs with ⟨hnil, _⟩ | hfirst | ⟨_, hall, _⟩
  · simp [hnil] at ho
  · exact hfirst
  · simp [hall o ho] at hof

/-! ## `ready` -/

/-- A candidate is ready exactly when its mode declares at least one
obligation and every one is met. -/
theorem ready_iff (os : Slice gate.Standing) :
    delivery.ready os ⦃ b => b = true ↔ os.val ≠ [] ∧ ∀ o ∈ os.val, Gate.Met o ⦄ := by
  unfold delivery.ready
  exact standing_met_iff os

/-- A mode that declares nothing is never ready. -/
theorem empty_never_ready (os : Slice gate.Standing) (h : os.val = []) :
    delivery.ready os ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (ready_iff os)
  intro b hb
  cases b
  · rfl
  · exact absurd h (hb.mp rfl).1

/-! ## `rechecked` -/

/-- A passed check whose cited evidence moved is stale; nothing else changes. -/
def recheckedOf (c : gate.State) (moved : Nat) : gate.State :=
  match c with
  | .Satisfied => if 0 < moved then .Stale else .Satisfied
  | c => c

theorem rechecked_spec (c : gate.State) (moved : Usize) :
    delivery.rechecked c moved ⦃ s => s = recheckedOf c moved.val ⦄ := by
  unfold delivery.rechecked
  cases c
  all_goals simp only [recheckedOf]
  all_goals try simp
  by_cases h : 0 < moved.val
  · have : moved > 0#usize := by scalar_tac
    simp [this, h]
  · have : ¬ moved > 0#usize := by scalar_tac
    simp [this, h]

/-! ## The delivery guard -/

/-- Known bytes pass only when a ready candidate holds them; unknown bytes
only when the run has a candidate and every candidate is ready. -/
theorem guard_allow_iff (body : delivery.Body) (same : Bool) (n : Usize) (all : Bool) :
    delivery.guard body same n all ⦃ b => b = true ↔
      (body = .Known ∧ same = true) ∨ (body = .Unknown ∧ 0 < n.val ∧ all = true) ⦄ := by
  unfold delivery.guard
  cases body
  · simp
  · by_cases h : 0 < n.val
    · have : n > 0#usize := by scalar_tac
      cases all <;> simp [this, h]
    · have : ¬ n > 0#usize := by scalar_tac
      simp [this, h]

end Writfold.Core.Delivery
