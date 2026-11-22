import Extracted.WritfoldCore.Checklist
import Extracted.WritfoldCore.Gate
import Extracted.WritfoldCore.Types
import Writfold.Current

/-!
# Checklist gates

Theorems over the Aeneas translation of `writfold_core::checklist`, the
kernel behind `writfold kb gate cover` and a checklist gate's standing.

- `item_spec`, `item_supported_iff`, `item_none_found_iff`: an item is
  `supported` (or `none-found`) exactly when its newest row is about the
  present, says so, and cites at least one reference that is still grounded.
- `covered_under_spec`, `not_applicable_rejected_never_covers`,
  `item_covered_iff`: an item is covered under its policy exactly when it is
  supported, not applicable where the policy accepts that, or none found
  where the policy accepts that; a row about other content, an unsupported
  row, a row that cites nothing or whose evidence moved never covers.
- `rollup_under_spec`, `rollup_covered_iff`: a checklist is covered exactly
  when every required item is covered under its policy; `first_owed` is the
  first required item not covered; it is stale exactly when a required item
  is stale. Optional items change none of these
  (`rollup_optional_irrelevant`).
- `standing_spec`, `standing_attested_iff`, `standing_never_satisfied`,
  `standing_met_iff`: through the gate kernel a checklist stands
  `attested` exactly when it is covered, never `satisfied` (a cover row is
  a judgement, not a passing check), and is met exactly when it is covered
  or a current waiver stands. `checklist_met_iff` composes them.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Checklist

/-! ## `item` and `covered_under` -/

/-- `item` on the model: the standing of one item from its newest row. -/
def itemOf : Option checklist.Row → checklist.Item
  | none => .Open
  | some r =>
    if r.current = false then .Stale else
    match r.outcome with
    | .Supported => if 0 < r.evidence.val ∧ r.grounded = true then .Supported else .Stale
    | .Unsupported => .Unsupported
    | .NotApplicable => .NotApplicable
    | .NoneFound => if 0 < r.evidence.val ∧ r.grounded = true then .NoneFound else .Stale

theorem item_spec (r : Option checklist.Row) : checklist.item r ⦃ i => i = itemOf r ⦄ := by
  unfold checklist.item
  rcases r with _ | ⟨o, cur, ev, g⟩
  · simp [itemOf]
  · cases cur
    · simp [itemOf]
    · cases o
      · simp only [itemOf]
        by_cases hev : 0 < ev.val
        · have : ev > 0#usize := by scalar_tac
          cases g <;> simp [this, hev]
        · have : ¬ ev > 0#usize := by scalar_tac
          simp [this, hev]
      · simp [itemOf]
      · simp [itemOf]
      · simp only [itemOf]
        by_cases hev : 0 < ev.val
        · have : ev > 0#usize := by scalar_tac
          cases g <;> simp [this, hev]
        · have : ¬ ev > 0#usize := by scalar_tac
          simp [this, hev]

/-- Supported exactly when the newest row is current, supported, and cites at
least one reference that is still grounded. -/
theorem item_supported_iff (r : Option checklist.Row) :
    checklist.item r ⦃ i => i = .Supported ↔ ∃ row, r = some row ∧ row.current = true ∧
      row.outcome = .Supported ∧ 0 < row.evidence.val ∧ row.grounded = true ⦄ := by
  apply WP.spec_mono (item_spec r)
  intro i hi
  subst hi
  rcases r with _ | ⟨o, cur, ev, g⟩
  · simp [itemOf]
  · cases cur
    · simp [itemOf]
    · cases o <;> by_cases hev : 0 < ev.val <;> cases g <;> simp [itemOf, hev]

/-- None found exactly when the newest row is current, says none-found, and
cites at least one look that still reads as having found nothing. -/
theorem item_none_found_iff (r : Option checklist.Row) :
    checklist.item r ⦃ i => i = .NoneFound ↔ ∃ row, r = some row ∧ row.current = true ∧
      row.outcome = .NoneFound ∧ 0 < row.evidence.val ∧ row.grounded = true ⦄ := by
  -- From the code of `item` itself, not through `item_spec`.
  unfold checklist.item
  rcases r with _ | ⟨o, cur, ev, g⟩
  · simp
  · cases cur
    · simp
    · cases o <;> by_cases hev : 0 < ev.val <;> cases g <;> simp [hev]

