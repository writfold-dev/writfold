//! Retrieval receipts (the Tuft adapter's write): tokens cited at a pinned
//! commit check through `stale`, a receipt lands in the run through the
//! write protocol, `resume` shows it, and a closed or missing run refuses.
mod support;

use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::workspace::discover;
use writfold_kb::write::Write;
use writfold_kb::write::cite::{Lines, Span};
use writfold_kb::write::retrieval::{self, Retrieval};
use writfold_kb::write::status::SetStatus;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn receipt(run: Option<&str>, commit: &str, cites: Vec<String>) -> Retrieval {
    Retrieval {
        run: run.map(str::to_owned),
        tool: "tuft/0".into(),
        program: format!("sha256:{}", "ab".repeat(32)),
        repo: "demo-service".into(),
        commit: commit.into(),
        trailer: "# coverage=complete files=5/5 hits=1 shown=1 omitted=0 cursor=none program=abababababab".into(),
        rows: 1,
        shown: 1,
        cites,
    }
}

/// Tokens for `src/batch.rs` bytes 0..10 (line 1) and lines 2-3 by byte
/// range, at the code repository's HEAD.
fn tokens(w: &World) -> TestResult<(String, Vec<String>)> {
    let ws = discover(Some(&w.kb), &w.base)?;
    let head = git(&w.code, &["rev-parse", "HEAD"])?;
    let spans = [
        Span {
            path: "src/batch.rs".into(),
            lines: Lines::Bytes(0, 10),
        },
        Span {
            path: "src/batch.rs".into(),
            lines: Lines::Range(4, 5),
        },
    ];
    let cited = retrieval::cite_spans(Some(&ws), &w.code, &head, &spans, NOW)?;
    assert_eq!(cited.repo, "demo-service");
    assert!(cited.resolvable, "the fixture maps demo-service");
    Ok((head, cited.tokens))
}

fn stale(w: &World, scope: &str) -> TestResult<String> {
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: Some(scope.into()),
        offline: false,
    }))?;
    Ok(text)
}

#[test]
fn receipts_are_recorded_resumed_and_checked() -> TestResult {
    let w = world(&[])?;
    let (head, tokens) = tokens(&w)?;
    assert!(
        tokens[0].contains(" path=src/batch.rs L=1-1 blob="),
        "{}",
        tokens[0]
    );
    assert!(tokens[1].contains(&format!(" rev={head} path=src/batch.rs L=4-5 ")));
    let (text, exit) = w.run(&w.request(Verb::Write(Write::Retrieval(receipt(
        Some(RUN),
        &head,
        tokens.clone(),
    )))))?;
    assert_eq!(exit, 0, "{text}");
    let path = text
        .split_whitespace()
        .skip_while(|w| *w != "recorded")
        .nth(1)
        .ok_or("no recorded path")?
        .to_owned();
    assert!(path.starts_with(&format!("runs/{RUN}/retrievals/20261111T000000Z-")));
    let bytes = std::fs::read_to_string(w.kb.join(&path))?;
    assert!(bytes.starts_with(&format!(
        "---\nkind: retrieval\ntool: tuft/0\nrun: {RUN}\nprogram: sha256:abab"
    )));
    assert!(bytes.contains("\nrows: 1\nshown: 1\ncites: 2\nrecorded: 2026-11-11T00:00:00Z\n---\n"));
    assert!(bytes.ends_with(&format!(
        "\n## Citations\n\n- {}\n- {}\n",
        tokens[0], tokens[1]
    )));
    // Both tokens check, and are fresh: nothing is listed.
    let checked = stale(&w, &path)?;
    assert!(
        checked.contains(" checked=2 ") && !checked.contains("retrievals/"),
        "{checked}"
    );
    let (resumed, _) = w.run(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }))?;
    assert!(
        resumed.contains(&format!(
            "retrieval {} tuft/0 program=abababababab demo-service@{} rows=1 shown=1 cites=2 @{path}:1",
            &path[path.len() - 26..path.len() - 3],
            &head[..12]
        )),
        "{resumed}"
    );
    assert!(
        resumed.contains("  trailer: # coverage=complete files=5/5"),
        "{resumed}"
    );
    assert!(resumed.contains("  cite: src/batch.rs:4-5 @"), "{resumed}");
    let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !verify.contains("[differs]"), "{verify}");
    Ok(())
}

#[test]
fn a_missing_or_closed_run_refuses() -> TestResult {
    let w = world(&[])?;
    let refused =
        |r: Retrieval| match writfold_kb::run(&w.request(Verb::Write(Write::Retrieval(r)))) {
            Err(error) if error.exit_code() == 3 => Ok(error.to_string()),
            other => Err(format!(
                "expected a refusal, got {:?}",
                other.map(|o| o.text)
            )),
        };
    let commit = "0".repeat(40);
    assert!(refused(receipt(None, &commit, Vec::new()))?.contains("no current run"));
    assert!(refused(receipt(Some("run-x"), &commit, Vec::new()))?.contains("no run run-x"));
    let ws = discover(Some(&w.kb), &w.base)?;
    assert_eq!(retrieval::current(&ws), None);
    w.run(&w.request(Verb::Write(Write::SetStatus(SetStatus {
        target: RUN.into(),
        state: "closed".into(),
        ..SetStatus::default()
    }))))?;
    assert!(refused(receipt(Some(RUN), &commit, Vec::new()))?.contains("is closed"));
    let bad = receipt(Some(RUN), &commit, vec!["not a token".into()]);
    match writfold_kb::run(&w.request(Verb::Write(Write::Retrieval(bad)))) {
        Err(error) => assert_eq!(error.exit_code(), 1),
        Ok(output) => return Err(format!("recorded a non-token:\n{}", output.text).into()),
    }
    assert!(!w.kb.join(format!("runs/{RUN}/retrievals")).exists());
    Ok(())
}
