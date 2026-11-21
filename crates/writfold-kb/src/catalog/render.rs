//! The bytes a catalog projects (#86): one asset record per row, pretty
//! JSON in a fixed key order with LF line ends and the ownership marker
//! last; and a standard Git patch over the affected files, sorted by path,
//! with no timestamps, absolute paths or local configuration. Tested only.

/// A mapped field's value.
#[derive(Clone, Debug)]
pub enum Value {
    Text(String),
    List(Vec<String>),
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

/// The record for one row: `values` in declaration-table order, then
/// `writfold_catalog` naming the catalog and the KB-relative source file.
pub fn record(values: &[(&str, Value)], catalog: &str, source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for (name, value) in values {
        let shown = match value {
            Value::Text(t) => quoted(t),
            Value::List(items) if items.is_empty() => "[]".into(),
            Value::List(items) => {
                let inner: Vec<String> =
                    items.iter().map(|i| format!("    {}", quoted(i))).collect();
                format!("[\n{}\n  ]", inner.join(",\n"))
            }
        };
        lines.push(format!("  {}: {shown}", quoted(name)));
    }
    lines.push(format!(
        "  \"writfold_catalog\": {{\n    \"name\": {},\n    \"source\": {}\n  }}",
        quoted(catalog),
        quoted(source)
    ));
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

/// One file's change: its KB-relative path, the present bytes (`None`
/// when absent) and the wanted bytes (`None` to remove it).
pub struct Change<'a> {
    pub path: &'a str,
    pub old: Option<&'a str>,
    pub new: Option<&'a str>,
}

/// The `+`/`-` lines of `text`, with Git's marker for a missing final LF.
fn hunk_lines(out: &mut String, sign: char, text: &str) {
    for line in text.split_inclusive('\n') {
        out.push(sign);
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push_str("\n\\ No newline at end of file\n");
        }
    }
}

fn count(text: &str) -> usize {
    text.split_inclusive('\n').count()
}

/// The range of a hunk side: `start,len`, with Git's `0,0` for nothing.
fn range(len: usize) -> String {
    match len {
        0 => "0,0".into(),
        n => format!("1,{n}"),
    }
}

/// A patch `git apply` takes, each file replaced whole, sorted by path.
pub fn patch(changes: &mut [Change<'_>]) -> String {
    changes.sort_by(|a, b| a.path.cmp(b.path));
    let mut out = String::new();
    for c in changes.iter() {
        let (old, new) = (c.old.unwrap_or_default(), c.new.unwrap_or_default());
        out.push_str(&format!("diff --git a/{p} b/{p}\n", p = c.path));
        match (c.old, c.new) {
            (None, _) => out.push_str("new file mode 100644\n--- /dev/null\n"),
            (_, None) => out.push_str(&format!("deleted file mode 100644\n--- a/{}\n", c.path)),
            _ => out.push_str(&format!("--- a/{}\n", c.path)),
        }
        match c.new {
            None => out.push_str("+++ /dev/null\n"),
            Some(_) => out.push_str(&format!("+++ b/{}\n", c.path)),
        }
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(count(old)),
            range(count(new))
        ));
        hunk_lines(&mut out, '-', old);
        hunk_lines(&mut out, '+', new);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Change, Value, patch, record};

    #[test]
    fn records_are_byte_stable_and_patches_have_git_headers() {
        let values = [
            ("id", Value::Text("mix-station".into())),
            ("name", Value::Text("Mix \"station\"".into())),
            ("aliases", Value::List(vec!["brewdeck".into()])),
        ];
        let text = record(
            &values,
            "brewery",
            ".writfold/catalog-inputs/equipment.json",
        );
        assert_eq!(
            text,
            "{\n  \"id\": \"mix-station\",\n  \"name\": \"Mix \\\"station\\\"\",\n  \"aliases\": [\n    \"brewdeck\"\n  ],\n  \"writfold_catalog\": {\n    \"name\": \"brewery\",\n    \"source\": \".writfold/catalog-inputs/equipment.json\"\n  }\n}\n"
        );
        assert!(serde_json::from_str::<serde_json::Value>(&text).is_ok());
        let mut changes = [
            Change {
                path: "b.json",
                old: Some("x"),
                new: None,
            },
            Change {
                path: "a.json",
                old: None,
                new: Some("{}\n"),
            },
        ];
        let p = patch(&mut changes);
        assert!(p.starts_with("diff --git a/a.json b/a.json\nnew file mode 100644\n--- /dev/null\n+++ b/a.json\n@@ -0,0 +1,1 @@\n+{}\n"), "{p}");
        assert!(p.contains("deleted file mode 100644\n--- a/b.json\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-x\n\\ No newline at end of file\n"), "{p}");
    }
}
