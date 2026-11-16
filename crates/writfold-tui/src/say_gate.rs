//! Gates and checklist items in plain words, for the inspector and the
//! Gates view; the Gates count line and the `not met` badge. A standing is
//! a lookup on `status`; the gate's id and its remainder come from the
//! `named` shape, and a remainder is shown only when it reads as words.
use crate::cells::{self, Glyphs};
use crate::lines::{self, Tone, more};
use crate::say::{Phrase, Piece, Told, align, quote, said};
use crate::say_shape::{Class, class, named, optional};
use crate::say_words::{CLASS, ITEM, STANDING, long, short};
use crate::source::{Reply, field};
use serde_json::Value;

/// Items whose standing is settled: the inspector leaves them to `d`.
const COVERED: [&str; 3] = ["supported", "not-applicable", "attested"];

/// The class in words: `checklist · 2 of 4 required items covered`.
pub fn class_words(class: Class) -> Vec<Piece> {
    match class {
        Class::Checklist { covered, required } => vec![
            said("checklist"),
            said(format!("{covered} of {required} required items covered")),
        ],
        Class::Mechanical => vec![short(CLASS, "mechanical")],
        Class::Attested => vec![short(CLASS, "attested")],
    }
}

/// One item: mark and id, then `optional`, the standing and either the
/// requirement (an open item's authored text) or a readable account.
pub fn item(value: &Value, indent: u16, glyphs: Glyphs) -> Option<Phrase> {
    let status = field(value, "status");
    let (id, rest) = named(value)?;
    let (optional, rest) = optional(rest).map_or((false, rest), |r| (true, r));
    let mut pieces = vec![
        Piece::Mark(cells::segment(status, glyphs)),
        Piece::Quoted(id.to_owned()),
    ];
    if optional {
        pieces.push(said("optional"));
    }
    // An out-of-date item's account names what moved; its long form says
    // so in words and stands in for it.
    pieces.push(if status == "stale" {
        long(ITEM, status)
    } else {
        short(ITEM, status)
    });
    let text = if status == "open" {
        (!rest.is_empty()).then(|| Piece::Quoted(rest.to_owned()))
    } else {
        quote(rest)
    };
    pieces.extend(text);
    let tone = if optional && lines::tone(value) == Tone::Warn {
        Tone::Dim
    } else {
        lines::tone(value)
    };
    Some(Phrase::led(indent, tone, pieces, 2))
}

/// A step's gate continuations for the inspector: each gate's standing,
/// its class, then the items still open or out of date.
pub fn step_gates(row: &Value, glyphs: Glyphs) -> Told {
    let mut told = Told::default();
    let all: Vec<&Value> = more(row).collect();
    for (at, gate) in all.iter().enumerate() {
        if field(gate, "label") != "gate" {
            continue;
        }
        let Some((id, rest)) = named(gate) else {
            told.withheld += 1;
            continue;
        };
        told.lines.push(Phrase::blank());
        let status = field(gate, "status");
        let pieces = vec![Piece::Quoted(id.to_owned()), short(STANDING, status)];
        told.lines
            .push(Phrase::led(0, lines::tone(gate), pieces, 1));
        if let Some((class, _)) = class(rest) {
            told.lines
                .push(Phrase::new(2, Tone::Dim, class_words(class)));
        }
        let reasons = all[at + 1..]
            .iter()
            .take_while(|v| field(v, "label") == "reason");
        told.lines
            .extend(reasons.map(|v| Phrase::new(2, Tone::Bad, vec![said(field(v, "text"))])));
        let mut items: Vec<Phrase> = all[at + 1..]
            .iter()
            .take_while(|v| field(v, "label") == "item")
            .filter(|v| !COVERED.contains(&field(v, "status")))
            .filter_map(|v| item(v, 2, glyphs))
            .collect();
        align(&mut items);
        told.lines.extend(items);
    }
    told
}

