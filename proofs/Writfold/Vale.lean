import Extracted.WritfoldCore.Vale
import Extracted.WritfoldCore.Prose
import Extracted.WritfoldCore.Types
import Writfold.Prose

/-!
# Vale for documents (#117, #131)

Theorems over the Aeneas translation of `writfold_core::vale`: what one run
of the user's `vale` executable means for a `prose.vale` delivery check.

- `weigh_spec`, `weigh_never`, `weigh_error_blocks`, `weigh_mono`: an alert
  blocks exactly when its severity is at or above the mode's threshold.
- `usable_spec`, `usable_exit`, `usable_exit_agrees`: a run is read only
  when the pinned version ran, its output was whole, and it exited 0 with no
  error-level alert or 1 with some.
- `settle_unusable_unknown`, `settle_no_coverage_unknown`: anything else,
  or a run that applied no rule, is `Unknown`: never a pass, never a fail.
- `settle_pass_iff`, `settle_fail_iff`: exact conditions.
- `settle_advice_never_decides`, `settle_perm`, `settle_agrees`: advice and
  order are not inputs, and the receipt reads back consistent.
- `pass_stricter_pass`: a pass under a stricter threshold is a pass under
  every laxer one.
-/

open Aeneas Aeneas.Std Result
open writfold_core
open Writfold.Core.Prose

namespace Writfold.Core.Vale

/-! ## `weigh` -/

/-- A severity's rank: suggestion < warning < error. -/
def rankL (l : vale.Level) : Nat :=
  match l with
  | .Suggestion => 1
  | .Warning => 2
  | .Error => 3

/-- The lowest rank that blocks under a threshold (`Never` is above all). -/
def floorB (f : vale.BlockFrom) : Nat :=
  match f with
  | .Never => 4
  | .Error => 3
  | .Warning => 2
  | .Suggestion => 1

def weighOf (l : vale.Level) (f : vale.BlockFrom) : prose.Weight :=
  if floorB f ≤ rankL l then .Blocking else .Advice

theorem weigh_spec (l : vale.Level) (f : vale.BlockFrom) :
    vale.weigh l f ⦃ w => w = weighOf l f ⦄ := by
  unfold vale.weigh weighOf
  cases f <;> cases l <;> simp [floorB, rankL]

/-- With `Never`, every alert is advice. -/
theorem weigh_never (l : vale.Level) : weighOf l .Never = .Advice := by
  cases l <;> simp [weighOf, floorB, rankL]

/-- With any other threshold, an error blocks. -/
theorem weigh_error_blocks (f : vale.BlockFrom) (h : f ≠ .Never) :
    weighOf .Error f = .Blocking := by
  cases f <;> simp_all [weighOf, floorB, rankL]

