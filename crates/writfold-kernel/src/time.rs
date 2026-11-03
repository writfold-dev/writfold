//! Mathematical freshness, with inclusive expiry and no machine-integer narrowing.
use std::ops::Add;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use writfold_domain::TtlView;
use writfold_domain::{Freshness, Integer, Ttl};

verus! {

pub open spec fn fresh_spec(ttl: TtlView, observed_at: int, now: int) -> bool {
    observed_at <= now && match ttl {
        TtlView::ExpireAfter(secs) => now <= observed_at + secs,
        TtlView::NoExpiry => true,
        TtlView::Missing => false,
    }
}

pub open spec fn assessment_spec(ttl: TtlView, observed_at: int, now: int) -> Freshness {
    if observed_at > now || matches!(ttl, TtlView::Missing) {
        Freshness::Unknown
    } else if fresh_spec(ttl, observed_at, now) {
        Freshness::Fresh
    } else {
        Freshness::Stale
    }
}

pub fn fresh_at(ttl: &Ttl, observed_at: &Integer, now: &Integer) -> (result: bool)
    ensures result == fresh_spec(ttl@, observed_at@, now@),
{
    if observed_at > now {
        return false;
    }
    match ttl {
        Ttl::ExpireAfter(secs) => now <= &observed_at.add(secs),
        Ttl::NoExpiry => true,
        Ttl::Missing => false,
    }
}

pub fn assess_fresh(ttl: &Ttl, observed_at: &Integer, now: &Integer) -> (result: Freshness)
    ensures result == assessment_spec(ttl@, observed_at@, now@),
{
    if observed_at > now {
        return Freshness::Unknown;
    }
    match ttl {
        Ttl::Missing => Freshness::Unknown,
        Ttl::NoExpiry => Freshness::Fresh,
        Ttl::ExpireAfter(secs) if now <= &observed_at.add(secs) => Freshness::Fresh,
        Ttl::ExpireAfter(_) => Freshness::Stale,
    }
}

} // verus!
