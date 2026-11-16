//! The checks of a plan template ([`super::template`]): every fault of
//! the file, each at its position; a step's own faults are found by
//! [`super::step_word`].
use super::template::{KEYS, MAX_STEPS, RESERVED, SCHEMA, Step};
use crate::defs::{Check, Spanned, Text};
use crate::gate::def_check::{description, schema};
use crate::write::is_slug;
use writfold_core::diag::Report;

type Parts = (String, Option<String>, Vec<Step>);

/// Every fault of one template, or its name, description and steps.
pub fn check(text: &str, stem: Option<&str>, shown: &str) -> std::result::Result<Parts, Report> {
    let mut c = Check::read(shown, text, "a plan template");
    c.unknown_keys(&KEYS, &KEYS, "not a plan template key");
    schema(&mut c, SCHEMA);
    let name = c.need_text("name", &format!("name = \"{}\"", stem.unwrap_or("change")));
    let name = name.filter(|n| match stem {
        Some(stem) => c.named("name", n, stem),
        None => is_slug(n.str()),
    });
    let name = name.filter(|n| {
        let reserved = n.str() == RESERVED;
        if reserved {
            let why = format!("`{RESERVED}` names the required-steps file, not a template");
            c.error("value.form", Some(n.span), why).label("reserved");
        }
        !reserved
    });
    let description = description(&mut c);
    let items = c.need_list("steps", "a plan template", "steps = [\"implement: unit\"]");
    let steps = items.and_then(|items| steps(&mut c, &items, name.as_ref().map(|n| n.str())));
    let parts = (|| Some((name?.value.text, description?, steps?)))();
    c.finish(parts)
}

fn steps(c: &mut Check<'_>, items: &[Spanned<Text>], name: Option<&str>) -> Option<Vec<Step>> {
    if items.len() > MAX_STEPS {
        let why = format!(
            "`steps` holds at most {MAX_STEPS} steps, and this has {}",
            items.len()
        );
        let span = items.first().map(|i| i.span);
        c.error("key.range", span, why).label("too many steps");
        return None;
    }
    let mut steps: Vec<Step> = Vec::new();
    let mut ok = true;
    for item in items {
        let Some(parsed) = super::step_word::check(c, item) else {
            ok = false;
            continue;
        };
        if steps.iter().any(|s| s.id == parsed.id) {
            let why = format!("step {} is listed twice", parsed.id);
            c.error("value.twice", Some(item.span), why)
                .label("listed again");
            ok = false;
        } else if name.is_some() && parsed.include.as_deref() == name {
            let why = format!(
                "step {} includes {} itself",
                parsed.id,
                name.unwrap_or_default()
            );
            c.error("ref.cycle", Some(item.span), why)
                .label("includes its own template");
            ok = false;
        }
        steps.push(parsed);
    }
    ok.then_some(steps)
}
