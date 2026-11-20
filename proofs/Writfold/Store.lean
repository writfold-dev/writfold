import Aeneas
import Extracted.WritfoldCore

/-!
# Attached stores

Theorems over the Aeneas translation of `writfold_core::store`: which rule
judges a citation into a companion repository of evidence, the verdict
under the pinned revision, and where a declared store's clone is found.

- `basis_pin_iff`, `basis_head_default`: the pin judges exactly a citation
  into a declared store whose freshness is `pinned` and whose token names
  a revision; every other repository is judged at head, as before.
- `pinned_fresh_iff`: under the pin, fresh exactly when the cited commit
  is in the clone, some ref reaches it, and `rev:path` is the cited blob.
  `pinned` takes nothing about the store's head, so a store reorganised
  since reads the same (its signature, not a theorem).
- `pinned_behind_unverifiable`: a commit the clone lacks is unverifiable,
  never gone and never fresh: a clone that is behind is not evidence of
  loss. `pinned_unanchored_gone`: a commit no ref reaches is gone.
  `pinned_other_bytes`: an anchored commit whose path holds other bytes,
  or nothing, is gone.
- `resolve_map_wins`: the local map, written by the machine's owner, wins
  whenever it names a clone, and is flagged when that clone's origin is
  not the declared one. `resolve_committed_needs_origin`: a clone found
  through a search root or a committed path hint is used only when its
  origin is the declared one. `resolve_search_before_hint`.
  `resolve_flag_only_map`: only the map's answer is ever flagged.
  `resolve_missing_iff`, `resolve_mismatch_iff`: missing exactly when no
  way names a clone; a mismatch exactly when one does but none that may be
  used.
- `artifact_admit_iff`: a run records a pointer into a store
  (`checkpoint --artifact`) exactly when the store is declared and found
  here, the revision is a full commit ID that some ref reaches, and the
  path holds a file there. `artifact_admit_reads_fresh`: an admitted
  pointer reads fresh under the pin when it is written; it is an ordinary
  citation, judged by `pinned` on every later read (there is no second
  rule). `artifact_undeclared`, `artifact_not_found`, `artifact_short_rev`,
  `artifact_no_commit`, `artifact_unanchored`, `artifact_no_blob`: the
  refusals, in the order they are checked.
- `claim_listed_iff`: a store directory's `subject:` entry lists the
  directory under a record exactly when it was read from a declared,
  readable store and names that record by its ID or by an alias no other
  record has. `claim_exact_wins`: a record's own ID is never hidden by an
  alias other records share. `claim_ambiguous_iff`, `claim_unknown_iff`:
  an alias of two or more records, or a name of none, lists nothing.
  `claim_needs_store`: nothing is listed from a repository that is not a
  declared, readable store. `Claim` is an input of no grounding function:
  a backlink is the store's claim, never evidence.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Store

/-- The pin judges exactly a declared, pinned store's citation with a
revision. -/
theorem basis_pin_iff (d p r : Bool) :
    store.basis d p r ⦃ b => b = .Pin ↔ d = true ∧ p = true ∧ r = true ⦄ := by
  unfold store.basis
  cases d <;> cases p <;> cases r <;> simp

/-- A repository that is not declared, or a store not declared `pinned`,
is judged at head: every KB without a store declaration reads as before. -/
theorem basis_head_default (d p r : Bool) (h : d = false ∨ p = false) :
    store.basis d p r ⦃ b => b = .Head ⦄ := by
  unfold store.basis
  cases d <;> cases p <;> cases r <;> simp_all

/-- Under the pin, fresh exactly for the cited bytes at an anchored
commit. -/
theorem pinned_fresh_iff (c : store.Commit) (a : store.AtPin) :
    store.pinned c a ⦃ v => v = .Fresh ↔ c = .Anchored ∧ a = .Cited ⦄ := by
  unfold store.pinned
  cases c <;> cases a <;> simp

/-- A commit the clone lacks is unverifiable, whatever the path. -/
theorem pinned_behind_unverifiable (a : store.AtPin) :
    store.pinned .Absent a ⦃ v => v = .Unverifiable ⦄ := by
  unfold store.pinned
  simp

/-- A commit no ref reaches is gone, even while its objects are on disk. -/
theorem pinned_unanchored_gone (a : store.AtPin) :
    store.pinned .Unanchored a ⦃ v => v = .Gone ⦄ := by
  unfold store.pinned
  simp

/-- An anchored commit whose path holds other bytes, or nothing, is gone:
the token never described that commit. -/
theorem pinned_other_bytes (a : store.AtPin) (h : a ≠ .Cited) :
    store.pinned .Anchored a ⦃ v => v = .Gone ⦄ := by
  unfold store.pinned
  cases a <;> simp at h ⊢

