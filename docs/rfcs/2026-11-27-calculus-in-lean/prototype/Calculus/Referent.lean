import Calculus.Word
/-!
# Referents (law R, calculus.md 3.11)

Which thing a fact is about is a decision with three outcomes. The type of
referents `ρ` and how candidates are found are parameters: a commit of a
repository, a run, a directory, a session, an actor. A provider of assets
supplies the candidates; nothing here knows about Git.
-/
namespace Calculus

/-- The outcome of choosing a referent. -/
inductive Resolution (ρ : Type) where
  | one (r : ρ)
  | ambiguous
  | none
  deriving DecidableEq, Repr

def Resolution.resolved {ρ : Type} : Resolution ρ → Bool
  | .one _ => true
  | _ => false

/-- One candidate the caller found. `level` says how strongly it names the
asset asked about (an operator's pin above a URL that equals the cited one,
above a last path segment). `referent` is what it offers to judge against;
`none` is a candidate that names the asset and offers nothing (a remote with
no default branch, a pin that does not resolve, a pointer to a closed run). -/
structure Candidate (ρ : Type) where
  level : Nat
  referent : Option ρ

/-- No candidate names the asset more strongly. -/
def Candidate.top {ρ : Type} (cs : List (Candidate ρ)) (c : Candidate ρ) : Bool :=
  cs.all fun c' => decide (c'.level ≤ c.level)

/-- The reference chooser: the strongest level present decides, alone. Its
candidates must all offer the same referent. There is no fall-back to a
weaker level, and no first-listed winner. -/
def resolve {ρ : Type} [DecidableEq ρ] (cs : List (Candidate ρ)) : Resolution ρ :=
  match cs.filter (Candidate.top cs) with
  | [] => .none
  | c :: rest =>
    match c.referent with
    | .none => .ambiguous
    | .some r => if rest.all (fun c' => c'.referent == some r) then .one r else .ambiguous

variable {ρ : Type} [DecidableEq ρ]

theorem resolve_nil : resolve ([] : List (Candidate ρ)) = .none := rfl

/-- What `resolve` chose was offered, at the deciding level. -/
theorem resolve_one_mem {cs : List (Candidate ρ)} {r : ρ} (h : resolve cs = .one r) :
    ∃ c ∈ cs, Candidate.top cs c = true ∧ c.referent = some r := by
  unfold resolve at h
  split at h
  · cases h
  · rename_i c rest hf
    have hc : c ∈ cs.filter (Candidate.top cs) := by rw [hf]; exact List.mem_cons_self
    rw [List.mem_filter] at hc
    split at h
    · cases h
    · rename_i r' hr
      split at h
      · cases h; exact ⟨c, hc.1, hc.2, hr⟩
      · cases h

/-- Every candidate at the deciding level agreed: two that disagree, or one
that offers nothing, never give a referent. -/
theorem resolve_one_agree {cs : List (Candidate ρ)} {r : ρ} (h : resolve cs = .one r)
    {c : Candidate ρ} (hc : c ∈ cs) (ht : Candidate.top cs c = true) : c.referent = some r := by
  unfold resolve at h
  have hm : c ∈ cs.filter (Candidate.top cs) := List.mem_filter.mpr ⟨hc, ht⟩
  split at h
  · rename_i hf; rw [hf] at hm; cases hm
  · rename_i c0 rest hf
    rw [hf] at hm
    split at h
    · cases h
    · rename_i r' hr
      split at h
      · rename_i hall
        cases h
        rcases List.mem_cons.mp hm with rfl | hin
        · exact hr
        · simpa using List.all_eq_true.mp hall c hin
      · cases h

/-- The choice does not depend on the order the candidates were found in:
two listings with the same members never resolve to different referents. -/
theorem resolve_order_free {cs ds : List (Candidate ρ)} {r s : ρ}
    (hmem : ∀ c, c ∈ cs ↔ c ∈ ds) (hc : resolve cs = .one r) (hd : resolve ds = .one s) :
    r = s := by
  obtain ⟨c, hcm, htop, hr⟩ := resolve_one_mem hc
  have htop' : Candidate.top ds c = true := by
    simp only [Candidate.top, List.all_eq_true, decide_eq_true_eq] at htop ⊢
    intro c' hc'; exact htop c' ((hmem c').mpr hc')
  have := resolve_one_agree hd ((hmem c).mp hcm) htop'
  rw [hr] at this
  exact Option.some.inj this