/-- `covered_under` on the model: supported; not applicable where the policy
accepts it (`na`); none found where it accepts that (`none`). -/
def coveredB : checklist.Item → Bool → Bool → Bool → Bool
  | .Supported, _, _, u => !u
  | .NotApplicable, na, _, _ => na
  | .NoneFound, _, nf, _ => nf
  | _, _, _, _ => false

/-- An entry is covered under its own policy. -/
def coveredE (e : checklist.Entry) : Bool := coveredB e.item e.na e.none e.unsnapped

theorem covered_under_spec (i : checklist.Item) (na nf u : Bool) :
    checklist.covered_under i na nf u ⦃ b => b = true ↔
      (i = .Supported ∧ u = false) ∨ (i = .NotApplicable ∧ na = true) ∨
        (i = .NoneFound ∧ nf = true) ⦄ := by
  unfold checklist.covered_under
  cases i <;> cases u <;> simp

theorem covered_under_coveredB (i : checklist.Item) (na nf u : Bool) :
    checklist.covered_under i na nf u ⦃ b => b = coveredB i na nf u ⦄ := by
  apply WP.spec_mono (covered_under_spec i na nf u)
  intro b hb
  cases i <;> cases na <;> cases nf <;> cases u <;> cases b <;> simp_all [coveredB]

/-- An item the checklist's `snapshot` list names, whose row cites no look
that kept a snapshot, is not covered by `supported`. -/
theorem snapshot_item_needs_snapshot (na nf : Bool) :
    checklist.covered_under .Supported na nf true ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (covered_under_spec .Supported na nf true)
  intro b hb
  cases b <;> simp_all

/-- An item the `snapshot` list does not name is covered exactly as before
the list existed. -/
theorem unlisted_item_unchanged (i : checklist.Item) (na nf : Bool) :
    checklist.covered_under i na nf false ⦃ b => b = true ↔
      i = .Supported ∨ (i = .NotApplicable ∧ na = true) ∨ (i = .NoneFound ∧ nf = true) ⦄ := by
  apply WP.spec_mono (covered_under_spec i na nf false)
  intro b hb
  rw [hb]
  simp

/-- An item whose policy rejects not-applicable is never covered by it. -/
theorem not_applicable_rejected_never_covers (nf u : Bool) :
    checklist.covered_under .NotApplicable false nf u ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (covered_under_spec .NotApplicable false nf u)
  intro b hb
  cases b <;> simp_all

/-- An item is covered under its policy exactly when its newest row is about
the present and is supported by at least one grounded reference, or not
applicable where the policy accepts it, or none found (citing at least one
look that still found nothing) where the policy accepts it. -/
theorem item_covered_iff (r : Option checklist.Row) (na nf u : Bool) :
    (do let i ← checklist.item r; checklist.covered_under i na nf u) ⦃ b => b = true ↔
      ∃ row, r = some row ∧ row.current = true ∧
        ((row.outcome = .Supported ∧ 0 < row.evidence.val ∧ row.grounded = true ∧ u = false) ∨
          (row.outcome = .NotApplicable ∧ na = true) ∨
          (row.outcome = .NoneFound ∧ 0 < row.evidence.val ∧ row.grounded = true ∧
            nf = true)) ⦄ := by
  step with item_spec as ⟨ i, hi ⟩
  apply WP.spec_mono (covered_under_coveredB i na nf u)
  intro b hb
  subst hb hi
  rcases r with _ | ⟨o, cur, ev, g⟩
  · simp [itemOf, coveredB]
  · cases cur
    · simp [itemOf, coveredB]
    · cases o <;> by_cases hev : 0 < ev.val <;> cases g <;> cases na <;> cases nf <;>
        cases u <;> simp [itemOf, coveredB, hev]

/-! ## `owes`, `first_owed`, `any_stale`, `rollup` -/

/-- An entry is owed: required and not covered under its policy. -/
def owesB (e : checklist.Entry) : Bool := e.required && !coveredE e

theorem owes_spec (e : checklist.Entry) : checklist.owes e ⦃ b => b = owesB e ⦄ := by
  unfold checklist.owes owesB coveredE
  rcases e with ⟨i, req, na, nf, u⟩
  cases req
  · simp
  · simp only [↓reduceIte]
    step with covered_under_coveredB as ⟨ b, hb ⟩
    simp [hb]

/-- The least entry that is owed. -/
def firstOwedL (es : List checklist.Entry) : Option Nat :=
  Plan.leastFailing (fun e => !owesB e) es

