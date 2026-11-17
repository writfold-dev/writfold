import Writfold.Gate

/-!
# A team's policy

Theorems over the Aeneas translation of `writfold_core::policy`: which
standings meet a gate under its `accept` list, whether a run's policy moved
since it was pinned, how a step was closed and one word for how a plan ended.

- `met_spec`: met exactly when the standing is satisfied, attested or waived
  and the policy accepts that standing. `met_default`: a policy without
  `accept` (all three) is today's rule, `Standing::met`. `met_narrows`: a
  policy only ever narrows. `waiver_rejected_never_met`: a gate whose policy
  rejects waivers is never met by one, whatever statements exist.
- `pin_one_same_iff`, `pin_one_unpinned_iff`: a pinned digest is the same
  only when something is there now with the same bytes. `pin_all_moved_iff`,
  `pin_all_same_iff`: one moved pin moves the run; `same` needs something
  pinned and all of it unchanged.
- `close_passed_iff`, `close_waived_iff`, `close_open_iff`: a step reads
  `passed` only when done with every gate passed; an attestation or a waiver
  is never read as a pass.
- `summary_spec`: the plan's word, row by row in the table's order.
  `summary_complete_iff`, `moved_never_complete`, `open_never_complete`,
  `exception_never_clean`, `empty_never_complete`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

deriving instance DecidableEq for writfold_core.policy.Pin
deriving instance DecidableEq for writfold_core.policy.How
deriving instance DecidableEq for writfold_core.policy.Close
deriving instance DecidableEq for writfold_core.policy.Summary

namespace Writfold.Core.Policy

/-! ## `met` -/

/-- Met under a policy: a met standing the policy accepts. -/
theorem met_spec (x : gate.Standing) (a : policy.Accept) :
    policy.met x a ⦃ b => b = true ↔
      (x = .Checked .Satisfied ∧ a.satisfied = true) ∨
      (x = .Attested ∧ a.attested = true) ∨ (x = .Waived ∧ a.waived = true) ⦄ := by
  unfold policy.met
  rcases a with ⟨s, t, w⟩
  rcases x with st | _ | _
  · cases st <;> simp
  · simp
  · simp

/-- A policy without `accept` is today's rule. Proved from the code of
`policy::met` itself, not through `met_spec`. -/
theorem met_default (x : gate.Standing) :
    (do let p ← policy.met x ⟨true, true, true⟩; let g ← gate.Standing.met x; ok (p, g))
      ⦃ r => r.1 = r.2 ⦄ := by
  unfold policy.met gate.Standing.met
  rcases x with st | _ | _
  · cases st <;> simp
  · simp
  · simp

/-- A policy only ever narrows. -/
theorem met_narrows (x : gate.Standing) (a : policy.Accept) :
    (do let p ← policy.met x a; let g ← gate.Standing.met x; ok (p, g))
      ⦃ r => r.1 = true → r.2 = true ⦄ := by
  step with met_spec as ⟨ p, hp ⟩
  step with Gate.met_spec as ⟨ g, hg ⟩
  intro hpt
  rw [hg]
  rcases hp.mp hpt with ⟨h, _⟩ | ⟨h, _⟩ | ⟨h, _⟩ <;> simp [h]

/-- A gate that rejects waivers is never met by one, whatever statements
exist: met only by a current attestation of an attested-class obligation the
policy accepts, or a satisfied check the policy accepts. -/
theorem waiver_rejected_never_met (ac : Bool) (s : gate.State) (st : gate.Statements)
    (a : policy.Accept) (h : a.waived = false) :
    (do let x ← gate.standing ac s st; policy.met x a) ⦃ b => b = true →
      if ac then st.attested_current = true ∧ a.attested = true
      else s = .Satisfied ∧ a.satisfied = true ⦄ := by
  step with Gate.standing_spec as ⟨ x, hx ⟩
  apply WP.spec_mono (met_spec x a)
  intro b hb hbt
  have hm := hb.mp hbt
  subst hx
  rcases st with ⟨ac', aa, wc⟩
  unfold Gate.standingOf at hm
  cases ac <;> cases s <;> cases ac' <;> cases aa <;> cases wc <;> simp_all

/-! ## Pins -/

