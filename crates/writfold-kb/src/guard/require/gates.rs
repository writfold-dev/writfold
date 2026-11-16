//! The `gates.met` requirement: every gate the run owes here, judged as
//! `gate status` judges it, through the kernel's
//! [`writfold_core::guard::require::gates_read`].
use super::{Facts, GateLine, Owed};
use writfold_core::guard::Fact;
use writfold_core::guard::require::{Looked, gates_read};

pub(super) fn undefined(id: &str) -> String {
    format!(
        "gate {id} [undefined]; next: define {}/{id}.toml in the KB root or the repository",
        crate::gate::def::DIR
    )
}

/// Every owed gate met, and no invalid receipt, statement or definition;
/// a run that owes no gate here does not meet it
/// ([`writfold_core::guard::gates_fact`]).
pub(super) fn met(facts: &Facts) -> Owed {
    let (lines, owed, invalid, deliveries) = match &facts.gates {
        None => {
            let fact = gates_read(Looked::No, 0, 0, 0);
            return Owed::of(fact, "the gates were not judged".into());
        }
        Some(Err(why)) => return Owed::of(gates_read(Looked::Failed, 0, 0, 0), why.clone()),
        Some(Ok(judged)) => judged,
    };
    let mut unmet: Vec<String> = Vec::new();
    let count = match owed {
        None => {
            unmet.extend(lines.iter().filter(|g| !g.met).map(|g| g.line.clone()));
            lines.len()
        }
        Some(ids) => {
            for id in ids {
                match lines.iter().find(|g| &g.id == id) {
                    Some(g) if g.met => {}
                    Some(g) => unmet.push(g.line.clone()),
                    None => unmet.push(undefined(id)),
                }
            }
            ids.len()
        }
    };
    let n = unmet.len();
    if count == 0 {
        unmet.push(super::super::empty::text(
            &facts.subject,
            owed.is_some(),
            deliveries,
        ));
    }
    if *invalid > 0 {
        unmet.push(format!(
            "invalid receipts or definitions: {invalid}; next: writfold kb gate status"
        ));
    }
    let relied = |g: &&GateLine| owed.as_ref().is_none_or(|ids| ids.contains(&g.id));
    match gates_read(Looked::Read, count, n, *invalid) {
        Fact::Met => Owed {
            waived: lines
                .iter()
                .filter(relied)
                .filter_map(|g| g.waived.clone())
                .collect(),
            ..Owed::met()
        },
        _ => Owed::of(Fact::Unmet, unmet.join("; ")),
    }
}
