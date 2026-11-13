//! `checker list` (each declared checked kind: its checker found or
//! absent, and what `describe` says) and `cite make --kind <kind>
//! key=value …` (a checked kind's token from its checker's `make`). Nothing
//! is written; `cite make` prints the token for the caller to place.
use super::kinds::{self, DIR, Kind};
use super::{exchange, proto, spawn};
use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, write::cite};
use serde_json::{Map, Value};

/// A checker verb with its arguments.
#[derive(Clone, Debug)]
pub enum CheckerVerb {
    List,
    /// `cite make --kind <kind> key=value …`.
    Make {
        kind: String,
        fields: Vec<(String, String)>,
    },
}

/// Answer one checker verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &CheckerVerb, now: i64) -> Result<Output> {
    let tool = match &ws.repo {
        Some(_) => Some(GitTool::locate()?),
        None => None,
    };
    let session = Session::open(ws, tool.as_ref(), false, now)?;
    let declared = kinds::read(&ws.root)?;
    let (answer, name, args) = match verb {
        CheckerVerb::List => (list(ws, &declared), "checker-list", Vec::new()),
        CheckerVerb::Make { kind, fields } => {
            let decl = declared
                .get(kind)
                .filter(|_| declared.checker(kind).is_some());
            let decl = decl.ok_or_else(|| {
                KbError::Refused(format!(
                    "kind {kind} is not declared freshness = \"checked\" in {DIR}/{kind}.toml"
                ))
            })?;
            let mut args = vec![kind.clone()];
            args.extend(fields.iter().map(|(k, v)| format!("{k}={v}")));
            (make(ws, decl, fields, now)?, "cite-make", args)
        }
    };
    let frame = Frame {
        verb: name.into(),
        args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    session.emit(&answer, &frame)
}

fn list(ws: &Workspace, declared: &kinds::Kinds) -> Answer {
    let mut answer = Answer::default();
    let mut unknown = 0;
    for kind in declared.iter() {
        let Some(name) = declared.checker(&kind.name) else {
            continue;
        };
        let program = spawn::program(name);
        let (status, text) = match exchange(&ws.root, name, &kind.name, "check", &[]) {
            None => ("absent", format!("{program} is not on PATH")),
            Some(x) => match (x.refused(&kind.name, "check"), &x.description) {
                (None, Some(d)) => (
                    "found",
                    format!(
                        "{program} {} {} ops {} token versions {:?}",
                        d.name,
                        d.version,
                        d.ops.join(","),
                        d.token_versions
                    ),
                ),
                (why, _) => ("failed", format!("{program}: {}", why.unwrap_or_default())),
            },
        };
        if status != "found" {
            unknown += 1;
        }
        answer.rows.push(Row {
            kind: "checker",
            id: kind.name.clone(),
            status: Some(status.into()),
            text,
            ..Row::default()
        });
    }
    if answer.rows.is_empty() {
        answer.notes.push(format!(
            "no kind in {DIR} is declared freshness = \"checked\""
        ));
    }
    answer.counts = vec![("checked-kinds", answer.rows.len()), ("unknown", unknown)];
    answer
}

fn refused(kind: &str, why: String) -> KbError {
    KbError::Refused(format!("checker-failed ({kind}): {why}"))
}

/// The token `make` returns, checked against the declaration.
fn make(ws: &Workspace, decl: &Kind, pairs: &[(String, String)], now: i64) -> Result<Answer> {
    let kind = decl.name.as_str();
    let name = decl.checker.as_deref().unwrap_or_default();
    let mut sent = Map::new();
    for (key, value) in pairs {
        sent.insert(key.clone(), Value::String(value.clone()));
    }
    let request = [("0".to_owned(), Value::Object(sent))];
    let x = exchange(&ws.root, name, kind, "make", &request).ok_or_else(|| {
        KbError::Refused(format!(
            "checker-absent ({kind}): {} is not on PATH",
            spawn::program(name)
        ))
    })?;
    if let Some(why) = x.refused(kind, "make") {
        return Err(refused(kind, why));
    }
    let returned = match x.replies.get("0").map(Vec::as_slice) {
        Some([one]) => one.fields.clone(),
        Some([]) | None => return Err(refused(kind, "no reply".into())),
        Some(_) => return Err(refused(kind, "more than one reply".into())),
    };
    let returned = returned.ok_or_else(|| refused(kind, "a reply without fields".into()))?;
    let reads = x.description.map(|d| d.token_versions).unwrap_or_default();
    let fields = checked_fields(decl, &returned, &reads)?;
    let mut parts = vec![("kind", kind.to_owned())];
    parts.extend(fields.iter().map(|(k, v)| (k.as_str(), v.clone())));
    parts.push(("seen", cite::seen(now)));
    let token = cite::token(&parts)?;
    Ok(Answer {
        rows: vec![Row {
            kind: "cite",
            id: crate::verbs::stale::target(&token),
            text: token,
            ..Row::default()
        }],
        notes: vec![format!("made by {}", spawn::program(name))],
        ..Answer::default()
    })
}

/// The returned fields in declaration order (then `v`), refusing any the
/// declaration does not list and requiring every `required` one.
fn checked_fields(
    decl: &Kind,
    returned: &Map<String, Value>,
    reads: &[u64],
) -> Result<Vec<(String, String)>> {
    let kind = decl.name.as_str();
    let mut text = Map::new();
    for (key, value) in returned {
        let Some(value) = value.as_str() else {
            return Err(refused(kind, format!("field {key} is not a string")));
        };
        if key != "v" && !decl.fields.contains(key) {
            return Err(KbError::Refused(format!(
                "the checker returned field {key}, which {DIR}/{kind}.toml does not list in fields"
            )));
        }
        text.insert(key.clone(), Value::String(value.to_owned()));
    }
    let version = proto::token_version(&Value::Object(text.clone()));
    if !version.is_some_and(|v| reads.contains(&v)) {
        return Err(refused(kind, "v= is not a token version it reads".into()));
    }
    if let Some(field) = decl.required.iter().find(|f| !text.contains_key(*f)) {
        return Err(KbError::Refused(format!(
            "the checker did not return {field}=, which {DIR}/{kind}.toml requires"
        )));
    }
    let get = |k: &str| text.get(k).and_then(Value::as_str).map(str::to_owned);
    let mut fields: Vec<(String, String)> = decl
        .fields
        .iter()
        .filter_map(|k| Some((k.clone(), get(k)?)))
        .collect();
    fields.extend(get("v").map(|v| ("v".to_owned(), v)));
    Ok(fields)
}