theorem pin_one_spec (p n : Option (alloc.vec.Vec U8)) :
    policy.pin_one p n ⦃ r => r = match p, n with
      | none, _ => .Unpinned
      | some _, none => .Moved
      | some a, some b => if a.val = b.val then .Same else .Moved ⦄ := by
  unfold policy.pin_one
  rcases p with _ | a
  · simp
  · rcases n with _ | b
    · simp
    · simp only
      step with same_spec as ⟨ r, hr ⟩
      have hr' : r = true ↔ a.val = b.val := by
        simpa [alloc.vec.Vec.deref, Slice.from_val] using hr
      cases r
      · have : a.val ≠ b.val := fun h => by simpa using hr'.mpr h
        simp [this]
      · simp [hr'.mp rfl]

/-- A pin is the same only when something was pinned and the same bytes are
there now. -/
theorem pin_one_same_iff (p n : Option (alloc.vec.Vec U8)) :
    policy.pin_one p n ⦃ r => r = .Same ↔ ∃ a b, p = some a ∧ n = some b ∧ a.val = b.val ⦄ := by
  -- From the code of `pin_one` itself, not through `pin_one_spec`.
  unfold policy.pin_one
  rcases p with _ | a
  · simp
  · rcases n with _ | b
    · simp
    · simp only
      step with same_spec as ⟨ r, hr ⟩
      have hr' : r = true ↔ a.val = b.val := by
        simpa [alloc.vec.Vec.deref, Slice.from_val] using hr
      cases r
      · have : a.val ≠ b.val := fun h => by simpa using hr'.mpr h
        simp [this]
      · simp [hr'.mp rfl]

/-- Unpinned exactly when nothing was pinned. -/
theorem pin_one_unpinned_iff (p n : Option (alloc.vec.Vec U8)) :
    policy.pin_one p n ⦃ r => r = .Unpinned ↔ p = none ⦄ := by
  apply WP.spec_mono (pin_one_spec p n)
  intro r hr
  subst hr
  rcases p with _ | a <;> rcases n with _ | b <;> simp
  all_goals split <;> simp

theorem is_moved_spec (x : policy.Pin) : policy.is_moved x ⦃ b => b = true ↔ x = .Moved ⦄ := by
  unfold policy.is_moved
  cases x <;> simp

theorem is_unpinned_spec (x : policy.Pin) : policy.is_unpinned x ⦃ b => b = true ↔ x = .Unpinned ⦄ := by
  unfold policy.is_unpinned
  cases x <;> simp

theorem any_moved_loop_spec (xs : Slice policy.Pin) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Moved) :
    policy.any_moved_loop xs i ⦃ b => b = true ↔ .Moved ∈ xs.val ⦄ := by
  unfold policy.any_moved_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Moved)
  · intro i ⟨hi, hprefix⟩
    unfold policy.any_moved_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with is_moved_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        rw [← hb.mp hbt, hx]; exact List.getElem_mem _
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro hm; exact hbt (hb.mpr hm)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj hjx
  · exact ⟨hi, hprefix⟩

theorem any_moved_spec (xs : Slice policy.Pin) :
    policy.any_moved xs ⦃ b => b = true ↔ .Moved ∈ xs.val ⦄ :=
  any_moved_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem any_unpinned_loop_spec (xs : Slice policy.Pin) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Unpinned) :
    policy.any_unpinned_loop xs i ⦃ b => b = true ↔ .Unpinned ∈ xs.val ⦄ := by
  unfold policy.any_unpinned_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Unpinned)
  · intro i ⟨hi, hprefix⟩
    unfold policy.any_unpinned_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with is_unpinned_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        rw [← hb.mp hbt, hx]; exact List.getElem_mem _
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro hm; exact hbt (hb.mpr hm)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj hjx
  · exact ⟨hi, hprefix⟩

