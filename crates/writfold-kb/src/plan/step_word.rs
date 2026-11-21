//! One plan step as a template writes it, `id: gate gate …`, read with
//! the position of each fault inside the string: the step's ID, a word
//! that is not a gate ID, a second `use=`. [`super::template::step`]
//! keeps the plain form; [`check`] reports into a definition check.
use super::template::{RESERVED, Repeat, Step};
use crate::defs::{Check, Spanned, Text};
use crate::write::is_slug;

/// A fault in one step string: its code, message, label, and the byte
/// range inside the string (`None`: the whole string).
pub struct Fault {
    pub code: &'static str,
    pub why: String,
    pub label: &'static str,
    pub at: Option<(usize, usize)>,
    pub help: Option<String>,
}

fn fault(
    code: &'static str,
    why: String,
    label: &'static str,
    at: Option<(usize, usize)>,
) -> Fault {
    Fault {
        code,
        why,
        label,
        at,
        help: None,
    }
}

/// The words after the colon with their byte offsets in `text`.
pub fn words(text: &str, from: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text[from..].char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                out.push((from + s, &text[from + s..from + i]));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((from + s, &text[from + s..]));
    }
    out
}

/// A step without a colon; help when every word is a slug.
fn no_colon(text: &str) -> Fault {
    let mut f = fault(
        "value.form",
        "a step is written \"id: gate gate …\"".into(),
        "no `:` after the step's ID",
        None,
    );
    let all = words(text, 0);
    if all.len() >= 2 && all.iter().all(|(_, w)| is_slug(w)) {
        let rest: Vec<&str> = all[1..].iter().map(|(_, w)| *w).collect();
        f.help = Some(format!(
            "did you mean \"{}: {}\"?",
            all[0].1,
            rest.join(" ")
        ));
    }
    f
}

/// One word after the colon, against what the step already owes.
fn word(id: &str, (at, word): (usize, &str), step: &mut Step) -> Result<(), Fault> {
    let here = Some((at, word.len()));
    if let Some(name) = word.strip_prefix("use=") {
        if !is_slug(name) || name == RESERVED {
            let why = format!("step {id}: {word} does not name a template");
            return Err(fault("value.form", why, "not a template name", here));
        }
        if step.include.replace(name.to_owned()).is_some() {
            let why = format!("step {id} includes more than one template");
            return Err(fault("value.twice", why, "a second include", here));
        }
        return Ok(());
    }
    if word.starts_with(crate::deliver::step::PREFIX)
        || word.starts_with(crate::deliver::step::POSTED)
    {
        if let Err(why) = crate::deliver::word::check(id, word, &step.gates) {
            let code = if why.ends_with("twice") {
                "value.twice"
            } else {
                "value.form"
            };
            return Err(fault(code, why, "not a delivery mode", here));
        }
    } else if !is_slug(word) {
        let why = format!("step {id}: gate {word:?} is not a gate ID");
        return Err(fault("value.form", why, "not a gate ID", here));
    } else if step.gates.iter().any(|g| g == word) {
        let why = format!("step {id} names gate {word} twice");
        return Err(fault("value.twice", why, "named again", here));
    }
    step.gates.push(word.to_owned());
    Ok(())
}

/// Read one `id: gate gate …` string.
pub fn read(text: &str) -> Result<Step, Fault> {
    let Some((raw, _)) = text.split_once(':') else {
        return Err(no_colon(text));
    };
    let id = raw.trim();
    let id_at = raw.len() - raw.trim_start().len();
    if !is_slug(id) {
        let at = (!id.is_empty()).then_some((id_at, id.len()));
        let why = format!("step ID {id:?} is not a slug");
        return Err(fault("value.form", why, "not a slug", at));
    }
    let mut step = Step {
        id: id.to_owned(),
        gates: Vec::new(),
        include: None,
        repeat: None,
    };
    let mut looping = Looping::default();
    for found in words(text, raw.len() + 1) {
        if !looping.take(id, found)? {
            word(id, found, &mut step)?;
        }
    }
    looping.apply(id, &mut step)?;
    if let Some(why) = crate::deliver::word::unpaired(id, &step.gates) {
        let at = step
            .gates
            .iter()
            .find(|g| crate::deliver::word::lone(g, &step.gates))
            .and_then(|g| locate(text, g));
        return Err(fault(
            "ref.undefined",
            why,
            "no deliver: word for this mode",
            at,
        ));
    }
    Ok(step)
}

