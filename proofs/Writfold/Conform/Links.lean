import Writfold.Conform.Words
import Writfold.Guard
import Writfold.Walk
import Writfold.Packet
import Writfold.Runs
import Writfold.Inventory
import Writfold.Account

/-!
# Links: shipped kernels shown to implement the calculus (#206)

Each `_implements` theorem is a Hoare triple over a translated function
whose postcondition names a calculus operation, derived from the kernel's
existing spec theorem; no existing proof is touched. A law of the calculus
then holds of the kernel through the link (`guard_unknown_never_passes`).
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-! ## Admission: `guard::decide` is `Calculus.admit`; `combine` is `join` -/

theorem all_metF (fs : List guard.Fact) :
    (Calculus.Tri.all (fs.map factTri)).admits = fs.all Core.Guard.metF := by
  rw [Bool.eq_iff_iff, Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes, List.all_eq_true]
  constructor
  · intro h f hf
    have := h (factTri f) (List.mem_map_of_mem hf)
    cases f <;> simp_all [factTri, Core.Guard.metF]
  · intro h t ht
    obtain ⟨f, hf, rfl⟩ := List.mem_map.mp ht
    have := h f hf
    cases f <;> simp_all [factTri, Core.Guard.metF]

/-- A matched rule admits as `Calculus.admit`; an unmatched rule passes. -/
theorem decide_implements (m : Bool) (a : guard.Action) (fs : Slice guard.Fact) :
    guard.decide m a fs ⦃ d =>
      decisionOf d = if m then Calculus.admit (actionOf a) (fs.val.map factTri) else .pass ⦄ := by
  apply WP.spec_mono (Core.Guard.decide_spec m a fs)
  intro d hd
  rw [hd]
  cases m
  · simp [decisionOf]
  · simp only [Bool.not_true, Bool.false_or, ite_true, Calculus.admit, all_metF]
    cases fs.val.all Core.Guard.metF <;> cases a <;>
      simp [decisionOf, actionOf, Core.Guard.actionOf, Calculus.Action.decision]

/-- Law 3.5 at the guard, transferred from `Calculus.unknown_never_passes`. -/
theorem guard_unknown_never_passes (a : guard.Action) (fs : Slice guard.Fact)
    (h : guard.Fact.Unknown ∈ fs.val) :
    guard.decide true a fs ⦃ d => decisionOf d ≠ .pass ⦄ := by
  apply WP.spec_mono (decide_implements true a fs)
  intro d hd; rw [hd]
  exact Calculus.unknown_never_passes _ (List.mem_map.mpr ⟨_, h, rfl⟩)

theorem combine_implements (a b : guard.Decision) :
    guard.combine a b ⦃ d =>
      decisionOf d = Calculus.Decision.join (decisionOf a) (decisionOf b) ⦄ := by
  apply WP.spec_mono (Core.Guard.combine_spec a b)
  intro d hd; rw [hd]
  cases a <;> cases b <;> rfl

/-! ## Law U: the "complete" kernels are `Calculus.whole` -/

theorem walk_complete_implements (frontier skipped : Usize) :
    walk.complete frontier skipped ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts 0 0 (frontier.val + skipped.val)) ⦄ := by
  apply WP.spec_mono (Core.Walk.complete_iff frontier skipped)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  omega

theorem walk_complete_within_implements (frontier skipped : Usize) (truncated : Bool) :
    walk.complete_within frontier skipped truncated ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts 0 0
        (frontier.val + skipped.val + if truncated then 1 else 0)) ⦄ := by
  apply WP.spec_mono (Core.Walk.complete_within_iff frontier skipped truncated)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  cases truncated <;> simp <;> omega

theorem packet_complete_implements (kept required unreadable unread : Usize) :
    packet.complete kept required unreadable unread ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts kept.val 0
        ((required.val - kept.val) + unreadable.val + unread.val)) ⦄ := by
  apply WP.spec_mono (Core.Packet.complete_iff kept required unreadable unread)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  omega

theorem leftCount_remainder (ms : List account.Member) :
    Core.Account.leftCount ms = Calculus.remainder (ms.map memberOf) := by
  induction ms with
  | nil => rfl
  | cons m ms ih =>
    cases m <;> simp_all [Core.Account.leftCount, Core.Account.leftB, Calculus.remainder,
      memberOf, List.countP_cons]

/-- Born linked (#181): `account::remainder` is `Calculus.remainder`. -/
theorem remainder_implements (ms : Slice account.Member) :
    account.remainder ms ⦃ r => r.val = Calculus.remainder (ms.val.map memberOf) ⦄ := by
  apply WP.spec_mono (Core.Account.remainder_spec ms)
  intro r hr; rw [hr, leftCount_remainder]

/-- `account::whole` is `Calculus.whole`. -/
theorem whole_implements (ms : Slice account.Member) :
    account.whole ms ⦃ b => b = Calculus.whole (ms.val.map memberOf) ⦄ := by
  unfold account.whole
  step with remainder_implements as ⟨ r, hr ⟩
  simp only [WP.spec_ok, Calculus.whole]
  rw [← hr]
  cases h : (r == 0#usize) <;> simp_all <;> scalar_tac

theorem runs_complete_implements (u n : Usize) :
    runs.complete u n ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts 0 0 (u.val + n.val)) ⦄ := by
  apply WP.spec_mono (Core.Runs.complete_spec u n)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  omega

theorem inventory_complete_implements (roots : Slice inventory.Root) :
    inventory.complete roots ⦃ b => b = Calculus.whole (roots.val.map rootMember) ⦄ := by
  apply WP.spec_mono (Core.Inventory.complete_spec roots)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_iff_no_remainder]
  constructor
  · intro h hm
    obtain ⟨r, hr, he⟩ := List.mem_map.mp hm
    cases r with
    | Unreadable => exact h _ hr rfl
    | Absent => simp [rootMember] at he
    | Listed => simp [rootMember] at he
  · intro h r hr hu
    subst hu
    exact h (List.mem_map.mpr ⟨_, hr, rfl⟩)

end Writfold.Core.Conform
