use super::{Accept, Close, Facts, How, Pin, Summary, close, met, pin_all, pin_one, summary};
use crate::gate::{Standing, State};
use alloc::vec::Vec;

const STANDINGS: [Standing; 7] = [
    Standing::Checked(State::Open),
    Standing::Checked(State::Satisfied),
    Standing::Checked(State::Stale),
    Standing::Checked(State::Failed),
    Standing::Checked(State::Unknown),
    Standing::Attested,
    Standing::Waived,
];

fn accepts() -> Vec<Accept> {
    let mut out = Vec::new();
    for bits in 0..8u8 {
        out.push(Accept {
            satisfied: bits & 1 != 0,
            attested: bits & 2 != 0,
            waived: bits & 4 != 0,
        });
    }
    out
}

/// Over every standing and policy: met exactly when the standing is a met
/// word the policy accepts; all three accepted is today's rule; a policy
/// only narrows; a rejected waiver never meets.
#[test]
fn met_is_the_accepted_met_words() {
    for x in STANDINGS {
        for a in accepts() {
            let want = (x == Standing::Checked(State::Satisfied) && a.satisfied)
                || (x == Standing::Attested && a.attested)
                || (x == Standing::Waived && a.waived);
            assert_eq!(met(x, &a), want, "{x:?}");
            assert!(!met(x, &a) || x.met());
            if !a.waived {
                assert!(!(x == Standing::Waived && met(x, &a)));
            }
        }
        let all = Accept {
            satisfied: true,
            attested: true,
            waived: true,
        };
        assert_eq!(met(x, &all), x.met(), "{x:?}");
    }
}

fn pin_name(p: &Pin) -> u8 {
    match p {
        Pin::Unpinned => 0,
        Pin::Same => 1,
        Pin::Moved => 2,
    }
}

#[test]
fn pin_one_compares_bytes() {
    let digests: [Option<Vec<u8>>; 4] = [
        None,
        Some(Vec::new()),
        Some(Vec::from([1u8])),
        Some(Vec::from([2u8])),
    ];
    for p in &digests {
        for n in &digests {
            let got = pin_name(&pin_one(p, n));
            let want = match (p, n) {
                (None, _) => 0,
                (Some(_), None) => 2,
                (Some(a), Some(b)) => {
                    if a == b {
                        1
                    } else {
                        2
                    }
                }
            };
            assert_eq!(got, want, "{p:?} {n:?}");
        }
    }
}

fn pin_of(n: u8) -> Pin {
    match n {
        0 => Pin::Unpinned,
        1 => Pin::Same,
        _ => Pin::Moved,
    }
}

/// Every list of up to three pins: moved when any moved; same only when
/// non-empty and all same; else unpinned.
#[test]
fn pin_all_over_every_list() {
    let mut lists: Vec<Vec<u8>> = Vec::from([Vec::new()]);
    for len in 1..=3u32 {
        for code in 0..3u32.pow(len) {
            let mut l = Vec::new();
            let mut c = code;
            for _ in 0..len {
                l.push(u8::try_from(c % 3).unwrap_or(0));
                c /= 3;
            }
            lists.push(l);
        }
    }
    for l in lists {
        let pins: Vec<Pin> = l.iter().map(|n| pin_of(*n)).collect();
        let got = pin_name(&pin_all(&pins));
        let want = if l.contains(&2) {
            2
        } else if !l.is_empty() && l.iter().all(|n| *n == 1) {
            1
        } else {
            0
        };
        assert_eq!(got, want, "{l:?}");
    }
}

fn how_of(n: u8) -> How {
    match n {
        0 => How::Passed,
        1 => How::Attested,
        _ => How::Waived,
    }
}

fn close_name(c: &Close) -> u8 {
    match c {
        Close::Open => 0,
        Close::Passed => 1,
        Close::Attested => 2,
        Close::Waived => 3,
        Close::Skipped => 4,
    }
}

/// Over every pair of flags and every list of up to three standings:
/// done wins; passed only when every gate passed; waived when any was.
#[test]
fn close_reads_the_done_record() {
    let mut lists: Vec<Vec<u8>> = Vec::from([Vec::new()]);
    for a in 0..3u8 {
        lists.push(Vec::from([a]));
        for b in 0..3u8 {
            lists.push(Vec::from([a, b]));
            for c in 0..3u8 {
                lists.push(Vec::from([a, b, c]));
            }
        }
    }
    for done in [false, true] {
        for skipped in [false, true] {
            for l in &lists {
                let met: Vec<How> = l.iter().map(|n| how_of(*n)).collect();
                let got = close_name(&close(done, skipped, &met));
                let want = if !done {
                    if skipped { 4 } else { 0 }
                } else if l.contains(&2) {
                    3
                } else if l.contains(&1) {
                    2
                } else {
                    1
                };
                assert_eq!(got, want, "{done} {skipped} {l:?}");
            }
        }
    }
}

fn summary_name(s: &Summary) -> u8 {
    match s {
        Summary::NoPlan => 0,
        Summary::Incomplete => 1,
        Summary::ClosedIncomplete => 2,
        Summary::PolicyMoved => 3,
        Summary::CompleteStale => 4,
        Summary::CompleteWithExceptions => 5,
        Summary::Complete => 6,
    }
}

fn close_of(n: u8) -> Close {
    match n {
        0 => Close::Open,
        1 => Close::Passed,
        2 => Close::Attested,
        3 => Close::Waived,
        _ => Close::Skipped,
    }
}

/// Every list of up to two steps under every fact: the table of the
/// policy spec, row by row, in order. Nothing with an exception, a moved
/// pin, a stale step or an open step is ever `Complete`.
#[test]
fn summary_is_the_table_in_order() {
    let mut lists: Vec<Vec<u8>> = Vec::from([Vec::new()]);
    for a in 0..5u8 {
        lists.push(Vec::from([a]));
        for b in 0..5u8 {
            lists.push(Vec::from([a, b]));
        }
    }
    for l in &lists {
        for bits in 0..16u8 {
            for p in 0..3u8 {
                let facts = Facts {
                    run_closed: bits & 1 != 0,
                    required_present: bits & 2 != 0,
                    pin: pin_of(p),
                    repinned: bits & 4 != 0,
                    stale: bits & 8 != 0,
                };
                let closes: Vec<Close> = l.iter().map(|n| close_of(*n)).collect();
                let got = summary_name(&summary(&closes, &facts));
                let want = if l.is_empty() {
                    0
                } else if l.contains(&0) || !facts.required_present {
                    if facts.run_closed { 2 } else { 1 }
                } else if p == 2 {
                    3
                } else if facts.stale {
                    4
                } else if facts.repinned || l.iter().any(|n| *n != 1) {
                    5
                } else {
                    6
                };
                assert_eq!(got, want, "{l:?} {bits} {p}");
            }
        }
    }
}
