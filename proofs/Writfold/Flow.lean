import Writfold.Bytes

/-!
# Trial edit-flow validator

Theorems over the Aeneas translation of `writfold_core::flow`: the validator
of a `writfold-trial-worktree/1` run accepts a record exactly when the record
carries every blob it names in its closure (`Carried`) and is the next step
of the run (`Follows`): `Prepared` with a path on an empty run, `Approved`
after it, `Claimed` after those naming the prepared candidate, then one
outcome, `Applied` naming that same candidate or `Abandoned` with a reason.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Flow

/-- The digests a record's payload names. -/
def blobsOf : flow.Event → List Digest
  | .Prepared _ s r c w => [s.digest, r.digest, c.digest, w.digest]
  | .Approved g a => [g.digest, a.digest]
  | .Claimed c => [c.digest]
  | .Applied c => [c.digest]
  | .Abandoned _ => []

/-- Every blob the payload names is in the closure. -/
def Carried (e : flow.Event) (closure : List Digest) : Prop :=
  ∀ d ∈ blobsOf e, d ∈ closure

theorem same_blob_spec (a b : flow.Blob) :
    flow.same_blob a b ⦃ r => r ↔ a = b ⦄ := by
  unfold flow.same_blob
  step as ⟨ s, hs ⟩
  step as ⟨ s1, hs1 ⟩
  step with same_spec as ⟨ q, hq ⟩
  have hd : q = true ↔ a.digest = b.digest := by
    rw [hq, hs, hs1, Aeneas.Std.Array.eq_iff]
    simp [Aeneas.Std.Array.to_slice, Slice.from_val]
  obtain ⟨ad, ab⟩ := a
  obtain ⟨bd, bb⟩ := b
  split
  · rename_i h
    simp only [WP.spec_ok, decide_eq_true_eq, flow.Blob.mk.injEq]
    exact ⟨fun e => ⟨hd.mp h, e⟩, fun e => e.2⟩
  · rename_i h
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, flow.Blob.mk.injEq, not_and]
    intro e; exact absurd (hd.mpr e) h

theorem carried_spec (e : flow.Event) (closure : Slice Digest) :
    flow.carried e closure ⦃ r => r ↔ Carried e closure.val ⦄ := by
  unfold flow.carried
  cases e with
  | Prepared path s r c w =>
    dsimp only
    step with contains_digest_spec as ⟨ b, hb ⟩
    split
    · rename_i h
      step with contains_digest_spec as ⟨ b1, hb1 ⟩
      split
      · rename_i h1
        step with contains_digest_spec as ⟨ b2, hb2 ⟩
        split
        · rename_i h2
          step with contains_digest_spec as ⟨ b3, hb3 ⟩
          simp only [Carried, blobsOf, List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp,
            forall_eq]
          simp_all
        · rename_i h2
          simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Carried, blobsOf]
          intro hc; exact h2 (hb2.mpr (hc _ (by simp)))
      · rename_i h1
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Carried, blobsOf]
        intro hc; exact h1 (hb1.mpr (hc _ (by simp)))
    · rename_i h
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Carried, blobsOf]
      intro hc; exact h (hb.mpr (hc _ (by simp)))
  | Approved g a =>
    dsimp only
    step with contains_digest_spec as ⟨ b, hb ⟩
    split
    · rename_i h
      step with contains_digest_spec as ⟨ b1, hb1 ⟩
      simp only [Carried, blobsOf, List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp,
        forall_eq]
      simp_all
    · rename_i h
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Carried, blobsOf]
      intro hc; exact h (hb.mpr (hc _ (by simp)))
  | Claimed c =>
    dsimp only
    step with contains_digest_spec as ⟨ b, hb ⟩
    simp_all [Carried, blobsOf]
  | Applied c =>
    dsimp only
    step with contains_digest_spec as ⟨ b, hb ⟩
    simp_all [Carried, blobsOf]
  | Abandoned _ =>
    simp [Carried, blobsOf]

/-- Whether a record is of each kind, as the translation decides it. -/
def isPrepared : flow.Event → Bool
  | .Prepared .. => true
  | _ => false

def isApproved : flow.Event → Bool
  | .Approved .. => true
  | _ => false

def isClaimed : flow.Event → Bool
  | .Claimed .. => true
  | _ => false

