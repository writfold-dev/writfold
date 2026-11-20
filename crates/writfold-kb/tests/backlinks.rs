//! Backlinks (stores-next spec S3): `context` and `uses` end with the
//! store directories whose `README.md` names the record in `subject:`, read
//! at the store's target commit and judged by the proved
//! `writfold_core::store::claim`.
mod support;

use std::path::{Path, PathBuf};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;

/// A store whose origin is `<base>/scratch`, cloned to `<base>/scratch-clone`
/// and declared through a path hint. Its directories claim: `harbor-api` by
/// ID, `hapi` (an alias of `harbor-api` only) with an unknown subject,
/// `ledger` (an alias two assets share), and nothing.
fn stored() -> TestResult<(World, PathBuf, PathBuf)> {
    let w = world(&[])?;
    let origin = w.base.join("scratch");
    for (dir, text) in [
        (
            "work/2026-11-18-a",
            "---\nsubject: harbor-api\n---\n# Review A\n",
        ),
        (
            "work/2026-11-19-b",
            "---\nsubject: [hapi, no-such-record]\n---\n# Probe B\n",
        ),
        ("work/c", "---\nsubject: ledger\n---\n# Shared alias\n"),
        ("work/d", "# No claim\n"),
    ] {
        std::fs::create_dir_all(origin.join(dir))?;
        std::fs::write(origin.join(dir).join("README.md"), text)?;
    }
    git(&origin, &["init", "-q", "-b", "main"])?;
    git(&origin, &["config", "user.email", "t@example.invalid"])?;
    git(&origin, &["config", "user.name", "t"])?;
    git(&origin, &["add", "-A"])?;
    git(&origin, &["commit", "-q", "-m", "material"])?;
    let clone = w.base.join("scratch-clone");
    let url = format!("file://{}", origin.display());
    git(
        &w.base,
        &["clone", "-q", &url, &clone.display().to_string()],
    )?;
    git(&clone, &["config", "user.email", "t@example.invalid"])?;
    git(&clone, &["config", "user.name", "t"])?;
    std::fs::create_dir_all(w.kb.join(".writfold/stores"))?;
    std::fs::write(
        w.kb.join(".writfold/stores/scratch.toml"),
        format!(
            "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"{url}\"\nrole = \"attached\"\npaths = [\"{}\"]\n",
            clone.display()
        ),
    )?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "store"])?;
    Ok((w, origin, clone))
}

fn context(w: &World, target: &str) -> TestResult<String> {
    let verb = Verb::Context {
        target: target.into(),
        depth: 1,
    };
    Ok(w.run(&w.request(verb))?.0)
}

fn add(repo: &Path, dir: &str, text: &str) -> TestResult {
    std::fs::create_dir_all(repo.join(dir))?;
    std::fs::write(repo.join(dir).join("README.md"), text)?;
    git(repo, &["add", "-A"])?;
    git(repo, &["commit", "-q", "-m", dir])?;
    Ok(())
}

#[test]
fn a_record_lists_the_directories_that_name_it_by_id_or_by_its_own_alias() -> TestResult {
    let (w, _, clone) = stored()?;
    let commit = git(&clone, &["rev-parse", "HEAD"])?;
    let text = context(&w, "harbor-api")?;
    let b = text.find("material scratch:work/2026-11-19-b Probe B !store-claim");
    let a = text.find("material scratch:work/2026-11-18-a Review A !store-claim");
    assert!(matches!((b, a), (Some(b), Some(a)) if b < a), "{text}");
    assert!(
        text.contains(&format!("at: scratch@{commit}:work/2026-11-18-a/README.md")),
        "{text}"
    );
    assert!(text.contains("via: subject: hapi"), "{text}");
    assert!(text.contains("material=2"), "{text}");
    // An alias two records share lists the directory under neither.
    for shared in ["harbor-db", "lantern-ui"] {
        let text = context(&w, shared)?;
        assert!(!text.contains("scratch:work/c"), "{text}");
        assert!(text.contains("material=0"), "{text}");
    }
    // A material row is the store's claim, not a record of the KB.
    let text = context(&w, "scratch:work/2026-11-18-a")?;
    assert!(!text.contains("material scratch:"), "{text}");
    let uses = w.run(&w.request(Verb::Uses {
        target: "harbor-api".into(),
    }))?;
    assert!(
        uses.0.contains("material scratch:work/2026-11-18-a"),
        "{}",
        uses.0
    );
    Ok(())
}

