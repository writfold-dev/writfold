/-!
# The words of the calculus (calculus.md 3.5, 3.8)

No import: this file is plain Lean and knows nothing of Rust, Aeneas or
the translation.
-/
namespace Calculus

/-- What a judgment says: it holds, it does not, or it could not be told. -/
inductive Tri where
  | yes | no | unknown
  deriving DecidableEq, Repr

namespace Tri

/-- Only `yes` admits. -/
def admits : Tri → Bool
  | yes => true
  | _ => false

/-- The knowledge order: `unknown` is below both answers. -/
def leb : Tri → Tri → Bool
  | unknown, _ => true
  | yes, yes => true
  | no, no => true
  | _, _ => false

instance : LE Tri := ⟨fun a b => leb a b = true⟩

instance (a b : Tri) : Decidable (a ≤ b) := inferInstanceAs (Decidable (leb a b = true))

/-- Both must hold: a refusal wins, then a doubt. -/
def and : Tri → Tri → Tri
  | no, _ => no
  | _, no => no
  | unknown, _ => unknown
  | _, unknown => unknown
  | yes, yes => yes

/-- Every one of them holds. Over none this is `yes`; a universal word
guards the empty case itself (`Calculus.universal`). -/
def all (ts : List Tri) : Tri := ts.foldr and yes

/-- Law 3.5: unknown never admits. -/
theorem unknown_never_admits : admits unknown = false := rfl

/-- Only `yes` admits. -/
theorem admits_iff (t : Tri) : t.admits = true ↔ t = yes := by
  cases t <;> simp [admits]

/-- The knowledge order is reflexive. -/
theorem le_refl (a : Tri) : a ≤ a := by cases a <;> decide
/-- The knowledge order is transitive. -/
theorem le_trans {a b c : Tri} : a ≤ b → b ≤ c → a ≤ c := by
  cases a <;> cases b <;> cases c <;> decide
/-- The knowledge order is antisymmetric. -/
theorem le_antisymm {a b : Tri} : a ≤ b → b ≤ a → a = b := by
  cases a <;> cases b <;> decide

/-- Knowing more changes an answer only where it was `unknown`. -/
theorem refines_only_unknown {a b : Tri} (h : a ≤ b) : a = b ∨ a = unknown := by
  revert h; cases a <;> cases b <;> decide

/-- Conjunction does not depend on order. -/
theorem and_comm (a b : Tri) : and a b = and b a := by cases a <;> cases b <;> rfl
/-- Conjunction groups freely. -/
theorem and_assoc (a b c : Tri) : and (and a b) c = and a (and b c) := by
  cases a <;> cases b <;> cases c <;> rfl
/-- Saying a word twice changes nothing. -/
theorem and_idem (a : Tri) : and a a = a := by cases a <;> rfl
/-- `yes` is the unit of conjunction. -/
theorem and_yes (a : Tri) : and yes a = a := by cases a <;> rfl

/-- Knowing more about either side never contradicts what was known. -/
theorem and_mono {a a' b b' : Tri} : a ≤ a' → b ≤ b' → and a b ≤ and a' b' := by
  cases a <;> cases a' <;> cases b <;> cases b' <;> decide

/-- Both hold exactly when each holds. -/
theorem and_eq_yes {a b : Tri} : and a b = yes ↔ a = yes ∧ b = yes := by
  cases a <;> cases b <;> simp [and]

/-- A conjunction is refused exactly when one side is refused. -/
theorem and_eq_no {a b : Tri} : and a b = no ↔ a = no ∨ b = no := by
  cases a <;> cases b <;> simp [and]

/-- All hold exactly when each one holds. -/
theorem all_eq_yes (ts : List Tri) : all ts = yes ↔ ∀ t ∈ ts, t = yes := by
  induction ts with
  | nil => simp [all]
  | cons t ts ih =>
    have : all (t :: ts) = and t (all ts) := rfl
    simp [this, and_eq_yes, ih]

/-- A refusal needs a witness. -/
theorem all_eq_no (ts : List Tri) : all ts = no ↔ no ∈ ts := by
  induction ts with
  | nil => simp [all]
  | cons t ts ih =>
    have : all (t :: ts) = and t (all ts) := rfl
    rw [this, and_eq_no, ih]; simp [eq_comm]

