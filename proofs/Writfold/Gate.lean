import Writfold.Bytes

/-!
# Gate obligations

Theorems over the Aeneas translation of `writfold_core::gate`'s decisions
(`outcome`, `state`, `standing`, `Standing::met`), the kernel behind
`writfold kb gate` and the met/owed answer `plan advance` sees.

- Only a clean exit with status 0 on an unmoved subject is a pass; any other
  exit status is a failure; every other ending is unknown (`outcome_spec`,
  `outcome_pass_iff`, `outcome_fail_iff`).
- `state` is decided by the newest receipt about the current definition and
  the current content in scope; with none, it is `stale` (or `open` without
  receipts). It is `satisfied` exactly when that newest current receipt
  passed, so a receipt about another definition or other bytes never
  satisfies (`state_spec`, `state_satisfied_iff`, `state_other_subject`).
- A statement is never a passing check: `standing` is `Checked Satisfied`
  exactly when the obligation is not of the attested class and its checked
  state is `satisfied`; an attestation or a waiver stands as itself
  (`standing_spec`, `standing_satisfied_iff`, `standing_attested_iff`,
  `standing_waived_iff`). `met` is exactly satisfied, attested or waived
  (`met_spec`, `standing_met_iff`).
- Composed: whatever the statements, an obligation stands `Checked
  Satisfied` only when its newest receipt about the current subject passed
  (`satisfied_only_by_current_pass`).

Which paths a scope covers (`writfold_core::scope`, glob matching) is not
translated and not proved; it is tested.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Gate

/-! ## `outcome` -/

/-- The outcome a receipt records for an ending. -/
def outcomeOf (e : gate.Ending) (moved : Bool) : gate.Outcome :=
  if moved then .Unknown else
  match e with
  | .Exited i => if i.val = 0 then .Pass else .Fail
  | _ => .Unknown

theorem outcome_spec (e : gate.Ending) (moved : Bool) :
    gate.outcome e moved ⦃ o => o = outcomeOf e moved ⦄ := by
  unfold gate.outcome outcomeOf
  cases moved
  · cases e with
    | Exited i =>
      simp only [Bool.false_eq_true, ↓reduceIte]
      split
      · simp only [WP.spec_ok]
        rfl
      · rename_i h
        simp only [WP.spec_ok]
        have : i.val ≠ 0 := by
          intro h0
          apply h
          exact IScalar.eq_of_val_eq (by rw [h0]; rfl)
        simp [this]
    | _ => simp
  · simp

/-- Only a clean exit with status 0, on a subject that did not move while the
command ran, is a pass. -/
theorem outcome_pass_iff (e : gate.Ending) (moved : Bool) :
    gate.outcome e moved ⦃ o =>
      o = .Pass ↔ moved = false ∧ ∃ i : I32, e = .Exited i ∧ i.val = 0 ⦄ := by
  apply WP.spec_mono (outcome_spec e moved)
  intro o ho
  subst ho
  unfold outcomeOf
  cases moved <;> cases e <;> simp

/-- Any other exit status on an unmoved subject is a failure, and nothing
else is. -/
theorem outcome_fail_iff (e : gate.Ending) (moved : Bool) :
    gate.outcome e moved ⦃ o =>
      o = .Fail ↔ moved = false ∧ ∃ i : I32, e = .Exited i ∧ i.val ≠ 0 ⦄ := by
  apply WP.spec_mono (outcome_spec e moved)
  intro o ho
  subst ho
  unfold outcomeOf
  cases moved <;> cases e <;> simp

/-! ## `state` -/

/-- A receipt is about the current subject: the current definition and the
present content of the paths in scope. -/
def isCurrent (e : gate.Evidence) : Bool := e.same_definition && e.same_scope

/-- The state a current receipt's outcome gives. -/
def stateOf : gate.Outcome → gate.State
  | .Pass => .Satisfied
  | .Fail => .Failed
  | .Unknown => .Unknown

/-- `state`'s decision: the deciding receipt is the newest current one, with
the state its outcome gives; with no current receipt, the newest receipt
decides and the state is `stale`; with no receipt, `open`. -/
def StateSpec (rs : List gate.Evidence) : gate.State × Option Usize → Prop
  | (s, some i) =>
      (∃ h : i.val < rs.length, isCurrent rs[i.val] = true ∧
        (∀ j (hj : j < rs.length), i.val < j → isCurrent rs[j] = false) ∧
        s = stateOf rs[i.val].outcome) ∨
      ((∀ e ∈ rs, isCurrent e = false) ∧ i.val + 1 = rs.length ∧ s = .Stale)
  | (s, none) => rs = [] ∧ s = .Open

