//! The TOML subset gate definitions use, as plain values: the reader is
//! [`crate::defs::toml`], which keeps spans and reports every fault; this
//! module keeps the earlier shape (a map of plain values, the first fault
//! as `line N: why`) for the callers and tests that read it that way.
//! Every file in this subset is valid TOML and means the same thing to a
//! full TOML parser.
use crate::defs::{self, Sink};
use std::collections::BTreeMap;

pub use crate::defs::toml::{Doc, Entry, Slot, Spanned, Text};

/// One value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    List(Vec<String>),
}

/// The key-value pairs of a definition file. Any fault (duplicate keys
/// included) refuses the file with the first one's line.
pub fn parse(text: &str) -> Result<BTreeMap<String, Value>, String> {
    let mut sink = Sink::new("", text);
    let doc = defs::toml::read(text, &mut sink);
    if let Some(first) = sink.report().diagnostics.first() {
        let line = first.at.map_or(1, |a| a.line);
        return Err(format!("line {line}: {}", first.message));
    }
    Ok(doc
        .entries
        .into_iter()
        .filter_map(|e| match e.value {
            Slot::Value(v) => Some((e.key.value, plain(v.value))),
            Slot::Broken => None,
        })
        .collect())
}

fn plain(value: defs::Value) -> Value {
    match value {
        defs::Value::Str(t) => Value::Str(t.text),
        defs::Value::Int(n) => Value::Int(n),
        defs::Value::List(items) => Value::List(items.into_iter().map(|i| i.value.text).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, parse};

    #[test]
    fn the_subset_parses_and_everything_else_is_refused() -> Result<(), String> {
        let text = "# a gate\nid = \"unit\" # trailing\nversion = 1_000\nargv = [\n  \"a\", # first\n  'b\\c',\n]\nempty = []\nq = \"x\\\"y\\u00e9\"\n";
        let map = parse(text)?;
        assert_eq!(map.get("id"), Some(&Value::Str("unit".into())));
        assert_eq!(map.get("version"), Some(&Value::Int(1000)));
        assert_eq!(
            map.get("argv"),
            Some(&Value::List(vec!["a".into(), "b\\c".into()]))
        );
        assert_eq!(map.get("empty"), Some(&Value::List(Vec::new())));
        assert_eq!(map.get("q"), Some(&Value::Str("x\"y\u{e9}".into())));
        for bad in [
            "[gate]\nid = \"x\"\n",
            "a.b = 1\n",
            "\"a\" = 1\n",
            "a = true\n",
            "a = 1.5\n",
            "a = 01\n",
            "a = \"\"\"x\"\"\"\n",
            "a = [1, 2]\n",
            "a = { b = 1 }\n",
            "a = \"x\" b\n",
            "a = 1\na = 2\n",
            "a = \"open\n",
            "a = \"\\q\"\n",
            "a\n",
        ] {
            assert!(parse(bad).is_err(), "accepted {bad:?}");
        }
        Ok(())
    }
}
