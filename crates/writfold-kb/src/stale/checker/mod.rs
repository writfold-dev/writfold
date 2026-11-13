//! External checkers: a kind declared `freshness = "checked"` is checked by
//! the executable `writfold-check-<checker>` on `PATH`, over the
//! `writfold-checker/1` protocol ([`proto`]). Only `writfold kb stale`
//! (without `--offline`) runs checkers ([`ask`]): one process per kind per
//! run ([`spawn`]). Every other reader (gate grounding, `why`, the hooks,
//! `proposals accept`) reads the verdict `kb stale` cached, and a token it
//! has not checked is `unverifiable: not-checked`.
//!
//! No failure is ever fresh: an absent helper, a timeout, a non-zero exit,
//! a malformed, missing or duplicate reply, an unknown status, a refused
//! handshake and an unreadable token version each leave the token
//! unverifiable with the reason (decided by the kernel,
//! [`writfold_core::verdict::exchange`]).
pub mod proto;
pub mod spawn;
pub mod verb;

use super::kinds::{self, Kinds};
use super::{Checked, Cite, Verdict, unverifiable};
use crate::error::Result;
use crate::session::Session;
use proto::{Description, Reply};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::{self as kernel, Checker, Handshake, Observed, Said};

/// Verdicts of this run, by citation (path, ordinal).
pub type Answers = BTreeMap<(String, u32), Checked>;

/// One exchange with a checker about one kind.
#[derive(Clone, Debug)]
pub struct Exchange {
    pub description: Option<Description>,
    pub handshake: Handshake,
    /// Why the run or its output is unusable as a whole.
    pub failed: Option<String>,
    pub replies: BTreeMap<String, Vec<Reply>>,
}

impl Exchange {
    /// Why nothing in the exchange counts, if anything says so.
    pub fn refused(&self, kind: &str, op: &str) -> Option<String> {
        self.failed.clone().or_else(|| match self.handshake {
            Handshake::Agreed => None,
            Handshake::Protocol => Some(format!("describe did not answer {}", proto::NAME)),
            Handshake::Kind => Some(format!("describe does not list kind {kind}")),
            Handshake::Op => Some(format!("describe does not list op {op}")),
        })
    }
}

/// Send `describe` and one `op` request per (id, fields) to the checker
/// `name`, run in `root`. `None` when the checker is not on `PATH`.
pub fn exchange(
    root: &Path,
    name: &str,
    kind: &str,
    op: &str,
    requests: &[(String, Value)],
) -> Option<Exchange> {
    let program = spawn::find(name)?;
    let mut input = format!("{}\n", proto::describe());
    for (id, fields) in requests {
        input.push_str(&proto::request(op, id, kind, fields));
        input.push('\n');
    }
    let ran = spawn::run(&program, root, &input);
    let mut lines = ran.lines.iter();
    let description = lines.next().and_then(|l| proto::description(l));
    let handshake = kernel::handshake(description.as_ref().map(|d| d.described(kind, op)));
    let mut failed = ran.failed;
    let mut replies: BTreeMap<String, Vec<Reply>> = BTreeMap::new();
    for (n, line) in lines.enumerate() {
        match proto::reply(line) {
            Ok(r) if requests.iter().any(|(id, _)| *id == r.id) => {
                replies.entry(r.id.clone()).or_default().push(r);
            }
            Ok(r) => {
                failed.get_or_insert(format!("a reply for an unknown id {:?}", r.id));
            }
            Err(why) => {
                failed.get_or_insert(format!("line {}: {why}", n + 2));
            }
        }
    }
    Some(Exchange {
        description,
        handshake,
        failed,
        replies,
    })
}

/// The verdict of one token from its exchange.
fn judged(name: &str, kind: &str, x: &Exchange, id: &str, cite: &Cite) -> Checked {
    let fields: Value = serde_json::from_str(&cite.fields).unwrap_or_default();
    let version = proto::token_version(&fields);
    let reads = x.description.as_ref().map(|d| d.token_versions.as_slice());
    let readable = version.is_some_and(|v| reads.unwrap_or_default().contains(&v));
    let replies = x.replies.get(id).map(Vec::as_slice).unwrap_or_default();
    let said: Vec<Option<Said>> = replies.iter().map(|r| r.said.clone().ok()).collect();
    let checker = kernel::exchange(x.handshake, x.failed.is_none(), readable, &said);
    let reply = replies.first();
    let mut head = format!("checker:{name}");
    if let Some(d) = &x.description {
        head = format!("{head}@{}", d.version);
        if let Some(at) = reply.and_then(|r| r.at.as_deref()) {
            head = format!("{head}:{at}");
        }
    }
    let detail = reply.and_then(|r| r.detail.clone());
    let verdict = match kernel::observed(Observed::Checked(checker)) {
        kernel::Verdict::Fresh => Verdict::Fresh,
        kernel::Verdict::Moved => match reply.and_then(|r| r.moved.clone()) {
            Some(m) => Verdict::Moved {
                path: m.path.filter(|p| cite.field("path").as_ref() != Some(p)),
                first: m.first,
                last: m.last,
            },
            None => Verdict::Unverifiable("moved without a range".into()),
        },
        kernel::Verdict::Changed => Verdict::Changed { detail },
        kernel::Verdict::Gone => Verdict::Gone { detail },
        _ => Verdict::Unverifiable(match checker {
            Checker::Absent => absent(kind),
            Checker::Version => format!(
                "token-version (v={}; {} reads {:?})",
                fields.get("v").and_then(Value::as_str).unwrap_or("1"),
                spawn::program(name),
                reads.unwrap_or_default()
            ),
            Checker::Said(_) => format!("checker: {}", detail.as_deref().unwrap_or("unverifiable")),
            Checker::Failed => {
                let why = x.refused(kind, "check").unwrap_or_else(|| match replies {
                    [] => "no reply".into(),
                    [one] => one.said.clone().err().unwrap_or_default(),
                    _ => "more than one reply".into(),
                });
                format!("checker-failed ({kind}): {why}")
            }
        }),
    };
    Checked { verdict, head }
}

