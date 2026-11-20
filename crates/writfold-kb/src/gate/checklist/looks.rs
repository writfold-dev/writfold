//! The `none-found` outcome (policy spec 2.3): a row that says the
//! recorded looks found nothing. Each reference is an observation of this
//! run (`runs/<run>/observed/…`) whose status is `no-data` or `not-found`;
//! at least one is cited, and the reason says what was searched and where
//! the search stopped. A look that did not answer (`timeout`, `denied`,
//! `error`) or answered (`ok`, `partial`) never grounds it. The row
//! stands while every cited look still reads with the status it was
//! recorded with. It is never a claim that nothing exists.
use super::cover::{Cited, Cover as Row};
use crate::error::{KbError, Result};
use crate::gate::def::Definition;
use crate::gate::ground::{self, Context};
use writfold_core::checklist::Outcome;

/// The statuses of a look that found nothing.
pub const NOTHING: [&str; 2] = ["no-data", "not-found"];

fn of_this_run(cx: &Context<'_, '_>, path: Option<&str>) -> bool {
    path.is_some_and(|p| {
        p.strip_prefix(&format!("{}/observed/", cx.run_dir))
            .is_some_and(|name| !name.contains('/'))
    })
}

/// The cited looks of a new `none-found` row, refusing the first that is
/// not a look of this run that found nothing.
pub fn recorded(cx: &Context<'_, '_>, references: &[String]) -> Result<Vec<Cited>> {
    let mut out = Vec::new();
    for reference in references {
        let r = ground::resolve(cx, reference)?;
        let what = if !of_this_run(cx, r.path.as_deref()) {
            Some("not an observation of this run".to_owned())
        } else if !NOTHING.contains(&r.standing.as_str()) {
            Some(format!("a look with status {}", r.standing))
        } else {
            None
        };
        if let Some(what) = what {
            return Err(KbError::Refused(format!(
                "none-found needs a look that found nothing: {reference} is {what}; record one with writfold kb observe --status no-data"
            )));
        }
        out.push(Cited {
            reference: reference.clone(),
            grounding: r.grounding,
            standing: r.standing,
            path: r.path,
        });
    }
    Ok(out)
}

/// The first cited look of a `none-found` row that no longer reads as it
/// was recorded: `(reference, then, now)`.
pub fn moved(cx: &Context<'_, '_>, row: &Row) -> Result<Option<(String, String, String)>> {
    for e in &row.evidence {
        let r = ground::resolve(cx, &e.reference)?;
        let same = of_this_run(cx, r.path.as_deref())
            && NOTHING.contains(&r.standing.as_str())
            && r.standing == e.standing;
        if !same {
            return Ok(Some((e.reference.clone(), e.standing.clone(), r.standing)));
        }
    }
    Ok(None)
}

/// Refuse an outcome the checklist's policy does not take for `item`.
pub fn allowed(d: &Definition, item: &super::spec::Item, outcome: Outcome) -> Result<()> {
    let shown = |key: &str, listed: &Option<Vec<String>>| {
        let words = serde_json::to_string(listed.as_deref().unwrap_or_default())
            .unwrap_or_default()
            .replace(',', ", ");
        let at = crate::gate::policy::key_line(d, key);
        format!("{key} = {words} @{}:{at}", d.path)
    };
    let Some(spec) = d.checklist.as_ref() else {
        return Ok(());
    };
    let refused = match outcome {
        Outcome::NotApplicable if !item.na => {
            let or = match item.none {
                true => ", or none-found citing a look",
                false => "",
            };
            format!(
                "{}.{} cannot be covered not-applicable ({}); cover it supported with evidence{or}",
                d.id,
                item.id,
                shown("not_applicable", &spec.lists.na)
            )
        }
        Outcome::NoneFound if !item.none => format!(
            "{}.{} cannot be covered none-found ({})",
            d.id,
            item.id,
            shown("none_found", &spec.lists.none)
        ),
        _ => return Ok(()),
    };
    Err(KbError::Refused(refused))
}
