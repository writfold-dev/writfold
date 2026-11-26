import Extracted.WritfoldCore.Invoke
import Extracted.WritfoldCore.Types

/-!
# Preparing an agent invocation (#147, #148, RFC 2026-11-25-agent-vocabulary §3.8)

Theorems over the Aeneas translation of `writfold_core::invoke`.

- `prepare_check_spec`: `prepare_check` names the first of its eight
  facts, in the fixed order, that does not hold (`firstRefusal`).
- `prepare_ok_iff`: admitted exactly when the run is open, the step is
  current, the closure is admitted, every skill's plan is in the run, the
  input was read, a required packet is complete, `--out` is outside the KB
  and `--for`, when given, names an item the checklist has now (#167).
- `plan_missing_refused`, `packet_incomplete_refused`, `out_inside_refused`,
  `for_unknown_refused`: a missing plan, an incomplete packet the profile
  requires, `--out` inside the KB, or `--for` an unknown item is never
  admitted.
- `bound_spec` (#149): `invocation.bound` fails when no invocation is
  named, is unknown when the named record does not read, and otherwise
  passes exactly when every fact holds (`bound_pass_iff`).
- `bound_unread_unknown`: an unreadable record is neither a pass nor a
  failure.
- `conflict_never_passes`: two different reports for one invocation never
  pass, whichever is newer.
- `first_unmet_spec` (#154): the least index of a need that is not met.
- `prepare_spec`, `prepare_needs_ok_iff`: `prepare` is `prepare_check`, then
  the needs in order; admitted exactly when the eight facts hold and every
  need is met. `needs_unknown_refused`: a need that could not be judged is
  never admitted. `needs_first`: the refusal names the least unmet need.
  `no_needs_unchanged`: with no needs the decision is `prepare_check`'s.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Invoke

/-- The first fact that does not hold, in the fixed order. -/
def firstRefusal (f : invoke.Prepare) : Option invoke.PrepareRefusal :=
  if !f.run_open then some .RunClosed
  else if !f.current_step then some .NotCurrentStep
  else if !f.closure_admitted then some .Closure
  else if !f.plans_in_run then some .PlanNotInRun
  else if !f.input_read then some .InputUnread
  else if f.packet_required && !f.packet_complete then some .PacketIncomplete
  else if !f.out_outside then some .OutInsideKb
  else match f.for_item with
    | .Unknown => some .ForUnknown
    | _ => none

/-- All eight facts hold. -/
def Admitted (f : invoke.Prepare) : Prop :=
  f.run_open = true ∧ f.current_step = true ∧ f.closure_admitted = true ∧
  f.plans_in_run = true ∧ f.input_read = true ∧
  (f.packet_required = true → f.packet_complete = true) ∧
  f.out_outside = true ∧ f.for_item ≠ .Unknown

theorem prepare_check_spec (f : invoke.Prepare) :
    invoke.prepare_check f ⦃ r => r = match firstRefusal f with
      | none => .Ok ()
      | some e => .Err e ⦄ := by
  rcases f with ⟨a, b, c, d, e, g, h, o, i⟩
  unfold invoke.prepare_check
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    cases o <;> cases i <;> simp [firstRefusal]

theorem firstRefusal_none_iff (f : invoke.Prepare) : firstRefusal f = none ↔ Admitted f := by
  rcases f with ⟨a, b, c, d, e, g, h, o, i⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    cases o <;> cases i <;> simp [firstRefusal, Admitted]

theorem prepare_ok_iff (f : invoke.Prepare) :
    invoke.prepare_check f ⦃ r => r = .Ok () ↔ Admitted f ⦄ := by
  apply WP.spec_mono (prepare_check_spec f)
  intro r hr
  rw [hr, ← firstRefusal_none_iff]
  cases firstRefusal f <;> simp

/-- A skill's plan template not in the run is never admitted. -/
theorem plan_missing_refused (f : invoke.Prepare) (hp : f.plans_in_run = false) :
    invoke.prepare_check f ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (prepare_ok_iff f)
  intro r hr hok
  have := (hr.mp hok).2.2.2.1
  rw [hp] at this
  cases this

/-- A required packet that is not complete is never admitted. -/
theorem packet_incomplete_refused (f : invoke.Prepare) (hr : f.packet_required = true)
    (hc : f.packet_complete = false) :
    invoke.prepare_check f ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (prepare_ok_iff f)
  intro r hiff hok
  have := (hiff.mp hok).2.2.2.2.2.1 hr
  rw [hc] at this
  cases this

/-- `--out` inside the KB is never admitted (#167). -/
theorem out_inside_refused (f : invoke.Prepare) (ho : f.out_outside = false) :
    invoke.prepare_check f ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (prepare_ok_iff f)
  intro r hiff hok
  have := (hiff.mp hok).2.2.2.2.2.2.1
  rw [ho] at this
  cases this

/-- `--for` an item the checklist does not have now is never admitted (#167). -/
theorem for_unknown_refused (f : invoke.Prepare) (hu : f.for_item = .Unknown) :
    invoke.prepare_check f ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (prepare_ok_iff f)
  intro r hiff hok
  exact (hiff.mp hok).2.2.2.2.2.2.2 hu

/-- What `invocation.bound` decides. -/
def boundOf (n : invoke.Named) : gate.Outcome :=
  if !n.names then .Fail
  else if !n.readable then .Unknown
  else if n.this_run && n.this_step && n.this_agent && n.closure_current && n.single then .Pass
  else .Fail

/-- Every fact holds. -/
def Bound (n : invoke.Named) : Prop :=
  n.names = true ∧ n.readable = true ∧ n.this_run = true ∧ n.this_step = true ∧
  n.this_agent = true ∧ n.closure_current = true ∧ n.single = true

theorem bound_spec (n : invoke.Named) : invoke.bound n ⦃ o => o = boundOf n ⦄ := by
  rcases n with ⟨a, b, c, d, e, g, h⟩
  unfold invoke.bound
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    simp [boundOf]

theorem bound_pass_iff (n : invoke.Named) : invoke.bound n ⦃ o => o = .Pass ↔ Bound n ⦄ := by
  apply WP.spec_mono (bound_spec n)
  intro o ho
  rw [ho]
  rcases n with ⟨a, b, c, d, e, g, h⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    simp [boundOf, Bound]

/-- A named record that does not read is unknown: never a pass, never a failure. -/
theorem bound_unread_unknown (n : invoke.Named) (hn : n.names = true)
    (hr : n.readable = false) : invoke.bound n ⦃ o => o = .Unknown ⦄ := by
  apply WP.spec_mono (bound_spec n)
  intro o ho
  rw [ho]
  simp [boundOf, hn, hr]

/-- Two different reports for one invocation never pass, whichever is newer. -/
theorem conflict_never_passes (n : invoke.Named) (hs : n.single = false) :
    invoke.bound n ⦃ o => o ≠ .Pass ⦄ := by
  apply WP.spec_mono (bound_pass_iff n)
  intro o hiff hp
  have := (hiff.mp hp).2.2.2.2.2.2
  rw [hs] at this
  cases this

/-- Every need is met. -/
def AllMet (needs : List guard.Fact) : Prop :=
  ∀ j (hj : j < needs.length), needs[j] = .Met

def FirstUnmet (needs : List guard.Fact) : Option Usize → Prop
  | none => AllMet needs
  | some i => ∃ h : i.val < needs.length, needs[i.val] ≠ .Met ∧
      ∀ j (hj : j < i.val), (needs[j]'(by omega)) = .Met

theorem first_unmet_loop_spec (needs : Slice guard.Fact) (i : Usize)
    (hi : i.val ≤ needs.val.length)
    (hmet : ∀ j (_ : j < i.val) (hj' : j < needs.val.length), needs.val[j] = .Met) :
    invoke.first_unmet_loop needs i ⦃ r => FirstUnmet needs.val r ⦄ := by
  unfold invoke.first_unmet_loop
  apply loop.spec_decr_nat (fun i : Usize => needs.val.length - i.val)
    (fun i : Usize => i.val ≤ needs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < needs.val.length), needs.val[j] = .Met)
  · intro i ⟨hi, hmet⟩
    unfold invoke.first_unmet_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      split
      · apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hmet j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          exact hx.symm
      all_goals
        simp only [WP.spec_ok, FirstUnmet]
        exact ⟨by scalar_tac, by rw [← hx]; simp, fun j hj => hmet j hj _⟩
    · simp only [WP.spec_ok, FirstUnmet, AllMet]
      intro j hj
      exact hmet j (by scalar_tac) hj
  · exact ⟨hi, hmet⟩

theorem first_unmet_spec (needs : Slice guard.Fact) :
    invoke.first_unmet needs ⦃ r => FirstUnmet needs.val r ⦄ := by
  unfold invoke.first_unmet
  apply first_unmet_loop_spec
  · simp
  · intro j hj; simp at hj

theorem firstRefusal_not_needs (f : invoke.Prepare) (k : Usize) :
    firstRefusal f ≠ some (.Needs k) := by
  rcases f with ⟨a, b, c, d, g, p, q, o, i⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases g <;> cases p <;> cases q <;>
    cases o <;> cases i <;> simp [firstRefusal]

/-- What `prepare` decides. -/
def PrepareSpec (f : invoke.Prepare) (needs : List guard.Fact) :
    core.result.Result Unit invoke.PrepareRefusal → Prop
  | .Ok () => firstRefusal f = none ∧ AllMet needs
  | .Err (.Needs i) => firstRefusal f = none ∧ FirstUnmet needs (some i)
  | .Err e => firstRefusal f = some e

theorem prepare_spec (f : invoke.Prepare) (needs : Slice guard.Fact) :
    invoke.prepare f needs ⦃ r => PrepareSpec f needs.val r ⦄ := by
  unfold invoke.prepare
  step with prepare_check_spec as ⟨ r, hr ⟩
  cases hf : firstRefusal f with
  | none =>
    rw [hf] at hr
    subst hr
    simp only
    step with first_unmet_spec as ⟨ o, ho ⟩
    cases o with
    | none => simp only [WP.spec_ok, PrepareSpec]; exact ⟨hf, ho⟩
    | some i => simp only [WP.spec_ok, PrepareSpec]; exact ⟨hf, ho⟩
  | some e =>
    rw [hf] at hr
    subst hr
    simp only [WP.spec_ok]
    cases e with
    | Needs k => exact absurd hf (firstRefusal_not_needs f k)
    | _ => simp [PrepareSpec, hf]

/-- Admitted exactly when the eight facts hold and every need is met. -/
theorem prepare_needs_ok_iff (f : invoke.Prepare) (needs : Slice guard.Fact) :
    invoke.prepare f needs ⦃ r => r = .Ok () ↔ Admitted f ∧ AllMet needs.val ⦄ := by
  apply WP.spec_mono (prepare_spec f needs)
  intro r hr
  rw [← firstRefusal_none_iff]
  constructor
  · intro h; subst h; exact hr
  · intro ⟨h1, h2⟩
    rcases r with ⟨⟩ | e
    · rfl
    · cases e with
      | Needs i =>
        obtain ⟨_, hne, _⟩ := hr.2
        exact absurd (h2 _ _) hne
      | _ => simp_all [PrepareSpec]

/-- A need that could not be judged is never admitted. -/
theorem needs_unknown_refused (f : invoke.Prepare) (needs : Slice guard.Fact)
    (j : Nat) (hj : j < needs.val.length) (hu : needs.val[j] = .Unknown) :
    invoke.prepare f needs ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (prepare_needs_ok_iff f needs)
  intro r hiff hok
  have := (hiff.mp hok).2 j hj
  rw [hu] at this
  cases this

/-- The refusal names the least need that is not met. -/
theorem needs_first (f : invoke.Prepare) (needs : Slice guard.Fact) :
    invoke.prepare f needs ⦃ r => ∀ i, r = .Err (.Needs i) →
      ∃ h : i.val < needs.val.length, needs.val[i.val] ≠ .Met ∧
        ∀ j (hj : j < i.val), (needs.val[j]'(by omega)) = .Met ⦄ := by
  apply WP.spec_mono (prepare_spec f needs)
  intro r hr i hi
  subst hi
  exact hr.2

/-- With no needs, `prepare` decides as `prepare_check`. -/
theorem no_needs_unchanged (f : invoke.Prepare) (needs : Slice guard.Fact)
    (h0 : needs.val = []) :
    invoke.prepare f needs ⦃ r => r = match firstRefusal f with
      | none => .Ok ()
      | some e => .Err e ⦄ := by
  apply WP.spec_mono (prepare_spec f needs)
  intro r hr
  rw [h0] at hr
  rcases r with ⟨⟩ | e
  · simp [PrepareSpec] at hr; simp [hr.1]
  · cases e <;> simp_all [PrepareSpec, FirstUnmet]

end Writfold.Core.Invoke
