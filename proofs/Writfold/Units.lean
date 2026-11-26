import Extracted.WritfoldCore.Units
import Extracted.WritfoldCore.Types

/-!
# Generated review units (#156, RFC 2026-11-25-kb-research-contracts)

Theorems over the Aeneas translation of `writfold_core::units`.

- `pairs_spec`: up to `MAX_MEMBERS` (64), `pairs n` is exactly the list
  `Pairs n`, the pairs `(a, b)` with `a < b < n`, row by row; over it,
  `None` (`over_cap_refused`).
- `pairs_mem`: a pair is listed exactly when `a < b < n`.
- `pairs_sorted`: the list is strictly increasing in lexicographic order,
  so each pair is listed once and in order.
- `pairs_len`: there are `n (n - 1) / 2` of them (stated as `2 * length =
  n * (n - 1)`).
- `generated_covered_iff`: a generated checklist is covered exactly when
  its universe is present, it has a unit or allows none, and every unit
  is covered; `absent_universe_never_covered`,
  `empty_universe_needs_allow`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Units

/-- Row `a` of `n` members: `(a, b)` for `a < b < n`. -/
def row (n a : Nat) : List (Nat × Nat) :=
  (List.range' (a + 1) (n - (a + 1))).map (fun b => (a, b))

/-- Rows `a`, …, `a + k - 1`. -/
def rows (n a k : Nat) : List (Nat × Nat) :=
  (List.range' a k).flatMap (row n)

/-- Every unordered pair of `n` members, row by row. -/
def Pairs (n : Nat) : List (Nat × Nat) := rows n 0 n

def nat (v : List (Usize × Usize)) : List (Nat × Nat) :=
  v.map (fun p => (p.1.val, p.2.val))

theorem rows_succ (n a k : Nat) : rows n a (k + 1) = rows n a k ++ row n (a + k) := by
  simp [rows, List.range'_concat, List.flatMap_append]

theorem pairs_inner_spec (n a : Usize) (out : alloc.vec.Vec (Usize × Usize)) (b : Usize)
    (ha : a.val < n.val) (hn : n.val ≤ 64) (hb : a.val + 1 ≤ b.val) (hbn : b.val ≤ n.val)
    (hlen : out.val.length ≤ 64 * a.val + b.val)
    (hout : nat out.val = rows n.val 0 a.val ++
      (List.range' (a.val + 1) (b.val - (a.val + 1))).map (fun y => (a.val, y))) :
    units.pairs_loop0_loop0 n out a b ⦃ r =>
      nat r.val = rows n.val 0 (a.val + 1) ∧ r.val.length ≤ 64 * (a.val + 1) ⦄ := by
  unfold units.pairs_loop0_loop0
  apply loop.spec_decr_nat (fun (p : alloc.vec.Vec (Usize × Usize) × Usize) => n.val - p.2.val)
    (fun (p : alloc.vec.Vec (Usize × Usize) × Usize) =>
      a.val + 1 ≤ p.2.val ∧ p.2.val ≤ n.val ∧ p.1.val.length ≤ 64 * a.val + p.2.val ∧
      nat p.1.val = rows n.val 0 a.val ++
        (List.range' (a.val + 1) (p.2.val - (a.val + 1))).map (fun y => (a.val, y)))
  · intro ⟨out, b⟩ ⟨hb, hbn, hlen, hout⟩
    dsimp only at hb hbn hlen hout
    unfold units.pairs_loop0_loop0.body
    dsimp only
    split
    · rename_i hlt
      have hmax : out.val.length < Usize.max := by scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hmax)
      intro out1 hout1
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro b' hb'
      simp only [WP.spec_ok]
      have hb1 : b'.val = b.val + 1 := by scalar_tac
      have hbn' : b.val < n.val := by scalar_tac
      refine ⟨⟨by omega, by omega, by simp [hout1]; omega, ?_⟩, by omega⟩
      rw [hout1, hb1]
      have hk : b.val + 1 - (a.val + 1) = (b.val - (a.val + 1)) + 1 := by omega
      simp only [nat, List.map_append] at hout ⊢
      rw [hout, hk, List.range'_concat]
      simp
      omega
    · rename_i hge
      simp only [WP.spec_ok]
      have hbe : b.val = n.val := by scalar_tac
      refine ⟨?_, by omega⟩
      rw [hout, rows_succ, hbe]
      simp [row]
  · exact ⟨hb, hbn, hlen, hout⟩

theorem pairs_outer_spec (n : Usize) (out : alloc.vec.Vec (Usize × Usize)) (a : Usize)
    (hn : n.val ≤ 64) (ha : a.val ≤ n.val) (hlen : out.val.length ≤ 64 * a.val)
    (hout : nat out.val = rows n.val 0 a.val) :
    units.pairs_loop0 n out a ⦃ r => nat r.val = Pairs n.val ⦄ := by
  unfold units.pairs_loop0
  apply loop.spec_decr_nat (fun (p : alloc.vec.Vec (Usize × Usize) × Usize) => n.val - p.2.val)
    (fun (p : alloc.vec.Vec (Usize × Usize) × Usize) =>
      p.2.val ≤ n.val ∧ p.1.val.length ≤ 64 * p.2.val ∧ nat p.1.val = rows n.val 0 p.2.val)
  · intro ⟨out, a⟩ ⟨ha, hlen, hout⟩
    dsimp only at ha hlen hout
    unfold units.pairs_loop0.body
    dsimp only
    split
    · rename_i hlt
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro b hb
      apply WP.spec_bind (pairs_inner_spec n a out b hlt hn (by scalar_tac) (by scalar_tac)
        (by scalar_tac) (by rw [hout]; have : b.val - (a.val + 1) = 0 := by scalar_tac
                            simp [this]))
      intro out1 ⟨hout1, hlen1⟩
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro a' ha'
      simp only [WP.spec_ok]
      have ha1 : a'.val = a.val + 1 := by scalar_tac
      have han : a.val < n.val := by scalar_tac
      refine ⟨⟨by omega, by rw [ha1]; exact hlen1, by rw [ha1]; exact hout1⟩, by omega⟩
    · rename_i hge
      simp only [WP.spec_ok, Pairs]
      have : a.val = n.val := by scalar_tac
      rw [hout, this]
  · exact ⟨ha, hlen, hout⟩

/-- Up to 64 members, `pairs` is exactly `Pairs n`; over it, `None`. -/
theorem pairs_spec (n : Usize) :
    units.pairs n ⦃ r => match r with
      | none => 64 < n.val
      | some v => n.val ≤ 64 ∧ nat v.val = Pairs n.val ⦄ := by
  unfold units.pairs units.MAX_MEMBERS
  split
  · simp only [WP.spec_ok]; scalar_tac
  · rename_i hle
    have hn : n.val ≤ 64 := by scalar_tac
    apply WP.spec_bind (pairs_outer_spec n _ 0#usize hn (by simp) (by simp) (by simp [nat, rows]))
    intro v hv
    simp only [WP.spec_ok]
    exact ⟨hn, hv⟩

/-- `None` exactly over 64 members. -/
theorem over_cap_refused (n : Usize) :
    units.pairs n ⦃ r => r = none ↔ 64 < n.val ⦄ := by
  apply WP.spec_mono (pairs_spec n)
  intro r hr
  cases r with
  | none => simpa using hr
  | some v => simp only [reduceCtorEq, false_iff, not_lt]; exact hr.1

theorem pairs_mem (n x y : Nat) : (x, y) ∈ Pairs n ↔ x < y ∧ y < n := by
  simp only [Pairs, rows, row, List.mem_flatMap, List.mem_range'_1, List.mem_map, Prod.mk.injEq]
  constructor
  · rintro ⟨a, ⟨_, ha⟩, b, ⟨hb1, hb2⟩, rfl, rfl⟩; omega
  · intro ⟨hxy, hy⟩
    exact ⟨x, ⟨by omega, by omega⟩, y, ⟨by omega, by omega⟩, rfl, rfl⟩

/-- Lexicographic order on pairs. -/
def Lex (p q : Nat × Nat) : Prop := p.1 < q.1 ∨ (p.1 = q.1 ∧ p.2 < q.2)

/-- Strictly increasing: each pair once, in order. -/
theorem pairs_sorted (n : Nat) : (Pairs n).Pairwise Lex := by
  simp only [Pairs, rows, List.pairwise_flatMap]
  constructor
  · intro a _
    simp only [row, List.pairwise_map]
    apply List.Pairwise.imp _ (List.pairwise_lt_range' (s := a + 1) (n := n - (a + 1)))
    intro b c h; exact Or.inr ⟨rfl, h⟩
  · apply List.Pairwise.imp _ (List.pairwise_lt_range' (s := 0) (n := n))
    intro a c h x hx y hy
    simp only [row, List.mem_map] at hx hy
    obtain ⟨_, _, rfl⟩ := hx
    obtain ⟨_, _, rfl⟩ := hy
    exact Or.inl h

theorem rows_from_len (n m : Nat) : ∀ a, n = a + m →
    2 * (rows n a m).length = m * (m - 1) := by
  induction m with
  | zero => intro a _; simp [rows]
  | succ k ih =>
    intro a h
    have hsplit : rows n a (k + 1) = row n a ++ rows n (a + 1) k := by
      simp [rows, List.range'_succ]
    rw [hsplit, List.length_append, Nat.mul_add, ih (a + 1) (by omega)]
    have : (row n a).length = k := by simp [row]; omega
    rw [this]
    cases k with
    | zero => simp
    | succ j =>
      simp only [Nat.add_sub_cancel, Nat.add_mul, Nat.mul_add, Nat.mul_one, Nat.one_mul]
      omega

/-- `n (n - 1) / 2` pairs. -/
theorem pairs_len (n : Nat) : 2 * (Pairs n).length = n * (n - 1) :=
  rows_from_len n n 0 (by omega)

/-- A generated checklist is covered exactly when its universe is present,
it has a unit or allows none, and every unit is covered. -/
theorem generated_covered_iff (present : Bool) (count : Usize) (allow : Bool)
    (r : checklist.Rollup) :
    units.generated present count allow r ⦃ o =>
      (o.covered = true ↔ present = true ∧ (0 < count.val ∨ allow = true) ∧ r.covered = true) ∧
      o.stale = r.stale ∧ o.first_owed = r.first_owed ⦄ := by
  unfold units.generated
  have hc : (count > 0#usize) ↔ 0 < count.val := by
    constructor <;> intro h <;> scalar_tac
  cases present <;> cases allow <;> by_cases h : count > 0#usize <;>
    simp [h, hc.mp, WP.spec_ok] <;> simp_all

theorem absent_universe_never_covered (count : Usize) (allow : Bool) (r : checklist.Rollup) :
    units.generated false count allow r ⦃ o => o.covered = false ⦄ := by
  apply WP.spec_mono (generated_covered_iff false count allow r)
  intro o ⟨h, _⟩
  cases ho : o.covered <;> simp_all

theorem empty_universe_needs_allow (present : Bool) (r : checklist.Rollup) :
    units.generated present 0#usize false r ⦃ o => o.covered = false ⦄ := by
  apply WP.spec_mono (generated_covered_iff present 0#usize false r)
  intro o ⟨h, _⟩
  cases ho : o.covered <;> simp_all

end Writfold.Core.Units
