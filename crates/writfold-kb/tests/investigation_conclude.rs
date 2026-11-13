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
        Err(error) if error.exit_code() == 3 => error.to_string(),
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
        message.contains("Checked against") && message.contains("Scope and limits"),
        "{message}"
    );
    append(
        &w,
        "\n## Hypotheses\n\n- H1: supported, the volume was slow again\n- H2: open, a second writer\n",
    )?;
    append(&w, "\n## Checked against\n\nfsync traces from 2026-06-01\n")?;
    append(&w, "\n## Scope and limits\n\nOne host only.\n")?;
    let message = refusal(&w)?;
    assert!(message.contains("1 open hypotheses"), "{message}");
    assert!(!message.contains("Checked against"), "{message}");
    let path = w.kb.join(PATH);
    let body = std::fs::read_to_string(&path)?;
    std::fs::write(&path, body.replace("- H2: open,", "- H2 refuted:"))?;
    assert!(refusal(&w)?.contains("unreadable hypothesis"));
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