/-- If a severity blocks, every higher severity blocks. -/
theorem weigh_mono (l l' : vale.Level) (f : vale.BlockFrom)
    (hb : weighOf l f = .Blocking) (hle : rankL l ≤ rankL l') :
    weighOf l' f = .Blocking := by
  unfold weighOf at *
  by_cases h : floorB f ≤ rankL l
  · have : floorB f ≤ rankL l' := by omega
    simp [this]
  · simp [h] at hb

/-! ## `usable` -/

def pinned (t : vale.Tool) : Bool :=
  match t with
  | .Pinned => true
  | _ => false

/-- The exit status agrees with the alerts: 0 with no error, 1 with some. -/
def exitAgrees (e : gate.Ending) (errors : Nat) : Bool :=
  match e with
  | .Exited i => (i.val = 0 && errors = 0) || (i.val = 1 && 0 < errors)
  | _ => false

def usableOf (r : vale.Ran) : Bool :=
  pinned r.tool && r.whole && exitAgrees r.ending r.errors.val

theorem usable_spec (r : vale.Ran) :
    vale.usable r ⦃ b => b = usableOf r ⦄ := by
  obtain ⟨tool, ending, whole, errors, enabled, canary⟩ := r
  unfold vale.usable usableOf
  cases tool <;> simp [pinned]
  cases whole <;> simp
  cases ending with
  | Exited i =>
    simp only [exitAgrees]
    split
    · simp only [WP.spec_ok]
      by_cases he : errors.val = 0
      · have : errors = 0#usize := by scalar_tac
        simp [this]; decide
      · have : errors ≠ 0#usize := by intro h; apply he; rw [h]; rfl
        simp [this, he]; decide
    · simp only [WP.spec_ok]
      by_cases he : errors.val = 0
      · simp [he]; decide
      · have hp : 0 < errors.val := by omega
        simp [hp]; exact Or.inr (by decide)
    · rename_i h0 h1
      have h0' : i.val ≠ 0 := fun h => h0 (IScalar.eq_of_val_eq (by rw [h]; rfl))
      have h1' : i.val ≠ 1 := fun h => h1 (IScalar.eq_of_val_eq (by rw [h]; rfl))
      simp [h0', h1']
  | _ => simp [exitAgrees]

/-- A usable run exited 0 with no error-level alert, or 1 with some. -/
theorem usable_exit_agrees (r : vale.Ran) (h : usableOf r = true) :
    ∃ i : I32, r.ending = .Exited i ∧
      ((i.val = 0 ∧ r.errors.val = 0) ∨ (i.val = 1 ∧ 0 < r.errors.val)) := by
  unfold usableOf at h
  simp only [Bool.and_eq_true] at h
  obtain ⟨_, hx⟩ := h
  revert hx
  unfold exitAgrees
  cases r.ending with
  | Exited i => intro hx; exact ⟨i, rfl, by simpa using hx⟩
  | _ => simp

/-- A usable run exited 0 or 1. -/
theorem usable_exit (r : vale.Ran) (h : usableOf r = true) :
    ∃ i : I32, r.ending = .Exited i ∧ (i.val = 0 ∨ i.val = 1) := by
  obtain ⟨i, he, hx⟩ := usable_exit_agrees r h
  exact ⟨i, he, by omega⟩

/-! ## `rules` and `settle` -/

def rulesOf (enabled canary : Nat) : Nat := if canary = 0 then 0 else enabled

theorem rules_spec (enabled canary : Usize) :
    vale.rules enabled canary ⦃ n => n.val = rulesOf enabled.val canary.val ⦄ := by
  unfold vale.rules rulesOf
  split
  · rename_i h; simp only [WP.spec_ok]; have : canary.val = 0 := by scalar_tac
    simp [this]
  · rename_i h; simp only [WP.spec_ok]; have : canary.val ≠ 0 := by scalar_tac
    simp [this]

def settleOf (r : vale.Ran) (l : List prose.Weight) : gate.Outcome :=
  if usableOf r then verdictOf (rulesOf r.enabled.val r.canary.val) (countB l)
  else .Unknown

theorem settle_spec (r : vale.Ran) (found : Slice prose.Weight) :
    vale.settle r found ⦃ o => o = settleOf r found.val ⦄ := by
  unfold vale.settle settleOf
  step with usable_spec as ⟨ b, hb ⟩
  split
  · rename_i hu
    have : usableOf r = true := by rw [← hb]; exact hu
    simp only [this, ↓reduceIte]
    step with rules_spec as ⟨ n, hn ⟩
    apply WP.spec_mono (verdict_spec n found)
    intro o ho; rw [ho, hn]
  · rename_i hu
    have : usableOf r = false := by rw [← hb]; simpa using hu
    simp [this]

/-- A run that cannot be read is `Unknown`: a missing executable, another
version, a signal, a timeout, a start failure, exit 2 or higher, an exit
status that disagrees with the alerts, and cut-off or unreadable output. -/
theorem settle_unusable_unknown (r : vale.Ran) (found : Slice prose.Weight)
    (h : usableOf r = false) :
    vale.settle r found ⦃ o => o = .Unknown ⦄ := by
  apply WP.spec_mono (settle_spec r found)
  intro o ho; rw [ho]; simp [settleOf, h]

/-- No enabled check, or a canary that drew no alert, is `Unknown`. -/
theorem settle_no_coverage_unknown (r : vale.Ran) (found : Slice prose.Weight)
    (h : r.enabled.val = 0 ∨ r.canary.val = 0) :
    vale.settle r found ⦃ o => o = .Unknown ⦄ := by
  apply WP.spec_mono (settle_spec r found)
  intro o ho; rw [ho]; unfold settleOf rulesOf verdictOf
  split <;> simp
  rcases h with h | h <;> simp [h]

theorem settle_pass_iff (r : vale.Ran) (found : Slice prose.Weight) :
    vale.settle r found ⦃ o => o = .Pass ↔
      usableOf r = true ∧ r.enabled.val ≠ 0 ∧ r.canary.val ≠ 0 ∧
        countB found.val = 0 ⦄ := by
  apply WP.spec_mono (settle_spec r found)
  intro o ho; rw [ho]; unfold settleOf rulesOf verdictOf
  cases usableOf r <;> simp
  by_cases hc : r.canary.val = 0 <;> by_cases he : r.enabled.val = 0 <;>
    by_cases hb : 0 < countB found.val <;> simp [hc, he, hb] <;> omega

theorem settle_fail_iff (r : vale.Ran) (found : Slice prose.Weight) :
    vale.settle r found ⦃ o => o = .Fail ↔
      usableOf r = true ∧ r.enabled.val ≠ 0 ∧ r.canary.val ≠ 0 ∧
        0 < countB found.val ⦄ := by
  apply WP.spec_mono (settle_spec r found)
  intro o ho; rw [ho]; unfold settleOf rulesOf verdictOf
  cases usableOf r <;> simp
  by_cases hc : r.canary.val = 0 <;> by_cases he : r.enabled.val = 0 <;>
    by_cases hb : 0 < countB found.val <;> simp [hc, he, hb]

/-- Removing every piece of advice leaves the outcome unchanged. -/
theorem settle_advice_never_decides (r : vale.Ran) (l : List prose.Weight) :
    settleOf r (l.filter isB) = settleOf r l := by
  unfold settleOf
  rw [advice_never_decides]

/-- Reordering the alerts leaves the outcome unchanged. -/
theorem settle_perm (r : vale.Ran) (l l' : List prose.Weight) (h : l.Perm l') :
    settleOf r l = settleOf r l' := by
  unfold settleOf
  rw [verdict_perm _ l l' h]

/-- A receipt written from `settle` reads back consistent. -/
theorem settle_agrees (r : vale.Ran) (l : List prose.Weight) :
    agreesOf (settleOf r l) (countB l) = true := by
  unfold settleOf
  split
  · exact agrees_verdict _ _
  · simp [agreesOf]

/-! ## Thresholds -/

theorem countB_weigh_zero (ls : List vale.Level) (f : vale.BlockFrom) :
    countB (ls.map (weighOf · f)) = 0 ↔ ∀ l ∈ ls, rankL l < floorB f := by
  unfold countB
  rw [List.countP_eq_zero]
  simp only [List.mem_map, forall_exists_index, and_imp, forall_apply_eq_imp_iff₂]
  apply forall_congr'; intro l; apply forall_congr'; intro _
  unfold isB weighOf
  by_cases h : floorB f ≤ rankL l <;> simp [h]
  omega

/-- A document that passes under a stricter threshold passes under every
laxer one. -/
theorem pass_stricter_pass (r : vale.Ran) (ls : List vale.Level)
    (strict lax : vale.BlockFrom) (hle : floorB strict ≤ floorB lax)
    (h : settleOf r (ls.map (weighOf · strict)) = .Pass) :
    settleOf r (ls.map (weighOf · lax)) = .Pass := by
  unfold settleOf verdictOf at *
  split at h
  · rename_i hu
    simp only [hu, ↓reduceIte]
    split at h
    · simp at h
    · rename_i hr
      simp only [hr, ↓reduceIte]
      split at h
      · simp at h
      · rename_i hb
        have h0 : countB (ls.map (weighOf · strict)) = 0 := by omega
        rw [countB_weigh_zero] at h0
        have : countB (ls.map (weighOf · lax)) = 0 := by
          rw [countB_weigh_zero]; intro l hl; have := h0 l hl; omega
        simp [this]
  · simp at h

end Writfold.Core.Vale
