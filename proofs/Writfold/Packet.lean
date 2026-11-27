import Extracted.WritfoldCore.Packet
import Writfold.Account
import Extracted.WritfoldCore.Types

/-!
# The handover packet (#109)

Theorems over the Aeneas translation of `writfold_core::packet`: which of a
run's rows `resume --packet` keeps within a budget, and when the packet may
say it is complete. The caller orders the rows required first; the packet is
the longest prefix within the budget, so it keeps the input order and never
holds an optional row while a required one is dropped (both by
construction: a prefix).

- `cut_spec`: the cut `k` is at most the number of rows, the first `k` rows
  cost at most the budget (`select_within_budget`), and when a row is left
  out the next one did not fit (`cut_maximal`).
- `all_fit_all_selected`: when every row fits, every row is kept.
- `required_omitted_spec` (`omitted_counted`): the reported number of
  required rows left out is `required - min kept required`.
- `complete_iff`: a packet is complete exactly when no required row was left
  out and nothing it was built from was unreadable.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Packet

/-- The cost of the first `k` rows. -/
def prefixSum (xs : List Usize) (k : Nat) : Nat := ((xs.map (·.val)).take k).sum

theorem prefixSum_zero (xs : List Usize) : prefixSum xs 0 = 0 := by
  simp [prefixSum]

