//! The rows `gate status`, `gate run` and `resume` show for an assessed
//! gate, and its `gate lines` line. Every recorded check gets a `check:`
//! line with Weft's result and Writfold's basis; a hypothetical check is
//! flagged `!hypothetical` and never counted as met; an ungrounded
//! citation behind it gets a `cite:` line naming what it stood on.
use super::Assessment;
use super::receipt::Assessed;
use crate::emit::{More, Row};
use crate::gate::status::{Judged, shown_path};
use crate::gate::subject::Subject;
use writfold_core::assess::{Basis, Grounding};
use writfold_core::gate::{Standing, State};

fn short(text: &str) -> &str {
    let text = text.strip_prefix("sha256:").unwrap_or(text);
    text.get(..12).unwrap_or(text)
}

fn more(label: &'static str, text: String, path: Option<&str>, flags: Vec<String>) -> More {
    More {
        label,
        text,
        line: path.map(|_| 1),
        path: path.map(str::to_owned),
        flags,
    }
}

/// `met=N/M`: required checks met, of those the definition requires.
fn met(a: &Assessed, judged: &Judged<'_>) -> (usize, usize) {
    let required = judged
        .definition
        .assessed
        .as_ref()
        .map_or(0, |s| s.checks.len());
    let met = a.checks.iter().filter(|c| c.required && c.met).count();
    (met, required)
}

/// The `check:` and `cite:` lines of a receipt.
fn checks(a: &Assessed, out: &mut Vec<More>) {
    for c in &a.checks {
        let mut flags = Vec::new();
        let mut text = format!("{} [{} {}]", c.name, c.verdict.name(), c.basis.name());
        if !c.required {
            text.push_str(" (not required)");
        } else if c.met {
            text.push_str(" met");
        } else {
            text.push_str(" not met");
        }
        let resting = |g: Grounding| -> Vec<&str> {
            c.premises
                .iter()
                .filter(|p| {
                    a.assumptions
                        .iter()
                        .any(|s| &s.name == *p && s.grounding == g)
                })
                .map(String::as_str)
                .collect()
        };
        if c.basis == Basis::Hypothetical {
            let ungrounded = resting(Grounding::Ungrounded).join(", ");
            text.push_str(&format!("; rests on ungrounded {ungrounded}"));
            flags.push("hypothetical".into());
        } else if !resting(Grounding::Attested).is_empty() {
            let attested = resting(Grounding::Attested).join(", ");
            text.push_str(&format!("; rests on attested {attested}"));
        }
        out.push(more("check", text, None, flags));
    }
    for name in &a.missing {
        out.push(more(
            "check",
            format!("{name} [missing] not met: the Weft source has no such check"),
            None,
            vec!["missing".into()],
        ));
    }
    for s in &a.assumptions {
        if s.citations.is_empty() {
            out.push(more(
                "cite",
                format!("{} [ungrounded] cites nothing", s.name),
                None,
                Vec::new(),
            ));
        }
        for c in s
            .citations
            .iter()
            .filter(|c| c.grounding != Grounding::Grounded)
        {
            let text = format!(
                "{} {} [{}] {}",
                s.name,
                c.binding,
                c.grounding.name(),
                c.standing
            );
            out.push(more("cite", text, c.path.as_deref(), Vec::new()));
        }
    }
}

/// The row of one assessed gate.
pub fn row(subject: &Subject, judged: &Judged<'_>, assessment: &Assessment<'_>) -> Row {
    let d = &judged.definition;
    let mut row = Row {
        kind: "gate",
        id: d.id.clone(),
        status: Some(judged.standing.name().into()),
        section: Some("gates"),
        ..Row::default()
    };
    match assessment.receipt {
        None => {
            row.text = format!("assessed no receipt in this run for {}", subject.name);
            row.path = Some(shown_path(d, subject));
            row.line = Some(1);
        }
        Some((f, a)) => {
            let (met, required) = met(a, judged);
            row.text = format!(
                "assessed {} met={met}/{required} weft={} bin={} at {}@{}",
                a.outcome.name(),
                short(&a.source),
                a.binary.as_deref().map_or("none", short),
                a.repo,
                short(&a.commit)
            );
            row.path = Some(f.path.clone());
            row.line = Some(1);
            checks(a, &mut row.more);
            if let Some(error) = &a.error {
                row.more.push(more("why", error.clone(), None, Vec::new()));
            }
        }
    }
    if let Some(read) = crate::gate::status::read_line(d, &subject.name) {
        row.more.push(read);
    }
    if judged.standing == Standing::Checked(State::Satisfied) {
        return row;
    }
    row.flags.push(judged.standing.name().into());
    if let Some(why) = &assessment.why {
        row.more.push(more("why", why.clone(), None, Vec::new()));
    }
    let weft = d.assessed.as_ref().map_or("", |s| s.weft.as_str());
    row.more.push(more(
        "next",
        format!("writfold kb gate run {} (Weft source {weft})", d.id),
        None,
        Vec::new(),
    ));
    row
}

/// The `gate lines` line of one assessed gate.
pub fn line(judged: &Judged<'_>, assessment: &Assessment<'_>, width: usize) -> String {
    let id = format!("{}:", judged.definition.id);
    let standing = judged.standing.name();
    match assessment.receipt {
        Some((f, a)) => {
            let (met, required) = met(a, judged);
            format!(
                "{id:<w$} {standing} assessed {} met={met}/{required} {}\n",
                a.outcome.name(),
                f.path,
                w = width + 1
            )
        }
        None => format!("{id:<w$} {standing}\n", w = width + 1),
    }
}
