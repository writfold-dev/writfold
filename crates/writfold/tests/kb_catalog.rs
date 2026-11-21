//! Project asset catalogs end to end (#86), through the real binary and
//! Git: both input shapes, review and `git apply`, the applied records in
//! `find`, `context`, a ticket and a proposed investigation, and every
//! way an input can be incomplete, conflicting or unsafe, none of which
//! prints a patch. Two clones give byte-identical patches.
#[path = "catalog/support.rs"]
mod support;

use std::path::Path;
use std::process::Command;
use support::*;

#[test]
fn a_reviewed_patch_makes_ordinary_records_every_reader_uses() -> Result {
    let k = kb("apply", &base())?;
    let (shown, _, exit) = kb_run(&k.0, &["catalog", "check"])?;
    assert_eq!(exit, 2, "drift before the patch: {shown}");
    let patch = apply(&k.0, "brewery")?;
    assert!(
        patch.contains("new file mode 100644")
            && patch.contains("+++ b/inventory/assets/mix-station.json"),
        "{patch}"
    );
    assert_eq!(
        kb_run(&k.0, &["catalog", "check"])?.2,
        0,
        "clean after the patch"
    );
    let (assets, _, _) = kb_run(&k.0, &["find", "--kind", "asset"])?;
    assert!(
        assets.contains("asset mix-station") && assets.contains("asset demo"),
        "{assets}"
    );
    let (ctx, _, _) = kb_run(&k.0, &["context", "brewdeck"])?;
    assert!(ctx.contains("mix-station"), "the alias resolves: {ctx}");
    let (t, terr, exit) = kb_run(
        &k.0,
        &[
            "record",
            "ticket",
            "BR-1",
            "--title",
            "Leak",
            "--assets",
            "mix-station",
        ],
    )?;
    assert_eq!(exit, 0, "{t}{terr}");
    let (p, perr, exit) = kb_run(
        &k.0,
        &[
            "propose",
            "investigation",
            "--title",
            "Temperature",
            "--assets",
            "mix-station",
            "--local",
        ],
    )?;
    assert_eq!(exit, 0, "{p}{perr}");
    let (bad, _, exit) = kb_run(
        &k.0,
        &[
            "record",
            "ticket",
            "BR-2",
            "--title",
            "x",
            "--assets",
            "not-in-catalog",
        ],
    )?;
    assert_eq!(exit, 3, "an unknown asset is still refused: {bad}");
    let (verify, _, exit) = kb_run(&k.0, &["index", "rebuild"])?;
    assert_eq!(exit, 0, "{verify}");
    // Reordering the input changes no record: no diff.
    let reordered = r#"{"equipment":[{"tag":"chiller","label":"Chiller","classification":"cooler","search":{"names":[]}},{"tag":"mix-station","label":"Mix station","classification":"fermenter","search":{"names":["brewdeck"]}}]}"#;
    put(&k.0, ".writfold/catalog-inputs/equipment.json", reordered)?;
    commit(&k.0, "reorder")?;
    let (empty, _, exit) = kb_run(&k.0, &["catalog", "diff", "brewery"])?;
    assert_eq!((empty.as_str(), exit), ("", 0));
    Ok(())
}

#[test]
fn change_rename_and_deliberate_removal_are_reviewable_patches() -> Result {
    let k = kb("change", &base())?;
    apply(&k.0, "brewery")?;
    let changed = INPUT
        .replace("Mix station", "Mixing station")
        .replace("\"chiller\"", "\"cold-box\"");
    put(&k.0, ".writfold/catalog-inputs/equipment.json", &changed)?;
    git(&k.0, &["add", "-A"])?;
    let patch = apply(&k.0, "brewery")?;
    assert!(
        patch.contains("-  \"name\": \"Mix station\",")
            && patch.contains("+  \"name\": \"Mixing station\","),
        "{patch}"
    );
    assert!(
        patch.contains("deleted file mode 100644\n--- a/inventory/assets/chiller.json"),
        "{patch}"
    );
    assert!(
        patch.contains("+++ b/inventory/assets/cold-box.json"),
        "{patch}"
    );
    // A hand edit to a managed record is drift, undone by a replacement.
    let path = k.0.join("inventory/assets/cold-box.json");
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)?.replace("cooler", "freezer"),
    )?;
    let (rows, _, exit) = kb_run(&k.0, &["catalog", "check", "brewery"])?;
    assert_eq!(exit, 2, "{rows}");
    assert!(rows.contains("[replace]"), "{rows}");
    git(&k.0, &["checkout", "--", "inventory/assets/cold-box.json"])?;
    // An explicit empty inventory removes every managed record; the
    // hand-owned one stays.
    put(
        &k.0,
        ".writfold/catalog-inputs/equipment.json",
        "{\"equipment\": []}",
    )?;
    git(&k.0, &["add", "-A"])?;
    let patch = apply(&k.0, "brewery")?;
    assert_eq!(patch.matches("deleted file mode").count(), 2, "{patch}");
    assert!(k.0.join("inventory/assets/demo.json").exists());
    Ok(())
}

