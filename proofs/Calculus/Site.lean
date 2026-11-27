import Calculus.Word
import Calculus.Evidence
/-!
# Sites (law S, calculus.md 3.8 and 3.13)
-/
namespace Calculus

/-- What kind of call an event is, in Writfold's own form. -/
inductive Kind where
  | shell | fileWrite | fileRead | skill | spawn | tool | stop
  deriving DecidableEq, Repr

/-- An event as a site is handed it. The fields a harness cannot vouch for
are parameters: `Subject` (command words, a path, a name), `Session`,
`Actor`, and `Issuer` (the adapter that reported it). -/
structure Event (Subject Session Actor Issuer : Type) where
  kind : Kind
  subject : Subject
  session : Option Session
  actor : Option Actor
  issuer : Issuer

/-- A remembered event is a fact of the reported class, whatever it says. -/
def Event.cls {S T A I : Type} (_ : Event S T A I) : Class := .reported

/-- How far the site got with one event. -/
inductive Basis where
  | noRules
  | noMatch
  | judged (d : Decision)
  | undecided
  deriving DecidableEq, Repr

/-- What an undecided event gets. `open` is a declared exemption from
"unknown never admits"; nothing else is. -/
inductive Posture where
  | open | ask | deny
  deriving DecidableEq, Repr

def Posture.undecided : Posture → Decision
  | .open => .pass
  | .ask => .ask
  | .deny => .deny

def Site.decide (b : Basis) (p : Posture) : Decision :=
  match b with
  | .noRules | .noMatch => .pass
  | .judged d => d
  | .undecided => p.undecided

/-- The stricter of two postures. -/
def Posture.stricter : Posture → Posture → Posture
  | .deny, _ | _, .deny => .deny
  | .ask, _ | _, .ask => .ask
  | .open, .open => .open

/-- A posture never changes a judged decision. -/
theorem judged_unchanged (d : Decision) (p q : Posture) :
    Site.decide (.judged d) p = Site.decide (.judged d) q := rfl

/-- Under a closed posture an undecided event never passes. -/
theorem closed_pass_iff (b : Basis) (p : Posture) (h : p ≠ .open) :
    Site.decide b p = .pass ↔ b = .noRules ∨ b = .noMatch ∨ b = .judged .pass := by
  cases b <;> cases p <;> simp_all [Site.decide, Posture.undecided]

/-- A posture only narrows: what passes under any posture passes under `open`. -/
theorem posture_only_narrows (b : Basis) (p : Posture) (h : Site.decide b p = .pass) :
    Site.decide b .open = .pass := by
  revert h; cases b <;> cases p <;> simp [Site.decide, Posture.undecided]

/-- The posture is the decision's upper bound in `join`: a stricter posture
never loosens a decision. -/
theorem stricter_never_loosens (b : Basis) (p q : Posture) :
    Decision.join (Site.decide b p) (Site.decide b (p.stricter q)) = Site.decide b (p.stricter q) := by
  cases b with
  | judged d => exact Decision.join_idem d
  | _ => cases p <;> cases q <;> rfl

/-- Combining postures does not depend on order. -/
theorem stricter_comm (a b : Posture) : a.stricter b = b.stricter a := by cases a <;> cases b <;> rfl
/-- Postures combine freely. -/
theorem stricter_assoc (a b c : Posture) : (a.stricter b).stricter c = a.stricter (b.stricter c) := by
  cases a <;> cases b <;> cases c <;> rfl
/-- `open` adds nothing to another posture. -/
theorem stricter_open_left (a : Posture) : Posture.open.stricter a = a := by cases a <;> rfl

/-- Under `open`, the only pass without a decision is the undecided row:
the exemption is declared, never implied. -/
theorem open_is_declared (b : Basis) (h : Site.decide b .open = .pass)
    (hn : b ≠ .noRules) (hm : b ≠ .noMatch) (hj : b ≠ .judged .pass) : b = .undecided := by
  cases b with
  | noRules => exact absurd rfl hn
  | noMatch => exact absurd rfl hm
  | judged d => simp [Site.decide] at h; subst h; exact absurd rfl hj
  | undecided => rfl

end Calculus
