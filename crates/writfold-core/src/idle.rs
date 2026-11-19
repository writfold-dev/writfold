//! Whether a run is idle: how long ago its last recorded write was, from
//! stamps read out of tracked text (never file modification times), so
//! every clone answers the same. The age filters and is shown; it decides
//! no state. Translated and proved (`proofs/Writfold/Idle.lean`): the
//! newest stamp is the first greatest one, and `idle` compares over the
//! integers, so no clock value overflows it.

/// Whether stamp `i` is newer than the newest so far. A position past the
/// end has no stamp, so it is never newer.
pub fn newer(stamps: &[i64], best: Option<(usize, i64)>, i: usize) -> bool {
    match stamps.get(i) {
        None => false,
        Some(stamp) => match best {
            None => true,
            Some((_, at)) => *stamp > at,
        },
    }
}

/// The newest stamp, and its index (the first of equal newest stamps).
pub fn newest(stamps: &[i64]) -> Option<(usize, i64)> {
    let mut best: Option<(usize, i64)> = None;
    let mut i = 0;
    while i < stamps.len() {
        if newer(stamps, best, i) {
            best = Some((i, stamps[i]));
        }
        i += 1;
    }
    best
}

/// `now - last` over the integers, held inside `i64`: past either end it
/// is that end.
pub fn since(now: i64, last: i64) -> i64 {
    if last < 0 && now > i64::MAX + last {
        return i64::MAX;
    }
    if last > 0 && now < i64::MIN + last {
        return i64::MIN;
    }
    now - last
}

/// `Some((index of the newest stamp, seconds since it))` when the newest
/// stamp is more than `age` seconds before `now` (over the integers);
/// `None` when it is not, or there is no stamp. The seconds shown are held
/// inside `i64`.
pub fn idle(now: i64, stamps: &[i64], age: i64) -> Option<(usize, i64)> {
    match newest(stamps) {
        None => None,
        Some((at, last)) => {
            if crate::verdict::expired(now, last, age) {
                Some((at, since(now, last)))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{idle, newer, newest};

    #[test]
    fn idle_is_measured_from_the_newest_stamp() {
        let day = 86_400;
        type Case<'a> = (&'a [i64], i64, Option<(usize, i64)>);
        let table: [Case<'_>; 7] = [
            (&[], day, None),
            (&[0], day, Some((0, 10 * day))),
            (&[0, 9 * day], day, None),
            (&[9 * day, 0], 2 * day, None),
            (&[0, 5 * day, 3 * day], 4 * day, Some((1, 5 * day))),
            // The boundary second: exactly the age is not older than it.
            (&[3 * day], 7 * day, None),
            (&[3 * day - 1], 7 * day, Some((0, 7 * day + 1))),
        ];
        for (stamps, age, want) in table {
            assert_eq!(idle(10 * day, stamps, age), want, "{stamps:?} {age}");
        }
        assert_eq!(newest(&[2, 7, 7, 1]), Some((1, 7)));
    }

    /// Clock values far apart never overflow: the age is held at the end
    /// of `i64`, and a stamp far in the future is never idle.
    #[test]
    fn far_apart_stamps_do_not_overflow() {
        assert_eq!(idle(i64::MAX, &[i64::MIN], 0), Some((0, i64::MAX)));
        assert_eq!(idle(i64::MIN, &[i64::MAX], i64::MIN), None);
        assert_eq!(idle(i64::MAX, &[-1], i64::MAX), Some((0, i64::MAX)));
        assert_eq!(idle(i64::MAX, &[0], i64::MAX), None);
    }

    /// A position past the end has no stamp, so it is never newer.
    #[test]
    fn newer_past_the_end_is_false() {
        assert!(!newer(&[], None, 0));
        assert!(!newer(&[5], Some((0, 1)), usize::MAX));
        assert!(newer(&[5], Some((0, 1)), 0));
    }
}
