import Writfold.Current

/-!
# Guard rules

Theorems over the Aeneas translation of `writfold_core::guard`, the kernel
behind `writfold kb hook pre-tool-use` and `stop`: each requirement's fact
from what was looked up, one rule's decision, one call's decision from
every matched rule, the guard log's filter and the Stop nudge.

- `all_met_spec`, `decide_spec`, `decide_pass_iff`: a rule passes exactly
  when it did not match or every requirement is met;
  `unknown_never_passes`: a requirement that could not be judged never lets
  a matched rule pass.
- `combine_spec`, `combine_deny_iff`, `combine_pass_iff`, with the model
  lemmas `combineOf_comm`, `combineOf_assoc`, `combineOf_pass_left`: a
  denial wins over a question, which wins over a pass, in any order.
- `effect_spec`, `effect_audit_passes`, `effect_enforce`: an audit rule never
  decides the call; an enforcing rule's decision stands.
- `decide_all_pass_iff`, `decide_all_deny_iff`: a call passes exactly when
  every enforcing rule's requirements are met, and is denied exactly when
  some enforcing deny rule's are not. `logged_iff`.
- `gates_fact_met_iff`, `gates_fact_never_unknown`: `gates.met` is met only
  when something is owed, all of it is met and nothing is invalid.
- `run_current_met_iff`, `step_fact_met_iff`, `step_fact_unknown_iff`,
  `gate_fact_met_iff`, `gate_fact_unknown_iff`, `gates_read_met_iff`: each
  requirement's fact from what was found. `step.met` holds for a run with
  no plan or no current step; `gates.met` does not hold for a run that owes
  no gate.
- `nudge_spec`, `nudge_with_iff`: the Stop nudge fires only when some
  requirement is not met, never while a stop hook is already continuing,
  and at most once a session for a `session` rule.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Guard

/-! ## `all_met`, `verdict`, `decide` -/

/-- A fact is met. -/
def metF : guard.Fact → Bool
  | .Met => true
  | _ => false

theorem met_spec (f : guard.Fact) : guard.met f ⦃ b => b = metF f ⦄ := by
  unfold guard.met
  cases f <;> simp [metF]

theorem all_met_loop_spec (fs : Slice guard.Fact) (i : Usize) (hi : i.val ≤ fs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < fs.val.length), fs.val[j] = .Met) :
    guard.all_met_loop fs i ⦃ b => b = true ↔ ∀ f ∈ fs.val, f = .Met ⦄ := by
  unfold guard.all_met_loop
  apply loop.spec_decr_nat (fun i : Usize => fs.val.length - i.val)
    (fun i : Usize => i.val ≤ fs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < fs.val.length), fs.val[j] = .Met)
  · intro i ⟨hi, hprefix⟩
    unfold guard.all_met_loop.body
    dsimp only
    split
    · rename_i hlt
      have hil : i.val < fs.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      step with met_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]
          cases x <;> simp_all [metF]
      · rename_i hbt
        have hxm : x ≠ .Met := by
          intro h; subst h; simp_all [metF]
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
        exact ⟨_, List.getElem_mem hil, by rw [← hx]; exact hxm⟩
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro f hf
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hf
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- `all_met` is whether every fact is met (true for none). -/
theorem all_met_spec (fs : Slice guard.Fact) :
    guard.all_met fs ⦃ b => b = true ↔ ∀ f ∈ fs.val, f = .Met ⦄ := by
  unfold guard.all_met
  apply all_met_loop_spec
  · simp
  · intro j hj; simp at hj

/-- The action's decision. -/
def actionOf : guard.Action → guard.Decision
  | .Deny => .Deny
  | .Ask => .Ask

theorem verdict_spec (m : Bool) (a : guard.Action) :
    guard.verdict m a ⦃ d => d = if m then .Pass else actionOf a ⦄ := by
  unfold guard.verdict
  cases m <;> cases a <;> simp [actionOf]

