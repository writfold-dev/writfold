//! Anchored shapes: an affix is taken off a verb's text only when it is
//! made of the row's own structured fields (or a fixed lexical form), so
//! the remainder is known to be the rest. `None` sends the text to
//! `say::quote` whole.
use crate::source::field;
use serde_json::Value;

/// `ID [STATUS] rest`, where `STATUS` is the row's own `status` and the
/// id has no space.
pub fn named(row: &Value) -> Option<(&str, &str)> {
    let text = field(row, "text");
    let status = field(row, "status");
    let (id, after) = text.split_once(' ')?;
    let rest = after.strip_prefix(&format!("[{status}]"))?;
    if status.is_empty() || id.is_empty() {
        return None;
    }
    match rest.strip_prefix(' ') {
        Some(rest) => Some((id, rest)),
        None if rest.is_empty() => Some((id, "")),
        None => None,
    }
}

/// `(optional)` at the start of an item's remainder.
pub fn optional(rest: &str) -> Option<&str> {
    let after = rest.strip_prefix("(optional)")?;
    match after.strip_prefix(' ') {
        Some(rest) => Some(rest),
        None if after.is_empty() => Some(""),
        None => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Mechanical,
    Attested,
    Checklist { covered: u64, required: u64 },
}

/// The gate class at the start of a gate's text, and the remainder.
pub fn class(text: &str) -> Option<(Class, &str)> {
    let (first, rest) = text.split_once(' ').unwrap_or((text, ""));
    match first.trim_end_matches(':') {
        "mechanical" => Some((Class::Mechanical, rest)),
        "attested" => Some((Class::Attested, rest)),
        "checklist" => {
            let (count, rest) = rest.split_once(' ')?;
            let rest = rest.strip_prefix("required covered")?;
            let (covered, required) = count.split_once('/')?;
            let class = Class::Checklist {
                covered: covered.parse().ok()?,
                required: required.parse().ok()?,
            };
            Some((class, rest.trim_start()))
        }
        _ => None,
    }
}

/// `NAME#ORDINAL @DIGEST12 from SOURCE`: the template's name and ordinal.
pub fn template(splice: &str) -> Option<(&str, u64)> {
    let mut words = splice.split(' ');
    let (name, ordinal) = words.next()?.rsplit_once('#')?;
    let digest = words.next()?.strip_prefix('@')?;
    let from = words.next()?;
    let hex = digest.len() == 12 && digest.bytes().all(|b| b.is_ascii_hexdigit());
    if name.is_empty() || !hex || from != "from" || words.next().is_none() {
        return None;
    }
    Some((name, ordinal.parse().ok()?))
}

fn shaped(token: &[u8], shape: &str) -> bool {
    token.len() == shape.len()
        && token.iter().zip(shape.bytes()).all(|(b, s)| match s {
            b'd' => b.is_ascii_digit(),
            other => *b == other,
        })
}

/// A leading `YYYY-MM-DDTHH:MM:SSZ` (respelt `YYYY-MM-DD HH:MM`) or
/// `YYYY-MM-DD` token, and the rest. Spelling only: no zone, no clock.
pub fn when(text: &str) -> Option<(String, &str)> {
    let (token, rest) = text.split_once(' ').unwrap_or((text, ""));
    let bytes = token.as_bytes();
    if shaped(bytes, "dddd-dd-ddTdd:dd:ddZ") {
        return Some((format!("{} {}", &token[..10], &token[11..16]), rest));
    }
    shaped(bytes, "dddd-dd-dd").then(|| (token.to_owned(), rest))
}

/// A field value that is a whole stamp, respelt; any other value whole.
pub fn respelt(value: &str) -> String {
    match when(value) {
        Some((at, "")) => at,
        _ => value.to_owned(),
    }
}

/// A timeline `plan` row: after its stamp, `STATUS ID` (the row's own
/// fields), then the remainder.
pub fn event(row: &Value) -> Option<(String, &str)> {
    let (at, rest) = when(field(row, "text"))?;
    let lead = format!("{} {}", field(row, "status"), field(row, "id"));
    let rest = rest.strip_prefix(&lead)?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some((at, rest.trim_start()))
}

/// A `field` continuation `key=value`, the key made of `[a-z_]`.
pub fn pair(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once('=')?;
    let fine = !key.is_empty() && key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    fine.then_some((key, value))
}

/// A guard's text ending in ` requires CONDITION`.
pub fn requires(text: &str) -> Option<&str> {
    let (_, condition) = text.rsplit_once(" requires ")?;
    (!condition.is_empty() && !condition.contains(' ')).then_some(condition)
}

/// A citation token as a place: `code:REPO:REST` is `REST` in `REPO`;
/// `web:URL` and `record:PATH` drop the scheme; any other id is whole.
pub fn locator(id: &str) -> (&str, Option<&str>) {
    if let Some(rest) = id.strip_prefix("code:")
        && let Some((repo, path)) = rest.split_once(':')
    {
        return (path, Some(repo));
    }
    for scheme in ["web:", "record:"] {
        if let Some(rest) = id.strip_prefix(scheme) {
            return (rest, None);
        }
    }
    (id, None)
}

/// The `kb` trailer `HASH+dirty:N`: `N` uncommitted changes, or 0 for a
/// bare hash.
pub fn stamp(kb: &str) -> Option<u64> {
    let (hash, dirty) = match kb.split_once("+dirty:") {
        Some((hash, n)) => (hash, n.parse().ok()?),
        None => (kb, 0),
    };
    let hex = !hash.is_empty() && hash.bytes().all(|b| b.is_ascii_hexdigit());
    hex.then_some(dirty)
}

/// A command's words when it is a `writfold kb` command.
pub fn argv(text: &str) -> Option<Vec<&str>> {
    let words: Vec<&str> = text.split(' ').collect();
    (words.len() > 2 && words[0] == "writfold" && words[1] == "kb").then_some(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn named_requires_the_rows_own_status() {
        let row = json!({"status": "stale", "text": "unit [satisfied] mechanical"});
        assert_eq!(named(&row), None);
        let row = json!({"status": "open", "text": "a b [open] x"});
        assert_eq!(named(&row), None);
        let row = json!({"status": "open", "text": "docs [open] The guide"});
        assert_eq!(named(&row), Some(("docs", "The guide")));
        let row = json!({"status": "later", "text": "unit [later]"});
        assert_eq!(named(&row), Some(("unit", "")));
        let row = json!({"status": "", "text": "unit [] x"});
        assert_eq!(named(&row), None);
    }

    #[test]
    fn shapes_refuse_what_they_cannot_anchor() {
        assert_eq!(optional("(optional) Names"), Some("Names"));
        assert_eq!(optional("(optional)x"), None);
        assert_eq!(
            class("checklist 2/4 required covered at kb"),
            Some((
                Class::Checklist {
                    covered: 2,
                    required: 4
                },
                "at kb"
            ))
        );
        assert_eq!(
            class("attested: statements"),
            Some((Class::Attested, "statements"))
        );
        assert_eq!(class("checklist 2/4 covered"), None);
        assert_eq!(class("manual"), None);
        let splice = "findings#1 @6ec817d3031a from .writfold/plans/findings.toml";
        assert_eq!(template(splice), Some(("findings", 1)));
        assert_eq!(template("findings#1 @6ec8 from x"), None);
        assert_eq!(template("findings @6ec817d3031a from x"), None);
        assert_eq!(pair("decided_by=ops"), Some(("decided_by", "ops")));
        assert_eq!(pair("Key=v"), None);
        assert_eq!(requires("stop requires step.met"), Some("step.met"));
        assert_eq!(requires("stop needs step.met"), None);
        assert_eq!(
            locator("code:demo:src/a.rs:2-4"),
            ("src/a.rs:2-4", Some("demo"))
        );
        assert_eq!(locator("web:https://x"), ("https://x", None));
        assert_eq!(locator("harbor-api"), ("harbor-api", None));
        assert_eq!(stamp("0ea3a743bfef+dirty:19"), Some(19));
        assert_eq!(stamp("441cbd386780"), Some(0));
        assert_eq!(stamp("unknown"), None);
        assert_eq!(argv("writfold kb gate run x").map(|a| a.len()), Some(5));
        assert_eq!(argv("git commit"), None);
    }

    #[test]
    fn stamps_are_respelt_only() {
        let at = when("2026-11-10T23:59:50Z done x");
        assert_eq!(at, Some(("2026-11-10 23:59".to_owned(), "done x")));
        assert_eq!(
            when("2026-03-02 created"),
            Some(("2026-03-02".into(), "created"))
        );
        for text in [
            "2026-11-10T23:59:50",
            "2026-11-1",
            "26-11-10",
            "x 2026-11-10",
            "",
        ] {
            assert_eq!(when(text), None, "{text}");
        }
        let row =
            json!({"status": "done", "id": "c#1/a", "text": "2026-11-10T23:47:00Z done c#1/a"});
        assert_eq!(event(&row), Some(("2026-11-10 23:47".into(), "")));
        let row =
            json!({"status": "done", "id": "c#1/a", "text": "2026-11-10T23:47:00Z done c#1/ab"});
        assert_eq!(event(&row), None);
    }
}
