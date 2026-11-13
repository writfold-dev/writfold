//! The state of an assessed gate, derived on every read: G1's kernel over
//! the gate's valid receipts for the subject, where a receipt is about
//! the present when its definition digest, its Weft source digest and the
//! standing of every citation it recorded all equal what is read now.
use super::Filed;
use super::receipt::Assessed;
use crate::error::Result;
use crate::gate::def::Definition;
use crate::gate::ground::{self, Context};
use crate::gate::statement;
use crate::gate::status::{Filings, Judged, Report};
use crate::gate::subject::Subject;
use crate::session::Session;
use std::collections::BTreeMap;
use writfold_core::gate::{Evidence, Statements, standing, state};

/// What an assessed gate's row shows besides its standing.
pub struct Assessment<'a> {
    /// The receipt that decides the state, if any.
    pub receipt: Option<(&'a Filed, &'a Assessed)>,
    /// Why that receipt is stale, when it is.
    pub why: Option<String>,
}

/// The source's present digest, or why it cannot be read.
fn source(
    d: &Definition,
    root: &std::path::Path,
    run_dir: &str,
) -> std::result::Result<String, String> {
    let spec = d.assessed.as_ref().ok_or("not an assessed gate")?;
    let file = spec.source.file(&d.dir, &root.join(run_dir))?;
    crate::gate::places::digest_file(&file)
}

/// The present standing of each reference, read once per reference.
struct Present<'c, 's, 'r> {
    cx: &'c Context<'s, 'r>,
    seen: BTreeMap<String, String>,
}

impl Present<'_, '_, '_> {
    fn standing(&mut self, reference: &str) -> Result<String> {
        if let Some(s) = self.seen.get(reference) {
            return Ok(s.clone());
        }
        let now = ground::resolve(self.cx, reference)?.standing;
        self.seen.insert(reference.to_owned(), now.clone());
        Ok(now)
    }

    /// The first recorded citation whose standing moved: `(binding,
    /// reference, then, now)`.
    fn moved(&mut self, a: &Assessed) -> Result<Option<(String, String, String, String)>> {
        for c in a.assumptions.iter().flat_map(|s| &s.citations) {
            let now = self.standing(&c.reference)?;
            if now != c.standing {
                return Ok(Some((
                    c.binding.clone(),
                    c.reference.clone(),
                    c.standing.clone(),
                    now,
                )));
            }
        }
        Ok(None)
    }
}

fn short(reference: &str) -> String {
    let one: String = reference.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.chars().count() > 60 {
        true => format!("{}…", one.chars().take(59).collect::<String>()),
        false => one,
    }
}

/// Judge every assessed definition on `subject`; `plain` is the report of
/// the run's other gates, which `gate:<id>` and receipt citations read.
pub fn judge<'a>(
    session: &Session<'_>,
    subject: &Subject,
    filings: &'a Filings,
    plain: &Report<'a>,
    definitions: Vec<Definition>,
) -> Result<Vec<Judged<'a>>> {
    let cx = Context {
        session,
        subject,
        plain,
        filings,
        run_dir: &filings.run_dir,
    };
    let mut present = Present {
        cx: &cx,
        seen: BTreeMap::new(),
    };
    let mut out = Vec::new();
    for d in definitions {
        let now = source(&d, &session.ws.root, &filings.run_dir);
        let mut mine: Vec<(&Filed, &Assessed)> = filings
            .assessed
            .iter()
            .filter_map(|f| f.parsed.as_ref().ok().map(|a| (f, a)))
            .filter(|(_, a)| a.gate == d.id && a.repo == subject.name)
            .collect();
        mine.sort_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)));
        let mut evidence = Vec::new();
        for (_, a) in &mine {
            let same_definition = a.definition == d.digest;
            let same_inputs = same_definition
                && now.as_deref() == Ok(a.source.as_str())
                && present.moved(a)?.is_none();
            evidence.push(Evidence {
                outcome: a.outcome,
                same_definition,
                same_scope: same_inputs,
            });
        }
        let (state, at) = state(&evidence);
        let receipt = at.and_then(|i| mine.get(i).copied());
        let why = match (state, receipt) {
            (writfold_core::gate::State::Stale, Some((_, a))) => {
                Some(stale_why(&d, a, &now, &mut present)?)
            }
            _ => None,
        };
        out.push(Judged {
            same_definition: receipt.is_none_or(|(_, a)| a.definition == d.digest),
            standing: standing(false, state, Statements::default()),
            statements: statement::summary(&[], &d.id, &subject.name, &d.digest, ""),
            assessment: Some(Assessment { receipt, why }),
            checklist: None,
            receipt: None,
            definition: d,
            state,
        });
    }
    Ok(out)
}

fn stale_why(
    d: &Definition,
    a: &Assessed,
    now: &std::result::Result<String, String>,
    present: &mut Present<'_, '_, '_>,
) -> Result<String> {
    if a.definition != d.digest {
        return Ok("the definition changed since this receipt".into());
    }
    match now {
        Err(why) => return Ok(format!("the Weft source cannot be read: {why}")),
        Ok(digest) if *digest != a.source => {
            return Ok(format!(
                "the Weft source changed ({} is not the assessed bytes)",
                a.weft
            ));
        }
        Ok(_) => {}
    }
    Ok(match present.moved(a)? {
        Some((binding, reference, then, now)) => format!(
            "cited evidence moved: {binding} ({}) was {then}, now {now}",
            short(&reference)
        ),
        None => "the inputs changed".into(),
    })
}