#[test]
fn untracked_symlinked_and_orphaned_inputs_are_faults() -> Result {
    let dir_decl = "schema = \"writfold-asset-catalog/1\"\ninput = \".writfold/catalog-inputs/dir\"\n\n[fields]\nid = \"/id\"\nname = \"/a~1b\"\n";
    let k = kb(
        "dir",
        &[
            (".writfold/catalogs/brewery.toml", dir_decl),
            (
                ".writfold/catalog-inputs/dir/one.json",
                "{\"id\": \"kettle\", \"a/b\": \"Kettle\"}",
            ),
        ],
    )?;
    let patch = apply(&k.0, "brewery")?;
    assert!(
        patch.contains("\"name\": \"Kettle\""),
        "escaped pointer ~1: {patch}"
    );
    put(
        &k.0,
        ".writfold/catalog-inputs/dir/two.json",
        "{\"id\": \"pump\", \"a/b\": \"Pump\"}",
    )?;
    let (_, err, exit) = kb_run(&k.0, &["catalog", "diff", "brewery"])?;
    assert_eq!(exit, 3, "an untracked input is diagnosed, not skipped");
    assert!(err.contains("not tracked by Git"), "{err}");
    std::fs::remove_file(k.0.join(".writfold/catalog-inputs/dir/two.json"))?;
    std::os::unix::fs::symlink("/etc", k.0.join(".writfold/catalog-inputs/link"))?;
    put(
        &k.0,
        ".writfold/catalogs/brewery.toml",
        &dir_decl.replace("/dir\"", "/link\""),
    )?;
    let (_, err, exit) = kb_run(&k.0, &["catalog", "diff", "brewery"])?;
    assert_eq!(exit, 3);
    assert!(err.contains("symbolic link"), "{err}");
    // A declaration removed while its records remain: orphaned, invalid.
    std::fs::remove_file(k.0.join(".writfold/catalogs/brewery.toml"))?;
    let (rows, _, exit) = kb_run(&k.0, &["catalog", "check"])?;
    assert_eq!(exit, 3, "{rows}");
    assert!(rows.contains("not declared"), "{rows}");
    let (lint, _, _) = kb_run(&k.0, &["check"])?;
    assert!(
        lint.contains("catalogs"),
        "kb check covers catalogs: {lint}"
    );
    Ok(())
}

#[test]
fn two_clones_give_the_same_patch_and_no_config_reads_as_before() -> Result {
    let a = kb("clone-a", &base())?;
    let b_dir =
        std::env::temp_dir().join(format!("writfold-catalog-clone-b-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&b_dir);
    let out = git(
        &a.0,
        &[
            "clone",
            "-q",
            a.0.to_str().ok_or("path")?,
            b_dir.to_str().ok_or("path")?,
        ],
    )?;
    assert!(out.status.success());
    let b = Kb(b_dir);
    let (pa, _, _) = kb_run(&a.0, &["catalog", "diff", "brewery"])?;
    let (pb, _, _) = kb_run(&b.0, &["catalog", "diff", "brewery"])?;
    assert!(!pa.is_empty());
    assert_eq!(pa, pb, "patch bytes do not depend on the clone");
    let plain = kb("plain", &[])?;
    let (check, _, _) = kb_run(&plain.0, &["check"])?;
    assert!(
        !check.contains("catalogs"),
        "no catalog part without one: {check}"
    );
    let (none, _, exit) = kb_run(&plain.0, &["catalog", "check"])?;
    assert_eq!(exit, 0, "{none}");
    assert!(none.contains("no catalog is declared"), "{none}");
    Ok(())
}

#[test]
fn the_documented_csv_export_feeds_a_catalog() -> Result {
    let csv = "tag,label,classification,names[]\nmix-station,Mix station,fermenter,brewdeck;mixer\nchiller,Chiller,cooler,\n";
    let k = kb(
        "csv",
        &[
            (
                ".writfold/catalogs/brewery.toml",
                DECL.replace("/search/names", "/names").as_str(),
            ),
            ("exports/equipment.csv", csv),
        ],
    )?;
    let recipe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/recipes/csv-to-catalog.py");
    let out = Command::new("python3")
        .arg(&recipe)
        .arg(k.0.join("exports/equipment.csv"))
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::create_dir_all(k.0.join(".writfold/catalog-inputs"))?;
    std::fs::write(
        k.0.join(".writfold/catalog-inputs/equipment.json"),
        &out.stdout,
    )?;
    git(&k.0, &["add", "-A"])?;
    let patch = apply(&k.0, "brewery")?;
    assert!(
        patch.contains("\"mixer\"") && patch.contains("+++ b/inventory/assets/chiller.json"),
        "{patch}"
    );
    let (ctx, _, _) = kb_run(&k.0, &["context", "mixer"])?;
    assert!(ctx.contains("mix-station"), "{ctx}");
    Ok(())
}