theorem first_owed_loop_spec (es : Slice checklist.Entry) (i : Usize)
    (hi : i.val ≤ es.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < es.val.length), owesB es.val[j] = false) :
    checklist.first_owed_loop es i ⦃ r => r.map (·.val) = firstOwedL es.val ⦄ := by
  unfold checklist.first_owed_loop
  apply loop.spec_decr_nat (fun i : Usize => es.val.length - i.val)
    (fun i : Usize => i.val ≤ es.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < es.val.length), owesB es.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold checklist.first_owed_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with owes_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, Option.map_some]
        symm
        apply Plan.leastFailing_of_prefix _ es.val i.val (by scalar_tac)
        · rw [← hx]; simp [← hb, hbt]
        · intro j hj; simp [hprefix j hj _]
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx, ← hb]
          simpa using hbt
    · rename_i hge
      simp only [WP.spec_ok, Option.map_none]
      symm
      apply Plan.leastFailing_of_all
      intro j hj
      simp [hprefix j (by scalar_tac) hj]
  · exact ⟨hi, hprefix⟩

theorem first_owed_spec (es : Slice checklist.Entry) :
    checklist.first_owed es ⦃ r => r.map (·.val) = firstOwedL es.val ⦄ := by
  unfold checklist.first_owed
  apply first_owed_loop_spec
  · simp
  · intro j hj; simp at hj

/-- A required entry is stale. -/
def staleB (e : checklist.Entry) : Bool :=
  e.required && (match e.item with | .Stale => true | _ => false)

theorem stale_spec (e : checklist.Entry) : checklist.stale e ⦃ b => b = staleB e ⦄ := by
  unfold checklist.stale staleB
  rcases e with ⟨it, req, na, nf⟩
  cases req <;> cases it <;> simp

theorem any_stale_loop_spec (es : Slice checklist.Entry) (i : Usize)
    (hi : i.val ≤ es.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < es.val.length), staleB es.val[j] = false) :
    checklist.any_stale_loop es i ⦃ b => b = true ↔ ∃ e ∈ es.val, staleB e = true ⦄ := by
  unfold checklist.any_stale_loop
  apply loop.spec_decr_nat (fun i : Usize => es.val.length - i.val)
    (fun i : Usize => i.val ≤ es.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < es.val.length), staleB es.val[j] = false)
  · intro i ⟨hi, hprefix⟩
    unfold checklist.any_stale_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with stale_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        exact ⟨es.val[i.val]'(by scalar_tac), List.getElem_mem _, by rw [← hx, ← hb, hbt]⟩
      · rename_i hbt
        step as ⟨ i', hi' ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hprefix j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hx, ← hb]; simpa using hbt
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro e he hse
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
      have := hprefix j (by scalar_tac) hj
      simp_all
  · exact ⟨hi, hprefix⟩

