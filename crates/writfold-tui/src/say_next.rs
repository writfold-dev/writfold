//! The suggested next step in words, from a command's leading argv. The
//! gate, item and draft names are the argv's own words; the command
//! itself is one `c` away.
use crate::say::{Piece, quote, said};
use crate::say_shape::argv;

/// The phrase for a `next`, `confirm` or `accept` command.
pub fn suggested(command: &str) -> Piece {
    let fallback = || said("a command is suggested (c)");
    let Some(words) = argv(command) else {
        return fallback();
    };
    let name = |at: usize| words.get(at).copied().and_then(|w| quote(w).map(|_| w));
    let text = match words.get(2..).unwrap_or_default() {
        ["gate", "run", ..] => name(4).map(|g| format!("run the {g} gate")),
        ["gate", "cover", ..] => name(4)
            .zip(name(5))
            .map(|(g, item)| format!("give evidence for {item} in {g}")),
        ["gate", "attest", ..] => name(4).map(|g| format!("record a sign-off for {g}")),
        ["checkpoint", "--from-draft", ..] => Some("confirming it records a checkpoint".into()),
        ["proposals", "accept", ..] => Some("accepting it adds it to the KB".into()),
        ["stale", ..] => Some("re-check the cited sources".into()),
        _ => None,
    };
    text.map_or_else(fallback, said)
}

#[cfg(test)]
mod tests {
    use super::suggested;
    use crate::say::said;

    #[test]
    fn suggestions_name_the_argv_words() {
        for (command, phrase) in [
            ("writfold kb gate run lint", "run the lint gate"),
            (
                "writfold kb gate cover review-list tests supported REF… --by WHO",
                "give evidence for tests in review-list",
            ),
            (
                "writfold kb gate attest signoff --statement TEXT --by WHO",
                "record a sign-off for signoff",
            ),
            (
                "writfold kb checkpoint --from-draft s-progress",
                "confirming it records a checkpoint",
            ),
            (
                "writfold kb proposals accept 2c6ba260f2cde3fa",
                "accepting it adds it to the KB",
            ),
            ("writfold kb stale harbor", "re-check the cited sources"),
            ("writfold kb gate run", "a command is suggested (c)"),
            ("writfold kb gate waive x", "a command is suggested (c)"),
            ("make test", "a command is suggested (c)"),
        ] {
            assert_eq!(suggested(command), said(phrase), "{command}");
        }
    }
}