theorem any_unpinned_spec (xs : Slice policy.Pin) :
    policy.any_unpinned xs ⦃ b => b = true ↔ .Unpinned ∈ xs.val ⦄ :=
  any_unpinned_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem pin_all_spec (ps : Slice policy.Pin) :
    policy.pin_all ps ⦃ r =>
      (r = .Moved ↔ .Moved ∈ ps.val) ∧
      (r = .Same ↔ ps.val ≠ [] ∧ ∀ p ∈ ps.val, p = .Same) ⦄ := by
  unfold policy.pin_all
  step with any_moved_spec as ⟨ m, hm ⟩
  split
  · rename_i hmt
    have hin := hm.mp hmt
    simp only [WP.spec_ok, true_iff, reduceCtorEq, false_iff, not_and, not_forall]
    refine ⟨hin, fun _ => ⟨_, hin, by simp⟩⟩
  · rename_i hmt
    have hno : .Moved ∉ ps.val := fun h => hmt (hm.mpr h)
    step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
    split
    · rename_i hempty
      have hnil : ps.val = [] := by
        have : ps.length = 0 := by simpa [hempty] using he.symm
        simpa [Slice.length] using this
      simp [hnil]
    · rename_i hne
      have hnil : ps.val ≠ [] := by
        intro h0; apply hne; rw [he]; simp [Slice.length, h0]
      step with any_unpinned_spec as ⟨ u, hu ⟩
      split
      · rename_i hut
        have hin := hu.mp hut
        simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and, not_forall]
        exact ⟨hno, fun _ => ⟨_, hin, by simp⟩⟩
      · rename_i hut
        have hnu : .Unpinned ∉ ps.val := fun h => hut (hu.mpr h)
        simp only [WP.spec_ok, reduceCtorEq, false_iff, true_iff]
        refine ⟨hno, hnil, fun p hp => ?_⟩
        cases p
        · exact absurd hp hnu
        · rfl
        · exact absurd hp hno

/-- One moved pin moves the run. -/
theorem pin_all_moved_iff (ps : Slice policy.Pin) :
    policy.pin_all ps ⦃ r => r = .Moved ↔ ∃ p ∈ ps.val, p = .Moved ⦄ := by
  apply WP.spec_mono (pin_all_spec ps)
  intro r hr
  simpa using hr.1

/-- Same only when something was pinned and all of it is unchanged: an empty
pin is not "same". -/
theorem pin_all_same_iff (ps : Slice policy.Pin) :
    policy.pin_all ps ⦃ r => r = .Same ↔ ps.val ≠ [] ∧ ∀ p ∈ ps.val, p = .Same ⦄ :=
  WP.spec_mono (pin_all_spec ps) (fun _ h => h.2)

/-! ## How a step was closed -/

theorem how_waived_spec (x : policy.How) : policy.how_waived x ⦃ b => b = true ↔ x = .Waived ⦄ := by
  unfold policy.how_waived
  cases x <;> simp

theorem how_attested_spec (x : policy.How) : policy.how_attested x ⦃ b => b = true ↔ x = .Attested ⦄ := by
  unfold policy.how_attested
  cases x <;> simp

theorem any_waived_loop_spec (xs : Slice policy.How) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Waived) :
    policy.any_waived_loop xs i ⦃ b => b = true ↔ .Waived ∈ xs.val ⦄ := by
  unfold policy.any_waived_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Waived)
  · intro i ⟨hi, hprefix⟩
    unfold policy.any_waived_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with how_waived_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        rw [← hb.mp hbt, hx]; exact List.getElem_mem _
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro hm; exact hbt (hb.mpr hm)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj hjx
  · exact ⟨hi, hprefix⟩

theorem any_waived_spec (xs : Slice policy.How) :
    policy.any_waived xs ⦃ b => b = true ↔ .Waived ∈ xs.val ⦄ :=
  any_waived_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem any_attested_loop_spec (xs : Slice policy.How) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Attested) :
    policy.any_attested_loop xs i ⦃ b => b = true ↔ .Attested ∈ xs.val ⦄ := by
  unfold policy.any_attested_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Attested)
  · intro i ⟨hi, hprefix⟩
    unfold policy.any_attested_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with how_attested_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        rw [← hb.mp hbt, hx]; exact List.getElem_mem _
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro hm; exact hbt (hb.mpr hm)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj hjx
  · exact ⟨hi, hprefix⟩

theorem any_attested_spec (xs : Slice policy.How) :
    policy.any_attested xs ⦃ b => b = true ↔ .Attested ∈ xs.val ⦄ :=
  any_attested_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

/-- `close` on the model: done wins; a done step is waived when any gate was
waived, else attested when any was attested, else passed. -/
def closeOf (done skipped : Bool) (met : List policy.How) : policy.Close :=
  if done then
    if .Waived ∈ met then .Waived
    else if .Attested ∈ met then .Attested
    else .Passed
  else if skipped then .Skipped else .Open

