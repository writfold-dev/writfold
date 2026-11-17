import Writfold.Current

/-!
# Deliveries and the delivery guard

Theorems over the Aeneas translation of `writfold_core::delivery`: which
candidate decides an open delivery step, one standing and one readiness for
a delivery from its obligations (each with the standings its policy takes:
a mode's `waivable`, a mode gate's `accept`), the plan fact it gives a step,
a `posted:<mode>` gate, the re-check of a passed check whose cited evidence
moved, and the delivery guard's decision over every candidate of the run.

- `deciding_spec`: scanning from the newest, withdrawn candidates are passed
  over; the first other one decides if it is open, nothing if it is bound.
  `deciding_none_after_bound`: an older draft never decides once a newer
  candidate of its mode is bound (fixed on 17 November 2026: the newest
  unbound candidate decided, so an abandoned draft met a later step).
  `deciding_withdrawn_irrelevant`: inserting a withdrawn candidate changes
  nothing but the index.
- `first_unmet_spec`, `standing_spec`, `standing_first_unmet`,
  `standing_satisfied_iff`, `standing_met_iff`.
- `ready_iff`: ready exactly when there is an obligation and each is met
  under its policy. `empty_never_ready`; `waiver_rejected_never_ready`
  (fixed on 17 November 2026: a mode gate's `accept` was ignored for a
  delivery); `default_accept_is_before`: with every word taken, readiness
  and the standing are the rule before this round; `accept_only_narrows`.
- `refused_iff`; `fact_met_iff_ready`: the plan's proved `fact_met` of what
  a `deliver:<mode>` gate is given is exactly readiness.
- `posted_spec`, `posted_never_satisfied`, `posted_met_iff`.
- `rechecked_spec`.
- `postable_spec`, `guard_any_loop_spec`, `guard_all_loop_spec`,
  `guard_allow_iff`, `guard_deny_unread`, `guard_withdrawn_irrelevant`,
  `guard_spent_never_passes`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

deriving instance DecidableEq for writfold_core.delivery.Place

namespace Writfold.Core.Delivery

/-! ## `deciding` -/

theorem is_open_spec (p : delivery.Place) : delivery.is_open p ⦃ b => b = true ↔ p = .Open ⦄ := by
  unfold delivery.is_open
  cases p <;> simp

theorem is_bound_spec (p : delivery.Place) : delivery.is_bound p ⦃ b => b = true ↔ p = .Bound ⦄ := by
  unfold delivery.is_bound
  cases p <;> simp

/-- The deciding candidate: an open one with only withdrawn candidates after
it; `none` when no candidate is such. -/
def DecidingSpec (l : List delivery.Place) : Option Nat → Prop
  | some i => ∃ h : i < l.length, l[i] = .Open ∧
      ∀ j (hj : j < l.length), i < j → l[j] = .Withdrawn
  | none => ∀ i (hi : i < l.length), l[i] = .Open →
      ∃ j, ∃ hj : j < l.length, i < j ∧ l[j] ≠ .Withdrawn

theorem deciding_loop_spec (ps : Slice delivery.Place) (i : Usize) (hi : i.val ≤ ps.val.length)
    (hlater : ∀ j (hj : j < ps.val.length), i.val ≤ j → ps.val[j] = .Withdrawn) :
    delivery.deciding_loop ps i ⦃ r => DecidingSpec ps.val (r.map (·.val)) ⦄ := by
  unfold delivery.deciding_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ ps.val.length ∧
      ∀ j (hj : j < ps.val.length), i.val ≤ j → ps.val[j] = .Withdrawn)
  · intro i ⟨hi, hlater⟩
    unfold delivery.deciding_loop.body
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ p, hp ⟩
      have hil : i1.val < ps.val.length := by scalar_tac
      step with is_open_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, Option.map_some, DecidingSpec]
        refine ⟨hil, by rw [← hp]; exact hb.mp hbt, ?_⟩
        intro j hj hij
        exact hlater j hj (by scalar_tac)
      · rename_i hbt
        step with is_bound_spec as ⟨ b1, hb1 ⟩
        split
        · rename_i hb1t
          simp only [WP.spec_ok, Option.map_none, DecidingSpec]
          intro k hk hko
          have hbound : ps.val[i1.val] = .Bound := by rw [← hp]; exact hb1.mp hb1t
          by_cases hki : k < i1.val
          · exact ⟨i1.val, hil, hki, by rw [hbound]; simp⟩
          · by_cases hke : k = i1.val
            · subst hke; rw [hbound] at hko; cases hko
            · rw [hlater k hk (by scalar_tac)] at hko; cases hko
        · rename_i hb1t
          simp only [WP.spec_ok]
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro j hj hij
          by_cases hji : i1.val = j
          · subst hji
            rw [← hp]
            have h1 : p ≠ .Open := fun h => hbt (hb.mpr h)
            have h2 : p ≠ .Bound := fun h => hb1t (hb1.mpr h)
            cases p <;> simp_all
          · exact hlater j hj (by scalar_tac)
    · rename_i hzero
      simp only [WP.spec_ok, Option.map_none, DecidingSpec]
      intro k hk hko
      rw [hlater k hk (by scalar_tac)] at hko
      cases hko
  · exact ⟨hi, hlater⟩