/// A gate row of `gate status`: mark, id, standing, then the reason when
/// it reads as words, else the class (a passed gate needs neither).
pub fn gate_row(row: &Value, glyphs: Glyphs) -> Vec<Phrase> {
    let status = field(row, "status");
    let mut pieces = vec![
        Piece::Mark(cells::chip(status, glyphs)),
        Piece::Quoted(field(row, "id").to_owned()),
        short(STANDING, status),
    ];
    let why = more(row)
        .find(|m| field(m, "label") == "why")
        .and_then(|m| quote(field(m, "text")));
    let class = class(field(row, "text")).map(|(c, _)| c);
    match (status, why, class) {
        ("satisfied", _, _) => {}
        (_, Some(why), _) => pieces.push(why),
        (_, None, Some(class)) => pieces.extend(class_words(class)),
        _ => {}
    }
    let mut out = vec![Phrase::led(0, lines::tone(row), pieces, 2)];
    out.extend(
        more(row)
            .filter(|m| field(m, "label") == "item")
            .filter_map(|m| item(m, 4, glyphs)),
    );
    out
}

const COUNTS: [(&str, &str); 8] = [
    ("satisfied", "passed"),
    ("stale", "out of date"),
    ("open", "with nothing recorded yet"),
    ("failed", "failed"),
    ("unknown", "with no verdict"),
    ("attested", "signed off"),
    ("waived", "waived"),
    ("invalid", "invalid"),
];

/// Gates that are not satisfied, attested or waived; `None` when a count
/// it needs is missing.
pub fn not_met(answer: &Reply) -> Option<u64> {
    let gates = answer.count("gates")?;
    let met = ["satisfied", "attested", "waived"]
        .iter()
        .map(|n| answer.count(n))
        .sum::<Option<u64>>()?;
    Some(gates.saturating_sub(met))
}

/// The badge's tone: bad when a gate is out of date, failed or invalid.
pub fn badge_tone(answer: &Reply) -> Tone {
    let bad = ["stale", "failed", "invalid"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|v| v > 0));
    if bad { Tone::Bad } else { Tone::Warn }
}

/// `6 gates · 2 passed · 2 out of date · … · 4 not met`, non-zero only.
pub fn counts(answer: &Reply) -> Phrase {
    let mut pieces = Vec::new();
    if let Some(gates) = answer.count("gates") {
        let s = if gates == 1 { "" } else { "s" };
        pieces.push(said(format!("{gates} gate{s}")));
    }
    for (name, word) in COUNTS {
        if let Some(n) = answer.count(name).filter(|n| *n > 0) {
            pieces.push(said(format!("{n} {word}")));
        }
    }
    if let Some(n) = not_met(answer).filter(|n| *n > 0) {
        pieces.push(said(format!("{n} not met")));
    }
    pieces.extend(crate::say_diag::count(answer));
    let tone = match not_met(answer) {
        Some(0) => Tone::Good,
        _ => badge_tone(answer),
    };
    Phrase::new(0, tone, pieces)
}

/// The whole Gates view at the plain level.
pub fn view(answer: &Reply, glyphs: Glyphs) -> Told {
    let mut told = Told::default();
    if answer.trailer_text("reason").as_deref() == Some("no-gates") {
        told.lines.push(Phrase::new(
            0,
            Tone::Dim,
            vec![said("No gates are defined.")],
        ));
        return told;
    }
    told.lines.push(counts(answer));
    told.lines.push(Phrase::blank());
    let mut rows: Vec<Phrase> = answer
        .rows
        .iter()
        .filter(|r| field(r, "kind") == "gate")
        .flat_map(|r| gate_row(r, glyphs))
        .collect();
    align(&mut rows);
    told.lines.extend(rows);
    told.lines.extend(crate::say_diag::entries(answer, glyphs));
    told.lines.extend(crate::say_diag::blocks(answer));
    let standing = "Standings are worked out from the run's recorded results each time this view asks; nothing is run.";
    crate::say_row::notes(&mut told, answer, &[("receipts of run ", standing)]);
    told
}
