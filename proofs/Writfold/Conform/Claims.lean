import Writfold.Conform.Words
import Writfold.Claims

/-!
# Links: claims inside text are law U (#189)

A candidate's blocks are an accounted universe (`blockOf`: a bare block is
the remainder, an excused one declared out, a tagged or cited one judged by
how its citations stand): `claims::accounted` satisfies `Calculus.Universal`
with the empty universe allowed (the kernel itself keeps an empty draft
unknown, `empty_never_passes`). `claims::cited` is `Calculus.Tri.and` of
the floor and the shipped outcome.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-- A block as a member of the candidate's universe, with its word. -/
def blockOf : claims.Block → Calculus.Member × Calculus.Tri
  | .Bare => (.remainder, .unknown)
  | .Excused => (.declaredOut, .yes)
  | .Tagged => (.judged, .yes)
  | .Cited .Fresh => (.judged, .yes)
  | .Cited .Stale => (.judged, .no)
  | .Cited .Unverifiable => (.judged, .unknown)

theorem okB_blockOf (b : claims.Block) (h : Core.Claims.okB b) :
    (blockOf b).1 ≠ .remainder ∧ ((blockOf b).1 = .judged → (blockOf b).2 = .yes) := by
  rcases h with rfl | rfl | rfl <;> simp [blockOf]

/-- `claims.accounted` is a universal word over the candidate's blocks. -/
theorem claims_accounted_implements (bs : Slice claims.Block) :
    claims.accounted bs ⦃ r => Calculus.Universal true (bs.val.map blockOf) (outcomeTri r) ⦄ :=
  WP.spec_mono (Core.Claims.accounted_pass_iff bs) (fun r hr => by
    have hyes : outcomeTri r = .yes → ∀ b ∈ bs.val, Core.Claims.okB b := by
      intro h; cases r <;> simp_all [outcomeTri]
    refine ⟨?_, ?_, fun _ => Or.inl rfl⟩
    · intro h
      rw [Calculus.whole_iff_no_remainder]
      intro hm
      obtain ⟨⟨m, t⟩, hmt, he⟩ := List.mem_map.mp hm
      obtain ⟨b, hb, hbe⟩ := List.mem_map.mp hmt
      have := (okB_blockOf b (hyes h b hb)).1
      rw [hbe] at this
      exact this he
    · intro h t ht
      unfold Calculus.judgedOf at ht
      obtain ⟨⟨m, t'⟩, hmt, he⟩ := List.mem_filterMap.mp ht
      obtain ⟨b, hb, hbe⟩ := List.mem_map.mp hmt
      have hk := (okB_blockOf b (hyes h b hb)).2
      rw [hbe] at hk
      split at he
      · rename_i hj
        simp only [Option.some.injEq] at he
        rw [← he]; exact hk hj
      · cases he)

/-- `claims.cited` is the floor and the shipped outcome, together. -/
theorem claims_cited_implements (t m : Usize) (s : gate.Outcome) :
    claims.cited t m s ⦃ r =>
      outcomeTri r = Calculus.Tri.and (if m.val ≤ t.val then .yes else .no) (outcomeTri s) ⦄ :=
  WP.spec_mono (Core.Claims.cited_spec t m s) (fun r hr => by
    subst hr
    by_cases h : t.val < m.val
    · simp [h, show ¬ m.val ≤ t.val by omega, outcomeTri, Calculus.Tri.and]
    · simp only [h, ↓reduceIte, show m.val ≤ t.val by omega]
      cases s <;> rfl)

end Writfold.Core.Conform
