import Writfold.Conform.Referents
import Writfold.Verdict
import Writfold.Recorded
import Writfold.Store
import Calculus.Knowledge

/-!
# Links: citation verdicts and stores (#212)

A verdict is a word of a vocabulary (`Core.Verdict.words`, unknown word
`unverifiable`) and, onto `Tri`, `fresh` is `yes` (`verdictTri`). The
verdict kernels are monotone into `Calculus.Words.refines`: a fuller
observation (`Core.Verdict.Knows`), a clone that has the cited commit, or
a known verdict below a mark changes a verdict only where it could not be
told (`Core.Verdict.observed_monotone` is the link of `verdict::observed`).
A receipt stands in only for the unknown word; only told words are
shared. A store's clone is chosen as `Calculus.resolve` does, and a run
artifact or snapshot is admitted as `Calculus.Tri.all` of its facts.
-/

open Aeneas Aeneas.Std Result
open writfold_core

deriving instance DecidableEq for writfold_core.store.Via

namespace Writfold.Core.Conform

/-! ## The dictionary -/

/-- A verdict onto `Tri`: fresh holds; a verdict that could not be told, a
mark and a due recheck say nothing; every other word says it does not. -/
def verdictTri : verdict.Verdict → Calculus.Tri
  | .Fresh => .yes
  | .Unverifiable | .Marked | .Due => .unknown
  | _ => .no

theorem verdictTri_onto (t : Calculus.Tri) : ∃ v, verdictTri v = t := by
  cases t
  · exact ⟨.Fresh, rfl⟩
  · exact ⟨.Changed, rfl⟩
  · exact ⟨.Unverifiable, rfl⟩

/-- Refining a verdict refines its `Tri`: the vocabulary's order is the
knowledge order. -/
theorem verdictTri_refines {v w : verdict.Verdict} (h : Core.Verdict.words.refines v w) :
    verdictTri v ≤ verdictTri w := by
  rcases h with rfl | rfl
  · exact Calculus.Tri.le_refl _
  · cases w <;> decide

/-- What a checker said, onto `Tri`. -/
def saidTri : verdict.Said → Calculus.Tri
  | .Fresh => .yes
  | .Unverifiable => .unknown
  | _ => .no

theorem saidTri_onto (t : Calculus.Tri) : ∃ s, saidTri s = t := by
  cases t
  · exact ⟨.Fresh, rfl⟩
  · exact ⟨.Changed, rfl⟩
  · exact ⟨.Unverifiable, rfl⟩

/-- What a checker said as a vocabulary: moved, changed and gone claim
change. -/
def saidWords : Calculus.Words verdict.Said :=
  ⟨.Unverifiable, fun s => match s with | .Moved | .Changed | .Gone => true | _ => false, rfl⟩

/-- A mark is the unknown word of a marked citation: more knowledge only
replaces it by a verdict a mark cannot hide. -/
def markWords (m : Bool) : Calculus.Words verdict.Verdict :=
  ⟨if m then .Marked else .Unverifiable, Core.Verdict.claimsChange, by cases m <;> rfl⟩

/-! ## `observed`, `other_blob`, `missing`: monotone -/

theorem verdict_other_blob_implements {k k' : Bool} {l l' : Option verdict.Located}
    (h : Core.Verdict.Knows (.Code (.OtherBlob k l)) (.Code (.OtherBlob k' l'))) :
    verdict.other_blob k l ⦃ v =>
      verdict.other_blob k' l' ⦃ w => Calculus.Words.refines Core.Verdict.words v w ⦄ ⦄ := by
  apply WP.spec_mono (Core.Verdict.other_blob_spec k l); intro v hv
  apply WP.spec_mono (Core.Verdict.other_blob_spec k' l'); intro w hw
  subst hv hw; exact Core.Verdict.observedOf_mono h

theorem verdict_missing_implements {a b : verdict.Absence}
    (h : Core.Verdict.Knows (.Code (.Missing a)) (.Code (.Missing b))) :
    verdict.missing a ⦃ v =>
      verdict.missing b ⦃ w => Calculus.Words.refines Core.Verdict.words v w ⦄ ⦄ := by
  apply WP.spec_mono (Core.Verdict.missing_spec a); intro v hv
  apply WP.spec_mono (Core.Verdict.missing_spec b); intro w hw
  subst hv hw; exact Core.Verdict.observedOf_mono h

/-! ## Marks: monotone into the marked vocabulary -/

def markOf (v : verdict.Verdict) (m : Bool) : verdict.Verdict :=
  if Core.Verdict.Strong v = true then v else if m then .Marked else v

theorem markOf_mono {v w : verdict.Verdict} (m : Bool) (h : Core.Verdict.words.refines v w) :
    (markWords m).refines (markOf v m) (markOf w m) := by
  rcases h with rfl | rfl
  · exact Calculus.Words.refines_refl _ _
  · right; cases m <;> rfl

theorem verdict_with_mark_implements {v w : verdict.Verdict} (m : Bool)
    (h : Core.Verdict.words.refines v w) :
    verdict.with_mark v m ⦃ x =>
      verdict.with_mark w m ⦃ y => Calculus.Words.refines (markWords m) x y ⦄ ⦄ := by
  apply WP.spec_mono (Core.Verdict.with_mark_spec v m); intro x hx
  apply WP.spec_mono (Core.Verdict.with_mark_spec w m); intro y hy
  subst hx hy; exact markOf_mono m h

theorem verdict_decide_implements {f g : verdict.Observed} (m : Bool)
    (h : Core.Verdict.Knows f g) :
    verdict.decide f m ⦃ x =>
      verdict.decide g m ⦃ y => Calculus.Words.refines (markWords m) x y ⦄ ⦄ := by
  apply WP.spec_mono (Core.Verdict.decide_spec f m); intro x hx
  apply WP.spec_mono (Core.Verdict.decide_spec g m); intro y hy
  subst hx hy; exact markOf_mono m (Core.Verdict.observedOf_mono h)

