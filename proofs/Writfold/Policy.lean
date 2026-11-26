import Extracted.WritfoldCore.Gate
import Extracted.WritfoldCore.Policy
import Extracted.WritfoldCore.Types
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
- `gate lines` (`writfold_core::policy::lines`): `all_skipped_spec` (a gate
  is listed `skipped` exactly when every step naming it was skipped),
  `owed_met_spec` (a skipped gate never counts), `lines_clean_iff` (exit 0
  exactly for `complete` or `complete-with-exceptions` with every other gate
  met and nothing invalid), `lines_flagged_never_clean`,
  `lines_agree_with_summary` (clean lines imply every step closed, every
  required template present, the policy unmoved and nothing stale).
- `plan repin` (`writfold_core::policy::repin`): `in_force_spec` (the pin
  entry in force is the newest that applies; none exactly when none
  applies, `in_force_none_iff`), `repin_check_spec` (the refusals in their
  fixed order), `repin_ok_iff` (written exactly for an open run whose
  readable pins moved, whose owed gates load and whose skipped steps stay
  skipped), `repin_unmoved_refused`, `repin_closed_refused`,
  `repin_keeps_skips`.
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

/-- An entry pinned with some bytes reads moved when the present has other
bytes or nothing loads: whatever the entry names, a gate, a delivery mode
or an agent's closure (#161). -/
theorem pinned_change_is_moved (a : alloc.vec.Vec U8) (n : Option (alloc.vec.Vec U8))
    (h : ∀ b, n = some b → a.val ≠ b.val) :
    policy.pin_one (some a) n ⦃ r => r = .Moved ⦄ := by
  apply WP.spec_mono (pin_one_spec (some a) n)
  intro r hr
  subst hr
  rcases n with _ | b
  · rfl
  · simp [h b rfl]

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

/-! ## `gate lines` agrees with the plan's word -/

theorem is_skipped_spec (x : policy.Close) :
    policy.lines.is_skipped x ⦃ b => b = true ↔ x = .Skipped ⦄ := by
  unfold policy.lines.is_skipped; cases x <;> simp

theorem all_skipped_loop_spec (xs : Slice policy.Close) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = .Skipped) :
    policy.lines.all_skipped_loop xs i ⦃ b => b = true ↔ ∀ x ∈ xs.val, x = .Skipped ⦄ := by
  unfold policy.lines.all_skipped_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), xs.val[j] = .Skipped)
  · intro i ⟨hi, hprefix⟩
    unfold policy.lines.all_skipped_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with is_skipped_spec as ⟨ b, hb ⟩
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

/-- A gate is listed `skipped` exactly when some step names it and every
step naming it was skipped. -/
theorem all_skipped_spec (xs : Slice policy.Close) :
    policy.lines.all_skipped xs ⦃ b => b = true ↔ xs.val ≠ [] ∧ ∀ x ∈ xs.val, x = .Skipped ⦄ := by
  unfold policy.lines.all_skipped
  step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
  split
  · rename_i hempty
    have hnil : xs.val = [] := by
      have : xs.length = 0 := by simpa [hempty] using he.symm
      simpa [Slice.length] using this
    simp [hnil]
  · rename_i hne
    have hnil : xs.val ≠ [] := by
      intro h0; apply hne; rw [he]; simp [Slice.length, h0]
    apply WP.spec_mono (all_skipped_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj))
    intro b hb
    simp only [ne_eq, hnil, not_false_eq_true, true_and]
    exact hb

/-- The plan's word closes it exactly when it is `complete` or
`complete-with-exceptions`. -/
theorem closes_spec (s : policy.Summary) :
    policy.lines.closes s ⦃ b => b = true ↔ s = .Complete ∨ s = .CompleteWithExceptions ⦄ := by
  unfold policy.lines.closes; cases s <;> simp

/-- Owed: listed and not skipped. -/
def Owed (l : policy.lines.Listed) : Prop := l.skipped = false → l.met = true

