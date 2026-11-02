//! Mathematical freshness, with inclusive expiry and no machine-integer narrowing.
use writfold_domain::{Freshness, Integer, Ttl};

pub fn fresh_at(ttl: &Ttl, observed_at: &Integer, now: &Integer) -> bool {
    if observed_at > now {
        return false;
    }
    match ttl {
        Ttl::ExpireAfter(secs) => now <= &(observed_at + secs),
        Ttl::NoExpiry => true,
        Ttl::Missing => false,
    }
}

pub fn assess_fresh(ttl: &Ttl, observed_at: &Integer, now: &Integer) -> Freshness {
    if observed_at > now {
        return Freshness::Unknown;
    }
    match ttl {
        Ttl::Missing => Freshness::Unknown,
        Ttl::NoExpiry => Freshness::Fresh,
        Ttl::ExpireAfter(secs) if now <= &(observed_at + secs) => Freshness::Fresh,
        Ttl::ExpireAfter(_) => Freshness::Stale,
    }
}
