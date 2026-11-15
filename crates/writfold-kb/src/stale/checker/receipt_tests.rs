use super::{Entry, Moved, Receipt, digest, encode, parse};
use writfold_core::verdict::Said;

fn sample() -> Receipt {
    let entry = |raw: &str, said, moved| Entry {
        token: digest(raw),
        said,
        at: Some("r42".into()),
        detail: None,
        moved,
    };
    Receipt {
        path: ".writfold/checks/src/x.json".into(),
        kind: "src".into(),
        checker: "acme-src".into(),
        version: Some("1.4.0".into()),
        protocol: Some("writfold-checker/1".into()),
        checked_at: "2026-11-15T02:00:00Z".into(),
        at: crate::time::parse("2026-11-15T02:00:00Z").unwrap_or_default(),
        verdicts: vec![
            entry("b", Said::Fresh, None),
            entry(
                "a",
                Said::Moved,
                Some(Moved {
                    path: Some("lib/new.c".into()),
                    first: 12,
                    last: 18,
                }),
            ),
        ],
    }
}

fn reparse(text: &str) -> Result<Receipt, String> {
    parse(".writfold/checks/src/x.json", "src", text)
}

#[test]
fn encoding_round_trips_sorted_one_verdict_per_line() -> Result<(), String> {
    let receipt = sample();
    let text = encode(&receipt);
    let mut back = reparse(&text)?;
    let mut want = receipt.verdicts.clone();
    want.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(back.verdicts, want);
    back.verdicts = receipt.verdicts.clone();
    assert_eq!(back, receipt);
    assert_eq!(text.lines().filter(|l| l.contains("\"token\"")).count(), 2);
    assert!(text.starts_with("{\n  \"schema\": \"writfold-check-receipt/1\",\n"));
    Ok(())
}

/// Replace `from` in the encoded sample and expect a refusal naming `why`.
fn refused(from: &str, to: &str, why: &str) -> Result<(), String> {
    let text = encode(&sample());
    if !text.contains(from) {
        return Err(format!("{from} not in {text}"));
    }
    let broken = text.replacen(from, to, 1);
    match reparse(&broken) {
        Err(got) if got.contains(why) => Ok(()),
        other => Err(format!("{other:?} lacks {why}: {broken}")),
    }
}

#[test]
fn anything_unexpected_skips_the_whole_file() -> Result<(), String> {
    refused(
        "\"kind\": \"src\"",
        "\"kind\": \"src\", \"extra\": 1",
        "unknown key extra",
    )?;
    refused(
        "writfold-check-receipt/1",
        "writfold-check-receipt/2",
        "schema",
    )?;
    refused("\"kind\": \"src\"", "\"kind\": \"doc\"", "kind is not src")?;
    refused("2026-11-15T02:00:00Z", "2026-11-15", "checked_at")?;
    refused("2026-11-15T02:00:00Z", "yesterday", "checked_at")?;
    refused(
        "\"status\":\"fresh\"",
        "\"status\":\"unverifiable\"",
        "status",
    )?;
    refused("\"status\":\"fresh\"", "\"status\":\"stale\"", "status")?;
    refused("\"first\":12", "\"first\":0", "valid range")?;
    refused("\"last\":18", "\"last\":11", "valid range")?;
    refused(
        ",\"moved\":{\"first\":12,\"last\":18,\"path\":\"lib/new.c\"}",
        "",
        "moved without a range",
    )?;
    refused(
        "\"status\":\"fresh\"",
        "\"status\":\"fresh\",\"moved\":{\"first\":1,\"last\":1}",
        "moved beside fresh",
    )?;
    refused("\"token\":\"sha256:", "\"token\":\"sha1:", "64 hex")?;
    refused(
        "\"name\":\"acme-src\"",
        "\"name\":\"acme-src\",\"key\":\"x\"",
        "unknown key key",
    )?;
    refused(
        "\"at\":\"r42\"",
        "\"at\":\"r42\",\"why\":\"x\"",
        "unknown key why",
    )?;
    let text = encode(&sample());
    let token = digest("a");
    let dup = text.replacen(&digest("b"), &token, 1);
    assert!(
        reparse(&dup).is_err_and(|e| e.contains("duplicate token")),
        "{dup}"
    );
    assert!(reparse("[]").is_err_and(|e| e.contains("not a JSON object")));
    assert!(reparse("{").is_err_and(|e| e.contains("not JSON")));
    Ok(())
}
