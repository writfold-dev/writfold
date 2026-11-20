//! Obligation state, derived on every read from the definitions, the run's
//! receipts and its statements (never stored): `open`, `satisfied`,
//! `stale`, `failed`, `unknown`, and the two statement standings
//! `attested` and `waived`. The digest comparisons and the decisions are
//! the kernel's ([`writfold_core::gate::judge`],
//! [`writfold_core::gate::statements`], [`writfold_core::gate::standing`]);
//! this module gathers the facts and renders rows for `gate status`,
//! `gate lines` and `resume`.
use super::def::{self, Definition, Origin};
use super::receipt::{self, Receipt};
use super::statement;
use super::subject::{Snapshot, Subject, snapshot};
use crate::emit::{More, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use std::path::Path;
use writfold_core::gate::{Evidence, Recorded, Standing, State, standing};
use writfold_core::scope::in_scope;
use writfold_runtime::git::GitExecutable;

pub use super::receipt::{Filed, filed};

/// What a run's plan does to its gates: which gates it owes (all defined
/// gates when the run has no plan) and the run-record digest its
/// run-record gates are judged against ([`crate::plan::records`]).
#[derive(Clone, Debug, Default)]
pub struct Owed {
    /// `Some` when the run has a plan: only these gates are owed.
    pub gates: Option<Vec<String>>,
    /// The run-record digest of the current step's window (the whole run
    /// without a plan); `None` when the records could not be read.
    pub records: Option<String>,
    /// The deliveries the plan owes, `(deliver:<mode>, step path)`: judged
    /// at their steps, never listed or counted as gates.
    pub deliveries: Vec<(String, String)>,
}

/// A run's receipts, statements and assessed receipts, and what its plan
/// owes.
pub struct Filings {
    pub receipts: Vec<Filed>,
    pub statements: Vec<statement::Filed>,
    pub assessed: Vec<super::assessed::Filed>,
    pub covers: Vec<super::checklist::Filed>,
    /// `runs/<run>`.
    pub run_dir: String,
    pub owed: Owed,
}

impl Filings {
    /// The filings alone: every defined gate owed, no run-record digest.
    pub fn read(root: &Path, run_dir: &str, run: &str) -> Self {
        Self {
            receipts: filed(root, run_dir, run),
            statements: statement::filed(root, run_dir, run),
            assessed: super::assessed::filed(root, run_dir, run),
            covers: super::checklist::filed(root, run_dir, run),
            run_dir: run_dir.to_owned(),
            owed: Owed::default(),
        }
    }

    /// The filings of a run with what its plan owes.
    pub fn for_run(session: &crate::session::Session<'_>, rec: &crate::index::query::Rec) -> Self {
        let mut filings = Self::read(&session.ws.root, &crate::runs::dir(rec), &rec.id);
        filings.owed = crate::plan::owed(session, rec);
        filings
    }

    /// The valid receipts about `repo`, in judging order ([`receipt::order`]).
    pub fn valid(&self, repo: &str) -> Vec<(&Filed, &Receipt)> {
        let mut valid = self.parsed();
        valid.retain(|(_, r)| r.repo == repo);
        valid
    }

    /// Every receipt of the run that reads (its digest intact, filed in
    /// this run), about any subject, in judging order. The gate kernel
    /// decides which are bound ([`writfold_core::gate::bound`]).
    pub fn parsed(&self) -> Vec<(&Filed, &Receipt)> {
        let mut all: Vec<(&Filed, &Receipt)> = self
            .receipts
            .iter()
            .filter_map(|f| f.parsed.as_ref().ok().map(|r| (f, r)))
            .collect();
        all.sort_by(|a, b| receipt::order((&a.0.name, a.1), (&b.0.name, b.1)));
        all
    }

    /// Receipts and statements that do not read as valid.
    pub fn invalid(&self) -> usize {
        self.receipts.iter().filter(|f| f.parsed.is_err()).count()
            + self.statements.iter().filter(|f| f.parsed.is_err()).count()
            + self.assessed.iter().filter(|f| f.parsed.is_err()).count()
            + self.covers.iter().filter(|f| f.parsed.is_err()).count()
    }
}

/// One gate's derived state.
pub struct Judged<'a> {
    pub definition: Definition,
    /// The state from receipts alone.
    pub state: State,
    /// What stands once statements are counted.
    pub standing: Standing,
    /// The receipt that decides the state, if any.
    pub receipt: Option<(&'a Filed, &'a Receipt)>,
    pub same_definition: bool,
    pub statements: statement::Summary<'a>,
    /// An assessed gate's receipt and checks (its `receipt` is `None`).
    pub assessment: Option<super::assessed::Assessment<'a>>,
    /// A checklist gate's items (its `receipt` is `None`).
    pub checklist: Option<super::checklist::View<'a>>,
}

