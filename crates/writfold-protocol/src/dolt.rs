//! Pure exact read consistency, not IO permission or knowledge admission.
use std::{error::Error, fmt};
use vstd::prelude::*;
use writfold_domain::{
    DoltToolGeneration,
    dolt::{DOLT_READ_PROFILE, DoltReadPin},
};

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadRefusal { Realm, Kb, Database, Commit, Epoch, Profile, Producer }

/// Complete pin equality plus the actual executing tool and supported profile.
pub open spec fn readable(
    expected: &DoltReadPin, observed: &DoltReadPin, actual: &DoltToolGeneration,
) -> bool {
    expected@ == observed@
        && expected@.producer == actual@
        && expected@.profile == DOLT_READ_PROFILE@
}

/// Ordered refusals describe every failed identity or compatibility obligation.
pub open spec fn read_refusal(
    expected: &DoltReadPin, observed: &DoltReadPin, actual: &DoltToolGeneration,
) -> Option<ReadRefusal> {
    if expected@.realm != observed@.realm { Some(ReadRefusal::Realm) }
    else if expected@.kb != observed@.kb { Some(ReadRefusal::Kb) }
    else if expected@.database != observed@.database { Some(ReadRefusal::Database) }
    else if expected@.commit != observed@.commit { Some(ReadRefusal::Commit) }
    else if expected@.epoch != observed@.epoch { Some(ReadRefusal::Epoch) }
    else if expected@.profile != observed@.profile || expected@.profile != DOLT_READ_PROFILE@ {
        Some(ReadRefusal::Profile)
    }
    else if expected@.producer != observed@.producer || expected@.producer != actual@ {
        Some(ReadRefusal::Producer)
    }
    else { None }
}

/// This body is the allocation-free decision consumed by the store.
/// Refusal precedence follows the complete pin's field order.
pub fn admit_read(
    expected: &DoltReadPin, observed: &DoltReadPin, actual: &DoltToolGeneration,
) -> (result: Result<(), ReadRefusal>)
    ensures
        result.is_ok() <==> readable(expected, observed, actual),
        match result {
            Ok(()) => read_refusal(expected, observed, actual).is_none(),
            Err(error) => read_refusal(expected, observed, actual) == Some(error),
        },
{
    if expected.realm() != observed.realm() { return Err(ReadRefusal::Realm); }
    if expected.kb() != observed.kb() { return Err(ReadRefusal::Kb); }
    if expected.database() != observed.database() { return Err(ReadRefusal::Database); }
    if expected.commit() != observed.commit() { return Err(ReadRefusal::Commit); }
    if expected.epoch() != observed.epoch() { return Err(ReadRefusal::Epoch); }
    if expected.profile() != observed.profile()
        || expected.profile().as_str() != DOLT_READ_PROFILE {
        return Err(ReadRefusal::Profile);
    }
    if expected.producer() != observed.producer() || expected.producer() != actual {
        return Err(ReadRefusal::Producer);
    }
    Ok(())
}
}

impl fmt::Display for ReadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Realm => "Dolt realm mismatch",
            Self::Kb => "Dolt knowledge-base mismatch",
            Self::Database => "Dolt database mismatch",
            Self::Commit => "Dolt exact commit mismatch",
            Self::Epoch => "Dolt schema epoch or digest mismatch",
            Self::Profile => "Dolt storage profile mismatch or unsupported profile",
            Self::Producer => "Dolt producer or actual tool generation mismatch",
        })
    }
}
impl Error for ReadRefusal {}
