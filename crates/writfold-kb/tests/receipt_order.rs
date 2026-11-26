//! Receipts are judged oldest first by `seq`, then by file name; no clock
//! enters. A receipt written later with a clock that ran back still
//! decides: a delivery check's newest receipt is the one with the highest
//! `seq` ([`writfold_core::order`]).
mod support;

use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn seqs(paths: &[std::path::PathBuf]) -> TestResult<Vec<u64>> {
    let mut out = Vec::new();
    for path in paths {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        out.push(value["seq"].as_u64().ok_or("no seq")?);
    }
    out.sort();
    Ok(out)
}

fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

fn check(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: Some(id.into()),
        check: Some("paragraphs.bounded".into()),
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

#[test]
fn a_delivery_receipt_written_after_a_clock_ran_back_still_decides() -> TestResult {
    let w = world(&[])?;
    let from = w.base.join("update.md");
    std::fs::write(&from, "One short paragraph.\n")?;
    let p = Prepare {
        mode: "slack-summary".into(),
        from: from.clone(),
        audience: Some("internal".into()),
        destination: None,
        run: Some(RUN.into()),
        invocation: None,
    };
    let (text, exit) = deliver(&w, DeliverVerb::Prepare(p), NOW)?;
    assert_eq!(exit, 0, "{text}");
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let candidate = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .next()
        .ok_or("no candidate")?;
    check(&w, &candidate, NOW + 600)?;
    check(&w, &candidate, NOW + 300)?;
    let checks = dir.join(&candidate).join("checks");
    let mut written: Vec<std::path::PathBuf> = std::fs::read_dir(&checks)?
        .flatten()
        .map(|e| e.path())
        .collect();
    written.sort();
    assert_eq!(seqs(&written)?, vec![1, 2]);
    let verb = DeliverVerb::Status {
        candidate: Some(candidate.clone()),
        run: Some(RUN.into()),
        all: false,
    };
    let (text, _) = deliver(&w, verb, NOW + 3600)?;
    // The second check (written at 00:05, after the one stamped 00:10)
    // decides.
    let line = (text.lines())
        .find(|l| l.contains("check: paragraphs.bounded ["))
        .ok_or("no check line")?;
    assert!(
        line.contains("paragraphs.bounded-20261111T000500Z-"),
        "{text}"
    );
    Ok(())
}

/// A receipt written before receipts carried `seq` reads as 0: it is read,
/// and judged before every receipt that carries one.
#[test]
fn a_receipt_without_seq_is_read_and_comes_first() -> TestResult {
    use sha2::{Digest, Sha256};
    use writfold_kb::deliver::receipt;
    let w = world(&[])?;
    let from = w.base.join("update.md");
    std::fs::write(&from, "One short paragraph.\n")?;
    let p = Prepare {
        mode: "slack-summary".into(),
        from,
        audience: Some("internal".into()),
        destination: None,
        run: Some(RUN.into()),
        invocation: None,
    };
    deliver(&w, DeliverVerb::Prepare(p), NOW)?;
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let candidate = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .next()
        .ok_or("no candidate")?;
    check(&w, &candidate, NOW + 60)?;
    let checks = dir.join(&candidate).join("checks");
    let path = std::fs::read_dir(&checks)?
        .flatten()
        .next()
        .ok_or("no receipt")?
        .path();
    // Rewrite it as an older Writfold wrote it: no `seq`, its own digest.
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let map = value.as_object_mut().ok_or("not an object")?;
    map.remove("seq");
    map.remove("digest");
    let hex: String = Sha256::digest(writfold_kb::gate::receipt::canonical(&value).as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    value["digest"] = serde_json::Value::String(format!("sha256:{hex}"));
    std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;
    let candidate_dir = format!("runs/{RUN}/deliveries/{candidate}");
    let old = receipt::filed(&w.kb, &candidate_dir, RUN, &candidate);
    let old_seq = old[0].parsed.as_ref().map_err(|e| e.clone())?.seq;
    assert_eq!(old_seq, 0);
    // A new receipt is judged after it, whatever the names say.
    check(&w, &candidate, NOW + 30)?;
    let all = receipt::filed(&w.kb, &candidate_dir, RUN, &candidate);
    let mut valid: Vec<_> = (all.iter())
        .filter_map(|f| f.parsed.as_ref().ok().map(|r| (f, r)))
        .collect();
    valid.sort_by(|a, b| receipt::order(*a, *b));
    let order: Vec<u64> = valid.iter().map(|(_, r)| r.seq).collect();
    assert_eq!(order, vec![0, 1]);
    Ok(())
}
