//! `status <investigation> --set concluded`: refused (exit 3, nothing
//! written) until the record has `Checked against` and `Scope and limits`
//! text and no open hypothesis; then one in-place change of `status` and
//! `updated`, with the index equal to a rebuild.
mod support;

use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::status::SetStatus;

const ID: &str = "2026/2026-06-01-flush-stalls-again";
const PATH: &str = "investigations/2026/2026-06-01-flush-stalls-again/README.md";

fn conclude(w: &World) -> Result<(String, u8), writfold_kb::error::KbError> {
    writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(SetStatus {
        target: ID.into(),
        state: "concluded".into(),
        ..SetStatus::default()
    }))))
    .map(|o| (o.text, o.exit))
}

fn refusal(w: &World) -> TestResult<String> {
    let before = std::fs::read(w.kb.join(PATH))?;
    let message = match conclude(w) {
        Err(writfold_kb::error::KbError::Definition(report)) => report
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", writfold_kb::diag::line(d), d.code))
            .collect::<Vec<_>>()
            .join("\n"),
        other => return Err(format!("expected a refusal, got {other:?}").into()),
    };
    assert_eq!(
        std::fs::read(w.kb.join(PATH))?,
        before,
        "a refusal writes nothing"
    );
    Ok(message)
}

fn append(w: &World, text: &str) -> TestResult {
    let path = w.kb.join(PATH);
    let mut body = std::fs::read_to_string(&path)?;
    body.push_str(text);
    std::fs::write(path, body)?;
    Ok(())
}

#[test]
fn concluded_needs_the_write_up() -> TestResult {
    let w = world(&[])?;
    let message = refusal(&w)?;
    assert!(
        message.contains("concluding needs a ## Checked against section; add a section")
            && message.contains("concluding needs a ## Scope and limits section")
            && message.contains("section.missing"),
        "{message}"
    );
    append(
        &w,
        "\n## Hypotheses\n\n- H1: supported, the volume was slow again\n- H2: open, a second writer\n",
    )?;
    append(&w, "\n## Checked against\n\nfsync traces from 2026-06-01\n")?;
    append(&w, "\n## Scope and limits\n\nOne host only.\n")?;
    let message = refusal(&w)?;
    let open = format!("{PATH}:19:1: this hypothesis is still open; write its outcome");
    assert!(message.contains(&open), "{message}");
    assert!(!message.contains("Checked against"), "{message}");
    let path = w.kb.join(PATH);
    let body = std::fs::read_to_string(&path)?;
    std::fs::write(&path, body.replace("- H2: open,", "- H2 refuted:"))?;
    let message = refusal(&w)?;
    assert!(
        message.contains("a hypothesis without an outcome"),
        "{message}"
    );
    let body = std::fs::read_to_string(&path)?;
    std::fs::write(&path, body.replace("- H2 refuted:", "- H2: refuted,"))?;
    let before = std::fs::read_to_string(&path)?;
    let (text, exit) = conclude(&w)?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(
        std::fs::read_to_string(&path)?,
        before
            .replace("status: open", "status: concluded")
            .replace("updated: 2026-06-02", "updated: 2026-11-11")
    );
    let (text, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !text.contains("[differs]"), "{text}");
    Ok(())
}

#[test]
fn a_misspelt_heading_is_named_with_the_one_it_should_be() -> TestResult {
    let w = world(&[])?;
    append(&w, "\n## Hypotheses\n\n- H1: supported\n")?;
    append(&w, "\n## Checked againts\n\nfsync traces\n")?;
    append(&w, "\n## Scope and limits\n\nOne host only.\n")?;
    let message = refusal(&w)?;
    assert!(
        message.contains(
            "concluding needs a ## Checked against section; did you mean \"## Checked against\"?"
        ),
        "{message}"
    );
    assert!(!message.contains("Scope and limits"), "{message}");
    Ok(())
}