/-- A vocabulary of verdicts: the word for "could not be told", and which
words claim change or absence (`changed`, `gone`, `moved`, `failed`,
`unmet`). -/
structure Words (W : Type) where
  unknown : W
  claimsChange : W → Bool
  unknown_claims_nothing : claimsChange unknown = false

/-- The three-valued word as a vocabulary. -/
def Tri.words : Words Tri := ⟨.unknown, fun t => t == .no, rfl⟩

/-- What a referent can support. A verdict stands only over a resolved
referent, and one that claims change only over a referent that could have
shown the opposite (`refutable`: the snapshot judged against is not behind
the cited one). -/
def settle {W : Type} (ws : Words W) (resolved refutable : Bool) (w : W) : W :=
  if !resolved then ws.unknown
  else if ws.claimsChange w && !refutable then ws.unknown
  else w

/-- Law A (asset boundary): a verdict on a cited asset is
`judge(resolved, relation, fact)`. A provider supplies the three inputs
and no verdict: there is no argument through which it could. `Fact` and
`observe` are parameters, so nothing here knows what a provider is. -/
def judge {W Fact : Type} (ws : Words W) (observe : Fact → W)
    (resolved refutable : Bool) (fact : Fact) : W :=
  settle ws resolved refutable (observe fact)

variable {W Fact : Type} (ws : Words W)

/-- The boundary takes verdicts away; it never makes one. -/
theorem settle_only_withholds (r f : Bool) (w : W) :
    settle ws r f w = w ∨ settle ws r f w = ws.unknown := by
  unfold settle; cases r <;> cases f <;> cases ws.claimsChange w <;> simp

theorem unresolved_never_judged (f : Bool) (w : W) : settle ws false f w = ws.unknown := rfl

/-- A claim of change needs a resolved referent that could have shown the
opposite. -/
theorem change_needs_refutable (r f : Bool) (w : W)
    (h : ws.claimsChange (settle ws r f w) = true) : r = true ∧ f = true := by
  revert h; unfold settle
  cases r <;> cases f <;> cases hc : ws.claimsChange w <;>
    simp [hc, ws.unknown_claims_nothing]

theorem settle_identity (w : W) : settle ws true true w = w := by
  unfold settle; cases ws.claimsChange w <;> rfl

/-- Words that claim no change survive a referent that is behind. -/
theorem unclaimed_survives (f : Bool) (w : W) (h : ws.claimsChange w = false) :
    settle ws true f w = w := by
  unfold settle; cases f <;> simp [h]

/-- Knowing more about the referent changes a verdict only where it was
unknown: a provider that knows less never contradicts one that knows more. -/
theorem settle_refines (r f r' f' : Bool) (w : W) (hr : r = true → r' = true)
    (hf : f = true → f' = true) :
    settle ws r f w = settle ws r' f' w ∨ settle ws r f w = ws.unknown := by
  unfold settle
  cases r <;> cases f <;> cases r' <;> cases f' <;> cases ws.claimsChange w <;> simp_all

theorem judge_needs (observe : Fact → W) (r f : Bool) (fact : Fact)
    (h : judge ws observe r f fact ≠ ws.unknown) :
    r = true ∧ judge ws observe r f fact = observe fact := by
  revert h; unfold judge settle
  cases r <;> cases f <;> cases ws.claimsChange (observe fact) <;> simp

/-- A judgment about a chosen referent: unresolved is unknown. -/
def about {ρ : Type} (res : Resolution ρ) (judgeAt : ρ → Tri) : Tri :=
  match res with
  | .one r => judgeAt r
  | _ => .unknown

theorem unresolved_unknown {ρ : Type} (res : Resolution ρ) (judgeAt : ρ → Tri)
    (h : res.resolved = false) : about res judgeAt = .unknown := by
  cases res <;> simp_all [about, Resolution.resolved]

end Calculus
