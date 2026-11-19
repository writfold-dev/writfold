import Writfold.Verdict

/-!
# Pins

Theorems over the Aeneas translation of `writfold_core::pin`: evidence holds
while every pin it was taken against holds.

- `holds_spec`, `holds_nil`, `unheld_never_holds`: evidence holds exactly
  when every pin is `Held`. No pins hold (today's evidence is unchanged),
  and one moved, aged or unreadable pin is enough to stop it holding.
- `timed_spec`, `timed_untimed`, `timed_aged_iff`, `timed_never_moved`: a
  time pin from the kind catalog ages exactly when `now - seen > ttl` over
  the integers (the `verdict::expired` rule citations age by); an immutable
  kind or no lifetime never ages.
- `nothing_found_spec`, `nothing_found_refines`: a recorded look still reads
  as having found nothing exactly when it was `no-data` or `not-found` and
  every pin holds; with no pins this is the status test alone.
- `current_under_spec`, `current_under_refines`, `current_under_about`:
  pins only take standing away: a receipt is current under its pins exactly
  when it is about the present bytes and every pin holds.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Pin

/-- A pin holds. -/
def heldB : pin.Pin → Bool
  | .Held => true
  | _ => false

theorem heldB_iff (p : pin.Pin) : heldB p = true ↔ p = .Held := by
  cases p <;> simp [heldB]

theorem holds_from_loop_spec (pins : Slice pin.Pin) (i : Usize)
    (hi : i.val ≤ pins.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < pins.val.length), heldB pins.val[j] = true) :
    pin.holds_from_loop pins i ⦃ b => b = true ↔ ∀ p ∈ pins.val, heldB p = true ⦄ := by
  unfold pin.holds_from_loop
  apply loop.spec_decr_nat (fun i : Usize => pins.val.length - i.val)
    (fun i : Usize => i.val ≤ pins.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < pins.val.length), heldB pins.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold pin.holds_from_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hmem : pins.val[i.val]'(by scalar_tac) ∈ pins.val := List.getElem_mem _
      cases x
      · step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx]; rfl
      all_goals
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
        exact ⟨_, hmem, by rw [← hx]; simp [heldB]⟩
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro p hp
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hp
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- Evidence holds exactly when every pin holds. -/
theorem holds_spec (pins : Slice pin.Pin) :
    pin.holds pins ⦃ b => b = true ↔ ∀ p ∈ pins.val, p = .Held ⦄ := by
  unfold pin.holds pin.holds_from
  apply WP.spec_mono (holds_from_loop_spec pins 0#usize (by simp)
    (by intro j hj; simp at hj))
  intro b hb
  rw [hb]
  simp [heldB_iff]

/-- No pins: the evidence holds, as it did before pins existed. -/
theorem holds_nil (pins : Slice pin.Pin) (h : pins.val = []) :
    pin.holds pins ⦃ b => b = true ⦄ := by
  apply WP.spec_mono (holds_spec pins)
  intro b hb; rw [hb, h]; simp

/-- One pin that moved, aged or cannot be read is enough. -/
theorem unheld_never_holds (pins : Slice pin.Pin) (p : pin.Pin)
    (hp : p ∈ pins.val) (hne : p ≠ .Held) :
    pin.holds pins ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (holds_spec pins)
  intro b hb
  cases b
  · rfl
  · exact absurd (hb.mp rfl p hp) hne

/-! ## `timed` -/

/-- `timed` on the model. -/
def timedOf (life : verdict.Lifetime I64) (seen now : I64) : pin.Pin :=
  match life with
  | .Immutable => .Held
  | .Never => .Held
  | .Token ttl => if now.val - seen.val > ttl.val then .Aged else .Held
  | .Default ttl => if now.val - seen.val > ttl.val then .Aged else .Held

theorem timed_spec (life : verdict.Lifetime I64) (seen now : I64) :
    pin.timed life seen now ⦃ p => p = timedOf life seen now ⦄ := by
  unfold pin.timed
  cases life
  · simp [timedOf]
  · rename_i ttl
    step with Verdict.expired_spec as ⟨ b, hb ⟩
    unfold timedOf
    cases b <;> simp_all
  · rename_i ttl
    step with Verdict.expired_spec as ⟨ b, hb ⟩
    unfold timedOf
    cases b <;> simp_all
  · simp [timedOf]