/// Read one step of a template, reporting a fault at its position.
pub fn check(c: &mut Check<'_>, item: &Spanned<Text>) -> Option<Step> {
    match read(item.str()) {
        Ok(step) => Some(step),
        Err(f) => {
            let span = f.at.map_or(item.span, |(at, len)| item.sub(at, len));
            let d = c.error(f.code, Some(span), f.why);
            d.label(f.label);
            d.help = f.help;
            None
        }
    }
}

/// Where `word` is written in a step string, as (offset, length).
pub fn locate(text: &str, word: &str) -> Option<(usize, usize)> {
    let colon = text.find(':')?;
    words(text, colon + 1)
        .into_iter()
        .find(|(_, w)| *w == word)
        .map(|(at, w)| (at, w.len()))
}

/// A loop's words as written (`loop=`, `max=`, `min=`, #76), each with its
/// position.
#[derive(Default)]
struct Looping<'a> {
    body: Option<(usize, &'a str)>,
    max: Option<(usize, &'a str)>,
    min: Option<(usize, &'a str)>,
}

impl<'a> Looping<'a> {
    /// Keep a loop word; `false` for any other word.
    fn take(&mut self, id: &str, (at, word): (usize, &'a str)) -> Result<bool, Fault> {
        let slot = match () {
            () if word.starts_with("loop=") => &mut self.body,
            () if word.starts_with("max=") => &mut self.max,
            () if word.starts_with("min=") => &mut self.min,
            () => return Ok(false),
        };
        if slot.replace((at, word)).is_some() {
            let why = format!(
                "step {id} writes {} twice",
                &word[..word.find('=').unwrap_or(0) + 1]
            );
            return Err(fault(
                "value.twice",
                why,
                "written again",
                Some((at, word.len())),
            ));
        }
        Ok(true)
    }

    /// Check the loop words together and set the step's body and bounds.
    fn apply(self, id: &str, step: &mut Step) -> Result<(), Fault> {
        let at = |w: (usize, &str)| Some((w.0, w.1.len()));
        let Some(body) = self.body else {
            if let Some(w) = self.max.or(self.min) {
                let why = format!(
                    "step {id}: {} applies only to a loop (loop=<template>)",
                    w.1
                );
                return Err(fault("key.not-here", why, "not a loop", at(w)));
            }
            return Ok(());
        };
        let name = &body.1["loop=".len()..];
        if !is_slug(name) || name == RESERVED {
            let why = format!("step {id}: {} does not name a template", body.1);
            return Err(fault("value.form", why, "not a template name", at(body)));
        }
        if step.include.is_some() || step.gates.iter().any(|g| g.contains(':')) {
            let why = format!("step {id}: a loop cannot also include a template or owe a delivery");
            return Err(fault("value.conflict", why, "loop here", at(body)));
        }
        let Some(max) = self.max else {
            let why =
                format!("step {id}: a loop needs max=N, the most rounds it may take (1 to 32)");
            return Err(fault("key.missing", why, "no max=", at(body)));
        };
        let number = |w: (usize, &str)| w.1[4..].parse::<usize>().ok();
        let bounds = writfold_core::plan::round::Bounds {
            max: number(max).unwrap_or(0),
            min: self.min.map_or(Some(1), number).unwrap_or(0),
        };
        if !writfold_core::plan::round::bounds_ok(&bounds) {
            let why = format!("step {id}: a loop takes 1 <= min <= max <= 32");
            let w = self
                .min
                .filter(|_| (1..=32).contains(&bounds.max))
                .unwrap_or(max);
            return Err(fault("key.range", why, "out of range", at(w)));
        }
        if !writfold_core::plan::round::has_exit(&bounds, step.gates.len()) {
            let why = format!(
                "step {id}: a loop with no gate and min=1 has no exit of its own; name the gates that end it, or write use={name}"
            );
            return Err(fault("value.no-exit", why, "no exit", at(body)));
        }
        step.include = Some(name.to_owned());
        step.repeat = Some(Repeat {
            min: bounds.min,
            max: bounds.max,
        });
        Ok(())
    }
}