theorem close_spec (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = closeOf done skipped met.val ⦄ := by
  unfold policy.close closeOf
  cases done
  · cases skipped <;> simp
  · simp only [↓reduceIte]
    step with any_waived_spec as ⟨ w, hw ⟩
    split
    · rename_i hwt
      simp [hw.mp hwt]
    · rename_i hwt
      have hnw : .Waived ∉ met.val := fun h => hwt (hw.mpr h)
      step with any_attested_spec as ⟨ t, ht ⟩
      split
      · rename_i htt
        simp [hnw, ht.mp htt]
      · rename_i htt
        have hna : .Attested ∉ met.val := fun h => htt (ht.mpr h)
        simp [hnw, hna]

/-- A step reads `passed` only when it is done and every gate passed a check:
an attestation or a waiver is never read as a pass. -/
theorem close_passed_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Passed ↔ done = true ∧ ∀ h ∈ met.val, h = .Passed ⦄ := by
  apply WP.spec_mono (close_spec done skipped met)
  intro c hc
  subst hc
  unfold closeOf
  cases done
  · cases skipped <;> simp
  · simp only [↓reduceIte, true_and]
    by_cases hw : .Waived ∈ met.val
    · simp only [hw, ↓reduceIte, reduceCtorEq, false_iff, not_forall]
      exact ⟨_, hw, by simp⟩
    · by_cases ha : .Attested ∈ met.val
      · simp only [hw, ha, ↓reduceIte, reduceCtorEq, false_iff, not_forall]
        exact ⟨_, ha, by simp⟩
      · simp only [hw, ha, ↓reduceIte, true_iff]
        intro h hh
        cases h
        · rfl
        · exact absurd hh ha
        · exact absurd hh hw

theorem close_waived_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Waived ↔ done = true ∧ ∃ h ∈ met.val, h = .Waived ⦄ := by
  apply WP.spec_mono (close_spec done skipped met)
  intro c hc
  subst hc
  unfold closeOf
  cases done
  · cases skipped <;> simp
  · by_cases hw : .Waived ∈ met.val
    · simp [hw]
    · by_cases ha : .Attested ∈ met.val <;> simp [hw, ha]

theorem close_open_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Open ↔ done = false ∧ skipped = false ⦄ := by
  apply WP.spec_mono (close_spec done skipped met)
  intro c hc
  subst hc
  unfold closeOf
  cases done
  · cases skipped <;> simp
  · by_cases hw : .Waived ∈ met.val
    · simp [hw]
    · by_cases ha : .Attested ∈ met.val <;> simp [hw, ha]

/-! ## The plan's word -/

theorem is_open_spec (x : policy.Close) : policy.is_open x ⦃ b => b = true ↔ x = .Open ⦄ := by
  unfold policy.is_open
  cases x <;> simp

theorem is_passed_spec (x : policy.Close) : policy.is_passed x ⦃ b => b = true ↔ x = .Passed ⦄ := by
  unfold policy.is_passed
  cases x <;> simp

theorem any_open_loop_spec (xs : Slice policy.Close) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Open) :
    policy.any_open_loop xs i ⦃ b => b = true ↔ .Open ∈ xs.val ⦄ := by
  unfold policy.any_open_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] ≠ .Open)
  · intro i ⟨hi, hprefix⟩
    unfold policy.any_open_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with is_open_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        rw [← hb.mp hbt, hx]; exact List.getElem_mem _
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; intro hm; exact hbt (hb.mpr hm)
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro he
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj hjx
  · exact ⟨hi, hprefix⟩

theorem any_open_spec (xs : Slice policy.Close) :
    policy.any_open xs ⦃ b => b = true ↔ .Open ∈ xs.val ⦄ :=
  any_open_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

theorem all_passed_loop_spec (xs : Slice policy.Close) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = .Passed) :
    policy.all_passed_loop xs i ⦃ b => b = true ↔ ∀ x ∈ xs.val, x = .Passed ⦄ := by
  unfold policy.all_passed_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = .Passed)
  · intro i ⟨hi, hprefix⟩
    unfold policy.all_passed_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with is_passed_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; exact hb.mp hbt
      · rename_i hbt
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
        exact ⟨x, by rw [hx]; exact List.getElem_mem _, fun h => hbt (hb.mpr h)⟩
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro e he
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

