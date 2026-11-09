import Writfold.Bytes

/-!
# Trial edit-flow decisions

Theorems over the Aeneas translation of `writfold_core::trial`: a prepared
candidate must change its source; an interrupted apply is reconciled from the
observed bytes alone (`Applied` exactly on the candidate's bytes, `Abandoned`
exactly on the source's, otherwise `Unknown`); and an approval's required
Weft checks are satisfied exactly when each is present and every check of that
name is `entailed`, the first unsatisfied requirement naming the refusal.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core

theorem changes_spec (source candidate : Slice U8) :
    trial.changes source candidate ⦃ b => b ↔ source.val ≠ candidate.val ⦄ := by
  unfold trial.changes
  step with same_spec as ⟨ b, hb ⟩
  try simp only [WP.spec_ok]
  cases b <;> simp_all

/-- The reconciliation decision, stated on byte strings. -/
def recoveryOutcome (observed source candidate : List U8) : trial.Outcome :=
  if observed = candidate then .Applied
  else if observed = source then .Abandoned
  else .Unknown

theorem recovery_spec (observed source candidate : Slice U8) :
    trial.recovery observed source candidate ⦃ o =>
      o = recoveryOutcome observed.val source.val candidate.val ⦄ := by
  unfold trial.recovery
  step with same_spec as ⟨ b, hb ⟩
  split
  · rename_i h; simp [recoveryOutcome, hb.mp h]
  · rename_i h
    step with same_spec as ⟨ b1, hb1 ⟩
    have hc : observed.val ≠ candidate.val := fun e => h (hb.mpr e)
    split
    · rename_i h1
      have ho := hb1.mp h1
      rw [ho] at hc
      simp [recoveryOutcome, ho, hc]
    · rename_i h1
      have hs : observed.val ≠ source.val := fun e => h1 (hb1.mpr e)
      simp [recoveryOutcome, hc, hs]

/-- Recovery is unambiguous when the candidate changes the source (which
`prepare` requires, `changes_spec`): the install happened exactly when the file
holds the candidate, and never happened exactly when it holds the source. -/
theorem recoveryOutcome_unambiguous (observed source candidate : List U8)
    (h : source ≠ candidate) :
    (recoveryOutcome observed source candidate = .Applied ↔ observed = candidate) ∧
    (recoveryOutcome observed source candidate = .Abandoned ↔ observed = source) := by
  unfold recoveryOutcome
  by_cases hc : observed = candidate
  · subst hc; simp [Ne.symm h]
  · by_cases hs : observed = source
    · subst hs; simp [hc, h]
    · simp [hc, hs]

/-- `fold_status` on naturals: 0 none seen, 1 all entailed so far, 2 final. -/
def foldN (s : Nat) (matched : Bool) (r : Nat) : Nat :=
  if !matched || s = 2 then s else if r = 0 then 1 else 2

theorem fold_status_spec (status : U8) (matched : Bool) (result : U8) :
    trial.fold_status status matched result ⦃ r =>
      r.val = foldN status.val matched result.val ⦄ := by
  unfold trial.fold_status
  simp only [trial.ENTAILED]
  cases matched <;> simp only [foldN] <;> split_ifs <;> simp_all

/-- The status `name_status` computes over a list of checks, from `s`. -/
def statusOf (nm : List U8) (l : List trial.Check) (s : Nat) : Nat :=
  l.foldl (fun s c => foldN s (decide (c.«name».val = nm)) c.result.val) s

/-- Some check has this nm. -/
def Present (checks : List trial.Check) (nm : List U8) : Prop :=
  ∃ c ∈ checks, c.«name».val = nm

/-- Every check with this nm is `entailed` (result 0). -/
def AllEntailed (checks : List trial.Check) (nm : List U8) : Prop :=
  ∀ c ∈ checks, c.«name».val = nm → c.result.val = 0

theorem foldN_le (s : Nat) (m : Bool) (r : Nat) (hs : s ≤ 2) : foldN s m r ≤ 2 := by
  unfold foldN; split_ifs <;> omega

theorem statusOf_le (nm : List U8) (l : List trial.Check) (s : Nat) (hs : s ≤ 2) :
    statusOf nm l s ≤ 2 := by
  induction l generalizing s with
  | nil => simpa [statusOf] using hs
  | cons c l ih =>
    simp only [statusOf, List.foldl_cons] at ih ⊢
    exact ih _ (foldN_le _ _ _ hs)

theorem foldN_two (s : Nat) (m : Bool) (r : Nat) (hs : s ≤ 2) :
    foldN s m r = 2 ↔ s = 2 ∨ (m = true ∧ r ≠ 0) := by
  unfold foldN
  cases m <;> by_cases h2 : s = 2 <;> by_cases hr : r = 0 <;> simp [h2, hr]

theorem foldN_zero (s : Nat) (m : Bool) (r : Nat) (hs : s ≤ 2) :
    foldN s m r = 0 ↔ s = 0 ∧ m = false := by
  unfold foldN
  cases m <;> by_cases h2 : s = 2 <;> by_cases hr : r = 0 <;> simp [h2, hr] <;> omega

theorem statusOf_two (nm : List U8) (l : List trial.Check) (s : Nat) (hs : s ≤ 2) :
    statusOf nm l s = 2 ↔ s = 2 ∨ ∃ c ∈ l, c.«name».val = nm ∧ c.result.val ≠ 0 := by
  induction l generalizing s with
  | nil => simp [statusOf]
  | cons c l ih =>
    simp only [statusOf, List.foldl_cons] at ih ⊢
    rw [ih _ (foldN_le _ _ _ hs), foldN_two _ _ _ hs]
    simp only [decide_eq_true_eq, List.mem_cons, or_and_right, exists_or, exists_eq_left]
    tauto

theorem statusOf_zero (nm : List U8) (l : List trial.Check) (s : Nat) (hs : s ≤ 2) :
    statusOf nm l s = 0 ↔ s = 0 ∧ ¬ ∃ c ∈ l, c.«name».val = nm := by
  induction l generalizing s with
  | nil => simp [statusOf]
  | cons c l ih =>
    simp only [statusOf, List.foldl_cons] at ih ⊢
    rw [ih _ (foldN_le _ _ _ hs), foldN_zero _ _ _ hs]
    simp only [decide_eq_false_iff_not, List.mem_cons, or_and_right, exists_or, exists_eq_left,
      not_or]
    tauto

/-- The three statuses, characterized. -/
theorem status_zero_iff (checks : List trial.Check) (nm : List U8) :
    statusOf nm checks 0 = 0 ↔ ¬ Present checks nm := by
  rw [statusOf_zero _ _ _ (by omega)]
  simp [Present]

theorem status_two_iff (checks : List trial.Check) (nm : List U8) :
    statusOf nm checks 0 = 2 ↔ ∃ c ∈ checks, c.«name».val = nm ∧ c.result.val ≠ 0 := by
  rw [statusOf_two _ _ _ (by omega)]
  simp

theorem status_one_iff (checks : List trial.Check) (nm : List U8) :
    statusOf nm checks 0 = 1 ↔ Present checks nm ∧ AllEntailed checks nm := by
  have hle := statusOf_le nm checks 0 (by omega)
  have h0 := status_zero_iff checks nm
  have h2 := status_two_iff checks nm
  unfold AllEntailed
  constructor
  · intro h1
    refine ⟨by by_contra hp; have := h0.mpr hp; omega, fun c hc hn => ?_⟩
    by_contra hr
    have := h2.mpr ⟨c, hc, hn, hr⟩
    omega
  · rintro ⟨hp, ha⟩
    have hn0 : statusOf nm checks 0 ≠ 0 := fun e => h0.mp e hp
    have hn2 : statusOf nm checks 0 ≠ 2 := by
      intro e
      obtain ⟨c, hc, hn, hr⟩ := h2.mp e
      exact hr (ha c hc hn)
    omega

theorem name_status_loop_spec (checks : Slice trial.Check) (nm : Slice U8)
    (status : U8) (i : Usize) (hi : i.val ≤ checks.val.length)
    (hs : status.val = statusOf nm.val (checks.val.take i.val) 0) :
    trial.name_status_loop checks nm status i ⦃ r =>
      r.val = statusOf nm.val checks.val 0 ⦄ := by
  unfold trial.name_status_loop
  apply loop.spec_decr_nat (fun p : U8 × Usize => checks.val.length - p.2.val)
    (fun p : U8 × Usize => p.2.val ≤ checks.val.length ∧
      p.1.val = statusOf nm.val (checks.val.take p.2.val) 0)
  · rintro ⟨st, i⟩ ⟨hi, hs⟩
    unfold trial.name_status_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ c, hc ⟩
      step with same_spec as ⟨ b, hb ⟩
      step with fold_status_spec as ⟨ st', hst ⟩
      step as ⟨ i', hi' ⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      have hlt' : i.val < checks.val.length := by simp [Slice.len] at hlt; scalar_tac
      rw [hi', List.take_add_one, List.getElem?_eq_getElem hlt', Option.toList_some]
      simp only [statusOf, List.foldl_append, List.foldl_cons, List.foldl_nil] at hs ⊢
      rw [hst, hs, ← hc]
      congr 1
      cases b <;> simp_all [alloc.vec.Vec.deref, Slice.from_val]
    · rename_i hge
      simp only [WP.spec_ok]
      have : checks.val.length ≤ i.val := by simp [Slice.len] at hge; scalar_tac
      rw [hs, List.take_of_length_le this]
  · exact ⟨hi, hs⟩

theorem name_status_spec (checks : Slice trial.Check) (nm : Slice U8) :
    trial.name_status checks nm ⦃ r => r.val = statusOf nm.val checks.val 0 ⦄ := by
  unfold trial.name_status
  apply name_status_loop_spec
  · simp
  · simp [statusOf]

/-- What `required_entailed` returns, requirement by requirement. -/
def RequiredOk (checks : Slice trial.Check) (required : Slice (alloc.vec.Vec U8))
    (r : core.result.Result Unit trial.Refusal) : Prop :=
  let St := fun (x : alloc.vec.Vec U8) => statusOf x.val checks.val 0
  match r with
  | .Ok () => ∀ x ∈ required.val, St x = 1
  | .Err (.Missing j) => ∃ h : j.val < required.val.length,
      (∀ k (hk : k < j.val), St (required.val[k]'(by omega)) = 1) ∧ St (required.val[j.val]) = 0
  | .Err (.NotEntailed j) => ∃ h : j.val < required.val.length,
      (∀ k (hk : k < j.val), St (required.val[k]'(by omega)) = 1) ∧ St (required.val[j.val]) = 2

theorem required_entailed_loop_spec (checks : Slice trial.Check)
    (required : Slice (alloc.vec.Vec U8)) (i : Usize) (hi : i.val ≤ required.val.length)
    (hsat : ∀ k (hk : k < i.val),
      statusOf (required.val[k]'(by omega)).val checks.val 0 = 1) :
    trial.required_entailed_loop checks required i ⦃ r => RequiredOk checks required r ⦄ := by
  unfold trial.required_entailed_loop
  apply loop.spec_decr_nat (fun i : Usize => required.val.length - i.val)
    (fun i : Usize => ∃ hi : i.val ≤ required.val.length, ∀ k (hk : k < i.val),
      statusOf (required.val[k]'(by omega)).val checks.val 0 = 1)
  · rintro i ⟨hi, hsat⟩
    unfold trial.required_entailed_loop.body
    dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < required.val.length := by simp [Slice.len] at hlt; scalar_tac
      step as ⟨ v, hv ⟩
      step with name_status_spec as ⟨ st, hst ⟩
      have hst' : st.val = statusOf (required.val[i.val]).val checks.val 0 := by
        rw [hst, hv]; simp [alloc.vec.Vec.deref, Slice.from_val]
      split
      · rename_i h0
        simp only [WP.spec_ok, RequiredOk]
        refine ⟨hlt', fun k hk => hsat k hk, ?_⟩
        rw [← hst', h0]; rfl
      · rename_i h0
        split
        · rename_i h2
          simp only [WP.spec_ok, RequiredOk]
          refine ⟨hlt', fun k hk => hsat k hk, ?_⟩
          rw [← hst', h2]; rfl
        · rename_i h2
          step as ⟨ i', hi' ⟩
          have hle := statusOf_le (required.val[i.val]).val checks.val 0 (by omega)
          have hn0 : st.val ≠ 0 := fun e => h0 (by scalar_tac)
          have hn2 : st.val ≠ 2 := fun e => h2 (by scalar_tac)
          refine ⟨⟨by scalar_tac, fun k hk => ?_⟩, by scalar_tac⟩
          by_cases hki : k < i.val
          · exact hsat k hki
          · have : k = i.val := by scalar_tac
            subst this
            omega
    · rename_i hge
      simp only [WP.spec_ok, RequiredOk]
      have hle : required.val.length ≤ i.val := by simp [Slice.len] at hge; scalar_tac
      intro x hx
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hx
      exact hsat k (by omega)
  · exact ⟨hi, hsat⟩

/-- An approval's required Weft checks: `Ok` exactly when every requirement is
present and every check of that name is `entailed`; otherwise the first
unsatisfied requirement is named, as missing or as not entailed. -/
theorem required_entailed_spec (checks : Slice trial.Check)
    (required : Slice (alloc.vec.Vec U8)) :
    trial.required_entailed checks required ⦃ r => RequiredOk checks required r ⦄ := by
  unfold trial.required_entailed
  apply required_entailed_loop_spec
  all_goals first | simp | (intro k hk; simp at hk)

/-- `Ok` means every requirement is satisfied in the calculus sense. -/
theorem required_ok_iff (checks : Slice trial.Check) (required : Slice (alloc.vec.Vec U8)) :
    RequiredOk checks required (.Ok ()) ↔
      ∀ x ∈ required.val, Present checks.val x.val ∧ AllEntailed checks.val x.val := by
  simp only [RequiredOk]
  exact forall₂_congr fun x _ => status_one_iff checks.val x.val

end Writfold.Core