/// The whole picture for one run and subject.
pub struct Report<'a> {
    pub gates: Vec<Judged<'a>>,
    pub problems: Vec<super::discover::Unusable>,
    pub snapshot: Snapshot,
}

impl Report<'_> {
    /// Every fault of the definitions that do not parse.
    pub fn diagnostics(&self) -> writfold_core::diag::Report {
        let mut out = writfold_core::diag::Report::default();
        for (_, problem) in &self.problems {
            out.extend(problem.report.clone());
        }
        out
    }

    /// The note naming how many cited references stand on a verdict read
    /// from a check receipt, if any.
    pub fn recorded_note(&self) -> Option<String> {
        let views = self.gates.iter().filter_map(|j| j.checklist.as_ref());
        let n: usize = views.flat_map(|v| &v.entries).map(|e| e.recorded).sum();
        super::ground::recorded_note(n)
    }

    /// Whether every gate is met under its policy.
    pub fn all_met(&self) -> bool {
        !self.gates.is_empty() && self.gates.iter().all(Judged::met)
    }
}

/// Judge every defined gate on `subject` from the run's filings. Assessed
/// and checklist gates are judged last, against the others
/// ([`super::assessed::judge`], [`super::checklist::judge`]).
pub fn judge<'a>(
    session: &Session<'_>,
    subject: &Subject,
    filings: &'a Filings,
) -> Result<Report<'a>> {
    let (mut report, later) = judge_plain(session, subject, filings)?;
    let (assessed, lists) = later.into_iter().partition(|d| d.assessed.is_some());
    let more = super::assessed::judge(session, subject, filings, &report, assessed)?;
    let lists = super::checklist::judge(session, subject, filings, &report, lists)?;
    report.gates.extend(more);
    report.gates.extend(lists);
    // Every gate is judged, so evidence grounds the same with or without
    // a plan; only the gates the plan owes are reported.
    if let Some(owed) = &filings.owed.gates {
        report.gates.retain(|j| owed.contains(&j.definition.id));
    }
    Ok(report)
}

/// Judge the command, attested and run-record gates; return the assessed
/// and checklist definitions, which are judged against this report.
pub fn judge_plain<'a>(
    session: &Session<'_>,
    subject: &Subject,
    filings: &'a Filings,
) -> Result<(Report<'a>, Vec<Definition>)> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let (mut definitions, problems) = def::all(&session.ws.root, Some(&subject.top));
    // Candidate gates judge delivery drafts, never a repository: a run
    // owes them only through a delivery mode (`deliver status`).
    definitions.retain(|d| !d.on_candidate());
    let later: Vec<Definition>;
    (later, definitions) =
        (definitions.into_iter()).partition(|d| d.assessed.is_some() || d.is_checklist());
    let patterns: Vec<Vec<&str>> = (definitions.iter().chain(&later))
        .map(Definition::patterns)
        .collect();
    let wanted = |p: &str| patterns.iter().any(|scope| in_scope(scope, p));
    let mut snapshot = snapshot(git, subject, &wanted)?;
    let scopes =
        (definitions.iter().chain(&later)).flat_map(|d| d.scope.iter().map(String::as_str));
    snapshot.attach(git, session.ws, scopes);
    let valid = filings.parsed();
    let records = filings.owed.records.clone().unwrap_or_default();
    let clock = super::pins::Clock::read(session);
    let mut gates = Vec::new();
    for definition in definitions {
        let content = match definition.run_record() {
            true => records.clone(),
            false => snapshot.scoped(&definition.patterns()).digest,
        };
        gates.push(judge_gate(
            definition,
            &valid,
            filings,
            &subject.name,
            &content,
            Some(&clock),
        ));
    }
    let report = Report {
        gates,
        problems,
        snapshot,
    };
    Ok((report, later))
}

