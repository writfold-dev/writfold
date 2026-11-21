//! Recorded verdicts for pinned store citations (U3, #4). `kb stale
//! --record` files what this machine judged under the pin into a check
//! receipt of the kind `store@<name>` (`.writfold/checks/store@<name>/`,
//! the same layout as a checker's, keyed by the token's text, so by its
//! `rev=` too). A machine without the store's clone, or without the cited
//! commit, then reads the newest such verdict instead of `unverifiable`,
//! when the store declares `recorded_max_age` and the receipt is within it.
//! The decisions are proved ([`writfold_core::store::recorded`],
//! [`writfold_core::store::use_recorded`],
//! [`writfold_core::store::pin_recordable`]); reading the files and the
//! clock is tested here.
use super::checker::receipt::{self, DIR, Entry, Receipt};
use super::{Checked, Cite, Recorded, Verdict};
use crate::store::{Freshness, Store, Stores};
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::store::{self as kernel, Basis};
use writfold_core::verdict::{self as shared, Pick, Said, Statement};

/// The checker name a pinned receipt carries.
pub const CHECKER: &str = "writfold-pin";

/// Whether `cite` is judged under the pin of `store`.
fn on_pin(store: &Store, cite: &Cite) -> bool {
    cite.kind.as_deref() != Some("gate")
        && matches!(
            kernel::basis(
                true,
                store.freshness == Freshness::Pinned,
                cite.rev.is_some()
            ),
            Basis::Pin
        )
}

/// `local`, or, when it is `unverifiable` and a usable receipt of `store`
/// holds a verdict for the token, that verdict marked as recorded.
pub fn stand_in(root: &Path, store: &Store, cite: &Cite, local: Checked, now: i64) -> Checked {
    let max = store.recorded_seconds();
    if !kernel::use_recorded(&super::kernel(&local.verdict), max.is_some()) || !on_pin(store, cite)
    {
        return local;
    }
    let (found, _) = receipt::read_kind(root, &store.receipt_kind(), CHECKER);
    let key = receipt::digest(&cite.raw);
    let hits: Vec<(&Receipt, &Entry, i64)> = (found.iter())
        .flat_map(|r| r.verdicts.iter().map(move |e| (r, e)))
        .filter(|(_, e)| e.token == key)
        .filter_map(|(r, e)| Some((r, e, shared::recorded_age(now, r.at)?)))
        .collect();
    let statements: Vec<Statement> = (hits.iter())
        .map(|(_, e, age)| Statement {
            said: e.said,
            age: *age,
            recorded: true,
        })
        .collect();
    let Some(Pick::Recorded(i)) = shared::newest(None, &statements, max) else {
        return local;
    };
    let Some((r, e, age)) = hits.get(i) else {
        return local;
    };
    let within = max.is_some_and(|m| *age <= m);
    let allowed = kernel::recorded(&Basis::Pin, max.is_some(), within);
    if !kernel::use_recorded(&super::kernel(&local.verdict), allowed) {
        return local;
    }
    let verdict = match e.said {
        Said::Fresh => Verdict::Fresh,
        _ => Verdict::Gone {
            detail: e.detail.clone(),
        },
    };
    let rev = cite.rev.as_deref().unwrap_or_default();
    Checked {
        verdict,
        head: format!("{}:pin", rev.get(..12).unwrap_or(rev)),
        recorded: Some(Recorded {
            checked_at: r.checked_at.clone(),
            path: r.path.clone(),
            checker: CHECKER.into(),
            version: None,
        }),
    }
}

/// The receipts `kb stale --record` writes for pinned store citations this
/// machine judged itself, one per store that declares `recorded_max_age`.
pub fn plan(stores: &Stores, cites: &[Cite], checked: &[Checked], now: i64) -> Vec<Receipt> {
    let mut by_store: BTreeMap<&str, BTreeMap<String, Entry>> = BTreeMap::new();
    for (cite, judged) in cites.iter().zip(checked) {
        let Some(store) = cite.repo.as_deref().and_then(|r| stores.get(r)) else {
            continue;
        };
        let mine = judged.recorded.is_none() && judged.head.ends_with(":pin");
        let verdict = super::kernel(&judged.verdict);
        if store.recorded_max_age.is_none()
            || !on_pin(store, cite)
            || !mine
            || !kernel::pin_recordable(&verdict)
        {
            continue;
        }
        let (said, detail) = match &judged.verdict {
            Verdict::Fresh => (Said::Fresh, None),
            Verdict::Gone { detail } => (Said::Gone, detail.clone()),
            _ => continue,
        };
        let token = receipt::digest(&cite.raw);
        by_store.entry(&store.name).or_default().insert(
            token.clone(),
            Entry {
                token,
                said,
                at: None,
                detail,
                moved: None,
            },
        );
    }
    by_store
        .into_iter()
        .filter_map(|(name, entries)| {
            let store = stores.get(name)?;
            let kind = store.receipt_kind();
            let file = crate::write::unique(now, &kind);
            Some(Receipt {
                path: format!("{DIR}/{kind}/{file}.json"),
                kind,
                checker: CHECKER.into(),
                version: None,
                protocol: None,
                checked_at: crate::time::stamp(now),
                at: now,
                verdicts: entries.into_values().collect(),
            })
        })
        .collect()
}
