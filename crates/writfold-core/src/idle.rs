//! Whether a run is idle: how long ago its last recorded write was, from
//! stamps read out of tracked text (never file modification times), so
//! every clone answers the same. The age filters and is shown; it decides
//! no state.

/// The newest stamp, and its index.
pub fn newest(stamps: &[i64]) -> Option<(usize, i64)> {
    let mut best: Option<(usize, i64)> = None;
    let mut i = 0;
    while i < stamps.len() {
        let newer = match best {
            None => true,
            Some((_, at)) => stamps[i] > at,
        };
        if newer {
            best = Some((i, stamps[i]));
        }
        i += 1;
    }
    best
}

/// `Some((index of the newest stamp, seconds since it))` when the newest
/// stamp is more than `age` seconds before `now`; `None` when it is not,
/// or there is no stamp.
pub fn idle(now: i64, stamps: &[i64], age: i64) -> Option<(usize, i64)> {
    let (at, last) = newest(stamps)?;
    let since = now - last;
    (since > age).then_some((at, since))
}

#[cfg(test)]
mod tests {
    use super::{idle, newest};

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
}
