//! Renderer goldens and the suggestion table.
use super::*;
use alloc::string::ToString;
use alloc::vec;

fn at(line: u32, col: u32, len: u32) -> Option<At> {
    Some(At { line, col, len })
}

fn report(file: &str, text: &str, diagnostics: Vec<Diagnostic>) -> Report {
    Report {
        sources: vec![Source {
            file: file.to_string(),
            text: text.to_string(),
        }],
        diagnostics,
    }
}

fn diag(code: &'static str, file: &str, at: Option<At>, message: &str, label: &str) -> Diagnostic {
    let mut d = Diagnostic::new(Severity::Error, code, file, message.to_string());
    d.at = at;
    d.label(label);
    d
}

const GATE: &str = ".writfold/gates/unit.toml";

#[test]
fn an_unclosed_array_and_a_misspelt_key_render_as_specified() {
    let text = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"cargo\", \"test\"\ntimeout_seconds = 600\n";
    let mut d = diag(
        "syntax.unclosed-array",
        GATE,
        at(5, 24, 0),
        "this array is never closed",
        "expected `,` or `]` here",
    );
    d.help("add `]` at the end of line 5".into());
    d.note("the array opens at 5:8, and line 6 starts the next key, `timeout_seconds`".into());
    let got = render(&report(GATE, text, vec![d]), &Style::plain());
    assert_eq!(
        got,
        "error[syntax.unclosed-array]: this array is never closed\n --> .writfold/gates/unit.toml:5:24\n  |\n5 | argv = [\"cargo\", \"test\"\n  |                        ^ expected `,` or `]` here\n  |\n  = help: add `]` at the end of line 5\n  = note: the array opens at 5:8, and line 6 starts the next key, `timeout_seconds`\n"
    );
    let text = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"cargo\"]\ntimout_seconds = 600\n";
    let mut d = diag(
        "key.unknown",
        GATE,
        at(6, 1, 14),
        "unknown key `timout_seconds` in a gate definition",
        "not a gate key",
    );
    d.help("did you mean `timeout_seconds`?".into());
    let got = render(&report(GATE, text, vec![d]), &Style::plain());
    assert_eq!(
        got,
        "error[key.unknown]: unknown key `timout_seconds` in a gate definition\n --> .writfold/gates/unit.toml:6:1\n  |\n6 | timout_seconds = 600\n  | ^^^^^^^^^^^^^^ not a gate key\n  |\n  = help: did you mean `timeout_seconds`?\n"
    );
}

#[test]
fn a_wrong_type_and_a_missing_key_render_together_with_a_tally() {
    let text = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = \"one\"\n";
    let mut a = diag(
        "key.type",
        GATE,
        at(3, 11, 5),
        "`version` must be an integer, and this is a string",
        "expected an integer from 1 to 4294967295",
    );
    a.help("write a number without quotes: version = 1".into());
    let mut b = diag(
        "key.missing",
        GATE,
        None,
        "a mechanical gate needs `timeout_seconds`",
        "",
    );
    b.help("add a line: timeout_seconds = 600   (seconds, 1 to 86400)".into());
    let mut r = report(GATE, text, vec![b, a]);
    r.sort();
    let got = render(&r, &Style::plain());
    assert_eq!(
        got,
        "error[key.type]: `version` must be an integer, and this is a string\n --> .writfold/gates/unit.toml:3:11\n  |\n3 | version = \"one\"\n  |           ^^^^^ expected an integer from 1 to 4294967295\n  |\n  = help: write a number without quotes: version = 1\n\nerror[key.missing]: a mechanical gate needs `timeout_seconds`\n --> .writfold/gates/unit.toml\n  |\n  = help: add a line: timeout_seconds = 600   (seconds, 1 to 86400)\n\n2 errors in 1 file\n"
    );
}

