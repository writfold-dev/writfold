//! The pins of a recorded look that found nothing (gate-evidence spec,
//! stage 4): it still reads as having found nothing only while every pin
//! holds ([`writfold_core::pin::nothing_found`]).
//!
//! - **Time, from the kind catalog.** The look's `--source` kind's
//!   declaration (`.writfold/sources/KIND.toml`) gives its lifetime: an
//!   immutable kind never ages, a `default_ttl` ages it that long after it
//!   was observed, an undeclared kind (or one with no default) never ages.
//!   The lifetime is chosen by [`writfold_core::verdict::lifetime`] and the
//!   pin by [`writfold_core::pin::timed`], the rule citations age by.
//! - **What it searched.** An optional `searched` citation token, judged
//!   like any citation: fresh holds, expired ages, anything else moved,
//!   unreadable when it is not exactly one token that can be checked.
//!
//! Reading the catalog, the clock and the citation is plumbing; the
//! decisions are the kernel's.
use super::ground::Context;
use crate::error::Result;
use crate::observe::Observation;
use crate::parse::cite::tokens;
use crate::stale::Verdict;
use crate::stale::kinds::{Freshness, Kinds};
use crate::time::{self, Ttl};
use writfold_core::pin::{self, Pin};
use writfold_core::verdict::{Lifetime, lifetime};

/// The time pin of a look observed at `observed` with `kind`'s lifetime.
pub fn timed(kinds: &Kinds, kind: &str, observed: &str, now: i64) -> Pin {
    let declared = kinds.get(kind);
    let immutable = declared.is_some_and(|d| d.freshness == Freshness::Immutable);
    let default = declared.and_then(|d| d.default_ttl.as_deref());
    let seconds = match default.map(|t| time::ttl(Some(t))) {
        None | Some(Some(Ttl::Never)) => None,
        Some(Some(Ttl::Seconds(s))) => Some(s),
        Some(None) => return Pin::Unreadable,
    };
    let life: Lifetime<i64> = lifetime(immutable, None, seconds);
    match time::parse(observed) {
        Some(at) => pin::timed(life, at, now),
        None => Pin::Unreadable,
    }
}

/// The pin of a `searched` token.
fn searched(cx: &Context<'_, '_>, token: &str, path: &str) -> Result<Pin> {
    let found = tokens(token);
    if found.len() != 1 {
        return Ok(Pin::Unreadable);
    }
    let checked = super::ground::verdicts(cx, found, path)?;
    Ok(match checked.as_slice() {
        [one] => match one.verdict {
            Verdict::Fresh => Pin::Held,
            Verdict::Expired => Pin::Aged,
            Verdict::Unverifiable(_) => Pin::Unreadable,
            _ => Pin::Moved,
        },
        _ => Pin::Unreadable,
    })
}

/// Every pin of a look.
pub fn of(cx: &Context<'_, '_>, kinds: &Kinds, o: &Observation, path: &str) -> Result<Vec<Pin>> {
    let mut pins = vec![timed(kinds, &o.source, &o.observed, cx.session.now)];
    if let Some(token) = &o.searched {
        pins.push(searched(cx, token, path)?);
    }
    Ok(pins)
}

/// The standing word of a look that no longer reads as having found
/// nothing: the first pin that does not hold.
pub fn word(pins: &[Pin]) -> &'static str {
    match pins.iter().find(|p| **p != Pin::Held) {
        Some(Pin::Aged) => "aged",
        Some(Pin::Moved) => "searched-moved",
        Some(Pin::Unreadable) => "unreadable",
        Some(Pin::Held) | None => "held",
    }
}

/// What ageing a receipt reads: the kind catalog and the time now.
/// A catalog that cannot be read leaves every ageing receipt
/// unreadable, so it never holds; gates that do not age are unaffected.
pub struct Clock {
    pub kinds: Option<Kinds>,
    pub now: i64,
}

impl Clock {
    pub fn read(session: &crate::session::Session<'_>) -> Clock {
        Clock {
            kinds: crate::stale::kinds::read(&session.ws.root).ok(),
            now: session.now,
        }
    }
}

/// The pins of a receipt recorded at `recorded` of a gate that ages as
/// `ages_as`: none when it does not age (its standing is then exactly the
/// gate kernel's, `pin::current_under_refines`).
pub fn receipt(clock: Option<&Clock>, ages_as: Option<&str>, recorded: &str) -> Vec<Pin> {
    match (clock, ages_as) {
        (Some(clock), Some(kind)) => vec![match &clock.kinds {
            Some(kinds) => timed(kinds, kind, recorded, clock.now),
            None => Pin::Unreadable,
        }],
        _ => Vec::new(),
    }
}
