import Writfold.Conform.Words
import Writfold.Site
import Calculus.Site

/-!
# Links: the guard site is law S (#184)

`site::decide` is `Calculus.Site.decide` and `site::stricter` is
`Calculus.Posture.stricter`, through the words `basisOf` and `postureOf`,
each onto.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-- How far a site got, as the calculus's basis. -/
def basisOf : site.Basis → Calculus.Basis
  | .NoRules => .noRules
  | .NoMatch => .noMatch
  | .Judged d => .judged (decisionOf d)
  | .Undecided => .undecided

theorem basisOf_onto (b : Calculus.Basis) : ∃ s, basisOf s = b := by
  cases b with
  | noRules => exact ⟨.NoRules, rfl⟩
  | noMatch => exact ⟨.NoMatch, rfl⟩
  | judged d =>
    obtain ⟨g, hg⟩ := decisionOf_onto d
    exact ⟨.Judged g, by simp [basisOf, hg]⟩
  | undecided => exact ⟨.Undecided, rfl⟩

/-- A posture, as the calculus's posture. -/
def postureOf : site.Posture → Calculus.Posture
  | .Open => .open
  | .Ask => .ask
  | .Deny => .deny

theorem postureOf_onto (p : Calculus.Posture) : ∃ s, postureOf s = p := by
  cases p
  · exact ⟨.Open, rfl⟩
  · exact ⟨.Ask, rfl⟩
  · exact ⟨.Deny, rfl⟩

/-- `site::decide` is the calculus's site decision. -/
theorem site_decide_implements (b : site.Basis) (p : site.Posture) :
    site.decide b p ⦃ d => decisionOf d = Calculus.Site.decide (basisOf b) (postureOf p) ⦄ :=
  WP.spec_mono (Core.Site.decide_spec b p) (fun d hd => by
    subst hd
    cases b <;> cases p <;> rfl)

/-- `site::stricter` is the calculus's stricter posture. -/
theorem site_stricter_implements (a b : site.Posture) :
    site.stricter a b ⦃ r => postureOf r = Calculus.Posture.stricter (postureOf a) (postureOf b) ⦄ :=
  WP.spec_mono (Core.Site.stricter_spec a b) (fun r hr => by
    subst hr
    cases a <;> cases b <;> rfl)

end Writfold.Core.Conform
