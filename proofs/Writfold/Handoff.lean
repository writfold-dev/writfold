import Writfold.Gate

/-!
# The strict handoff decision (#57)

Theorems over the Aeneas translation of `writfold_core::handoff`: a
receiver accepts a candidate only when the inventory of required gates is
complete, at least one gate is required, and every required gate has its
approved definition and a current, mechanically passing result.

- `item_spec`, `itemOf_none_iff`: one required gate passes exactly when it
  is defined, its evidence was made under the approved definition, and its
  raw standing is `Checked Satisfied`.
- `statement_never_passes`: an attested or waived gate is refused as a
  `Statement`, whatever else holds; the kernel has no input for a gate's
  `accept`, a plan, a skip or an exception.
- `strict_accepted_iff`: accepted exactly when complete, non-empty and every
  item passes.
- `strict_refused_names_lowest`: a refusal is `Incomplete`, `Empty`, or
  names the lowest failing index with that item's reason.
- `failing_member_refused`: no set holding a failing item is accepted, so
  adding requirements never hides a failure.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Handoff

/-- `handoff::item` on the model. -/
def itemOf (x : handoff.Item) : Option handoff.Reason :=
  if x.defined = false then some .Undefined
  else match x.standing with
    | .Attested => some .Statement
    | .Waived => some .Statement
    | .Checked .Open => some .NoEvidence
    | .Checked s =>
      if x.approved = false then some .DefinitionChanged
      else match s with
        | .Satisfied => none
        | .Stale => some .Stale
        | _ => some .NotPassed

/-- One required gate passes. -/
def Passes (x : handoff.Item) : Prop :=
  x.defined = true ∧ x.approved = true ∧ x.standing = .Checked .Satisfied

theorem item_spec (x : handoff.Item) : handoff.item x ⦃ o => o = itemOf x ⦄ := by
  rcases x with ⟨d, a, s⟩
  unfold handoff.item
  cases d <;> cases a <;> rcases s with st | _ | _ <;> (try cases st) <;> simp [itemOf]

theorem itemOf_none_iff (x : handoff.Item) : itemOf x = none ↔ Passes x := by
  rcases x with ⟨d, a, s⟩
  cases d <;> cases a <;> rcases s with st | _ | _ <;> (try cases st) <;> simp [itemOf, Passes]

/-- An attested or waived gate never passes: it is refused as a statement. -/
theorem statement_never_passes (x : handoff.Item) (hd : x.defined = true)
    (hs : x.standing = .Attested ∨ x.standing = .Waived) :
    handoff.item x ⦃ o => o = some .Statement ⦄ := by
  apply WP.spec_mono (item_spec x)
  intro o ho
  rw [ho]
  rcases x with ⟨d, a, s⟩
  simp only at hd hs
  subst hd
  rcases hs with h | h <;> subst h <;> simp [itemOf]

/-- What `first_failing` found: every item passes, or the first one that
does not, with its reason. -/
def Found (xs : List handoff.Item) : Option (Usize × handoff.Reason) → Prop
  | none => ∀ x ∈ xs, Passes x
  | some (at_, r) => ∃ h : at_.val < xs.length, itemOf xs[at_.val] = some r ∧
      ∀ j (hj : j < at_.val), Passes (xs[j]'(by omega))

theorem first_failing_loop_spec (xs : Slice handoff.Item) (i : Usize)
    (hi : i.val ≤ xs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < xs.val.length), Passes xs.val[j]) :
    handoff.first_failing_loop xs i ⦃ o => Found xs.val o ⦄ := by
  unfold handoff.first_failing_loop
  apply loop.spec_decr_nat (fun i : Usize => xs.val.length - i.val)
    (fun i : Usize => i.val ≤ xs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < xs.val.length), Passes xs.val[j])
  · intro i ⟨hi, hprefix⟩
    unfold handoff.first_failing_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with item_spec as ⟨ o, ho ⟩
      cases o with
      | none =>
        simp only
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; exact (itemOf_none_iff x).mp ho.symm
      | some r =>
        simp only [WP.spec_ok, Found]
        refine ⟨by scalar_tac, ?_, fun j hj => hprefix j hj _⟩
        rw [← hx]; exact ho.symm
    · rename_i hge
      simp only [WP.spec_ok, Found]
      intro x hx
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

theorem first_failing_spec (xs : Slice handoff.Item) :
    handoff.first_failing xs ⦃ o => Found xs.val o ⦄ := by
  unfold handoff.first_failing
  exact first_failing_loop_spec xs 0#usize (by simp) (by intro j hj; simp at hj)

