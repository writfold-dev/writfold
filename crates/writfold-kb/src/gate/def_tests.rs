//! Definition parsing: the digest ignores layout and covers what a gate
//! means, including its instructions and the scripts it ships.
use super::def::{Judges, Origin, parse};
use std::path::{Path, PathBuf};

const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"scripts/gate\", \"unit\", \"--receipt\", \"{artifacts}/unit.json\"]\ntimeout_seconds = 60\nscope = [\"**\", \"!docs/**\"]\n";
const NOWHERE: &str = "/nonexistent/.writfold/gates";

fn read(text: &str, stem: &str, dir: &Path) -> Result<super::def::Definition, String> {
    parse(text, stem, Origin::Kb, dir).map_err(|e| e.to_string())
}

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Result<Dir, String> {
    let base = std::env::temp_dir().join(format!("writfold-def-{name}-{}", std::process::id()));
    let gates = base.join("gates");
    std::fs::create_dir_all(&gates).map_err(|e| e.to_string())?;
    Ok(Dir(base))
}

#[test]
fn a_definition_parses_and_its_digest_ignores_layout() -> Result<(), String> {
    let nowhere = Path::new(NOWHERE);
    let d = parse(UNIT, "unit", Origin::Repo, nowhere).map_err(|e| e.to_string())?;
    assert_eq!((d.workdir.as_str(), d.timeout_seconds), (".", 60));
    assert_eq!(d.judges, Judges::Repository);
    let commented = UNIT.replace("version = 1\n", "# why\nversion   = 1 # first\n");
    let again = read(&commented, "unit", nowhere)?;
    assert_eq!(d.digest, again.digest);
    let changed = read(&UNIT.replace("60", "61"), "unit", nowhere)?;
    assert_ne!(d.digest, changed.digest);
    for (from, to) in [
        ("id = \"unit\"", "id = \"other\""),
        ("class = \"mechanical\"", "class = \"assessed\""),
        ("\"unit\", \"--receipt\"", "\"{receipt}\", \"--receipt\""),
        ("\"unit\", \"--receipt\"", "\"{candidate}\", \"--receipt\""),
        ("timeout_seconds = 60", "timeout_seconds = 0"),
        ("scope = [\"**\", \"!docs/**\"]", "scope = []"),
        ("scope = [\"**\", \"!docs/**\"]", "scope = [\"/abs\"]"),
        ("version = 1", "version = 1\nshell = \"sh -c\""),
        ("version = 1", "version = 1\nworkdir = \"../up\""),
        ("version = 1", "version = 1\nsubject = \"draft\""),
        ("version = 1", "version = 1\ninstructions = \"missing.md\""),
        ("writfold-gate/1", "writfold-gate/2"),
        ("class = \"mechanical\"", "class = \"attested\""),
    ] {
        assert!(
            read(&UNIT.replace(from, to), "unit", nowhere).is_err(),
            "{to}"
        );
    }
    let attested = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";
    let d = read(attested, "review", nowhere)?;
    assert!(d.attested() && d.argv.is_empty());
    Ok(())
}

/// The digest of a definition without the new keys is the one G1 wrote,
/// so receipts taken before instruction gates existed stay current.
#[test]
fn a_plain_definition_keeps_its_g1_digest() -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let d = read(UNIT, "unit", Path::new(NOWHERE))?;
    let g1 = "writfold-gate/1\nid=unit\nversion=1\nclass=mechanical\nargv=[\"scripts/gate\",\"unit\",\"--receipt\",\"{artifacts}/unit.json\"]\nworkdir=.\ntimeout_seconds=60\nscope=[\"**\",\"!docs/**\"]\n";
    let expected = format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(g1.as_bytes()))
    );
    assert_eq!(d.digest, expected);
    Ok(())
}

#[test]
fn instructions_and_shipped_scripts_enter_the_digest() -> Result<(), String> {
    let dir = scratch("digest")?;
    let gates = dir.0.join("gates");
    let write =
        |name: &str, text: &str| std::fs::write(gates.join(name), text).map_err(|e| e.to_string());
    write("language.md", "Write plain English.\n")?;
    write("check.sh", "exit 0\n")?;
    let text = "schema = \"writfold-gate/1\"\nid = \"language\"\nversion = 1\nclass = \"mechanical\"\nsubject = \"candidate\"\ninstructions = \"language.md\"\nargv = [\"sh\", \"{gate}/check.sh\", \"{candidate}\"]\ntimeout_seconds = 10\n";
    let first = read(text, "language", &gates)?;
    assert!(first.on_candidate() && first.files.len() == 1);
    assert_eq!(
        first.instructions_path().as_deref(),
        Some(".writfold/gates/language.md")
    );
    write("language.md", "Write plain English, and short sentences.\n")?;
    let edited = read(text, "language", &gates)?;
    assert_ne!(
        first.digest, edited.digest,
        "an instructions edit changes the digest"
    );
    write("check.sh", "exit 1\n")?;
    let script = read(text, "language", &gates)?;
    assert_ne!(
        edited.digest, script.digest,
        "a script edit changes the digest"
    );
    for (from, to) in [
        ("language.md\"", "../language.md\""),
        ("language.md\"", "/etc/passwd.md\""),
        ("language.md\"", "language.txt\""),
        ("\"{candidate}\"]", "\"x\"]"),
        ("\"{candidate}\"]", "\"{candidate}\", \"{artifacts}\"]"),
        ("{gate}/check.sh", "{gate}/../check.sh"),
        ("{gate}/check.sh", "{gate}/missing.sh"),
        (
            "timeout_seconds = 10\n",
            "timeout_seconds = 10\nscope = [\"**\"]\n",
        ),
    ] {
        assert!(
            read(&text.replace(from, to), "language", &gates).is_err(),
            "{to}"
        );
    }
    let repo = parse(text, "language", Origin::Repo, &gates);
    assert!(repo.is_err_and(|e| e.to_string().contains("KB root")));
    Ok(())
}
