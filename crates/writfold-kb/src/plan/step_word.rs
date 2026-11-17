//! One plan step as a template writes it, `id: gate gate …`, read with
//! the position of each fault inside the string: the step's ID, a word
//! that is not a gate ID, a second `use=`. [`super::template::step`]
//! keeps the plain form; [`check`] reports into a definition check.
use super::template::{RESERVED, Step};
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
    };
    for found in words(text, raw.len() + 1) {
        word(id, found, &mut step)?;
    }
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
