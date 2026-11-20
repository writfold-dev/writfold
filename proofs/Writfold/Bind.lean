import Writfold.Checklist

/-!
# Evidence binding

Theorems over the Aeneas translation of `writfold_core::bind`: whether the
references a checklist cover row cites answer the item they cover.

- `answers_spec`, `answers_any`, `other_never_answers`,
  `unbound_never_answers`, `named_need_iff`: what answers each need. Under
  `Any` (no `evidence` key) everything does; under `Own` and `Named` a look
  recorded for another item, or a reference that names nothing, never does.
- `answered_spec`: an item is answered exactly when some cited reference is
  grounded and answers it.
- `answeredB_skip`, `answered_unrelated`: monotonicity. Adding a reference
  that does not answer the item, or is not grounded, never changes whether
  it is answered; shared evidence that is about another item cannot ground
  this one.
- `item_under_spec`, `item_under_answered`, `item_under_supported_iff`,
  `item_under_none_found_iff`, `item_under_narrows`,
  `item_under_unanswered`: binding only narrows `checklist.item`. With the
  references answering (always so under `Any` with a grounded reference) it
  is `checklist.item` exactly; otherwise a supported or none-found standing
  becomes stale and nothing else changes.
- `bound_supported_iff`: relevance soundness, end to end. An item stands
  supported under its need exactly when its newest row is current,
  supported and grounded as before, and some cited reference is grounded
  and answers the item.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Bind

open Writfold.Core.Checklist (itemOf item_spec)

/-! ## `answers` -/

/-- `answers` on the model. -/
def answersB : bind.Need → bind.Bound → Bool
  | .Any, _ => true
  | .Own, .Named => true
  | .Own, .This => true
  | .Own, _ => false
  | .Named, .Named => true
  | .Named, _ => false

theorem answers_spec (need : bind.Need) (bound : bind.Bound) :
    bind.answers need bound ⦃ b => b = answersB need bound ⦄ := by
  unfold bind.answers
  cases need <;> cases bound <;> simp [answersB]

/-- Without an `evidence` key every reference answers. -/
theorem answers_any (bound : bind.Bound) : bind.answers .Any bound ⦃ b => b = true ⦄ := by
  apply WP.spec_mono (answers_spec .Any bound)
  intro b hb; simp [hb, answersB]

/-- A look recorded for another item never answers an item that declares
what answers it. -/
theorem other_never_answers (need : bind.Need) (h : need ≠ .Any) :
    bind.answers need .Other ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (answers_spec need .Other)
  intro b hb; cases need <;> simp_all [answersB]

/-- A reference that names no item, and that the item does not name, never
answers an item that declares what answers it. -/
theorem unbound_never_answers (need : bind.Need) (h : need ≠ .Any) :
    bind.answers need .Unbound ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (answers_spec need .Unbound)
  intro b hb; cases need <;> simp_all [answersB]

/-- Under `Named` only the gates the item names answer it. -/
theorem named_need_iff (bound : bind.Bound) :
    bind.answers .Named bound ⦃ b => b = true ↔ bound = .Named ⦄ := by
  apply WP.spec_mono (answers_spec .Named bound)
  intro b hb; subst hb; cases bound <;> simp [answersB]

/-! ## `answered` -/

/-- One reference grounds and answers the item. -/
def hitB (need : bind.Need) (r : bind.Ref) : Bool := r.grounded && answersB need r.bound

/-- `answered` on the model. -/
def answeredB (need : bind.Need) (l : List bind.Ref) : Bool := l.any (hitB need)

