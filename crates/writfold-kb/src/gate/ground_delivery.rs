//! `delivery:<candidate>` as a cover reference (#156): a worker's report
//! cited for a checklist item. Grounded exactly when the candidate is found
//! in this run, is ready, and passes `invocation.bound` now, re-checked on
//! read ([`writfold_core::ground::of_delivery`]). What the report says is
//! never judged. The reference's path is the invocation record the report
//! names, so binding ([`super::checklist::bound`]) reads which item it was
//! prepared `--for`. Finding and judging the candidate is tested IO.
use super::ground::{Context, Resolved};
use crate::deliver::{candidate, config::BOUND, recheck, status};
use writfold_core::gate::{Standing, State};
use writfold_core::ground::of_delivery;

pub fn resolve(cx: &Context<'_, '_>, reference: &str, id: &str) -> Resolved {
    let root = &cx.session.ws.root;
    let run = cx.run_dir.rsplit('/').next().unwrap_or_default();
    let cands = candidate::list(root, cx.run_dir, run);
    let c = cands.iter().find(|c| c.id == id);
    let (ready, bound, path) = match c {
        None => (false, false, None),
        Some(c) => {
            let filings = status::Filings::read(root, c, run);
            let judged = recheck::judged(cx.session, c, &filings).unwrap_or_default();
            let bound = (judged.iter()).any(|j| {
                j.obligation.name == BOUND && j.standing == Standing::Checked(State::Satisfied)
            });
            let invocation = (c.manifest.as_ref().ok())
                .and_then(|m| m.invocation.as_deref())
                .map(|i| crate::agents::invocation::path(cx.run_dir, i));
            (status::ready(&judged), bound, invocation)
        }
    };
    let standing = match (c.is_some(), ready, bound) {
        (false, _, _) => "no such candidate",
        (true, _, false) => "unbound",
        (true, false, true) => "not ready",
        (true, true, true) => "ready",
    };
    Resolved {
        reference: reference.to_owned(),
        grounding: of_delivery(c.is_some(), ready, bound),
        standing: standing.to_owned(),
        path,
        recorded: false,
    }
}