theorem state_loop_spec (rs : Slice gate.Evidence) (i : Usize) (hi : i.val ≤ rs.val.length)
    (hnewer : ∀ j (hj : j < rs.val.length), i.val ≤ j → isCurrent rs.val[j] = false) :
    gate.state_loop rs i ⦃ r => StateSpec rs.val r ⦄ := by
  unfold gate.state_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ rs.val.length ∧
      ∀ j (hj : j < rs.val.length), i.val ≤ j → isCurrent rs.val[j] = false)
  · intro i ⟨hi, hnewer⟩
    unfold gate.state_loop.body
    dsimp only
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ ev, hev ⟩
      have hlt : i1.val < rs.val.length := by scalar_tac
      have hlater : ∀ j (hj : j < rs.val.length), i1.val < j → isCurrent rs.val[j] = false :=
        fun j hj hij => hnewer j hj (by scalar_tac)
      split
      · rename_i hd
        split
        · rename_i hs
          have hcur : isCurrent rs.val[i1.val] = true := by
            rw [← hev]; simp [isCurrent, hd, hs]
          cases ho : ev.outcome <;>
          · simp only [ho, WP.spec_ok, StateSpec]
            left
            exact ⟨hlt, hcur, hlater, by rw [← hev, ho]; rfl⟩
        · rename_i hs
          simp only [WP.spec_ok]
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro j hj hij
          by_cases hji : i1.val = j
          · subst hji; rw [← hev]; simp [isCurrent, hs]
          · exact hnewer j hj (by scalar_tac)
      · rename_i hd
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hij
        by_cases hji : i1.val = j
        · subst hji; rw [← hev]; simp [isCurrent, hd]
        · exact hnewer j hj (by scalar_tac)
    · rename_i hzero
      have h0 : i.val = 0 := by scalar_tac
      have hall : ∀ e ∈ rs.val, isCurrent e = false := by
        intro e he
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
        exact hnewer j hj (by omega)
      split
      · rename_i hn
        simp only [WP.spec_ok, StateSpec, and_true]
        simpa [Slice.len] using hn
      · rename_i hn
        step as ⟨ k, hk ⟩
        simp only [WP.spec_ok, StateSpec]
        right
        refine ⟨hall, ?_, by simp⟩
        have : rs.val.length ≠ 0 := by simpa [Slice.len] using hn
        scalar_tac
  · exact ⟨hi, hnewer⟩

theorem state_spec (rs : Slice gate.Evidence) : gate.state rs ⦃ r => StateSpec rs.val r ⦄ := by
  unfold gate.state
  apply state_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-- `state` is `satisfied` exactly when the newest receipt about the current
definition and the current content in scope passed. -/
theorem state_satisfied_iff (rs : Slice gate.Evidence) :
    gate.state rs ⦃ r => r.1 = .Satisfied ↔
      ∃ i, ∃ h : i < rs.val.length, isCurrent rs.val[i] = true ∧
        (∀ j (hj : j < rs.val.length), i < j → isCurrent rs.val[j] = false) ∧
        rs.val[i].outcome = .Pass ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, (_ | i)⟩ hr
  · obtain ⟨hnil, rfl⟩ := hr
    simp [hnil]
  · rcases hr with ⟨h, hcur, hlater, rfl⟩ | ⟨hall, _, rfl⟩
    · constructor
      · intro hs
        refine ⟨i.val, h, hcur, hlater, ?_⟩
        cases ho : rs.val[i.val].outcome <;> simp_all [stateOf]
      · rintro ⟨k, hk, hkcur, hklater, hkpass⟩
        have hik : i.val = k := by
          by_contra hne
          rcases Nat.lt_or_gt_of_ne hne with hlt | hlt
          · have := hlater k hk hlt; simp_all
          · have := hklater i.val h hlt; simp_all
        subst hik
        simp [stateOf, hkpass]
    · simp only [reduceCtorEq, false_iff, not_exists, not_and]
      intro k hk hkcur
      have := hall _ (List.getElem_mem hk)
      simp_all

/-- A receipt about another definition or other bytes never satisfies: when no
receipt is about the current subject, the state is `stale` (or `open` when
there are none). -/
theorem state_other_subject (rs : Slice gate.Evidence)
    (hall : ∀ e ∈ rs.val, isCurrent e = false) :
    gate.state rs ⦃ r => r.1 = (if rs.val = [] then .Open else .Stale) ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, (_ | i)⟩ hr
  · obtain ⟨hnil, rfl⟩ := hr
    simp [hnil]
  · rcases hr with ⟨h, hcur, _, _⟩ | ⟨_, hlen, rfl⟩
    · have := hall _ (List.getElem_mem h); simp_all
    · have : rs.val ≠ [] := by
        intro hnil; simp [hnil] at hlen
      simp [this]