theorem any_stale_spec (es : Slice checklist.Entry) :
    checklist.any_stale es ⦃ b => b = true ↔
      ∃ e ∈ es.val, e.required = true ∧ e.item = .Stale ⦄ := by
  unfold checklist.any_stale
  apply WP.spec_mono (any_stale_loop_spec es 0#usize (by simp) (by intro j hj; simp at hj))
  intro b hb
  rw [hb]
  constructor
  · rintro ⟨e, he, hs⟩
    refine ⟨e, he, ?_⟩
    rcases e with ⟨it, req, na, nf⟩
    cases req <;> cases it <;> simp_all [staleB]
  · rintro ⟨e, he, hr, hi⟩
    exact ⟨e, he, by simp [staleB, hr, hi]⟩

theorem firstOwedL_none_iff (es : List checklist.Entry) :
    firstOwedL es = none ↔ ∀ e ∈ es, e.required = true → coveredE e = true := by
  unfold firstOwedL
  rw [Plan.leastFailing_none_iff]
  apply forall_congr'; intro e
  apply forall_congr'; intro _
  unfold owesB
  cases e.required <;> cases coveredE e <;> simp

/-- The roll-up under each item's policy: covered exactly when every required
item is covered under its policy; the first owed item is the first required
item not covered; stale exactly when a required item is stale. -/
theorem rollup_under_spec (es : Slice checklist.Entry) :
    checklist.rollup_under es ⦃ r =>
      (r.covered = true ↔ ∀ e ∈ es.val, e.required = true → coveredE e = true) ∧
      r.first_owed.map (·.val) = Plan.leastFailing (fun e => !e.required || coveredE e) es.val ∧
      (r.stale = true ↔ ∃ e ∈ es.val, e.required = true ∧ e.item = .Stale) ⦄ := by
  have hfl : firstOwedL es.val = Plan.leastFailing (fun e => !e.required || coveredE e) es.val := by
    unfold firstOwedL owesB
    congr 1
    funext e
    cases e.required <;> cases coveredE e <;> rfl
  rw [← hfl]
  unfold checklist.rollup_under
  step with first_owed_spec as ⟨ o, ho ⟩
  step with any_stale_spec as ⟨ s, hs ⟩
  refine ⟨?_, ho, hs⟩
  rw [← firstOwedL_none_iff, ← ho]
  cases o <;> simp

/-- A checklist is covered exactly when every required item is covered under
its policy. -/
theorem rollup_covered_iff (es : Slice checklist.Entry) :
    checklist.rollup_under es ⦃ r =>
      r.covered = true ↔ ∀ e ∈ es.val, e.required = true → coveredE e = true ⦄ :=
  WP.spec_mono (rollup_under_spec es) (fun _ h => h.1)

/-- Only required items count: two checklists whose entries agree on which
are required, and on the items and policies of the required ones, roll up
alike in `covered` and `stale` (and owe the same first entry). -/
theorem rollup_optional_irrelevant (es fs : Slice checklist.Entry)
    (hlen : es.val.length = fs.val.length)
    (hreq : ∀ j (h1 : j < es.val.length) (h2 : j < fs.val.length),
      es.val[j].required = fs.val[j].required ∧
      (es.val[j].required = true → es.val[j].item = fs.val[j].item ∧
        es.val[j].na = fs.val[j].na ∧ es.val[j].none = fs.val[j].none ∧
        es.val[j].unsnapped = fs.val[j].unsnapped)) :
    (do let a ← checklist.rollup_under es; let b ← checklist.rollup_under fs; ok (a, b)) ⦃ p =>
      p.1.covered = p.2.covered ∧ p.1.stale = p.2.stale ∧ p.1.first_owed = p.2.first_owed ⦄ := by
  have hfl : ∀ l : List checklist.Entry,
      Plan.leastFailing (fun e => !e.required || coveredE e) l = firstOwedL l := by
    intro l
    unfold firstOwedL owesB
    congr 1
    funext e
    cases e.required <;> cases coveredE e <;> rfl
  step with rollup_under_spec as ⟨ a, ha1, ha2, ha3 ⟩
  step with rollup_under_spec as ⟨ b, hb1, hb2, hb3 ⟩
  rw [hfl] at ha2 hb2
  have key : ∀ j (h1 : j < es.val.length) (h2 : j < fs.val.length),
      owesB es.val[j] = owesB fs.val[j] ∧ staleB es.val[j] = staleB fs.val[j] := by
    intro j h1 h2
    obtain ⟨hr, hi⟩ := hreq j h1 h2
    unfold owesB staleB coveredE
    cases hreq' : es.val[j].required
    · simp_all
    · obtain ⟨h1, h2, h3, h4⟩ := hi hreq'; rw [h1, h2, h3, h4]; simp_all
  have hfirst : firstOwedL es.val = firstOwedL fs.val := by
    unfold firstOwedL
    apply Option.ext
    intro i
    rw [Plan.leastFailing_some_iff, Plan.leastFailing_some_iff]
    constructor
    · rintro ⟨h, hf, hall⟩
      refine ⟨by omega, by rw [← (key i h (by omega)).1]; exact hf, ?_⟩
      intro j hj
      rw [← (key j (by omega) (by omega)).1]; exact hall j hj
    · rintro ⟨h, hf, hall⟩
      refine ⟨by omega, by rw [(key i (by omega) h).1]; exact hf, ?_⟩
      intro j hj
      rw [(key j (by omega) (by omega)).1]; exact hall j hj
  have hcov : a.covered = b.covered := by
    cases hac : a.covered <;> cases hbc : b.covered <;> try rfl
    all_goals
      first
      | (have := ha1.mp hac; rw [← firstOwedL_none_iff, hfirst, firstOwedL_none_iff] at this
         exact absurd (hb1.mpr this) (by simp [hbc]))
      | (have := hb1.mp hbc; rw [← firstOwedL_none_iff, ← hfirst, firstOwedL_none_iff] at this
         exact absurd (ha1.mpr this) (by simp [hac]))
  have hstale : a.stale = b.stale := by
    have hiff : (∃ e ∈ es.val, e.required = true ∧ e.item = .Stale) ↔
        (∃ e ∈ fs.val, e.required = true ∧ e.item = .Stale) := by
      constructor
      · rintro ⟨e, he, hr, hi⟩
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
        obtain ⟨hreqj, hitj⟩ := hreq j hj (by omega)
        exact ⟨fs.val[j], List.getElem_mem _, by rw [← hreqj]; exact hr,
          by rw [← (hitj hr).1]; exact hi⟩
      · rintro ⟨e, he, hr, hi⟩
        obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem he
        obtain ⟨hreqj, hitj⟩ := hreq j (by omega) hj
        exact ⟨es.val[j], List.getElem_mem _, by rw [hreqj]; exact hr,
          by rw [(hitj (by rw [hreqj]; exact hr)).1]; exact hi⟩
    cases has : a.stale <;> cases hbs : b.stale <;> try rfl
    · exact absurd (hb3.mp hbs |> hiff.mpr |> ha3.mpr) (by simp [has])
    · exact absurd (ha3.mp has |> hiff.mp |> hb3.mpr) (by simp [hbs])
  refine ⟨hcov, hstale, ?_⟩
  have h2 : a.first_owed.map (·.val) = b.first_owed.map (·.val) := by rw [ha2, hb2, hfirst]
  cases ha : a.first_owed <;> cases hb : b.first_owed <;> rw [ha, hb] at h2 <;> simp at h2 ⊢
  exact UScalar.eq_of_val_eq h2

/-! ## `state` and `standing` -/

theorem state_spec (r : checklist.Rollup) :
    checklist.state r ⦃ s => s = if r.stale then .Stale else .Open ⦄ := by
  unfold checklist.state
  cases r.stale <;> simp

/-- A checklist's standing is the gate kernel's for an attested-class
obligation whose current attestation is "covered" and whose earlier
attestation is "stale". -/
theorem standing_spec (r : checklist.Rollup) (w : Bool) :
    checklist.standing r w ⦃ s => s =
      Gate.standingOf true (if r.stale then .Stale else .Open) ⟨r.covered, r.stale, w⟩ ⦄ := by
  unfold checklist.standing
  step with state_spec as ⟨ s, hs ⟩
  apply WP.spec_mono (Gate.standing_spec true s _)
  intro x hx
  rw [hx, hs]

/-- `attested` exactly when every required item is covered. -/
theorem standing_attested_iff (r : checklist.Rollup) (w : Bool) :
    checklist.standing r w ⦃ s => s = .Attested ↔ r.covered = true ⦄ := by
  apply WP.spec_mono (standing_spec r w)
  intro s hs
  subst hs
  rcases r with ⟨c, st, f⟩
  cases c <;> cases st <;> cases w <;> simp [Gate.standingOf]

/-- A cover row is a judgement, never a passing check. -/
theorem standing_never_satisfied (r : checklist.Rollup) (w : Bool) :
    checklist.standing r w ⦃ s => s ≠ .Checked .Satisfied ⦄ := by
  apply WP.spec_mono (standing_spec r w)
  intro s hs
  subst hs
  rcases r with ⟨c, st, f⟩
  cases c <;> cases st <;> cases w <;> simp [Gate.standingOf]

/-- Met exactly when covered, or a current waiver stands. -/
theorem standing_met_iff (r : checklist.Rollup) (w : Bool) :
    (do let s ← checklist.standing r w; gate.Standing.met s) ⦃ b =>
      b = true ↔ r.covered = true ∨ w = true ⦄ := by
  step with standing_spec as ⟨ s, hs ⟩
  apply WP.spec_mono (Gate.met_spec s)
  intro b hb
  rw [hb, hs]
  rcases r with ⟨c, st, f⟩
  cases c <;> cases st <;> cases w <;> simp [Gate.standingOf]

/-- Composed: a checklist gate is met exactly when every required item is
covered under its policy (by a current row: supported with grounded
evidence, not applicable where accepted, or none found where accepted), or a
current waiver stands. -/
theorem checklist_met_iff (es : Slice checklist.Entry) (w : Bool) :
    (do let r ← checklist.rollup_under es
        let s ← checklist.standing r w
        gate.Standing.met s) ⦃ b => b = true ↔
      (∀ e ∈ es.val, e.required = true → coveredE e = true) ∨ w = true ⦄ := by
  step with rollup_covered_iff as ⟨ r, hr ⟩
  apply WP.spec_mono (standing_met_iff r w)
  intro b hb
  rw [hb, hr]

end Writfold.Core.Checklist
