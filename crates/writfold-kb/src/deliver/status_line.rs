//! The `check:` line of one obligation under a candidate's row, with its
//! additive JSON keys, and the `why:` under a statement the policy does
//! not take.
use super::status::Judged;
use crate::emit::More;
use serde_json::json;
use writfold_core::gate::{Standing, State};

pub(super) fn short(digest: &str) -> &str {
    let d = digest.strip_prefix("sha256:").unwrap_or(digest);
    d.get(..12).unwrap_or(d)
}

/// The `check:` line of one obligation.
pub(super) fn line(j: &Judged<'_>, id: &str) -> More {
    let ob = &j.obligation;
    let mut more = More {
        label: "check",
        ..More::default()
    };
    let said = match j.standing {
        Standing::Attested => j.statements.attested,
        Standing::Waived => j.statements.waived,
        Standing::Checked(_) => None,
    };
    let detail = match (said, j.receipt) {
        (Some((f, s)), _) => {
            more.path = Some(f.path.clone());
            more.line = Some(1);
            match j.standing {
                Standing::Waived if ob.class != "attested" => {
                    format!("by {}: {} (check: {})", s.by, s.text, j.checked.name())
                }
                _ => format!("by {}: {}", s.by, s.text),
            }
        }
        (None, Some((f, r))) => {
            more.path = Some(f.path.clone());
            more.line = Some(1);
            let found: Vec<String> = r
                .findings
                .iter()
                .take(3)
                .map(|x| format!("line {} {} ({})", x.line, x.rule, x.detail))
                .collect();
            let mut text = format!("{} for {}", r.outcome.name(), short(&r.content));
            if !found.is_empty() {
                text.push_str(&format!(": {}", found.join("; ")));
            }
            if r.total > 3 {
                text.push_str(&format!(" (+{} more)", r.total - 3));
            }
            if let Some(note) = r.note.as_ref().filter(|_| r.findings.is_empty()) {
                text.push_str(&format!(" ({note})"));
            }
            text
        }
        (None, None) if ob.class == "attested" => {
            format!(
                "needs: writfold kb deliver attest {id} --check {} --statement TEXT --by WHO",
                ob.name
            )
        }
        (None, None) => format!("needs: writfold kb deliver check {id}"),
    };
    let moved = super::recheck::note(j.moved).unwrap_or_default();
    more.text = format!(
        "{} [{}] {} {detail}{moved}",
        ob.name,
        j.standing.name(),
        ob.class
    );
    more.status = Some(j.standing.name().into());
    if j.standing != Standing::Checked(State::Satisfied) {
        more.flags.push(j.standing.name().into());
    }
    if j.moved.is_some() && !more.flags.iter().any(|f| f == "stale") {
        more.flags.push("stale".into());
    }
    if j.rejected() {
        more.flags.push("not-accepted".into());
    }
    more.extra = keys(j);
    more
}

/// The additive JSON keys of a `check:` line: `check`, `class`, `outcome`
/// and `findings` (`{line, column, width, rule, detail}`; `column` and
/// `width` only when the span is known).
fn keys(j: &Judged<'_>) -> Vec<(&'static str, serde_json::Value)> {
    let mut out = vec![
        ("check", json!(j.obligation.name)),
        ("class", json!(j.obligation.class)),
    ];
    if let Some((_, r)) = j.receipt {
        out.push(("outcome", json!(r.outcome.name())));
        let findings: Vec<serde_json::Value> = r
            .findings
            .iter()
            .map(|f| {
                let column = f
                    .detail
                    .strip_prefix("column ")
                    .and_then(|c| c.parse::<u64>().ok());
                match column {
                    Some(col) => {
                        json!({"line": f.line, "column": col, "rule": f.rule, "detail": f.detail})
                    }
                    None => json!({"line": f.line, "rule": f.rule, "detail": f.detail}),
                }
            })
            .collect();
        out.push(("findings", json!(findings)));
    }
    out
}

/// The `why:` under a statement the policy does not take.
pub(super) fn rejected(j: &Judged<'_>) -> Option<More> {
    j.rejected().then(|| More {
        label: "why",
        text: super::policy::why_rejected(&j.obligation, j.standing.name()),
        ..More::default()
    })
}