/-- The local map wins whenever it names a clone, flagged exactly when the
clone's origin is not the declared one. -/
theorem resolve_map_wins (o : Bool) (s h : store.Candidate) :
    store.resolve (.Clone o) s h ⦃ r => r = .Found .Map (!o) ⦄ := by
  unfold store.resolve
  cases o <;> simp

/-- A clone found through a search root or a committed hint is used only
when its origin is the declared one. -/
theorem resolve_committed_needs_origin (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      (∀ f, r = .Found .SearchRoot f → s = .Clone true ∧ f = false) ∧
      (∀ f, r = .Found .Hint f → h = .Clone true ∧ f = false) ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- With no map entry, a search root with the declared origin is used
before a committed hint. -/
theorem resolve_search_before_hint (h : store.Candidate) :
    store.resolve .None (.Clone true) h ⦃ r => r = .Found .SearchRoot false ⦄ := by
  unfold store.resolve
  simp

/-- Only the map's answer is ever flagged. -/
theorem resolve_flag_only_map (m s h : store.Candidate) :
    store.resolve m s h ⦃ r => ∀ v, r = .Found v true → v = .Map ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- Missing exactly when no way names a clone. -/
theorem resolve_missing_iff (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      r = .Missing ↔ m = .None ∧ s = .None ∧ h = .None ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- A mismatch exactly when no map entry, no committed way with the
declared origin, and some committed way names a clone. -/
theorem resolve_mismatch_iff (m s h : store.Candidate) :
    store.resolve m s h ⦃ r =>
      r = .Mismatch ↔ m = .None ∧ s ≠ .Clone true ∧ h ≠ .Clone true ∧
        (s ≠ .None ∨ h ≠ .None) ⦄ := by
  unfold store.resolve
  rcases m with _ | ⟨_ | _⟩ <;> rcases s with _ | ⟨_ | _⟩ <;>
    rcases h with _ | ⟨_ | _⟩ <;> simp

/-- A run artifact is recorded exactly when the store is declared and
found, the revision is full, the commit is anchored and the path holds a
file. -/
theorem artifact_admit_iff (d f r : Bool) (c : store.Commit) (b : Bool) :
    store.artifact d f r c b ⦃ a => a = .Admit ↔
      d = true ∧ f = true ∧ r = true ∧ c = .Anchored ∧ b = true ⦄ := by
  unfold store.artifact
  cases d <;> cases f <;> cases r <;> cases c <;> cases b <;> simp

/-- An admitted pointer reads fresh under the pin at the moment it is
written: admission asks for exactly the facts `pinned` needs. -/
theorem artifact_admit_reads_fresh (d f r : Bool) (c : store.Commit) (b : Bool) :
    store.artifact d f r c b ⦃ a => a = .Admit →
      store.pinned c (if b then .Cited else .Nothing) = ok .Fresh ⦄ := by
  unfold store.artifact store.pinned
  cases d <;> cases f <;> cases r <;> cases c <;> cases b <;> simp

/-- An undeclared store is refused first. -/
theorem artifact_undeclared (f r : Bool) (c : store.Commit) (b : Bool) :
    store.artifact false f r c b ⦃ a => a = .Refuse .Undeclared ⦄ := by
  unfold store.artifact
  simp

/-- A declared store with no usable clone here is refused next. -/
theorem artifact_not_found (r : Bool) (c : store.Commit) (b : Bool) :
    store.artifact true false r c b ⦃ a => a = .Refuse .NotLocal ⦄ := by
  unfold store.artifact
  simp

/-- A short or symbolic revision is refused before any Git fact counts. -/
theorem artifact_short_rev (c : store.Commit) (b : Bool) :
    store.artifact true true false c b ⦃ a => a = .Refuse .ShortRev ⦄ := by
  unfold store.artifact
  simp

/-- A commit the clone lacks is refused. -/
theorem artifact_no_commit (b : Bool) :
    store.artifact true true true .Absent b ⦃ a => a = .Refuse .NoCommit ⦄ := by
  unfold store.artifact
  simp

/-- A commit no ref reaches is refused: it would read gone at once. -/
theorem artifact_unanchored (b : Bool) :
    store.artifact true true true .Unanchored b ⦃ a => a = .Refuse .Unanchored ⦄ := by
  unfold store.artifact
  simp

/-- An anchored commit whose path holds no file is refused. -/
theorem artifact_no_blob :
    store.artifact true true true .Anchored false ⦃ a => a = .Refuse .NoBlob ⦄ := by
  unfold store.artifact
  simp

/-- Listed exactly from a store, for a record's ID or an alias of exactly
one record. -/
theorem claim_listed_iff (a e : Bool) (n : Usize) :
    store.claim a e n ⦃ c => c = .Listed ↔ a = true ∧ (e = true ∨ n.val = 1) ⦄ := by
  unfold store.claim
  split_ifs <;> simp_all

/-- A record's own ID wins over an alias other records share. -/
theorem claim_exact_wins (n : Usize) :
    store.claim true true n ⦃ c => c = .Listed ⦄ := by
  unfold store.claim
  simp

/-- An alias of two or more records lists nothing. -/
theorem claim_ambiguous_iff (a e : Bool) (n : Usize) :
    store.claim a e n ⦃ c => c = .Ambiguous ↔ a = true ∧ e = false ∧ 2 ≤ n.val ⦄ := by
  unfold store.claim
  split_ifs <;> simp_all
  all_goals try scalar_tac

set_option linter.unusedTactic false in
set_option linter.unreachableTactic false in
/-- A name of no record lists nothing. (The trailing `scalar_tac` is
idle on the shipped kernel and closes the arithmetic left when a mutant
changes a comparison, so the theorem fails only when its statement does.) -/
theorem claim_unknown_iff (a e : Bool) (n : Usize) :
    store.claim a e n ⦃ c => c = .Unknown ↔ a = true ∧ e = false ∧ n.val = 0 ⦄ := by
  unfold store.claim
  split_ifs <;> simp_all
  all_goals try scalar_tac

/-- Nothing is listed from a repository that is not a declared, readable
store. -/
theorem claim_needs_store (e : Bool) (n : Usize) :
    store.claim false e n ⦃ c => c = .NotAStore ⦄ := by
  unfold store.claim
  simp

/-- The five facts about a snapshot that are not Git commits or bytes. -/
def snapshotFactsHold (f : store.SnapshotFacts) : Prop :=
  f.embeds = true ∧ f.declared = true ∧ f.pinned = true ∧ f.local = true ∧ f.intact = true

/-- The reason a snapshot is withheld, the first that applies, or `Record`. -/
def snapshotExpected (f : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.Snapshot :=
  if f.embeds = false then .Withhold .NoCitation
  else if f.declared = false then .Withhold .NotAStore
  else if f.pinned = false then .Withhold .NotPinned
  else if f.local = false then .Withhold .NotLocal
  else if f.intact = false then .Withhold .Damaged
  else match c, a with
    | .Absent, _ | _, .Nothing => .Withhold .NotCommitted
    | _, .Other => .Withhold .OtherBytes
    | .Unanchored, .Cited => .Withhold .Unanchored
    | .Anchored, .Cited => .Record

/-- A snapshot is recorded exactly when the five facts hold, `HEAD` is
anchored and holds the snapshot's bytes. -/
theorem snapshot_record_iff (f : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.snapshot f c a ⦃ s => s = .Record ↔
      snapshotFactsHold f ∧ c = .Anchored ∧ a = .Cited ⦄ := by
  rcases f with ⟨e, d, p, l, i⟩
  unfold store.snapshot snapshotFactsHold
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> cases a <;> simp

/-- The citation a recorded snapshot writes is fresh under the rule that
judges it on every later read: there is no second validity rule. -/
theorem snapshot_record_is_fresh (f : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.snapshot f c a ⦃ s => s = .Record → store.pinned c a = ok .Fresh ⦄ := by
  rcases f with ⟨e, d, p, l, i⟩
  unfold store.snapshot store.pinned
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> cases a <;> simp

/-- An uncommitted snapshot is never cited. -/
theorem snapshot_uncommitted_withheld (f : store.SnapshotFacts) (c : store.Commit) :
    store.snapshot f c .Nothing ⦃ s => s ≠ .Record ⦄ := by
  rcases f with ⟨e, d, p, l, i⟩
  unfold store.snapshot
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> simp

/-- The reason is the first that applies, in the order `NoCitation`,
`NotAStore`, `NotPinned`, `NotLocal`, `Damaged`, `NotCommitted`,
`OtherBytes`, `Unanchored`. -/
theorem snapshot_order (f : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.snapshot f c a ⦃ s => s = snapshotExpected f c a ⦄ := by
  rcases f with ⟨e, d, p, l, i⟩
  unfold store.snapshot snapshotExpected
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> cases a <;> simp

/-- `NotCommitted` exactly when the five facts hold and the store has no
commit or nothing at the path: the only answer under which the wiring may
write into the store. -/
theorem snapshot_stage_iff (f : store.SnapshotFacts) (c : store.Commit) (a : store.AtPin) :
    store.snapshot f c a ⦃ s => s = .Withhold .NotCommitted ↔
      snapshotFactsHold f ∧ (c = .Absent ∨ a = .Nothing) ⦄ := by
  rcases f with ⟨e, d, p, l, i⟩
  unfold store.snapshot snapshotFactsHold
  cases e <;> cases d <;> cases p <;> cases l <;> cases i <;> cases c <;> cases a <;> simp

end Writfold.Core.Store