theorem prefixSum_succ (xs : List Usize) (n : Nat) (h : n < xs.length) :
    prefixSum xs (n + 1) = prefixSum xs n + xs[n].val := by
  unfold prefixSum
  rw [List.take_add_one, List.sum_append]
  have h' : n < (xs.map (·.val)).length := by simpa using h
  rw [List.getElem?_eq_getElem h']
  simp

theorem prefixSum_le_total (xs : List Usize) (k : Nat) :
    prefixSum xs k ≤ prefixSum xs xs.length := by
  unfold prefixSum
  have hl : (xs.map (·.val)).length = xs.length := by simp
  rw [← hl, List.take_length]
  have := congrArg List.sum (List.take_append_drop k (xs.map (·.val)))
  rw [List.sum_append] at this
  omega

/-- What the cut promises. -/
def cutOk (xs : List Usize) (budget k : Nat) : Prop :=
  k ≤ xs.length ∧ prefixSum xs k ≤ budget ∧
    (k < xs.length → budget < prefixSum xs (k + 1))

theorem cut_loop_spec (costs : Slice Usize) (budget used index : Usize)
    (hi : index.val ≤ costs.val.length)
    (hu : used.val = prefixSum costs.val index.val)
    (hb : used.val ≤ budget.val) :
    packet.cut_loop costs budget used index ⦃ k => cutOk costs.val budget.val k.val ⦄ := by
  unfold packet.cut_loop
  apply loop.spec_decr_nat (fun p : Usize × Usize => costs.val.length - p.2.val)
    (fun p : Usize × Usize => p.2.val ≤ costs.val.length ∧
      p.1.val = prefixSum costs.val p.2.val ∧ p.1.val ≤ budget.val)
  · rintro ⟨used, index⟩ ⟨hi, hu, hb⟩
    unfold packet.cut_loop.body
    dsimp only at hi hu hb ⊢
    split
    · rename_i hlt
      have hil : index.val < costs.val.length := by scalar_tac
      step as ⟨ cost, hcost ⟩
      step as ⟨ room, hroom ⟩
      have hs := prefixSum_succ costs.val index.val hil
      split
      · rename_i hgt
        simp only [WP.spec_ok]
        refine ⟨by omega, by omega, ?_⟩
        intro _
        rw [hs, ← hcost]
        scalar_tac
      · rename_i hle
        step as ⟨ used1, hused1 ⟩
        step as ⟨ index1, hindex1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac, by scalar_tac⟩
        rw [hindex1, hs, ← hcost, hused1, hu]
    · rename_i hge
      simp only [WP.spec_ok]
      have hlen : index.val = costs.val.length := by scalar_tac
      refine ⟨by scalar_tac, ?_, ?_⟩
      · have : (Slice.len costs).val = index.val := by scalar_tac
        rw [this, ← hu]; exact hb
      · intro h; scalar_tac
  · exact ⟨hi, hu, hb⟩

theorem cut_spec (costs : Slice Usize) (budget : Usize) :
    packet.cut costs budget ⦃ k => cutOk costs.val budget.val k.val ⦄ := by
  unfold packet.cut
  apply cut_loop_spec
  · simp
  · simp [prefixSum_zero]
  · simp

/-- The first `k` rows cost at most the budget. -/
theorem select_within_budget (costs : Slice Usize) (budget : Usize) :
    packet.cut costs budget ⦃ k => prefixSum costs.val k.val ≤ budget.val ⦄ :=
  WP.spec_mono (cut_spec costs budget) (fun _ h => h.2.1)

/-- A row is left out only when the next one did not fit. -/
theorem cut_maximal (costs : Slice Usize) (budget : Usize) :
    packet.cut costs budget ⦃ k =>
      k.val < costs.val.length → budget.val < prefixSum costs.val (k.val + 1) ⦄ :=
  WP.spec_mono (cut_spec costs budget) (fun _ h => h.2.2)

/-- When every row fits, every row is kept. -/
theorem all_fit_all_selected (costs : Slice Usize) (budget : Usize)
    (h : prefixSum costs.val costs.val.length ≤ budget.val) :
    packet.cut costs budget ⦃ k => k.val = costs.val.length ⦄ := by
  apply WP.spec_mono (cut_spec costs budget)
  rintro k ⟨hle, _, hmax⟩
  by_contra hne
  have hlt : k.val < costs.val.length := by omega
  have h1 := hmax hlt
  have h2 := prefixSum_le_total costs.val (k.val + 1)
  omega

/-- The reported number of required rows left out. -/
theorem required_omitted_spec (kept required : Usize) :
    packet.required_omitted kept required ⦃ n =>
      n.val = required.val - min kept.val required.val ⦄ := by
  unfold packet.required_omitted
  split
  · simp only [WP.spec_ok]
    simp; scalar_tac
  · step as ⟨ n, hn ⟩
    rw [hn]
    have : min kept.val required.val = kept.val := by scalar_tac
    rw [this]

theorem omitted_counted (kept required : Usize) :
    packet.required_omitted kept required ⦃ n =>
      n.val = 0 ↔ required.val ≤ kept.val ⦄ := by
  apply WP.spec_mono (required_omitted_spec kept required)
  intro n hn
  rw [hn]
  omega

/-- A packet is complete exactly when no required row was left out,
nothing it was built from was unreadable and no file of the run was left
unread (#181). -/
theorem complete_iff (kept required unreadable unread : Usize) :
    packet.complete kept required unreadable unread ⦃ b =>
      b = true ↔ required.val ≤ kept.val ∧ unreadable.val = 0 ∧ unread.val = 0 ⦄ := by
  unfold packet.complete
  split
  · split
    · simp only [WP.spec_ok, decide_eq_true_eq]
      constructor
      · intro h; exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
      · intro h; scalar_tac
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
      intro _ h; scalar_tac
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro h; scalar_tac

/-- A file left unread never lets a packet say it is complete. -/
theorem unread_never_complete (kept required unreadable unread : Usize) (h : 0 < unread.val) :
    packet.complete kept required unreadable unread ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (complete_iff kept required unreadable unread)
  intro b hb
  cases b
  · rfl
  · have := (hb.mp rfl).2.2; omega

/-- With nothing left unread the decision is #109's: a run Writfold wrote
in full reads as before. -/
theorem no_unread_unchanged (kept required unreadable : Usize) :
    packet.complete kept required unreadable 0#usize ⦃ b =>
      b = true ↔ required.val ≤ kept.val ∧ unreadable.val = 0 ⦄ := by
  apply WP.spec_mono (complete_iff kept required unreadable 0#usize)
  intro b hb
  rw [hb]; simp

/-- Complete with `unread` the run's remainder implies the run's files are
whole. -/
theorem complete_implies_whole (kept required unreadable : Usize) (ms : Slice account.Member)
    (unread : Usize) (hu : unread.val = Writfold.Core.Account.leftCount ms.val) :
    packet.complete kept required unreadable unread ⦃ b =>
      b = true → account.Member.Remainder ∉ ms.val ⦄ := by
  apply WP.spec_mono (complete_iff kept required unreadable unread)
  intro b hb hbt hm
  have h0 := (hb.mp hbt).2.2
  rw [hu, Writfold.Core.Account.leftCount, List.countP_eq_zero] at h0
  exact absurd (h0 _ hm) (by simp [Writfold.Core.Account.leftB])

end Writfold.Core.Packet