/-- All of two lists is the conjunction of each. -/
theorem all_append (xs ys : List Tri) : all (xs ++ ys) = and (all xs) (all ys) := by
  induction xs with
  | nil => simp [all, and_yes]
  | cons x xs ih =>
    have h1 : all (x :: xs ++ ys) = and x (all (xs ++ ys)) := rfl
    have h2 : all (x :: xs) = and x (all xs) := rfl
    rw [h1, h2, ih, and_assoc]

/-- Pointwise more knowledge gives more knowledge. -/
theorem all_mono : ∀ (xs ys : List Tri), xs.length = ys.length →
    (∀ p ∈ xs.zip ys, p.1 ≤ p.2) → all xs ≤ all ys
  | [], [], _, _ => le_refl _
  | [], _ :: _, h, _ => by simp at h
  | _ :: _, [], h, _ => by simp at h
  | x :: xs, y :: ys, hl, h => by
    have hx : x ≤ y := h (x, y) (by simp)
    have hr : all xs ≤ all ys :=
      all_mono xs ys (by simpa using hl) (fun p hp => h p (by simp [hp]))
    exact and_mono hx hr

/-- Unknown never admits, in any company. -/
theorem unknown_mem_never_admits {ts : List Tri} (h : unknown ∈ ts) :
    (all ts).admits = false := by
  cases hall : all ts <;> simp [admits]
  have := (all_eq_yes ts).mp hall _ h
  cases this

/-- Adding an obligation never makes an unmet set met (3.4). -/
theorem more_never_admits (xs ys : List Tri) :
    (all (xs ++ ys)).admits = true → (all xs).admits = true := by
  rw [all_append, admits_iff, admits_iff, and_eq_yes]; exact And.left

end Tri

/-- What a site answers. -/
inductive Decision where
  | pass | ask | deny
  deriving DecidableEq, Repr

namespace Decision

/-- Several rules: a denial beats a question, a question beats a pass. -/
def join : Decision → Decision → Decision
  | deny, _ => deny
  | _, deny => deny
  | ask, _ => ask
  | _, ask => ask
  | pass, pass => pass

/-- Several rules: the order they are read in does not matter. -/
theorem join_comm (a b : Decision) : join a b = join b a := by cases a <;> cases b <;> rfl
/-- Several rules group freely. -/
theorem join_assoc (a b c : Decision) : join (join a b) c = join a (join b c) := by
  cases a <;> cases b <;> cases c <;> rfl
/-- A rule repeated changes nothing. -/
theorem join_idem (a : Decision) : join a a = a := by cases a <;> rfl
/-- A pass adds nothing to another decision. -/
theorem pass_join (a : Decision) : join pass a = a := by cases a <;> rfl
/-- Several rules pass exactly when each passes. -/
theorem join_eq_pass {a b : Decision} : join a b = pass ↔ a = pass ∧ b = pass := by
  cases a <;> cases b <;> simp [join]

end Decision

/-- What a refusing rule does. It is never a pass. -/
inductive Action where
  | ask | deny
  deriving DecidableEq, Repr

def Action.decision : Action → Decision
  | .ask => .ask
  | .deny => .deny

/-- L4: an operation whose requirements are `reqs` passes only when every
one of them holds; otherwise the rule's action stands. -/
def admit (action : Action) (reqs : List Tri) : Decision :=
  if (Tri.all reqs).admits then .pass else action.decision

/-- An operation passes exactly when every requirement holds. -/
theorem admit_pass_iff (a : Action) (reqs : List Tri) :
    admit a reqs = .pass ↔ ∀ t ∈ reqs, t = .yes := by
  unfold admit
  rw [← Tri.all_eq_yes, ← Tri.admits_iff]
  cases (Tri.all reqs).admits <;> cases a <;> simp [Action.decision]

/-- Law 3.5 at L4: unknown never admits. -/
theorem unknown_never_passes (a : Action) {reqs : List Tri} (h : .unknown ∈ reqs) :
    admit a reqs ≠ .pass := by
  intro hp
  have := (admit_pass_iff a reqs).mp hp _ h
  cases this

end Calculus
