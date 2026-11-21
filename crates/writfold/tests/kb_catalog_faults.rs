//! A catalog whose input is incomplete, conflicting or unsafe prints no
//! patch (#86): `catalog diff` exits 3 with nothing on stdout, and
//! `catalog check` exits 3 too.
#[path = "catalog/support.rs"]
mod support;

use support::*;

const DIR_DECL: &str = "schema = \"writfold-asset-catalog/1\"\ninput = \".writfold/catalog-inputs/dir\"\n\n[fields]\nid = \"/id\"\nname = \"/a~1b\"\n";
const DECL_PATH: &str = ".writfold/catalogs/brewery.toml";
const INPUT_PATH: &str = ".writfold/catalog-inputs/equipment.json";

/// The declaration with `input` as written.
fn with_input(input: String) -> Vec<(&'static str, String)> {
    vec![(DECL_PATH, DECL.to_owned()), (INPUT_PATH, input)]
}

/// Faults of the input itself.
fn inputs() -> Vec<Case> {
    vec![
        (
            "missing input",
            vec![(DECL_PATH, DECL.to_owned())],
            "does not exist",
        ),
        (
            "duplicate key",
            with_input(INPUT.replacen("\"label\"", "\"tag\":\"x\",\"label\"", 1)),
            "duplicate JSON key",
        ),
        (
            "duplicate id",
            with_input(INPUT.replace("\"chiller\"", "\"mix-station\"")),
            "given twice",
        ),
        (
            "wrong type",
            with_input(INPUT.replace("\"Chiller\"", "7")),
            "must be a string",
        ),
        (
            "empty directory",
            vec![
                (DECL_PATH, DIR_DECL.to_owned()),
                (".writfold/catalog-inputs/dir/README", "x".into()),
            ],
            "no tracked .json",
        ),
    ]
}

/// Collisions, declarations and paths.
fn declarations() -> Vec<Case> {
    let mut other = with_input(INPUT.replace("\"chiller\"", "\"harbor\""));
    other.push((
        "harbor/STATUS.md",
        "---\nid: harbor\nkind: project\ntitle: Harbor\nstatus: active\n---\n".into(),
    ));
    vec![
        (
            "hand-owned collision",
            with_input(INPUT.replace("\"chiller\"", "\"demo\"")),
            "hand-owned",
        ),
        ("other-kind collision", other, "already claims"),
        (
            "bad declaration key",
            vec![
                (DECL_PATH, DECL.replace("items", "itemz")),
                (INPUT_PATH, INPUT.to_owned()),
            ],
            "brewery.toml",
        ),
        (
            "escaping path",
            vec![(DECL_PATH, DECL.replace(INPUT_PATH, "../outside.json"))],
            "..",
        ),
    ]
}

#[test]
fn an_incomplete_conflicting_or_unsafe_input_prints_no_patch() -> Result {
    for (what, files, why) in inputs().into_iter().chain(declarations()) {
        let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let k = kb("bad", &files)?;
        let (patch, err, exit) = kb_run(&k.0, &["catalog", "diff", "brewery"])?;
        assert_eq!((patch.as_str(), exit), ("", 3), "{what}: {err}");
        assert!(err.contains(why), "{what}: {err}");
        assert_eq!(kb_run(&k.0, &["catalog", "check"])?.2, 3, "{what}");
    }
    Ok(())
}
