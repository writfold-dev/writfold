import Extracted.WritfoldCore.Referent
import Extracted.WritfoldCore.Types
import Writfold.Verdict
import Calculus.Knowledge

/-!
# The asset boundary (#196, RFC 2026-11-27-asset-boundary)

Theorems over the Aeneas translation of `writfold_core::referent`.

- `settle_spec`: `settle` is the table `settleOf`: an unresolved referent
  gives `Unverifiable`; `Behind` turns `Changed`, `Gone`, `Moved` and
  `Renamed` into `Unverifiable`; otherwise the input.
- `ambiguous_never_judged`, `behind_never_stale`, `fresh_survives_behind`,
  `settle_identity`: the cases of the table one by one.
- `settle_only_withholds`: `settle` returns its input or `Unverifiable`;
  the boundary takes verdicts away and never makes one.
- `judge_spec`: `judge` is `observed` then `settle`, for every input;
  `judge_fresh_needs`: a fresh verdict needs a resolved referent and a
  fresh observation.
- `judge_refines`: a resolved referent that is not behind, with the same
  fact, changes the verdict only where it was `Unverifiable`. With
  `history_only_refines` this is the general form: a provider that knows
  less never contradicts one that knows more. It is `Calculus.settle_refines`
  through `settleOf_settle` (#212). `judge_monotone`: knowing more about
  the fact (`Core.Verdict.Knows`) and about the referent together refine
  the verdict (`observedOf_mono` composed with `Calculus.settle_monotone`).
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Verdict (observedOf observed_spec)

namespace Writfold.Core.Referent

/-- A verdict that says the cited content changed, moved or went. -/
def claimsB : verdict.Verdict → Bool
  | .Moved | .Renamed | .Changed | .Gone => true
  | _ => false

/-- The verdict a referent can support. -/
def settleOf : Bool → referent.Relation → verdict.Verdict → verdict.Verdict
  | false, _, _ => .Unverifiable
  | true, .Behind, v => if claimsB v then .Unverifiable else v
  | true, _, v => v

theorem settle_spec (r : Bool) (rel : referent.Relation) (v : verdict.Verdict) :
    referent.settle r rel v ⦃ x => x = settleOf r rel v ⦄ := by
  unfold referent.settle referent.claims_change
  cases r <;> cases rel <;> cases v <;> simp [settleOf, claimsB]

theorem ambiguous_never_judged (rel : referent.Relation) (v : verdict.Verdict) :
    referent.settle false rel v ⦃ x => x = .Unverifiable ⦄ := by
  apply WP.spec_mono (settle_spec false rel v)
  intro x hx; simp [hx, settleOf]

theorem behind_never_stale (r : Bool) (v : verdict.Verdict) :
    referent.settle r .Behind v
      ⦃ x => x ≠ .Changed ∧ x ≠ .Gone ∧ x ≠ .Moved ∧ x ≠ .Renamed ⦄ := by
  apply WP.spec_mono (settle_spec r .Behind v)
  intro x hx; subst hx
  cases r <;> cases v <;> simp [settleOf, claimsB]

theorem fresh_survives_behind :
    referent.settle true .Behind .Fresh ⦃ x => x = .Fresh ⦄ := by
  apply WP.spec_mono (settle_spec true .Behind .Fresh)
  intro x hx; simp [hx, settleOf, claimsB]

theorem settle_identity (rel : referent.Relation) (v : verdict.Verdict)
    (h : rel ≠ .Behind) : referent.settle true rel v ⦃ x => x = v ⦄ := by
  apply WP.spec_mono (settle_spec true rel v)
  intro x hx; subst hx
  cases rel <;> simp_all [settleOf]

theorem settleOf_withholds (r : Bool) (rel : referent.Relation) (v : verdict.Verdict) :
    settleOf r rel v = v ∨ settleOf r rel v = .Unverifiable := by
  cases r <;> cases rel <;> cases v <;> simp [settleOf, claimsB]

theorem settle_only_withholds (r : Bool) (rel : referent.Relation) (v : verdict.Verdict) :
    referent.settle r rel v ⦃ x => x = v ∨ x = .Unverifiable ⦄ := by
  apply WP.spec_mono (settle_spec r rel v)
  intro x hx; subst hx; exact settleOf_withholds r rel v

theorem judge_spec (r : Bool) (rel : referent.Relation) (f : verdict.Observed) :
    referent.judge r rel f ⦃ x => x = settleOf r rel (observedOf f) ⦄ := by
  unfold referent.judge
  apply WP.spec_bind (observed_spec f)
  intro v hv; subst hv
  exact settle_spec r rel _

theorem judge_fresh_needs (r : Bool) (rel : referent.Relation) (f : verdict.Observed) :
    referent.judge r rel f ⦃ x => x = .Fresh → r = true ∧ observedOf f = .Fresh ⦄ := by
  apply WP.spec_mono (judge_spec r rel f)
  intro x hx hfresh; subst hx
  cases r <;> cases rel <;> cases h : observedOf f <;> simp_all [settleOf, claimsB]

theorem settleOf_refines (r : Bool) (rel rel' : referent.Relation) (v : verdict.Verdict)
    (h : rel' ≠ .Behind) (hd : settleOf r rel v ≠ settleOf true rel' v) :
    settleOf r rel v = .Unverifiable := by
  cases r <;> cases rel <;> cases rel' <;> cases v <;> simp_all [settleOf, claimsB]

/-- A referent that could refute a claim of change: one not behind. -/
def refutableB : referent.Relation → Bool
  | .Behind => false
  | _ => true

/-- `settleOf` is the calculus's `settle` over the verdict vocabulary. -/
theorem settleOf_settle (r : Bool) (rel : referent.Relation) (v : verdict.Verdict) :
    settleOf r rel v = Calculus.settle Core.Verdict.words r (refutableB rel) v := by
  cases r <;> cases rel <;> cases v <;>
    simp [settleOf, Calculus.settle, Core.Verdict.words, Core.Verdict.claimsChange, refutableB, claimsB]

theorem judge_refines (r : Bool) (rel rel' : referent.Relation) (f : verdict.Observed)
    (h : rel' ≠ .Behind) :
    referent.judge r rel f ⦃ x =>
      x = settleOf true rel' (observedOf f) ∨ x = .Unverifiable ⦄ := by
  apply WP.spec_mono (judge_spec r rel f)
  intro x hx; subst hx
  rw [settleOf_settle, settleOf_settle]
  have hf : refutableB rel = true → refutableB rel' = true := by cases rel' <;> simp_all [refutableB]
  exact Calculus.settle_refines _ r _ true _ _ (fun _ => rfl) hf

/-- Knowing more about the fact and about the referent refines the
verdict: a resolved, refutable referent and a fuller observation never
contradict a verdict that was told. -/
theorem judge_monotone {f g : verdict.Observed} (r r' : Bool) (rel rel' : referent.Relation)
    (hk : Core.Verdict.Knows f g) (hr : r = true → r' = true)
    (hrel : refutableB rel = true → refutableB rel' = true) :
    referent.judge r rel f ⦃ x =>
      referent.judge r' rel' g ⦃ y => Calculus.Words.refines Core.Verdict.words x y ⦄ ⦄ := by
  apply WP.spec_mono (judge_spec r rel f); intro x hx
  apply WP.spec_mono (judge_spec r' rel' g); intro y hy
  subst hx hy
  rw [settleOf_settle, settleOf_settle]
  exact Calculus.settle_monotone _ _ _ _ _ hr hrel (Core.Verdict.observedOf_mono hk)

end Writfold.Core.Referent
