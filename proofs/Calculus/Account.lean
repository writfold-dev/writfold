import Calculus.Word
/-!
# Accounting for a universe (law U, calculus.md 3.12; 3.5 and 3.9)
-/
namespace Calculus

/-- Every member of a universe is exactly one of these. -/
inductive Member where
  | judged | declaredOut | remainder
  deriving DecidableEq, Repr

/-- How many members were neither judged nor declared out. -/
def remainder (ms : List Member) : Nat := ms.countP (· = .remainder)

/-- Nothing is left over. -/
def whole (ms : List Member) : Bool := remainder ms == 0

theorem remainder_append (xs ys : List Member) :
    remainder (xs ++ ys) = remainder xs + remainder ys := by
  simp [remainder, List.countP_append]

theorem whole_iff_no_remainder (ms : List Member) :
    whole ms = true ↔ Member.remainder ∉ ms := by
  simp only [whole, remainder, beq_iff_eq, List.countP_eq_zero, decide_eq_true_eq]
  exact ⟨fun h hm => h _ hm rfl, fun h a ha e => h (e ▸ ha)⟩

theorem whole_append (xs ys : List Member) :
    whole (xs ++ ys) = (whole xs && whole ys) := by
  simp only [whole, remainder_append]
  cases remainder xs <;> cases remainder ys <;> simp

theorem judged_never_counts (ms : List Member) :
    remainder (.judged :: ms) = remainder ms := by simp [remainder]

theorem declared_out_never_counts (ms : List Member) :
    remainder (.declaredOut :: ms) = remainder ms := by simp [remainder]

/-- An account by counts, for kernels that are handed numbers. -/
def Member.counts (judged out rest : Nat) : List Member :=
  List.replicate judged .judged ++ List.replicate out .declaredOut ++
    List.replicate rest .remainder

theorem remainder_counts (j o r : Nat) : remainder (Member.counts j o r) = r := by
  simp [Member.counts, remainder, List.countP_append, List.countP_replicate]

theorem whole_counts (j o r : Nat) : whole (Member.counts j o r) = true ↔ r = 0 := by
  simp [whole, remainder_counts]

/-- The judged members' answers. -/
def judgedOf (ms : List (Member × Tri)) : List Tri :=
  ms.filterMap fun m => if m.1 = .judged then some m.2 else none

/-- A word that claims all of something ("complete", "ready", "covered",
"nothing matched"). A judged member that fails is a witness, whole or not.
Short of that the word needs the whole universe, and a member to speak of
unless the empty universe was allowed. -/
def universal (allowEmpty : Bool) (ms : List (Member × Tri)) : Tri :=
  if .no ∈ judgedOf ms then .no
  else if !whole (ms.map (·.1)) then .unknown
  else if (judgedOf ms).isEmpty && !allowEmpty then .unknown
  else Tri.all (judgedOf ms)

/-- Law U as a property of any word said of an accounted universe: only
the `yes` row is constrained. A kernel whose table differs from `universal`
off that row (a `fail` where a member is left over, a `false` for the empty
universe) proves this instead of equality. -/
structure Universal (allowEmpty : Bool) (ms : List (Member × Tri)) (word : Tri) : Prop where
  accounted : word = .yes → Calculus.whole (ms.map (·.1)) = true
  each : word = .yes → ∀ t ∈ judgedOf ms, t = .yes
  inhabited : word = .yes → allowEmpty = true ∨ judgedOf ms ≠ []

theorem universal_yes_iff (e : Bool) (ms : List (Member × Tri)) :
    universal e ms = .yes ↔
      whole (ms.map (·.1)) = true ∧ (∀ t ∈ judgedOf ms, t = .yes) ∧
        (e = true ∨ judgedOf ms ≠ []) := by
  unfold universal
  by_cases hno : Tri.no ∈ judgedOf ms
  · simp only [hno, ↓reduceIte, reduceCtorEq, false_iff, not_and]
    intro _ hall; cases hall _ hno
  · by_cases hw : whole (ms.map (·.1)) = true
    · by_cases he : (judgedOf ms).isEmpty && !e
      · simp only [hno, hw, he, ↓reduceIte, Bool.not_true, Bool.false_eq_true, reduceCtorEq, false_iff]
        simp only [Bool.and_eq_true, List.isEmpty_iff, Bool.not_eq_true'] at he
        simp [he.1, he.2]
      · simp only [hno, hw, he, ↓reduceIte, Bool.not_true, Bool.false_eq_true, Tri.all_eq_yes, true_and]
        simp only [Bool.and_eq_true, List.isEmpty_iff, Bool.not_eq_true', not_and, Bool.not_eq_false] at he
        constructor
        · intro h; refine ⟨h, ?_⟩
          by_cases hj : judgedOf ms = []
          · exact Or.inl (he hj)
          · exact Or.inr hj
        · exact And.left
    · simp [hno, hw]

/-- Law U: the word holds only when the remainder is zero. -/
theorem remainder_never_whole (e : Bool) (ms : List (Member × Tri))
    (h : Member.remainder ∈ ms.map (·.1)) : universal e ms ≠ .yes := by
  intro hy
  exact (whole_iff_no_remainder _).mp ((universal_yes_iff e ms).mp hy).1 h

/-- 3.5: an empty universe is never "all met" unless that was allowed. -/
theorem empty_never_yes (ms : List (Member × Tri)) (h : judgedOf ms = []) :
    universal false ms ≠ .yes := by
  intro hy
  rcases ((universal_yes_iff false ms).mp hy).2.2 with h' | h'
  · cases h'
  · exact h' h

theorem universal_sound (e : Bool) (ms : List (Member × Tri)) :
    Universal e ms (universal e ms) :=
  ⟨fun h => ((universal_yes_iff e ms).mp h).1, fun h => ((universal_yes_iff e ms).mp h).2.1,
    fun h => ((universal_yes_iff e ms).mp h).2.2⟩

/-- A failing member is a witness in any universe, whole or not. -/
theorem universal_no_of_witness (e : Bool) (ms : List (Member × Tri))
    (h : Tri.no ∈ judgedOf ms) : universal e ms = .no := by
  simp [universal, h]

/-- The mirror: a word that claims some member exists ("saw", "found").
A witness is enough in any universe; saying there is none needs all of it. -/
def search (isWhole found : Bool) : Tri :=
  if found then .yes else if isWhole then .no else .unknown

theorem search_yes_iff (w f : Bool) : search w f = .yes ↔ f = true := by
  cases w <;> cases f <;> simp [search]

/-- A partial universe never proves that something is absent. -/
theorem absence_needs_whole (w f : Bool) (h : search w f = .no) : w = true := by
  revert h; cases w <;> cases f <;> simp [search]

end Calculus