#[test]
fn guard_and_plan_cases_render_as_specified() {
    let file = ".writfold/guards/commit.toml";
    let text = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gates.mett\"]\n";
    let mut d = diag(
        "value.unknown",
        file,
        at(5, 12, 12),
        "unknown requirement \"gates.mett\"",
        "not a requirement",
    );
    d.help("did you mean \"gates.met\"?".into());
    d.note("a rule may require run.current, gates.met, step.met or gate:<id>".into());
    let got = render(&report(file, text, vec![d]), &Style::plain());
    assert!(
        got.contains(
            "5 | require = [\"gates.mett\"]\n  |            ^^^^^^^^^^^^ not a requirement\n"
        ),
        "{got}"
    );
    let file = ".writfold/plans/main.toml";
    let text = "schema = \"x\"\nname = \"main\"\nsteps = [\n  \"implement: unitt\",\n]\n";
    let d = diag(
        "ref.undefined",
        file,
        at(4, 15, 5),
        "step `implement` owes gate `unitt`, which is not defined",
        "no .writfold/gates/unitt.toml in the KB root or the repository",
    );
    let got = render(&report(file, text, vec![d]), &Style::plain());
    assert!(
        got.contains("4 |   \"implement: unitt\",\n  |               ^^^^^ no .writfold"),
        "{got}"
    );
    let d = diag(
        "value.form",
        file,
        at(4, 3, 16),
        "a step is written \"id: gate gate …\"",
        "no `:` after the step's ID",
    );
    let text = "schema = \"x\"\nname = \"main\"\nsteps = [\n  \"implement unit\",\n]\n";
    let got = render(&report(file, text, vec![d]), &Style::plain());
    assert!(
        got.contains("4 |   \"implement unit\",\n  |   ^^^^^^^^^^^^^^^^ no `:`"),
        "{got}"
    );
}

#[test]
fn long_lines_window_and_tabs_wide_characters_and_escapes_keep_the_caret() {
    let line = alloc::format!("k = \"{}\"", "a".repeat(300));
    let mut d = diag("value.form", "f", at(1, 150, 3), "m", "here");
    let r = report("f", &line, vec![d.clone()]);
    let style = Style {
        width: 60,
        ..Style::plain()
    };
    let got = render(&r, &style);
    let lines: Vec<&str> = got.lines().collect();
    assert!(
        lines[3].starts_with("1 | …") && lines[3].ends_with('…'),
        "{got}"
    );
    assert!(lines.iter().all(|l| l.chars().count() <= 60), "{got}");
    let caret = lines[4].find('^').unwrap_or(0);
    assert_eq!(&lines[3][caret..caret + 3], "aaa");
    d.at = at(1, 2, 1);
    let r = report("f", "\tx = 1", vec![d.clone()]);
    assert!(render(&r, &Style::plain()).contains("1 |     x = 1\n  |     ^ here"));
    fn wide(c: char) -> usize {
        if c == '界' { 2 } else { 1 }
    }
    d.at = at(1, 3, 1);
    let r = report("f", "界 x", vec![d.clone()]);
    let style = Style {
        cell: wide,
        ..Style::plain()
    };
    assert!(render(&r, &style).contains("1 | 界 x\n  |    ^ here"));
    let r = report("f", "a\u{1b}[31mb", vec![d.clone()]);
    let got = render(&r, &Style::plain());
    assert!(
        !got.contains('\u{1b}') && got.contains("a\\u{1b}[31mb"),
        "{got}"
    );
    d.at = at(1, 6, 0);
    let r = report("f", "a = [", vec![d.clone()]);
    assert!(render(&r, &Style::plain()).contains("1 | a = [\n  |      ^ here"));
    d.at = None;
    d.message = "bad \u{1b}]0;title\u{7}".into();
    let got = render(&report("f", "x", vec![d]), &Style::plain());
    assert_eq!(
        got,
        "error[value.form]: bad \\u{1b}]0;title\\u{7}\n --> f\n"
    );
}

#[test]
fn coloured_output_is_plain_output_with_escapes() {
    let mut d = diag(
        "key.unknown",
        GATE,
        at(1, 1, 3),
        "unknown key `abc`",
        "not a key",
    );
    d.help("did you mean `abd`?".into());
    d.note("n".into());
    let mut w = d.clone();
    w.severity = Severity::Warning;
    let r = report(GATE, "abc = 1\n", vec![d, w]);
    let plain = render(&r, &Style::plain());
    let coloured = render(
        &r,
        &Style {
            colour: true,
            ..Style::plain()
        },
    );
    assert_ne!(plain, coloured);
    let mut stripped = String::new();
    let mut chars = coloured.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            stripped.push(c);
        }
    }
    assert_eq!(stripped, plain);
}

#[test]
fn caps_count_what_they_cut() {
    let ds: Vec<Diagnostic> = (1..=25)
        .map(|n| diag("key.twice", "f", at(n, 1, 1), "m", ""))
        .collect();
    let text = "x\n".repeat(25);
    let got = render(&report("f", &text, ds.clone()), &Style::plain());
    assert!(
        got.contains("… and 5 more in this file") && got.ends_with("25 errors in 1 file\n"),
        "{got}"
    );
    let got = render_capped(&report("f", &text, ds), &Style::plain(), 20, 3);
    assert!(got.contains("… and 22 more\n"), "{got}");
    assert_eq!(got.matches("error[").count(), 3);
}
