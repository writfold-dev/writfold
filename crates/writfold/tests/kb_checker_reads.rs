//! Who runs external checkers: only `writfold kb stale`. Gate grounding
//! (`gate cover`, `gate status`), the hooks and `--offline` read the
//! verdict `kb stale` cached and never start the checker (counted by the
//! fake checker's counter file). And `cite make --kind`: a checked kind's
//! token from its checker, refused when the checker writes an undeclared
//! field.
mod checker;

use checker::{RUN, Result, kb, run, world};

const TOKEN: &str = "⟦cite kind=src depot=main path=lib/fresh.c L=3-9 want=fresh⟧";

fn cover(w: &checker::Root) -> Result<(String, i32)> {
    let repo = w.kb().display().to_string();
    let args = [
        "gate",
        "cover",
        "review",
        "src",
        "supported",
        TOKEN,
        "--by",
        "agent",
        "--run",
        RUN,
        "--repo",
        &repo,
    ];
    kb(w, &args, &[])
}

#[test]
fn only_kb_stale_runs_a_checker() -> Result {
    let w = world("reads", &[TOKEN, TOKEN], true)?;
    // Not yet checked: grounding reads the cache, finds nothing, and the
    // reference is not grounded.
    let (text, exit) = cover(&w)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("(unverifiable)"), "{text}");
    assert_eq!(w.spawns(), 0);
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!((exit, w.spawns()), (0, 1), "{text}");
    let (text, exit) = cover(&w)?;
    assert_eq!(exit, 0, "{text}");
    let repo = w.kb().display().to_string();
    let status = ["gate", "status", "--run", RUN, "--repo", &repo];
    let (text, _) = kb(&w, &status, &[])?;
    assert!(text.starts_with("gate review [attested]"), "{text}");
    let start = format!(
        "{{\"session_id\": \"s\", \"cwd\": \"{}\"}}",
        w.kb().display()
    );
    for event in ["session-start", "stop", "session-end"] {
        let out = run(&w, &["kb", "hook", event], &[], &start)?;
        assert_eq!(out.status.code(), Some(0), "{event}");
    }
    kb(&w, &["stale", "--offline"], &[])?;
    kb(&w, &["why", "sources"], &[])?;
    assert_eq!(w.spawns(), 1, "only kb stale starts the checker");
    kb(&w, &["stale"], &[])?;
    assert_eq!(w.spawns(), 2);
    Ok(())
}

fn cached_rows(w: &checker::Root) -> Result<Vec<u8>> {
    let script = "import pathlib,sqlite3,sys,json\n\
        files=list(pathlib.Path(sys.argv[1]).rglob('kb-index-*.sqlite'))\n\
        assert len(files)==1, files\n\
        db=sqlite3.connect('file:'+str(files[0])+'?mode=ro',uri=True)\n\
        print(json.dumps(db.execute('SELECT * FROM cite_checks ORDER BY path,ord').fetchall()))";
    let out = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(w.kb().join(".git"))
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(out.stdout)
}

#[test]
fn cached_stale_never_starts_a_checker_or_changes_its_cache() -> Result {
    let w = world("cached", &[TOKEN], true)?;
    kb(&w, &["index", "status"], &[])?;
    let empty = cached_rows(&w)?;
    let (text, exit) = kb(&w, &["stale", "--cached"], &[])?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("not-checked"), "{text}");
    assert!(text.contains("cached: no checker was started"), "{text}");
    assert_eq!(w.spawns(), 0);
    assert_eq!(cached_rows(&w)?, empty);
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!((exit, w.spawns()), (0, 1), "{text}");
    let checked = cached_rows(&w)?;
    assert_ne!(checked, empty);
    let (text, exit) = kb(&w, &["stale", "--cached"], &[])?;
    assert_eq!((exit, w.spawns()), (0, 1), "{text}");
    assert_eq!(cached_rows(&w)?, checked);
    let (text, exit) = kb(&w, &["stale", "--cached", "--offline"], &[])?;
    assert_eq!(exit, 1, "{text}");
    assert!(text.contains("not both"), "{text}");
    Ok(())
}

#[test]
fn session_start_names_an_absent_checker_without_running_anything() -> Result {
    let w = world("start", &[TOKEN], false)?;
    let start = format!(
        "{{\"session_id\": \"s\", \"cwd\": \"{}\"}}",
        w.kb().display()
    );
    let out = run(&w, &["kb", "hook", "session-start"], &[], &start)?;
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Checkers not on PATH: writfold-check-acme-src (kind src)"),
        "{text}"
    );
    Ok(())
}

#[test]
fn make_round_trips_and_undeclared_fields_are_refused() -> Result {
    let w = world("make", &[], true)?;
    let make = [
        "cite",
        "make",
        "--kind",
        "src",
        "depot=main",
        "path=lib/made.c",
        "L=3-9",
    ];
    let (text, exit) = kb(&w, &make, &[])?;
    assert_eq!(exit, 0, "{text}");
    let token = text
        .lines()
        .find_map(|l| l.split_once(' ').map(|(_, rest)| rest))
        .and_then(|rest| rest.find('⟦').map(|at| &rest[at..]))
        .and_then(|rest| rest.find('⟧').map(|end| &rest[..end + '⟧'.len_utf8()]))
        .ok_or(text.clone())?;
    assert!(
        token.starts_with(
            "⟦cite kind=src depot=main rev=r42 path=lib/made.c L=3-9 digest=d0 v=1 seen="
        ),
        "{token}"
    );
    let notes = w.kb().join("notes/sources.md");
    std::fs::write(&notes, format!("# Sources\n\n{token}\n"))?;
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("checked=1 stale=0 unverifiable=0"), "{text}");
    let (text, exit) = kb(&w, &make, &[("FAKE_MODE", "extra")])?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("returned field owner, which .writfold/sources/src.toml does not list"),
        "{text}"
    );
    let (text, exit) = kb(&w, &["cite", "make", "--kind", "web", "url=x"], &[])?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("not declared freshness = \"checked\""),
        "{text}"
    );
    Ok(())
}
