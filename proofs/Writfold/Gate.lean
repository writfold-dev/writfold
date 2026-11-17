import Writfold.Bytes

/-!
# Gate obligations

Theorems over the Aeneas translation of `writfold_core::gate`'s decisions
(`outcome`, `state`, `standing`, `Standing::met`), the kernel behind
`writfold kb gate` and the met/owed answer `plan advance` sees.

- Only a clean exit with status 0 on an unmoved subject is a pass; any other
  exit status is a failure; every other ending is unknown (`outcome_spec`,
  `outcome_pass_iff`, `outcome_fail_iff`).
- A record is bound only when it is filed in this run, names this subject
  and is intact (`bound_spec`). `state` is decided by the newest receipt
  that is bound and about the current definition and the current content in
  scope; with none, by the newest bound receipt, and it is `stale` (or
  `open` with no bound receipt). It is `satisfied` exactly when that newest
  current receipt passed, so a receipt about another definition or other
  bytes never satisfies (`state_spec`, `state_satisfied_iff`,
  `state_other_subject`), a receipt that is not bound never satisfies
  (`state_unbound_never_satisfies`), and unbound receipts change nothing
  (`state_ignores_unbound`).
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

/-! ## `bound`, `current` and `newest_bound` -/

/-- A record is bound to the run and subject it is judged for only when it is
filed in this run, names this repository or candidate, and is intact. -/
theorem bound_spec (r s i : Bool) : gate.bound r s i ⦃ b => b = (r && s && i) ⦄ := by
  unfold gate.bound
  cases r <;> cases s <;> cases i <;> simp

/-- A receipt decides: bound, and about the current definition and the
present content of the paths in scope. -/
def isCurrent (e : gate.Evidence) : Bool := e.bound && e.same_definition && e.same_scope

theorem current_spec (e : gate.Evidence) : gate.current e ⦃ b => b = isCurrent e ⦄ := by
  unfold gate.current isCurrent
  rcases e with ⟨o, d, s, b⟩
  cases b <;> cases d <;> simp

theorem isCurrent_bound {e : gate.Evidence} (h : isCurrent e = true) : e.bound = true := by
  unfold isCurrent at h
  cases hb : e.bound <;> simp_all

/-- `newest_bound`'s answer: the greatest index of a bound receipt, or none
when no receipt is bound. -/
def NewestBound (rs : List gate.Evidence) : Option Usize → Prop
  | some i => ∃ h : i.val < rs.length, rs[i.val].bound = true ∧
      ∀ j (hj : j < rs.length), i.val < j → rs[j].bound = false
  | none => ∀ e ∈ rs, e.bound = false

theorem newest_bound_loop_spec (rs : Slice gate.Evidence) (i : Usize) (hi : i.val ≤ rs.val.length)
    (hnewer : ∀ j (hj : j < rs.val.length), i.val ≤ j → rs.val[j].bound = false) :
    gate.newest_bound_loop rs i ⦃ r => NewestBound rs.val r ⦄ := by
  unfold gate.newest_bound_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ rs.val.length ∧
      ∀ j (hj : j < rs.val.length), i.val ≤ j → rs.val[j].bound = false)
  · intro i ⟨hi, hnewer⟩
    unfold gate.newest_bound_loop.body
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ ev, hev ⟩
      split
      · rename_i hb
        simp only [WP.spec_ok, NewestBound]
        refine ⟨by scalar_tac, by rw [← hev]; exact hb, ?_⟩
        intro j hj hij
        exact hnewer j hj (by scalar_tac)
      · rename_i hb
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hij
        by_cases hji : i1.val = j
        · subst hji; rw [← hev]; simpa using hb
        · exact hnewer j hj (by scalar_tac)
    · rename_i hzero
      simp only [WP.spec_ok, NewestBound]
      intro e he
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
      exact hnewer j hj (by scalar_tac)
  · exact ⟨hi, hnewer⟩

theorem newest_bound_spec (rs : Slice gate.Evidence) :
    gate.newest_bound rs ⦃ r => NewestBound rs.val r ⦄ := by
  unfold gate.newest_bound
  apply newest_bound_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-! ## `state` -/

/-- The state a current receipt's outcome gives. -/
def stateOf : gate.Outcome → gate.State
  | .Pass => .Satisfied
  | .Fail => .Failed
  | .Unknown => .Unknown