/// Judge one gate from the run's receipts (in judging order) against the
/// present `content`: the scope digest of a repository gate, the
/// run-record digest of a run-record gate. Each receipt goes to the kernel
/// with whether it is bound: filed in this run (it parsed), about `repo`,
/// and intact (it parsed); one that is not counts as absent.
pub fn judge_gate<'a>(
    definition: Definition,
    valid: &[(&'a Filed, &'a Receipt)],
    filings: &'a Filings,
    repo: &str,
    content: &str,
    clock: Option<&super::pins::Clock>,
) -> Judged<'a> {
    let mine: Vec<(&Filed, &Receipt)> = valid
        .iter()
        .filter(|(_, r)| r.gate == definition.id)
        .copied()
        .collect();
    let recorded: Vec<Recorded> = mine
        .iter()
        .map(|(_, r)| Recorded {
            outcome: r.outcome,
            definition: r.definition.as_bytes().to_vec(),
            content: r.scope.as_bytes().to_vec(),
            bound: writfold_core::gate::bound(true, r.repo == repo, true),
        })
        .collect();
    let stamps: Vec<&str> = mine.iter().map(|(_, r)| r.recorded.as_str()).collect();
    let evidence: Vec<Evidence> =
        super::pins::evidence(&recorded, &stamps, &definition, content, clock);
    let (state, at) = writfold_core::gate::state(&evidence);
    let receipt = at.and_then(|i| mine.get(i).copied());
    let statements = statement::summary(
        &filings.statements,
        &definition.id,
        repo,
        &definition.digest,
        content,
    );
    Judged {
        same_definition: receipt.is_none_or(|(_, r)| r.definition == definition.digest),
        standing: standing(definition.attested(), state, statements.statements),
        definition,
        state,
        receipt,
        statements,
        assessment: None,
        checklist: None,
    }
}

fn short(text: &str) -> &str {
    let text = text.strip_prefix("sha256:").unwrap_or(text);
    text.get(..12).unwrap_or(text)
}

fn line(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// Where a definition is shown: its KB path, or `<repo>:<path>`.
pub fn shown_path(d: &Definition, subject: &Subject) -> String {
    match d.origin {
        Origin::Kb => d.path.clone(),
        Origin::Repo => format!("{}:{}", subject.name, d.path),
    }
}

/// The `read:` line of a gate that names instructions: the file the
/// agent reads (on an attested gate, what is being attested to).
/// A repository definition's path is shown as `<repo>:<path>`.
pub fn read_line(d: &Definition, repo: &str) -> Option<More> {
    let path = d.instructions_path()?;
    Some(More {
        label: "read",
        text: format!("instructions for {}", d.id),
        path: Some(match d.origin {
            Origin::Kb => path,
            Origin::Repo => format!("{repo}:{path}"),
        }),
        line: Some(1),
        ..More::default()
    })
}

fn invalid_row(kind: &'static str, name: &str, path: &str, why: &str) -> Row {
    Row {
        kind,
        id: name.trim_end_matches(".json").to_owned(),
        status: Some("invalid".into()),
        text: why.to_owned(),
        path: Some(path.to_owned()),
        flags: vec!["invalid".into()],
        section: Some("gates"),
        ..Row::default()
    }
}

/// The rows: one per gate (with why and the next command when it is not
/// met), one per invalid receipt or statement, one per unusable
/// definition.
pub fn rows(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    filings: &Filings,
) -> Result<Vec<Row>> {
    let mut out = Vec::new();
    for judged in &report.gates {
        out.push(match (&judged.assessment, &judged.checklist) {
            (Some(assessment), _) => super::assessed::row(subject, judged, assessment),
            (_, Some(view)) => super::checklist::row(subject, judged, view),
            _ => gate_row(git, subject, report, judged)?,
        });
    }
    for f in &filings.receipts {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("receipt", &f.name, &f.path, why));
        }
    }
    for f in &filings.statements {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("statement", &f.name, &f.path, why));
        }
    }
    for f in &filings.assessed {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("receipt", &f.name, &f.path, why));
        }
    }
    for f in &filings.covers {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("cover", &f.name, &f.path, why));
        }
    }
    for (_, problem) in &report.problems {
        let row =
            crate::defs::invalid_row("gate-definition", &problem.path, problem, Some("gates"));
        out.push(row);
    }
    Ok(out)
}

