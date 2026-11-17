//! `find`'s `typo` tier judges every record's ID and title, never a cap of
//! candidates ranked by their bodies: a title one edit from the query is
//! found however many bodies share the query's trigrams.
mod support;

use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

fn find(w: &World, text: &str) -> TestResult<Vec<Value>> {
    let mut request = w.request(Verb::Find {
        text: text.into(),
        filter: Filter::default(),
        signatures: false,
    });
    request.json = true;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    Ok(body["rows"].as_array().ok_or("rows")?.clone())
}

const TICKET: &str = "---\nticket: HBR-900\ntitle: Quartz spindel\nassets: [harbor-api]\nstatus: done\nprs: []\nsummary: Rare.\ncreated: 2026-10-28\nupdated: 2026-11-03\npromoted_to:\n---\n\n# HBR-900: Quartz spindel\n\n## Symptom\n\nLOREM\n";

#[test]
fn a_title_one_edit_away_is_found_past_many_matching_bodies() -> TestResult {
    let w = world(&[])?;
    let filler = "lorem ipsum dolor sit amet consectetur adipiscing elit ".repeat(40);
    std::fs::write(
        w.kb.join("ktlo/2026/hbr-900-quartz.md"),
        TICKET.replace("LOREM", &filler),
    )?;
    let notes = w.kb.join("notes");
    std::fs::create_dir_all(&notes)?;
    for i in 0..260 {
        std::fs::write(
            notes.join(format!("n-{i:03}.md")),
            "quartzite blitz spindles\n",
        )?;
    }
    let rows = find(&w, "quartz spindle")?;
    let hit = rows
        .iter()
        .find(|r| r["id"] == "HBR-900")
        .ok_or_else(|| format!("no HBR-900 row: {rows:?}"))?;
    assert_eq!(hit["matched_on"], "typo", "{hit}");
    Ok(())
}
