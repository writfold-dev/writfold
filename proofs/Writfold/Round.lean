import Writfold.Plan

/-!
# Loops (#75)

Theorems over the Aeneas translation of `writfold_core::plan::round`: a plan
step that repeats a template until its gates are met
(`docs/rfcs/2026-11-21-loops.md`).

- `exit_spec`, `exit_met_iff`, `exit_again_below_max`, `exit_spent_iff`: a
  loop is met with enough rounds and every gate met, may go around while
  rounds are below `max`, and is spent otherwise.
- `again_spec`, `again_admitted_iff`, `again_bounded`: `plan again` is
  admitted exactly at the current loop step whose exit is `Again`, and the
  round it records never exceeds `max`; the refusal order is in the spec.
- `advance_iff`, `advance_implies_plan_advance`: a loop step advances
  exactly when it is current and met, and never when the plan kernel would
  refuse: declaring a loop cannot weaken a step's gates.
- `spent_never_advances`, `spent_never_again`, `again_advance_exclusive`,
  `loop_step_progress`, `one_round_is_plain`.
- `superseded_iff`, `superseded_any_iff`, `superseded_never_stale`,
  `newest_round_unchanged`, `over_iff`: earlier rounds are history.
- `next_advance_iff`, `next_judge_first`, `next_again_implies_admissible`,
  `next_report_implies_spent`: the `next:` line is honest.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Plan

namespace Writfold.Core.Round

/-- `exit` on the model. -/
def exitOf (min max r : Nat) (allMet : Bool) : plan.round.Exit :=
  if min ≤ r ∧ allMet = true then .Met else if r < max then .Again else .Spent

theorem all_met_spec (met : Slice Bool) :
    plan.round.all_met met ⦃ a => a = (firstOwed met.val).isNone ⦄ := by
  unfold plan.round.all_met
  step with first_owed_spec as ⟨ o, ho ⟩
  rw [← ho]; cases o <;> rfl

