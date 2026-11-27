//! #181 (RFC 2026-11-27-run-directory-accounting, law U): `resume` and
//! `resume --packet` account for every file of the run. Another tool's
//! files are named, counted `unread` and make the answer partial; they
//! never leave a packet saying `complete`.
mod support;

use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn put(w: &World, path: &str, text: &str) -> TestResult {
    let file = w.kb.join(format!("runs/{RUN}/{path}"));
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    Ok(())
}

fn resume(w: &World) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    request.now = Some(NOW + 120);
    w.run(&request)
}

fn packet(w: &World, budget: usize) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Packet {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    request.now = Some(NOW + 120);
    request.budget = budget;
    w.run(&request)
}

#[test]
fn another_tools_files_are_counted_and_never_complete() -> TestResult {
    let w = world(&[])?;
    let (clean, code) = packet(&w, 4000)?;
    assert!(clean.contains("packet=complete"), "{clean}");
    assert_eq!(code, 0, "{clean}");
    for name in ["a.yaml", "b.yaml", "c.yaml"] {
        put(&w, &format!("observations/{name}"), "x\n")?;
    }
    put(&w, "strange.bin", "x")?;
    put(&w, ".hidden", "x")?;
    let (text, code) = resume(&w)?;
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("unread=4"), "{text}");
    assert!(text.contains("coverage=partial"), "{text}");
    assert!(
        text.contains("4 files in runs/run-20261101T090000Z-a1b2c3/ are in no format Writfold reads: observations/ (3), strange.bin"),
        "{text}"
    );
    let (text, code) = packet(&w, 4000)?;
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("packet=partial") && text.contains("unread=4"),
        "{text}"
    );
    // The unread row is required: too small a budget says so, never drops it.
    let (text, _) = packet(&w, 1)?;
    assert!(
        text.contains("reason=budget") || text.contains("answer=unknown"),
        "{text}"
    );
    Ok(())
}

/// For each directory Writfold reads, a file of an unknown kind there is
/// counted, never silent.
#[test]
fn a_foreign_file_in_a_known_directory_is_counted() -> TestResult {
    for dir in [
        "gates",
        "statements",
        "covers",
        "observed",
        "plan",
        "policy",
        "artifacts",
        "invocations",
        "evidence",
        "deliveries",
        "deliveries/c1/checks",
    ] {
        let w = world(&[])?;
        put(&w, &format!("{dir}/foreign.bin"), "x")?;
        let (text, code) = resume(&w)?;
        assert_eq!(code, 2, "{dir}: {text}");
        assert!(text.contains("unread=1"), "{dir}: {text}");
    }
    Ok(())
}

#[test]
fn a_walk_cut_short_or_an_unlistable_directory_reads_partial() -> TestResult {
    let w = world(&[])?;
    let many = w.kb.join(format!("runs/{RUN}/observations"));
    std::fs::create_dir_all(&many)?;
    for i in 0..20_001 {
        std::fs::write(many.join(format!("{i}.txt")), "")?;
    }
    let (text, code) = resume(&w)?;
    assert_eq!(code, 2, "{}", &text[..text.len().min(400)]);
    assert!(text.contains("coverage=partial"));

    let w = world(&[])?;
    let shut = w.kb.join(format!("runs/{RUN}/gates/shut"));
    std::fs::create_dir_all(&shut)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o000))?;
    let listable = std::fs::read_dir(&shut).is_ok();
    let (text, code) = resume(&w)?;
    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o755))?;
    // A superuser lists it anyway: then it is an empty directory.
    if !listable {
        assert_eq!(code, 2, "{text}");
        assert!(text.contains("could not be listed"), "{text}");
    }
    Ok(())
}
