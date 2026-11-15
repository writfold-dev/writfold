use super::{tracker, trailer, tree};
use crate::kb_entry::Entry;
use crate::source::Reply;
use serde_json::{Value, json};

/// A small deterministic generator, so a failure names its seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

const PARTS: [&str; 6] = ["harbor", "db", "2026", "ops", "x", "flush"];
const SEPARATORS: [&str; 4] = [":", "/", "#", "-"];

fn id(rng: &mut Lcg, n: usize) -> String {
    let mut out = format!("n{n}");
    for _ in 0..1 + rng.next(3) {
        out.push_str(SEPARATORS[rng.next(SEPARATORS.len())]);
        out.push_str(PARTS[rng.next(PARTS.len())]);
    }
    out
}

/// One `impact` row reached from `parent` at `depth`; the row is the
/// edge's source or its destination.
fn row(id: &str, parent: &str, depth: u64, source: bool) -> Value {
    let (from, to) = if source { (id, parent) } else { (parent, id) };
    json!({"kind": "ticket", "id": id, "status": "open", "text": "",
           "flags": [], "more": [], "path": "p.md", "line": 1,
           "edge": {"from": from, "rel": "concerns", "to": to, "origin": "field", "depth": depth}})
}

fn reply(rows: Vec<Value>) -> Reply {
    Reply {
        rows,
        ..Reply::default()
    }
}

/// The parent each entry hangs under, read back from the drawn order.
fn parents(entries: &[Entry], root: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut last_top = root.to_owned();
    for entry in entries.iter().filter(|e| !e.header) {
        if entry.depth == 0 {
            out.push((entry.id.clone(), root.to_owned()));
            last_top = entry.id.clone();
        } else {
            out.push((entry.id.clone(), last_top.clone()));
        }
    }
    out
}

#[test]
fn generated_answers_reproduce_their_parents() {
    for seed in 0..200u64 {
        let mut rng = Lcg(seed);
        let root = format!("root{seed}:{}", PARTS[rng.next(PARTS.len())]);
        let mut rows = Vec::new();
        let mut expected = Vec::new();
        let tops: Vec<String> = (0..1 + rng.next(5)).map(|n| id(&mut rng, n)).collect();
        let mut n = 10;
        for top in &tops {
            rows.push(row(top, &root, 1, rng.next(2) == 0));
            expected.push((top.clone(), root.clone()));
        }
        for _ in 0..rng.next(8) {
            let parent = &tops[rng.next(tops.len())];
            let child = id(&mut rng, n);
            n += 1;
            rows.push(row(&child, parent, 2, rng.next(2) == 0));
            expected.push((child, parent.clone()));
        }
        let entries = tree(&reply(rows.clone()), &root);
        assert_eq!(
            entries.iter().filter(|e| !e.header).count(),
            rows.len(),
            "seed {seed}: every row once"
        );
        let mut found = parents(&entries, &root);
        found.sort();
        expected.sort();
        assert_eq!(found, expected, "seed {seed}");
    }
}

#[test]
fn rows_without_an_edge_or_a_parent_are_drawn_flat() {
    let mut orphan = row("lost", "nowhere", 2, true);
    let mut bare = row("bare", "root", 1, true);
    bare.as_object_mut().map(|o| o.remove("edge"));
    orphan["status"] = json!("done");
    let rows = vec![row("a", "root", 1, false), orphan, bare];
    let entries = tree(&reply(rows), "root");
    let labels: Vec<_> = entries
        .iter()
        .map(|e| (e.header, e.label.as_str()))
        .collect();
    assert_eq!(
        labels,
        [
            (true, "root"),
            (false, "a"),
            (true, "depth 2"),
            (false, "lost"),
            (true, "depth ?"),
            (false, "bare"),
        ]
    );
}

#[test]
fn the_tracker_keeps_its_kinds_under_their_parents() {
    let mut asset = row("harbor-db", "project:harbor", 1, true);
    asset["kind"] = json!("asset");
    let rows = vec![
        row("HBR-1", "project:harbor", 1, true),
        asset,
        row("HBR-2", "harbor-db", 2, true),
    ];
    let entries = tracker(&reply(rows), "project:harbor");
    let labels: Vec<_> = entries
        .iter()
        .map(|e| (e.header, e.label.as_str()))
        .collect();
    assert_eq!(
        labels,
        [
            (true, "project:harbor"),
            (false, "HBR-1"),
            (true, "harbor-db"),
            (false, "HBR-2"),
        ]
    );
}

#[test]
fn the_trailer_names_every_non_zero_count() {
    let answer = Reply {
        trailer: json!({"shown": 14, "coverage": "partial",
            "counts": {"frontier": 3, "skipped_retired": 0, "skipped_disputed": 1,
                       "skipped_inferred": 6, "skipped_other": 2}}),
        ..Reply::default()
    };
    let (text, partial) = trailer(&answer);
    assert_eq!(
        text,
        "14 reached   not followed: 1 disputed, 6 inferred, 2 other   frontier 3"
    );
    assert!(partial);
}