/-! ## `standing` and `met` -/

/-- `standing`'s decision. An attested-class obligation has no check: it
stands on a current attestation, else a current waiver, else it is owed
(`stale` when some attestation exists, else `open`). Any other obligation
stands on its checked state when that is `satisfied`, else on a current
waiver, else on its checked state. -/
def standingOf (ac : Bool) (s : gate.State) (st : gate.Statements) : gate.Standing :=
  if ac then
    if st.attested_current then .Attested
    else if st.waived_current then .Waived
    else if st.attested_any then .Checked .Stale
    else .Checked .Open
  else
    match s with
    | .Satisfied => .Checked .Satisfied
    | _ => if st.waived_current then .Waived else .Checked s

theorem standing_spec (ac : Bool) (s : gate.State) (st : gate.Statements) :
    gate.standing ac s st ⦃ r => r = standingOf ac s st ⦄ := by
  unfold gate.standing standingOf
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-- A statement is never a passing check: the standing is `Checked
Satisfied` exactly when the obligation is checked (not attested-class) and
its checked state is `satisfied`, whatever attestations or waivers exist. -/
theorem standing_satisfied_iff (ac : Bool) (s : gate.State) (st : gate.Statements) :
    gate.standing ac s st ⦃ r => r = .Checked .Satisfied ↔ ac = false ∧ s = .Satisfied ⦄ := by
  apply WP.spec_mono (standing_spec ac s st)
  intro r hr
  subst hr
  unfold standingOf
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-- `Attested` only for an attested-class obligation with a current
attestation. -/
theorem standing_attested_iff (ac : Bool) (s : gate.State) (st : gate.Statements) :
    gate.standing ac s st ⦃ r => r = .Attested ↔ ac = true ∧ st.attested_current = true ⦄ := by
  apply WP.spec_mono (standing_spec ac s st)
  intro r hr
  subst hr
  unfold standingOf
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-- `Waived` exactly when a current waiver exists and the obligation is not
already satisfied by a check or attested. -/
theorem standing_waived_iff (ac : Bool) (s : gate.State) (st : gate.Statements) :
    gate.standing ac s st ⦃ r => r = .Waived ↔
      st.waived_current = true ∧
        (if ac then st.attested_current = false else s ≠ .Satisfied) ⦄ := by
  apply WP.spec_mono (standing_spec ac s st)
  intro r hr
  subst hr
  unfold standingOf
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-- `met` is exactly satisfied by a check, attested or waived. -/
theorem met_spec (x : gate.Standing) :
    gate.Standing.met x ⦃ b => b = true ↔
      (x = .Checked .Satisfied ∨ x = .Attested ∨ x = .Waived) ⦄ := by
  unfold gate.Standing.met
  rcases x with s | _ | _
  · cases s <;> simp
  · simp
  · simp

/-- Whether an obligation is met, from its inputs: an attested-class
obligation by a current attestation or a current waiver; any other by a
satisfied check or a current waiver. -/
theorem standing_met_iff (ac : Bool) (s : gate.State) (st : gate.Statements) :
    (do let x ← gate.standing ac s st; gate.Standing.met x) ⦃ b => b = true ↔
      if ac then st.attested_current = true ∨ st.waived_current = true
      else s = .Satisfied ∨ st.waived_current = true ⦄ := by
  step with standing_spec as ⟨ x, hx ⟩
  apply WP.spec_mono (met_spec x)
  intro b hb
  rw [hb, hx]
  unfold standingOf
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-! ## The composed claim -/

/-- Whatever attestations and waivers exist, an obligation derived from its
receipts stands `Checked Satisfied` only when it is a checked obligation and
the newest receipt about the current definition and the current content in
scope passed. -/
theorem satisfied_only_by_current_pass (rs : Slice gate.Evidence) (ac : Bool)
    (st : gate.Statements) :
    (do let r ← gate.state rs; gate.standing ac r.1 st) ⦃ x =>
      x = .Checked .Satisfied → ac = false ∧
        ∃ i, ∃ h : i < rs.val.length, isCurrent rs.val[i] = true ∧
          (∀ j (hj : j < rs.val.length), i < j → isCurrent rs.val[j] = false) ∧
          rs.val[i].outcome = .Pass ⦄ := by
  step with state_satisfied_iff as ⟨ r, hr ⟩
  apply WP.spec_mono (standing_satisfied_iff ac r.1 st)
  intro x hx hsat
  obtain ⟨hac, hs⟩ := hx.mp hsat
  exact ⟨hac, hr.mp hs⟩

end Writfold.Core.Gate
