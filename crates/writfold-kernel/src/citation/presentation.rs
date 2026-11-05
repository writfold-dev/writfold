use vstd::prelude::*;
use writfold_domain::citation::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationError { CursorMismatch, ZeroBudget, InvalidOffset }
#[derive(Debug)]
pub struct NoticePage<'a> {
    notices: &'a [Notice<'a>], offset: usize, omitted_count: usize,
    presentation_complete: bool, coverage: Coverage, next: Option<NoticeCursor<'a>>,
}
impl<'a> NoticePage<'a> {
    pub closed spec fn notices_value(&self) -> Seq<(NoticeKind, Seq<char>)> {
        self.notices@.map_values(|notice: Notice| notice.material())
    }
    pub closed spec fn offset_value(&self) -> nat { self.offset as nat }
    pub closed spec fn omitted_value(&self) -> nat { self.omitted_count as nat }
    pub closed spec fn complete_value(&self) -> bool { self.presentation_complete }
    pub closed spec fn coverage_value(&self) -> Coverage { self.coverage }
    pub closed spec fn next_value(&self) -> Option<NoticeCursor<'a>> { self.next }
    pub fn notices(&self) -> (value: &'a [Notice<'a>])
        ensures value@.map_values(|notice: Notice| notice.material()) == self.notices_value(),
    { self.notices }
    pub fn offset(&self) -> (value: usize)
        ensures value == self.offset_value(),
    { self.offset }
    pub fn omitted_count(&self) -> (value: usize)
        ensures value == self.omitted_value(),
    { self.omitted_count }
    pub fn presentation_complete(&self) -> (value: bool)
        ensures value == self.complete_value(),
    { self.presentation_complete }
    pub fn coverage(&self) -> (value: Coverage)
        ensures value == self.coverage_value(),
    { self.coverage }
    pub fn next(&self) -> (value: Option<NoticeCursor<'a>>)
        ensures value == self.next_value(),
    { self.next }
}

fn bounded_page<'a>(query: &'a NoticeQuery<'a>, offset: usize)
    -> (result: Result<NoticePage<'a>, PresentationError>)
    ensures match result {
        Ok(page) => page.offset_value() == offset && page.notices_value().len() <= query@.budget
            && page.omitted_value() + page.notices_value().len() == query@.notices.len()
            && page.complete_value() == (offset == 0 && page.omitted_value() == 0)
            && page.coverage_value() == query@.coverage
            && page.notices_value() == query@.notices.subrange(offset as int,
                offset + page.notices_value().len())
            && match page.next_value() {
                Some(cursor) => cursor.query_value() == query@
                    && cursor.offset_value() == offset + page.notices_value().len(),
                None => offset + page.notices_value().len() == query@.notices.len(),
            },
        Err(_) => true,
    },
{
    if query.budget() == 0 { return Err(PresentationError::ZeroBudget); }
    let all = query.notices();
    if offset > all.len() { return Err(PresentationError::InvalidOffset); }
    let remaining = all.len() - offset;
    let shown = if query.budget() < remaining { query.budget() } else { remaining };
    let following = offset + shown;
    let omitted_count = all.len() - shown;
    let next = if following < all.len() { Some(NoticeCursor::new(query, following)) } else { None };
    Ok(NoticePage { notices: &all[offset..following], offset, omitted_count,
        presentation_complete: offset == 0 && omitted_count == 0, coverage: query.coverage(), next })
}

/// Omission counts cover the entire query, including previous pages. Finishing a
/// pagination sequence does not make its last partial page a complete presentation.
pub fn present_notices<'a>(query: &'a NoticeQuery<'a>, cursor: Option<NoticeCursor<'_>>)
    -> (result: Result<NoticePage<'a>, PresentationError>)
    ensures result.is_ok() ==> match cursor {
            None => result.unwrap().offset_value() == 0,
            Some(continuation) => continuation.query_value().scope == query@.scope
                && continuation.query_value().identity == query@.identity
                && continuation.query_value().ordering_profile == query@.ordering_profile
                && continuation.query_value().budget == query@.budget
                && continuation.query_value().notices == query@.notices
                && continuation.query_value().coverage == query@.coverage
                && result.unwrap().offset_value() == continuation.offset_value(),
        },
        result.is_ok() ==> result.unwrap().coverage_value() == query@.coverage
            && result.unwrap().omitted_value() + result.unwrap().notices_value().len() == query@.notices.len()
            && result.unwrap().notices_value().len() <= query@.budget
            && result.unwrap().complete_value() == (result.unwrap().offset_value() == 0
                && result.unwrap().omitted_value() == 0),
{
    match cursor {
        None => bounded_page(query, 0),
        Some(continuation) => {
            if !continuation.query().same(query) { return Err(PresentationError::CursorMismatch); }
            bounded_page(query, continuation.offset())
        },
    }
}
}