theorem owed_met_loop_spec (xs : Slice policy.lines.Listed) (i : Usize)
    (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), Owed xs.val[j]) :
    policy.lines.owed_met_loop xs i ⦃ b => b = true ↔ ∀ l ∈ xs.val, Owed l ⦄ := by
  unfold policy.lines.owed_met_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), Owed xs.val[j])
  · intro i ⟨hi, hprefix⟩
    unfold policy.lines.owed_met_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have step_ok : ∀ (hok : Owed x), ∀ i' : Usize, i'.val = i.val + 1 →
          i'.val ≤ xs.val.length ∧
          (∀ j (hj : j < i'.val) (hj' : j < xs.val.length), Owed xs.val[j]) := by
        intro hok i' hi'
        refine ⟨by scalar_tac, ?_⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; exact hok
      split
      · rename_i hs
        step as ⟨ i', hi' ⟩
        exact ⟨(step_ok (fun h => by simp_all) i' hi').1,
               (step_ok (fun h => by simp_all) i' hi').2, by scalar_tac⟩
      · rename_i hs
        split
        · rename_i hm
          step as ⟨ i', hi' ⟩
          exact ⟨(step_ok (fun _ => hm) i' hi').1,
                 (step_ok (fun _ => hm) i' hi').2, by scalar_tac⟩
        · rename_i hm
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
          refine ⟨x, by rw [hx]; exact List.getElem_mem _, ?_⟩
          intro h; exact hm (h (by simpa using hs))
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro e he
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- Every listed gate that is not skipped is met; a skipped one never
counts. -/
theorem owed_met_spec (xs : Slice policy.lines.Listed) :
    policy.lines.owed_met xs ⦃ b => b = true ↔ ∀ l ∈ xs.val, Owed l ⦄ :=
  owed_met_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

/-- `gate lines` exits 0 for a planned run exactly when the plan's word is
`complete` or `complete-with-exceptions`, every gate not skipped is met
and nothing is invalid. -/
theorem lines_clean_iff (s : policy.Summary) (xs : Slice policy.lines.Listed) (v : Bool) :
    policy.lines.lines_clean s xs v ⦃ b => b = true ↔
      (s = .Complete ∨ s = .CompleteWithExceptions) ∧ (∀ l ∈ xs.val, Owed l) ∧ v = true ⦄ := by
  unfold policy.lines.lines_clean
  step with closes_spec as ⟨ c, hc ⟩
  split
  · rename_i hct
    step with owed_met_spec as ⟨ o, ho ⟩
    split
    · rename_i hot
      simp only [WP.spec_ok]
      exact ⟨fun h => ⟨hc.mp hct, ho.mp hot, h⟩, fun h => h.2.2⟩
    · rename_i hot
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
      exact fun _ h => absurd (ho.mpr h) hot
  · rename_i hct
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    exact fun h => absurd (hc.mpr h) hct

/-- A moved policy or a stale done step never gives clean lines. -/
theorem lines_flagged_never_clean (s : policy.Summary) (xs : Slice policy.lines.Listed) (v : Bool)
    (h : s = .PolicyMoved ∨ s = .CompleteStale) :
    policy.lines.lines_clean s xs v ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (lines_clean_iff s xs v)
  intro b hb
  cases b
  · rfl
  · rcases (hb.mp rfl).1 with h1 | h1 <;> rcases h with h2 | h2 <;> simp_all

/-- `gate lines` agrees with the plan's word: clean lines need a plan that
reads `complete` or `complete-with-exceptions` from `summary`, so every
step is closed, every required template is present, the policy did not
move and no done step went stale. -/
theorem lines_agree_with_summary (cs : Slice policy.Close) (f : policy.Facts)
    (xs : Slice policy.lines.Listed) (v : Bool) :
    (do let s ← policy.summary cs f; policy.lines.lines_clean s xs v) ⦃ b => b = true →
      cs.val ≠ [] ∧ .Open ∉ cs.val ∧ f.required_present = true ∧ f.pin ≠ .Moved ∧
        f.stale = false ⦄ := by
  apply WP.spec_bind (summary_spec cs f)
  intro s hs
  apply WP.spec_mono (lines_clean_iff s xs v)
  intro b hb hbt
  have hw := (hb.mp hbt).1
  unfold SummarySpec at hs
  rcases f with ⟨rc, rp, pin, rep, st⟩
  dsimp only at hs ⊢
  by_cases h0 : cs.val = []
  · rw [if_pos h0] at hs; subst hs; simp at hw
  rw [if_neg h0] at hs
  by_cases h1 : (∃ c ∈ cs.val, c = .Open) ∨ rp = false
  · rw [if_pos h1] at hs; subst hs; cases rc <;> simp at hw
  rw [if_neg h1] at hs
  by_cases h2 : pin = .Moved
  · rw [if_pos h2] at hs; subst hs; simp at hw
  rw [if_neg h2] at hs
  by_cases h3 : st = true
  · rw [if_pos h3] at hs; subst hs; simp at hw
  refine ⟨h0, fun ho => h1 (Or.inl ⟨_, ho, rfl⟩), ?_, h2, ?_⟩
  · cases rp
    · exact absurd (Or.inr rfl) h1
    · rfl
  · cases st
    · rfl
    · exact absurd rfl h3

/-! ## `plan repin`: the entry in force, and whether a repin may be written -/

/-- `in_force`'s answer: the greatest index of an entry that applies, or
none when no entry applies. -/
def InForce (xs : List Bool) : Option Usize → Prop
  | some i => ∃ h : i.val < xs.length, xs[i.val] = true ∧
      ∀ j (hj : j < xs.length), i.val < j → xs[j] = false
  | none => ∀ b ∈ xs, b = false