/-- The candidate that decides an open step: scanning from the newest, the
first candidate that is not withdrawn, if it is open. -/
theorem deciding_spec (ps : Slice delivery.Place) :
    delivery.deciding ps ⦃ r => DecidingSpec ps.val (r.map (·.val)) ⦄ := by
  unfold delivery.deciding
  apply deciding_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-- `DecidingSpec` names one answer. -/
theorem decidingSpec_unique (l : List delivery.Place) (r s : Option Nat)
    (hr : DecidingSpec l r) (hs : DecidingSpec l s) : r = s := by
  rcases r with _ | i <;> rcases s with _ | k
  · rfl
  · obtain ⟨hk, hko, hkl⟩ := hs
    obtain ⟨j, hj, hkj, hjw⟩ := hr k hk hko
    exact absurd (hkl j hj hkj) hjw
  · obtain ⟨hi, hio, hil⟩ := hr
    obtain ⟨j, hj, hij, hjw⟩ := hs i hi hio
    exact absurd (hil j hj hij) hjw
  · obtain ⟨hi, hio, hil⟩ := hr
    obtain ⟨hk, hko, hkl⟩ := hs
    congr 1
    rcases Nat.lt_trichotomy i k with h | h | h
    · rw [hil k hk h] at hko; cases hko
    · exact h
    · rw [hkl i hi h] at hio; cases hio

/-- An older draft never decides once a newer candidate of its mode is
bound: a bound candidate with only withdrawn ones after it leaves no
deciding candidate. -/
theorem deciding_none_after_bound (ps : Slice delivery.Place) (j : Nat)
    (hj : j < ps.val.length) (hb : ps.val[j] = .Bound)
    (hlater : ∀ k (hk : k < ps.val.length), j < k → ps.val[k] = .Withdrawn) :
    delivery.deciding ps ⦃ r => r = none ⦄ := by
  apply WP.spec_mono (deciding_spec ps)
  intro r hr
  rcases r with _ | i
  · rfl
  · obtain ⟨hi, hio, hil⟩ := hr
    exfalso
    rcases Nat.lt_trichotomy i.val j with h | h | h
    · rw [hil j hj h] at hb; cases hb
    · simp only [h] at hio; rw [hb] at hio; cases hio
    · rw [hlater i.val hi h] at hio; cases hio

