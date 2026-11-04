import Kctx.Engineering.OperationalInvariant
import Kctx.Engineering.Actions
import Kctx.Engineering.ControlProofs

set_option autoImplicit false
namespace Kctx.Engineering

theorem eraseDups_length_le {α : Type} [BEq α] (xs : List α) :
    xs.eraseDups.length ≤ xs.length := by
  cases xs with
  | nil => simp
  | cons a xs =>
      rw [List.eraseDups_cons]
      simp only [List.length_cons]
      exact Nat.succ_le_succ (Nat.le_trans (eraseDups_length_le _) (List.length_filter_le _ _))
termination_by xs.length
decreasing_by
  simp_wf
  exact Nat.lt_succ_of_le (List.length_filter_le _ _)

theorem unique_sound {α : Type} [BEq α] [LawfulBEq α] {xs : List α}
    (h : unique xs = true) : xs.Nodup := by
  revert h
  induction xs with
  | nil => intro _; exact List.nodup_nil
  | cons a xs ih =>
      intro h
      have equal : (a :: xs).eraseDups.length = (a :: xs).length := by
        simpa [unique] using h
      rw [List.eraseDups_cons] at equal
      have erased := eraseDups_length_le (xs.filter (fun b => !b == a))
      have filtered := List.length_filter_le (fun b => !b == a) xs
      have length : (xs.filter (fun b => !b == a)).length = xs.length := by
        simp only [List.length_cons] at equal
        omega
      have keep := List.length_filter_eq_length_iff.mp length
      have unchanged : xs.filter (fun b => !b == a) = xs := List.filter_eq_self.mpr keep
      refine List.nodup_cons.mpr ⟨?_, ih ?_⟩
      · intro member
        have impossible := keep a member
        simp at impossible
      · rw [unchanged] at equal
        have rest := Nat.succ.inj equal
        simpa [unique] using rest

end Kctx.Engineering
