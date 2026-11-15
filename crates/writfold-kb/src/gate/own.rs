//! A statement or cover row that makes itself stale: when the subject is
//! the KB's own repository and a gate's scope admits the run's files, the
//! file just written is inside the content it is bound to. `gate waive`,
//! `gate attest` and `gate cover` re-check the gate after their write and
//! say so when it no longer stands as written.
use super::status::Report;
use super::subject::Subject;
use crate::workspace::Workspace;
use writfold_core::gate::Standing;
use writfold_core::scope::in_scope;

/// The note a self-staling write adds.
pub const NOTE: &str = "this gate's scope includes the run's own files, so the statement changed the content it is about; exclude runs/** from the scope";

/// Whether the write of `paths` (relative to the KB root) left gate `id`
/// standing other than as `wanted` (`None`: any standing is suspect)
/// because its scope admits one of those paths in the KB's own
/// repository.
pub fn staled(
    ws: &Workspace,
    subject: &Subject,
    report: &Report<'_>,
    (id, wanted): (&str, Option<Standing>),
    paths: &[String],
) -> bool {
    let Some(repo) = &ws.repo else {
        return false;
    };
    let same = match (
        std::fs::canonicalize(&repo.work_tree),
        std::fs::canonicalize(&subject.top),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    let Some(judged) = report.gates.iter().find(|j| j.definition.id == id) else {
        return false;
    };
    if !same || wanted.is_some_and(|w| judged.standing == w) {
        return false;
    }
    let patterns = judged.definition.patterns();
    paths
        .iter()
        .any(|p| in_scope(&patterns, &format!("{}{p}", repo.prefix)))
}
