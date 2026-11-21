//! `kb policy --paths GATE` (policy spec 2.5, build step 14): the paths a
//! gate's `scope` covers at the subject's present state, one row each, so
//! a scope can be inspected rather than inferred from `paths=N`. The
//! declared scope only; Writfold does not infer what else the check
//! depends on. Read-only: the subject's status and tree, and each
//! `repo:` store's tree, read as `gate status` reads them.
use crate::emit::{Answer, Row};
use crate::error::{KbError, Result};
use crate::gate::def;
use crate::gate::subject::{locate, snapshot};
use crate::session::Session;
use std::path::Path;
use writfold_core::scope::in_scope;

/// The answer, the verb name and its arguments, as the dispatcher wants.
pub fn answer(
    session: &Session<'_>,
    cwd: &Path,
    gate: &str,
) -> Result<(Answer, &'static str, Vec<String>)> {
    Ok((paths(session, cwd, gate)?, "policy", vec![gate.to_owned()]))
}

pub fn paths(session: &Session<'_>, cwd: &Path, gate: &str) -> Result<Answer> {
    let git = (session.git.as_ref()).ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let subject = locate(session.ws, git, None, cwd, None)?.ok_or_else(|| {
        KbError::Refused(
            "not in a Git work tree: run it in the repository whose gates you mean".into(),
        )
    })?;
    let found = def::inventory(&session.ws.root, Some(&subject.top), def::list);
    let name = format!("{}/{gate}.toml", def::DIR);
    if let Some((_, problem)) = found.problems.iter().find(|(_, p)| p.path == name) {
        return Err(KbError::Definition(Box::new(problem.report.clone())));
    }
    let unread = match found.complete() {
        true => "",
        false => {
            "; a gates directory could not be listed, so it may be defined there (writfold kb lint)"
        }
    };
    let d = (found.definitions.into_iter().find(|d| d.id == gate)).ok_or_else(|| {
        KbError::Refused(format!(
            "no gate {gate} is defined (writfold kb policy lists them){unread}"
        ))
    })?;
    if d.run_record() || d.on_candidate() {
        return Err(KbError::Refused(format!(
            "gate {gate} has no path scope: it judges {}",
            match d.run_record() {
                true => "the run's records",
                false => "a delivery's candidate",
            }
        )));
    }
    let patterns = d.patterns();
    let wanted = |p: &str| in_scope(&patterns, p);
    let mut snap = snapshot(git, &subject, &wanted)?;
    snap.attach(git, session.ws, d.scope.iter().map(String::as_str));
    let (listed, unread) = snap.listed(&patterns);
    let mut answer = Answer {
        rows: (listed.iter())
            .map(|p| Row {
                kind: "path",
                id: p.clone(),
                section: Some("policy"),
                ..Row::default()
            })
            .collect(),
        ..Answer::default()
    };
    answer.notes.push(format!(
        "gate {gate} scope={} in {}: the declared scope; Writfold does not infer what else the check depends on",
        match d.scope.is_empty() {
            true => "(none)".to_owned(),
            false => d.scope.join(","),
        },
        subject.name
    ));
    for store in &unread {
        answer.notes.push(format!(
            "store {store} could not be read here: its paths are not listed (writfold kb repos)"
        ));
    }
    answer.partial = !unread.is_empty();
    answer.counts = vec![("paths", listed.len())];
    Ok(answer)
}