/-- `state`'s decision: the deciding receipt is the newest current one (bound,
and about the current definition and content), with the state its outcome
gives; with no current receipt, the newest bound receipt decides and the
state is `stale`; with no bound receipt, `open`. A receipt that is not bound
counts as absent. -/
def StateSpec (rs : List gate.Evidence) : gate.State × Option Usize → Prop
  | (s, some i) =>
      (∃ h : i.val < rs.length, isCurrent rs[i.val] = true ∧
        (∀ j (hj : j < rs.length), i.val < j → isCurrent rs[j] = false) ∧
        s = stateOf rs[i.val].outcome) ∨
      ((∀ e ∈ rs, isCurrent e = false) ∧ NewestBound rs (some i) ∧ s = .Stale)
  | (s, none) => (∀ e ∈ rs, e.bound = false) ∧ s = .Open

theorem state_loop_spec (rs : Slice gate.Evidence) (i : Usize) (hi : i.val ≤ rs.val.length)
    (hnewer : ∀ j (hj : j < rs.val.length), i.val ≤ j → isCurrent rs.val[j] = false) :
    gate.state_loop rs i ⦃ r => StateSpec rs.val r ⦄ := by
  unfold gate.state_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ rs.val.length ∧
      ∀ j (hj : j < rs.val.length), i.val ≤ j → isCurrent rs.val[j] = false)
  · intro i ⟨hi, hnewer⟩
    unfold gate.state_loop.body
    split
    · rename_i hpos
      step as ⟨ i1, hi1 ⟩
      step as ⟨ ev, hev ⟩
      step with current_spec as ⟨ b, hb ⟩
      have hlt : i1.val < rs.val.length := by scalar_tac
      split
      · rename_i hbt
        have hcur : isCurrent rs.val[i1.val] = true := by rw [← hev, ← hb]; exact hbt
        have hlater : ∀ j (hj : j < rs.val.length), i1.val < j → isCurrent rs.val[j] = false :=
          fun j hj hij => hnewer j hj (by scalar_tac)
        cases ho : ev.outcome <;>
        · simp only [WP.spec_ok, StateSpec]
          left
          exact ⟨hlt, hcur, hlater, by rw [← hev, ho]; rfl⟩
      · rename_i hbt
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hij
        by_cases hji : i1.val = j
        · subst hji; rw [← hev, ← hb]; simpa using hbt
        · exact hnewer j hj (by scalar_tac)
    · rename_i hzero
      have hall : ∀ e ∈ rs.val, isCurrent e = false := by
        intro e he
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
        exact hnewer j hj (by scalar_tac)
      step with newest_bound_spec as ⟨ o, ho ⟩
      cases o with
      | none =>
        simp only [WP.spec_ok, StateSpec]
        exact ⟨ho, trivial⟩
      | some k =>
        simp only [WP.spec_ok, StateSpec]
        right
        exact ⟨hall, ho, trivial⟩
  · exact ⟨hi, hnewer⟩

theorem state_spec (rs : Slice gate.Evidence) : gate.state rs ⦃ r => StateSpec rs.val r ⦄ := by
  unfold gate.state
  apply state_loop_spec
  · simp [Slice.len]
  · intro j hj hle
    simp [Slice.len] at hle
    omega

/-- `state` is `satisfied` exactly when the newest receipt that is bound and
about the current definition and the current content in scope passed. -/
theorem state_satisfied_iff (rs : Slice gate.Evidence) :
    gate.state rs ⦃ r => r.1 = .Satisfied ↔
      ∃ i, ∃ h : i < rs.val.length, isCurrent rs.val[i] = true ∧
        (∀ j (hj : j < rs.val.length), i < j → isCurrent rs.val[j] = false) ∧
        rs.val[i].outcome = .Pass ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, (_ | i)⟩ hr
  · obtain ⟨hnone, rfl⟩ := hr
    simp only [reduceCtorEq, false_iff, not_exists, not_and]
    intro k hk hkcur
    have := hnone _ (List.getElem_mem hk)
    simp [isCurrent_bound hkcur] at this
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
receipt is current, the state is `stale` when some receipt is bound, else
`open`. -/
theorem state_other_subject (rs : Slice gate.Evidence)
    (hall : ∀ e ∈ rs.val, isCurrent e = false) :
    gate.state rs ⦃ r => r.1 = (if rs.val.any (·.bound) then .Stale else .Open) ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, (_ | i)⟩ hr
  · obtain ⟨hnone, rfl⟩ := hr
    have : rs.val.any (·.bound) = false := by
      simp only [List.any_eq_false]
      intro e he; simp [hnone e he]
    simp [this]
  · rcases hr with ⟨h, hcur, _, _⟩ | ⟨_, ⟨h, hb, _⟩, rfl⟩
    · have := hall _ (List.getElem_mem h); simp_all
    · have : rs.val.any (·.bound) = true := List.any_eq_true.mpr ⟨_, List.getElem_mem h, hb⟩
      simp [this]