theorem is_prepared_eq (e : flow.Event) : flow.is_prepared e = ok (isPrepared e) := by
  cases e <;> rfl

theorem is_approved_eq (e : flow.Event) : flow.is_approved e = ok (isApproved e) := by
  cases e <;> rfl

theorem is_claimed_eq (e : flow.Event) : flow.is_claimed e = ok (isClaimed e) := by
  cases e <;> rfl

theorem is_prepared_spec (e : flow.Event) : flow.is_prepared e ⦃ r => r = isPrepared e ⦄ := by
  rw [is_prepared_eq]; simp

theorem is_approved_spec (e : flow.Event) : flow.is_approved e ⦃ r => r = isApproved e ⦄ := by
  rw [is_approved_eq]; simp

theorem is_claimed_spec (e : flow.Event) : flow.is_claimed e ⦃ r => r = isClaimed e ⦄ := by
  rw [is_claimed_eq]; simp

/-- `first` prepared exactly this candidate. -/
def PreparedWith (first : flow.Event) (c : flow.Blob) : Prop :=
  match first with
  | .Prepared _ _ _ c' _ => c' = c
  | _ => False

theorem prepared_spec (first : flow.Event) (c : flow.Blob) :
    flow.prepared first c ⦃ r => r ↔ PreparedWith first c ⦄ := by
  unfold flow.prepared
  cases first with
  | Prepared _ _ _ c' _ =>
    dsimp only
    step with same_blob_spec as ⟨ b, hb ⟩
    simpa [PreparedWith] using hb
  | _ => simp [PreparedWith]

/-- The run's order and bindings: which record may follow the run's records
so far. `Prepared` (with a path) opens an empty run; `Approved` follows it;
`Claimed` follows those two, naming the prepared candidate; then exactly one
outcome: `Applied` naming that same candidate, or `Abandoned` with a reason. -/
def Follows (pre : List flow.Event) : flow.Event → Prop
  | .Prepared path _ _ _ _ => pre = [] ∧ path.val ≠ []
  | .Approved _ _ => ∃ p, pre = [p] ∧ isPrepared p
  | .Claimed c => ∃ p a, pre = [p, a] ∧ isApproved a ∧ PreparedWith p c
  | .Applied c => ∃ p a k, pre = [p, a, k] ∧ isClaimed k ∧ PreparedWith p c
  | .Abandoned reason => ∃ p a k, pre = [p, a, k] ∧ isClaimed k ∧ reason.val ≠ []

/-- The edit-flow rule. -/
def Valid (pre : List flow.Event) (e : flow.Event) (closure : List Digest) : Prop :=
  Carried e closure ∧ Follows pre e

theorem length_of_len (pre : Slice flow.Event) (n : Usize) (h : Slice.len pre = n) :
    pre.val.length = n.val := by
  rw [← h]; simp [Slice.len]

