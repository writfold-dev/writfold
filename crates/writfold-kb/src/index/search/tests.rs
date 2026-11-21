//! `find`'s edit distances and typo candidates.
use super::{closeness, distance, near, trigrams};

/// Every option compared, as before options were skipped by length.
fn every(query: &str, candidate: &str) -> (usize, u8, String) {
    let (query, candidate) = (query.to_lowercase(), candidate.to_lowercase());
    let split = |c: char| c.is_whitespace() || matches!(c, '/' | ':' | ',' | '(' | ')');
    let words: Vec<&str> = candidate.split(split).filter(|w| !w.is_empty()).collect();
    let width = query.split_whitespace().count().max(1);
    let mut options: Vec<(u8, String)> = vec![(0, candidate.clone())];
    options.extend(words.iter().map(|w| (1, (*w).to_owned())));
    options.extend(words.windows(width).map(|w| (1, w.join(" "))));
    (options.into_iter())
        .map(|(part, option)| (distance(&query, &option), part, option))
        .min()
        .unwrap_or((usize::MAX, 1, String::new()))
}

#[test]
fn skipping_options_by_length_changes_no_answer() {
    let names = [
        "Slow ledger flush at month end",
        "harbor-api",
        "runbook:ops/restart-harbor",
        "SYN-1034",
        "Quartz spindel",
        "a b c",
        "",
    ];
    let queries = [
        "ledgr flush",
        "harbr-api",
        "syn-1234",
        "quartz spindle",
        "flsh",
        "x y",
    ];
    for q in queries {
        for c in names {
            let all = every(q, c);
            for bound in 0..6 {
                let want = (all.0 <= bound).then(|| all.clone());
                assert_eq!(near(q, c, bound), want, "{q} / {c} / {bound}");
            }
        }
    }
}

#[test]
fn distances() {
    assert_eq!(distance("ledger", "ledger"), 0);
    assert_eq!(distance("ledgr", "ledger"), 1);
    assert_eq!(distance("legder", "ledger"), 1);
    assert_eq!(distance("harbr-api", "harbor-api"), 1);
    assert_eq!(distance("", "abc"), 3);
    assert_eq!(
        closeness("fsync stal", "Fsync stall on the ledger store").0,
        1
    );
    assert_eq!(trigrams("Abcd"), ["abc", "bcd"]);
}

#[test]
fn a_substring_with_like_wildcards_matches_them_literally() -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE tri USING fts5(id, title, body, path UNINDEXED, tokenize = 'trigram');
         INSERT INTO tri VALUES ('a', 'Rate 50% off', 'x', 'a.md');
         INSERT INTO tri VALUES ('b', 'Rate 50 percent off', 'x', 'b.md');
         INSERT INTO tri VALUES ('c', 'snake_case', 'x', 'c.md');
         INSERT INTO tri VALUES ('d', 'snakeXcase', 'x', 'd.md');
         INSERT INTO tri VALUES ('e', 'plain', 'the LEDGER flush', 'e.md');",
    )?;
    let ids = |text: &str, limit: usize| -> rusqlite::Result<Vec<String>> {
        let found = super::substring(&conn, text, limit)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        Ok(found.into_iter().map(|h| h.id).collect())
    };
    assert_eq!(ids("50% off", 10)?, ["a"]);
    assert_eq!(ids("e_c", 10)?, ["c"]);
    assert_eq!(ids("ledger fl", 10)?, ["e"], "ASCII case folds");
    assert_eq!(ids("ate", 1)?, ["a"], "reading stops at the limit");
    Ok(())
}