/-- An immutable kind, or one with no lifetime, never ages. -/
theorem timed_untimed (seen now : I64) :
    pin.timed .Immutable seen now ⦃ p => p = .Held ⦄ ∧
    pin.timed .Never seen now ⦃ p => p = .Held ⦄ := by
  constructor
  · apply WP.spec_mono (timed_spec .Immutable seen now); intro p hp; simp [hp, timedOf]
  · apply WP.spec_mono (timed_spec .Never seen now); intro p hp; simp [hp, timedOf]

/-- A lifetime from the token or the catalog ages exactly when more than its
`ttl` seconds have passed since `seen`, over the integers. -/
theorem timed_aged_iff (ttl seen now : I64) :
    pin.timed (.Token ttl) seen now ⦃ p => p = .Aged ↔ now.val - seen.val > ttl.val ⦄ ∧
    pin.timed (.Default ttl) seen now ⦃ p => p = .Aged ↔ now.val - seen.val > ttl.val ⦄ := by
  constructor
  · apply WP.spec_mono (timed_spec (.Token ttl) seen now)
    intro p hp; subst hp; unfold timedOf; split <;> simp_all <;> omega
  · apply WP.spec_mono (timed_spec (.Default ttl) seen now)
    intro p hp; subst hp; unfold timedOf; split <;> simp_all <;> omega

/-- Time only ages: it never reads as moved or unreadable. -/
theorem timed_never_moved (life : verdict.Lifetime I64) (seen now : I64) :
    pin.timed life seen now ⦃ p => p = .Held ∨ p = .Aged ⦄ := by
  apply WP.spec_mono (timed_spec life seen now)
  intro p hp; subst hp; unfold timedOf
  cases life <;> simp <;> omega

/-! ## `nothing_found` -/

theorem nothing_found_spec (status : observe.Status) (pins : Slice pin.Pin) :
    pin.nothing_found status pins ⦃ b => b = true ↔
      (status = .NoData ∨ status = .NotFound) ∧ ∀ p ∈ pins.val, p = .Held ⦄ := by
  unfold pin.nothing_found
  cases status <;> simp
  all_goals exact holds_spec pins

/-- With no pins a look reads as having found nothing exactly when its
status says so: what `none-found` rows stood on before pins existed. -/
theorem nothing_found_refines (status : observe.Status) (pins : Slice pin.Pin)
    (h : pins.val = []) :
    pin.nothing_found status pins ⦃ b => b = true ↔ status = .NoData ∨ status = .NotFound ⦄ := by
  apply WP.spec_mono (nothing_found_spec status pins)
  intro b hb; rw [hb, h]; simp

/-! ## `current_under` -/

theorem current_under_spec (about : Bool) (pins : Slice pin.Pin) :
    pin.current_under about pins ⦃ b => b = true ↔
      about = true ∧ ∀ p ∈ pins.val, p = .Held ⦄ := by
  unfold pin.current_under
  cases about
  · simp
  · apply WP.spec_mono (holds_spec pins); intro b hb; simp [hb]

/-- Pins only take standing away: current under pins implies about the
present bytes. -/
theorem current_under_about (about : Bool) (pins : Slice pin.Pin) :
    pin.current_under about pins ⦃ b => b = true → about = true ⦄ := by
  apply WP.spec_mono (current_under_spec about pins)
  intro b hb h; exact (hb.mp h).1

/-- With no pins, currency is aboutness: today's receipts are unchanged. -/
theorem current_under_refines (about : Bool) (pins : Slice pin.Pin) (h : pins.val = []) :
    pin.current_under about pins ⦃ b => b = about ⦄ := by
  apply WP.spec_mono (current_under_spec about pins)
  intro b hb
  rw [h] at hb
  cases b <;> cases about <;> simp_all

/-- `holds_from` never fails, from any start index (tauten's total shape,
docs/writfold/tauten-adoption.md): the loop reads only in-bounds positions
and its increment cannot overflow. -/
theorem holds_from_total (pins : Slice pin.Pin) (index : Usize) :
    pin.holds_from pins index ⦃ _ => True ⦄ := by
  unfold pin.holds_from pin.holds_from_loop
  apply loop.spec_decr_nat (fun i : Usize => pins.val.length - i.val) (fun _ => True)
  · intro i _
    unfold pin.holds_from_loop.body
    step*
  · trivial

end Writfold.Core.Pin
