//! Retrieval receipts: recorded in the current (or named) run through the
//! KB write protocol, shown by `resume`, never in a closed run, and never
//! changing standard output.
use super::{fixture, kb, kb_run, one};
use crate::trial::Result;
use std::fs;
use std::path::Path;
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::run::{Checkpoint, NewRun};
use writfold_kb::write::status::SetStatus;

pub(super) fn start_run(kb: &Path) -> Result<String> {
    let new_run = Some(NewRun {
        project: "notes".into(),
        task: "Tuft receipts".into(),
        mode: None,
    });
    let note = "Start.".into();
    kb_run(
        kb,
        Verb::Write(Write::Checkpoint(Checkpoint {
            run: None,
            new_run,
            note,
        })),
    )?;
    let entry = fs::read_dir(kb.join("runs"))?.next().ok_or("no run")??;
    Ok(entry.file_name().to_string_lossy().into_owned())
}

#[test]
fn receipts_land_in_the_current_run_and_resume_shows_them() -> Result<()> {
    let f = fixture()?;
    let kb = kb(&f)?;
    let run = start_run(&kb)?;
    let (shown, notes) = one(&f, &["--kb", &kb.to_string_lossy()], &f.root)?;
    let (plain, _) = one(&f, &[], &f.root)?;
    assert_eq!(shown, plain, "recording leaves standard output as it is");
    let recorded = format!("recorded runs/{run}/retrievals/20261111T000000Z-");
    assert!(notes.iter().any(|n| n.starts_with(&recorded)), "{notes:?}");
    let resumed = kb_run(&kb, Verb::Resume { run: None })?;
    let commit = &f.commit[..12];
    assert!(
        resumed.contains(" tuft/0 ")
            && resumed.contains(&format!(
                " repo@{commit} rows=1 shown=1 cites=1 @runs/{run}/retrievals/"
            )),
        "{resumed}"
    );
    assert!(resumed.contains("  cite: src/lib.rs:3-3 @"), "{resumed}");
    let receipts = kb.join(format!("runs/{run}/retrievals"));
    let count = || fs::read_dir(&receipts).map(|d| d.count()).unwrap_or(0);
    // The KB found from the working directory, with its open current run.
    if std::env::var_os("WRITFOLD_KB").is_none() {
        let (_, notes) = one(&f, &[], &kb)?;
        assert!(notes.iter().any(|n| n.starts_with(&recorded)), "{notes:?}");
        assert_eq!(count(), 2);
    }
    let before = count();
    kb_run(
        &kb,
        Verb::Write(Write::SetStatus(SetStatus {
            target: run.clone(),
            state: "closed".into(),
            ..SetStatus::default()
        })),
    )?;
    let (quiet, notes) = one(&f, &[], &kb)?;
    assert_eq!(
        (quiet, notes.len(), count()),
        (plain, 0, before),
        "a closed run takes no receipt"
    );
    let refused = one(&f, &["--run", &run], &kb);
    assert!(refused.is_err_and(|e| e.to_string().contains("is closed")));
    let missing = one(&f, &["--kb", &f.root.to_string_lossy()], &f.root);
    assert!(missing.is_err(), "an explicit --kb that is not a KB fails");
    Ok(())
}