#[test]
fn json_rows_and_the_stores_trailer() -> TestResult {
    let (w, _, _) = stored()?;
    let mut request = w.request(Verb::Context {
        target: "harbor-api".into(),
        depth: 1,
    });
    request.json = true;
    let (text, _) = w.run(&request)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let rows = value["rows"].as_array().cloned().unwrap_or_default();
    let material: Vec<_> = rows.iter().filter(|r| r["kind"] == "material").collect();
    assert_eq!(material.len(), 2, "{text}");
    assert_eq!(material[1]["store"], "scratch");
    assert_eq!(material[1]["dir"], "work/2026-11-18-a");
    assert_eq!(material[1]["subject"], "harbor-api");
    assert_eq!(material[1]["claim"], "store");
    assert!(material[1]["path"].is_null(), "{text}");
    let stores = &value["trailer"]["stores"];
    assert_eq!(stores[0]["name"], "scratch", "{text}");
    assert_eq!(stores[0]["state"], "read", "{text}");
    assert_eq!(stores[0]["via"], "hint", "{text}");
    Ok(())
}

#[test]
fn repos_counts_claims_and_names_the_ones_listed_nowhere() -> TestResult {
    let (w, _, _) = stored()?;
    let (text, _) = w.run(&w.request(Verb::Repos { missing: false }))?;
    assert!(text.contains("material=2 ambiguous=1 unknown=1"), "{text}");
    assert!(
        text.contains("ambiguous scratch:work/c (subject: ledger)"),
        "{text}"
    );
    assert!(
        text.contains("unknown scratch:work/2026-11-19-b (subject: no-such-record)"),
        "{text}"
    );
    Ok(())
}

#[test]
fn rows_follow_the_store_target_commit_and_say_when_head_differs() -> TestResult {
    let (w, origin, clone) = stored()?;
    assert!(!context(&w, "harbor-api")?.contains("work/e"));
    // Pushed upstream and fetched: listed, though a read was cached.
    add(
        &origin,
        "work/e",
        "---\nsubject: harbor-api\n---\n# Later\n",
    )?;
    git(&clone, &["pull", "-q", "--ff-only"])?;
    let text = context(&w, "harbor-api")?;
    assert!(text.contains("material scratch:work/e Later"), "{text}");
    assert!(!text.contains("differs"), "{text}");
    // Committed in the clone, not pushed: origin/HEAD is read, with a note.
    add(&clone, "work/f", "---\nsubject: harbor-api\n---\n# Local\n")?;
    let text = context(&w, "harbor-api")?;
    assert!(!text.contains("work/f"), "{text}");
    assert!(text.contains("material not pushed is not listed"), "{text}");
    Ok(())
}

#[test]
fn a_store_that_cannot_be_read_is_a_note_and_no_store_changes_nothing() -> TestResult {
    let (w, _, clone) = stored()?;
    std::fs::remove_dir_all(&clone)?;
    let text = context(&w, "harbor-api")?;
    assert!(
        text.contains("material: store scratch not read (missing)"),
        "{text}"
    );
    assert!(!text.contains("material scratch:"), "{text}");
    // With no declaration, no material section, count or trailer at all.
    let plain = world(&[])?;
    let text = context(&plain, "harbor-api")?;
    assert!(!text.contains("material"), "{text}");
    let mut request = plain.request(Verb::Context {
        target: "harbor-api".into(),
        depth: 1,
    });
    request.json = true;
    let (text, _) = plain.run(&request)?;
    assert!(
        !text.contains("\"stores\"") && !text.contains("material"),
        "{text}"
    );
    Ok(())
}
