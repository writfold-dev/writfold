//! Shared by `gate_assessed.rs` and `gate_assessed_faults.rs`: the fixture
//! gates, the labelled TEST DOUBLE for the `weft` binary, and `gate run`
//! in a child process. The binary is named by `$WRITFOLD_WEFT`, so every
//! assessed `gate run` is a child of the test binary (its `child` test),
//! given that variable. `tests/weft/weft-double.sh` does not reason: it
//! prints the report the real Weft printed for `tests/weft/step.weft`.
#![allow(dead_code)]
use crate::support::{NOW, TestResult, World};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use writfold_kb::gate::GateVerb;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
pub const CHILD: &str = "WRITFOLD_KB_ASSESS_CHILD";
pub const SOURCE: &str = include_str!("../weft/step.weft");

pub fn weft_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/weft")
}

pub fn double() -> PathBuf {
    weft_dir().join("weft-double.sh")
}

pub fn sha(path: &Path) -> TestResult<String> {
    Ok(format!(
        "sha256:{}",
        hex(&Sha256::digest(std::fs::read(path)?))
    ))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// The `unit` gate on `demo-service`, the step source, and one assessed
/// gate per `(id, weft, check)`, all pinned to `pin`.
pub fn define(w: &World, pin: &str, assessed: &[(&str, &str, &str)]) -> TestResult {
    let gates = w.kb.join(".writfold/gates");
    put(
        &gates.join("unit.toml"),
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n",
    )?;
    put(&gates.join("step.weft"), SOURCE)?;
    for (id, weft, check) in assessed {
        put(
            &gates.join(format!("{id}.toml")),
            &format!(
                "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"assessed\"\nweft = \"{weft}\"\nchecks = [\"{check}\"]\nweft_sha256 = \"{pin}\"\ntimeout_seconds = 30\ndescription = \"the plan step is verified\"\n"
            ),
        )?;
    }
    Ok(())
}

pub fn request(kb: &Path, verb: GateVerb, at: i64) -> Request {
    Request {
        verb: Verb::Gate(verb),
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(at),
        probe: writfold_kb::write::no_probe,
    }
}

pub fn run_verb(id: &str, repo: &Path) -> GateVerb {
    GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(repo.to_path_buf()),
    }
}

pub fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one `gate run`, its output between markers. Does nothing
/// unless the parent set the environment. Each test binary calls it from
/// its own `child` test.
pub fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    let gate = std::env::var("WRITFOLD_KB_ASSESS_GATE")?;
    let repo = PathBuf::from(std::env::var("WRITFOLD_KB_ASSESS_REPO")?);
    let at: i64 = std::env::var("WRITFOLD_KB_ASSESS_AT")?.parse()?;
    let kb = PathBuf::from(kb);
    let mut req = request(&kb, run_verb(&gate, &repo), at);
    req.probe = abort_at;
    match writfold_kb::run(&req) {
        Ok(out) => println!("<<<\n{}\nEXIT={}\n>>>", out.text, out.exit),
        Err(error) => println!("<<<\nERROR {error}\nEXIT=3\n>>>"),
    }
    Ok(())
}

/// `gate run <id>` in a child with `$WRITFOLD_WEFT` set to `weft` (unset
/// when `None`, with no configuration file either).
pub fn assess(w: &World, id: &str, weft: Option<&Path>, at: i64) -> TestResult<(String, u8)> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, &w.kb)
        .env("WRITFOLD_KB_ASSESS_GATE", id)
        .env("WRITFOLD_KB_ASSESS_REPO", &w.code)
        .env("WRITFOLD_KB_ASSESS_AT", at.to_string())
        .env_remove("WRITFOLD_WEFT")
        .env_remove("WRITFOLD_KB_FAULT_STEP")
        .env("XDG_CONFIG_HOME", w.base.join("no-config"));
    if let Some(weft) = weft {
        command.env("WRITFOLD_WEFT", weft);
    }
    let out = String::from_utf8(command.output()?.stdout)?;
    let body = out
        .split_once("<<<\n")
        .and_then(|(_, rest)| rest.split_once("\n>>>"))
        .map(|(body, _)| body.to_owned())
        .ok_or_else(|| format!("no child output:\n{out}"))?;
    let (text, exit) = body.rsplit_once("\nEXIT=").ok_or("no exit")?;
    let text = text.replace(&w.base.display().to_string(), "<WORLD>");
    Ok((text, exit.parse()?))
}

pub fn gate(w: &World, verb: GateVerb, at: i64) -> TestResult<(String, u8)> {
    let mut req = w.request(Verb::Gate(verb));
    req.now = Some(at);
    w.run(&req)
}

pub fn unit(w: &World, at: i64) -> TestResult {
    let (text, exit) = gate(w, run_verb("unit", &w.code), at)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

pub fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    gate(w, verb, NOW + 3600)
}

pub fn state_of(text: &str, id: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("gate {id} [")))
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("absent")
        .to_owned()
}

/// The run's assessed receipts, by name.
pub fn receipts(w: &World) -> TestResult<Vec<PathBuf>> {
    let dir = w.kb.join(format!("runs/{RUN}/assessed"));
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    out.sort();
    Ok(out)
}

pub fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let text = names
        .replace_all(&text.replace(RUN, "<RUN>"), "-$1-XXXXXX")
        .replace("<RUN>", RUN);
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    assert_eq!(expected, actual, "golden {name} differs");
    Ok(())
}
