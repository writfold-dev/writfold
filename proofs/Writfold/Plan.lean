import Writfold.Bytes

/-!
# The run graph

Theorems over the Aeneas translation of `writfold_core::plan`, the kernel
behind `writfold kb plan`. The caller folds a run's plan events into one
ordered list of steps; these are the decisions on that list.

- `current` is the least index whose step is neither done nor skipped, and
  `none` exactly when every step is closed (`current_spec`, `current_least`).
- `advance_check` admits exactly the current step with every gate met, and
  otherwise refuses in the fixed order: complete, not current, the first
  owed gate (`advance_check_spec`, `advance_ok_iff`).
- `stale` is a done step with an owed gate; it is closed, so it is never the
  current step, and `current` reads no gate at all (`stale_spec`,
  `stale_never_current`).
- `splice` keeps every existing step, in order and with its state, inserts
  open steps at `at`, and so keeps the done prefix; inserted before the
  current step, the first new step becomes current (`splice_spec`,
  `splice_keeps_existing`, `splice_keeps_prefix`, `splice_new_open`,
  `splice_current`).
- `splice_check` is `Already` exactly when the key is already spliced, so a
  splice is idempotent on its key; it admits a splice only within the depth
  bound and only before the current step (`splice_check_spec`,
  `splice_check_already_iff`, `splice_idempotent`, `splice_check_depth`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Plan

/-! ## The least failing index -/

/-- The least index whose element fails `p`; `none` when every element
satisfies it. -/
def leastFailing {α : Type} (p : α → Bool) : List α → Option Nat
  | [] => none
  | x :: l => if p x then (leastFailing p l).map (· + 1) else some 0

theorem leastFailing_none_iff {α : Type} (p : α → Bool) (l : List α) :
    leastFailing p l = none ↔ ∀ x ∈ l, p x = true := by
  induction l with
  | nil => simp [leastFailing]
  | cons x l ih =>
    cases hx : p x <;> simp [leastFailing, hx, ih]

theorem leastFailing_some_iff {α : Type} (p : α → Bool) (l : List α) (i : Nat) :
    leastFailing p l = some i ↔
      ∃ h : i < l.length, p l[i] = false ∧
        ∀ j (hj : j < i), p (l[j]'(Nat.lt_trans hj h)) = true := by
  induction l generalizing i with
  | nil => simp [leastFailing]
  | cons x l ih =>
    cases hx : p x
    · simp only [leastFailing, hx, Bool.false_eq_true, ↓reduceIte, Option.some.injEq]
      constructor
      · rintro rfl
        exact ⟨by simp, by simpa using hx, fun j hj => absurd hj (Nat.not_lt_zero _)⟩
      · rintro ⟨_, _, hall⟩
        cases i with
        | zero => rfl
        | succ i =>
          have := hall 0 (by omega)
          simp [hx] at this
    · simp only [leastFailing, hx, ↓reduceIte]
      cases i with
      | zero =>
        simp only [Option.map_eq_some_iff, Nat.add_eq_zero_iff, one_ne_zero, and_false,
          exists_false, false_iff]
        rintro ⟨_, h0, _⟩
        simp [hx] at h0
      | succ i =>
        simp only [Option.map_eq_some_iff, Nat.add_right_cancel_iff, exists_eq_right, ih]
        constructor
        · rintro ⟨h, hf, hall⟩
          refine ⟨by simp; omega, by simpa using hf, ?_⟩
          intro j hj
          cases j with
          | zero => simpa using hx
          | succ j => simpa using hall j (by omega)
        · rintro ⟨h, hf, hall⟩
          refine ⟨by simp at h; omega, by simpa using hf, ?_⟩
          intro j hj
          simpa using hall (j + 1) (by omega)

/-- A generic loop lemma: the `while index < len { if !p(xs[index]) return
Some(index); index += 1 }` shape of `current` and `first_owed`. -/
theorem leastFailing_of_prefix {α : Type} (p : α → Bool) (l : List α) (i : Nat)
    (hi : i < l.length) (hf : p l[i] = false)
    (hprefix : ∀ j (hj : j < i), p (l[j]'(Nat.lt_trans hj hi)) = true) :
    leastFailing p l = some i :=
  (leastFailing_some_iff p l i).mpr ⟨hi, hf, hprefix⟩

theorem leastFailing_of_all {α : Type} (p : α → Bool) (l : List α)
    (hall : ∀ j (hj : j < l.length), p l[j] = true) :
    leastFailing p l = none := by
  rw [leastFailing_none_iff]
  intro x hx
  obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
  exact hall j hj

/-! ## `current` -/

/-- A step is closed when a `done-` or a `skip-` record closes it. -/
def closedB (n : plan.Node) : Bool := n.done || n.skipped

/-- The current step, on the list: the least index that is not closed. -/
def cur (l : List plan.Node) : Option Nat := leastFailing closedB l

theorem closed_spec (n : plan.Node) : plan.closed n ⦃ b => b = closedB n ⦄ := by
  unfold plan.closed closedB
  cases n with
  | mk d s => cases d <;> simp

theorem current_loop_spec (nodes : Slice plan.Node) (i : Usize)
    (hi : i.val ≤ nodes.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < nodes.val.length), closedB nodes.val[j] = true) :
    plan.current_loop nodes i ⦃ r => r.map (·.val) = cur nodes.val ⦄ := by
  unfold plan.current_loop
  apply loop.spec_decr_nat (fun i : Usize => nodes.val.length - i.val)
    (fun i : Usize => i.val ≤ nodes.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < nodes.val.length), closedB nodes.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold plan.current_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with closed_spec as ⟨ b, hb ⟩
      split
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
          rw [← hx, ← hb, hbt]
      · rename_i hbt
        simp only [WP.spec_ok, Option.map_some]
        symm
        apply leastFailing_of_prefix closedB nodes.val i.val (by scalar_tac)
        · rw [← hx, ← hb]; simpa using hbt
        · intro j hj; exact hprefix j hj _
    · rename_i hge
      simp only [WP.spec_ok, Option.map_none]
      symm
      apply leastFailing_of_all
      intro j hj
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- `current` is the least step that is neither done nor skipped. -/
theorem current_spec (nodes : Slice plan.Node) :
    plan.current nodes ⦃ r => r.map (·.val) = cur nodes.val ⦄ := by
  unfold plan.current
  apply current_loop_spec
  · simp
  · intro j hj; simp at hj

/-- The headline form: `some i` names a step that is not closed and every
earlier step is closed; `none` means every step is closed. -/
theorem current_least (nodes : Slice plan.Node) :
    plan.current nodes ⦃ r => match r with
      | some i => ∃ h : i.val < nodes.val.length, closedB nodes.val[i.val] = false ∧
          ∀ j (hj : j < i.val), closedB (nodes.val[j]'(Nat.lt_trans hj h)) = true
      | none => ∀ n ∈ nodes.val, closedB n = true ⦄ := by
  apply WP.spec_mono (current_spec nodes)
  intro r hr
  cases r with
  | none => exact (leastFailing_none_iff _ _).mp (by simpa [cur] using hr.symm)
  | some i => exact (leastFailing_some_iff _ _ _).mp (by simpa [cur] using hr.symm)

/-! ## `first_owed` -/

/-- The first gate that is not met, on the list. -/
def firstOwed (met : List Bool) : Option Nat := leastFailing id met

theorem first_owed_loop_spec (met : Slice Bool) (i : Usize)
    (hi : i.val ≤ met.val.length)
    (hprefix : ∀ j (hj : j < i.val) (hj' : j < met.val.length), met.val[j] = true) :
    plan.first_owed_loop met i ⦃ r => r.map (·.val) = firstOwed met.val ⦄ := by
  unfold plan.first_owed_loop
  apply loop.spec_decr_nat (fun i : Usize => met.val.length - i.val)
    (fun i : Usize => i.val ≤ met.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < met.val.length), met.val[j] = true)
  · intro i ⟨hi, hprefix⟩
    unfold plan.first_owed_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      split
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
          rw [← hx, hbt]
      · rename_i hbt
        simp only [WP.spec_ok, Option.map_some]
        symm
        apply leastFailing_of_prefix id met.val i.val (by scalar_tac)
        · rw [← hx]; simpa using hbt
        · intro j hj; exact hprefix j hj _
    · rename_i hge
      simp only [WP.spec_ok, Option.map_none]
      symm
      apply leastFailing_of_all
      intro j hj
      exact hprefix j (by scalar_tac) hj
  · exact ⟨hi, hprefix⟩

/-- `first_owed` is the first gate that is not met, in list order. -/
theorem first_owed_spec (met : Slice Bool) :
    plan.first_owed met ⦃ r => r.map (·.val) = firstOwed met.val ⦄ := by
  unfold plan.first_owed
  apply first_owed_loop_spec
  · simp
  · intro j hj; simp at hj

theorem firstOwed_none_iff (met : List Bool) : firstOwed met = none ↔ ∀ b ∈ met, b = true := by
  unfold firstOwed
  rw [leastFailing_none_iff]
  simp

/-! ## `advance_check` -/

/-- `advance_check`'s decision, refusal by refusal, in the fixed order:
complete, then not current, then the first owed gate. -/
def AdvanceSpec (nodes : List plan.Node) (pos : Nat) (met : List Bool) :
    core.result.Result Unit plan.Refusal → Prop
  | .Ok _ => cur nodes = some pos ∧ firstOwed met = none
  | .Err .Complete => cur nodes = none
  | .Err (.NotCurrent now) => cur nodes = some now.val ∧ now.val ≠ pos
  | .Err (.Owed gate) => cur nodes = some pos ∧ firstOwed met = some gate.val

theorem advance_check_spec (nodes : Slice plan.Node) (pos : Usize) (met : Slice Bool) :
    plan.advance_check nodes pos met ⦃ r => AdvanceSpec nodes.val pos.val met.val r ⦄ := by
  unfold plan.advance_check
  step with current_spec as ⟨ o, ho ⟩
  cases o with
  | none =>
    simp only [WP.spec_ok, AdvanceSpec]
    simpa using ho.symm
  | some now =>
    simp only
    split
    · rename_i hne
      simp only [WP.spec_ok, AdvanceSpec]
      refine ⟨by simpa using ho.symm, ?_⟩
      intro h
      simp only [bne_iff_ne, ne_eq] at hne
      exact hne (UScalar.eq_of_val_eq h)
    · rename_i heq
      step with first_owed_spec as ⟨ o1, ho1 ⟩
      have hnow : cur nodes.val = some pos.val := by
        have : now.val = pos.val := by simpa using heq
        rw [← this]
        simpa using ho.symm
      cases o1 with
      | none =>
        simp only [WP.spec_ok, AdvanceSpec]
        exact ⟨hnow, by simpa using ho1.symm⟩
      | some g =>
        simp only [WP.spec_ok, AdvanceSpec]
        exact ⟨hnow, by simpa using ho1.symm⟩

/-- An advance is admitted exactly when the step is the current one and every
one of its gates is met. -/
theorem advance_ok_iff (nodes : Slice plan.Node) (pos : Usize) (met : Slice Bool) :
    plan.advance_check nodes pos met ⦃ r =>
      r = .Ok () ↔ cur nodes.val = some pos.val ∧ ∀ b ∈ met.val, b = true ⦄ := by
  apply WP.spec_mono (advance_check_spec nodes pos met)
  intro r hr
  rw [← firstOwed_none_iff]
  cases r with
  | Ok u => simpa [AdvanceSpec] using hr
  | Err e =>
    cases e with
    | Complete => simp_all [AdvanceSpec]
    | NotCurrent now =>
      simp only [AdvanceSpec] at hr
      simp only [reduceCtorEq, false_iff, not_and]
      intro h
      rw [hr.1] at h
      simp only [Option.some.injEq] at h
      exact absurd h hr.2
    | Owed g => simp_all [AdvanceSpec]

/-! ## `stale` -/

/-- `stale` is a done step with an owed gate. -/
theorem stale_spec (node : plan.Node) (met : Slice Bool) :
    plan.stale node met ⦃ b => b = true ↔ node.done = true ∧ false ∈ met.val ⦄ := by
  unfold plan.stale
  split
  · rename_i hd
    step with first_owed_spec as ⟨ o, ho ⟩
    try simp only [WP.spec_ok]
    have hnone : (o = none) ↔ ∀ b ∈ met.val, b = true := by
      rw [← firstOwed_none_iff, ← ho]
      cases o <;> simp
    cases o with
    | none =>
      simp only [Option.isSome_none, Bool.false_eq_true, hd, true_and, false_iff]
      intro hx
      have := (hnone.mp rfl) false hx
      simp at this
    | some g =>
      simp only [Option.isSome_some, hd, true_and, true_iff]
      by_contra hall
      have : (some g = none) := hnone.mpr (by
        intro b hb
        cases b
        · exact absurd hb hall
        · rfl)
      simp at this
  · rename_i hd
    simp_all

/-- A stale step is never the current step: staleness reads gates, the current
step reads only `done`/`skipped`, and a stale step is done. Showing `!stale`
never moves the current step back. -/
theorem stale_never_current (nodes : Slice plan.Node) (i : Nat) (hi : i < nodes.val.length)
    (met : Slice Bool) :
    plan.stale nodes.val[i] met ⦃ b => b = true → cur nodes.val ≠ some i ⦄ := by
  apply WP.spec_mono (stale_spec nodes.val[i] met)
  intro b hb hbt hcur
  have hdone := (hb.mp hbt).1
  obtain ⟨h, hf, _⟩ := (leastFailing_some_iff _ _ _).mp hcur
  simp [closedB, hdone] at hf

/-! ## `splice` -/

/-- An open step, as `splice` inserts it. -/
def openNode : plan.Node := { done := false, skipped := false }

/-- `splice` on lists: `count` open steps inserted at `pos`. -/
def spliceList (l : List plan.Node) (pos count : Nat) : List plan.Node :=
  l.take pos ++ List.replicate count openNode ++ l.drop pos

theorem splice_loop0_spec (nodes : Slice plan.Node) (pos : Usize) (out : alloc.vec.Vec plan.Node)
    (i : Usize) (hi : i.val ≤ nodes.val.length) (hpos : i.val ≤ pos.val)
    (hout : out.val = nodes.val.take i.val) :
    plan.splice_loop0 nodes pos out i ⦃ r =>
      r.1.val = nodes.val.take pos.val ∧ r.2.val = min pos.val nodes.val.length ⦄ := by
  unfold plan.splice_loop0
  apply loop.spec_decr_nat (fun p : alloc.vec.Vec plan.Node × Usize => nodes.val.length - p.2.val)
    (fun p : alloc.vec.Vec plan.Node × Usize => p.2.val ≤ nodes.val.length ∧
      p.2.val ≤ pos.val ∧ p.1.val = nodes.val.take p.2.val)
  · rintro ⟨out, i⟩ ⟨hi, hpos, hout⟩
    unfold plan.splice_loop0.body
    dsimp only at hi hpos hout ⊢
    split
    · rename_i hlt
      split
      · rename_i hlp
        step as ⟨ x, hx ⟩
        apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ (by scalar_tac))
        intro out1 hout1
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        have hi1 : i'.val = i.val + 1 := by scalar_tac
        and_intros
        all_goals try scalar_tac
        rw [hout1, hout, hi1, List.take_add_one,
          List.getElem?_eq_getElem (show i.val < nodes.val.length by scalar_tac), hx]
        simp
      · rename_i hlp
        simp only [WP.spec_ok]
        have : i.val = pos.val := by scalar_tac
        refine ⟨by rw [hout, this], by scalar_tac⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = nodes.val.length := by scalar_tac
      refine ⟨?_, by scalar_tac⟩
      rw [hout, this, List.take_length, List.take_of_length_le (by scalar_tac)]
  · exact ⟨hi, hpos, hout⟩

theorem splice_loop1_spec (count : Usize) (out : alloc.vec.Vec plan.Node) (added : Usize)
    (pre : List plan.Node) (hadded : added.val ≤ count.val)
    (hbound : pre.length + count.val ≤ Usize.max)
    (hout : out.val = pre ++ List.replicate added.val openNode) :
    plan.splice_loop1 count out added ⦃ r =>
      r.val = pre ++ List.replicate count.val openNode ⦄ := by
  unfold plan.splice_loop1
  apply loop.spec_decr_nat (fun p : alloc.vec.Vec plan.Node × Usize => count.val - p.2.val)
    (fun p : alloc.vec.Vec plan.Node × Usize => p.2.val ≤ count.val ∧
      p.1.val = pre ++ List.replicate p.2.val openNode)
  · rintro ⟨out, added⟩ ⟨hadded, hout⟩
    unfold plan.splice_loop1.body
    dsimp only at hadded hout ⊢
    split
    · rename_i hlt
      have hlen : out.val.length < Usize.max := by
        rw [hout]; simp; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro out1 hout1
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro a' ha'
      simp only [WP.spec_ok]
      have ha1 : a'.val = added.val + 1 := by scalar_tac
      and_intros
      all_goals try scalar_tac
      rw [hout1, hout, ha1, List.replicate_succ', List.append_assoc]
      rfl
    · rename_i hge
      simp only [WP.spec_ok]
      have : added.val = count.val := by scalar_tac
      rw [hout, this]
  · exact ⟨hadded, hout⟩

theorem splice_loop2_spec (nodes : Slice plan.Node) (out : alloc.vec.Vec plan.Node) (i : Usize)
    (target : List plan.Node) (hi : i.val ≤ nodes.val.length)
    (hbound : target.length ≤ Usize.max)
    (hout : out.val ++ nodes.val.drop i.val = target) :
    plan.splice_loop2 nodes out i ⦃ r => r.val = target ⦄ := by
  unfold plan.splice_loop2
  apply loop.spec_decr_nat (fun p : alloc.vec.Vec plan.Node × Usize => nodes.val.length - p.2.val)
    (fun p : alloc.vec.Vec plan.Node × Usize => p.2.val ≤ nodes.val.length ∧
      p.1.val ++ nodes.val.drop p.2.val = target)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold plan.splice_loop2.body
    dsimp only at hi hout ⊢
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hlen : out.val.length < Usize.max := by
        have h1 := congrArg List.length hout
        simp only [List.length_append, List.length_drop] at h1
        scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro out1 hout1
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro i' hi'
      simp only [WP.spec_ok]
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      and_intros
      all_goals try scalar_tac
      rw [hout1, hi1, ← hout, hx,
        List.drop_eq_getElem_cons (show i.val < nodes.val.length by scalar_tac)]
      simp
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = nodes.val.length := by scalar_tac
      rw [← hout, this, List.drop_length, List.append_nil]
  · exact ⟨hi, hout⟩

/-- `splice` inserts `count` open steps at `pos` (appending when `pos` is
past the end). The bound is the model's: `Vec::push` fails past `usize::MAX`
elements. -/
theorem splice_spec (nodes : Slice plan.Node) (pos count : Usize)
    (hbound : nodes.val.length + count.val ≤ Usize.max) :
    plan.splice nodes pos count ⦃ out => out.val = spliceList nodes.val pos.val count.val ⦄ := by
  unfold plan.splice
  apply WP.spec_bind (splice_loop0_spec nodes pos (alloc.vec.Vec.new plan.Node) 0#usize
    (by simp) (by simp) (by simp))
  rintro ⟨out, i⟩ ⟨hout, hi⟩
  dsimp only at hout hi ⊢
  have htake : (nodes.val.take pos.val).length + count.val ≤ Usize.max := by
    simp; omega
  apply WP.spec_bind (splice_loop1_spec count out 0#usize _ (by simp) htake (by simp [hout]))
  intro out1 hout1
  apply splice_loop2_spec
  · scalar_tac
  · simp [spliceList]; omega
  · rw [hout1, hi, spliceList]
    by_cases h : pos.val ≤ nodes.val.length
    · rw [Nat.min_eq_left h]
    · rw [Nat.min_eq_right (by omega), List.drop_length,
        List.drop_of_length_le (by omega)]

/-- Every existing step keeps its state and its order: removing the inserted
block gives back the list. -/
theorem splice_keeps_existing (l : List plan.Node) (pos count : Nat) (h : pos ≤ l.length) :
    (spliceList l pos count).take pos ++ (spliceList l pos count).drop (pos + count) = l := by
  have ht : (l.take pos).length = pos := by simp; omega
  have h1 : (spliceList l pos count).take pos = l.take pos := by
    unfold spliceList
    rw [List.append_assoc]
    exact List.take_left' ht
  have h2 : (spliceList l pos count).drop (pos + count) = l.drop pos := by
    unfold spliceList
    apply List.drop_left'
    simp [ht]
  rw [h1, h2, List.take_append_drop]

/-- The steps before the insertion point, which an admitted splice makes the
done (closed) prefix, are unchanged. -/
theorem splice_keeps_prefix (l : List plan.Node) (pos count : Nat) (h : pos ≤ l.length) :
    (spliceList l pos count).take pos = l.take pos := by
  have ht : (l.take pos).length = pos := by simp; omega
  unfold spliceList
  rw [List.append_assoc]
  exact List.take_left' ht

/-- The inserted steps are open. -/
theorem splice_new_open (l : List plan.Node) (pos count : Nat) (h : pos ≤ l.length) (j : Nat)
    (hj : j < count) : (spliceList l pos count)[pos + j]? = some openNode := by
  have ht : (l.take pos).length = pos := by simp; omega
  simp only [spliceList, List.append_assoc]
  rw [List.getElem?_append_right (by omega), ht, Nat.add_sub_cancel_left,
    List.getElem?_append_left (by simpa using hj)]
  simp [hj]

theorem leastFailing_append_of_all {α : Type} (p : α → Bool) (a b : List α)
    (hall : ∀ x ∈ a, p x = true) :
    leastFailing p (a ++ b) = (leastFailing p b).map (· + a.length) := by
  induction a with
  | nil => simp
  | cons x a ih =>
    have hx : p x = true := hall x (by simp)
    simp only [List.cons_append, leastFailing, hx, ↓reduceIte, List.length_cons]
    rw [ih (fun y hy => hall y (by simp [hy]))]
    cases leastFailing p b <;> simp; omega

/-- A splice of at least one step inserted before the current step (or into
an empty list at 0) makes the first new step current. -/
theorem splice_current (l : List plan.Node) (pos count : Nat) (hcount : 0 < count)
    (hat : cur l = some pos ∨ (l = [] ∧ pos = 0)) :
    cur (spliceList l pos count) = some pos := by
  have hprefix : ∀ x ∈ l.take pos, closedB x = true := by
    rcases hat with hcur | ⟨rfl, rfl⟩
    · obtain ⟨h, _, hall⟩ := (leastFailing_some_iff _ _ _).mp hcur
      intro x hx
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
      simp only [List.length_take] at hj
      simpa using hall j (by omega)
    · simp
  have hlen : (l.take pos).length = pos := by
    rcases hat with hcur | ⟨rfl, rfl⟩
    · obtain ⟨h, _, _⟩ := (leastFailing_some_iff _ _ _).mp hcur
      simp; omega
    · simp
  unfold cur spliceList
  rw [List.append_assoc, leastFailing_append_of_all _ _ _ hprefix, hlen]
  obtain ⟨c, rfl⟩ : ∃ c, count = c + 1 := ⟨count - 1, by omega⟩
  simp [List.replicate_succ, leastFailing, closedB, openNode]

/-- A splice of no steps changes nothing. -/
theorem splice_none (l : List plan.Node) (pos : Nat) : spliceList l pos 0 = l := by
  simp [spliceList]

/-! ## `splice_check` -/

/-- `key` is among the keys already spliced at the current step or its
ancestors. -/
def KeyIn (keys : List (alloc.vec.Vec U8)) (key : List U8) : Prop := ∃ k ∈ keys, k.val = key

theorem contains_loop_spec (keys : Slice (alloc.vec.Vec U8)) (key : Slice U8) (i : Usize)
    (hi : i.val ≤ keys.val.length)
    (hmiss : ∀ j (hj : j < i.val) (hj' : j < keys.val.length), keys.val[j].val ≠ key.val) :
    plan.contains_loop keys key i ⦃ r => r = true ↔ KeyIn keys.val key.val ⦄ := by
  unfold plan.contains_loop
  apply loop.spec_decr_nat (fun i : Usize => keys.val.length - i.val)
    (fun i : Usize => i.val ≤ keys.val.length ∧
      ∀ j (hj : j < i.val) (hj' : j < keys.val.length), keys.val[j].val ≠ key.val)
  · intro i ⟨hi, hmiss⟩
    unfold plan.contains_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with same_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨x, by rw [hx]; exact List.getElem_mem _, ?_⟩
        have := hb.mp hbt
        simpa [alloc.vec.Vec.deref, Slice.from_val] using this
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hj'
        by_cases hji : j < i.val
        · exact hmiss j hji hj'
        · have : j = i.val := by scalar_tac
          subst this
          intro heq
          apply hbt
          apply hb.mpr
          rw [← hx] at heq
          simpa [alloc.vec.Vec.deref, Slice.from_val] using heq
    · rename_i hge
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨k, hk, hkey⟩
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hk
      exact hmiss j (by scalar_tac) hj hkey
  · exact ⟨hi, hmiss⟩

theorem contains_spec (keys : Slice (alloc.vec.Vec U8)) (key : Slice U8) :
    plan.contains keys key ⦃ r => r = true ↔ KeyIn keys.val key.val ⦄ := by
  unfold plan.contains
  apply contains_loop_spec
  · simp
  · intro j hj; simp at hj

/-- `splice_check`'s decision, in its fixed order: a key already spliced is
`Already`; then the depth bound; then an empty list takes a root splice at
0; otherwise the steps go before the current step, which must exist. -/
def SpliceCheckSpec (nodes : List plan.Node) (keys : List (alloc.vec.Vec U8)) (key : List U8)
    (depth : Nat) : core.result.Result plan.Spliced plan.SpliceRefusal → Prop
  | .Ok .Already => KeyIn keys key
  | .Err .TooDeep => ¬ KeyIn keys key ∧ depth > 4
  | .Ok (.Insert pos) => ¬ KeyIn keys key ∧ depth ≤ 4 ∧
      ((nodes = [] ∧ pos.val = 0) ∨ cur nodes = some pos.val)
  | .Err .Complete => ¬ KeyIn keys key ∧ depth ≤ 4 ∧ nodes ≠ [] ∧ cur nodes = none

theorem splice_check_spec (nodes : Slice plan.Node) (keys : Slice (alloc.vec.Vec U8))
    (key : Slice U8) (depth : Usize) :
    plan.splice_check nodes keys key depth ⦃ r =>
      SpliceCheckSpec nodes.val keys.val key.val depth.val r ⦄ := by
  unfold plan.splice_check
  step with contains_spec as ⟨ b, hb ⟩
  split
  · rename_i hbt
    simp only [WP.spec_ok, SpliceCheckSpec]
    exact hb.mp hbt
  · rename_i hbt
    have hnot : ¬ KeyIn keys.val key.val := fun h => hbt (hb.mpr h)
    split
    · rename_i hdeep
      simp only [WP.spec_ok, SpliceCheckSpec]
      refine ⟨hnot, ?_⟩
      simp only [plan.MAX_DEPTH] at hdeep
      scalar_tac
    · rename_i hdeep
      have hd : depth.val ≤ 4 := by simp only [plan.MAX_DEPTH] at hdeep; scalar_tac
      step as ⟨ e, he ⟩
      split
      · rename_i het
        simp only [WP.spec_ok, SpliceCheckSpec]
        refine ⟨hnot, hd, Or.inl ⟨?_, by simp⟩⟩
        have hlen : nodes.val.length = 0 := by simpa [he, Slice.length] using het
        exact List.eq_nil_of_length_eq_zero hlen
      · rename_i het
        step with current_spec as ⟨ o, ho ⟩
        cases o with
        | none =>
          simp only [WP.spec_ok, SpliceCheckSpec]
          refine ⟨hnot, hd, ?_, by simpa using ho.symm⟩
          intro hnil
          apply het
          simp [he, Slice.length, hnil]
        | some p =>
          simp only [WP.spec_ok, SpliceCheckSpec]
          exact ⟨hnot, hd, Or.inr (by simpa using ho.symm)⟩

/-- `splice_check` is `Already` exactly when the key is already spliced. -/
theorem splice_check_already_iff (nodes : Slice plan.Node) (keys : Slice (alloc.vec.Vec U8))
    (key : Slice U8) (depth : Usize) :
    plan.splice_check nodes keys key depth ⦃ r =>
      r = .Ok .Already ↔ KeyIn keys.val key.val ⦄ := by
  apply WP.spec_mono (splice_check_spec nodes keys key depth)
  intro r hr
  rcases r with (_ | pos) | (_ | _) <;> simp_all [SpliceCheckSpec]

/-- Idempotence on the key: once a template's key is recorded at the current
step or an ancestor, splicing it again changes nothing, whatever else holds. -/
theorem splice_idempotent (nodes : Slice plan.Node) (keys : Slice (alloc.vec.Vec U8))
    (key : Slice U8) (depth : Usize) (h : KeyIn keys.val key.val) :
    plan.splice_check nodes keys key depth ⦃ r => r = .Ok .Already ⦄ := by
  apply WP.spec_mono (splice_check_already_iff nodes keys key depth)
  intro r hr
  exact hr.mpr h

/-- Depth is bounded: an admitted insertion is never deeper than
`MAX_DEPTH` (4). -/
theorem splice_check_depth (nodes : Slice plan.Node) (keys : Slice (alloc.vec.Vec U8))
    (key : Slice U8) (depth : Usize) :
    plan.splice_check nodes keys key depth ⦃ r =>
      ∀ pos, r = .Ok (.Insert pos) → depth.val ≤ 4 ⦄ := by
  apply WP.spec_mono (splice_check_spec nodes keys key depth)
  intro r hr pos hpos
  subst hpos
  exact hr.2.1

/-- The composed claim: when `splice_check` admits an insertion and at least
one step is inserted there, the first new step becomes current, and the done
prefix and every existing step are kept. -/
theorem splice_admitted (nodes : Slice plan.Node) (keys : Slice (alloc.vec.Vec U8))
    (key : Slice U8) (depth : Usize) (count : Nat) (hcount : 0 < count) :
    plan.splice_check nodes keys key depth ⦃ r =>
      ∀ pos : Usize, r = .Ok (.Insert pos) →
        cur (spliceList nodes.val pos.val count) = some pos.val ∧
        (spliceList nodes.val pos.val count).take pos.val = nodes.val.take pos.val ∧
        (spliceList nodes.val pos.val count).take pos.val ++
          (spliceList nodes.val pos.val count).drop (pos.val + count) = nodes.val ⦄ := by
  apply WP.spec_mono (splice_check_spec nodes keys key depth)
  intro r hr pos hpos
  subst hpos
  obtain ⟨_, _, hat⟩ := hr
  have hle : pos.val ≤ nodes.val.length := by
    rcases hat with ⟨hnil, h0⟩ | hcur
    · rw [h0]; omega
    · obtain ⟨h, _, _⟩ := (leastFailing_some_iff _ _ _).mp hcur
      omega
  exact ⟨splice_current _ _ _ hcount hat.symm,
    splice_keeps_prefix _ _ _ hle, splice_keeps_existing _ _ _ hle⟩

end Writfold.Core.Plan
