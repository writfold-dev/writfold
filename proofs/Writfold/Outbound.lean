import Extracted.WritfoldCore.Delivery
import Extracted.WritfoldCore.Types
import Writfold.Guard

/-!
# The outbound remainder (#187)

Theorems over the Aeneas translation of `writfold_core::delivery::outbound`:
one call's delivery decision from the class of each of its simple commands
and tool calls, under the KB's `unclassified` policy.

- `left_over_spec`, `contribution_spec`, `outbound_loop_spec`,
  `outbound_spec`: the decision is every member's contribution combined by
  `guard::combine`.
- `default_unchanged`: under `pass`, only routed and listed members decide,
  which is the shipped guard.
- `unclassified_pass_iff`: a call with an opaque or unclassified member
  passes exactly when the policy is `pass` or `log` and no routed or listed
  member refuses.
- `opaque_is_unclassified`: a wrapper is never a way round the policy.
- `local_never_refuses`, `log_never_decides`.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Guard (combineOf combine_spec)

namespace Writfold.Core.Delivery

/-- What an unclassified member gets, as a model. -/
def leftOverOf : delivery.outbound.Unclassified → guard.Decision
  | .Pass => .Pass
  | .Log => .Pass
  | .Ask => .Ask
  | .Deny => .Deny

theorem left_over_spec (p : delivery.outbound.Unclassified) :
    delivery.outbound.left_over p ⦃ d => d = leftOverOf p ⦄ := by
  unfold delivery.outbound.left_over
  cases p <;> simp [leftOverOf]

/-- One member's contribution, as a model. -/
def contributionOf (p : delivery.outbound.Unclassified) : delivery.outbound.Class → guard.Decision
  | .Local => .Pass
  | .Routed d => d
  | .Listed d => d
  | .Opaque => leftOverOf p
  | .Unclassified => leftOverOf p

theorem contribution_spec (c : delivery.outbound.Class) (p : delivery.outbound.Unclassified) :
    delivery.outbound.contribution c p ⦃ d => d = contributionOf p c ⦄ := by
  unfold delivery.outbound.contribution
  cases c with
  | Opaque => exact WP.spec_mono (left_over_spec p) (fun d h => by simpa [contributionOf] using h)
  | Unclassified => exact WP.spec_mono (left_over_spec p) (fun d h => by simpa [contributionOf] using h)
  | _ => simp [contributionOf]

/-- One call's decision, as a model: the contributions combined. -/
def outboundOf (cs : List delivery.outbound.Class) (p : delivery.outbound.Unclassified) : guard.Decision :=
  (cs.map (contributionOf p)).foldl combineOf .Pass

theorem outbound_loop_spec (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified)
    (acc : guard.Decision) (i : Usize) (hi : i.val ≤ cs.val.length)
    (hacc : acc = ((cs.val.take i.val).map (contributionOf p)).foldl combineOf .Pass) :
    delivery.outbound.outbound_loop cs p acc i ⦃ d => d = outboundOf cs.val p ⦄ := by
  unfold delivery.outbound.outbound_loop
  apply loop.spec_decr_nat (fun q : guard.Decision × Usize => cs.val.length - q.2.val)
    (fun q : guard.Decision × Usize => q.2.val ≤ cs.val.length ∧
      q.1 = ((cs.val.take q.2.val).map (contributionOf p)).foldl combineOf .Pass)
  · rintro ⟨acc, i⟩ ⟨hi, hacc⟩
    unfold delivery.outbound.outbound_loop.body
    dsimp only at hi hacc ⊢
    split
    · rename_i hlt
      have hil : i.val < cs.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      step with contribution_spec as ⟨ e, he ⟩
      step with combine_spec as ⟨ c, hc ⟩
      step as ⟨ i', hi' ⟩
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hc, hi1, List.take_succ, List.getElem?_eq_getElem hil, Option.toList_some,
        List.map_append, List.foldl_append, ← hacc, he, hx]
      simp
    · rename_i hge
      simp only [WP.spec_ok]
      have hlen : i.val = cs.val.length := by scalar_tac
      rw [hacc, hlen, List.take_length]
      rfl
  · exact ⟨hi, hacc⟩

theorem outbound_spec (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified) :
    delivery.outbound.outbound cs p ⦃ d => d = outboundOf cs.val p ⦄ := by
  unfold delivery.outbound.outbound
  exact outbound_loop_spec cs p .Pass 0#usize (by simp) (by simp)

/-! ## The model's laws -/

theorem combineOf_pass_right (a : guard.Decision) : combineOf a .Pass = a := by
  cases a <;> rfl

