//! A delivery's findings drawn against the payload, in the form of file
//! diagnostics (docs/guide/errors.md): for each failed or unknown check
//! whose receipt is about the present bytes, the line, a caret under the
//! match, the rule and the repair; up to ten, then where the rest are.
//! The span is found again by matching the rule on that line of the
//! present payload; receipts are unchanged and copy no matched text.
//! Nothing is drawn for a stale receipt.
use super::candidate::Candidate;
use super::checks::Finding;
use super::config::{self, CLAIMS, PARAGRAPHS, REFERENCES};
use super::status::Judged;
use std::path::Path;
use writfold_core::gate::{Standing, State};

/// The most findings drawn for one candidate.
const MAX: usize = 10;

struct Ctx<'a> {
    root: &'a Path,
    c: &'a Candidate,
    payload: String,
    path: String,
    check: String,
}

fn caret(out: &mut String, n: u32, text: &str, col: usize, width: usize, label: &str) {
    let pad = n.to_string().len();
    let blank = " ".repeat(pad);
    out.push_str(&format!("{blank} |\n{n} | {text}\n"));
    out.push_str(&format!(
        "{blank} | {}{} {label}\n{blank} |\n",
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(width.max(1))
    ));
}

/// One finding of a failed check.
fn finding(x: &Ctx<'_>, j: &Judged<'_>, f: &Finding) -> String {
    let name = &j.obligation.name;
    let line_text = x
        .payload
        .lines()
        .nth(f.line.saturating_sub(1) as usize)
        .unwrap_or("");
    let at = |col: usize| format!(" --> {}:{}:{col}\n", x.path, f.line);
    let mut out;
    match name.as_str() {
        REFERENCES => {
            let audience =
                x.c.manifest
                    .as_ref()
                    .map_or("internal", |m| m.audience.as_str());
            let deny = config::deny(x.root, audience).ok();
            let rule = deny
                .as_ref()
                .and_then(|d| d.rules.iter().find(|r| r.name == f.rule));
            let span = rule.and_then(|r| r.regex.find(line_text.as_bytes()));
            out = format!(
                "failed[{name}]: line {} matches the deny rule `{}` for audience {audience}\n",
                f.line, f.rule
            );
            let col = span.map_or(1, |m| line_text[..m.start()].chars().count() + 1);
            out.push_str(&at(col));
            if let Some(m) = span {
                let width = line_text[m.start()..m.end()].chars().count();
                caret(&mut out, f.line, line_text, col, width, &f.rule);
            }
            if let (Some(r), Some(d)) = (rule, &deny) {
                let file = d.path.clone().unwrap_or_default();
                let at = std::fs::read_to_string(x.root.join(&file))
                    .ok()
                    .and_then(|t| {
                        t.lines()
                            .position(|l| l.contains(&format!("\"{}:", r.name)))
                    })
                    .map_or(1, |n| n + 1);
                out.push_str(&format!(
                    "  = note: rule `{}: {}` @{file}:{at}\n",
                    r.name, r.pattern
                ));
            }
            out.push_str(&format!(
                "  = help: reword or remove the match, then: {}\n",
                x.check
            ));
        }
        PARAGRAPHS => {
            let mode =
                x.c.manifest
                    .as_ref()
                    .ok()
                    .and_then(|m| config::mode(x.root, &m.mode).ok());
            let (bound, source, mode_name) = mode.map_or((0, String::new(), String::new()), |m| {
                (m.paragraph_words, m.source.clone(), m.name.clone())
            });
            out = format!(
                "failed[{name}]: the paragraph at line {} has {}; this mode takes at most {bound}\n",
                f.line, f.detail
            );
            out.push_str(&at(1));
            let place = match source.as_str() {
                "built-in" => format!("built-in {mode_name}"),
                s => s.to_owned(),
            };
            out.push_str(&format!(
                "  = help: split or shorten it (paragraph_words = {bound} @{place})\n"
            ));
        }
        CLAIMS => {
            out = format!(
                "failed[{name}]: the citation at line {} no longer matches its source ({})\n",
                f.line, f.rule
            );
            let col = line_text
                .find('⟦')
                .map_or(1, |b| line_text[..b].chars().count() + 1);
            out.push_str(&at(col));
            out.push_str(&format!("  = note: {}\n", f.detail));
            out.push_str("  = help: read the source again, correct the sentence if it no longer holds, and replace the token: writfold kb cite make\n");
        }
        _ => {
            out = format!("failed[{name}]: {}\n", f.detail);
            if f.line > 0 {
                out.push_str(&at(1));
            }
            out.push_str(&format!(
                "  = help: edit the payload's source, then: {}\n",
                x.check
            ));
        }
    }
    out
}

/// The drawing for candidate `c`; empty when nothing failed or is unknown
/// about its present bytes.
pub fn draw(root: &Path, c: &Candidate, judged: &[Judged<'_>]) -> String {
    let Some(payload) = super::render::payload(root, c) else {
        return String::new();
    };
    let x = Ctx {
        root,
        c,
        payload: String::from_utf8_lossy(&payload).into_owned(),
        path: super::found::Found::path_of(root, c),
        check: format!("writfold kb deliver check {}", c.id),
    };
    let mut parts: Vec<String> = Vec::new();
    let mut more: Vec<(usize, String)> = Vec::new();
    for j in judged {
        let Some((filed, r)) = j.receipt else {
            continue;
        };
        match j.standing {
            Standing::Checked(State::Failed) => {
                let room = MAX.saturating_sub(parts.len());
                parts.extend(r.findings.iter().take(room).map(|f| finding(&x, j, f)));
                let total = usize::try_from(r.total).unwrap_or(r.findings.len());
                let left = total.saturating_sub(room.min(r.findings.len()));
                if left > 0 {
                    more.push((left, filed.path.clone()));
                }
            }
            Standing::Checked(State::Unknown) if parts.len() < MAX => {
                parts.push(unknown(
                    &x,
                    j,
                    r.note.as_deref().unwrap_or("the check could not decide"),
                ));
            }
            _ => {}
        }
    }
    let mut out = parts.join("\n");
    for (n, path) in more {
        out.push_str(&format!("… and {n} more: {path}\n"));
    }
    out
}

fn unknown(x: &Ctx<'_>, j: &Judged<'_>, note: &str) -> String {
    let name = &j.obligation.name;
    if name == REFERENCES && note.contains("no deny list") {
        let audience =
            x.c.manifest
                .as_ref()
                .map_or("internal", |m| m.audience.as_str());
        return format!(
            "unknown[{name}]: no deny list for audience {audience}\n  = help: the deny list is the team's policy: ask before writing {}/deny-{audience}.toml (rules = [] says nothing is denied)\n",
            config::DIR
        );
    }
    format!("unknown[{name}]: {note}\n  = help: {}\n", x.check)
}
