//! The terse list: notation the plain surface must not show.
use super::TestResult;

pub const TERSE: &[&str] = &[
    r"\d{4}-\d\d-\d\dT\d\d:\d\d",
    r"\d\d:\d\d:\d\dZ",
    r"(^|\s)@\S",
    r"\b[0-9a-f]{12,}\b",
    r"\b[a-z_]+=\S",
    r"\[[a-z-]+\]",
    r"(^|\s)![a-z]",
    r"run-\d{8}T",
    r"REF…|--by\b|--statement\b|\bWHO\b",
    r"(^|\s)(owes|next|read|confirm|accept|splice|under|field): ",
    r"# note:",
    r"\b(project|inv|sig|code|web|record|gate|seen):\S",
    r"\+dirty:",
    r"writfold kb",
    r"→",
    r"would-[a-z]+",
    r"[●◆○!×?·]{2,}",
];

/// The terse patterns `text` matches.
pub fn found(text: &str) -> TestResult<Vec<String>> {
    let mut out = Vec::new();
    for pattern in TERSE {
        let re = regex::Regex::new(pattern)?;
        if let Some(m) = re.find(text) {
            out.push(format!("{pattern}: {:?}", m.as_str()));
        }
    }
    Ok(out)
}
