//! `observe --source KIND --locator TEXT --status STATUS [--revision R]
//! [--for GATE.ITEM] [--note TEXT] [--cite TOKEN] [--run ID]`: record one
//! observation ([`crate::observe`]) in the named or current run.
//!
//! An `ok` or `partial` observation embeds one citation token: the one
//! `--cite` passes (it must be a single well-formed token of kind
//! `KIND`), else `⟦cite kind=KIND locator=TEXT [rev=R] seen=<now>⟧`. A kind
//! Writfold checks against Git or the index, or one declared `checked`,
//! needs `--cite` (from `cite make`), since only those tokens can be
//! checked. The other statuses take no token.
use super::{Op, Plan, one_line};
use crate::error::{KbError, Result};
use crate::observe::{self, Observation};
use crate::parse::cite::tokens;
use crate::session::Session;
use crate::stale::kinds::{self, BUILT_IN};
use crate::{runs, time};
use writfold_core::observe::{Status, embeds};

/// One observation to record.
#[derive(Clone, Debug, Default)]
pub struct Observe {
    pub source: String,
    pub locator: String,
    pub status: String,
    pub revision: Option<String>,
    /// `<gate>.<item>`.
    pub item: Option<String>,
    pub note: Option<String>,
    pub cite: Option<String>,
    /// The run; `None` takes the current-run pointer.
    pub run: Option<String>,
}

fn usage(text: impl Into<String>) -> KbError {
    KbError::Usage(text.into())
}

fn optional(flag: &str, text: Option<&String>) -> Result<Option<String>> {
    text.map(|t| one_line(flag, t)).transpose()
}

/// The `--cite` token, checked to be one well-formed token of `source`.
fn given(source: &str, token: &str) -> Result<String> {
    let token = token.trim();
    let read = tokens(token);
    let ok = match read.as_slice() {
        [one] => {
            !one.illustrative
                && one.complete
                && one.malformed.is_none()
                && one.raw.trim() == token
                && one.kind.as_deref() == Some(source)
        }
        _ => false,
    };
    match ok {
        true => Ok(token.to_owned()),
        false => Err(usage(format!(
            "--cite must be exactly one well-formed citation token of kind={source}"
        ))),
    }
}

/// The embedded token of an `ok` or `partial` observation.
fn token(session: &Session<'_>, o: &Observe, source: &str, locator: &str) -> Result<String> {
    if let Some(cite) = &o.cite {
        return given(source, cite);
    }
    if BUILT_IN.contains(&source) {
        return Err(usage(format!(
            "kind {source} is checked against Git or the index: pass --cite with its token (cite make)"
        )));
    }
    if kinds::read(&session.ws.root)?.checker(source).is_some() {
        return Err(usage(format!(
            "kind {source} is declared checked: pass --cite with the token cite make --kind {source} prints"
        )));
    }
    let mut fields = vec![("kind", source.to_owned()), ("locator", locator.to_owned())];
    if let Some(rev) = &o.revision {
        fields.push(("rev", rev.clone()));
    }
    fields.push(("seen", super::cite::seen(session.now)));
    super::cite::token(&fields)
}

/// The observation's content, validated.
fn shaped(session: &Session<'_>, o: &Observe, run: &str) -> Result<Observation> {
    let source = one_line("--source", &o.source)?;
    if !kinds::is_word(&source) {
        return Err(usage(format!(
            "--source {source:?} must be a kind name ([a-z0-9._-])"
        )));
    }
    let locator = one_line("--locator", &o.locator)?;
    let status = Status::parse(&o.status).ok_or_else(|| {
        usage(format!(
            "--status must be one of ok, partial, no-data, not-found, timeout, denied, error, not {:?}",
            o.status
        ))
    })?;
    let item = match &o.item {
        None => None,
        Some(text) => Some(
            observe::item(text).ok_or_else(|| usage(format!("--for {text:?} is not GATE.ITEM")))?,
        ),
    };
    let cite = match (embeds(status), &o.cite) {
        (true, _) => Some(token(session, o, &source, &locator)?),
        (false, Some(_)) => {
            return Err(usage(format!(
                "--cite is for ok and partial observations; a {} look embeds no citation",
                status.name()
            )));
        }
        (false, None) => None,
    };
    Ok(Observation {
        run: run.to_owned(),
        source,
        locator,
        status,
        revision: optional("--revision", o.revision.as_ref())?,
        item,
        note: optional("--note", o.note.as_ref())?,
        cite,
        observed: time::stamp(session.now),
    })
}

/// Plan one observation in the named or current run.
pub fn plan(session: &Session<'_>, o: &Observe) -> Result<Plan> {
    let rec = super::run::target(session, o.run.as_deref())?;
    if runs::state(session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    let observation = shaped(session, o, &rec.id)?;
    let name = super::unique(session.now, &observation.locator);
    let path = format!("{}/{}/{name}.json", runs::dir(&rec), observe::DIR);
    let status = observation.status;
    let mut notes = vec![format!("recorded {path} ({})", status.name())];
    notes.push(match status {
        Status::Ok => {
            "evidence while its citation is fresh (cite it on a supported cover row)".into()
        }
        _ => format!(
            "a {} look is shown under its item, never evidence for supported",
            status.name()
        ),
    });
    Ok(Plan {
        ops: vec![Op::Create {
            path,
            bytes: observe::bytes(&observation),
        }],
        show: None,
        notes,
        current: None,
        verb: "observe",
        args: vec![rec.id, name],
    })
}