/-! ## Stores: the pinned verdict, receipts, the clone -/

/-- A clone that lacks the cited commit knows less than one that has it. -/
def CommitBelow (c c' : store.Commit) (a a' : store.AtPin) : Prop :=
  c = .Absent ∨ (c = c' ∧ a = a')

theorem store_pinned_implements {c c' : store.Commit} {a a' : store.AtPin}
    (h : CommitBelow c c' a a') :
    store.pinned c a ⦃ v =>
      store.pinned c' a' ⦃ w => Calculus.Words.refines Core.Verdict.words v w ⦄ ⦄ := by
  rcases h with rfl | ⟨rfl, rfl⟩
  · cases c' <;> cases a' <;> cases a <;>
      simp [store.pinned, Calculus.Words.refines, Core.Verdict.words]
  · cases c <;> cases a <;> simp [store.pinned, Calculus.Words.refines]

/-- A receipt stands in exactly where the local verdict refines to every
verdict, that is where it could not be told. -/
theorem store_use_recorded_implements (v : verdict.Verdict) (r : Bool) :
    store.use_recorded v r ⦃ u =>
      u = true ↔ r = true ∧ ∀ w, Calculus.Words.refines Core.Verdict.words v w ⦄ := by
  apply WP.spec_mono (Core.Store.use_recorded_iff v r); intro u hu
  rw [hu]
  have : (∀ w, Calculus.Words.refines Core.Verdict.words v w) ↔ v = .Unverifiable :=
    ⟨fun h => (Calculus.Words.refines_unknown_iff _ v).mp (h _),
     fun h w => h ▸ Calculus.Words.unknown_refines _ w⟩
  rw [this]; exact And.comm

/-- Only a told verdict is written to a receipt. -/
theorem store_pin_recordable_implements (v : verdict.Verdict) :
    store.pin_recordable v ⦃ b => b = true ↔ Calculus.Words.told Core.Verdict.words v ⦄ := by
  apply WP.spec_mono (Core.Store.code_recorded_unverifiable_never v); intro b hb
  rw [hb]; rfl

/-- Only a statement that was told may decide. -/
theorem recorded_usable_implements (s : verdict.recorded.Statement) :
    verdict.recorded.usable s ⦃ b => b = true ↔ Calculus.Words.told saidWords s.said ⦄ := by
  apply WP.spec_mono (Core.Recorded.usable_spec s); intro b hb
  rw [hb]; rfl

/-- Only what a checker told about the token itself is shared. -/
theorem recorded_recordable_implements (c : verdict.Checker) :
    verdict.recorded.recordable c ⦃ b =>
      b = true ↔ ∃ s, c = .Said s ∧ Calculus.Words.told saidWords s ⦄ := by
  apply WP.spec_mono (Core.Recorded.recordable_iff c); intro b hb
  rw [hb]; rfl

/-- The clones that could be a store's, strongest first: the owner's map
(any origin; flagged when it differs), then a search root or a committed
hint whose origin is the declared one. A clone with another origin is a
different repository, not a candidate. -/
def declaredOrigin : store.Candidate → Bool
  | .Clone true => true
  | _ => false

def storeCands (m s h : store.Candidate) : List (Calculus.Candidate store.Via) :=
  (match m with | .Clone _ => [⟨2, some .Map⟩] | .None => []) ++
  (if declaredOrigin s then [⟨1, some .SearchRoot⟩] else []) ++
  (if declaredOrigin h then [⟨0, some .Hint⟩] else [])

def viaOf : store.Resolved → Option store.Via
  | .Found v _ => some v
  | _ => none

theorem store_resolve_implements (m s h : store.Candidate) :
    store.resolve m s h ⦃ r => viaOf r = resolvedOf (Calculus.resolve (storeCands m s h)) ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;> rcases h with _ | ⟨_ | _⟩ <;>
    simp [storeCands, declaredOrigin, viaOf, resolvedOf, Calculus.resolve, Calculus.Candidate.top]

/-! ## Admission of a store artifact or snapshot: all of its facts -/

def triOf (b : Bool) : Calculus.Tri := if b then .yes else .no

def anchored : store.Commit → Bool
  | .Anchored => true
  | _ => false

def cited : store.AtPin → Bool
  | .Cited => true
  | _ => false

theorem store_artifact_implements (d f r : Bool) (c : store.Commit) (b : Bool) :
    store.artifact d f r c b ⦃ x =>
      x = .Admit ↔ Calculus.Tri.all [triOf d, triOf f, triOf r, triOf (anchored c), triOf b] = .yes ⦄ := by
  apply WP.spec_mono (Core.Store.artifact_admit_iff d f r c b); intro x hx
  rw [hx, Calculus.Tri.all_eq_yes]
  cases d <;> cases f <;> cases r <;> cases c <;> cases b <;> simp [triOf, anchored]

theorem store_snapshot_implements (fs : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.snapshot fs c a ⦃ x =>
      x = .Record ↔ Calculus.Tri.all [triOf fs.embeds, triOf fs.declared, triOf fs.pinned,
        triOf fs.local, triOf fs.intact, triOf (anchored c), triOf (cited a)] = .yes ⦄ := by
  apply WP.spec_mono (Core.Store.snapshot_record_iff fs c a); intro x hx
  rw [hx, Calculus.Tri.all_eq_yes]
  rcases fs with ⟨e, d, p, l, i⟩
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> cases a <;>
    simp [triOf, anchored, cited, Core.Store.snapshotFactsHold]

end Writfold.Core.Conform