theorem all_passed_spec (xs : Slice policy.Close) :
    policy.all_passed xs ⦃ b => b = true ↔ ∀ x ∈ xs.val, x = .Passed ⦄ :=
  all_passed_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

/-- The table of the plan's words, row by row, the first that applies:
`no-plan` with no step; `incomplete` or `closed-incomplete` with a step open
or a required template missing; `policy-moved`; `complete-stale`;
`complete-with-exceptions` for a step not passed or a repin; else
`complete`. -/
def SummarySpec (cs : List policy.Close) (f : policy.Facts) (r : policy.Summary) : Prop :=
  if cs = [] then r = .NoPlan
  else if (∃ c ∈ cs, c = .Open) ∨ f.required_present = false then
    r = (if f.run_closed then .ClosedIncomplete else .Incomplete)
  else if f.pin = .Moved then r = .PolicyMoved
  else if f.stale = true then r = .CompleteStale
  else if f.repinned = true ∨ ∃ c ∈ cs, c ≠ .Passed then r = .CompleteWithExceptions
  else r = .Complete

theorem summary_spec (cs : Slice policy.Close) (f : policy.Facts) :
    policy.summary cs f ⦃ r => SummarySpec cs.val f r ⦄ := by
  unfold policy.summary SummarySpec
  rcases f with ⟨rc, rp, pin, rep, st⟩
  step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
  split
  · rename_i hempty
    have hnil : cs.val = [] := by
      have : cs.length = 0 := by simpa [hempty] using he.symm
      simpa [Slice.length] using this
    simp [hnil]
  · rename_i hne
    have hnil : cs.val ≠ [] := by
      intro h0; apply hne; rw [he]; simp [Slice.length, h0]
    step with any_open_spec as ⟨ o, ho ⟩
    split
    · rename_i hot
      have hin : .Open ∈ cs.val := ho.mp hot
      cases rc <;> simp [hnil, hin]
    · rename_i hot
      have hno : .Open ∉ cs.val := fun h => hot (ho.mpr h)
      cases rp
      · cases rc <;> simp [hnil, hno]
      · simp only [↓reduceIte, if_true]
        step with is_moved_spec as ⟨ m, hm ⟩
        split
        · rename_i hmt
          simp [hnil, hno, hm.mp hmt]
        · rename_i hmt
          have hpin : pin ≠ .Moved := fun h => hmt (hm.mpr h)
          cases st
          · cases rep
            · simp only [Bool.false_eq_true, ↓reduceIte]
              step with all_passed_spec as ⟨ a, ha ⟩
              split
              · rename_i hat
                have hall := ha.mp hat
                have : ¬ ∃ c ∈ cs.val, c ≠ .Passed := fun ⟨c, hc, hcp⟩ => hcp (hall c hc)
                simp only [WP.spec_ok]
                simp [hnil, hno, hpin, this]
              · rename_i hat
                have : ∃ c ∈ cs.val, c ≠ .Passed := by
                  by_contra hall
                  exact hat (ha.mpr (fun c hc => by
                    by_contra hcp; exact hall ⟨c, hc, hcp⟩))
                simp only [WP.spec_ok]
                simp [hnil, hno, hpin, this]
            · simp [hnil, hno, hpin]
          · simp [hnil, hno, hpin]