/-- Inserting a withdrawn candidate anywhere changes which candidate decides
only by the shift of its index; removing one is the converse. With
`deciding_spec` and `decidingSpec_unique`, `deciding` of the longer list is
this answer. -/
theorem deciding_withdrawn_irrelevant (l : List delivery.Place) (k : Nat) (hk : k ≤ l.length)
    (r : Option Nat) (hr : DecidingSpec l r) :
    DecidingSpec (l.insertIdx k .Withdrawn) (r.map (fun i => if i < k then i else i + 1)) := by
  have hlen : (l.insertIdx k .Withdrawn).length = l.length + 1 :=
    List.length_insertIdx_of_le_length hk _
  have at_lt : ∀ i (hi : i < k), (l.insertIdx k .Withdrawn)[i]'(by omega) = l[i]'(by omega) := by
    intro i hi; exact List.getElem_insertIdx_of_lt hi _
  have at_k : (l.insertIdx k .Withdrawn)[k]'(by omega) = .Withdrawn :=
    List.getElem_insertIdx_self (by omega)
  have at_gt : ∀ i (hi : i < l.length), k ≤ i →
      (l.insertIdx k .Withdrawn)[i + 1]'(by omega) = l[i] := by
    intro i hi hki
    have := List.getElem_insertIdx_add_succ l .Withdrawn k (i - k) (by omega)
    simp only [show k + (i - k) = i by omega] at this
    exact this
  rcases r with _ | i
  · simp only [Option.map_none, DecidingSpec]
    intro m hm hmo
    rcases Nat.lt_trichotomy m k with h | h | h
    · rw [at_lt m h] at hmo
      obtain ⟨j, hj, hmj, hjw⟩ := hr m (by omega) hmo
      by_cases hjk : j < k
      · exact ⟨j, by omega, hmj, by rw [at_lt j hjk]; exact hjw⟩
      · exact ⟨j + 1, by omega, by omega, by rw [at_gt j hj (by omega)]; exact hjw⟩
    · subst h; rw [at_k] at hmo; cases hmo
    · obtain ⟨m', rfl⟩ : ∃ m', m = m' + 1 := ⟨m - 1, by omega⟩
      rw [at_gt m' (by omega) (by omega)] at hmo
      obtain ⟨j, hj, hmj, hjw⟩ := hr m' (by omega) hmo
      exact ⟨j + 1, by omega, by omega, by rw [at_gt j hj (by omega)]; exact hjw⟩
  · obtain ⟨hi, hio, hil⟩ := hr
    simp only [Option.map_some, DecidingSpec]
    split
    · rename_i hik
      refine ⟨by omega, by rw [at_lt i hik]; exact hio, ?_⟩
      intro j hj hij
      rcases Nat.lt_trichotomy j k with h | h | h
      · rw [at_lt j h]; exact hil j (by omega) hij
      · simp only [h]; exact at_k
      · obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
        rw [at_gt j' (by omega) (by omega)]; exact hil j' (by omega) (by omega)
    · rename_i hik
      refine ⟨by omega, by rw [at_gt i hi (by omega)]; exact hio, ?_⟩
      intro j hj hij
      obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
      rw [at_gt j' (by omega) (by omega)]; exact hil j' (by omega) (by omega)

/-! ## Obligations under their policy -/

/-- An obligation met under its policy (`Policy.met_spec`). -/
def ometB (o : delivery.Owed) : Bool := Plan.pmetB o.standing o.accept

theorem ometB_iff (o : delivery.Owed) : ometB o = true ↔ Plan.PMet o.standing o.accept :=
  Plan.pmetB_iff _ _

theorem unmet_spec (o : delivery.Owed) : delivery.unmet o ⦃ b => b = !ometB o ⦄ := by
  unfold delivery.unmet
  step with Policy.met_spec as ⟨ m, hm ⟩
  have hpm := Plan.pmetB_iff o.standing o.accept
  unfold Plan.PMet at hpm
  have hmo : m = ometB o := by
    apply Bool.eq_iff_iff.mpr
    unfold ometB
    rw [hm, hpm]
  rw [hmo]
  cases ometB o <;> simp

/-- A waiver. -/
def isW : gate.Standing → Bool
  | .Waived => true
  | _ => false

/-- An attestation. -/
def isA : gate.Standing → Bool
  | .Attested => true
  | _ => false

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

/-! ## `first_unmet` -/

theorem first_unmet_loop_spec (os : Slice delivery.Owed) (i : Usize)
    (hi : i.val ≤ os.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < os.val.length), ometB os.val[j] = true) :
    delivery.first_unmet_loop os i ⦃ r => r.map (·.val) = Plan.leastFailing ometB os.val ⦄ := by
  unfold delivery.first_unmet_loop
  apply loop.spec_decr_nat (fun i : Usize => os.val.length - i.val)
    (fun i : Usize => i.val ≤ os.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < os.val.length), ometB os.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold delivery.first_unmet_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with unmet_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, Option.map_some]
        symm
        apply Plan.leastFailing_of_prefix _ os.val i.val (by scalar_tac)
        · rw [← hx]; cases h : ometB x <;> simp_all
        · intro j hj; exact hprefix j hj _
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; cases h : ometB x <;> simp_all
    · rename_i hge
      simp only [WP.spec_ok, Option.map_none]
      symm
      apply Plan.leastFailing_of_all
      intro j hj
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- The first obligation not met under its policy: `some i` exactly for the
least such `i`, `none` exactly when every obligation is met
(`Plan.leastFailing_some_iff`, `Plan.leastFailing_none_iff`). -/
theorem first_unmet_spec (os : Slice delivery.Owed) :
    delivery.first_unmet os ⦃ r => r.map (·.val) = Plan.leastFailing ometB os.val ⦄ := by
  unfold delivery.first_unmet
  apply first_unmet_loop_spec
  · simp
  · intro j hj; simp at hj

/-! ## `standing` -/

/-- With every obligation met: waived if any is, else attested if any is,
else satisfied. -/
def allMet (l : List delivery.Owed) : gate.Standing :=
  if l.any (fun o => isW o.standing) then .Waived
  else if l.any (fun o => isA o.standing) then .Attested else .Checked .Satisfied

/-- `standing`'s decision. -/
def StandingSpec (l : List delivery.Owed) (s : gate.Standing) : Prop :=
  (l = [] ∧ s = .Checked .Open) ∨
  (∃ i, ∃ h : i < l.length, ometB l[i] = false ∧
      (∀ j (hj : j < i), ometB (l[j]'(Nat.lt_trans hj h)) = true) ∧ s = l[i].standing) ∨
  (l ≠ [] ∧ (∀ o ∈ l, ometB o = true) ∧ s = allMet l)

theorem standing_loop_spec (os : Slice delivery.Owed) (w a : Bool) (i : Usize)
    (hi : i.val ≤ os.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < os.val.length), ometB os.val[j] = true)
    (hw : w = true ↔ ∃ j, ∃ hj : j < os.val.length, j < i.val ∧ isW os.val[j].standing = true)
    (ha : a = true ↔ ∃ j, ∃ hj : j < os.val.length, j < i.val ∧ isA os.val[j].standing = true) :
    delivery.standing_loop os w a i ⦃ s =>
      (∃ i, ∃ h : i < os.val.length, ometB os.val[i] = false ∧
        (∀ j (hj : j < i), ometB (os.val[j]'(Nat.lt_trans hj h)) = true) ∧
        s = os.val[i].standing) ∨
      ((∀ o ∈ os.val, ometB o = true) ∧ s = allMet os.val) ⦄ := by
  unfold delivery.standing_loop
  apply loop.spec_decr_nat (fun p : Bool × Bool × Usize => os.val.length - p.2.2.val)
    (fun p : Bool × Bool × Usize => p.2.2.val ≤ os.val.length ∧
      (∀ j (hj : j < p.2.2.val) (hj' : j < os.val.length), ometB os.val[j] = true) ∧
      (p.1 = true ↔ ∃ j, ∃ hj : j < os.val.length, j < p.2.2.val ∧
        isW os.val[j].standing = true) ∧
      (p.2.1 = true ↔ ∃ j, ∃ hj : j < os.val.length, j < p.2.2.val ∧
        isA os.val[j].standing = true))
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
        refine ⟨i.val, hil, ?_, fun j hj => hprefix j hj _, rfl⟩
        cases h : ometB os.val[i.val] <;> simp_all
      · rename_i hbt
        have hm : ometB os.val[i.val] = true := by
          cases h : ometB os.val[i.val] <;> simp_all
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
      have hall : ∀ o ∈ os.val, ometB o = true := by
        intro o ho
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ho
        exact hprefix j (by omega) hj
      have hW : os.val.any (fun o => isW o.standing) = w := by
        cases w
        · rw [List.any_eq_false]
          intro o ho hwo
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ho
          have := hw.mpr ⟨j, hj, by omega, hwo⟩
          cases this
        · rw [List.any_eq_true]
          obtain ⟨j, hj, _, hjw⟩ := hw.mp rfl
          exact ⟨_, List.getElem_mem hj, hjw⟩
      have hA : os.val.any (fun o => isA o.standing) = a := by
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

theorem standing_spec (os : Slice delivery.Owed) :
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

/-- When some obligation is not met under its policy, the delivery stands as
the first such obligation. -/
theorem standing_first_unmet (os : Slice delivery.Owed)
    (h : ∃ o ∈ os.val, ometB o = false) :
    delivery.standing os ⦃ s => ∃ i, ∃ h : i < os.val.length, ometB os.val[i] = false ∧
      (∀ j (hj : j < i), ometB (os.val[j]'(Nat.lt_trans hj h)) = true) ∧
      s = os.val[i].standing ⦄ := by
  apply WP.spec_mono (standing_spec os)
  intro s hs
  obtain ⟨o, ho, hof⟩ := h
  rcases hs with ⟨hnil, _⟩ | hfirst | ⟨_, hall, _⟩
  · simp [hnil] at ho
  · exact hfirst
  · simp [hall o ho] at hof

theorem ometB_met {o : delivery.Owed} (h : ometB o = true) : Gate.Met o.standing :=
  Plan.PMet.met ((ometB_iff o).mp h)

/-- `satisfied` exactly when there are obligations and each is a passing
check its policy takes, or when the first obligation its policy does not
meet is a passing check (a policy whose `accept` lacks `satisfied`, which
`accept`'s reader never builds: a class's own word is always taken). An
attestation or a waiver met under its policy is never read as a pass. -/
theorem standing_satisfied_iff (os : Slice delivery.Owed) :
    delivery.standing os ⦃ s => s = .Checked .Satisfied ↔
      (os.val ≠ [] ∧ ∀ o ∈ os.val, o.standing = .Checked .Satisfied ∧ o.accept.satisfied = true) ∨
      (∃ i, ∃ h : i < os.val.length, Plan.leastFailing ometB os.val = some i ∧
        os.val[i].standing = .Checked .Satisfied) ⦄ := by
  apply WP.spec_mono (standing_spec os)
  intro s hs
  rcases hs with ⟨hnil, rfl⟩ | ⟨i, h, hf, hpre, rfl⟩ | ⟨hne, hall, rfl⟩
  · simp [hnil]
  · have hlf : Plan.leastFailing ometB os.val = some i :=
      Plan.leastFailing_of_prefix _ _ _ h hf hpre
    constructor
    · intro hs; exact Or.inr ⟨i, h, hlf, hs⟩
    · rintro (⟨_, hall⟩ | ⟨k, hk, hlk, hks⟩)
      · obtain ⟨h1, h2⟩ := hall _ (List.getElem_mem h)
        simp [ometB, Plan.pmetB, h1, h2] at hf
      · rw [hlf] at hlk
        cases hlk
        exact hks
  · have hnone : Plan.leastFailing ometB os.val = none := by
      rw [Plan.leastFailing_none_iff]; exact hall
    unfold allMet
    constructor
    · intro hsat
      refine Or.inl ⟨hne, fun o ho => ?_⟩
      have hm := ometB_met (hall o ho)
      have hp := hall o ho
      split at hsat
      · cases hsat
      · split at hsat
        · cases hsat
        · rename_i hw ha
          rcases hm with h1 | h1 | h1
          · refine ⟨h1, ?_⟩
            simpa [ometB, Plan.pmetB, h1] using hp
          · exact absurd (List.any_eq_true.mpr ⟨o, ho, by simp [isA, h1]⟩) ha
          · exact absurd (List.any_eq_true.mpr ⟨o, ho, by simp [isW, h1]⟩) hw
    · rintro (⟨_, hsat⟩ | ⟨k, hk, hlk, _⟩)
      · have hnw : os.val.any (fun o => isW o.standing) = false := by
          rw [List.any_eq_false]
          intro o ho; rw [(hsat o ho).1]; simp [isW]
        have hna : os.val.any (fun o => isA o.standing) = false := by
          rw [List.any_eq_false]
          intro o ho; rw [(hsat o ho).1]; simp [isA]
        simp [hnw, hna]
      · rw [hnone] at hlk; cases hlk

/-! ## `ready` -/

/-- A candidate is ready exactly when it owes at least one obligation and
every one is met under its policy. -/
theorem ready_iff (os : Slice delivery.Owed) :
    delivery.ready os ⦃ b => b = true ↔
      os.val ≠ [] ∧ ∀ o ∈ os.val, Plan.PMet o.standing o.accept ⦄ := by
  unfold delivery.ready
  step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
  split
  · rename_i hempty
    have hnil : os.val = [] := by
      have : os.length = 0 := by simpa [hempty] using he.symm
      simpa [Slice.length] using this
    simp [hnil]
  · rename_i hne
    have hnil : os.val ≠ [] := by
      intro h0; apply hne; rw [he]; simp [Slice.length, h0]
    step with first_unmet_spec as ⟨ r, hr ⟩
    have hall : (∀ o ∈ os.val, Plan.PMet o.standing o.accept) ↔
        Plan.leastFailing ometB os.val = none := by
      rw [Plan.leastFailing_none_iff]
      exact forall_congr' fun o => forall_congr' fun _ => (ometB_iff o).symm
    rcases r with _ | i
    · simp only [WP.spec_ok, true_iff]
      refine ⟨hnil, hall.mpr ?_⟩
      simpa using hr.symm
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
      intro _ h
      rw [hall.mp h] at hr
      simp at hr

/-- A mode that declares nothing is never ready. -/
theorem empty_never_ready (os : Slice delivery.Owed) (h : os.val = []) :
    delivery.ready os ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (ready_iff os)
  intro b hb
  cases b
  · rfl
  · exact absurd h (hb.mp rfl).1

/-- A waiver its policy rejects never makes a candidate ready, whatever the
other obligations are. -/
theorem waiver_rejected_never_ready (os : Slice delivery.Owed)
    (h : ∃ o ∈ os.val, o.standing = .Waived ∧ o.accept.waived = false) :
    delivery.ready os ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (ready_iff os)
  intro b hb
  cases b
  · rfl
  · obtain ⟨o, ho, hw, ha⟩ := h
    have := (hb.mp rfl).2 o ho
    rcases this with ⟨h1, _⟩ | ⟨h1, _⟩ | ⟨_, h2⟩
    · rw [hw] at h1; cases h1
    · rw [hw] at h1; cases h1
    · rw [ha] at h2; cases h2

/-- Ready means met: a ready delivery's standing is a met word. The converse
does not hold (a waiver the policy rejects stands `waived`). -/
theorem standing_met_iff (os : Slice delivery.Owed) :
    (do let r ← delivery.ready os; let s ← delivery.standing os; ok (r, s))
      ⦃ p => p.1 = true → Gate.Met p.2 ⦄ := by
  step with ready_iff as ⟨ r, hr ⟩
  step with standing_spec as ⟨ s, hs ⟩
  intro hrt
  obtain ⟨hne, hall⟩ := hr.mp hrt
  have hallB : ∀ o ∈ os.val, ometB o = true := fun o ho => (ometB_iff o).mpr (hall o ho)
  rcases hs with ⟨hnil, _⟩ | ⟨i, h, hf, _, _⟩ | ⟨_, _, rfl⟩
  · exact absurd hnil hne
  · rw [hallB _ (List.getElem_mem h)] at hf; cases hf
  · unfold allMet Gate.Met
    split
    · simp
    · split <;> simp

/-- The rule before this round, on standings alone. -/
def allMetS (l : List gate.Standing) : gate.Standing :=
  if l.any isW then .Waived else if l.any isA then .Attested else .Checked .Satisfied

/-- `standing`'s decision before this round, on standings alone. -/
def BeforeSpec (l : List gate.Standing) (s : gate.Standing) : Prop :=
  (l = [] ∧ s = .Checked .Open) ∨
  (∃ i, ∃ h : i < l.length, Gate.metB l[i] = false ∧
      (∀ j (hj : j < i), Gate.metB (l[j]'(Nat.lt_trans hj h)) = true) ∧ s = l[i]) ∨
  (l ≠ [] ∧ (∀ o ∈ l, Gate.metB o = true) ∧ s = allMetS l)

theorem ometB_all (o : delivery.Owed) (h : o.accept = ⟨true, true, true⟩) :
    ometB o = Gate.metB o.standing := by
  rcases o with ⟨st, ac⟩
  simp only at h
  subst h
  rcases st with s | _ | _
  · cases s <;> rfl
  · rfl
  · rfl

/-- With every word taken (no `accept`, no `waivable`), readiness and the
standing are the rule before this round, on the standings alone: an absent
key changes nothing. -/
theorem default_accept_is_before (os : Slice delivery.Owed)
    (h : ∀ o ∈ os.val, o.accept = ⟨true, true, true⟩) :
    (do let r ← delivery.ready os; let s ← delivery.standing os; ok (r, s))
      ⦃ p => (p.1 = true ↔ os.val ≠ [] ∧ ∀ o ∈ os.val, Gate.Met o.standing) ∧
        BeforeSpec (os.val.map (·.standing)) p.2 ⦄ := by
  have hm : ∀ o ∈ os.val, ometB o = Gate.metB o.standing := fun o ho => ometB_all o (h o ho)
  step with ready_iff as ⟨ r, hr ⟩
  step with standing_spec as ⟨ s, hs ⟩
  refine ⟨?_, ?_⟩
  · rw [hr]
    apply and_congr Iff.rfl
    apply forall_congr'; intro o; apply forall_congr'; intro ho
    rw [← ometB_iff, hm o ho, Gate.metB_iff]
  · have hmi : ∀ i (hi : i < os.val.length), ometB os.val[i] = Gate.metB os.val[i].standing :=
      fun i hi => hm _ (List.getElem_mem hi)
    rcases hs with ⟨hnil, rfl⟩ | ⟨i, hi, hf, hpre, rfl⟩ | ⟨hne, hall, rfl⟩
    · left; simp [hnil]
    · right; left
      refine ⟨i, by simpa using hi, ?_, ?_, by simp⟩
      · simpa [hmi i hi] using hf
      · intro j hj
        have := hpre j hj
        simpa [hmi j (by omega)] using this
    · right; right
      refine ⟨by simpa using hne, ?_, ?_⟩
      · intro x hx
        obtain ⟨o, ho, rfl⟩ := List.mem_map.mp hx
        rw [← hm o ho]; exact hall o ho
      · simp [allMet, allMetS, List.any_map, Function.comp_def]

/-- A policy only narrows: a candidate ready under its policies is ready with
every word taken. -/
theorem accept_only_narrows (os os' : Slice delivery.Owed)
    (h : os'.val = os.val.map (fun o => ⟨o.standing, ⟨true, true, true⟩⟩)) :
    (do let r ← delivery.ready os; let r' ← delivery.ready os'; ok (r, r'))
      ⦃ p => p.1 = true → p.2 = true ⦄ := by
  step with ready_iff as ⟨ r, hr ⟩
  step with ready_iff as ⟨ r', hr' ⟩
  intro hrt
  obtain ⟨hne, hall⟩ := hr.mp hrt
  apply hr'.mpr
  refine ⟨by rw [h]; simpa using hne, ?_⟩
  intro o' ho'
  rw [h] at ho'
  obtain ⟨o, ho, rfl⟩ := List.mem_map.mp ho'
  rcases Plan.PMet.met (hall o ho) with h1 | h1 | h1 <;> simp [Plan.PMet, h1]

/-! ## `refused` and `fact` -/

/-- Held by a statement its policy rejects: the first obligation not met
under its policy stands on a met word. -/
theorem refused_iff (os : Slice delivery.Owed) :
    delivery.refused os ⦃ b => b = true ↔
      ∃ i, ∃ h : i < os.val.length, Plan.leastFailing ometB os.val = some i ∧
        Gate.Met os.val[i].standing ⦄ := by
  unfold delivery.refused
  step with first_unmet_spec as ⟨ r, hr ⟩
  rcases r with _ | i
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
    intro k hk hlk
    simp only [Option.map_none] at hr
    rw [← hr] at hlk; cases hlk
  · simp only [Option.map_some] at hr
    have hi : i.val < os.val.length := by
      obtain ⟨h, _⟩ := (Plan.leastFailing_some_iff _ _ _).mp hr.symm
      exact h
    step as ⟨ o, ho ⟩
    apply WP.spec_mono (Gate.met_spec o.standing)
    intro b hb
    rw [hb]
    constructor
    · intro hm
      exact ⟨i.val, hi, hr.symm, by rw [← ho]; exact hm⟩
    · rintro ⟨k, hk, hlk, hm⟩
      rw [← hr] at hlk
      cases hlk
      rw [ho]; exact hm

/-- The plan's proved `fact_met` of what a `deliver:<mode>` gate is given is
exactly readiness, so `Plan.done_only_when_met` covers a delivery. -/
theorem fact_met_iff_ready (os : Slice delivery.Owed) :
    (do let f ← delivery.fact os; let m ← plan.fact_met f; let r ← delivery.ready os; ok (m, r))
      ⦃ p => p.1 = p.2 ⦄ := by
  unfold delivery.fact
  step with ready_iff as ⟨ t, ht ⟩
  step with standing_spec as ⟨ s, hs ⟩
  try simp only [bind_tc_ok]
  step with Plan.fact_met_spec as ⟨ m, hm ⟩
  step with ready_iff as ⟨ r, hr ⟩
  have htr : t = r := Bool.eq_iff_iff.mpr (ht.trans hr.symm)
  subst htr
  rw [hm]
  simp only [Plan.factMetB]
  cases t
  · rcases s with x | _ | _
    · cases x <;> rfl
    · rfl
    · rfl
  · obtain ⟨hne, hall⟩ := ht.mp rfl
    have hallB : ∀ o ∈ os.val, ometB o = true := fun o ho => (ometB_iff o).mpr (hall o ho)
    rcases hs with ⟨hnil, _⟩ | ⟨i, h, hf, _, _⟩ | ⟨_, _, rfl⟩
    · exact absurd hnil hne
    · rw [hallB _ (List.getElem_mem h)] at hf; cases hf
    · unfold allMet
      split
      · rfl
      · split <;> rfl

/-! ## `posted` -/

theorem posted_spec (r : Bool) :
    delivery.posted r ⦃ s => s = if r then .Attested else .Checked .Open ⦄ := by
  unfold delivery.posted
  cases r <;> simp

/-- A post record is a statement: never a passing check. -/
theorem posted_never_satisfied (r : Bool) :
    delivery.posted r ⦃ s => s ≠ .Checked .Satisfied ⦄ := by
  unfold delivery.posted
  cases r <;> simp

theorem posted_met_iff (r : Bool) :
    delivery.posted r ⦃ s => Gate.Met s ↔ r = true ⦄ := by
  apply WP.spec_mono (posted_spec r)
  intro s hs
  cases r <;> simp_all [Gate.Met]

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

/-- The guard may pass a candidate's bytes. -/
def Postable (c : delivery.Seen) : Prop :=
  c.same = true ∧ c.ready = true ∧ c.live = true ∧ c.routed = true ∧ c.spent = false

def postableB (c : delivery.Seen) : Bool :=
  c.same && c.ready && c.live && c.routed && !c.spent

theorem postableB_iff (c : delivery.Seen) : postableB c = true ↔ Postable c := by
  rcases c with ⟨a, b, l, r, s⟩
  unfold postableB Postable
  cases a <;> cases b <;> cases l <;> cases r <;> cases s <;> simp

theorem postable_spec (c : delivery.Seen) :
    delivery.postable c ⦃ b => b = postableB c ⦄ := by
  unfold delivery.postable postableB
  rcases c with ⟨a, b, l, r, s⟩
  cases a <;> cases b <;> cases l <;> cases r <;> cases s <;> simp

theorem guard_any_loop_spec (cs : Slice delivery.Seen) (i : Usize) (hi : i.val ≤ cs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < cs.val.length), postableB cs.val[j] = false) :
    delivery.guard_any_loop cs i ⦃ b => b = true ↔ ∃ c ∈ cs.val, Postable c ⦄ := by
  unfold delivery.guard_any_loop
  apply loop.spec_decr_nat (fun i : Usize => cs.val.length - i.val)
    (fun i : Usize => i.val ≤ cs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < cs.val.length), postableB cs.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold delivery.guard_any_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with postable_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        exact ⟨x, by rw [hx]; exact List.getElem_mem _,
          (postableB_iff x).mp (by rw [← hb]; exact hbt)⟩
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx, ← hb]; simpa using hbt
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro c hc hp
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hc
      have := hprefix j (by scalar_tac) hj
      rw [(postableB_iff _).mpr hp] at this
      cases this
  · exact ⟨hi, hprefix⟩

/-- At least one candidate is live and every live one is ready. -/
def AllLiveReady (l : List delivery.Seen) : Prop :=
  (∃ c ∈ l, c.live = true) ∧ ∀ c ∈ l, c.live = true → c.ready = true

theorem guard_all_loop_spec (cs : Slice delivery.Seen) (live : Bool) (i : Usize)
    (hi : i.val ≤ cs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < cs.val.length),
      cs.val[j].live = true → cs.val[j].ready = true)
    (hlive : live = true ↔ ∃ j, ∃ hj : j < cs.val.length, j < i.val ∧ cs.val[j].live = true) :
    delivery.guard_all_loop cs live i ⦃ b => b = true ↔ AllLiveReady cs.val ⦄ := by
  unfold delivery.guard_all_loop
  apply loop.spec_decr_nat (fun p : Bool × Usize => cs.val.length - p.2.val)
    (fun p : Bool × Usize => p.2.val ≤ cs.val.length ∧
      (∀ j (hj : j < p.2.val) (hj' : j < cs.val.length),
        cs.val[j].live = true → cs.val[j].ready = true) ∧
      (p.1 = true ↔ ∃ j, ∃ hj : j < cs.val.length, j < p.2.val ∧ cs.val[j].live = true))
  · rintro ⟨live, i⟩ ⟨hi, hprefix, hlive⟩
    unfold delivery.guard_all_loop.body
    dsimp only at hi hprefix hlive ⊢
    split
    · rename_i hlt
      have hil : i.val < cs.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      split
      · rename_i hl
        split
        · rename_i hr
          try simp only [hl, ↓reduceIte, bind_tc_ok]
          step as ⟨ i', hi' ⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · intro j hj hj'
            by_cases hji : j < i.val
            · exact hprefix j hji hj'
            · have : j = i.val := by scalar_tac
              subst this; rw [← hx]; intro _; exact hr
          · simp only [true_iff]
            exact ⟨i.val, hil, by scalar_tac, by rw [← hx]; exact hl⟩
        · rename_i hr
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
          rintro ⟨_, hall⟩
          exact hr (hall x (by rw [hx]; exact List.getElem_mem _) hl)
      · rename_i hl
        try simp only [hl, Bool.false_eq_true, ↓reduceIte, bind_tc_ok]
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · intro j hj hj'
          by_cases hji : j < i.val
          · exact hprefix j hji hj'
          · have : j = i.val := by scalar_tac
            subst this; rw [← hx]; intro h; exact absurd h hl
        · rw [hlive]
          constructor
          · rintro ⟨j, hj, hji, hjl⟩; exact ⟨j, hj, by scalar_tac, hjl⟩
          · rintro ⟨j, hj, hji, hjl⟩
            refine ⟨j, hj, ?_, hjl⟩
            by_contra hc
            have : j = i.val := by scalar_tac
            subst this; rw [← hx] at hjl; exact hl hjl
    · rename_i hge
      simp only [WP.spec_ok]
      rw [hlive]
      constructor
      · rintro ⟨j, hj, _, hjl⟩
        refine ⟨⟨_, List.getElem_mem hj, hjl⟩, ?_⟩
        intro c hc hcl
        obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hc
        exact hprefix k (by scalar_tac) hk hcl
      · rintro ⟨⟨c, hc, hcl⟩, _⟩
        obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hc
        exact ⟨k, hk, by scalar_tac, hcl⟩
  · exact ⟨hi, hprefix, hlive⟩

theorem guard_any_spec (cs : Slice delivery.Seen) :
    delivery.guard_any cs ⦃ b => b = true ↔ ∃ c ∈ cs.val, Postable c ⦄ := by
  unfold delivery.guard_any
  apply guard_any_loop_spec
  · simp
  · intro j hj; simp at hj

theorem guard_all_spec (cs : Slice delivery.Seen) :
    delivery.guard_all cs ⦃ b => b = true ↔ AllLiveReady cs.val ⦄ := by
  unfold delivery.guard_all
  apply guard_all_loop_spec
  · simp
  · intro j hj; simp at hj
  · simp

/-- Known bytes pass exactly when some candidate is postable (same bytes,
ready, not withdrawn, of the route's mode, not yet posted). Unknown bytes
never pass under a route; under a listed command they pass when at least
one candidate is live and every live candidate is ready. -/
theorem guard_allow_iff (body : delivery.Body) (u : delivery.Unread) (cs : Slice delivery.Seen) :
    delivery.guard body u cs ⦃ b => b = true ↔
      (body = .Known ∧ ∃ c ∈ cs.val, Postable c) ∨
      (body = .Unknown ∧ u = .AllReady ∧ AllLiveReady cs.val) ⦄ := by
  unfold delivery.guard
  cases body
  · apply WP.spec_mono (guard_any_spec cs)
    intro b hb; simp [hb]
  · cases u
    · apply WP.spec_mono (guard_all_spec cs)
      intro b hb; simp [hb]
    · simp

/-- Under a route, bytes the guard cannot read never pass. -/
theorem guard_deny_unread (cs : Slice delivery.Seen) :
    delivery.guard .Unknown .Deny cs ⦃ b => b = false ⦄ := by
  unfold delivery.guard
  simp

/-- Withdrawn candidates are irrelevant: two runs whose live candidates are
the same get the same decision. -/
theorem guard_withdrawn_irrelevant (body : delivery.Body) (u : delivery.Unread)
    (cs ds : Slice delivery.Seen)
    (h : cs.val.filter (·.live) = ds.val.filter (·.live)) :
    (do let a ← delivery.guard body u cs; let b ← delivery.guard body u ds; ok (a, b))
      ⦃ p => p.1 = p.2 ⦄ := by
  have key : ∀ (P : delivery.Seen → Prop), (∀ c, P c → c.live = true) →
      ((∃ c ∈ cs.val, P c) ↔ ∃ c ∈ ds.val, P c) := by
    intro P hP
    have e : ∀ l : List delivery.Seen, (∃ c ∈ l, P c) ↔ ∃ c ∈ l.filter (·.live), P c := by
      intro l
      constructor
      · rintro ⟨c, hc, hp⟩; exact ⟨c, List.mem_filter.mpr ⟨hc, hP c hp⟩, hp⟩
      · rintro ⟨c, hc, hp⟩; exact ⟨c, (List.mem_filter.mp hc).1, hp⟩
    rw [e, e ds.val, h]
  have keyAll : AllLiveReady cs.val ↔ AllLiveReady ds.val := by
    unfold AllLiveReady
    apply and_congr (key _ (fun c hc => hc))
    have e : ∀ l : List delivery.Seen, (∀ c ∈ l, c.live = true → c.ready = true) ↔
        ∀ c ∈ l.filter (·.live), c.ready = true := by
      intro l
      constructor
      · intro hall c hc
        obtain ⟨hc, hl⟩ := List.mem_filter.mp hc
        exact hall c hc hl
      · intro hall c hc hl
        exact hall c (List.mem_filter.mpr ⟨hc, hl⟩)
    rw [e, e ds.val, h]
  step with guard_allow_iff as ⟨ a, ha ⟩
  step with guard_allow_iff as ⟨ b, hb ⟩
  have hp : ∀ c, Postable c → c.live = true := fun c hc => hc.2.2.1
  have hab : (a = true) ↔ (b = true) := by
    rw [ha, hb, key _ hp, keyAll]
  cases a <;> cases b <;> simp_all

/-- A posted candidate is spent: when every candidate holding these bytes
has a post record about its present bytes, known bytes never pass. -/
theorem guard_spent_never_passes (u : delivery.Unread) (cs : Slice delivery.Seen)
    (h : ∀ c ∈ cs.val, c.same = true → c.spent = true) :
    delivery.guard .Known u cs ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (guard_allow_iff .Known u cs)
  intro b hb
  cases b
  · rfl
  · exfalso
    rcases hb.mp rfl with ⟨_, c, hc, hp⟩ | ⟨h1, _⟩
    · have := h c hc hp.1
      rw [hp.2.2.2.2] at this; cases this
    · cases h1

end Writfold.Core.Delivery
