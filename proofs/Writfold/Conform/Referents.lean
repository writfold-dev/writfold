import Writfold.Conform.Words
import Writfold.Policy
import Writfold.Pointer
import Writfold.Subject
import Writfold.Store
import Writfold.Referent
import Calculus.Evidence
import Calculus.Referent

/-!
# Links: policy, referents and the asset boundary (#207, #208)

`policy::met` implements `Calculus.met`, so the meet law 3.4 holds of it
(`narrow_met_iff`). The current-run pointer, `subject::pick` and
`store::claim` choose a referent as `Calculus.resolve` does (law R), and
`referent::settle` and `judge` are `Calculus.settle` and `judge` (law A).
-/

open Aeneas Aeneas.Std Result
open writfold_core

deriving instance DecidableEq for writfold_core.subject.Pick
deriving instance DecidableEq for writfold_core.pointer.Level
deriving instance DecidableEq for writfold_core.pointer.Pointer

namespace Writfold.Core.Conform

/-! ## Policy: `met` is `Calculus.met`; the meet law follows -/

theorem policy_met_implements (x : gate.Standing) (a : policy.Accept) :
    policy.met x a ⦃ b => b = Calculus.met (standingOf x) (acceptOf a) ⦄ := by
  apply WP.spec_mono (Core.Policy.met_spec x a)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  cases x with
  | Checked s => cases s <;> simp [standingOf, acceptOf, Calculus.met]
  | Attested => simp [standingOf, acceptOf, Calculus.met]
  | Waived => simp [standingOf, acceptOf, Calculus.met]

/-- calculus.md 3.4: met under the meet of two policies is met under each. -/
theorem narrow_met_iff (x : gate.Standing) (a b c : policy.Accept)
    (hc : acceptOf c = (acceptOf a).inf (acceptOf b)) :
    (do let m ← policy.met x c; let ma ← policy.met x a; let mb ← policy.met x b; ok (m, ma, mb))
      ⦃ r => r.1 = (r.2.1 && r.2.2) ⦄ := by
  step with policy_met_implements as ⟨ m, hm ⟩
  step with policy_met_implements as ⟨ ma, hma ⟩
  step with policy_met_implements as ⟨ mb, hmb ⟩
  simp [hm, hma, hmb, hc, Calculus.met_inf]

/-! ## Law R: the current-run pointer -/

/-- Each pointer that is set offers its seat; an open run offers itself. -/
def seats (s t h : pointer.Pointer) : List (Calculus.Candidate pointer.Level) :=
  (if s = .Unset then [] else [⟨3, some .Session⟩]) ++
  (if t = .Unset then [] else [⟨2, some .Tree⟩]) ++
  (if h = .Unset then [] else [⟨1, some .Shared⟩])

def resolvedOf {ρ : Type} : Calculus.Resolution ρ → Option ρ
  | .one r => some r
  | _ => none

theorem level_implements (s t h : pointer.Pointer) :
    pointer.level s t h ⦃ l => l = resolvedOf (Calculus.resolve (seats s t h)) ⦄ := by
  unfold pointer.level pointer.is_set
  cases s <;> cases t <;> cases h <;>
    simp [seats, resolvedOf, Calculus.resolve, Calculus.Candidate.top]

/-- A pointer as a candidate: an open run offers itself, a closed or
missing one names the run and offers nothing. -/
def cand (level : Nat) : pointer.Pointer → List (Calculus.Candidate Nat)
  | .Unset => []
  | .Open => [⟨level, some level⟩]
  | _ => [⟨level, none⟩]

theorem open_implements (s t h : pointer.Pointer) :
    pointer.open s t h ⦃ b =>
      b = (Calculus.resolve (cand 3 s ++ cand 2 t ++ cand 1 h)).resolved ⦄ := by
  apply WP.spec_mono (Core.Pointer.open_spec s t h)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  cases s <;> cases t <;> cases h <;>
    simp [Core.Pointer.deciding, cand, Calculus.resolve, Calculus.Resolution.resolved,
      Calculus.Candidate.top]

/-! ## Law R: the subject a gate is about -/

/-- The checkouts that could be the subject, strongest first: the enclosing
one when it is not the KB's, a recorded one that is not the KB's, the
enclosing KB checkout, a recorded KB checkout. -/
def subjects (here hereKb recorded recordedKb : Bool) : List (Calculus.Candidate subject.Pick) :=
  (if here && !hereKb then [⟨3, some .Here⟩] else []) ++
  (if recorded && !recordedKb then [⟨2, some .Recorded⟩] else []) ++
  (if here then [⟨1, some .Here⟩] else []) ++
  (if recorded then [⟨0, some .Kb⟩] else [])

def pickOf (r : Calculus.Resolution subject.Pick) : subject.Pick :=
  (resolvedOf r).getD .Nothing

