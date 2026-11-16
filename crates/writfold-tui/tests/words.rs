//! The plain vocabulary's honesty rules (H2–H6, H8), each failing when its
//! table row is reworded or removed.
use writfold_tui::say::Piece;
use writfold_tui::say_words::{self, GUARD, ITEM, RELATION, STANDING, STEP, Word};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn all() -> Vec<&'static Word> {
    say_words::tables()
        .into_iter()
        .flat_map(|(_, table)| table.iter())
        .collect()
}

#[test]
fn every_standing_has_a_phrase() {
    let sets: [(&[Word], &[&str]); 4] = [
        (
            STANDING,
            &[
                "open",
                "satisfied",
                "failed",
                "unknown",
                "stale",
                "waived",
                "attested",
                "later",
            ],
        ),
        (
            ITEM,
            &["supported", "not-applicable", "attested", "stale", "open"],
        ),
        (
            GUARD,
            &["would-deny", "would-ask", "would-nudge", "would-pass"],
        ),
        (STEP, &["done", "skipped", "current", "later"]),
    ];
    for (table, statuses) in sets {
        for status in statuses {
            assert!(
                say_words::find(table, status).is_some(),
                "{status} has no phrase"
            );
        }
    }
}

#[test]
fn only_satisfied_says_passed() -> TestResult {
    for word in all() {
        let text = format!("{} {}", word.short, word.long).to_lowercase();
        if text.contains("passed") && !text.contains("did not pass") {
            assert_eq!(word.exact, "satisfied", "{word:?}");
        }
    }
    for exact in ["attested", "waived"] {
        let word = say_words::standing(exact).ok_or(exact)?;
        let short = word.short.to_lowercase();
        for bad in ["pass", "satisf", "verified", "checked"] {
            assert!(!short.contains(bad), "{exact}: {short}");
        }
        assert!(word.long.contains("not"), "{exact}: {}", word.long);
    }
    Ok(())
}

#[test]
fn no_liveness_words() -> TestResult {
    let live =
        regex::Regex::new(r"\b(running|working|in progress|underway|busy|live|active|waiting)\b")?;
    let mut texts: Vec<String> = all()
        .iter()
        .flat_map(|w| [w.short.to_owned(), w.long.to_owned()])
        .collect();
    texts.extend(
        RELATION
            .iter()
            .flat_map(|r| [r.from.to_owned(), r.to.to_owned()]),
    );
    texts.extend(say_words::glossary());
    for text in texts {
        assert!(!live.is_match(&text.to_lowercase()), "{text}");
    }
    let current = say_words::find(STEP, "current").ok_or("current")?;
    assert_eq!(current.short, "next up");
    Ok(())
}

#[test]
fn guards_stay_conditional() {
    for word in GUARD {
        assert!(word.short.starts_with("would "), "{word:?}");
        assert!(word.long.starts_with("would "), "{word:?}");
    }
}

#[test]
fn stale_is_out_of_date_never_broken() {
    for word in all().into_iter().filter(|w| w.exact == "stale") {
        assert!(word.short.starts_with("out of date"), "{word:?}");
        for bad in ["fail", "broken", "wrong"] {
            assert!(!word.long.contains(bad), "{word:?}");
        }
    }
    let later = say_words::standing("later").map(|w| w.short);
    assert_eq!(later, Some("not judged yet"));
    let unknown = say_words::standing("unknown").map(|w| w.short);
    assert_eq!(unknown, Some("no verdict"));
}

#[test]
fn unknown_words_are_quoted() {
    assert_eq!(
        say_words::short(STANDING, "weird"),
        Piece::Quoted("weird".into())
    );
    assert_eq!(
        say_words::long(say_words::MOVED, "odd"),
        Piece::Quoted("odd".into())
    );
    assert_eq!(
        say_words::relation("rivals", true),
        Piece::Quoted("rivals".into())
    );
    assert_eq!(
        say_words::relation("concerns", true),
        Piece::Said("about it".into())
    );
}

#[test]
fn help_words_match_the_tables() {
    let help = say_words::glossary().join("\n");
    for word in all() {
        let plain = if word.short.is_empty() {
            "(no words)"
        } else {
            word.short
        };
        assert!(
            help.contains(&format!("{plain} = {}", word.exact)),
            "{word:?}"
        );
    }
    for relation in RELATION {
        assert!(
            help.contains(&format!("= {}", relation.exact)),
            "{relation:?}"
        );
    }
}
