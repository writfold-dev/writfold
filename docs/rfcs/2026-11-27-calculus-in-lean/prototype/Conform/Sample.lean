import Writfold.Guard
import Writfold.Walk
import Writfold.Packet
import Writfold.Runs
import Writfold.Policy
import Writfold.Pointer
import Calculus.Word
import Calculus.Account
import Calculus.Referent
import Calculus.Evidence
/-!
# Conformance, a sample: six shipped kernels shown to implement the core

Each theorem is a Hoare triple over the translated Rust whose postcondition
names a core operation. Each is derived from the kernel's existing spec
theorem; no existing proof is touched.
-/
open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Conform

/-! ## The dictionary: one abstraction per Rust word -/

def factTri : guard.Fact → Calculus.Tri
  | .Met => .yes
  | .Unmet => .no
  | .Unknown => .unknown

def decisionOf : guard.Decision → Calculus.Decision
  | .Pass => .pass
  | .Ask => .ask
  | .Deny => .deny

def actionOf : guard.Action → Calculus.Action
  | .Ask => .ask
  | .Deny => .deny

def standingOf : gate.Standing → Calculus.Standing
  | .Checked .Satisfied => .satisfied
  | .Checked .Unknown => .unknown
  | .Checked _ => .unmet
  | .Attested => .attested
  | .Waived => .waived

def acceptOf (a : policy.Accept) : Calculus.Accept := ⟨a.satisfied, a.attested, a.waived⟩

/-! ## Transfer: a law of the core is a law of the kernel -/

/-- If a kernel's abstracted result equals a core operation, every property
of that operation holds of the kernel's result. -/
theorem transfer {α β : Type} {k : Result α} {abs : α → β} {op : β} (P : β → Prop)
    (h : k ⦃ r => abs r = op ⦄) (law : P op) : k ⦃ r => P (abs r) ⦄ := by
  apply WP.spec_mono h
  intro r hr; rw [hr]; exact law

/-! ## L4 admission: `guard::decide` implements `Calculus.admit` -/

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

theorem guard_decide_implements (a : guard.Action) (fs : Slice guard.Fact) :
    guard.decide true a fs ⦃ d =>
      decisionOf d = Calculus.admit (actionOf a) (fs.val.map factTri) ⦄ := by
  apply WP.spec_mono (Core.Guard.decide_spec true a fs)
  intro d hd
  rw [hd, Calculus.admit, all_metF]
  cases fs.val.all Core.Guard.metF <;> cases a <;>
    simp [decisionOf, actionOf, Core.Guard.actionOf, Calculus.Action.decision]

/-- Law 3.5, transferred, not re-proved. -/
theorem guard_unknown_never_passes (a : guard.Action) (fs : Slice guard.Fact)
    (h : guard.Fact.Unknown ∈ fs.val) :
    guard.decide true a fs ⦃ d => decisionOf d ≠ .pass ⦄ := by
  apply WP.spec_mono (guard_decide_implements a fs)
  intro d hd; rw [hd]
  exact Calculus.unknown_never_passes _ (List.mem_map.mpr ⟨_, h, rfl⟩)

theorem guard_combine_implements (a b : guard.Decision) :
    guard.combine a b ⦃ d => decisionOf d = Calculus.Decision.join (decisionOf a) (decisionOf b) ⦄ := by
  apply WP.spec_mono (Core.Guard.combine_spec a b)
  intro d hd; rw [hd]
  cases a <;> cases b <;> rfl

/-! ## Law U: four "complete" kernels are one `whole` -/

theorem walk_complete_implements (frontier skipped : Usize) (truncated : Bool) :
    walk.complete_within frontier skipped truncated ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts 0 0
        (frontier.val + skipped.val + if truncated then 1 else 0)) ⦄ := by
  apply WP.spec_mono (Core.Walk.complete_within_iff frontier skipped truncated)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  cases truncated <;> simp

theorem packet_complete_implements (kept required unreadable : Usize) :
    packet.complete kept required unreadable ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts kept.val 0
        ((required.val - kept.val) + unreadable.val)) ⦄ := by
  apply WP.spec_mono (Core.Packet.complete_iff kept required unreadable)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  omega

theorem runs_complete_implements (u n : Usize) :
    runs.complete u n ⦃ b => b = Calculus.whole (Calculus.Member.counts 0 0 (u.val + n.val)) ⦄ := by
  apply WP.spec_mono (Core.Runs.complete_spec u n)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.whole_counts]
  omega

/-! ## L2 policy: `policy::met` implements `Calculus.met`; the meet law is free -/

theorem policy_met_implements (x : gate.Standing) (a : policy.Accept) :
    policy.met x a ⦃ b => b = Calculus.met (standingOf x) (acceptOf a) ⦄ := by
  apply WP.spec_mono (Core.Policy.met_spec x a)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  cases x with
  | Checked s => cases s <;> simp [standingOf, acceptOf, Calculus.met]
  | Attested => simp [standingOf, acceptOf, Calculus.met]
  | Waived => simp [standingOf, acceptOf, Calculus.met]

/-- calculus.md 3.4 names `narrow_met_iff` as owed by the first kernel
that combines two policies. With conformance it is the core's `met_inf`. -/
theorem narrow_met_iff (x : gate.Standing) (a b : policy.Accept) (c : policy.Accept)
    (hc : acceptOf c = (acceptOf a).inf (acceptOf b)) :
    (do let m ← policy.met x c; let ma ← policy.met x a; let mb ← policy.met x b; ok (m, ma, mb))
      ⦃ r => r.1 = (r.2.1 && r.2.2) ⦄ := by
  step with policy_met_implements as ⟨ m, hm ⟩
  step with policy_met_implements as ⟨ ma, hma ⟩
  step with policy_met_implements as ⟨ mb, hmb ⟩
  simp [hm, hma, hmb, hc, Calculus.met_inf]

/-! ## Law R: the current-run pointer is a referent chosen without fall-back -/

/-- A pointer as a candidate: unset is no candidate; an open run offers
itself; a closed or missing run names the run and offers nothing. -/
def cand (level : Nat) : pointer.Pointer → List (Calculus.Candidate Nat)
  | .Unset => []
  | .Open => [⟨level, some level⟩]
  | _ => [⟨level, none⟩]

theorem pointer_open_implements (s t h : pointer.Pointer) :
    pointer.open s t h ⦃ b =>
      b = (Calculus.resolve (cand 3 s ++ cand 2 t ++ cand 1 h)).resolved ⦄ := by
  apply WP.spec_mono (Core.Pointer.open_spec s t h)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  cases s <;> cases t <;> cases h <;>
    simp [Core.Pointer.deciding, cand, Calculus.resolve, Calculus.Resolution.resolved, Calculus.Candidate.top]

end Writfold.Conform
