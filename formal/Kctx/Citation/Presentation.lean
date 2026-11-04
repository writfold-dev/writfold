import Kctx.Citation.Types

set_option autoImplicit false
namespace Kctx.Citation

/-- The budget is a count of notices, not a search limit. Source input is retained
in the cursor as well as its identity: raw cursors cannot substitute a different
result list or coverage under an unchanged external query name. -/
structure NoticeQuery where
  scope : AliasScope
  identity : ContentId
  orderingProfile : ContentId
  budget : Nat
  notices : List Notice
  coverage : Coverage
  deriving DecidableEq, Repr, Inhabited

structure NoticeCursor where
  query : NoticeQuery
  offset : Nat
  deriving DecidableEq, Repr, Inhabited

structure NoticePage where
  notices : List Notice
  offset : Nat
  omittedCount : Nat
  presentationComplete : Bool
  coverage : Coverage
  next : Option NoticeCursor
  deriving DecidableEq, Repr, Inhabited

inductive PresentationError where
  | cursorMismatch | zeroBudget | invalidOffset
  deriving DecidableEq, Repr, Inhabited

/-- Omitted counts refer to the whole query, including previous pages. Finishing
pagination does not retroactively make the final page a complete presentation. -/
def makeNoticePage (query : NoticeQuery) (offset : Nat) : NoticePage :=
  let shown := (query.notices.drop offset).take query.budget
  let omitted := query.notices.length - shown.length
  let following := offset + shown.length
  { notices := shown
    offset := offset
    omittedCount := omitted
    presentationComplete := decide (offset = 0 ∧ omitted = 0)
    coverage := query.coverage
    next := if following < query.notices.length then some ⟨query, following⟩ else none }

def boundedNoticePage (query : NoticeQuery) (offset : Nat) : Except PresentationError NoticePage :=
  if query.budget = 0 then .error .zeroBudget
  else if query.notices.length < offset then .error .invalidOffset
  else .ok (makeNoticePage query offset)

def presentNotices (query : NoticeQuery) (cursor : Option NoticeCursor) :
    Except PresentationError NoticePage :=
  match cursor with
  | none => boundedNoticePage query 0
  | some continuation =>
      if continuation.query = query then boundedNoticePage query continuation.offset
      else .error .cursorMismatch

end Kctx.Citation