/-- What `strict` decides, on the model. -/
def Decided (c : Bool) (xs : List handoff.Item) : handoff.Verdict → Prop
  | .Accepted => c = true ∧ xs ≠ [] ∧ ∀ x ∈ xs, Passes x
  | .Refused i r =>
      (c = false ∧ i.val = 0 ∧ r = .Incomplete) ∨
      (c = true ∧ xs = [] ∧ i.val = 0 ∧ r = .Empty) ∨
      (c = true ∧ ∃ h : i.val < xs.length, itemOf xs[i.val] = some r ∧
        ∀ j (hj : j < i.val), Passes (xs[j]'(by omega)))

theorem strict_spec (c : Bool) (xs : Slice handoff.Item) :
    handoff.strict c xs ⦃ v => Decided c xs.val v ⦄ := by
  unfold handoff.strict
  cases c
  · simp [Decided]
  · simp only [ite_true]
    step as ⟨ e, he ⟩
    split
    · rename_i het
      simp only [WP.spec_ok, Decided]
      refine Or.inr (Or.inl ⟨trivial, ?_, by simp, by simp⟩)
      simp_all [Slice.length]
    · rename_i hef
      step with first_failing_spec as ⟨ o, ho ⟩
      have hne : xs.val ≠ [] := by intro h; apply hef; simp_all [Slice.length]
      cases o with
      | none => simp only [WP.spec_ok, Decided]; exact ⟨trivial, hne, ho⟩
      | some p =>
        obtain ⟨at_, r⟩ := p
        show (ok (handoff.Verdict.Refused at_ r)) ⦃ v => Decided true xs.val v ⦄
        simp only [WP.spec_ok, Decided]
        exact Or.inr (Or.inr ⟨trivial, ho⟩)

/-- Accepted exactly when the inventory is complete, something is required
and every required gate passes. -/
theorem strict_accepted_iff (c : Bool) (xs : Slice handoff.Item) :
    handoff.strict c xs ⦃ v => v = .Accepted ↔
      c = true ∧ xs.val ≠ [] ∧ ∀ x ∈ xs.val, Passes x ⦄ := by
  apply WP.spec_mono (strict_spec c xs)
  intro v hv
  cases v with
  | Accepted => simpa [Decided] using hv
  | Refused i r =>
    simp only [Decided] at hv
    simp only [reduceCtorEq, false_iff, not_and]
    intro hc hne hall
    rcases hv with ⟨hc', _, _⟩ | ⟨_, hnil, _, _⟩ | ⟨_, h, hr, _⟩
    · simp_all
    · exact hne hnil
    · have := (itemOf_none_iff _).mpr (hall _ (List.getElem_mem h))
      simp_all

/-- A refusal is for an incomplete inventory, an empty set, or names the
lowest failing gate with its reason. -/
theorem strict_refused_names_lowest (c : Bool) (xs : Slice handoff.Item) :
    handoff.strict c xs ⦃ v => ∀ i r, v = .Refused i r →
      (c = false ∧ r = .Incomplete) ∨ (xs.val = [] ∧ r = .Empty) ∨
      (∃ h : i.val < xs.val.length, itemOf xs.val[i.val] = some r ∧
        ∀ j (hj : j < i.val), Passes (xs.val[j]'(by omega))) ⦄ := by
  apply WP.spec_mono (strict_spec c xs)
  intro v hv i r hvr
  subst hvr
  simp only [Decided] at hv
  rcases hv with ⟨hc, _, hr⟩ | ⟨_, hnil, _, hr⟩ | ⟨_, h⟩
  · exact Or.inl ⟨hc, hr⟩
  · exact Or.inr (Or.inl ⟨hnil, hr⟩)
  · exact Or.inr (Or.inr h)

/-- No set that holds a failing gate is accepted. -/
theorem failing_member_refused (c : Bool) (xs : Slice handoff.Item) (x : handoff.Item)
    (hx : x ∈ xs.val) (hfail : ¬ Passes x) :
    handoff.strict c xs ⦃ v => v ≠ .Accepted ⦄ := by
  apply WP.spec_mono (strict_accepted_iff c xs)
  intro v hv hacc
  exact hfail ((hv.mp hacc).2.2 x hx)

/-! ## Totality (tauten claims) -/

theorem first_failing_total (xs : Slice handoff.Item) :
    handoff.first_failing xs ⦃ _ => True ⦄ :=
  WP.spec_mono (first_failing_spec xs) (fun _ _ => trivial)

theorem strict_total (c : Bool) (xs : Slice handoff.Item) :
    handoff.strict c xs ⦃ _ => True ⦄ :=
  WP.spec_mono (strict_spec c xs) (fun _ _ => trivial)

end Writfold.Core.Handoff
