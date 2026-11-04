import Kctx.Citation.Presentation

set_option autoImplicit false
namespace Kctx.Citation

theorem boundedNoticePage_exact (query : NoticeQuery) (offset : Nat) (page : NoticePage)
    (success : boundedNoticePage query offset = .ok page) :
    page = makeNoticePage query offset := by
  unfold boundedNoticePage at success
  split at success
  · contradiction
  · split at success
    · contradiction
    · cases success
      rfl

theorem presentNotices_exact (query : NoticeQuery) (cursor : Option NoticeCursor)
    (page : NoticePage) (success : presentNotices query cursor = .ok page) :
    ∃ offset, page = makeNoticePage query offset := by
  cases cursor with
  | none => exact ⟨0, boundedNoticePage_exact query 0 page success⟩
  | some continuation =>
      simp only [presentNotices] at success
      split at success
      · exact ⟨continuation.offset, boundedNoticePage_exact _ _ _ success⟩
      · contradiction

theorem presentNotices_displayed_from_input (query : NoticeQuery)
    (cursor : Option NoticeCursor) (page : NoticePage) (notice : Notice)
    (success : presentNotices query cursor = .ok page) (displayed : notice ∈ page.notices) :
    notice ∈ query.notices := by
  obtain ⟨offset, rfl⟩ := presentNotices_exact query cursor page success
  exact List.mem_of_mem_drop (List.mem_of_mem_take displayed)

theorem presentNotices_bounded (query : NoticeQuery) (cursor : Option NoticeCursor)
    (page : NoticePage) (success : presentNotices query cursor = .ok page) :
    page.notices.length ≤ query.budget := by
  obtain ⟨offset, rfl⟩ := presentNotices_exact query cursor page success
  exact List.length_take_le query.budget (query.notices.drop offset)

theorem presentNotices_omitted_not_complete (query : NoticeQuery)
    (cursor : Option NoticeCursor) (page : NoticePage)
    (success : presentNotices query cursor = .ok page) (omitted : page.omittedCount ≠ 0) :
    page.presentationComplete = false := by
  obtain ⟨offset, rfl⟩ := presentNotices_exact query cursor page success
  change query.notices.length - ((query.notices.drop offset).take query.budget).length ≠ 0 at omitted
  simp only [List.length_take, List.length_drop] at omitted
  simp [makeNoticePage, omitted]

theorem presentNotices_coverage_unchanged (query : NoticeQuery) (cursor : Option NoticeCursor)
    (page : NoticePage) (success : presentNotices query cursor = .ok page) :
    page.coverage = query.coverage := by
  obtain ⟨offset, rfl⟩ := presentNotices_exact query cursor page success
  rfl

theorem presentNotices_incomplete_search_stays_incomplete (query : NoticeQuery)
    (cursor : Option NoticeCursor) (page : NoticePage)
    (success : presentNotices query cursor = .ok page)
    (incomplete : query.coverage.complete = false) : page.coverage.complete = false := by
  rw [presentNotices_coverage_unchanged query cursor page success]
  exact incomplete

/-- This equality pins the source list and coverage too, not just presentation
labels. Each required component below follows from actual acceptance. -/
theorem presentNotices_cursor_pinned (query : NoticeQuery) (cursor : NoticeCursor)
    (page : NoticePage) (success : presentNotices query (some cursor) = .ok page) :
    cursor.query = query := by
  simp only [presentNotices] at success
  split at success
  · assumption
  · contradiction

theorem presentNotices_cursor_contract (query : NoticeQuery) (cursor : NoticeCursor)
    (page : NoticePage) (success : presentNotices query (some cursor) = .ok page) :
    cursor.query.scope = query.scope ∧ cursor.query.identity = query.identity ∧
    cursor.query.orderingProfile = query.orderingProfile ∧ cursor.query.budget = query.budget := by
  have pinned := presentNotices_cursor_pinned query cursor page success
  simp [pinned]

theorem presentNotices_mismatched_cursor_refused (query : NoticeQuery) (cursor : NoticeCursor)
    (mismatch : cursor.query ≠ query) :
    presentNotices query (some cursor) = .error .cursorMismatch := by
  simp [presentNotices, mismatch]

theorem boundedNoticePage_zero_budget_refused (query : NoticeQuery) (offset : Nat)
    (zero : query.budget = 0) : boundedNoticePage query offset = .error .zeroBudget := by
  simp [boundedNoticePage, zero]

theorem boundedNoticePage_invalid_offset_refused (query : NoticeQuery) (offset : Nat)
    (budget : query.budget ≠ 0) (invalid : query.notices.length < offset) :
    boundedNoticePage query offset = .error .invalidOffset := by
  simp [boundedNoticePage, budget, invalid]

end Kctx.Citation
