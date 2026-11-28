import Extracted.WritfoldCore.Types
import Extracted.WritfoldCore.Verdict
import Calculus.Knowledge

/-!
# Citation verdicts

Theorems over the Aeneas translation of `writfold_core::verdict`: from
what a check observed to exactly one verdict.

- `observed_spec`: `observed` is the table `observedOf`, for every fact
  (`other_blob_spec`, `missing_spec` for code citations).
  `observed_fresh_iff`: a citation reads fresh exactly for the eight fresh
  facts (`FreshFact`); `moved_span_never_fresh`, `unsearched_never_fresh`,
  `changed_gate_never_fresh`, `checker_fresh_iff`.
- `absence_never_evidence`, `unknown_blob_changed_only_by_span`,
  `gone_witness`, `changed_witness`, `history_only_refines`,
  `rename_never_fresh` (#90): missing history is never evidence of a
  change; a negative verdict for a code citation has a witness.
- `observed_monotone` (#212): `observed` is monotone from `Knows` (what a
  fuller clone or a longer history sees) to `Calculus.Words.refines`;
  `history_only_refines`, `fetch_only_refines`, `unfetched_never_decides`,
  `absence_never_evidence` and `unsearched_never_fresh` (k = false) follow
  through `observedOf_mono` and `observed_undecided`.
- `with_mark_spec`, `mark_keeps_change`, `mark_never_fresh`, `decide_spec`:
  an authored mark wins over every verdict except changed, gone and
  expired, so a mark never hides a change and never reads fresh.
- `handshake_spec`, `exchange_said_iff`, `exchange_spec`: a checker's
  reply counts only after an agreed handshake, a clean exit, a readable
  version and exactly one well-formed reply.
- `aged_spec`, `aged_expired_iff`: a cached reply with a `max_age` expires
  once its age exceeds it or cannot be read.
- `lifetime_spec`: an immutable kind first, then the token's own `ttl=`,
  then the kind's default, else never.
- `expired_spec`: a timed citation has expired exactly when `now - seen >
  ttl` over the integers: the subtraction never overflows.
- `window_spec`, `place_same_iff`, `place_at_iff`, `place_absent_iff`:
  where the cited span sits, from its windows' flags: the cited window
  first, then the first window from the top.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Verdict

/-- `other_blob`: another blob at the cited path. A located span decides from
the bytes at the target; otherwise only a known cited blob says changed. -/
def otherBlobOf (k : Bool) : Option verdict.Located → verdict.Verdict
  | some .Same => .Fresh
  | some (.At _ _) => .Moved
  | some .Absent => .Changed
  | some .Unfetched => .Unverifiable
  | some .Unsearched | none => if k then .Changed else .Unverifiable

/-- `missing`: a path absent at the target, by how following it ended. -/
def missingOf : verdict.Absence → verdict.Verdict
  | .Renamed => .Renamed
  | .Followed (some .Same) | .Followed (some (.At _ _)) => .Moved
  | .Followed (some .Unfetched) => .Unverifiable
  | .Followed _ | .NoRename => .Gone
  | .RevAbsent | .NoRev | .Capped | .Unfetched => .Unverifiable

/-- The verdict of each observation, as `verdict.rs` lists it. -/
def observedOf : verdict.Observed → verdict.Verdict
  | .Malformed | .Unresolved => .Unverifiable
  | .Code .SameBlob => .Fresh
  | .Code (.OtherBlob k l) => otherBlobOf k l
  | .Code (.Missing a) => missingOf a
  | .Gate true => .Fresh
  | .Gate false => .Changed
  | .State none => .Gone
  | .State (some none) => .Unverifiable
  | .State (some (some true)) => .Fresh
  | .State (some (some false)) => .Changed
  | .Timed (some true) => .Expired
  | .Timed (some false) => .Fresh
  | .Timed none => .Unverifiable
  | .Offline true => .Due
  | .Offline false => .Fresh
  | .Immutable => .Fresh
  | .Checked (.Said .Fresh) => .Fresh
  | .Checked (.Said .Moved) => .Moved
  | .Checked (.Said .Changed) => .Changed
  | .Checked (.Said .Gone) => .Gone
  | .Checked _ => .Unverifiable

theorem other_blob_spec (k : Bool) (l : Option verdict.Located) :
    verdict.other_blob k l ⦃ v => v = otherBlobOf k l ⦄ := by
  unfold verdict.other_blob
  rcases l with _ | l
  · cases k <;> simp [otherBlobOf]
  · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> try simp [otherBlobOf]
    cases k <;> simp [otherBlobOf]

theorem missing_spec (a : verdict.Absence) :
    verdict.missing a ⦃ v => v = missingOf a ⦄ := by
  unfold verdict.missing
  rcases a with _ | l | _ | _ | _ | _ | _ <;> try simp [missingOf]
  rcases l with _ | l
  · simp [missingOf]
  · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [missingOf]

theorem observed_spec (f : verdict.Observed) :
    verdict.observed f ⦃ v => v = observedOf f ⦄ := by
  unfold verdict.observed
  rcases f with _ | _ | t | b | o | o | b | _ | c
  all_goals try simp [observedOf]
  · rcases t with _ | ⟨k, l⟩ | a
    · simp [observedOf]
    · simpa [observedOf] using other_blob_spec k l
    · simpa [observedOf] using missing_spec a
  · cases b <;> simp [observedOf]
  · rcases o with _ | (_ | b) <;> try simp [observedOf]
    cases b <;> simp [observedOf]
  · rcases o with _ | b <;> try simp [observedOf]
    cases b <;> simp [observedOf]
  · cases b <;> simp [observedOf]
  · rcases c with _ | _ | _ | s | _ <;> try simp [observedOf]
    cases s <;> simp [observedOf]

/-! ## Knowing more (#212): `observed` is monotone

Facts are ordered by what a fuller clone or a longer history knows beyond
them; verdicts by `Calculus.Words.refines` over `words`, whose unknown word
is `unverifiable`. `observed_monotone`: knowing more changes a verdict only
where it was unverifiable. `history_only_refines` (#90),
`fetch_only_refines` and `unfetched_never_decides` (#182),
`absence_never_evidence` and `unsearched_never_fresh` are its corollaries. -/

/-- Moved, renamed, changed and gone claim change (law A). -/
def claimsChange : verdict.Verdict → Bool
  | .Moved | .Renamed | .Changed | .Gone => true
  | _ => false

/-- Verdicts as a vocabulary: `unverifiable` could not be told. -/
def words : Calculus.Words verdict.Verdict := ⟨.Unverifiable, claimsChange, rfl⟩

/-- A follow that did not complete: the cited commit is not in this clone,
there is no `rev=`, the per-run limit was reached, or the clone lacks the
bytes. -/
def Unfollowed (a : verdict.Absence) : Prop :=
  a = .RevAbsent ∨ a = .NoRev ∨ a = .Capped ∨ a = .Unfetched

/-- `Knows f g`: `g` is what a fuller observation of the same citation can
see where `f` was seen. History supplies the cited blob; a span search or
fetched bytes supply where the span sits; a completed follow supplies how
the path left.

One row is deliberately not here (owner default, #216): a known cited blob
(`blob=`) that differs at the target, in a file too long for the span
search (`Unsearched`), reads `changed`. `blob=` is a claim about the whole file, so a different blob is
a change, whatever a span search would find; `changed` is that claim's own
answer, not a lack of knowledge. The law holds given the citation's claim:
over every row where the citation leaves room to know more. -/
inductive Knows : verdict.Observed → verdict.Observed → Prop
  | refl (f) : Knows f f
  | trans {f g h} : Knows f g → Knows g h → Knows f h
  | history (l) : Knows (.Code (.OtherBlob false l)) (.Code (.OtherBlob true l))
  | search (l) : Knows (.Code (.OtherBlob false none)) (.Code (.OtherBlob false l))
  | unsearched (l) : Knows (.Code (.OtherBlob false (some .Unsearched))) (.Code (.OtherBlob false l))
  | fetch (k l) : Knows (.Code (.OtherBlob k (some .Unfetched))) (.Code (.OtherBlob k l))
  | follow (a b) : Unfollowed a → Knows (.Code (.Missing a)) (.Code (.Missing b))
  | followFetch (l) : Knows (.Code (.Missing (.Followed (some .Unfetched))))
      (.Code (.Missing (.Followed l)))

/-- The table is monotone: the pure core of `observed_monotone`. -/
theorem observedOf_mono {f g : verdict.Observed} (h : Knows f g) :
    words.refines (observedOf f) (observedOf g) := by
  induction h with
  | refl => exact words.refines_refl _
  | trans _ _ ih1 ih2 => exact words.refines_trans ih1 ih2
  | history l =>
    unfold Calculus.Words.refines
    rcases l with _ | l
    · simp [observedOf, otherBlobOf, words]
    · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [observedOf, otherBlobOf, words]
  | search l => right; simp [observedOf, otherBlobOf, words]
  | unsearched l => right; simp [observedOf, otherBlobOf, words]
  | fetch k l => right; simp [observedOf, otherBlobOf, words]
  | follow a b ha =>
    right; rcases ha with rfl | rfl | rfl | rfl <;> simp [observedOf, missingOf, words]
  | followFetch l => right; simp [observedOf, missingOf, words]

/-- `observed` is monotone from what is known about a citation to the
knowledge order on verdicts: a fuller observation never contradicts a
verdict that was told. -/
theorem observed_monotone {f g : verdict.Observed} (h : Knows f g) :
    verdict.observed f ⦃ v => verdict.observed g ⦃ w => Calculus.Words.refines words v w ⦄ ⦄ := by
  apply WP.spec_mono (observed_spec f); intro v hv
  apply WP.spec_mono (observed_spec g); intro w hw
  subst hv hw; exact observedOf_mono h

/-- What `observed_monotone` says of one computation: the verdict refines
to the table at any fuller fact. -/
theorem observed_below {f g : verdict.Observed} (h : Knows f g) :
    verdict.observed f ⦃ v => words.refines v (observedOf g) ⦄ := by
  apply WP.spec_mono (observed_spec f); intro v hv; subst hv; exact observedOf_mono h

/-- A fact below two fuller facts that disagree decided nothing. -/
theorem observed_undecided {f g₁ g₂ : verdict.Observed} (h₁ : Knows f g₁) (h₂ : Knows f g₂)
    (hne : observedOf g₁ ≠ observedOf g₂) :
    verdict.observed f ⦃ v => v = .Unverifiable ⦄ := by
  apply WP.spec_mono (observed_spec f); intro v hv; subst hv
  exact words.refines_disagree (observedOf_mono h₁) (observedOf_mono h₂) hne

/-- The facts that read fresh. -/
def FreshFact (f : verdict.Observed) : Prop :=
  f = .Code .SameBlob ∨ (∃ k, f = .Code (.OtherBlob k (some .Same))) ∨ f = .Gate true ∨
  f = .State (some (some true)) ∨ f = .Timed (some false) ∨ f = .Offline false ∨
  f = .Immutable ∨ f = .Checked (.Said .Fresh)

/-- A citation reads fresh exactly for the fresh facts. -/
theorem observed_fresh_iff (f : verdict.Observed) :
    verdict.observed f ⦃ v => v = .Fresh ↔ FreshFact f ⦄ := by
  apply WP.spec_mono (observed_spec f)
  intro v hv; subst hv
  rcases f with _ | _ | t | b | o | o | b | _ | c
  all_goals try simp [observedOf, FreshFact]
  · rcases t with _ | ⟨k, l⟩ | a
    · simp [observedOf]
    · rcases l with _ | l
      · cases k <;> simp [observedOf, otherBlobOf]
      · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> try simp [observedOf, otherBlobOf]
        cases k <;> simp [observedOf, otherBlobOf]
    · rcases a with _ | l | _ | _ | _ | _ | _ <;> try simp [observedOf, missingOf]
      rcases l with _ | l
      · simp [missingOf]
      · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [missingOf]
  · cases b <;> simp [observedOf]
  · rcases o with _ | (_ | b) <;> try simp [observedOf]
    cases b <;> simp [observedOf]
  · rcases o with _ | b <;> try simp [observedOf]
    cases b <;> simp [observedOf]
  · cases b <;> simp [observedOf]
  · rcases c with _ | _ | _ | s | _ <;> try simp [observedOf]
    cases s <;> simp [observedOf]

/-- A span found at other lines reads moved, never fresh. -/
theorem moved_span_never_fresh (k : Bool) (a b : U32) :
    verdict.observed (.Code (.OtherBlob k (some (.At a b)))) ⦃ v => v = .Moved ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; simp [hv, observedOf, otherBlobOf]

/-- A file too long to search reads changed when the cited blob was
compared, unverifiable when it was not; never fresh. -/
theorem unsearched_never_fresh (k : Bool) :
    verdict.observed (.Code (.OtherBlob k (some .Unsearched)))
      ⦃ v => v = if k then .Changed else .Unverifiable ⦄ := by
  cases k
  · apply WP.spec_mono (observed_undecided (Knows.unsearched (some .Same))
      (Knows.unsearched (some .Absent)) (by simp [observedOf, otherBlobOf]))
    intro v hv; simpa using hv
  · apply WP.spec_mono (observed_spec _); intro v hv; simp [hv, observedOf, otherBlobOf]

/-- A gate citation away from the target commit reads changed. -/
theorem changed_gate_never_fresh :
    verdict.observed (.Gate false) ⦃ v => v = .Changed ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; simp [hv, observedOf]

/-- A checked kind reads fresh exactly when its checker said fresh. -/
theorem checker_fresh_iff (c : verdict.Checker) :
    verdict.observed (.Checked c) ⦃ v => v = .Fresh ↔ c = .Said .Fresh ⦄ := by
  apply WP.spec_mono (observed_fresh_iff _)
  intro v hv; rw [hv]; simp [FreshFact]

/-! ## Missing history is not evidence (#90) -/

/-- A follow that could not run (the cited commit is not in this clone,
there is no `rev=`, or the per-run limit was reached) reads unverifiable:
never gone, never changed. -/
theorem absence_never_evidence (a : verdict.Absence)
    (h : a = .RevAbsent ∨ a = .NoRev ∨ a = .Capped) :
    verdict.observed (.Code (.Missing a)) ⦃ v => v = .Unverifiable ⦄ := by
  have hu : Unfollowed a := by rcases h with h | h | h <;> simp [Unfollowed, h]
  exact observed_undecided (Knows.follow a .Renamed hu) (Knows.follow a .NoRename hu)
    (by simp [observedOf, missingOf])

/-- With no cited blob known, a citation reads changed only when a span
search ran and did not find the span, and never reads gone. -/
theorem unknown_blob_changed_only_by_span (l : Option verdict.Located) :
    verdict.observed (.Code (.OtherBlob false l))
      ⦃ v => (v = .Changed ↔ l = some .Absent) ∧ v ≠ .Gone ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; subst hv
  rcases l with _ | l
  · simp [observedOf, otherBlobOf]
  · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [observedOf, otherBlobOf]

/-- A located span is not followed; bytes this clone lacks are not a
completed follow (#182). -/
def Unlocated (l : Option verdict.Located) : Prop :=
  l ≠ some .Same ∧ (∀ a b, l ≠ some (.At a b)) ∧ l ≠ some .Unfetched

/-- Gone has a witness: the path is absent at the target, no exact rename
exists, and the follow completed (no rename, or a rename in which the cited
span was not found). -/
theorem gone_witness (t : verdict.Target) :
    verdict.observed (.Code t) ⦃ v => v = .Gone ↔
      (t = .Missing .NoRename ∨ ∃ l, t = .Missing (.Followed l) ∧ Unlocated l) ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; subst hv
  rcases t with _ | ⟨k, l⟩ | a
  · simp [observedOf]
  · rcases l with _ | l
    · cases k <;> simp [observedOf, otherBlobOf]
    · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> try simp [observedOf, otherBlobOf]
      cases k <;> simp [observedOf, otherBlobOf]
  · rcases a with _ | l | _ | _ | _ | _ | _ <;> try simp [observedOf, missingOf, Unlocated]
    rcases l with _ | l
    · simp [missingOf]
    · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [missingOf]

/-- Changed has a witness: a known cited blob that differs, or a completed
span search that did not find the span. -/
theorem changed_witness (t : verdict.Target) :
    verdict.observed (.Code t) ⦃ v => v = .Changed ↔
      ∃ k l, t = .OtherBlob k l ∧
        (l = some .Absent ∨ (k = true ∧ (l = none ∨ l = some .Unsearched))) ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; subst hv
  rcases t with _ | ⟨k, l⟩ | a
  · simp [observedOf]
  · rcases l with _ | l
    · cases k <;> simp [observedOf, otherBlobOf]
    · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> cases k <;> simp [observedOf, otherBlobOf]
  · rcases a with _ | l | _ | _ | _ | _ | _ <;> try simp [observedOf, missingOf]
    rcases l with _ | l
    · simp [missingOf]
    · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [missingOf]

/-- Supplying history only refines: knowing the cited blob changes a
verdict only where it was unverifiable, so every verdict decided from the
bytes at the target is the same with or without history. -/
theorem history_only_refines (l : Option verdict.Located) :
    verdict.observed (.Code (.OtherBlob false l))
      ⦃ v => v = observedOf (.Code (.OtherBlob true l)) ∨ v = .Unverifiable ⦄ := by
  exact observed_below (Knows.history l)

/-- A path absent at the target never reads fresh, renamed or not. -/
theorem rename_never_fresh (a : verdict.Absence) :
    verdict.observed (.Code (.Missing a)) ⦃ v => v ≠ .Fresh ⦄ := by
  apply WP.spec_mono (observed_spec _); intro v hv; subst hv
  rcases a with _ | l | _ | _ | _ | _ | _ <;> try simp [observedOf, missingOf]
  rcases l with _ | l
  · simp [missingOf]
  · rcases l with _ | ⟨_, _⟩ | _ | _ | _ <;> simp [missingOf]

/-! ## Partial clones (#182) -/

/-- A verdict that decides something about the cited content. -/
def decides : verdict.Verdict → Bool
  | .Fresh | .Moved | .Renamed | .Changed | .Gone => true
  | _ => false

/-- Bytes this clone lacks never decide: not fresh, moved, renamed,
changed or gone. -/
theorem unfetched_never_decides (k : Bool) :
    verdict.observed (.Code (.OtherBlob k (some .Unfetched))) ⦃ v => decides v = false ⦄ ∧
    verdict.observed (.Code (.Missing .Unfetched)) ⦃ v => decides v = false ⦄ ∧
    verdict.observed (.Code (.Missing (.Followed (some .Unfetched)))) ⦃ v => decides v = false ⦄ := by
  have hu : Unfollowed .Unfetched := by simp [Unfollowed]
  refine ⟨?_, ?_, ?_⟩
  · apply WP.spec_mono (observed_undecided (Knows.fetch k (some .Same))
      (Knows.fetch k (some .Absent)) (by cases k <;> simp [observedOf, otherBlobOf]))
    intro v hv; simp [hv, decides]
  · apply WP.spec_mono (observed_undecided (Knows.follow _ .Renamed hu)
      (Knows.follow _ .NoRename hu) (by simp [observedOf, missingOf]))
    intro v hv; simp [hv, decides]
  · apply WP.spec_mono (observed_undecided (Knows.followFetch (some .Same))
      (Knows.followFetch none) (by simp [observedOf, missingOf]))
    intro v hv; simp [hv, decides]

/-- What a partial clone decides from blob ids alone takes no fact about
contents: the same blob is fresh, an exact rename is renamed. -/
theorem identity_needs_no_bytes :
    verdict.observed (.Code .SameBlob) ⦃ v => v = .Fresh ⦄ ∧
    verdict.observed (.Code (.Missing .Renamed)) ⦃ v => v = .Renamed ⦄ := by
  refine ⟨?_, ?_⟩ <;>
    (apply WP.spec_mono (observed_spec _); intro v hv; subst hv; simp [observedOf, missingOf])

/-- Fetching only refines: replacing `Unfetched` by what a full clone
observes changes a verdict only where it was unverifiable. -/
theorem fetch_only_refines (k : Bool) (l : Option verdict.Located) (a : verdict.Absence) :
    verdict.observed (.Code (.OtherBlob k (some .Unfetched)))
      ⦃ v => v = observedOf (.Code (.OtherBlob k l)) ∨ v = .Unverifiable ⦄ ∧
    verdict.observed (.Code (.Missing .Unfetched))
      ⦃ v => v = observedOf (.Code (.Missing a)) ∨ v = .Unverifiable ⦄ := by
  exact ⟨observed_below (Knows.fetch k l),
    observed_below (Knows.follow .Unfetched a (by simp [Unfollowed]))⟩

/-! ## Marks -/

/-- Changed, gone and expired say more than a mark. -/
def Strong : verdict.Verdict → Bool
  | .Changed | .Gone | .Expired => true
  | _ => false

theorem with_mark_spec (v : verdict.Verdict) (m : Bool) :
    verdict.with_mark v m ⦃ r => r = if Strong v = true then v else if m then .Marked else v ⦄ := by
  unfold verdict.with_mark
  cases v <;> cases m <;> simp [Strong]

/-- A mark never hides a change. -/
theorem mark_keeps_change (v : verdict.Verdict) (m : Bool) (h : Strong v = true) :
    verdict.with_mark v m ⦃ r => r = v ⦄ := by
  apply WP.spec_mono (with_mark_spec v m); intro r hr; simp [hr, h]

/-- A marked citation never reads fresh. -/
theorem mark_never_fresh (v : verdict.Verdict) :
    verdict.with_mark v true ⦃ r => r ≠ .Fresh ⦄ := by
  apply WP.spec_mono (with_mark_spec v true)
  intro r hr; subst hr
  cases v <;> simp [Strong]

theorem decide_spec (f : verdict.Observed) (m : Bool) :
    verdict.decide f m ⦃ r => r = if Strong (observedOf f) = true then observedOf f
      else if m then .Marked else observedOf f ⦄ := by
  unfold verdict.decide
  step with observed_spec as ⟨ v, hv ⟩
  subst hv
  exact with_mark_spec _ m

/-! ## Checkers -/

/-- The handshake's outcome: the protocol first, then the kind, then the
operation. -/
def handshakeOf : Option verdict.Described → verdict.Handshake
  | none => .Protocol
  | some d => if !d.protocol then .Protocol else if !d.lists_kind then .Kind
      else if !d.has_op then .Op else .Agreed

theorem handshake_spec (d : Option verdict.Described) :
    verdict.handshake d ⦃ r => r = handshakeOf d ⦄ := by
  unfold verdict.handshake
  rcases d with _ | ⟨p, k, o⟩
  · simp [handshakeOf]
  · cases p <;> cases k <;> cases o <;> simp [handshakeOf]

theorem reply_spec (o : Option verdict.Said) :
    verdict.reply o ⦃ r => r = match o with | some s => .Said s | none => .Failed ⦄ := by
  unfold verdict.reply
  cases o <;> simp

/-- A reply counts exactly after an agreed handshake, a clean exit, a
readable version and exactly one well-formed reply. -/
theorem exchange_said_iff (h : verdict.Handshake) (clean readable : Bool)
    (replies : Slice (Option verdict.Said)) (s : verdict.Said) :
    verdict.exchange h clean readable replies ⦃ r => r = .Said s ↔
      h = .Agreed ∧ clean = true ∧ readable = true ∧ replies.val = [some s] ⦄ := by
  unfold verdict.exchange
  cases h <;> cases clean <;> cases readable <;> try simp
  split
  · rename_i hl
    step as ⟨ o, ho ⟩
    apply WP.spec_mono (reply_spec o)
    intro r hr; subst hr
    obtain ⟨x, hx⟩ := List.length_eq_one_iff.mp hl
    simp only [hx, List.getElem_cons_zero] at ho ⊢
    subst ho
    cases o <;> simp
  · rename_i hlen
    simp only [WP.spec_ok, reduceCtorEq, false_iff]
    intro he; apply hlen; simp [he]

/-- Otherwise the checker failed, or the version could not be read. -/
theorem exchange_spec (h : verdict.Handshake) (clean readable : Bool)
    (replies : Slice (Option verdict.Said)) :
    verdict.exchange h clean readable replies ⦃ r =>
      (r = .Version ↔ h = .Agreed ∧ clean = true ∧ readable = false) ∧
      r ≠ .Absent ∧ r ≠ .Expired ⦄ := by
  unfold verdict.exchange
  cases h <;> cases clean <;> cases readable <;> try simp
  split
  · step as ⟨ o, ho ⟩
    apply WP.spec_mono (reply_spec o)
    intro r hr; subst hr
    cases o <;> simp
  · simp

/-! ## Ageing and lifetimes -/

/-- A cached reply with a `max_age` is kept only while its age is known
and within it. -/
theorem aged_spec (c : verdict.Checker) (age max : Option I64) :
    verdict.aged c age max ⦃ r => r = match max with
      | none => c
      | some m => match age with
        | some a => if a.val ≤ m.val then c else .Expired
        | none => .Expired ⦄ := by
  unfold verdict.aged
  rcases max with _ | m
  · simp
  · rcases age with _ | a
    · simp
    · simp only
      split <;> rename_i h <;> simp only [WP.spec_ok] <;> split <;> first | rfl | scalar_tac

/-- With a `max_age`, a reply expires exactly when its age exceeds it or
cannot be read (or it had already expired). -/
theorem aged_expired_iff (c : verdict.Checker) (age : Option I64) (m : I64) :
    verdict.aged c age (some m) ⦃ r => r = .Expired ↔
      c = .Expired ∨ age = none ∨ ∃ a, age = some a ∧ a.val > m.val ⦄ := by
  apply WP.spec_mono (aged_spec c age (some m))
  intro r hr; subst hr
  rcases age with _ | a
  · simp
  · simp only
    split <;> rename_i h
    · simp; intro _; omega
    · simp; omega

theorem lifetime_spec {T : Type} (immutable : Bool) (token default : Option T) :
    verdict.lifetime immutable token default ⦃ r => r =
      if immutable then .Immutable else match token with
      | some t => .Token t
      | none => match default with
        | some d => .Default d
        | none => .Never ⦄ := by
  unfold verdict.lifetime
  cases immutable <;> rcases token with _ | t <;> rcases default with _ | d <;> simp

/-- A timed citation has expired exactly when `now - seen > ttl` over the
integers: no overflow, whatever the three values. -/
theorem expired_spec (now seen ttl : I64) :
    verdict.expired now seen ttl ⦃ b => b = true ↔ now.val - seen.val > ttl.val ⦄ := by
  unfold verdict.expired
  have hmax : core.num.I64.MAX.val = 2^63 - 1 := by simp [core.num.I64.MAX]; rfl
  have hmin : core.num.I64.MIN.val = -2^63 := by simp [core.num.I64.MIN]; rfl
  split
  · rename_i hs
    step as ⟨ i, hi ⟩
    split
    · rename_i hn; simp; scalar_tac
    · rename_i hn
      split
      · rename_i hs'; scalar_tac
      · step as ⟨ d, hd ⟩
        simp only [decide_eq_true_eq]
        constructor <;> intro h <;> scalar_tac
  · rename_i hs
    split
    · rename_i hs'
      step as ⟨ i, hi ⟩
      split
      · rename_i hn; simp; scalar_tac
      · step as ⟨ d, hd ⟩
        simp only [decide_eq_true_eq]
        constructor <;> intro h <;> scalar_tac
    · step as ⟨ d, hd ⟩
      simp only [decide_eq_true_eq]
      constructor <;> intro h <;> scalar_tac

/-! ## Where the cited span sits -/

/-- Which windows to search. -/
def WindowSpec (lines first last limit : Nat) : verdict.locate.Window → Prop
  | .Unsearched => lines > limit
  | .Absent => lines ≤ limit ∧ (first = 0 ∨ last < first ∨ last - first + 1 > lines)
  | .Search cited height => lines ≤ limit ∧ 0 < first ∧ first ≤ last ∧
      height.val = last - first + 1 ∧ height.val ≤ lines ∧
      (match cited with
       | some c => c.val = first - 1 ∧ last ≤ lines
       | none => lines < last)

theorem window_spec (lines first last limit : Usize) :
    verdict.locate.window lines first last limit ⦃ w =>
      WindowSpec lines.val first.val last.val limit.val w ⦄ := by
  unfold verdict.locate.window
  split
  · simp [WindowSpec]; scalar_tac
  · split
    · simp [WindowSpec]; scalar_tac
    · split
      · simp [WindowSpec]; scalar_tac
      · step as ⟨ d, hd ⟩
        step as ⟨ h, hh ⟩
        split
        · simp [WindowSpec]; scalar_tac
        · step as ⟨ c, hc ⟩
          split <;> simp [WindowSpec] <;> scalar_tac

theorem place_loop_spec (ws : Slice Bool) (i : Usize) (hi : i.val ≤ ws.val.length)
    (hbefore : ∀ j (hj : j < ws.val.length), j < i.val → ws.val[j] = false) :
    verdict.locate.place_loop ws i ⦃ p => match p with
      | .Same => False
      | .At s => ∃ h : s.val < ws.val.length, ws.val[s.val] = true ∧
          ∀ j (hj : j < ws.val.length), j < s.val → ws.val[j] = false
      | .Absent => ∀ b ∈ ws.val, b = false ⦄ := by
  unfold verdict.locate.place_loop
  apply loop.spec_decr_nat (fun i : Usize => ws.val.length - i.val)
    (fun i : Usize => i.val ≤ ws.val.length ∧
      ∀ j (hj : j < ws.val.length), j < i.val → ws.val[j] = false)
  · intro i ⟨hi, hbefore⟩
    unfold verdict.locate.place_loop.body
    dsimp only
    split
    · rename_i hlt
      step as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok]
        exact ⟨by scalar_tac, by rw [← hb]; exact hbt, hbefore⟩
      · rename_i hbt
        step as ⟨ i1, hi1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hji
        by_cases hjj : j < i.val
        · exact hbefore j hj hjj
        · have : j = i.val := by scalar_tac
          subst this
          rw [← hb]; simpa using hbt
    · rename_i hge
      simp only [WP.spec_ok]
      intro b hb
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hb
      exact hbefore j hj (by scalar_tac)
  · exact ⟨hi, hbefore⟩

/-- The cited window holds the span exactly when the answer is `Same`. -/
theorem place_same_iff (cited : Bool) (ws : Slice Bool) :
    verdict.locate.place cited ws ⦃ p => p = .Same ↔ cited = true ⦄ := by
  unfold verdict.locate.place
  cases cited
  · simp only [Bool.false_eq_true, ↓reduceIte, iff_false]
    apply WP.spec_mono (place_loop_spec ws 0#usize (by simp) (by simp))
    intro p hp; cases p <;> simp_all
  · simp

/-- Otherwise the first window from the top that holds it. -/
theorem place_at_iff (cited : Bool) (ws : Slice Bool) (s : Usize) :
    verdict.locate.place cited ws ⦃ p => p = .At s ↔ cited = false ∧
      ∃ h : s.val < ws.val.length, ws.val[s.val] = true ∧
        ∀ j (hj : j < ws.val.length), j < s.val → ws.val[j] = false ⦄ := by
  unfold verdict.locate.place
  cases cited
  · simp only [Bool.false_eq_true, ↓reduceIte, true_and]
    apply WP.spec_mono (place_loop_spec ws 0#usize (by simp) (by simp))
    intro p hp
    rcases p with _ | t | _
    · exact absurd hp id
    · obtain ⟨ht, htrue, hfirst⟩ := hp
      simp only [verdict.locate.Place.At.injEq]
      constructor
      · rintro rfl; exact ⟨ht, htrue, hfirst⟩
      · rintro ⟨hs, hstrue, hsfirst⟩
        apply UScalar.eq_of_val_eq
        by_contra hne
        rcases Nat.lt_or_gt_of_ne hne with hlt | hlt
        · have := hsfirst t.val ht hlt; simp_all
        · have := hfirst s.val hs hlt; simp_all
    · simp only [reduceCtorEq, false_iff, not_exists, not_and]
      intro hs hstrue _
      have := hp _ (List.getElem_mem hs); simp_all
  · simp

/-- No window holds it, and the cited one does not. -/
theorem place_absent_iff (cited : Bool) (ws : Slice Bool) :
    verdict.locate.place cited ws ⦃ p => p = .Absent ↔
      cited = false ∧ ∀ b ∈ ws.val, b = false ⦄ := by
  unfold verdict.locate.place
  cases cited
  · simp only [Bool.false_eq_true, ↓reduceIte, true_and]
    apply WP.spec_mono (place_loop_spec ws 0#usize (by simp) (by simp))
    intro p hp
    rcases p with _ | t | _
    · exact absurd hp id
    · obtain ⟨ht, htrue, _⟩ := hp
      simp only [reduceCtorEq, false_iff]
      intro hall; have := hall _ (List.getElem_mem ht); simp_all
    · simpa using hp
  · simp

end Writfold.Core.Verdict
