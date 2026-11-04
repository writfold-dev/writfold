import Kctx.Engineering.Core

set_option autoImplicit false
namespace Kctx.Engineering

/-- Ledger storage and rendered cuts do not participate in invocation authority. -/
theorem ancestryLive_operational (s t : State) (same : s.toOperationalState = t.toOperationalState)
    (fuel : Nat) (run : Run) : ancestryLive s fuel run = ancestryLive t fuel run := by
  induction fuel generalizing run with
  | zero => rfl
  | succ fuel ih =>
      have localSame : locallyLive s run = locallyLive t run := by
        cases s; cases t; cases same; rfl
      have findSame : ∀ id, findRun s id = findRun t id := by
        intro id
        cases s; cases t; cases same; rfl
      simp only [ancestryLive, localSame, findSame, ih]

theorem live_operational (s t : State) (same : s.toOperationalState = t.toOperationalState)
    (run : Run) : live s run = live t run := by
  have counts : s.runs.length = t.runs.length := congrArg (fun op => op.runs.length) same
  simp only [live, counts]
  exact ancestryLive_operational s t same _ run

theorem receiptValid_operational (s t : State)
    (same : s.toOperationalState = t.toOperationalState) (run : Run) (receipt : Receipt) :
    receiptValid s run receipt = receiptValid t run receipt := by
  unfold receiptValid
  rw [live_operational s t same run]
  cases s; cases t; cases same; rfl

end Kctx.Engineering