theorem answered_from_loop_spec (need : bind.Need) (refs : Slice bind.Ref) (i : Usize)
    (hi : i.val ≤ refs.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < refs.val.length),
      hitB need refs.val[j] = false) :
    bind.answered_from_loop need refs i ⦃ b => b = true ↔ ∃ r ∈ refs.val, hitB need r = true ⦄ := by
  unfold bind.answered_from_loop
  apply loop.spec_decr_nat (fun i : Usize => refs.val.length - i.val)
    (fun i : Usize => i.val ≤ refs.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < refs.val.length), hitB need refs.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold bind.answered_from_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      split
      · rename_i hg
        step with answers_spec as ⟨ b, hb ⟩
        split
        · rename_i hbt
          simp only [WP.spec_ok, true_iff]
          refine ⟨refs.val[i.val]'(by scalar_tac), List.getElem_mem _, ?_⟩
          rw [← hx]; simp [hitB, hg, ← hb, hbt]
        · rename_i hbt
          step as ⟨ i', hi' ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro j hj hj'
          by_cases hji : j < i.val
          · exact hprefix j hji hj'
          · have : j = i.val := by scalar_tac
            subst this
            rw [← hx]; simp [hitB, ← hb, hbt]
      · rename_i hg
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; simp [hitB, hg]
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro r hr hhit
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hr
      have := hprefix j (by scalar_tac) hj
      simp_all
  · exact ⟨hi, hprefix⟩

/-- An item is answered exactly when some cited reference is grounded and
answers it. -/
theorem answered_spec (need : bind.Need) (refs : Slice bind.Ref) :
    bind.answered need refs ⦃ b => b = answeredB need refs.val ⦄ := by
  unfold bind.answered bind.answered_from
  apply WP.spec_mono (answered_from_loop_spec need refs 0#usize (by simp)
    (by intro j hj; simp at hj))
  intro b hb
  apply Bool.eq_iff_iff.mpr
  rw [hb]
  simp [answeredB, List.any_eq_true]

theorem answered_iff (need : bind.Need) (refs : Slice bind.Ref) :
    bind.answered need refs ⦃ b => b = true ↔
      ∃ r ∈ refs.val, r.grounded = true ∧ answersB need r.bound = true ⦄ := by
  apply WP.spec_mono (answered_spec need refs)
  intro b hb; subst hb
  simp [answeredB, List.any_eq_true, hitB]

/-- Monotonicity on the model: a reference that does not ground, or does
not answer the item, never changes whether the item is answered, wherever
it is cited. -/
theorem answeredB_skip (need : bind.Need) (l₁ l₂ : List bind.Ref) (r : bind.Ref)
    (h : hitB need r = false) :
    answeredB need (l₁ ++ r :: l₂) = answeredB need (l₁ ++ l₂) := by
  simp [answeredB, List.any_append, h]

/-- Monotonicity on the code: citing an unrelated reference alongside others
answers the item exactly when the others alone do. -/
theorem answered_unrelated (need : bind.Need) (refs : Slice bind.Ref)
    (l₁ l₂ : List bind.Ref) (r : bind.Ref)
    (hrefs : refs.val = l₁ ++ r :: l₂) (h : hitB need r = false) :
    bind.answered need refs ⦃ b => b = answeredB need (l₁ ++ l₂) ⦄ := by
  apply WP.spec_mono (answered_spec need refs)
  intro b hb; rw [hb, hrefs, answeredB_skip need l₁ l₂ r h]

/-! ## `item_under` -/

/-- `item_under` on the model. -/
def itemUnderOf (r : Option checklist.Row) (a : Bool) : checklist.Item :=
  match itemOf r with
  | .Supported => if a then .Supported else .Stale
  | .NoneFound => if a then .NoneFound else .Stale
  | i => i

theorem item_under_spec (r : Option checklist.Row) (a : Bool) :
    bind.item_under r a ⦃ i => i = itemUnderOf r a ⦄ := by
  unfold bind.item_under
  step with item_spec as ⟨ base, hbase ⟩
  subst hbase
  unfold itemUnderOf
  cases h : itemOf r <;> cases a <;> simp

/-- Answered, binding changes nothing: `item_under` is `checklist.item`. -/
theorem item_under_answered (r : Option checklist.Row) :
    bind.item_under r true ⦃ i => i = itemOf r ⦄ := by
  apply WP.spec_mono (item_under_spec r true)
  intro i hi; subst hi
  unfold itemUnderOf
  cases itemOf r <;> simp

