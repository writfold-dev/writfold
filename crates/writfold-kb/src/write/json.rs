//! JSON in the layout the KB's files already use: keys in a fixed order,
//! two-space indentation, `", "`-free item separators (one item per line),
//! every non-ASCII character escaped as `\uXXXX`, and a final line break.
//! Writfold renders it itself because `serde_json`'s map sorts keys.

/// A value Writfold writes into a record.
#[derive(Clone, Debug)]
pub enum Json {
    Int(i64),
    Text(String),
    List(Vec<String>),
}

/// A JSON string literal with ASCII-only output.
pub fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An object of `fields`, in order.
pub fn object(fields: &[(&str, Json)]) -> String {
    let mut out = String::from("{\n");
    for (index, (key, value)) in fields.iter().enumerate() {
        out.push_str(&format!("  {}: ", quote(key)));
        match value {
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Text(text) => out.push_str(&quote(text)),
            Json::List(items) if items.is_empty() => out.push_str("[]"),
            Json::List(items) => {
                out.push_str("[\n");
                let rendered: Vec<String> =
                    items.iter().map(|i| format!("    {}", quote(i))).collect();
                out.push_str(&rendered.join(",\n"));
                out.push_str("\n  ]");
            }
        }
        if index + 1 < fields.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::{Json, object, quote};

    #[test]
    fn layout() {
        let text = object(&[
            ("schema_version", Json::Int(1)),
            ("id", Json::Text("a".into())),
            ("evidence", Json::List(vec![])),
            ("tags", Json::List(vec!["x".into(), "y".into()])),
        ]);
        assert_eq!(
            text,
            "{\n  \"schema_version\": 1,\n  \"id\": \"a\",\n  \"evidence\": [],\n  \"tags\": [\n    \"x\",\n    \"y\"\n  ]\n}\n"
        );
        assert_eq!(
            quote("é→\u{1f600}\"\u{1}"),
            "\"\\u00e9\\u2192\\ud83d\\ude00\\\"\\u0001\""
        );
        assert!(serde_json::from_str::<serde_json::Value>(&text).is_ok());
    }
}
