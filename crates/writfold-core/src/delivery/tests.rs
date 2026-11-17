use super::{
    Body, Owed, Place, Seen, Unread, deciding, fact, first_unmet, guard, posted, ready, rechecked,
    refused, standing,
};
use crate::gate::{Standing, State};
use crate::plan::fact_met;
use crate::policy::{Accept, met};
use alloc::vec::Vec;

const ALL: [Standing; 7] = [
    Standing::Checked(State::Open),
    Standing::Checked(State::Satisfied),
    Standing::Checked(State::Stale),
    Standing::Checked(State::Failed),
    Standing::Checked(State::Unknown),
    Standing::Attested,
    Standing::Waived,
];

fn accept(bits: u8) -> Accept {
    Accept {
        satisfied: bits & 1 != 0,
        attested: bits & 2 != 0,
        waived: bits & 4 != 0,
    }
}

/// Every (standing, accept bits) list of up to `max` elements.
fn lists(max: usize) -> Vec<Vec<(Standing, u8)>> {
    let mut out = Vec::from([Vec::new()]);
    let mut layer: Vec<Vec<(Standing, u8)>> = Vec::from([Vec::new()]);
    for _ in 0..max {
        let mut next = Vec::new();
        for list in &layer {
            for item in ALL {
                for bits in 0..8u8 {
                    let mut grown = list.clone();
                    grown.push((item, bits));
                    next.push(grown);
                }
            }
        }
        out.extend(next.iter().cloned());
        layer = next;
    }
    out
}

fn owed(list: &[(Standing, u8)]) -> Vec<Owed> {
    list.iter()
        .map(|&(standing, bits)| Owed {
            standing,
            accept: accept(bits),
        })
        .collect()
}

#[test]
fn standing_ready_and_fact_follow_the_policy() {
    for list in lists(2) {
        let os = owed(&list);
        let is_met = |&(s, bits): &(Standing, u8)| met(s, &accept(bits));
        let first = list.iter().position(|o| !is_met(o));
        assert_eq!(first_unmet(&os), first, "{list:?}");
        let want_ready = !list.is_empty() && first.is_none();
        assert_eq!(ready(&os), want_ready, "{list:?}");
        assert_eq!(fact_met(&fact(&os)), want_ready, "{list:?}");
        let want = match first {
            Some(i) => list[i].0,
            None if list.is_empty() => Standing::Checked(State::Open),
            None if list.iter().any(|o| o.0 == Standing::Waived) => Standing::Waived,
            None if list.iter().any(|o| o.0 == Standing::Attested) => Standing::Attested,
            None => Standing::Checked(State::Satisfied),
        };
        assert_eq!(standing(&os), want, "{list:?}");
        assert_eq!(
            refused(&os),
            first.is_some_and(|i| list[i].0.met()),
            "{list:?}"
        );
        if want_ready {
            assert!(standing(&os).met(), "{list:?}");
        }
        // every word accepted: today's rule on the standings alone
        let all: Vec<(Standing, u8)> = list.iter().map(|o| (o.0, 7)).collect();
        let all = owed(&all);
        let plain: Vec<Standing> = list.iter().map(|o| o.0).collect();
        assert_eq!(
            ready(&all),
            !plain.is_empty() && plain.iter().all(|s| s.met())
        );
        if want_ready {
            assert!(ready(&all), "{list:?}");
        }
    }
}

/// W1: a waiver the policy rejects is never ready, whatever else stands.
#[test]
fn a_rejected_waiver_is_never_ready() {
    let os = [
        Owed {
            standing: Standing::Checked(State::Satisfied),
            accept: accept(7),
        },
        Owed {
            standing: Standing::Waived,
            accept: accept(1),
        },
    ];
    assert!(!ready(&os));
    assert!(refused(&os));
    assert_eq!(standing(&os), Standing::Waived);
}

#[test]
fn deciding_passes_over_withdrawn_and_stops_at_bound() {
    const P: [Place; 3] = [Place::Open, Place::Bound, Place::Withdrawn];
    for len in 0..=4u32 {
        for code in 0..3u32.pow(len) {
            let places: Vec<Place> = (0..len)
                .map(|i| P[(code / 3u32.pow(i) % 3) as usize])
                .collect();
            let newest = (0..places.len())
                .rev()
                .find(|&i| places[i] != Place::Withdrawn);
            let want = newest.filter(|&i| places[i] == Place::Open);
            assert_eq!(deciding(&places), want, "{places:?}");
        }
    }
}

#[test]
fn posted_and_rechecked() {
    assert_eq!(posted(true), Standing::Attested);
    assert_eq!(posted(false), Standing::Checked(State::Open));
    assert_eq!(rechecked(State::Satisfied, 1), State::Stale);
    assert_eq!(rechecked(State::Satisfied, 0), State::Satisfied);
    assert_eq!(rechecked(State::Failed, 3), State::Failed);
}

fn seen(bits: u8) -> Seen {
    Seen {
        same: bits & 1 != 0,
        ready: bits & 2 != 0,
        live: bits & 4 != 0,
        routed: bits & 8 != 0,
        spent: bits & 16 != 0,
    }
}

#[test]
fn guard_is_any_postable_or_all_live_ready() {
    let mut lists: Vec<Vec<Seen>> = Vec::from([Vec::new()]);
    for a in 0..32u8 {
        lists.push(Vec::from([seen(a)]));
        for b in [0u8, 7, 15, 6, 4, 2, 31, 23] {
            lists.push(Vec::from([seen(a), seen(b)]));
        }
    }
    for cs in &lists {
        let any = cs
            .iter()
            .any(|c| c.same && c.ready && c.live && c.routed && !c.spent);
        let all = cs.iter().any(|c| c.live) && cs.iter().all(|c| !c.live || c.ready);
        assert_eq!(guard(Body::Known, Unread::AllReady, cs), any, "{cs:?}");
        assert_eq!(guard(Body::Known, Unread::Deny, cs), any, "{cs:?}");
        assert_eq!(guard(Body::Unknown, Unread::AllReady, cs), all, "{cs:?}");
        assert!(!guard(Body::Unknown, Unread::Deny, cs), "{cs:?}");
    }
}