/-- A rule passes when it did not match or every requirement is met;
otherwise it takes its action. -/
theorem decide_spec (m : Bool) (a : guard.Action) (fs : Slice guard.Fact) :
    guard.decide m a fs ⦃ d =>
      d = if !m || fs.val.all metF then .Pass else actionOf a ⦄ := by
  unfold guard.decide
  cases m
  · simp
  · simp only [↓reduceIte, Bool.not_true, Bool.false_or]
    step with all_met_spec as ⟨ b, hb ⟩
    apply WP.spec_mono (verdict_spec b a)
    intro d hd
    rw [hd]
    have : b = fs.val.all metF := by
      cases b <;> cases hall : fs.val.all metF <;> simp_all [List.all_eq_true, metF] <;>
        first
        | (obtain ⟨f, hf, hne⟩ := hb; have := hall f hf; cases f <;> simp_all [metF])
        | (obtain ⟨f, hf, hne⟩ := hall; have := hb f hf; cases f <;> simp_all [metF])
    rw [this]

theorem allMetF_iff (fs : List guard.Fact) : fs.all metF = true ↔ ∀ f ∈ fs, f = .Met := by
  rw [List.all_eq_true]
  apply forall_congr'; intro f
  apply forall_congr'; intro _
  cases f <;> simp [metF]

theorem decide_pass_iff (m : Bool) (a : guard.Action) (fs : Slice guard.Fact) :
    guard.decide m a fs ⦃ d => d = .Pass ↔ m = false ∨ ∀ f ∈ fs.val, f = .Met ⦄ := by
  apply WP.spec_mono (decide_spec m a fs)
  intro d hd
  rw [hd, ← allMetF_iff]
  cases m <;> cases fs.val.all metF <;> cases a <;> simp [actionOf]

/-- A requirement that could not be judged never lets a matched rule pass. -/
theorem unknown_never_passes (a : guard.Action) (fs : Slice guard.Fact)
    (h : guard.Fact.Unknown ∈ fs.val) :
    guard.decide true a fs ⦃ d => d ≠ .Pass ⦄ := by
  apply WP.spec_mono (decide_pass_iff true a fs)
  intro d hd hpass
  rcases hd.mp hpass with h1 | h1
  · cases h1
  · cases h1 _ h

/-! ## `combine` and `effect` -/

/-- The stricter of two decisions. -/
def combineOf : guard.Decision → guard.Decision → guard.Decision
  | .Deny, _ => .Deny
  | _, .Deny => .Deny
  | .Ask, _ => .Ask
  | _, .Ask => .Ask
  | .Pass, .Pass => .Pass

theorem combine_spec (a b : guard.Decision) :
    guard.combine a b ⦃ d => d = combineOf a b ⦄ := by
  unfold guard.combine
  cases a <;> cases b <;> simp [combineOf]

theorem combine_deny_iff (a b : guard.Decision) :
    guard.combine a b ⦃ d => d = .Deny ↔ a = .Deny ∨ b = .Deny ⦄ := by
  apply WP.spec_mono (combine_spec a b)
  intro d hd; subst hd
  cases a <;> cases b <;> simp [combineOf]

theorem combine_pass_iff (a b : guard.Decision) :
    guard.combine a b ⦃ d => d = .Pass ↔ a = .Pass ∧ b = .Pass ⦄ := by
  apply WP.spec_mono (combine_spec a b)
  intro d hd; subst hd
  cases a <;> cases b <;> simp [combineOf]

theorem combineOf_comm (a b : guard.Decision) : combineOf a b = combineOf b a := by
  cases a <;> cases b <;> rfl

theorem combineOf_assoc (a b c : guard.Decision) :
    combineOf (combineOf a b) c = combineOf a (combineOf b c) := by
  cases a <;> cases b <;> cases c <;> rfl

theorem combineOf_pass_left (a : guard.Decision) : combineOf .Pass a = a := by
  cases a <;> rfl

