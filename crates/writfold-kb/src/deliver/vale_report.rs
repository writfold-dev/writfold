//! Reading `vale --output=JSON --counts` (#117): `{"files": {path:
//! [alert…]}, "counts": {check: n}}`. A file with no alert is absent from
//! `files`; `counts` has one entry per loaded check, zeros included. The
//! report is *whole* only when it parses in that shape, every severity is
//! one of Vale's three, and every alert is about one of the two files
//! Writfold gave. Tested, not proved; what a report means is
//! [`writfold_core::vale`].
use serde_json::Value;
use writfold_core::vale::Level;

/// The file names Writfold gives Vale.
pub const DOCUMENT: &str = "document.md";
pub const CANARY: &str = "canary.md";

/// One alert on the document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Alert {
    pub line: u32,
    pub column: u32,
    /// Vale's check name, verbatim (`House.Hedges`).
    pub check: String,
    pub level: Level,
    pub message: String,
}

/// What one report says.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    /// Alerts on the document, in Vale's order.
    pub document: Vec<Alert>,
    /// Alerts on the canary.
    pub canary: usize,
    /// Error-level alerts on either file.
    pub errors: usize,
    /// Checks Vale loaded.
    pub enabled: usize,
}

pub fn level_name(level: Level) -> &'static str {
    match level {
        Level::Suggestion => "suggestion",
        Level::Warning => "warning",
        Level::Error => "error",
    }
}

fn level(text: &str) -> Option<Level> {
    match text {
        "suggestion" => Some(Level::Suggestion),
        "warning" => Some(Level::Warning),
        "error" => Some(Level::Error),
        _ => None,
    }
}

fn alert(v: &Value) -> Option<Alert> {
    let span = v.get("Span")?.as_array()?;
    let number = |v: Option<&Value>| v?.as_u64().and_then(|n| u32::try_from(n).ok());
    Some(Alert {
        line: number(v.get("Line"))?,
        column: number(span.first())?,
        check: v.get("Check")?.as_str()?.to_owned(),
        level: level(v.get("Severity")?.as_str()?)?,
        message: v.get("Message")?.as_str()?.to_owned(),
    })
}

/// Which of the two files a reported path is: `Some(true)` for the
/// canary. `dir` is the private directory Vale ran in.
fn which(path: &str, dir: &str) -> Option<bool> {
    let name = path
        .strip_prefix(dir)
        .map_or(path, |p| p.trim_start_matches('/'));
    match name {
        DOCUMENT => Some(false),
        CANARY => Some(true),
        _ => None,
    }
}

/// The report in `stdout`, or `None` when it is not whole.
pub fn read(stdout: &[u8], dir: &str) -> Option<Report> {
    let v: Value = serde_json::from_slice(stdout).ok()?;
    let files = v.get("files")?.as_object()?;
    let counts = v.get("counts")?.as_object()?;
    let mut report = Report {
        enabled: counts.len(),
        ..Report::default()
    };
    for (path, alerts) in files {
        let canary = which(path, dir)?;
        for a in alerts.as_array()? {
            let a = alert(a)?;
            report.errors += usize::from(a.level == Level::Error);
            match canary {
                true => report.canary += 1,
                false => report.document.push(a),
            }
        }
    }
    Some(report)
}

/// `vale version 3.28.1` → `3.28.1`.
pub fn version(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    let v = text.trim().strip_prefix("vale version ")?.trim();
    super::vale_profile::version(v).map(|_| v.to_owned())
}

/// Vale's error on stderr for exit 2: `Code: Text`, or the raw first line.
pub fn error(stderr: &[u8]) -> String {
    let parsed: Option<Value> = serde_json::from_slice(stderr).ok();
    let field = |k: &str| {
        let v = parsed.as_ref()?.get(k)?.as_str()?;
        Some(v.trim().to_owned())
    };
    match (field("Code"), field("Text")) {
        (Some(code), Some(text)) => format!("{code}: {text}"),
        _ => {
            let text = String::from_utf8_lossy(stderr);
            let first = text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("no message");
            first.chars().take(200).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, read, version};

    #[test]
    fn a_report_is_read_only_when_whole() -> Result<(), &'static str> {
        let good = br#"{"files": {"document.md": [{"Check": "House.Hedges", "Severity": "warning", "Line": 3, "Span": [5, 9], "Message": "Avoid 'very'."}], "/t/canary.md": [{"Check": "House.Todo", "Severity": "error", "Line": 1, "Span": [1, 4], "Message": "No TODO."}]}, "counts": {"House.Hedges": 1, "House.Todo": 1, "House.Terms": 0}}"#;
        let r = read(good, "/t").ok_or("unread")?;
        assert_eq!(
            (r.document.len(), r.canary, r.errors, r.enabled),
            (1, 1, 1, 3)
        );
        assert_eq!(r.document[0].level, Level::Warning);
        assert_eq!((r.document[0].line, r.document[0].column), (3, 5));
        let text = String::from_utf8_lossy(good);
        for bad in [
            text.replace("\"warning\"", "\"fatal\""),
            text.replace("document.md", "other.md"),
            text.replace("\"counts\"", "\"tally\""),
            text[..40].to_owned(),
        ] {
            assert!(read(bad.as_bytes(), "/t").is_none(), "{bad}");
        }
        assert_eq!(version(b"vale version 3.28.1\n").as_deref(), Some("3.28.1"));
        assert_eq!(version(b"vale 3"), None);
        Ok(())
    }
}
