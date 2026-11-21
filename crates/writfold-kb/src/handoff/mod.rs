//! `kb handoff check` (#57): did every gate the operator requires pass,
//! mechanically, on this candidate? The required set and the approved
//! definitions are read at `--policy` ([`policy`]); the evidence is the
//! run's receipts, judged against each approved definition and the
//! candidate commit's scoped content; the decision is
//! [`writfold_core::handoff::strict`]. Read-only: nothing is run.
pub mod policy;
mod rows;

use crate::emit::Answer;
use crate::error::{KbError, Result};
use crate::gate::status::{Filings, judge_gate};
use crate::index::query::Rec;
use crate::session::Session;
use std::path::Path;
use writfold_core::handoff::{Item, Verdict, strict};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, kb_commit};

/// What one required gate came to, for the rows.
pub struct Judged {
    pub id: String,
    pub item: Item,
    /// The approved definition's path (or why there is none).
    pub definition: std::result::Result<String, String>,
    /// The receipt the standing rests on, and the commit it was taken at.
    pub receipt: Option<(String, String)>,
}

fn commit(
    git: &GitExecutable<'_>,
    subject: &crate::gate::subject::Subject,
    spec: &str,
    flag: &str,
) -> Result<GitObjectId> {
    let found = kb_commit(
        git,
        subject.reads(),
        spec,
        crate::workspace::budgets()?,
        &crate::workspace::CANCELLED,
    )?;
    found.ok_or_else(|| {
        KbError::Usage(format!(
            "{flag} {spec}: no such commit in {}",
            subject.top.display()
        ))
    })
}

/// The handoff answer for `rec`'s receipts, the candidate and the policy.
pub fn check(
    session: &Session<'_>,
    rec: &Rec,
    (policy_rev, candidate_rev): (&str, &str),
    repo: Option<&Path>,
    cwd: &Path,
) -> Result<Answer> {
    let git = (session.git.as_ref()).ok_or_else(|| KbError::Refused("handoff needs Git".into()))?;
    let subject = crate::gate::locate(session, rec, repo, cwd)?.ok_or_else(|| {
        KbError::Refused("no subject repository: run inside one, or pass --repo DIR".into())
    })?;
    let policy_commit = commit(git, &subject, policy_rev, "--policy")?;
    let candidate = commit(git, &subject, candidate_rev, "--candidate")?;
    let required = policy::read(git, &subject, &policy_commit)?;
    let mut snapshot = crate::gate::subject::at_commit(git, &subject, candidate.clone())?;
    let scopes: Vec<String> = (required.definitions.iter().flatten())
        .flat_map(|d| d.scope.clone())
        .collect();
    snapshot.attach(git, session.ws, scopes.iter().map(String::as_str));
    let filings = Filings::for_run(session, rec);
    let valid = filings.parsed();
    let mut judged = Vec::new();
    for (id, definition) in required.ids.iter().zip(required.definitions) {
        judged.push(match definition {
            Err(why) => Judged {
                id: id.clone(),
                item: Item {
                    defined: false,
                    approved: false,
                    standing: writfold_core::gate::Standing::Checked(
                        writfold_core::gate::State::Open,
                    ),
                },
                definition: Err(why),
                receipt: None,
            },
            Ok(d) => {
                let content = snapshot.scoped(&d.patterns()).digest;
                let mine = filings.valid(&subject.name);
                let approved =
                    (mine.iter()).any(|(_, r)| r.gate == d.id && r.definition == d.digest);
                let path = format!("{}/{}.toml", crate::gate::def::DIR, d.id);
                let one = judge_gate(d, &valid, &filings, &subject.name, &content, None);
                Judged {
                    id: id.clone(),
                    item: Item {
                        defined: true,
                        approved,
                        standing: one.standing,
                    },
                    definition: Ok(path),
                    receipt: one.receipt.map(|(f, r)| (f.path.clone(), r.commit.clone())),
                }
            }
        });
    }
    let items: Vec<Item> = judged.iter().map(|j| copy(&j.item)).collect();
    let verdict = strict(required.complete, &items);
    let refused = matches!(verdict, Verdict::Refused(..));
    let mut answer = rows::answer(
        &judged,
        &verdict,
        rec,
        (&policy_commit, policy_rev),
        &candidate,
    );
    answer.flagged = refused;
    Ok(answer)
}

fn copy(item: &Item) -> Item {
    Item {
        defined: item.defined,
        approved: item.approved,
        standing: item.standing,
    }
}

/// `handoff check` for the run named (else the current one): the answer,
/// the verb's name and its arguments.
pub fn answer(
    session: &Session<'_>,
    (policy, candidate): (&str, &str),
    (run, repo): (Option<&str>, Option<&Path>),
    cwd: &Path,
) -> Result<(Answer, &'static str, Vec<String>)> {
    let rec = crate::gate::target(session, run)?;
    let answer = check(session, &rec, (policy, candidate), repo, cwd)?;
    Ok((
        answer,
        "handoff",
        vec![policy.to_owned(), candidate.to_owned(), rec.id],
    ))
}