/-- Complete exactly when there are steps, every one done on passing checks,
every required template is present, the policy did not move, the run was not
repinned and no done step went stale. -/
theorem summary_complete_iff (cs : Slice policy.Close) (f : policy.Facts) :
    policy.summary cs f ⦃ r => r = .Complete ↔
      cs.val ≠ [] ∧ (∀ c ∈ cs.val, c = .Passed) ∧ f.required_present = true ∧
      f.pin ≠ .Moved ∧ f.repinned = false ∧ f.stale = false ⦄ := by
  apply WP.spec_mono (summary_spec cs f)
  intro r hr
  unfold SummarySpec at hr
  rcases f with ⟨rc, rp, pin, rep, st⟩
  dsimp only at hr ⊢
  by_cases h0 : cs.val = []
  · rw [if_pos h0] at hr; subst hr; simp [h0]
  rw [if_neg h0] at hr
  by_cases h1 : (∃ c ∈ cs.val, c = .Open) ∨ rp = false
  · rw [if_pos h1] at hr; subst hr
    have hx : ¬ ((∀ c ∈ cs.val, c = .Passed) ∧ rp = true) := by
      rintro ⟨hall, hrp⟩
      rcases h1 with ⟨c, hc, hco⟩ | hrp'
      · have := hall c hc; rw [hco] at this; exact absurd this (by decide)
      · rw [hrp] at hrp'; exact absurd hrp' (by decide)
    constructor
    · intro h; cases rc <;> simp at h
    · rintro ⟨_, hall, hrp, _⟩; exact absurd ⟨hall, hrp⟩ hx
  rw [if_neg h1] at hr
  by_cases h2 : pin = .Moved
  · rw [if_pos h2] at hr; subst hr; simp [h2]
  rw [if_neg h2] at hr
  by_cases h3 : st = true
  · rw [if_pos h3] at hr; subst hr; simp [h3]
  rw [if_neg h3] at hr
  by_cases h4 : rep = true ∨ ∃ c ∈ cs.val, c ≠ .Passed
  · rw [if_pos h4] at hr; subst hr
    simp only [reduceCtorEq, false_iff]
    rintro ⟨_, hall, _, _, hrep, _⟩
    rcases h4 with h | ⟨c, hc, hcp⟩
    · rw [hrep] at h; exact absurd h (by decide)
    · exact hcp (hall c hc)
  rw [if_neg h4] at hr; subst hr
  simp only [true_iff]
  refine ⟨h0, fun c hc => ?_, ?_, h2, ?_, ?_⟩
  · by_contra hcp; exact h4 (Or.inr ⟨c, hc, hcp⟩)
  · cases rp
    · exact absurd (Or.inr rfl) h1
    · rfl
  · cases rep
    · rfl
    · exact absurd (Or.inl rfl) h4
  · cases st
    · rfl
    · exact absurd rfl h3

/-- A run whose policy differs from its pin never reads complete, with or
without exceptions. -/
theorem moved_never_complete (cs : Slice policy.Close) (f : policy.Facts) (h : f.pin = .Moved) :
    policy.summary cs f ⦃ r => r ≠ .Complete ∧ r ≠ .CompleteWithExceptions ∧ r ≠ .CompleteStale ⦄ := by
  apply WP.spec_mono (summary_spec cs f)
  intro r hr
  unfold SummarySpec at hr
  by_cases h0 : cs.val = []
  · rw [if_pos h0] at hr; subst hr; simp
  rw [if_neg h0] at hr
  by_cases h1 : (∃ c ∈ cs.val, c = .Open) ∨ f.required_present = false
  · rw [if_pos h1] at hr; subst hr; cases f.run_closed <;> simp
  rw [if_neg h1, if_pos h] at hr
  subst hr; simp

/-- An open step never reads as any kind of complete, whether or not the run
is closed. -/
theorem open_never_complete (cs : Slice policy.Close) (f : policy.Facts)
    (h : ∃ c ∈ cs.val, c = .Open) :
    policy.summary cs f ⦃ r => r = (if f.run_closed then .ClosedIncomplete else .Incomplete) ⦄ := by
  apply WP.spec_mono (summary_spec cs f)
  intro r hr
  unfold SummarySpec at hr
  have hne : cs.val ≠ [] := by
    obtain ⟨c, hc, _⟩ := h; intro h0; simp [h0] at hc
  simp only [hne, ↓reduceIte, h, true_or] at hr
  exact hr

/-- A skip, a waiver, an attestation or a repin never reads as plain
complete. -/
theorem exception_never_clean (cs : Slice policy.Close) (f : policy.Facts)
    (h : f.repinned = true ∨ ∃ c ∈ cs.val, c ≠ .Passed) :
    policy.summary cs f ⦃ r => r ≠ .Complete ⦄ := by
  apply WP.spec_mono (summary_complete_iff cs f)
  intro r hr hc
  obtain ⟨_, hall, _, _, hrep, _⟩ := hr.mp hc
  rcases h with h | ⟨c, hc, hcp⟩
  · simp_all
  · exact hcp (hall c hc)

/-- A run with no step has no plan to call complete. -/
theorem empty_never_complete (cs : Slice policy.Close) (h : cs.val = []) (f : policy.Facts) :
    policy.summary cs f ⦃ r => r = .NoPlan ⦄ := by
  apply WP.spec_mono (summary_spec cs f)
  intro r hr
  simpa [SummarySpec, h] using hr

end Writfold.Core.Policy
