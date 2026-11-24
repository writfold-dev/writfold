//! `prose.rules` (#127, RFC 2026-11-25-prose-rules-and-advice): a delivery
//! check against a rule set the owner keeps as data,
//! `.writfold/deliveries/prose-<name>.toml`:
//!
//! ```toml
//! schema = "writfold-prose-rules/1"
//! name = "plain"
//! advise = ["filler: \\b(just|simply)\\b"]
//! block = ["placeholder: \\bTODO\\b"]
//! ```
//!
//! Entries are the deny list's `"name: pattern"` rules (ASCII, case
//! insensitive). Rules match prose lines only: lines in code fences and
//! indented code blocks are skipped, and inline code spans and URLs are
//! blanked first. A blocking match fails the check; advice is recorded and
//! shown and never decides; a set with no rules is `unknown`. The decision
//! is [`writfold_core::prose::verdict`]; reading and matching are tested.
use super::checks::{Evaluated, Finding};
use super::config::Rule;
use super::config_check::{list, named};
use crate::defs::Check;
use crate::gate::def_check::schema;
use writfold_core::diag::Report;
use writfold_core::prose::{Weight, verdict};

pub const SCHEMA: &str = "writfold-prose-rules/1";

/// One rule set.
#[derive(Clone, Debug)]
pub struct Rules {
    pub name: String,
    pub path: String,
    /// Every rule with its weight, advice first, each list in file order.
    pub rules: Vec<(Weight, Rule)>,
}

impl Rules {
    /// What the rules mean, for the obligation digest: the set's name and
    /// each rule's tier, name and pattern.
    pub fn material(&self) -> String {
        let lines: Vec<String> = (self.rules.iter())
            .map(|(w, r)| format!("{}\t{}\t{}", tier(*w), r.name, r.pattern))
            .collect();
        format!("set={}\n{}", self.name, lines.join("\n"))
    }
}

fn tier(w: Weight) -> &'static str {
    match w {
        Weight::Advice => "advise",
        Weight::Blocking => "block",
    }
}

/// `prose-<name>.toml`.
pub fn check(name: &str, path: &str, text: &str) -> Result<Rules, Report> {
    let mut c = Check::read(path, text, "a prose rule set");
    let keys = ["schema", "name", "advise", "block"];
    c.unknown_keys(&keys, &keys, "not a prose rule set key");
    schema(&mut c, SCHEMA);
    let ok = named(&mut c, "name", name, &format!("prose-{name}.toml"));
    let mut rules = Some(Vec::new());
    let mut seen: Vec<String> = Vec::new();
    for (weight, key) in [(Weight::Advice, "advise"), (Weight::Blocking, "block")] {
        for entry in list(&mut c, key).unwrap_or_default() {
            let read = super::config_check::rule(&mut c, &entry);
            if let Some(r) = &read
                && seen.contains(&r.name)
            {
                let why = format!("rule {} is named twice", r.name);
                c.error("value.twice", Some(entry.span), why)
                    .label("named again");
                rules = None;
                continue;
            }
            match (read, rules.as_mut()) {
                (Some(r), Some(all)) => {
                    seen.push(r.name.clone());
                    all.push((weight, r));
                }
                _ => rules = None,
            }
        }
    }
    let set = rules.filter(|_| ok).map(|rules| Rules {
        name: name.to_owned(),
        path: path.to_owned(),
        rules,
    });
    c.finish(set)
}

/// The prose lines of a payload, numbered from 1, with inline code spans
/// and URLs blanked. Code fences and indented code blocks are skipped.
pub fn prose_lines(text: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut fenced: Option<&str> = None;
    let mut previous_blank = true;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let opener = ["```", "~~~"].into_iter().find(|f| trimmed.starts_with(f));
        match (fenced, opener) {
            (Some(open), Some(f)) if f == open => fenced = None,
            (Some(_), _) => {}
            (None, Some(f)) => fenced = Some(f),
            (None, None) => {
                let indented = line.starts_with("    ") || line.starts_with('\t');
                if !(indented && previous_blank) {
                    let n = u32::try_from(index + 1).unwrap_or(u32::MAX);
                    out.push((n, blank(line)));
                }
            }
        }
        previous_blank = line.trim().is_empty();
    }
    out
}

/// A line with each inline code span and each URL replaced by spaces, so
/// columns still point at the original text.
fn blank(line: &str) -> String {
    let mut out: Vec<char> = line.chars().collect();
    let mut code = false;
    for ch in &mut out {
        if *ch == '`' {
            code = !code;
            *ch = ' ';
        } else if code {
            *ch = ' ';
        }
    }
    let text: String = out.into_iter().collect();
    text.split(' ')
        .map(
            |w| match w.starts_with("http://") || w.starts_with("https://") {
                true => " ".repeat(w.len()),
                false => w.to_owned(),
            },
        )
        .collect::<Vec<_>>()
        .join(" ")
}

/// Evaluate a payload against a rule set: the first match of each rule on
/// each prose line.
pub fn evaluate(text: &str, rules: &Rules) -> Evaluated {
    let mut findings = Vec::new();
    let mut advice = Vec::new();
    let mut weights = Vec::new();
    for (line, prose) in prose_lines(text) {
        for (weight, rule) in &rules.rules {
            let Some(found) = rule.regex.find(prose.as_bytes()) else {
                continue;
            };
            let f = Finding {
                line,
                rule: rule.name.clone(),
                detail: format!("column {}", found.start() + 1),
            };
            weights.push(*weight);
            match weight {
                Weight::Advice => advice.push(f),
                Weight::Blocking => findings.push(f),
            }
        }
    }
    let outcome = verdict(rules.rules.len(), &weights);
    let note = match rules.rules.len() {
        0 => format!("{} has no rules", rules.path),
        n => format!("{n} rules in {}", rules.path),
    };
    Evaluated {
        outcome,
        findings,
        advice,
        note: Some(note),
    }
}

#[cfg(test)]
mod tests {
    use super::{check, prose_lines};

    #[test]
    fn code_and_urls_are_not_prose() {
        let text = "Just do it.\n\n```\njust code\n```\n\n    just indented\n\nSee `just` at https://x.invalid/just here.\n~~~\njust\n~~~\nend";
        let lines: Vec<u32> = prose_lines(text).iter().map(|(n, _)| *n).collect();
        assert_eq!(lines, [1, 2, 6, 8, 9, 13]);
        let seen = &prose_lines(text)[4].1;
        assert!(!seen.contains("just"), "{seen}");
        assert!(seen.contains("here."));
    }

    #[test]
    fn a_rule_set_is_checked() {
        let good = "schema = \"writfold-prose-rules/1\"\nname = \"plain\"\nadvise = [\"filler: \\\\bjust\\\\b\"]\nblock = [\"placeholder: \\\\bTODO\\\\b\"]\n";
        let rules = check("plain", "p.toml", good).map_err(|r| format!("{r:?}"));
        assert_eq!(rules.map(|r| r.rules.len()), Ok(2));
        for bad in [
            good.replace("name = \"plain\"", "name = \"other\""),
            good.replace("placeholder:", "filler:"),
            good.replace("\\\\bTODO\\\\b", "(unclosed"),
            format!("{good}extra = 1\n"),
        ] {
            assert!(check("plain", "p.toml", &bad).is_err(), "{bad}");
        }
    }
}
