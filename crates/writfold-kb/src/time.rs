//! UTC instants as seconds since the Unix epoch, for `seen + ttl` and
//! `verified_at + ttl`. Only the forms the KB writes are read: `YYYY-MM-DD`,
//! optionally followed by `THH:MM[:SS]Z`.

/// Seconds since the epoch, now.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Days since the epoch of a proleptic Gregorian date.
fn days(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let shifted = (month + 9) % 12;
    let of_year = (153 * shifted + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_era_days - 719_468
}

fn number(text: &str) -> Option<i64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// The instant a KB timestamp names, or `None` when it is not one.
pub fn parse(text: &str) -> Option<i64> {
    let text = text.trim();
    let date = text.get(..10)?;
    let mut parts = date.split('-');
    let (year, month, day) = (
        number(parts.next()?)?,
        number(parts.next()?)?,
        number(parts.next()?)?,
    );
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut seconds = days(year, month, day) * 86_400;
    let rest = &text[10..];
    if rest.is_empty() {
        return Some(seconds);
    }
    let clock = rest.strip_prefix('T')?.strip_suffix('Z')?;
    let mut fields = clock.split(':');
    let hours = number(fields.next()?)?;
    let minutes = number(fields.next()?)?;
    let secs = fields.next().map_or(Some(0), number)?;
    if fields.next().is_some() || hours > 23 || minutes > 59 || secs > 60 {
        return None;
    }
    seconds += hours * 3600 + minutes * 60 + secs;
    Some(seconds)
}

/// A time-to-live: `<N>d`, `<N>h`, or `none` (never expires).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ttl {
    Never,
    Seconds(i64),
}

pub fn ttl(text: Option<&str>) -> Option<Ttl> {
    let text = text.map(str::trim).unwrap_or("none");
    if text.is_empty() || text == "none" {
        return Some(Ttl::Never);
    }
    let (count, unit) = text.split_at(text.len() - 1);
    let count = number(count)?;
    match unit {
        "d" => Some(Ttl::Seconds(count.checked_mul(86_400)?)),
        "h" => Some(Ttl::Seconds(count.checked_mul(3600)?)),
        _ => None,
    }
}

/// Whether `observed + ttl` lies strictly before `now`. `None` when either
/// value cannot be read.
pub fn expired(observed: &str, ttl_text: Option<&str>, now: i64) -> Option<bool> {
    let at = parse(observed)?;
    match ttl(ttl_text)? {
        Ttl::Never => Some(false),
        Ttl::Seconds(seconds) => Some(now - at > seconds),
    }
}

#[cfg(test)]
mod tests {
    use super::{Ttl, expired, parse, ttl};

    #[test]
    fn dates_and_ttls() {
        assert_eq!(parse("1970-01-01"), Some(0));
        assert_eq!(parse("2000-03-01"), Some(951_868_800));
        assert_eq!(parse("2026-11-11T10:30Z"), Some(1_794_393_000));
        assert_eq!(parse("2026-11-11T10:30:05Z"), Some(1_794_393_005));
        assert_eq!(parse("2026-13-01"), None);
        assert_eq!(parse("soon"), None);
        assert_eq!(ttl(Some("30d")), Some(Ttl::Seconds(2_592_000)));
        assert_eq!(ttl(None), Some(Ttl::Never));
        assert_eq!(ttl(Some("3w")), None);
        let now = 86_400 * 31;
        assert_eq!(expired("1970-01-01", Some("30d"), now), Some(true));
        assert_eq!(expired("1970-01-01", Some("31d"), now), Some(false));
    }
}