theorem follows_spec (pre : Slice flow.Event) (e : flow.Event) :
    flow.follows pre e ⦃ r => r ↔ Follows pre.val e ⦄ := by
  unfold flow.follows
  cases e with
  | Prepared path s r c w =>
    dsimp only
    step as ⟨ b, hb ⟩
    split
    · rename_i h
      have h0 : pre.val = [] := by simp_all [Slice.length]
      step as ⟨ b1, hb1 ⟩
      simp_all [Follows, alloc.vec.Vec.length, List.length_eq_zero_iff]
    · rename_i h
      have h0 : pre.val ≠ [] := by intro e; simp_all [Slice.length]
      simp [Follows, h0]
  | Approved g a =>
    dsimp only
    split
    · rename_i h
      have hn : pre.val.length = 1 := by simpa using length_of_len _ _ h
      obtain ⟨p, hp⟩ := List.length_eq_one_iff.mp hn
      step as ⟨ x, hx ⟩
      have hxp : x = p := by simp only [hp, List.getElem_cons_zero] at hx; exact hx
      rw [is_prepared_eq]
      simp [Follows, hp, hxp]
    · rename_i h
      have hn : pre.val.length ≠ 1 := fun e => h (by scalar_tac)
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Follows, not_exists, not_and]
      intro p hp; simp [hp] at hn
  | Claimed cand =>
    dsimp only
    split
    · rename_i h
      have hn : pre.val.length = 2 := by simpa using length_of_len _ _ h
      obtain ⟨p, a, hp⟩ := List.length_eq_two.mp hn
      step as ⟨ x, hx ⟩
      have hxa : x = a := by simp only [hp] at hx; simpa using hx
      step with is_approved_spec as ⟨ b, hb ⟩
      rw [hxa] at hb
      split
      · rename_i ha
        step as ⟨ y, hy ⟩
        have hyp : y = p := by simp only [hp] at hy; simpa using hy
        step with prepared_spec as ⟨ q, hq ⟩
        subst hb hyp
        simp [Follows, hp, ha, hq]
      · rename_i ha
        subst hb
        simp [Follows, hp, ha]
    · rename_i h
      have hn : pre.val.length ≠ 2 := fun e => h (by scalar_tac)
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Follows, not_exists, not_and]
      intro p a hp; simp [hp] at hn
  | Applied cand =>
    dsimp only
    split
    · rename_i h
      have hn : pre.val.length = 3 := by simpa using length_of_len _ _ h
      obtain ⟨p, a, k, hp⟩ := List.length_eq_three.mp hn
      step as ⟨ x, hx ⟩
      have hxk : x = k := by simp only [hp] at hx; simpa using hx
      step with is_claimed_spec as ⟨ b, hb ⟩
      rw [hxk] at hb
      split
      · rename_i hk
        step as ⟨ y, hy ⟩
        have hyp : y = p := by simp only [hp] at hy; simpa using hy
        step with prepared_spec as ⟨ q, hq ⟩
        subst hb hyp
        simp [Follows, hp, hk, hq]
      · rename_i hk
        subst hb
        simp [Follows, hp, hk]
    · rename_i h
      have hn : pre.val.length ≠ 3 := fun e => h (by scalar_tac)
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Follows, not_exists, not_and]
      intro p a k hp; simp [hp] at hn
  | Abandoned reason =>
    dsimp only
    split
    · rename_i h
      have hn : pre.val.length = 3 := by simpa using length_of_len _ _ h
      obtain ⟨p, a, k, hp⟩ := List.length_eq_three.mp hn
      step as ⟨ x, hx ⟩
      have hxk : x = k := by simp only [hp] at hx; simpa using hx
      step with is_claimed_spec as ⟨ b, hb ⟩
      rw [hxk] at hb
      split
      · rename_i hk
        step as ⟨ b1, hb1 ⟩
        subst hb
        simp_all [Follows, alloc.vec.Vec.length, List.length_eq_zero_iff]
      · rename_i hk
        subst hb
        simp [Follows, hp, hk]
    · rename_i h
      have hn : pre.val.length ≠ 3 := fun e => h (by scalar_tac)
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Follows, not_exists, not_and]
      intro p a k hp; simp [hp] at hn

/-- The edit-flow validator accepts a record exactly when it carries its
material and follows the run so far. -/
theorem validate_spec (pre : Slice flow.Event) (e : flow.Event) (closure : Slice Digest) :
    flow.validate pre e closure ⦃ r => r ↔ Valid pre.val e closure.val ⦄ := by
  unfold flow.validate
  step with carried_spec as ⟨ b, hb ⟩
  split
  · rename_i h
    step with follows_spec as ⟨ b1, hb1 ⟩
    simp_all [Valid]
  · rename_i h
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Valid, not_and]
    intro hc; exact absurd (hb.mpr hc) h

/-- A run has at most four records and one outcome: nothing follows a run of
four, so nothing follows `Applied` or `Abandoned`, and no second claim or
outcome is ever accepted. -/
theorem follows_closed (pre : List flow.Event) (e : flow.Event) (h : 4 ≤ pre.length) :
    ¬ Follows pre e := by
  cases e <;> simp only [Follows, not_exists, not_and] <;> intros <;> simp_all

/-- An accepted outcome `Applied` names exactly the candidate the run's
`Prepared` named, and comes after a claim. -/
theorem applied_binds_candidate (p a k : flow.Event) (c : flow.Blob)
    (h : Follows [p, a, k] (.Applied c)) :
    isClaimed k ∧ PreparedWith p c := by
  simpa [Follows] using h

end Writfold.Core.Flow
