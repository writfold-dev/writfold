//! A `record` call as a proposal's arguments, and back.
//!
//! Arguments are kept by flag name (`title`, `target-source`, `assets` …;
//! the positionals as `id`, `type`, `from`, `to`), with surrounding
//! whitespace trimmed and empty values left out, so the same call always
//! has the same digest. Lists are JSON arrays.
use crate::error::{KbError, Result};
use crate::write::Write;
use crate::write::record::{Decision, Link, Ticket};
use serde_json::{Map, Value};

/// The kinds a proposal may hold.
pub const KINDS: [&str; 3] = ["ticket", "decision", "link"];

struct Args(Map<String, Value>);

impl Args {
    fn one(&mut self, key: &str, value: Option<&String>) {
        if let Some(value) = value.map(|v| v.trim()).filter(|v| !v.is_empty()) {
            self.0.insert(key.into(), Value::String(value.to_owned()));
        }
    }

    fn many(&mut self, key: &str, values: &[String]) {
        let items: Vec<Value> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| Value::String(v.to_owned()))
            .collect();
        if !items.is_empty() {
            self.0.insert(key.into(), Value::Array(items));
        }
    }
}

/// The kind and arguments of a `record` write; other writes are refused.
pub fn encode(write: &Write) -> Result<(&'static str, Map<String, Value>)> {
    let mut a = Args(Map::new());
    let kind = match write {
        Write::Ticket(t) => {
            a.one("id", Some(&t.ticket));
            a.one("title", Some(&t.title));
            a.many("assets", &t.assets);
            a.one("status", t.status.as_ref());
            a.one("summary", t.summary.as_ref());
            a.many("prs", &t.prs);
            "ticket"
        }
        Write::Decision(d) => {
            a.one("title", Some(&d.title));
            a.one("target-source", Some(&d.target_source));
            a.one("project", d.project.as_ref());
            a.many("references", &d.references);
            for (key, value) in [
                ("context", &d.context),
                ("evidence", &d.evidence),
                ("change", &d.change),
                ("why", &d.why),
                ("origin", &d.origin),
            ] {
                a.one(key, value.as_ref());
            }
            "decision"
        }
        Write::Link(l) => {
            a.one("type", Some(&l.rel));
            a.one("from", Some(&l.from));
            a.one("to", Some(&l.to));
            a.one("status", l.status.as_ref());
            a.many("evidence", &l.evidence);
            a.one("applicability", l.applicability.as_ref());
            "link"
        }
        _ => {
            return Err(KbError::Usage(
                "a proposal holds a record ticket|decision|link call".into(),
            ));
        }
    };
    Ok((kind, a.0))
}

fn text(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key)?.as_str().map(str::to_owned)
}

fn need(args: &Map<String, Value>, key: &str) -> Result<String> {
    text(args, key).ok_or_else(|| KbError::Usage(format!("the proposal has no {key}")))
}

fn list(args: &Map<String, Value>, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The `record` write a proposal replays.
pub fn decode(kind: &str, args: &Map<String, Value>) -> Result<Write> {
    Ok(match kind {
        "ticket" => Write::Ticket(Ticket {
            ticket: need(args, "id")?,
            title: need(args, "title")?,
            assets: list(args, "assets"),
            status: text(args, "status"),
            summary: text(args, "summary"),
            prs: list(args, "prs"),
        }),
        "decision" => Write::Decision(Decision {
            title: need(args, "title")?,
            target_source: need(args, "target-source")?,
            project: text(args, "project"),
            references: list(args, "references"),
            context: text(args, "context"),
            evidence: text(args, "evidence"),
            change: text(args, "change"),
            why: text(args, "why"),
            origin: text(args, "origin"),
        }),
        "link" => Write::Link(Link {
            rel: need(args, "type")?,
            from: need(args, "from")?,
            to: need(args, "to")?,
            status: text(args, "status"),
            evidence: list(args, "evidence"),
            applicability: text(args, "applicability"),
        }),
        other => return Err(KbError::Usage(format!("unknown proposal kind {other:?}"))),
    })
}

/// One line saying what the call would write.
pub fn summary(kind: &str, args: &Map<String, Value>) -> String {
    let get = |key: &str| text(args, key).unwrap_or_default();
    match kind {
        "ticket" => format!("ticket {}: {}", get("id"), get("title")),
        "decision" => format!("decision: {}", get("title")),
        _ => format!("link {} {} → {}", get("type"), get("from"), get("to")),
    }
}

fn quote(value: &str) -> String {
    let plain = !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./:@,=+".contains(&b));
    if plain {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

/// The `writfold kb record …` command the proposal stands for, quoted for
/// a POSIX shell.
pub fn command(kind: &str, args: &Map<String, Value>) -> String {
    let mut out = format!("writfold kb record {kind}");
    let positional: &[&str] = match kind {
        "ticket" => &["id"],
        "link" => &["type", "from", "to"],
        _ => &[],
    };
    for key in positional {
        out.push(' ');
        out.push_str(&quote(&text(args, key).unwrap_or_default()));
    }
    for (key, value) in args {
        if positional.contains(&key.as_str()) {
            continue;
        }
        let value = match value {
            Value::Array(_) => list(args, key).join(","),
            _ => text(args, key).unwrap_or_default(),
        };
        out.push_str(&format!(" --{key} {}", quote(&value)));
    }
    out
}

/// Every argument value, one per line: where citation tokens and
/// evidence locators are looked for.
pub fn values(args: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::new();
    for value in args.values() {
        match value {
            Value::String(s) => out.push(s.clone()),
            Value::Array(items) => {
                out.extend(items.iter().filter_map(Value::as_str).map(str::to_owned))
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{command, decode, encode};
    use crate::write::Write;
    use crate::write::record::Link;

    #[test]
    fn round_trip_and_command() -> crate::error::Result<()> {
        let link = Write::Link(Link {
            rel: "depends-on".into(),
            from: " a ".into(),
            to: "b".into(),
            evidence: vec!["x@0123456:src/a.rs:1-2".into(), " ".into()],
            ..Link::default()
        });
        let (kind, args) = encode(&link)?;
        let back = decode(kind, &args)?;
        assert!(matches!(&back, Write::Link(l) if l.from == "a" && l.evidence.len() == 1));
        assert_eq!(
            command(kind, &args),
            "writfold kb record link depends-on a b --evidence x@0123456:src/a.rs:1-2"
        );
        assert!(decode("ticket", &args).is_err());
        Ok(())
    }
}