fn gate_row(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    judged: &Judged<'_>,
) -> Result<Row> {
    let d = &judged.definition;
    let mut row = Row {
        kind: "gate",
        id: d.id.clone(),
        status: Some(judged.standing.name().into()),
        section: Some("gates"),
        ..Row::default()
    };
    super::head::head(&mut row, judged, subject);
    if let Some(more) = read_line(d, &subject.name) {
        row.more.push(more);
    }
    super::policy::json_keys(&mut row, judged);
    let shown = match judged.standing {
        Standing::Attested => judged.statements.attested,
        Standing::Waived => judged.statements.waived,
        Standing::Checked(_) => None,
    };
    if let Some((f, s)) = shown {
        row.more.push(statement::more(f, s));
    }
    if judged.standing == Standing::Checked(State::Satisfied) {
        return Ok(row);
    }
    row.flags.push(judged.standing.name().into());
    if judged.met() {
        return Ok(row);
    }
    super::policy::flag(&mut row, judged, &shown_path(d, subject));
    if judged.state == State::Stale
        && let Some((_, r)) = judged.receipt
    {
        let why = stale_why(git, subject, report, judged, r)?;
        row.more.push(line("why", why));
    }
    if judged.statements.waiver_moved {
        row.more.push(line(
            "why",
            "a waiver exists, but for other content or another definition".into(),
        ));
    }
    let next = match d.attested() {
        true => format!("writfold kb gate attest {} --statement TEXT --by WHO", d.id),
        false => format!("writfold kb gate run {}", d.id),
    };
    row.more.push(line("next", next));
    Ok(row)
}

/// Why a stale receipt is stale: the definition changed, or which
/// in-scope paths differ.
fn stale_why(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    judged: &Judged<'_>,
    r: &Receipt,
) -> Result<String> {
    if !judged.same_definition {
        return Ok("the definition changed since this receipt".into());
    }
    if judged.definition.run_record() {
        return Ok(
            "the run's records changed since this receipt, or it is about another step".into(),
        );
    }
    let patterns = judged.definition.patterns();
    let present = report.snapshot.scoped(&patterns).digest;
    if let Some(why) = super::pins::why(&judged.definition, &present, r) {
        return Ok(why);
    }
    Ok(
        match report
            .snapshot
            .changed_since(git, subject, &r.commit, &patterns)?
        {
            None => format!(
                "in-scope paths changed; commit {} is not in this clone",
                short(&r.commit)
            ),
            Some(paths) if paths.is_empty() => "in-scope content differs".into(),
            Some(paths) => {
                let first: Vec<&str> = paths.iter().take(5).map(String::as_str).collect();
                let more = paths.len().saturating_sub(5);
                let tail = if more > 0 {
                    format!(" (+{more} more)")
                } else {
                    String::new()
                };
                format!("changed in scope: {}{tail}", first.join(", "))
            }
        },
    )
}
