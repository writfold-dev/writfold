import Extracted.WritfoldCore

/-!
# Byte identities

Theorems over the Aeneas translation of `writfold_core::bytes`: `same` is
exact equality of byte strings, `same_option` is equality of optional
identities, and `contains_digest` is list membership. Every theorem is a
total-correctness Hoare triple `f args ⦃ r => P r ⦄`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

/-- A 32-byte digest, as the extraction types `[u8; 32]`. -/
abbrev Digest := Aeneas.Std.Array U8 32#usize

namespace Writfold.Core

/-- The loop of `same` from position `i`, given equal lengths and an equal prefix. -/
theorem same_loop_spec (a b : Slice U8) (i : Usize)
    (hlen : a.val.length = b.val.length) (hi : i.val ≤ a.val.length)
    (hprefix : ∀ j, j < i.val → a.val[j]? = b.val[j]?) :
    bytes.same_loop a b i ⦃ r => r ↔ a.val = b.val ⦄ := by
  unfold bytes.same_loop
  apply loop.spec_decr_nat (fun i : Usize => a.val.length - i.val)
    (fun i : Usize => i.val ≤ a.val.length ∧ ∀ j, j < i.val → a.val[j]? = b.val[j]?)
  · intro i ⟨hi, hprefix⟩
    unfold bytes.same_loop.body
    dsimp only
    split_ifs with hlt
    · step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      split_ifs with hne
      · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        intro heq
        rw [bne_iff_ne] at hne
        apply hne
        rw [hx, hy]
        simp only [heq]
      · rw [bne_iff_ne, not_not] at hne
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro j hj
        by_cases hji : j < i.val
        · exact hprefix j hji
        · have : j = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac), List.getElem?_eq_getElem (by scalar_tac),
            ← hx, ← hy, hne]
    · simp only [WP.spec_ok, true_iff]
      apply List.ext_getElem?
      intro j
      by_cases hj : j < a.val.length
      · exact hprefix j (by scalar_tac)
      · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]
  · exact ⟨hi, hprefix⟩

/-- `same` is exact equality of the two byte strings. -/
theorem same_spec (a b : Slice U8) : bytes.same a b ⦃ r => r ↔ a.val = b.val ⦄ := by
  unfold bytes.same
  dsimp only
  split_ifs with hne
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro heq
    rw [bne_iff_ne] at hne
    apply hne
    simp [Slice.len, heq]
  · rw [bne_iff_ne, not_not] at hne
    apply same_loop_spec
    · have h := congrArg (fun x : Usize => x.val) hne
      simpa [Slice.len] using h
    · simp
    · intro j hj; simp at hj

/-- `same_option` is equality of optional identities. -/
theorem same_option_spec (a b : Option (alloc.vec.Vec U8)) :
    bytes.same_option a b ⦃ r => r ↔ a = b ⦄ := by
  unfold bytes.same_option
  cases a <;> cases b <;> simp only [WP.spec_ok, reduceCtorEq, iff_false, iff_true, Bool.false_eq_true,
    not_false_eq_true, Option.some.injEq]
  case some.some l r =>
    step with same_spec as ⟨ q, hq ⟩
    rw [hq]
    simp only [alloc.vec.Vec.deref, Slice.from_val]
    exact (alloc.vec.Vec.eq_iff l r).symm

/-- The loop of `contains_digest` from `i`, given that no earlier element matched. -/
theorem contains_digest_loop_spec (digests : Slice (Digest)) (d : Digest)
    (i : Usize) (hi : i.val ≤ digests.val.length)
    (hmiss : ∀ j, j < i.val → digests.val[j]? ≠ some d) :
    bytes.contains_digest_loop digests d i ⦃ r => r ↔ d ∈ digests.val ⦄ := by
  unfold bytes.contains_digest_loop
  apply loop.spec_decr_nat (fun i : Usize => digests.val.length - i.val)
    (fun i : Usize => i.val ≤ digests.val.length ∧ ∀ j, j < i.val → digests.val[j]? ≠ some d)
  · intro i ⟨hi, hmiss⟩
    unfold bytes.contains_digest_loop.body
    dsimp only
    split_ifs with hlt
    · step as ⟨ x, hx ⟩
      step as ⟨ s, hs ⟩
      step as ⟨ s1, hs1 ⟩
      step with same_spec as ⟨ b, hb ⟩
      split_ifs with hbt
      · simp only [WP.spec_ok, true_iff]
        have hxd : x = d := by
          rw [Aeneas.Std.Array.eq_iff]
          have h := hb.mp hbt
          rw [hs, hs1] at h
          simpa [Aeneas.Std.Array.to_slice, Slice.from_val] using h
        rw [← hxd, hx]
        exact List.getElem_mem _
      · apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro j hj
        by_cases hji : j < i.val
        · exact hmiss j hji
        · have : j = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)]
          intro hsome
          apply hbt
          apply hb.mpr
          rw [hs, hs1]
          simp only [Option.some.injEq] at hsome
          simp [Aeneas.Std.Array.to_slice, Slice.from_val, hx, hsome]
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro hmem
      obtain ⟨j, hj, hjd⟩ := List.getElem_of_mem hmem
      exact hmiss j (by scalar_tac) (by rw [List.getElem?_eq_getElem hj, hjd])
  · exact ⟨hi, hmiss⟩

/-- `contains_digest` is membership. -/
theorem contains_digest_spec (digests : Slice (Digest)) (d : Digest) :
    bytes.contains_digest digests d ⦃ r => r ↔ d ∈ digests.val ⦄ := by
  unfold bytes.contains_digest
  apply contains_digest_loop_spec
  · simp
  · intro j hj; simp at hj

end Writfold.Core
