//! The definition reader with spans: recovery at line boundaries, one
//! diagnostic per fault, and the invariants every reading keeps on
//! generated files (valid exemplars with random byte edits).
use writfold_core::diag::{LineIndex, Report};
use writfold_kb::defs::{Check, Field, Sink, Slot, toml};

const GATE: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"cargo\", \"test\"\ntimeout_seconds = 600\nscope = [\"src/**\"]\n";

fn read(text: &str) -> (toml::Doc, Report) {
    let mut sink = Sink::new("f.toml", text);
    let doc = toml::read(text, &mut sink);
    (doc, sink.report())
}

#[test]
fn an_unclosed_array_is_reported_once_and_the_keys_after_it_are_read() {
    let (doc, report) = read(GATE);
    assert_eq!(report.diagnostics.len(), 1, "{report:?}");
    let d = &report.diagnostics[0];
    assert_eq!(d.code, "syntax.unclosed-array");
    let at = d.at.map(|a| (a.line, a.col));
    assert_eq!(at, Some((5, 24)));
    assert_eq!(d.help.as_deref(), Some("add `]` at the end of line 5"));
    assert_eq!(
        d.notes,
        ["the array opens at 5:8, and line 6 starts the next key, `timeout_seconds`"]
    );
    assert_eq!(doc.get("argv").map(|e| &e.value), Some(&Slot::Broken));
    assert!(doc.has("timeout_seconds") && doc.has("scope"));
    let (_, report) = read("argv = [\"a\",\n  \"b\"\n");
    let note = &report.diagnostics[0].notes[0];
    assert!(note.ends_with("the file ends before it closes"), "{note}");
}

#[test]
fn a_broken_value_is_silent_afterwards_and_two_faults_give_two() {
    let text = "id = \"open\nversion = true\nclass = \"x\"\n";
    let mut check = Check::read("f.toml", text, "a gate definition");
    assert!(matches!(check.text("id"), Field::Bad));
    check.need_text("schema", "schema = \"writfold-gate/1\"");
    let report = check.sink.report();
    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, ["syntax.unclosed-string", "syntax.subset"]);
    let lines: Vec<u32> = report
        .diagnostics
        .iter()
        .filter_map(|d| d.at.map(|a| a.line))
        .collect();
    assert_eq!(lines, [1, 2]);
}

#[test]
fn faults_of_the_subset_each_have_their_code() {
    for (text, code) in [
        ("[gate]\n", "syntax.subset"),
        ("a.b = 1\n", "syntax.subset"),
        ("\"a\" = 1\n", "syntax.subset"),
        ("a = 1.5\n", "syntax.subset"),
        ("a = 01\n", "syntax.expected"),
        ("a = \"\"\"x\"\"\"\n", "syntax.subset"),
        ("a = [1, 2]\n", "syntax.subset"),
        ("a = { b = 1 }\n", "syntax.subset"),
        ("a = \"x\" b\n", "syntax.expected"),
        ("a = 1\na = 2\n", "key.twice"),
        ("a = \"\\q\"\n", "syntax.escape"),
        ("a\n", "syntax.expected"),
        ("a = mechanical\n", "syntax.expected"),
        ("a = 99999999999999999999\n", "key.range"),
        ("a = [\"x\" \"y\"]\n", "syntax.expected"),
    ] {
        let (_, report) = read(text);
        let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [code], "{text:?}");
    }
    let (doc, report) = read("[gate]\nid = \"x\"\n");
    assert!(doc.has("id") && report.diagnostics.len() == 1);
}

/// A small deterministic generator: the same edits on every run.
pub struct Rng(pub u64);

impl Rng {
    pub fn step(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.step() % u64::try_from(n.max(1)).unwrap_or(1)).unwrap_or(0)
    }
}

/// `text` with a few random byte edits, kept only when still UTF-8.
pub fn mutate(rng: &mut Rng, text: &str) -> String {
    let alphabet = b"\"'[]=,#\\\n\r\t {}.-_aZ09\x1b";
    let mut bytes = text.as_bytes().to_vec();
    for _ in 0..=rng.below(3) {
        let at = rng.below(bytes.len() + 1);
        let byte = alphabet[rng.below(alphabet.len())];
        match rng.below(3) {
            0 if at < bytes.len() => bytes[at] = byte,
            1 if at < bytes.len() => {
                bytes.remove(at);
            }
            _ => bytes.insert(at, byte),
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| text.to_owned())
}

/// The invariants of any report about `text`.
pub fn holds(text: &str, report: &Report) {
    let index = LineIndex::new(text);
    let lines: Vec<&str> = text.split('\n').collect();
    for d in &report.diagnostics {
        let Some(at) = d.at else { continue };
        assert!(
            at.line >= 1 && (at.line as usize) <= lines.len(),
            "{d:?} in {text:?}"
        );
        let line = index.line(text, at.line);
        assert!(
            (at.col as usize) <= line.chars().count() + 1,
            "{d:?} in {text:?}"
        );
    }
    let mut sorted = report.clone();
    sorted.sort();
    assert_eq!(&sorted, report, "unsorted or duplicated: {text:?}");
}

#[test]
fn reading_generated_files_never_panics_and_keeps_its_invariants() {
    let mut rng = Rng(0x5eed_1234_abcd_0001);
    let exemplars = [
        GATE,
        "schema = 'x'\nsteps = [\n  \"a: b\", # c\n  \"d:\",\n]\n",
        "a = \"\\u00e9\\t\"\nb = -1_000\n",
    ];
    for n in 0..4000 {
        let text = mutate(&mut rng, exemplars[n % exemplars.len()]);
        let (_, report) = read(&text);
        holds(&text, &report);
    }
}
