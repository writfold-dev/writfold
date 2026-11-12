//! `gate attest <id>` and `gate waive <id>`: record a statement about a
//! gate on the current subject ([`super::statement`]). An attestation is
//! accepted only by an `attested` gate, which runs no command; a waiver,
//! which needs a reason, may set aside any gate. Neither makes a gate
//! `satisfied`: they stand as `attested` and `waived`, and stop standing
//! when the gate's definition or the content in its scope moves.
use super::statement::{self, Kind, Statement};
use super::status::Filings;
use super::{def, status, subject};
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic, unique};
use crate::{runs, time};

/// One statement to record about a gate.
#[derive(Clone, Debug)]
pub struct Say {
    pub kind: Kind,
    pub id: String,
    pub text: String,
    pub by: String,
    pub run: Option<String>,
    pub repo: Option<std::path::PathBuf>,
}

pub(super) fn record(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    say: &Say,
    now: i64,
) -> Result<Output> {
    let flag = match say.kind {
        Kind::Attested => "--statement",
        Kind::Waived => "--reason",
    };
    let text = statement::check_text(flag, &say.text)?;
    let by = statement::check_text("--by", &say.by)?;
    let session = Session::open(ws, Some(tool), false, now)?;
    let rec = super::target(&session, say.run.as_deref())?;
    if runs::state(&session, &rec) == "closed" {
        return Err(KbError::Refused(format!("run {} is closed", rec.id)));
    }
    let subject =
        super::locate(&session, &rec, say.repo.as_deref(), &request.cwd)?.ok_or_else(|| {
            KbError::Refused("no subject repository: run inside one, or pass --repo DIR".into())
        })?;
    let definition = def::find_repository(&ws.root, Some(&subject.top), &say.id)?;
    if definition.assessed.is_some() {
        return Err(KbError::Refused(format!(
            "gate {} is assessed: only gate run, with every required Weft check entailed from grounded assumptions, meets it (statements about an assessed gate are not built)",
            say.id
        )));
    }
    // A run-record gate's statement binds the run-record digest it is judged by.
    let records = crate::plan::owed(&session, &rec).records;
    drop(session);
    if say.kind == Kind::Attested && !definition.attested() {
        return Err(KbError::Refused(format!(
            "gate {} is a {} check: only a passing gate run satisfies it (gate waive sets it aside, with a reason)",
            say.id, definition.class
        )));
    }
    let git = tool.admit()?;
    let patterns = definition.patterns();
    let wanted = |p: &str| writfold_core::gate::in_scope(&patterns, p);
    let snapshot = subject::snapshot(&git, &subject, &wanted)?;
    let mut scoped = snapshot.scoped(&patterns);
    if definition.run_record() {
        scoped.digest = records.unwrap_or_default();
    }
    let bytes = statement::bytes(&Statement {
        kind: say.kind,
        obligation: say.id.clone(),
        run: rec.id.clone(),
        subject: subject.name.clone(),
        commit: Some(snapshot.commit.as_str().to_owned()),
        content: scoped.digest.clone(),
        definition: definition.digest.clone(),
        text,
        by,
        recorded: time::stamp(now),
    });
    let path = format!(
        "{}/{}/{}-{}.json",
        runs::dir(&rec),
        statement::DIR,
        say.id,
        unique(now, &say.id)
    );
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let ops = [Op::Create {
        path: path.clone(),
        bytes,
    }];
    atomic::apply(&ws.root, &state, &ops, request.probe)?;
    answer(ws, tool, request, (&rec, &subject), say, &path, now)
}

fn answer(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    (rec, subject): (&crate::index::query::Rec, &subject::Subject),
    say: &Say,
    path: &str,
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let mut filings = Filings::for_run(&session, rec);
    filings.owed.gates = None;
    let report = status::judge(&session, subject, &filings)?;
    let mut answer = Answer {
        rows: status::rows(git, subject, &report, &filings)?
            .into_iter()
            .filter(|r| r.kind == "gate" && r.id == say.id)
            .collect(),
        ..Answer::default()
    };
    answer.notes.push(format!(
        "recorded {path} ({}: a statement, not a check)",
        say.kind.name()
    ));
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let verb = match say.kind {
        Kind::Attested => "gate-attest",
        Kind::Waived => "gate-waive",
    };
    let frame = super::frame(request, verb, vec![say.id.clone(), rec.id.clone()]);
    session.emit(&answer, &frame)
}
