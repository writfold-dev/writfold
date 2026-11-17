//! The head of a `gate status` row: the deciding receipt, or why there is
//! none, with the row's text and position.
use super::receipt;
use super::status::{Judged, shown_path};
use super::subject::Subject;
use crate::emit::Row;

fn short(digest: &str) -> &str {
    let d = digest.strip_prefix("sha256:").unwrap_or(digest);
    d.get(..12).unwrap_or(d)
}

/// A gate row's text and position: its deciding receipt, or why there is
/// none.
pub(super) fn head(row: &mut Row, judged: &Judged<'_>, subject: &Subject) {
    let d = &judged.definition;
    match judged.receipt {
        None if d.attested() => {
            row.text = format!(
                "attested: statements only, no command, for {}",
                subject.name
            );
            row.path = Some(shown_path(d, subject));
            row.line = Some(1);
        }
        None => {
            row.text = format!("{} no receipt in this run for {}", d.class, subject.name);
            row.path = Some(shown_path(d, subject));
            row.line = Some(1);
        }
        Some((f, r)) => {
            let dirty = if r.dirty > 0 {
                format!("+dirty:{}", r.dirty)
            } else {
                String::new()
            };
            row.text = format!(
                "{} {} {} at {}@{}{dirty} scope={} paths={}",
                r.class,
                r.outcome.name(),
                receipt::ended(r),
                r.repo,
                short(&r.commit),
                short(&r.scope),
                r.paths
            );
            row.path = Some(f.path.clone());
            row.line = Some(1);
        }
    }
}
