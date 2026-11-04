import Kctx.Engineering.LedgerTypes

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

/-- Reconstruct against the recorded prefix, not today's authority. The
validator checks origin, producer/approval bindings and closed-action premises. -/
def historyCheck {P H : Type} (validate : Validator P H) (stream : StreamId) :
    List (Record P H) → Bool
  | [] => true
  | r :: entries =>
      historyCheck validate stream entries &&
      decide (r.stream = stream) &&
      decide (r.sequence = entries.length + 1) &&
      decide (r.predecessor = predecessor entries) && validate entries r

def Linked {P H : Type} (validate : Validator P H) (stream : StreamId)
    (entries : List (Record P H)) : Prop := historyCheck validate stream entries = true

def Retained {P H : Type} (s : Store P H) (entries : List (Record P H)) : Prop :=
  ∀ r ∈ entries, r ∈ s.records ∧ ∀ o ∈ r.closure, o ∈ s.material

def SelectionValid {P H : Type} (validate : Validator P H) (s : Store P H)
    (selection : Selection P H) : Prop :=
  selection.entries.head?.map Record.head = some selection.head ∧
  selection.head.stream = selection.stream ∧
  Linked validate selection.stream selection.entries ∧ Retained s selection.entries

def WellFormed {P H : Type} (validate : Validator P H) (s : Store P H) : Prop :=
  UniqueRecords s ∧ UniqueHeads s ∧
  ∀ selection ∈ s.selections, SelectionValid validate s selection

theorem linked_nil {P H : Type} (validate : Validator P H) (stream : StreamId) :
    Linked validate stream [] := rfl

theorem linked_cons {P H : Type} (validate : Validator P H) (stream : StreamId)
    (r : Record P H) (entries : List (Record P H))
    (old : Linked validate stream entries) (same : r.stream = stream)
    (seq : r.sequence = entries.length + 1)
    (prev : r.predecessor = predecessor entries) (valid : validate entries r = true) :
    Linked validate stream (r :: entries) := by
  change historyCheck validate stream entries = true at old
  simp [Linked, historyCheck, old, same, seq, prev, valid]

theorem linked_cons_fields {P H : Type} (validate : Validator P H)
    (stream : StreamId) (r : Record P H) (entries : List (Record P H))
    (h : Linked validate stream (r :: entries)) :
    Linked validate stream entries ∧ r.stream = stream ∧
    r.sequence = entries.length + 1 ∧ r.predecessor = predecessor entries ∧
    validate entries r = true := by
  simpa [Linked, historyCheck, Bool.and_eq_true, and_assoc] using h

theorem empty_wellFormed {P H : Type} (validate : Validator P H) :
    WellFormed validate ({} : Store P H) := by
  simp [WellFormed, UniqueRecords, UniqueHeads]

theorem selected_mem {P H : Type} (s : Store P H) (stream : StreamId)
    (selection : Selection P H) (h : selected s stream = some selection) :
    selection ∈ s.selections := by
  exact List.mem_of_find?_eq_some h

theorem selected_stream {P H : Type} (s : Store P H) (stream : StreamId)
    (selection : Selection P H) (h : selected s stream = some selection) :
    selection.stream = stream := by
  have h' := List.find?_some h
  simpa using h'

theorem currentPrefix_linked {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (wf : WellFormed validate s) :
    Linked validate stream (currentPrefix s stream) := by
  cases h : selected s stream with
  | none => simp [currentPrefix, h, Linked, historyCheck]
  | some selection =>
      have hs := wf.2.2 selection (selected_mem s stream selection h)
      have he := selected_stream s stream selection h
      simpa [currentPrefix, h, he] using hs.2.2.1

theorem currentPrefix_retained {P H : Type} (validate : Validator P H)
    (s : Store P H) (stream : StreamId) (wf : WellFormed validate s) :
    Retained s (currentPrefix s stream) := by
  cases h : selected s stream with
  | none => simp [currentPrefix, h, Retained]
  | some selection =>
      have hs := wf.2.2 selection (selected_mem s stream selection h)
      simpa [currentPrefix, h] using hs.2.2.2

end Kctx.Engineering.Ledger