theorem in_force_loop_spec (xs : Slice Bool) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hnewer : ∀ j (hj : j < xs.val.length), i.val ≤ j → xs.val[j] = false) :
    policy.repin.in_force_loop xs i ⦃ r => InForce xs.val r ⦄ := by
  unfold policy.repin.in_force_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < xs.val.length), i.val ≤ j → xs.val[j] = false)
  · intro i ⟨hi, hnewer⟩
    unfold policy.repin.in_force_loop.body
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, InForce]
        refine ⟨by scalar_tac, by rw [← hb]; exact hbt, ?_⟩
        intro j hj hij
        exact hnewer j hj (by scalar_tac)
      · rename_i hbf
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hij
        by_cases hji : i1.val = j
        · subst hji; rw [← hb]; simpa using hbf
        · exact hnewer j hj (by scalar_tac)
    · rename_i hzero
      simp only [WP.spec_ok, InForce]
      intro e he
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
      exact hnewer j hj (by scalar_tac)
  · exact ⟨hi, hnewer⟩

/-- The pin entry in force is the newest that applies: it applies, and no
entry after it does; there is none exactly when no entry applies. -/
theorem in_force_spec (xs : Slice Bool) :
    policy.repin.in_force xs ⦃ r => InForce xs.val r ⦄ := by
  unfold policy.repin.in_force
  apply in_force_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-- Nothing is in force exactly when no pin entry applies: one entry naming
the gate (or the required file) is enough for a comparison to be made. -/
theorem in_force_none_iff (xs : Slice Bool) :
    policy.repin.in_force xs ⦃ r => r = none ↔ ∀ b ∈ xs.val, b = false ⦄ := by
  apply WP.spec_mono (in_force_spec xs)
  intro r hr
  cases r with
  | none => simpa [InForce] using hr
  | some i =>
    obtain ⟨h, ht, _⟩ := hr
    simp only [reduceCtorEq, false_iff, not_forall]
    exact ⟨xs.val[i.val], List.getElem_mem h, by simp [ht]⟩

/-- `repin_check`'s decision, refusal by refusal, in the fixed order: the
run is closed; a pin file does not read; nothing was pinned; nothing moved;
a gate an open step owes does not load; the lock list as it is now would
reopen a skipped step. -/
theorem repin_check_spec (w v : Bool) (p : policy.Pin) (l k : Bool) :
    policy.repin.repin_check w v p l k ⦃ r => match r with
      | .Ok _ => w = true ∧ v = true ∧ p = .Moved ∧ l = true ∧ k = true
      | .Err .Closed => w = false
      | .Err .Invalid => w = true ∧ v = false
      | .Err .Unpinned => w = true ∧ v = true ∧ p = .Unpinned
      | .Err .Same => w = true ∧ v = true ∧ p = .Same
      | .Err .Undefined => w = true ∧ v = true ∧ p = .Moved ∧ l = false
      | .Err .Reopens => w = true ∧ v = true ∧ p = .Moved ∧ l = true ∧ k = false ⦄ := by
  unfold policy.repin.repin_check
  cases w <;> cases v <;> cases p <;> cases l <;> cases k <;> simp

/-- A repin is written exactly when the run is open, every pin file reads,
the policy moved, every gate an open step owes loads now, and the lock list
as it is now locks no step the run skipped. -/
theorem repin_ok_iff (w v : Bool) (p : policy.Pin) (l k : Bool) :
    policy.repin.repin_check w v p l k ⦃ r =>
      r = .Ok () ↔ w = true ∧ v = true ∧ p = .Moved ∧ l = true ∧ k = true ⦄ := by
  unfold policy.repin.repin_check
  cases w <;> cases v <;> cases p <;> cases l <;> cases k <;> simp

/-- A run whose policy did not move (the same, or never pinned) never
writes a repin. -/
theorem repin_unmoved_refused (w v : Bool) (p : policy.Pin) (l k : Bool) (h : p ≠ .Moved) :
    policy.repin.repin_check w v p l k ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (repin_ok_iff w v p l k)
  intro r hr hok
  exact h (hr.mp hok).2.2.1

/-- A closed run never writes a repin. -/
theorem repin_closed_refused (v : Bool) (p : policy.Pin) (l k : Bool) :
    policy.repin.repin_check false v p l k ⦃ r => r = .Err .Closed ⦄ := by
  unfold policy.repin.repin_check
  simp

/-- A repin never reopens a step: it is refused when the lock list as it
is now locks a step the run already skipped. -/
theorem repin_keeps_skips (w v : Bool) (p : policy.Pin) (l : Bool) :
    policy.repin.repin_check w v p l false ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (repin_ok_iff w v p l false)
  intro r hr hok
  simpa using (hr.mp hok).2.2.2.2

end Writfold.Core.Policy
