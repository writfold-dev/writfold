//! `kb lint --records`: a JSON record that is not JSON at the parser's
//! line and column, a front matter fence never closed, a key given twice,
//! a citation token that does not read, and a token missing a field its
//! kind declares, with the written field it may mean.
mod support;

use support::{TestResult, world};
use writfold_kb::Verb;

#[test]
fn records_report_each_fault_at_its_position() -> TestResult {
    let w = world(&[])?;
    let put = |rel: &str, text: &str| std::fs::write(w.kb.join(rel), text);
    let (text, exit) = w.run(&w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: true,
    }))?;
    assert_eq!(exit, 0, "{text}");
    put(
        "inventory/assets/broken.json",
        "{\"id\": \"x\",\n  \"name\" \"X\"}\n",
    )?;
    put(
        "ktlo/2026/hbr-9-twice.md",
        "---\nticket: HBR-9\nstatus: open\nstatus: done\n---\n# t\n",
    )?;
    put(
        "ktlo/2026/hbr-10-open.md",
        "---\nticket: HBR-10\n# never closed\n",
    )?;
    std::fs::create_dir_all(w.kb.join(".writfold/sources"))?;
    put(
        ".writfold/sources/tracker.toml",
        "schema = \"writfold-source/1\"\nkind = \"tracker\"\nfreshness = \"ttl\"\nrequired = [\"board\"]\n",
    )?;
    put(
        "ktlo/2026/hbr-11-cite.md",
        "---\nticket: HBR-11\n---\nSee ⟦cite kind=tracker bord=ops seen=2026-01-01T00:00Z⟧ and ⟦cite kind=web L=x-y⟧.\n",
    )?;
    let mut request = w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: true,
    });
    request.json = true;
    let out = writfold_kb::run(&request)?;
    assert_eq!(out.exit, 2, "{}", out.text);
    let body: serde_json::Value = serde_json::from_str(&out.text)?;
    let found: Vec<String> = body["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .map(|d| {
            format!(
                "{} {}:{}:{} {}",
                d["code"], d["file"], d["line"], d["col"], d["help"]
            )
        })
        .collect();
    let has = |want: &str| found.iter().any(|f| f.contains(want));
    assert!(
        has("\"syntax.json\" \"inventory/assets/broken.json\":2:10"),
        "{found:#?}"
    );
    assert!(
        has("\"syntax.expected\" \"ktlo/2026/hbr-10-open.md\":1:1"),
        "{found:#?}"
    );
    assert!(
        has("\"key.twice\" \"ktlo/2026/hbr-9-twice.md\":4:1"),
        "{found:#?}"
    );
    assert!(
        has("\"value.form\" \"ktlo/2026/hbr-11-cite.md\":4:"),
        "{found:#?}"
    );
    assert!(
        has(
            "\"key.missing\" \"ktlo/2026/hbr-11-cite.md\":4:5 \"did you mean `board`? (written `bord`)\""
        ),
        "{found:#?}"
    );
    assert_eq!(body["rows"][0]["kind"], "record");
    Ok(())
}