/-- What a rule's decision does to the call. -/
def effectOf : guard.Mode → guard.Decision → guard.Decision
  | .Enforce, d => d
  | .Audit, _ => .Pass

theorem effect_spec (m : guard.Mode) (d : guard.Decision) :
    guard.effect m d ⦃ r => r = effectOf m d ⦄ := by
  unfold guard.effect
  cases m <;> simp [effectOf]

/-- An audit rule never decides the call. -/
theorem effect_audit_passes (d : guard.Decision) : guard.effect .Audit d ⦃ r => r = .Pass ⦄ :=
  WP.spec_mono (effect_spec .Audit d) (fun r h => by simpa [effectOf] using h)

/-- An enforcing rule's decision stands. -/
theorem effect_enforce (d : guard.Decision) : guard.effect .Enforce d ⦃ r => r = d ⦄ :=
  WP.spec_mono (effect_spec .Enforce d) (fun r h => by simpa [effectOf] using h)

/-! ## One call: `ruled`, `decide_all`, `logged` -/

/-- One judged rule's effect on the call. -/
def ruledOf (r : guard.call.Ruled) : guard.Decision :=
  effectOf r.mode (if r.met then .Pass else actionOf r.action)

theorem ruled_spec (r : guard.call.Ruled) : guard.call.ruled r ⦃ d => d = ruledOf r ⦄ := by
  unfold guard.call.ruled ruledOf
  step with verdict_spec as ⟨ v, hv ⟩
  apply WP.spec_mono (effect_spec r.mode v)
  intro d hd
  rw [hd, hv]

/-- A rule holds the call back: it enforces and a requirement is unmet. -/
def holds (r : guard.call.Ruled) : Prop := r.mode = .Enforce ∧ r.met = false

theorem ruledOf_pass_iff (r : guard.call.Ruled) : ruledOf r = .Pass ↔ ¬ holds r := by
  rcases r with ⟨m, a, met⟩
  cases m <;> cases a <;> cases met <;> simp [ruledOf, effectOf, actionOf, holds]

theorem ruledOf_deny_iff (r : guard.call.Ruled) :
    ruledOf r = .Deny ↔ holds r ∧ r.action = .Deny := by
  rcases r with ⟨m, a, met⟩
  cases m <;> cases a <;> cases met <;> simp [ruledOf, effectOf, actionOf, holds]

