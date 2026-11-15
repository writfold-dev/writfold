//! The empty KB, and that visiting every KB view writes nothing and starts
//! no checker: the work tree, local state and the `cite_checks` rows are
//! equal before and after.
mod support;

use support::kb_app::{kb_app, press};
use support::kb_world::atlas;
use support::projection::{git_status, render, rows};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key};

#[test]
fn empty_kb_mode() -> TestResult {
    let w = support::map::nested("t5-empty")?;
    for dir in [
        "proposals",
        "ktlo",
        "investigations",
        "inventory",
        "signatures",
        "runbooks",
        "lantern",
        "corrections",
    ] {
        std::fs::remove_dir_all(w.kb.join(dir))?;
    }
    // The KB layout requires the assets directory, even when it is empty.
    std::fs::create_dir_all(w.kb.join("inventory/assets"))?;
    std::fs::write(
        w.kb.join("harbor/STATUS.md"),
        "---\nphase: build\n---\n\n# Harbor status\n",
    )?;
    let mut app = App::new(w.source(), Some(RUN));
    app.key(Key::Char('m'));
    let text = rows(&render(&mut app, 136, 38)?).join("\n");
    assert!(text.contains("harbor names no decisions"), "{text}");
    assert!(!text.contains("stale !"), "{text}");
    press(&mut app, "3");
    let text = rows(&render(&mut app, 136, 38)?).join("\n");
    assert!(
        text.contains("nothing tracked within 2 links of harbor"),
        "{text}"
    );
    assert!(!app.key(Key::Char('q')));
    Ok(())
}

fn cached_rows(w: &support::World) -> TestResult<Vec<u8>> {
    let script = "import pathlib,sqlite3,sys,json\n\
        files=list(pathlib.Path(sys.argv[1]).rglob('kb-index-*.sqlite'))\n\
        assert len(files)==1, files\n\
        db=sqlite3.connect('file:'+str(files[0])+'?mode=ro',uri=True)\n\
        print(json.dumps(db.execute('SELECT * FROM cite_checks ORDER BY path,ord').fetchall()))";
    let out = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(w.kb.join(".git"))
        .output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(out.stdout)
}

#[test]
fn kb_mode_starts_no_process_and_writes_nothing() -> TestResult {
    let w = atlas("writes")?;
    // The cited code goes back to its first commit, so a full `stale`
    // would cache new (fresh) verdicts; `--cached` writes none.
    support::git(
        &w.base.join(support::kb_world::CODE),
        &["reset", "-q", "--hard", "HEAD~1"],
    )?;
    let mut app = kb_app(&w);
    render(&mut app, 136, 38)?;
    let (status, cache) = (git_status(&w)?, cached_rows(&w)?);
    let local = support::projection::local_snapshot(&w)?;
    for keys in [
        "1", "jjl", "d", "2", "3", "a", "v", "v", "v", "4", "5", "a", "]", "1", "[", "d", "f", "m",
    ] {
        press(&mut app, keys);
        app.focus = Focus::Detail;
        render(&mut app, 136, 38)?;
    }
    app.key(Key::Char('r'));
    render(&mut app, 136, 38)?;
    assert_eq!(git_status(&w)?, status);
    assert_eq!(cached_rows(&w)?, cache);
    assert_eq!(support::projection::local_snapshot(&w)?, local);
    Ok(())
}