theorem foldl_combine_pass_iff (acc : guard.Decision) (xs : List guard.Decision) :
    xs.foldl combineOf acc = .Pass ↔ acc = .Pass ∧ ∀ x ∈ xs, x = .Pass := by
  induction xs generalizing acc with
  | nil => simp
  | cons x xs ih =>
    rw [List.foldl_cons, ih]
    cases acc <;> cases x <;> simp [combineOf]

/-- The routed and listed decisions of a call. -/
def judgedOf : delivery.outbound.Class → Option guard.Decision
  | .Routed d => some d
  | .Listed d => some d
  | _ => none

theorem foldl_unchanged (acc : guard.Decision) (cs : List delivery.outbound.Class) :
    (cs.map (contributionOf .Pass)).foldl combineOf acc = (cs.filterMap judgedOf).foldl combineOf acc := by
  induction cs generalizing acc with
  | nil => rfl
  | cons c cs ih =>
    cases c <;> simp [contributionOf, judgedOf, leftOverOf, combineOf_pass_right, ih, List.filterMap_cons]

/-- Under `pass`, only routed and listed members decide: a `guard.toml`
without the new keys behaves as the shipped guard. -/
theorem default_unchanged (cs : Slice delivery.outbound.Class) :
    delivery.outbound.outbound cs .Pass ⦃ d => d = (cs.val.filterMap judgedOf).foldl combineOf .Pass ⦄ :=
  WP.spec_mono (outbound_spec cs .Pass) (fun d h => by rw [h, outboundOf, foldl_unchanged])

/-- `log` passes what `pass` passes: it only keeps the command in the log. -/
theorem log_never_decides (cs : Slice delivery.outbound.Class) :
    delivery.outbound.outbound cs .Log ⦃ d => d = outboundOf cs.val .Pass ⦄ :=
  WP.spec_mono (outbound_spec cs .Log) (fun d h => by
    rw [h, outboundOf, outboundOf]
    congr 1)

/-- A wrapper that runs its arguments is unclassified. -/
def unwrap : delivery.outbound.Class → delivery.outbound.Class
  | .Opaque => .Unclassified
  | c => c

theorem opaque_is_unclassified (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified) :
    delivery.outbound.outbound cs p ⦃ d => d = outboundOf (cs.val.map unwrap) p ⦄ :=
  WP.spec_mono (outbound_spec cs p) (fun d h => by
    rw [h, outboundOf, outboundOf, List.map_map]
    congr 1
    apply List.map_congr_left
    intro c _; cases c <;> rfl)

/-- A call made only of local commands passes under every policy. -/
theorem local_never_refuses (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified)
    (h : ∀ c ∈ cs.val, c = .Local) :
    delivery.outbound.outbound cs p ⦃ d => d = .Pass ⦄ :=
  WP.spec_mono (outbound_spec cs p) (fun d hd => by
    rw [hd, outboundOf, foldl_combine_pass_iff]
    refine ⟨rfl, ?_⟩
    intro x hx
    obtain ⟨c, hc, rfl⟩ := List.mem_map.mp hx
    rw [h c hc]; rfl)

/-- A call with an opaque or unclassified member passes exactly when the
policy is `pass` or `log` and no routed or listed member refuses. -/
theorem unclassified_pass_iff (cs : Slice delivery.outbound.Class) (p : delivery.outbound.Unclassified)
    (h : ∃ c ∈ cs.val, c = .Opaque ∨ c = .Unclassified) :
    delivery.outbound.outbound cs p ⦃ d => d = .Pass ↔
      (p = .Pass ∨ p = .Log) ∧ ∀ c ∈ cs.val, ∀ j, judgedOf c = some j → j = .Pass ⦄ :=
  WP.spec_mono (outbound_spec cs p) (fun d hd => by
    rw [hd, outboundOf, foldl_combine_pass_iff]
    obtain ⟨u, hu, hk⟩ := h
    constructor
    · rintro ⟨_, hall⟩
      have hl : leftOverOf p = .Pass := by
        have := hall _ (List.mem_map.mpr ⟨u, hu, rfl⟩)
        rcases hk with rfl | rfl <;> simpa [contributionOf] using this
      refine ⟨by cases p <;> simp_all [leftOverOf], ?_⟩
      intro c hc j hj
      have := hall _ (List.mem_map.mpr ⟨c, hc, rfl⟩)
      cases c <;> simp_all [judgedOf, contributionOf]
    · rintro ⟨hp, hall⟩
      refine ⟨rfl, ?_⟩
      intro x hx
      obtain ⟨c, hc, rfl⟩ := List.mem_map.mp hx
      have hl : leftOverOf p = .Pass := by rcases hp with rfl | rfl <;> rfl
      cases c with
      | Routed j => exact hall _ hc j rfl
      | Listed j => exact hall _ hc j rfl
      | _ => simp [contributionOf, hl])

end Writfold.Core.Delivery
