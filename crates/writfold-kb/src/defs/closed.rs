//! Faults about closed sets and names: unknown keys, keys another class
//! or event takes, values outside their set, and a name that must equal
//! the file name. Each offers a suggestion from the set at that position
//! ([`writfold_core::diag::suggest`]), or lists a small set.
use super::Check;
use super::toml::{Spanned, Text};
use writfold_core::diag::suggest::{did_you_mean, listed};
use writfold_core::diag::{Span, suggest};

impl Check<'_> {
    /// Report every key not in `allowed`; suggestions come from
    /// `candidates` minus the keys already in the file.
    pub fn unknown_keys(&mut self, allowed: &[&str], candidates: &[&str], label: &str) {
        let present: Vec<String> = self
            .doc
            .entries
            .iter()
            .map(|e| e.key.value.clone())
            .collect();
        let open: Vec<&str> = candidates
            .iter()
            .copied()
            .filter(|c| !present.iter().any(|p| p == c))
            .collect();
        for (key, span) in self
            .doc
            .entries
            .iter()
            .map(|e| (e.key.value.clone(), e.key.span))
            .collect::<Vec<_>>()
        {
            if allowed.contains(&key.as_str()) {
                continue;
            }
            let found = suggest(&key, &open);
            let what = self.what;
            let d = self.sink.error(
                "key.unknown",
                Some(span),
                format!("unknown key `{key}` in {what}"),
            );
            d.label(label);
            match did_you_mean(&found, '`') {
                Some(help) => {
                    d.help(help);
                }
                None => {
                    if let Some(set) = listed(&open) {
                        d.note(format!("the keys {what} takes are {set}"));
                    }
                }
            }
            self.sink
                .suggested
                .extend(found.iter().map(|s| (*s).to_owned()));
        }
    }

    /// Report `key` as one this file's class or event does not take.
    pub fn not_here(&mut self, key: &str, message: String, label: &str) -> bool {
        let Some(span) = self.key_span(key) else {
            return false;
        };
        let d = self.sink.error("key.not-here", Some(span), message);
        d.label(label);
        d.help("remove this line".into());
        true
    }

    /// A value outside a closed set, with a suggestion or the set.
    pub fn unknown_value(
        &mut self,
        span: Span,
        message: String,
        label: &str,
        written: &str,
        set: &[&str],
        quote: char,
    ) {
        let found = suggest(written, set);
        let d = self.sink.error("value.unknown", Some(span), message);
        d.label(label);
        match did_you_mean(&found, quote) {
            Some(help) => {
                d.help(help);
            }
            None => {
                if let Some(all) = listed(set) {
                    d.note(format!("one of {all}"));
                }
            }
        }
    }

    /// `id` (or `name`) must equal the file name.
    pub fn named(&mut self, key: &str, v: &Spanned<Text>, stem: &str) -> bool {
        if v.str() == stem && crate::write::is_slug(stem) {
            return true;
        }
        if !crate::write::is_slug(v.str()) {
            let why = format!(
                "`{key}` must be a slug (lower-case letters, digits and -), and {:?} is not",
                v.str()
            );
            self.sink
                .error("value.form", Some(v.span), why)
                .label("not a slug");
            return false;
        }
        let why = format!(
            "`{key}` is {:?}, but the file is named {stem}.toml",
            v.str()
        );
        let d = self.sink.error("file.name", Some(v.span), why);
        d.label("does not match the file name");
        d.help(format!(
            "write {key} = \"{stem}\", or rename the file to {}.toml",
            v.str()
        ));
        false
    }
}
