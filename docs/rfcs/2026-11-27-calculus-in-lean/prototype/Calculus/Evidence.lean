import Calculus.Word
/-!
# Evidence: classes, policy, one record decides (calculus.md 3.2 to 3.4, 3.13)
-/
namespace Calculus

/-- How a fact came to be recorded. `reported` is what a harness said
happened (law S); it is the weakest class and no policy takes it. -/
inductive Class where
  | checked | attested | waived | reported
  deriving DecidableEq, Repr

/-- What the deciding record of an obligation says now. -/
inductive Standing where
  | satisfied            -- a mechanical pass about the present
  | attested | waived | reported
  | unmet                -- open, stale or failed
  | unknown
  deriving DecidableEq, Repr

/-- The class a standing stands on, when it stands. -/
def Standing.cls : Standing → Option Class
  | .satisfied => some .checked
  | .attested => some .attested
  | .waived => some .waived
  | .reported => some .reported
  | _ => none

/-- 3.3 in one definition: the standing a record of class `c` can reach.
A record that is not about the present reaches none. -/
def stand (c : Class) (aboutPresent : Bool) (outcome : Tri) : Standing :=
  if !aboutPresent then .unmet
  else match c, outcome with
    | .checked, .yes => .satisfied
    | .checked, .no => .unmet
    | .checked, .unknown => .unknown
    | .attested, _ => .attested
    | .waived, _ => .waived
    | .reported, _ => .reported

/-- A class is never promoted. -/
theorem class_never_promoted (c : Class) (p : Bool) (o : Tri) (c' : Class)
    (h : (stand c p o).cls = some c') : c' = c := by
  revert h; cases c <;> cases p <;> cases o <;> simp [stand, Standing.cls] <;> exact fun h => h.symm

theorem satisfied_only_by_current_pass (c : Class) (p : Bool) (o : Tri)
    (h : stand c p o = .satisfied) : c = .checked ∧ p = true ∧ o = .yes := by
  revert h; cases c <;> cases p <;> cases o <;> simp [stand]

/-- A policy: the standings an obligation takes. There is no field for
`reported`, so no policy can take it. -/
structure Accept where
  satisfied : Bool
  attested : Bool
  waived : Bool
  deriving DecidableEq, Repr

def Accept.top : Accept := ⟨true, true, true⟩
def Accept.inf (a b : Accept) : Accept :=
  ⟨a.satisfied && b.satisfied, a.attested && b.attested, a.waived && b.waived⟩
def Accept.le (a b : Accept) : Prop :=
  (a.satisfied → b.satisfied) ∧ (a.attested → b.attested) ∧ (a.waived → b.waived)

/-- L2: does this standing meet the obligation? -/
def met (s : Standing) (a : Accept) : Bool :=
  match s with
  | .satisfied => a.satisfied
  | .attested => a.attested
  | .waived => a.waived
  | _ => false

/-- 3.4: narrowing is a meet, exactly. -/
theorem met_inf (s : Standing) (a b : Accept) : met s (a.inf b) = (met s a && met s b) := by
  cases s <;> simp [met, Accept.inf]

theorem met_narrows (s : Standing) {a b : Accept} (h : a.le b) : met s a = true → met s b = true := by
  cases s <;> simp [met] <;> first | exact h.1 | exact h.2.1 | exact h.2.2

theorem reported_never_met (a : Accept) : met .reported a = false := rfl
theorem unknown_never_met (a : Accept) : met .unknown a = false := rfl

theorem inf_comm (a b : Accept) : a.inf b = b.inf a := by
  cases a; cases b; simp [Accept.inf, Bool.and_comm]
theorem inf_assoc (a b c : Accept) : (a.inf b).inf c = a.inf (b.inf c) := by
  cases a; cases b; cases c; simp [Accept.inf, Bool.and_assoc]
theorem inf_idem (a : Accept) : a.inf a = a := by cases a; simp [Accept.inf]
theorem inf_top (a : Accept) : a.inf .top = a := by cases a; simp [Accept.inf, Accept.top]

/-- 3.2: of the records that count (bound, intact, of this subject), the
one with the greatest `seq` decides; a later one wins a tie. -/
def deciding {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) : Option α :=
  (rs.filter counts).foldl
    (fun best r => match best with
      | none => some r
      | some b => if seq b ≤ seq r then some r else some b) none

theorem deciding_none_iff {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) :
    deciding seq counts rs = none ↔ ∀ r ∈ rs, counts r = false := by
  unfold deciding
  have key : ∀ (l : List α) (init : Option α),
      l.foldl (fun best r => match best with
        | none => some r
        | some b => if seq b ≤ seq r then some r else some b) init = none ↔ init = none ∧ l = [] := by
    intro l
    induction l with
    | nil => intro init; simp
    | cons x xs ih =>
      intro init
      rw [List.foldl_cons, ih]
      cases init <;> simp <;> split <;> simp
  rw [key]
  simp [List.filter_eq_nil_iff]

/-- A record that does not count never changes which record decides. -/
theorem uncounted_never_decides {α : Type} (seq : α → Nat) (counts : α → Bool)
    (rs : List α) (r : α) (h : counts r = false) :
    deciding seq counts (r :: rs) = deciding seq counts rs := by
  simp [deciding, h]

end Calculus
