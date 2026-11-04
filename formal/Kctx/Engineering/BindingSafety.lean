import Kctx.Engineering.Actions

set_option autoImplicit false
namespace Kctx.Engineering

/-- A known changed provider cannot consume checks for the earlier native input. -/
theorem E11_changed_provider_refuses_prepare (contracts : Contracts) (s : State)
    (intent : Intent) (generation : Nat)
    (changed : intent.expectedProvider ≠ s.world.provider) :
    prepare contracts s intent generation = none := by
  cases hr : findRun s intent.invocation with
  | none => simp [prepare, hr]
  | some run =>
      cases hn : findNode run.procedure intent.node with
      | none => simp [prepare, hr, hn, bind, Option.bind]
      | some node => simp [prepare, hr, hn, bind, Option.bind, changed]

/-- Editing only the requested precondition is not an authorized rebind: the
exact binding is also covered by the grant, receipts and prepared inputs. -/
theorem E11_binding_cannot_silently_refresh (contracts : Contracts) (s : State)
    (intent : Intent) (generation : Nat) (run : Run)
    (found : findRun s intent.invocation = some run)
    (changed : intent.expectedProvider ≠ run.binding.expectedProvider) :
    prepare contracts s intent generation = none := by
  cases hn : findNode run.procedure intent.node with
  | none => simp [prepare, found, hn, bind, Option.bind]
  | some node => simp [prepare, found, hn, bind, Option.bind, intentReady, changed]

theorem E11_changed_provider_refuses_request (contracts : Contracts) (s : State)
    (intent : Intent) (generation : Nat) (stream : StreamId) (expected : Option Ledger.Head)
    (record : LedgerRecordId) (changed : intent.expectedProvider ≠ s.world.provider) :
    step contracts s (.append stream expected record (.prepare intent generation)) = none := by
  let captured := recordFor contracts s stream record (.prepare intent generation)
  let before := historicalState (Ledger.currentPrefix s.ledger stream) captured
  have refused : prepare contracts before intent generation = none :=
    E11_changed_provider_refuses_prepare contracts before intent generation changed
  have replayed : replay (Ledger.currentPrefix s.ledger stream) captured = none := by
    change ((prepare contracts before intent generation).map
      (frameDomain before (.prepare intent generation))).map State.toOperationalState = none
    rw [refused]
    rfl
  dsimp only [captured] at replayed
  simp only [step]
  split
  · rw [replayed]
    rfl
  · rfl

end Kctx.Engineering
