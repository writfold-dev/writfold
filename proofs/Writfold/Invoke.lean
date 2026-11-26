import Extracted.WritfoldCore.Invoke
import Extracted.WritfoldCore.Types

/-!
# Preparing an agent invocation (#147, #148, RFC 2026-11-25-agent-vocabulary §3.8)

Theorems over the Aeneas translation of `writfold_core::invoke`.

- `prepare_check_spec`: `prepare_check` names the first of its six facts,
  in the fixed order, that does not hold (`firstRefusal`).
- `prepare_ok_iff`: admitted exactly when the run is open, the step is
  current, the closure is admitted, every skill's plan is in the run, the
  input was read, and a required packet is complete.
- `plan_missing_refused`, `packet_incomplete_refused`: a missing plan, or
  an incomplete packet the profile requires, is never admitted.
- `bound_spec` (#149): `invocation.bound` fails when no invocation is
  named, is unknown when the named record does not read, and otherwise
  passes exactly when every fact holds (`bound_pass_iff`).
- `bound_unread_unknown`: an unreadable record is neither a pass nor a
  failure.
- `conflict_never_passes`: two different reports for one invocation never
  pass, whichever is newer.
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
  else none

/-- All six facts hold. -/
def Admitted (f : invoke.Prepare) : Prop :=
  f.run_open = true ∧ f.current_step = true ∧ f.closure_admitted = true ∧
  f.plans_in_run = true ∧ f.input_read = true ∧
  (f.packet_required = true → f.packet_complete = true)

theorem prepare_check_spec (f : invoke.Prepare) :
    invoke.prepare_check f ⦃ r => r = match firstRefusal f with
      | none => .Ok ()
      | some e => .Err e ⦄ := by
  rcases f with ⟨a, b, c, d, e, g, h⟩
  unfold invoke.prepare_check
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    simp [firstRefusal]

theorem firstRefusal_none_iff (f : invoke.Prepare) : firstRefusal f = none ↔ Admitted f := by
  rcases f with ⟨a, b, c, d, e, g, h⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;>
    simp [firstRefusal, Admitted]

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
  have := (hiff.mp hok).2.2.2.2.2 hr
  rw [hc] at this
  cases this

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

end Writfold.Core.Invoke
