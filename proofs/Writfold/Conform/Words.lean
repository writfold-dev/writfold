import Extracted.WritfoldCore.Types
import Calculus.Word
import Calculus.Account
import Calculus.Evidence
import Extracted.WritfoldCore.Policy
import Extracted.WritfoldCore.Account

/-!
# The dictionary (#206, RFC 2026-11-27-calculus-in-lean)

One abstraction per Rust word type onto a word of the calculus, each with
an `_onto` theorem: every core word is the image of some Rust word, so no
abstraction is a constant that would make a link vacuous.
-/

open writfold_core

namespace Writfold.Core.Conform

/-- A guard requirement: met, unmet or not judged. -/
def factTri : guard.Fact → Calculus.Tri
  | .Met => .yes
  | .Unmet => .no
  | .Unknown => .unknown

theorem factTri_onto (t : Calculus.Tri) : ∃ f, factTri f = t := by
  cases t
  · exact ⟨.Met, rfl⟩
  · exact ⟨.Unmet, rfl⟩
  · exact ⟨.Unknown, rfl⟩

/-- A site's answer. -/
def decisionOf : guard.Decision → Calculus.Decision
  | .Pass => .pass
  | .Ask => .ask
  | .Deny => .deny

theorem decisionOf_onto (d : Calculus.Decision) : ∃ g, decisionOf g = d := by
  cases d
  · exact ⟨.Pass, rfl⟩
  · exact ⟨.Ask, rfl⟩
  · exact ⟨.Deny, rfl⟩

/-- What a refusing rule does. -/
def actionOf : guard.Action → Calculus.Action
  | .Ask => .ask
  | .Deny => .deny

theorem actionOf_onto (a : Calculus.Action) : ∃ g, actionOf g = a := by
  cases a
  · exact ⟨.Ask, rfl⟩
  · exact ⟨.Deny, rfl⟩

/-- A check's outcome: a pass, a fail or unknown. -/
def outcomeTri : gate.Outcome → Calculus.Tri
  | .Pass => .yes
  | .Fail => .no
  | .Unknown => .unknown

theorem outcomeTri_onto (t : Calculus.Tri) : ∃ o, outcomeTri o = t := by
  cases t
  · exact ⟨.Pass, rfl⟩
  · exact ⟨.Fail, rfl⟩
  · exact ⟨.Unknown, rfl⟩

/-- An inventory root as a member of the inventory's universe: read (listed
or absent) is judged, unreadable is the remainder. -/
def rootMember : inventory.Root → Calculus.Member
  | .Unreadable => .remainder
  | _ => .judged

/-- A gate's standing as the calculus's standing. -/
def standingOf : gate.Standing → Calculus.Standing
  | .Checked .Satisfied => .satisfied
  | .Checked .Unknown => .unknown
  | .Checked _ => .unmet
  | .Attested => .attested
  | .Waived => .waived

/-- Onto every standing a gate can reach: `reported` (law S) is a class a
gate never records, so it has no Rust word. -/
theorem standingOf_onto (s : Calculus.Standing) (h : s ≠ .reported) : ∃ g, standingOf g = s := by
  cases s
  · exact ⟨.Checked .Satisfied, rfl⟩
  · exact ⟨.Attested, rfl⟩
  · exact ⟨.Waived, rfl⟩
  · exact absurd rfl h
  · exact ⟨.Checked .Open, rfl⟩
  · exact ⟨.Checked .Unknown, rfl⟩

/-- A policy's accepted standings. -/
def acceptOf (a : policy.Accept) : Calculus.Accept := ⟨a.satisfied, a.attested, a.waived⟩

theorem acceptOf_onto (a : Calculus.Accept) : ∃ p, acceptOf p = a :=
  ⟨⟨a.satisfied, a.attested, a.waived⟩, rfl⟩

/-- An account's member as the calculus's member. -/
def memberOf : account.Member → Calculus.Member
  | .Judged => .judged
  | .DeclaredOut => .declaredOut
  | .Remainder => .remainder

theorem memberOf_onto (m : Calculus.Member) : ∃ a, memberOf a = m := by
  cases m
  · exact ⟨.Judged, rfl⟩
  · exact ⟨.DeclaredOut, rfl⟩
  · exact ⟨.Remainder, rfl⟩

end Writfold.Core.Conform
