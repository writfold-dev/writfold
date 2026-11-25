//! Vale profiles (#132): every input that changes what `prose.vale` means
//! stales an earlier receipt, the timeout does not, and the mode keys and
//! the profile tree are checked before anything runs.
mod support;
mod vale_support;

use support::TestResult;
use vale_support::{HEDGES, INI, PROFILE, mode, one, prepare, status, vale_world, write};

const TREE: &str = ".writfold/vale/house";

#[test]
fn every_bound_input_stales_the_check() -> TestResult {
    let edits: [(&str, &str, String); 7] = [
        ("config", ".vale.ini", INI.replace("suggestion", "warning")),
        (
            "rule",
            "styles/House/Hedges.yml",
            HEDGES.replace("very", "quite"),
        ),
        (
            "added",
            "styles/House/Terms.yml",
            "extends: substitution\n".into(),
        ),
        (
            "vocabulary",
            "styles/config/vocabularies/House/accept.txt",
            "Writfold\n".into(),
        ),
        ("pin", "", PROFILE.replace("3.28.1", "3.28.2")),
        ("canary", "", PROFILE.replace("This is TODO.", "TODO here.")),
        ("timeout", "", PROFILE.replace("= 2", "= 30")),
    ];
    for (what, file, text) in edits {
        let w = vale_world("")?;
        let (exit, shown, _) = one(&w, "<!-- standin: clean -->\nText.\n")?;
        assert_eq!(exit, 0, "{shown}");
        match file {
            "" => write(&w, ".writfold/deliveries/vale-house.toml", &text)?,
            f => write(&w, &format!("{TREE}/{f}"), &text)?,
        }
        let (shown, _) = status(&w)?;
        let want = match what {
            "timeout" => "prose.vale [satisfied]",
            _ => "prose.vale [stale]",
        };
        assert!(shown.contains(want), "{what}: {shown}");
    }
    let w = vale_world("")?;
    one(&w, "<!-- standin: clean -->\nText.\n")?;
    std::fs::remove_file(w.kb.join(TREE).join("styles/House/Hedges.yml"))?;
    assert!(status(&w)?.0.contains("prose.vale [stale]"), "removed");
    let w = vale_world("")?;
    one(&w, "<!-- standin: clean -->\nText.\n")?;
    mode(&w, "vale = \"house\"\nvale_block = \"warning\"\n")?;
    assert!(status(&w)?.0.contains("prose.vale [stale]"), "threshold");
    Ok(())
}

fn refused(w: &support::World, want: &str) -> TestResult {
    let why = match prepare(w, "x\n") {
        Ok((text, _)) => text,
        Err(e) => e.to_string(),
    };
    assert!(why.contains(want), "want {want:?}: {why}");
    Ok(())
}

#[test]
fn the_mode_keys_are_checked() -> TestResult {
    let w = vale_world("")?;
    mode(&w, "")?;
    refused(&w, "prose.vale needs vale")?;
    mode(&w, "vale = \"missing\"\n")?;
    refused(&w, "no Vale profile")?;
    mode(&w, "vale = \"house\"\nvale_block = \"fatal\"\n")?;
    refused(&w, "vale_block")?;
    let plain = "schema = \"writfold-delivery-mode/1\"\nmode = \"note\"\nchecks = []\n";
    for key in ["vale = \"house\"\n", "vale_block = \"error\"\n"] {
        write(
            &w,
            ".writfold/deliveries/note.toml",
            &format!("{plain}{key}"),
        )?;
        refused(&w, "checks does not list prose.vale")?;
    }
    write(
        &w,
        ".writfold/deliveries/vale-house.toml",
        &PROFILE.replace("3.28.1", "3.19.0"),
    )?;
    mode(&w, "vale = \"house\"\n")?;
    refused(&w, "3.20.0 or later")?;
    Ok(())
}

#[test]
fn the_tree_is_contained_and_whole() -> TestResult {
    let styles = [
        ("StylesPath = /etc/vale", "must be a relative path"),
        ("StylesPath = ../styles", "must be a relative path"),
        ("StylesPath = nowhere", "is not a directory"),
        ("MinAlertLevel = warning", "no top-level StylesPath"),
    ];
    for (line, want) in styles {
        let w = vale_world("")?;
        write(
            &w,
            &format!("{TREE}/.vale.ini"),
            &INI.replace("StylesPath = styles", line),
        )?;
        refused(&w, want)?;
    }
    let w = vale_world("")?;
    std::os::unix::fs::symlink("/etc/hostname", w.kb.join(TREE).join("styles/link.yml"))?;
    refused(&w, "is a symbolic link")?;
    let w = vale_world("")?;
    std::fs::remove_file(w.kb.join(TREE).join(".vale.ini"))?;
    refused(&w, "has no .vale.ini")?;
    let w = vale_world("")?;
    std::fs::remove_dir_all(w.kb.join(TREE))?;
    refused(&w, "copy your .vale.ini and styles")?;
    let w = vale_world("")?;
    let big = "x".repeat(1 << 20);
    for i in 0..9 {
        write(&w, &format!("{TREE}/styles/big{i}.txt"), &big)?;
    }
    refused(&w, "more than 2000 files or")?;
    Ok(())
}
