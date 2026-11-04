import Kctx.Engineering.LedgerHistory

set_option autoImplicit false
namespace Kctx.Engineering.Ledger

/-- These are input checks, not a desired post-state invariant. Material is
prepared first; selection is committed only after origin/payload validation. -/
def AppendReady {P H : Type} (validate : Validator P H) (s : Store P H)
    (stream : StreamId) (expected : Option Head) (r : Record P H)
    (available : List ObjectId) : Prop :=
  expected = currentHead s stream ∧
  (currentPrefix s stream).head?.map Record.head = currentHead s stream ∧
  Linked validate stream (currentPrefix s stream) ∧
  Retained s (currentPrefix s stream) ∧
  r.stream = stream ∧
  r.sequence = (currentPrefix s stream).length + 1 ∧
  r.predecessor = predecessor (currentPrefix s stream) ∧
  (∀ old ∈ s.records, old.id ≠ r.id) ∧
  (∀ o ∈ r.closure, o ∈ available ++ s.material) ∧
  validate (currentPrefix s stream) r = true

instance {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId) :
    Decidable (AppendReady validate s stream expected r available) := by
  unfold AppendReady Linked Retained
  infer_instance

/-- Retention is a separate executable phase; no head changes here. -/
def retain {P H : Type} (s : Store P H) (r : Record P H)
    (available : List ObjectId) : Store P H :=
  { s with records := r :: s.records, material := available ++ s.material }

/-- Called only after retain. Old immutable record values are never rewritten. -/
def select {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (entries : List (Record P H)) : Store P H :=
  { s with
    selections := ⟨stream, r.head, r :: entries⟩ ::
      s.selections.filter (fun old => old.stream != stream)
    inventoryEpoch := s.inventoryEpoch + if (selected s stream).isNone then 1 else 0 }

def commit {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) : Store P H :=
  select (retain s r available) stream r (currentPrefix s stream)

def append {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId := []) :
    Option (Store P H) :=
  if AppendReady validate s stream expected r available
  then some (commit s stream r available) else none

/-- Refusal is a frame, including material staged by a rejected caller. -/
def appendOrStay {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId := []) :
    Store P H := (append validate s stream expected r available).getD s

theorem append_success {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s t : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : append validate s stream expected r available = some t) :
    AppendReady validate s stream expected r available ∧
    t = commit s stream r available := by
  unfold append at h
  split at h
  · exact ⟨‹_›, (Option.some.inj h).symm⟩
  · contradiction

theorem append_refused {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : ¬ AppendReady validate s stream expected r available) :
    append validate s stream expected r available = none := by
  simp [append, h]

theorem refusal_frame {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : append validate s stream expected r available = none) :
    appendOrStay validate s stream expected r available = s := by
  simp [appendOrStay, h]

theorem commit_currentHead {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) :
    currentHead (commit s stream r available) stream = some r.head := by
  simp [currentHead, selected, commit, select, retain, List.find?]

theorem commit_retains_records {P H : Type} (s : Store P H) (stream : StreamId)
    (r old : Record P H) (available : List ObjectId) (h : old ∈ s.records) :
    old ∈ (commit s stream r available).records := List.mem_cons_of_mem _ h

theorem commit_retains_material {P H : Type} (s : Store P H) (stream : StreamId)
    (r : Record P H) (available : List ObjectId) (o : ObjectId) (h : o ∈ s.material) :
    o ∈ (commit s stream r available).material := List.mem_append_right _ h

theorem stale_head_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : expected ≠ currentHead s stream) :
    append validate s stream expected r available = none := by
  apply append_refused
  intro ready
  exact h ready.1

theorem malformed_head_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : (currentPrefix s stream).head?.map Record.head ≠ currentHead s stream) :
    append validate s stream expected r available = none := by
  apply append_refused
  intro ready
  exact h ready.2.1

theorem invalid_origin_refuses {P H : Type} [DecidableEq P] [DecidableEq H]
    (validate : Validator P H) (s : Store P H) (stream : StreamId)
    (expected : Option Head) (r : Record P H) (available : List ObjectId)
    (h : validate (currentPrefix s stream) r = false) :
    append validate s stream expected r available = none := by
  apply append_refused
  intro ready
  have valid := ready.2.2.2.2.2.2.2.2.2
  simp [h] at valid

end Kctx.Engineering.Ledger