fn absent(kind: &str) -> String {
    format!("checker-absent ({kind})")
}

/// Ask each declared checker about its kind's tokens among `cites`: one
/// process per kind. Malformed tokens and tokens missing a required field
/// are not sent (they are judged without the checker).
pub fn ask(root: &Path, kinds: &Kinds, cites: &[Cite]) -> Answers {
    let mut by_kind: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, cite) in cites.iter().enumerate() {
        let Some(kind) = cite.kind.as_deref().filter(|_| cite.malformed.is_none()) else {
            continue;
        };
        if kinds.checker(kind).is_some() && kinds::missing(kinds, cite, kind).is_none() {
            by_kind.entry(kind).or_default().push(index);
        }
    }
    let mut answers = Answers::new();
    for (kind, indexes) in by_kind {
        let Some(name) = kinds.checker(kind) else {
            continue;
        };
        let requests: Vec<(String, Value)> = indexes
            .iter()
            .map(|i| {
                let fields = serde_json::from_str(&cites[*i].fields).unwrap_or_default();
                (i.to_string(), fields)
            })
            .collect();
        let x = exchange(root, name, kind, "check", &requests);
        for (id, _) in &requests {
            let Some(cite) = id.parse::<usize>().ok().and_then(|i| cites.get(i)) else {
                continue;
            };
            let checked = match &x {
                Some(x) => judged(name, kind, x, id, cite),
                None => Checked {
                    verdict: Verdict::Unverifiable(absent(kind)),
                    head: format!("checker:{name}"),
                },
            };
            answers.insert((cite.path.clone(), cite.ord), checked);
        }
    }
    answers
}

/// The verdict of a token of the checked `kind`: this run's answer when
/// `kb stale` asked, else the cached one, else `not-checked`.
pub fn verdict(
    session: &Session<'_>,
    kinds: &Kinds,
    kind: &str,
    cite: &Cite,
    answers: &Answers,
) -> Result<Checked> {
    if let Some(missing) = kinds::missing(kinds, cite, kind) {
        return Ok(missing);
    }
    if let Some(answer) = answers.get(&(cite.path.clone(), cite.ord)) {
        return Ok(answer.clone());
    }
    let name = kinds.checker(kind).unwrap_or_default();
    Ok(cached(session, name, cite)?.unwrap_or_else(|| unverifiable("not-checked".into())))
}

type Row = (
    String,
    Option<u32>,
    Option<u32>,
    Option<String>,
    String,
    Option<String>,
);

/// The verdict `kb stale` last cached for the same token text from the same
/// checker (this citation's row first).
fn cached(session: &Session<'_>, name: &str, cite: &Cite) -> Result<Option<Checked>> {
    let head = format!("checker:{name}");
    let row: Option<Row> = rusqlite::OptionalExtension::optional(session.conn.query_row(
        "SELECT k.verdict, k.new_first, k.new_last, k.new_path, k.repo_head, k.detail \
         FROM cite_checks k JOIN citations c ON c.path = k.path AND c.ord = k.ord \
         WHERE c.raw = ?1 AND (k.repo_head = ?2 OR k.repo_head GLOB ?2 || '@*') \
         ORDER BY (k.path = ?3 AND k.ord = ?4) DESC, k.path, k.ord LIMIT 1",
        (&cite.raw, &head, &cite.path, cite.ord),
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    ))?;
    let Some((said, first, last, path, head, detail)) = row else {
        return Ok(None);
    };
    let verdict = match (said.as_str(), first, last) {
        ("fresh", _, _) => Verdict::Fresh,
        ("moved", Some(first), Some(last)) => Verdict::Moved { path, first, last },
        ("changed", _, _) => Verdict::Changed { detail },
        ("gone", _, _) => Verdict::Gone { detail },
        ("unverifiable", _, _) => Verdict::Unverifiable(detail.unwrap_or(said)),
        _ => Verdict::Unverifiable("not-checked".into()),
    };
    Ok(Some(Checked { verdict, head }))
}

/// The session-start line naming each declared checker not on `PATH`
/// (a `PATH` stat only; nothing is run). `None` when every one is found.
pub fn absent_line(root: &Path) -> Option<String> {
    let kinds = kinds::read(root).ok()?;
    let missing: Vec<String> = kinds
        .iter()
        .filter_map(|k| Some((k.name.as_str(), kinds.checker(&k.name)?)))
        .filter(|(_, name)| spawn::find(name).is_none())
        .map(|(kind, name)| format!("{} (kind {kind})", spawn::program(name)))
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "\nCheckers not on PATH: {}; their citations stay unverifiable (`writfold kb checker list`)\n",
            missing.join(", ")
        )
    })
}