/-- Receipts from another run, about another repository or candidate, or with
a broken digest never satisfy, whatever they say: with none bound, `open`. -/
theorem state_unbound_never_satisfies (rs : Slice gate.Evidence)
    (h : ∀ e ∈ rs.val, e.bound = false) :
    gate.state rs ⦃ r => r.1 = .Open ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, (_ | i)⟩ hr
  · exact hr.2
  · rcases hr with ⟨hi, hcur, _, _⟩ | ⟨_, ⟨hi, hb, _⟩, _⟩
    · have := h _ (List.getElem_mem hi); simp [isCurrent_bound hcur] at this
    · have := h _ (List.getElem_mem hi); simp_all

/-- The state of a list of receipts, on the model: the outcome of the newest
current receipt; else `stale` when some receipt is bound; else `open`. -/
def stateL (l : List gate.Evidence) : gate.State :=
  match l.reverse.find? isCurrent with
  | some e => stateOf e.outcome
  | none => if l.any (·.bound) then .Stale else .Open

theorem find_reverse_last {α : Type} (p : α → Bool) (l : List α) (i : Nat) (h : i < l.length)
    (hp : p l[i] = true) (hlater : ∀ j (hj : j < l.length), i < j → p l[j] = false) :
    l.reverse.find? p = some l[i] := by
  induction l using List.reverseRecOn generalizing i with
  | nil => simp at h
  | append_singleton l x ih =>
    simp only [List.reverse_append, List.reverse_cons, List.reverse_nil, List.nil_append,
      List.singleton_append, List.find?_cons]
    have hlen : (l ++ [x]).length = l.length + 1 := by simp
    by_cases hix : i = l.length
    · subst hix
      have : (l ++ [x])[l.length]'h = x := by simp
      rw [this] at hp ⊢
      simp [hp]
    · have hil : i < l.length := by omega
      have hx : p x = false := by
        have := hlater l.length (by simp) (by omega)
        simpa using this
      simp only [hx]
      have heq : (l ++ [x])[i]'h = l[i]'hil := List.getElem_append_left hil
      rw [heq]
      apply ih i hil
      · rw [← heq]; exact hp
      · intro j hj hij
        have := hlater j (by simp; omega) hij
        rwa [List.getElem_append_left hj] at this

theorem stateSpec_stateL (l : List gate.Evidence) (s : gate.State) (o : Option Usize)
    (h : StateSpec l (s, o)) : s = stateL l := by
  unfold stateL
  cases o with
  | none =>
    obtain ⟨hnone, rfl⟩ := h
    have hf : l.reverse.find? isCurrent = none := by
      rw [List.find?_eq_none]
      intro e he hc
      have := hnone e (List.mem_reverse.mp he)
      simp [isCurrent_bound hc] at this
    have ha : l.any (·.bound) = false := by
      simp only [List.any_eq_false]; intro e he; simp [hnone e he]
    simp [hf, ha]
  | some i =>
    rcases h with ⟨hi, hcur, hlater, rfl⟩ | ⟨hall, ⟨hi, hb, _⟩, rfl⟩
    · rw [find_reverse_last isCurrent l i.val hi hcur hlater]
    · have hf : l.reverse.find? isCurrent = none := by
        rw [List.find?_eq_none]
        intro e he
        simp [hall e (List.mem_reverse.mp he)]
      have ha : l.any (·.bound) = true := List.any_eq_true.mpr ⟨_, List.getElem_mem hi, hb⟩
      simp [hf, ha]

theorem stateL_filter_bound (l : List gate.Evidence) :
    stateL (l.filter (·.bound)) = stateL l := by
  unfold stateL
  have hf : (l.filter (·.bound)).reverse.find? isCurrent = l.reverse.find? isCurrent := by
    rw [← List.filter_reverse, List.find?_filter]
    congr 1
    funext e
    cases hc : isCurrent e
    · simp
    · simp [isCurrent_bound hc]
  have ha : (l.filter (·.bound)).any (·.bound) = l.any (·.bound) := by
    induction l with
    | nil => rfl
    | cons x l ih => cases hx : x.bound <;> simp_all
  rw [hf, ha]

/-- Unbound receipts change nothing: the state is the state of the bound
receipts alone. -/
theorem state_ignores_unbound (rs : Slice gate.Evidence) :
    gate.state rs ⦃ r => r.1 = stateL (rs.val.filter (·.bound)) ⦄ := by
  apply WP.spec_mono (state_spec rs)
  rintro ⟨s, o⟩ hr
  rw [stateL_filter_bound]
  exact stateSpec_stateL _ s o hr

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