theorem decide_all_loop_spec (rs : Slice guard.call.Ruled) (acc : guard.Decision) (i : Usize)
    (hi : i.val ≤ rs.val.length)
    (hpass : acc = .Pass ↔ ∀ j (hj : j < i.val) (hj' : j < rs.val.length), ¬ holds rs.val[j])
    (hdeny : acc = .Deny ↔ ∃ j, ∃ hj : j < rs.val.length, j < i.val ∧
      holds rs.val[j] ∧ rs.val[j].action = .Deny) :
    guard.call.decide_all_loop rs acc i ⦃ d =>
      (d = .Pass ↔ ∀ r ∈ rs.val, ¬ holds r) ∧
      (d = .Deny ↔ ∃ r ∈ rs.val, holds r ∧ r.action = .Deny) ⦄ := by
  unfold guard.call.decide_all_loop
  apply loop.spec_decr_nat (fun p : guard.Decision × Usize => rs.val.length - p.2.val)
    (fun p : guard.Decision × Usize => p.2.val ≤ rs.val.length ∧
      (p.1 = .Pass ↔ ∀ j (hj : j < p.2.val) (hj' : j < rs.val.length), ¬ holds rs.val[j]) ∧
      (p.1 = .Deny ↔ ∃ j, ∃ hj : j < rs.val.length, j < p.2.val ∧
        holds rs.val[j] ∧ rs.val[j].action = .Deny))
  · rintro ⟨acc, i⟩ ⟨hi, hpass, hdeny⟩
    unfold guard.call.decide_all_loop.body
    dsimp only at hi hpass hdeny ⊢
    split
    · rename_i hlt
      have hil : i.val < rs.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      step with ruled_spec as ⟨ e, he ⟩
      step with combine_spec as ⟨ c, hc ⟩
      step as ⟨ i', hi' ⟩
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
      · rw [hc, hi1]
        have hcp : combineOf acc e = .Pass ↔ acc = .Pass ∧ e = .Pass := by
          cases acc <;> cases e <;> simp [combineOf]
        rw [hcp, hpass, he, hx, ruledOf_pass_iff]
        constructor
        · rintro ⟨hall, hlast⟩ j hj hj'
          by_cases hji : j < i.val
          · exact hall j hji hj'
          · have : j = i.val := by omega
            subst this; exact hlast
        · intro hall
          exact ⟨fun j hj hj' => hall j (by omega) hj', hall i.val (by omega) hil⟩
      · rw [hc, hi1]
        have hcd : combineOf acc e = .Deny ↔ acc = .Deny ∨ e = .Deny := by
          cases acc <;> cases e <;> simp [combineOf]
        rw [hcd, hdeny, he, hx, ruledOf_deny_iff]
        constructor
        · rintro (⟨j, hj, hji, hh, ha⟩ | ⟨hh, ha⟩)
          · exact ⟨j, hj, by omega, hh, ha⟩
          · exact ⟨i.val, hil, by omega, hh, ha⟩
        · rintro ⟨j, hj, hji, hh, ha⟩
          by_cases hjeq : j = i.val
          · subst hjeq; exact Or.inr ⟨hh, ha⟩
          · exact Or.inl ⟨j, hj, by omega, hh, ha⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have hlen : i.val = rs.val.length := by scalar_tac
      constructor
      · rw [hpass]
        constructor
        · intro hall r hr
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hr
          exact hall j (by omega) hj
        · intro hall j _ hj'
          exact hall _ (List.getElem_mem hj')
      · rw [hdeny]
        constructor
        · rintro ⟨j, hj, _, hh, ha⟩
          exact ⟨_, List.getElem_mem hj, hh, ha⟩
        · rintro ⟨r, hr, hh, ha⟩
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hr
          exact ⟨j, hj, by omega, hh, ha⟩
  · exact ⟨hi, hpass, hdeny⟩

/-- A call passes exactly when no enforcing rule has an unmet requirement:
an audit rule never holds it back. -/
theorem decide_all_pass_iff (rs : Slice guard.call.Ruled) :
    guard.call.decide_all rs ⦃ d => d = .Pass ↔
      ∀ r ∈ rs.val, r.mode = .Audit ∨ r.met = true ⦄ := by
  unfold guard.call.decide_all
  apply WP.spec_mono (decide_all_loop_spec rs .Pass 0#usize (by simp)
    (by simp) (by simp))
  intro d hd
  rw [hd.1]
  apply forall_congr'; intro r
  apply forall_congr'; intro _
  rcases r with ⟨m, a, met⟩
  cases m <;> cases met <;> simp [holds]

/-- A call is denied exactly when some enforcing deny rule has an unmet
requirement. -/
theorem decide_all_deny_iff (rs : Slice guard.call.Ruled) :
    guard.call.decide_all rs ⦃ d => d = .Deny ↔
      ∃ r ∈ rs.val, r.mode = .Enforce ∧ r.action = .Deny ∧ r.met = false ⦄ := by
  unfold guard.call.decide_all
  apply WP.spec_mono (decide_all_loop_spec rs .Pass 0#usize (by simp)
    (by simp) (by simp))
  intro d hd
  rw [hd.2]
  simp only [holds, and_assoc]
  constructor
  · rintro ⟨r, hr, hm, hmet, ha⟩; exact ⟨r, hr, hm, ha, hmet⟩
  · rintro ⟨r, hr, hm, ha, hmet⟩; exact ⟨r, hr, hm, hmet, ha⟩

/-- The guard log keeps every audit evaluation, every decision that is not
a pass and every pass that rested on a waiver. -/
theorem logged_iff (m : guard.Mode) (d : guard.Decision) (w : Bool) :
    guard.call.logged m d w ⦃ b => b = true ↔ m = .Audit ∨ d ≠ .Pass ∨ w = true ⦄ := by
  unfold guard.call.logged
  cases m <;> cases d <;> cases w <;> simp

/-! ## `gates_fact` -/

theorem gates_fact_met_iff (o u i : Usize) :
    guard.gates_fact o u i ⦃ f => f = .Met ↔ 0 < o.val ∧ u.val = 0 ∧ i.val = 0 ⦄ := by
  unfold guard.gates_fact
  by_cases h : 0 < o.val ∧ u.val = 0 ∧ i.val = 0
  · have h1 : o > 0#usize := by scalar_tac
    have h2 : u = 0#usize := by scalar_tac
    have h3 : i = 0#usize := by scalar_tac
    simp [h1, h2, h3, h]
  · split <;> simp_all <;> scalar_tac

theorem gates_fact_never_unknown (o u i : Usize) :
    guard.gates_fact o u i ⦃ f => f ≠ .Unknown ⦄ := by
  unfold guard.gates_fact
  repeat' split
  all_goals simp

/-! ## The requirements' facts -/

theorem run_current_met_iff (found open_ : Bool) :
    guard.require.run_current found open_ ⦃ f => f = .Met ↔ found = true ∧ open_ = true ⦄ := by
  unfold guard.require.run_current
  cases found <;> cases open_ <;> simp

/-- `step.met` is met exactly when the plan was read and the run has no
plan, no current step, or a current step that owes nothing. -/
theorem step_fact_met_iff (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) :
    guard.require.step_fact r planned cur owed ⦃ f => f = .Met ↔
      r = .Read ∧ (planned = false ∨ cur = false ∨ owed.val = 0) ⦄ := by
  unfold guard.require.step_fact
  cases r
  · simp
  · simp
  · by_cases h : owed.val = 0
    · have : owed = 0#usize := by scalar_tac
      cases planned <;> cases cur <;> simp [this, h]
    · have : owed ≠ 0#usize := by intro h0; apply h; rw [h0]; rfl
      cases planned <;> cases cur <;> simp [this, h]

/-- `step.met` is unknown exactly when the plan or the step could not be read. -/
theorem step_fact_unknown_iff (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) :
    guard.require.step_fact r planned cur owed ⦃ f => f = .Unknown ↔ r = .Failed ⦄ := by
  unfold guard.require.step_fact
  cases r
  · simp
  · simp
  · by_cases h : owed.val = 0
    · have : owed = 0#usize := by scalar_tac
      cases planned <;> cases cur <;> simp [this]
    · have : owed ≠ 0#usize := by intro h0; apply h; rw [h0]; rfl
      cases planned <;> cases cur <;> simp [this]

/-- `step.settled` (#79) is met exactly when the plan was read and either
the current step is a spent loop or `step.met` would be met. -/
theorem settled_met_iff (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) (spent : Bool) :
    guard.require.settled_fact r planned cur owed spent ⦃ f => f = .Met ↔
      r = .Read ∧ (spent = true ∨ planned = false ∨ cur = false ∨ owed.val = 0) ⦄ := by
  unfold guard.require.settled_fact
  cases r
  · simp
  · simp
  · cases spent
    · simp only [Bool.false_eq_true, ite_false, false_or, true_and]
      apply WP.spec_mono (step_fact_met_iff .Read planned cur owed)
      intro f hf; simpa using hf
    · simp

/-- An unread plan is never settled: `step.settled` is unknown exactly when
the plan or the step could not be read. -/
theorem settled_unknown_iff_unread (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) (spent : Bool) :
    guard.require.settled_fact r planned cur owed spent ⦃ f => f = .Unknown ↔ r = .Failed ⦄ := by
  unfold guard.require.settled_fact
  cases r
  · simp
  · simp
  · dsimp only
    repeat' split
    all_goals first
      | simp; done
      | (apply WP.spec_mono (step_fact_unknown_iff .Read planned cur owed)
         intro f hf; simpa using hf)

/-- Without a spent loop, `step.settled` is `step.met`. -/
theorem not_spent_equals_step_fact (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) :
    guard.require.settled_fact r planned cur owed false ⦃ f => f = .Met ↔
      r = .Read ∧ (planned = false ∨ cur = false ∨ owed.val = 0) ⦄ := by
  apply WP.spec_mono (settled_met_iff r planned cur owed false)
  intro f hf; simpa using hf

/-- `gate:<id>` is met exactly when the gates were judged and the gate is
defined and met. -/
theorem gate_fact_met_iff (r : guard.require.Looked) (defined met : Bool) :
    guard.require.gate_fact r defined met ⦃ f => f = .Met ↔
      r = .Read ∧ defined = true ∧ met = true ⦄ := by
  unfold guard.require.gate_fact
  cases r <;> cases defined <;> cases met <;> simp

theorem gate_fact_unknown_iff (r : guard.require.Looked) (defined met : Bool) :
    guard.require.gate_fact r defined met ⦃ f => f = .Unknown ↔ r ≠ .Read ⦄ := by
  unfold guard.require.gate_fact
  cases r <;> cases defined <;> cases met <;> simp

/-- `gates.met` is met exactly when the gates were judged, something is
owed, all of it is met and nothing is invalid. -/
theorem gates_read_met_iff (r : guard.require.Looked) (o u i : Usize) :
    guard.require.gates_read r o u i ⦃ f => f = .Met ↔
      r = .Read ∧ 0 < o.val ∧ u.val = 0 ∧ i.val = 0 ⦄ := by
  unfold guard.require.gates_read
  cases r
  · simp
  · simp
  · apply WP.spec_mono (gates_fact_met_iff o u i)
    intro f hf
    simp [hf]

/-! ## The Stop nudge -/

theorem nudge_spec (active nudged : Bool) (fs : Slice guard.Fact) :
    guard.nudge active nudged fs ⦃ b => b = true ↔
      active = false ∧ nudged = false ∧ ∃ f ∈ fs.val, f ≠ .Met ⦄ := by
  unfold guard.nudge
  cases active <;> cases nudged <;> simp only [Bool.false_eq_true, ↓reduceIte]
  · step with all_met_spec as ⟨ b, hb ⟩
    cases b
    · have hn : ¬ ∀ f ∈ fs.val, f = .Met := fun h => by simpa using hb.mpr h
      push_neg at hn
      simpa using hn
    · have h := hb.mp rfl
      simp only [true_and, not_true_eq_false, decide_false, Bool.false_eq_true, false_iff,
        not_exists, not_and, not_not]
      exact h
  all_goals simp

/-- The nudge fires only when some requirement is not met, never while a
stop hook is already continuing, and, for a `session` rule, at most once a
session. -/
theorem nudge_with_iff (rep : guard.Repeat) (active nudged : Bool) (fs : Slice guard.Fact) :
    guard.nudge_with rep active nudged fs ⦃ b => b = true ↔
      active = false ∧ ¬ (nudged = true ∧ rep = .Session) ∧ ∃ f ∈ fs.val, f ≠ .Met ⦄ := by
  unfold guard.nudge_with
  cases rep <;> cases nudged <;>
    simp only [Bool.false_eq_true, ↓reduceIte, bind_tc_ok, bind_ok, true_and, and_true, not_true,
      not_false_eq_true, false_and, and_false, true_and, reduceCtorEq]
  · exact WP.spec_mono (nudge_spec active false fs) (fun b hb => by rw [hb]; simp)
  · exact WP.spec_mono (nudge_spec active true fs) (fun b hb => by rw [hb]; simp)
  · exact WP.spec_mono (nudge_spec active false fs) (fun b hb => by rw [hb]; simp)
  · exact WP.spec_mono (nudge_spec active false fs) (fun b hb => by rw [hb]; simp)

end Writfold.Core.Guard