theorem item_under_supported_iff (r : Option checklist.Row) (a : Bool) :
    bind.item_under r a ⦃ i => i = .Supported ↔ itemOf r = .Supported ∧ a = true ⦄ := by
  apply WP.spec_mono (item_under_spec r a)
  intro i hi; subst hi
  unfold itemUnderOf
  cases itemOf r <;> cases a <;> simp

theorem item_under_none_found_iff (r : Option checklist.Row) (a : Bool) :
    bind.item_under r a ⦃ i => i = .NoneFound ↔ itemOf r = .NoneFound ∧ a = true ⦄ := by
  apply WP.spec_mono (item_under_spec r a)
  intro i hi; subst hi
  unfold itemUnderOf
  cases itemOf r <;> cases a <;> simp

/-- Binding never widens: whatever `item_under` covers, `checklist.item`
covered already, under the same policy. -/
theorem item_under_narrows (r : Option checklist.Row) (a na nf u : Bool) :
    bind.item_under r a ⦃ i => Checklist.coveredB i na nf u = true →
      Checklist.coveredB (itemOf r) na nf u = true ⦄ := by
  apply WP.spec_mono (item_under_spec r a)
  intro i hi; subst hi
  unfold itemUnderOf
  cases itemOf r <;> cases a <;> simp [Checklist.coveredB]

/-- Unanswered, a supported or none-found row is stale; every other standing
is unchanged. -/
theorem item_under_unanswered (r : Option checklist.Row) :
    bind.item_under r false ⦃ i =>
      (itemOf r = .Supported ∨ itemOf r = .NoneFound → i = .Stale) ∧
      (itemOf r ≠ .Supported ∧ itemOf r ≠ .NoneFound → i = itemOf r) ⦄ := by
  apply WP.spec_mono (item_under_spec r false)
  intro i hi; subst hi
  unfold itemUnderOf
  cases itemOf r <;> simp

/-! ## End to end -/

/-- Relevance soundness: an item stands supported under its need exactly
when its newest row is current, supported, cites at least one reference and
every reference is grounded (as before binding), and some cited reference is
grounded and answers the item. -/
theorem bound_supported_iff (need : bind.Need) (refs : Slice bind.Ref)
    (r : Option checklist.Row) :
    (do let a ← bind.answered need refs; bind.item_under r a) ⦃ i => i = .Supported ↔
      (∃ row, r = some row ∧ row.current = true ∧ row.outcome = .Supported ∧
        0 < row.evidence.val ∧ row.grounded = true) ∧
      ∃ x ∈ refs.val, x.grounded = true ∧ answersB need x.bound = true ⦄ := by
  step with answered_iff as ⟨ a, ha ⟩
  apply WP.spec_mono (item_under_supported_iff r a)
  intro i hi
  rw [hi, ← ha]
  have hsup : itemOf r = .Supported ↔ ∃ row, r = some row ∧ row.current = true ∧
      row.outcome = .Supported ∧ 0 < row.evidence.val ∧ row.grounded = true := by
    rcases r with _ | ⟨o, cur, ev, g⟩
    · simp [itemOf]
    · cases cur
      · simp [itemOf]
      · cases o <;> by_cases hev : 0 < ev.val <;> cases g <;> simp [itemOf, hev]
  rw [hsup]

attribute [local step] answers_spec in
/-- `answered_from` never fails, from any start index (tauten's total shape,
docs/writfold/tauten-adoption.md): the loop reads only in-bounds positions
and its increment cannot overflow. -/
theorem answered_from_total (need : bind.Need) (refs : Slice bind.Ref) (index : Usize) :
    bind.answered_from need refs index ⦃ _ => True ⦄ := by
  unfold bind.answered_from bind.answered_from_loop
  apply loop.spec_decr_nat (fun i : Usize => refs.val.length - i.val) (fun _ => True)
  · intro i _
    unfold bind.answered_from_loop.body
    step*
  · trivial

end Writfold.Core.Bind