theorem exit_spec (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.exit b r met ⦃ e => e = exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone ⦄ := by
  unfold plan.round.exit
  split
  · rename_i h1
    step with all_met_spec as ⟨ a, ha ⟩
    split
    · rename_i h2
      simp only [WP.spec_ok, exitOf]
      rw [if_pos ⟨by scalar_tac, by rw [← ha]; exact h2⟩]
    · rename_i h2
      have hn : ¬ (b.min.val ≤ r.val ∧ (firstOwed met.val).isNone = true) := by
        rintro ⟨_, h⟩; exact h2 (by rw [ha]; exact h)
      split
      · simp only [WP.spec_ok, exitOf]; rw [if_neg hn, if_pos (by scalar_tac)]
      · simp only [WP.spec_ok, exitOf]; rw [if_neg hn, if_neg (by scalar_tac)]
  · rename_i h1
    have hn : ¬ (b.min.val ≤ r.val ∧ (firstOwed met.val).isNone = true) := by
      rintro ⟨h, _⟩; exact h1 (by scalar_tac)
    split
    · simp only [WP.spec_ok, exitOf]; rw [if_neg hn, if_pos (by scalar_tac)]
    · simp only [WP.spec_ok, exitOf]; rw [if_neg hn, if_neg (by scalar_tac)]

theorem exit_met_iff (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.exit b r met ⦃ e => e = .Met ↔
      b.min.val ≤ r.val ∧ ∀ x ∈ met.val, x = true ⦄ := by
  apply WP.spec_mono (exit_spec b r met)
  intro e he; subst he
  rw [← firstOwed_none_iff]
  unfold exitOf
  by_cases h : b.min.val ≤ r.val ∧ (firstOwed met.val).isNone = true
  · rw [if_pos h]; simp only [true_iff]; exact ⟨h.1, by simpa using h.2⟩
  · rw [if_neg h]
    constructor
    · intro hc; split at hc <;> simp at hc
    · rintro ⟨h1, h2⟩; exact absurd ⟨h1, by simp [h2]⟩ h

theorem exit_again_below_max (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.exit b r met ⦃ e => e = .Again → r.val < b.max.val ⦄ := by
  apply WP.spec_mono (exit_spec b r met)
  intro e he hA; subst he
  unfold exitOf at hA
  split at hA
  · simp at hA
  · split at hA
    · assumption
    · simp at hA

theorem exit_spent_iff (b : plan.round.Bounds) (r : Usize) (met : Slice Bool)
    (hb : b.min.val ≤ b.max.val) :
    plan.round.exit b r met ⦃ e => e = .Spent ↔
      b.max.val ≤ r.val ∧ false ∈ met.val ⦄ := by
  apply WP.spec_mono (exit_spec b r met)
  intro e he; subst he
  have hf : (firstOwed met.val).isNone = true ↔ ¬ false ∈ met.val := by
    rw [Option.isNone_iff_eq_none, firstOwed_none_iff]
    constructor
    · intro h hm; exact absurd (h false hm) (by simp)
    · intro h x hx; cases x
      · exact absurd hx h
      · rfl
  unfold exitOf
  by_cases h1 : b.min.val ≤ r.val ∧ (firstOwed met.val).isNone = true
  · rw [if_pos h1]; simp only [reduceCtorEq, false_iff, not_and]
    intro _ hm; exact (hf.mp h1.2) hm
  · rw [if_neg h1]
    by_cases h2 : r.val < b.max.val
    · rw [if_pos h2]; simp only [reduceCtorEq, false_iff, not_and]; intro h; omega
    · rw [if_neg h2]; simp only [true_iff]
      refine ⟨by omega, ?_⟩
      by_contra hm
      exact h1 ⟨by omega, hf.mpr hm⟩

/-- `plan again`'s decision, refusal by refusal, in the fixed order. -/
def AgainSpec (nodes : List plan.Node) (pos : Nat) (looped : Bool) (e : plan.round.Exit)
    (r : Nat) : core.result.Result Usize plan.round.AgainRefusal → Prop
  | .Ok n => cur nodes = some pos ∧ looped = true ∧ e = .Again ∧ n.val = r + 1
  | .Err .Complete => cur nodes = none
  | .Err (.NotCurrent now) => cur nodes = some now.val ∧ now.val ≠ pos
  | .Err .NotLoop => cur nodes = some pos ∧ looped = false
  | .Err .Met => cur nodes = some pos ∧ looped = true ∧ e = .Met
  | .Err .Spent => cur nodes = some pos ∧ looped = true ∧ e = .Spent

theorem again_spec (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.again_check nodes pos looped b r met ⦃ res =>
      AgainSpec nodes.val pos.val looped
        (exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone) r.val res ⦄ := by
  unfold plan.round.again_check
  step with current_spec as ⟨ o, ho ⟩
  cases o with
  | none =>
    simp only [WP.spec_ok, AgainSpec]
    simpa using ho.symm
  | some now =>
    simp only
    split
    · rename_i hne
      simp only [WP.spec_ok, AgainSpec]
      refine ⟨by simpa using ho.symm, ?_⟩
      intro h
      simp only [bne_iff_ne, ne_eq] at hne
      exact hne (UScalar.eq_of_val_eq h)
    · rename_i heq
      have hnow : cur nodes.val = some pos.val := by
        have : now.val = pos.val := by simpa using heq
        rw [← this]
        simpa using ho.symm
      cases looped
      · simp only [Bool.false_eq_true, ite_false, WP.spec_ok, AgainSpec]
        exact ⟨hnow, by simp⟩
      · simp only [ite_true]
        step with exit_spec as ⟨ e, he ⟩
        cases e with
        | Met =>
          simp only [WP.spec_ok, AgainSpec]
          exact ⟨hnow, by simp, he.symm⟩
        | Spent =>
          simp only [WP.spec_ok, AgainSpec]
          exact ⟨hnow, by simp, he.symm⟩
        | Again =>
          have hlt : r.val < b.max.val := by
            have h := he.symm
            unfold exitOf at h
            split at h
            · simp at h
            · split at h
              · assumption
              · simp at h
          step as ⟨ n, hn ⟩
          simp only [WP.spec_ok, AgainSpec]
          exact ⟨hnow, by simp, he.symm, hn⟩

theorem again_admitted_iff (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.again_check nodes pos looped b r met ⦃ res =>
      (∃ n, res = .Ok n) ↔ cur nodes.val = some pos.val ∧ looped = true ∧
        exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Again ⦄ := by
  apply WP.spec_mono (again_spec nodes pos looped b r met)
  intro res hres
  cases res with
  | Ok n =>
    simp only [AgainSpec] at hres
    exact ⟨fun _ => ⟨hres.1, hres.2.1, hres.2.2.1⟩, fun _ => ⟨n, rfl⟩⟩
  | Err refusal =>
    constructor
    · rintro ⟨n, hn⟩; simp at hn
    · rintro ⟨hc, hl, he⟩
      exfalso
      cases refusal <;> simp only [AgainSpec] at hres
      · rw [hres] at hc; simp at hc
      · rename_i now; rw [hres.1] at hc; exact hres.2 (by simpa using hc)
      · rw [hres.2] at hl; simp at hl
      · rw [hres.2.2] at he; simp at he
      · rw [hres.2.2] at he; simp at he

theorem again_bounded (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.again_check nodes pos looped b r met ⦃ res =>
      ∀ n, res = .Ok n → n.val ≤ b.max.val ⦄ := by
  apply WP.spec_mono (again_spec nodes pos looped b r met)
  intro res hres n hn
  subst hn
  simp only [AgainSpec] at hres
  obtain ⟨_, _, he, hn⟩ := hres
  unfold exitOf at he
  split at he
  · simp at he
  · split at he
    · omega
    · simp at he

/-- `plan advance` on a loop step: admitted exactly when the step is current
and its exit is met. -/
theorem advance_iff (nodes : Slice plan.Node) (pos : Usize) (b : plan.round.Bounds)
    (r : Usize) (met : Slice Bool) :
    plan.round.advance_check nodes pos b r met ⦃ res =>
      res = .Ok () ↔ cur nodes.val = some pos.val ∧
        exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Met ⦄ := by
  unfold plan.round.advance_check
  step with advance_check_spec as ⟨ p, hp ⟩
  cases p with
  | Ok u =>
    simp only [AdvanceSpec] at hp
    simp only
    split
    · rename_i hlt
      have hle : r.val ≤ b.min.val := by scalar_tac
      step as ⟨ i, hi ⟩
      simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and]
      intro _ h
      unfold exitOf at h
      split at h
      · rename_i hc; have := hc.1; scalar_tac
      · split at h <;> simp at h
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      refine ⟨hp.1, ?_⟩
      unfold exitOf
      rw [if_pos ⟨by scalar_tac, by simp [hp.2]⟩]
  | Err refusal =>
    simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and]
    intro hc he
    unfold exitOf at he
    split at he
    · rename_i hm
      have hnone : firstOwed met.val = none := by simpa using hm.2
      cases refusal <;> simp only [AdvanceSpec] at hp
      · rw [hp] at hc; simp at hc
      · rename_i now; rw [hp.1] at hc; exact hp.2 (by simpa using hc)
      · rename_i g; rw [hnone] at hp; simp at hp
    · split at he <;> simp at he

/-- A loop never advances a step the plan kernel would refuse. -/
theorem advance_implies_plan_advance (nodes : Slice plan.Node) (pos : Usize)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.advance_check nodes pos b r met ⦃ res =>
      res = .Ok () → cur nodes.val = some pos.val ∧ ∀ x ∈ met.val, x = true ⦄ := by
  apply WP.spec_mono (advance_iff nodes pos b r met)
  intro res hres hok
  obtain ⟨hc, he⟩ := hres.mp hok
  refine ⟨hc, ?_⟩
  rw [← firstOwed_none_iff]
  unfold exitOf at he
  split at he
  · rename_i h; simpa using h.2
  · split at he <;> simp at he

/-- Running out of rounds is never a way through. -/
theorem spent_never_advances (nodes : Slice plan.Node) (pos : Usize)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool)
    (hs : exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Spent) :
    plan.round.advance_check nodes pos b r met ⦃ res => res ≠ .Ok () ⦄ := by
  apply WP.spec_mono (advance_iff nodes pos b r met)
  intro res hres hok
  have := (hres.mp hok).2; rw [hs] at this; simp at this

theorem spent_never_again (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool)
    (hs : exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Spent) :
    plan.round.again_check nodes pos looped b r met ⦃ res => ∀ n, res ≠ .Ok n ⦄ := by
  apply WP.spec_mono (again_admitted_iff nodes pos looped b r met)
  intro res hres n hok
  have := (hres.mp ⟨n, hok⟩).2.2; rw [hs] at this; simp at this

/-- On the same inputs, `plan again` and `plan advance` are never both
admitted. -/
theorem again_advance_exclusive (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Again →
      exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone ≠ .Met := by
  intro h; rw [h]; simp

/-- At the current loop step exactly one holds: advance, again, or spent. -/
theorem loop_step_progress (min max r : Nat) (a : Bool) :
    (exitOf min max r a = .Met ∧ exitOf min max r a ≠ .Again ∧ exitOf min max r a ≠ .Spent) ∨
    (exitOf min max r a = .Again ∧ exitOf min max r a ≠ .Met ∧ exitOf min max r a ≠ .Spent) ∨
    (exitOf min max r a = .Spent ∧ exitOf min max r a ≠ .Met ∧ exitOf min max r a ≠ .Again) := by
  unfold exitOf
  split
  · simp
  · split <;> simp

/-- One round at most is a plain step: met exactly when every gate is met
(after the one round the plan splices), and never repeated. -/
theorem one_round_is_plain (a : Bool) :
    exitOf 1 1 1 a = .Met ↔ a = true := by
  unfold exitOf; cases a <;> simp

theorem one_round_never_again (r : Nat) (a : Bool) (hr : 1 ≤ r) :
    exitOf 1 1 r a ≠ .Again := by
  unfold exitOf; split
  · simp
  · split
    · omega
    · simp

theorem superseded_iff (p : plan.round.Place) :
    plan.round.superseded p ⦃ s => s = true ↔ p.round.val < p.rounds.val ⦄ := by
  unfold plan.round.superseded; simp

theorem superseded_any_loop_spec (path : Slice plan.round.Place) (i : Usize)
    (hi : i.val ≤ path.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < path.val.length),
      ¬ (path.val[j].round.val < path.val[j].rounds.val)) :
    plan.round.superseded_any_loop path i ⦃ s => s = true ↔
      ∃ p ∈ path.val, p.round.val < p.rounds.val ⦄ := by
  unfold plan.round.superseded_any_loop
  apply loop.spec_decr_nat (fun i : Usize => path.val.length - i.val)
    (fun i : Usize => i.val ≤ path.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < path.val.length),
        ¬ (path.val[j].round.val < path.val[j].rounds.val))
  · intro i ⟨hi, hprefix⟩
    unfold plan.round.superseded_any_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hmem : path.val[i.val]'(by scalar_tac) ∈ path.val := List.getElem_mem _
      step with superseded_iff as ⟨ s, hs ⟩
      split
      · rename_i hst
        simp only [WP.spec_ok, true_iff]
        exact ⟨_, hmem, by rw [← hx]; exact hs.mp hst⟩
      · rename_i hsf
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro h; exact hsf (hs.mpr h)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro p hp
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hp
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

theorem superseded_any_iff (path : Slice plan.round.Place) :
    plan.round.superseded_any path ⦃ s => s = true ↔
      ∃ p ∈ path.val, p.round.val < p.rounds.val ⦄ := by
  unfold plan.round.superseded_any
  exact superseded_any_loop_spec path 0#usize (by simp) (by intro j hj; simp at hj)

theorem superseded_any_total (path : Slice plan.round.Place) :
    plan.round.superseded_any path ⦃ _ => True ⦄ :=
  WP.spec_mono (superseded_any_iff path) (fun _ _ => trivial)

theorem superseded_never_stale (n : plan.Node) (met : Slice Bool) :
    plan.round.stale n met true ⦃ s => s = false ⦄ := by
  unfold plan.round.stale; simp

theorem newest_round_unchanged (n : plan.Node) (met : Slice Bool) :
    plan.round.stale n met false ⦃ s => s = true ↔ n.done = true ∧ false ∈ met.val ⦄ := by
  unfold plan.round.stale
  try (split; · rename_i h; simp at h)
  exact stale_spec n met

theorem over_iff (b : plan.round.Bounds) (r : Usize) :
    plan.round.over b r ⦃ o => o = true ↔ b.max.val < r.val ⦄ := by
  unfold plan.round.over; simp

theorem judged_at_spec (judged : Slice Bool) (i : Usize) :
    plan.round.judged_at judged i ⦃ j => j = true ↔
      ∃ h : i.val < judged.val.length, judged.val[i.val] = true ⦄ := by
  unfold plan.round.judged_at
  dsimp only
  split
  · rename_i hlt
    step as ⟨ x, hx ⟩
    try simp only [WP.spec_ok]
    constructor
    · intro h; exact ⟨by scalar_tac, by rw [← hx]; exact h⟩
    · rintro ⟨_, h⟩; rw [hx]; exact h
  · rename_i hge
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists]
    intro h; exact absurd (by scalar_tac) hge

theorem judged_at_total (judged : Slice Bool) (i : Usize) :
    plan.round.judged_at judged i ⦃ _ => True ⦄ :=
  WP.spec_mono (judged_at_spec judged i) (fun _ _ => trivial)

/-- What `next` names, on the model. -/
def NextSpec (e : plan.round.Exit) (owed : Option Nat) (judged : List Bool) :
    plan.round.Next → Prop
  | .Advance => e = .Met
  | .Judge i => e ≠ .Met ∧ owed = some i.val ∧
      ¬ (∃ h : i.val < judged.length, judged[i.val] = true)
  | .Again => e = .Again ∧
      (∀ i, owed = some i → ∃ h : i < judged.length, judged[i] = true)
  | .Report => e = .Spent ∧
      (∀ i, owed = some i → ∃ h : i < judged.length, judged[i] = true)

/-- `owed_or`: the first owed gate if it has no verdict, else `then`. -/
theorem owed_or_spec (met judged : Slice Bool) (t : plan.round.Next) :
    plan.round.owed_or met judged t ⦃ n =>
      (∃ i, n = .Judge i ∧ firstOwed met.val = some i.val ∧
        ¬ (∃ h : i.val < judged.val.length, judged.val[i.val] = true)) ∨
      (n = t ∧ ∀ i, firstOwed met.val = some i → ∃ h : i < judged.val.length, judged.val[i] = true) ⦄ := by
  unfold plan.round.owed_or
  step with first_owed_spec as ⟨ o, ho ⟩
  cases o with
  | none =>
    simp only [WP.spec_ok]
    right; refine ⟨by simp, ?_⟩
    intro i hi; rw [← ho] at hi; simp at hi
  | some i =>
    simp only
    step with judged_at_spec as ⟨ j, hj ⟩
    split
    · rename_i hjt
      simp only [WP.spec_ok]
      right; refine ⟨by simp, ?_⟩
      intro k hk; rw [← ho] at hk; simp only [Option.map_some, Option.some.injEq] at hk
      subst hk; exact hj.mp hjt
    · rename_i hjf
      simp only [WP.spec_ok]
      left; exact ⟨i, rfl, by rw [← ho]; simp, fun h => hjf (hj.mpr h)⟩

theorem next_spec (b : plan.round.Bounds) (r : Usize) (met judged : Slice Bool) :
    plan.round.next b r met judged ⦃ n =>
      NextSpec (exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone)
        (firstOwed met.val) judged.val n ⦄ := by
  unfold plan.round.next
  step with exit_spec as ⟨ e, he ⟩
  rw [← he]
  cases e with
  | Met => simp [NextSpec]
  | Again =>
    simp only
    apply WP.spec_mono (owed_or_spec met judged .Again)
    rintro n (⟨i, rfl, h1, h2⟩ | ⟨rfl, h⟩)
    · exact ⟨by simp, h1, h2⟩
    · exact ⟨rfl, h⟩
  | Spent =>
    simp only
    apply WP.spec_mono (owed_or_spec met judged .Report)
    rintro n (⟨i, rfl, h1, h2⟩ | ⟨rfl, h⟩)
    · exact ⟨by simp, h1, h2⟩
    · exact ⟨rfl, h⟩

theorem next_advance_iff (b : plan.round.Bounds) (r : Usize) (met judged : Slice Bool) :
    plan.round.next b r met judged ⦃ n => n = .Advance ↔
      exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Met ⦄ := by
  apply WP.spec_mono (next_spec b r met judged)
  intro n hn
  cases n with
  | Advance => simp only [NextSpec] at hn; simp [hn]
  | Judge i => simp only [NextSpec] at hn; simp only [reduceCtorEq, false_iff]; exact hn.1
  | Again => simp only [NextSpec] at hn; simp only [reduceCtorEq, false_iff]; rw [hn.1]; simp
  | Report => simp only [NextSpec] at hn; simp only [reduceCtorEq, false_iff]; rw [hn.1]; simp

theorem next_judge_first (b : plan.round.Bounds) (r : Usize) (met judged : Slice Bool) :
    plan.round.next b r met judged ⦃ n => ∀ i, n = .Judge i →
      firstOwed met.val = some i.val ∧ ¬ (∃ h : i.val < judged.val.length, judged.val[i.val] = true) ⦄ := by
  apply WP.spec_mono (next_spec b r met judged)
  intro n hn i hi; subst hi
  simp only [NextSpec] at hn
  exact ⟨hn.2.1, hn.2.2⟩

theorem next_again_implies_admissible (b : plan.round.Bounds) (r : Usize) (met judged : Slice Bool) :
    plan.round.next b r met judged ⦃ n => n = .Again →
      exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Again ⦄ := by
  apply WP.spec_mono (next_spec b r met judged)
  intro n hn h; subst h; simp only [NextSpec] at hn; exact hn.1

theorem next_report_implies_spent (b : plan.round.Bounds) (r : Usize) (met judged : Slice Bool) :
    plan.round.next b r met judged ⦃ n => n = .Report →
      exitOf b.min.val b.max.val r.val (firstOwed met.val).isNone = .Spent ⦄ := by
  apply WP.spec_mono (next_spec b r met judged)
  intro n hn h; subst h; simp only [NextSpec] at hn; exact hn.1

theorem again_total (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.again_check nodes pos looped b r met ⦃ _ => True ⦄ :=
  WP.spec_mono (again_spec nodes pos looped b r met) (fun _ _ => trivial)

theorem advance_total (nodes : Slice plan.Node) (pos : Usize) (b : plan.round.Bounds)
    (r : Usize) (met : Slice Bool) :
    plan.round.advance_check nodes pos b r met ⦃ _ => True ⦄ :=
  WP.spec_mono (advance_iff nodes pos b r met) (fun _ _ => trivial)

end Writfold.Core.Round
