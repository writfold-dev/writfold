//! An unreadable gate directory is not a complete inventory (#56): with a
//! good, met gate in the KB root and the repository's `.writfold/gates`
//! unreadable (a regular file, a dangling link, or failing part-way),
//! `kb policy` is partial, `gate status` counts the problem, `gate lines`
//! is not clean and lint names the directory. An absent directory, and
//! the same world with the directory repaired, keep today's answers.
mod support;

use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_core::inventory::Root;
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn define(dir: &Path, id: &str) -> TestResult {
    let gates = dir.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    let text = format!(
        "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 60\nscope = [\"src/ledger.rs\"]\n"
    );
    std::fs::write(gates.join(format!("{id}.toml")), text)?;
    Ok(())
}

fn gate(w: &World, verb: GateVerb) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(NOW + 60);
    w.run(&request)
}

fn status(w: &World) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.into()), Some(w.code.clone()));
    gate(w, GateVerb::Status { run, repo })
}

fn lines(w: &World) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.into()), Some(w.code.clone()));
    gate(w, GateVerb::Lines { run, repo })
}

fn policy(w: &World) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Policy);
    request.cwd = w.code.clone();
    w.run(&request)
}

fn lint(w: &World) -> TestResult<(String, u8)> {
    let verb = Verb::Lint {
        paths: Vec::new(),
        repo: Some(w.code.clone()),
        deny_warnings: false,
        records: false,
    };
    w.run(&w.request(verb))
}

/// A world with the gate `pass` defined in the KB root and met.
fn met() -> TestResult<World> {
    let w = world(&[])?;
    define(&w.kb, "pass")?;
    let (run, repo) = (Some(RUN.into()), Some(w.code.clone()));
    let (text, exit) = gate(
        &w,
        GateVerb::Run {
            id: "pass".into(),
            run,
            repo,
        },
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(w)
}

fn assert_clean(w: &World) -> TestResult {
    let (text, exit) = status(w)?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("invalid=0"), "{text}");
    let (text, exit) = lines(w)?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = policy(w)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("coverage=complete") && text.contains("gates=1"),
        "{text}"
    );
    Ok(())
}

fn assert_partial(w: &World, why: &str) -> TestResult {
    let (text, exit) = status(w)?;
    assert_ne!(exit, 0, "{why}: {text}");
    assert!(text.contains("invalid=1"), "{why}: {text}");
    let (text, exit) = lines(w)?;
    assert_eq!(exit, 2, "{why}: {text}");
    let (text, exit) = policy(w)?;
    assert_eq!(exit, 2, "{why}: {text}");
    assert!(text.contains("coverage=partial"), "{why}: {text}");
    assert!(!text.contains("none configured"), "{why}: {text}");
    let (text, exit) = lint(w)?;
    assert_eq!(exit, 2, "{why}: {text}");
    assert!(text.contains("invalid=1 errors=1"), "{why}: {text}");
    let verb = Verb::Lint {
        paths: Vec::new(),
        repo: Some(w.code.clone()),
        deny_warnings: false,
        records: false,
    };
    let output = writfold_kb::run(&w.request(verb))?;
    let codes: Vec<&str> = (output.shown.iter())
        .flat_map(|r| r.diagnostics.iter().map(|d| d.code))
        .collect();
    assert_eq!(codes, ["file.unlisted"], "{why}");
    Ok(())
}

#[test]
fn an_absent_repository_root_keeps_the_answers() -> TestResult {
    let w = met()?;
    assert!(!w.code.join(".writfold/gates").exists());
    assert_clean(&w)
}

#[test]
fn a_non_directory_root_is_partial_and_repairing_it_restores_the_answers() -> TestResult {
    let w = met()?;
    std::fs::create_dir_all(w.code.join(".writfold"))?;
    std::fs::write(w.code.join(".writfold/gates"), "not a directory\n")?;
    assert_partial(&w, "regular file")?;
    std::fs::remove_file(w.code.join(".writfold/gates"))?;
    assert_clean(&w)
}

#[test]
fn a_dangling_link_root_is_partial() -> TestResult {
    let w = met()?;
    std::fs::create_dir_all(w.code.join(".writfold"))?;
    std::os::unix::fs::symlink(w.base.join("nowhere"), w.code.join(".writfold/gates"))?;
    assert_partial(&w, "dangling link")
}

#[test]
fn an_unreadable_kb_root_is_partial_too() -> TestResult {
    // The met gate lives in the repository; the KB root's directory is the
    // one that cannot be listed.
    let w = world(&[])?;
    define(&w.code, "pass")?;
    support::git(&w.code, &["add", "-A"])?;
    support::git(&w.code, &["commit", "-q", "-m", "gate"])?;
    let (run, repo) = (Some(RUN.into()), Some(w.code.clone()));
    let (text, exit) = gate(
        &w,
        GateVerb::Run {
            id: "pass".into(),
            run,
            repo,
        },
    )?;
    assert_eq!(exit, 0, "{text}");
    assert_clean(&w)?;
    std::fs::create_dir_all(w.kb.join(".writfold"))?;
    std::fs::write(w.kb.join(".writfold/gates"), "")?;
    assert_partial(&w, "KB root")
}

/// An entry that fails part-way: the names read before it are kept, the
/// root is unreadable and one problem names it.
fn failing(dir: &Path) -> std::io::Result<Vec<std::io::Result<String>>> {
    let mut names = writfold_kb::gate::def::list(dir)?;
    names.push(Err(std::io::Error::other("injected")));
    Ok(names)
}

#[test]
fn a_mid_listing_error_keeps_what_was_read() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "pass")?;
    let found = writfold_kb::gate::def::inventory(&w.kb, Some(&w.code), failing);
    assert_eq!(found.definitions.len(), 1, "the definition read is kept");
    assert!(!found.complete());
    assert!(matches!(found.roots.first(), Some(Root::Unreadable)));
    let codes: Vec<String> = (found.problems.iter())
        .flat_map(|(_, p)| p.report.diagnostics.iter().map(|d| d.code.to_owned()))
        .collect();
    assert_eq!(codes, ["file.unlisted"], "one problem, for the KB root");
    let found =
        writfold_kb::gate::def::inventory(&w.kb, Some(&w.code), writfold_kb::gate::def::list);
    assert!(found.complete() && found.problems.is_empty());
    Ok(())
}