theorem pick_implements (h hk r rk : Bool) :
    subject.pick h hk r rk ⦃ p => p = pickOf (Calculus.resolve (subjects h hk r rk)) ⦄ := by
  apply WP.spec_mono (Core.Subject.pick_spec h hk r rk)
  intro p hp; rw [hp]
  cases h <;> cases hk <;> cases r <;> cases rk <;>
    simp [Core.Subject.Rule, subjects, pickOf, resolvedOf, Calculus.resolve, Calculus.Candidate.top]

/-! ## Law R: which record a store's `subject:` names -/

/-- The records a name could mean: its own record at the top level, else
each record that has it as an alias. -/
def named (exact : Bool) (aliased : Nat) : List (Calculus.Candidate Nat) :=
  if exact then [⟨1, some 0⟩] else (List.range aliased).map fun i => ⟨0, some i⟩

def claimOf : Calculus.Resolution Nat → store.Claim
  | .one _ => .Listed
  | .ambiguous => .Ambiguous
  | .none => .Unknown

theorem resolve_aliases (n : Nat) :
    Calculus.resolve ((List.range n).map fun i => (⟨0, some i⟩ : Calculus.Candidate Nat)) =
      if n = 0 then .none else if n = 1 then .one 0 else .ambiguous := by
  match n with
  | 0 => rfl
  | 1 => rfl
  | k + 2 =>
    have htop : ∀ cs : List (Calculus.Candidate Nat), (∀ c ∈ cs, c.level = 0) →
        cs.filter (Calculus.Candidate.top cs) = cs := by
      intro cs hcs
      apply List.filter_eq_self.mpr
      intro c hc
      simp only [Calculus.Candidate.top, List.all_eq_true, decide_eq_true_eq]
      intro c' hc'; rw [hcs c' hc', hcs c hc]
    unfold Calculus.resolve
    rw [htop _ (by intro c hc; simp at hc; obtain ⟨_, _, rfl⟩ := hc; rfl)]
    rw [List.range_succ_eq_map]
    simp only [List.map_cons, List.map_map]
    have h1 : (1 : Nat) ∈ (List.range (k + 1)).map (Nat.succ) := by
      simp
    simp only [Nat.add_eq, Nat.add_one_ne_zero, ite_false, Nat.succ_ne_self]
    split
    · rename_i hall
      have := List.all_eq_true.mp hall ⟨0, some 1⟩ (by simp)
      simp at this
    · rfl

theorem claim_implements (a e : Bool) (n : Usize) :
    store.claim a e n ⦃ c =>
      c = if a then claimOf (Calculus.resolve (named e n.val)) else .NotAStore ⦄ := by
  unfold store.claim
  cases a
  · simp
  · cases e
    · simp only [ite_true, Bool.false_eq_true, ite_false, named, resolve_aliases]
      split
      · rename_i h1; simp [UScalar.val_eq_of_eq h1, claimOf]
      · rename_i h1
        split
        · rename_i h0; simp [UScalar.val_eq_of_eq h0, claimOf]
        · rename_i h0
          have h0' : n.val ≠ 0 := fun e => h0 (UScalar.eq_of_val_eq (by simpa using e))
          have h1' : n.val ≠ 1 := fun e => h1 (UScalar.eq_of_val_eq (by simpa using e))
          simp [h0', h1', claimOf]
    · simp [named, Calculus.resolve, Calculus.Candidate.top, claimOf]

/-! ## Law A: the asset boundary -/

/-- Verdicts as a vocabulary: `unverifiable` cannot be told; changed, gone,
moved and renamed claim change. -/
def verdictWords : Calculus.Words verdict.Verdict :=
  ⟨.Unverifiable, Core.Referent.claimsB, rfl⟩

/-- A referent behind the cited snapshot could not have shown that the
cited content is unchanged, so it cannot refute a claim of change. -/
def refutable : referent.Relation → Bool
  | .Behind => false
  | _ => true

theorem settle_implements (r : Bool) (rel : referent.Relation) (v : verdict.Verdict) :
    referent.settle r rel v ⦃ x => x = Calculus.settle verdictWords r (refutable rel) v ⦄ := by
  apply WP.spec_mono (Core.Referent.settle_spec r rel v)
  intro x hx; rw [hx]
  cases r <;> cases rel <;> cases v <;>
    simp [Core.Referent.settleOf, Calculus.settle, verdictWords, refutable, Core.Referent.claimsB]

theorem judge_implements (r : Bool) (rel : referent.Relation) (f : verdict.Observed) :
    referent.judge r rel f ⦃ x =>
      x = Calculus.judge verdictWords Core.Verdict.observedOf r (refutable rel) f ⦄ := by
  apply WP.spec_mono (Core.Referent.judge_spec r rel f)
  intro x hx; rw [hx]
  unfold Calculus.judge
  generalize Core.Verdict.observedOf f = v
  cases r <;> cases rel <;> cases v <;>
    simp [Core.Referent.settleOf, Calculus.settle, verdictWords, refutable, Core.Referent.claimsB]

end Writfold.Core.Conform
