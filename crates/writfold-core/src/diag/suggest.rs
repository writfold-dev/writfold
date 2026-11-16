//! "Did you mean": the candidates at a position that are close to what was
//! written. The set is always the closed set at that position, nothing
//! wider, and the bounds keep a deliberately different name from being
//! "corrected".
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Optimal string alignment distance (one transposition counts once).
pub fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut rows = alloc::vec![alloc::vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    rows[0] = (0..=b.len()).collect();
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

fn normal(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '_' | '-' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The most edits allowed when the shorter word has `n` characters.
pub fn bound(n: usize) -> usize {
    match n {
        0..=2 => 0,
        3 | 4 => 1,
        5..=8 => 2,
        _ => 3,
    }
}

/// At most two candidates close to `written`: equal once case and `_`,
/// `-`, `.` are removed; or within the distance bound of the shorter
/// word; or, failing both, the one candidate `written` (3 characters or
/// more) is a prefix of. Only the closest are kept; three or more at the
/// same distance tie, and nothing is suggested.
pub fn suggest<'c>(written: &str, candidates: &[&'c str]) -> Vec<&'c str> {
    let size = written.chars().count();
    if size < 3 {
        return Vec::new();
    }
    let lower: String = written.chars().flat_map(char::to_lowercase).collect();
    let same: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|c| *c != written && normal(c) == normal(written))
        .collect();
    let mut found: Vec<(usize, &str)> = same.iter().map(|c| (0, *c)).collect();
    if found.is_empty() {
        for c in candidates.iter().copied().filter(|c| *c != written) {
            let other: String = c.chars().flat_map(char::to_lowercase).collect();
            let d = distance(&lower, &other);
            let shorter = size.min(other.chars().count());
            if d >= 1 && d <= bound(shorter) {
                found.push((d, c));
            }
        }
    }
    if found.is_empty() {
        let prefixed: Vec<&str> = candidates
            .iter()
            .copied()
            .filter(|c| c.len() > written.len() && c.to_lowercase().starts_with(&lower))
            .collect();
        if let [one] = prefixed.as_slice() {
            found.push((0, one));
        }
    }
    let best = found.iter().map(|(d, _)| *d).min().unwrap_or(0);
    let kept: Vec<&str> = found
        .into_iter()
        .filter(|(d, _)| *d == best)
        .map(|(_, c)| c)
        .collect();
    match kept.len() {
        1 | 2 => kept,
        _ => Vec::new(),
    }
}

/// ``did you mean `x`?`` or ``did you mean `x` or `y`?``, quoting each
/// with `open` and `close` (backticks, or double quotes for values).
pub fn did_you_mean(found: &[&str], quote: char) -> Option<String> {
    match found {
        [one] => Some(format!("did you mean {quote}{one}{quote}?")),
        [one, two] => Some(format!(
            "did you mean {quote}{one}{quote} or {quote}{two}{quote}?"
        )),
        _ => None,
    }
}

/// The closed set as a note, when it has at most eight members:
/// `a, b or c`.
pub fn listed(set: &[&str]) -> Option<String> {
    match set {
        [] => None,
        [one] => Some(String::from(*one)),
        _ if set.len() > 8 => None,
        [head @ .., last] => Some(format!("{} or {last}", head.join(", "))),
    }
}
