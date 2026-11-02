//! Result codecs preserve reported values; decoding does not recompute decisions.
//! `dec_checked` decodes Lean's non-null Checked record, not the optional check response.
use super::codec::*;
use super::formula::{dec_statement, enc_statement};
use super::kb::{dec_dependency, dec_kb, enc_dependency};
use super::workflow::{dec_commit_outcome, dec_journal_entry};
use serde_json::{Value, json};
use writfold_domain::*;

pub fn enc_support(value: &Support) -> Value {
    Value::String(
        match value {
            Support::None => "none",
            Support::Positive => "positive",
            Support::Negative => "negative",
            Support::Both => "both",
        }
        .to_owned(),
    )
}
pub fn enc_freshness(value: &Freshness) -> Value {
    Value::String(
        match value {
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
            Freshness::Unknown => "unknown",
        }
        .to_owned(),
    )
}
pub fn enc_assessment(value: &Assessment) -> Value {
    json!({"support": enc_support(value.support()), "applicable": value.applicable(), "freshness": enc_freshness(value.freshness()), "validated": value.validated(), "contested": value.contested(), "available": value.available(), "usable": value.usable()})
}
pub fn enc_checked(value: Option<&Checked>) -> Value {
    match value {
        None => Value::Null,
        Some(value) => {
            json!({"statement": enc_statement(value.statement()), "dependencies": value.dependencies().iter().map(enc_dependency).collect::<Vec<_>>()})
        }
    }
}

pub fn dec_support(value: &Value) -> Result<Support> {
    match value.as_str() {
        Some("none") => Ok(Support::None),
        Some("positive") => Ok(Support::Positive),
        Some("negative") => Ok(Support::Negative),
        Some("both") => Ok(Support::Both),
        _ => Err(InvalidRequest),
    }
}

pub fn dec_freshness(value: &Value) -> Result<Freshness> {
    match value.as_str() {
        Some("fresh") => Ok(Freshness::Fresh),
        Some("stale") => Ok(Freshness::Stale),
        Some("unknown") => Ok(Freshness::Unknown),
        _ => Err(InvalidRequest),
    }
}

pub fn dec_assessment(value: &Value) -> Result<Assessment> {
    let v = obj(
        value,
        "support applicable freshness validated contested available usable",
    )?;
    Ok(Assessment::new(
        dec_support(field(v, "support")?)?,
        boolean(field(v, "applicable")?)?,
        dec_freshness(field(v, "freshness")?)?,
        boolean(field(v, "validated")?)?,
        boolean(field(v, "contested")?)?,
        boolean(field(v, "available")?)?,
        boolean(field(v, "usable")?)?,
    ))
}

pub fn dec_checked(value: &Value) -> Result<Checked> {
    let v = obj(value, "statement dependencies")?;
    Ok(Checked::new(
        dec_statement(field(v, "statement")?)?,
        items(field(v, "dependencies")?, dec_dependency)?,
    ))
}

/// The application response is the KB, ordered journal, and reported outcome.
pub fn dec_apply_result(value: &Value) -> Result<(KB, Vec<JournalEntry>, CommitOutcome)> {
    let v = obj(value, "kb journal outcome")?;
    Ok((
        dec_kb(field(v, "kb")?)?,
        items(field(v, "journal")?, dec_journal_entry)?,
        dec_commit_outcome(field(v, "outcome")?)?,
    ))
}

/// Outcome order corresponds to input plan order; it need not match journal length.
pub fn dec_sequence_result(value: &Value) -> Result<(KB, Vec<JournalEntry>, Vec<CommitOutcome>)> {
    let v = obj(value, "kb journal outcomes")?;
    Ok((
        dec_kb(field(v, "kb")?)?,
        items(field(v, "journal")?, dec_journal_entry)?,
        items(field(v, "outcomes")?, dec_commit_outcome)?,
    ))
}
