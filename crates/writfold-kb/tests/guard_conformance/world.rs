//! Reading a conformance case strictly and building its world: the
//! fixture KB of the test support, the repositories, rules, run and gates
//! the case names, built through the ordinary verbs.
use super::support::{TestResult, World, git, world};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";

/// The object's keys must be among `allowed`, and `required` present.
pub fn keys(value: &Value, allowed: &[&str], required: &[&str], what: &str) -> TestResult<()> {
    let map = value
        .as_object()
        .ok_or_else(|| format!("{what}: not an object"))?;
    if let Some(k) = map.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(format!("{what}: unknown key {k}").into());
    }
    if let Some(k) = required.iter().find(|k| !map.contains_key(**k)) {
        return Err(format!("{what}: missing key {k}").into());
    }
    Ok(())
}

/// `{root}` and `{home}` replaced in every string of `value`.
pub fn fill(value: &Value, root: &Path, home: &str) -> Value {
    match value {
        Value::String(s) => Value::String(
            s.replace("{home}", home)
                .replace("{root}", &root.display().to_string()),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| fill(v, root, home)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), fill(v, root, home)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// A path the case names: it must start at `{root}` or `{home}`.
pub fn placed(text: &str) -> TestResult<()> {
    match text.starts_with("{root}") || text.starts_with("{home}") {
        true if !text.split('/').any(|p| p == "..") => Ok(()),
        _ => Err(format!("path {text:?} is outside {{root}} and {{home}}").into()),
    }
}

fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn text(value: &Value, key: &str) -> TestResult<String> {
    Ok(value[key]
        .as_str()
        .ok_or_else(|| format!("{key}: not a string"))?
        .to_owned())
}

fn rules(dir: &Path, value: &Value) -> TestResult {
    for (id, body) in value.as_object().into_iter().flatten() {
        let body = body.as_str().ok_or("a rule's text is a string")?;
        put(&dir.join(format!(".writfold/guards/{id}.toml")), body)?;
    }
    Ok(())
}

fn repo(spec: &Value) -> TestResult<PathBuf> {
    keys(
        spec,
        &["top", "origin", "worktrees", "rules"],
        &["top"],
        "repo",
    )?;
    let top = PathBuf::from(text(spec, "top")?);
    put(&top.join("src/lib.rs"), "pub fn lib() {}\n")?;
    put(&top.join("README.md"), "# repository\n")?;
    git(&top, &["init", "-q"])?;
    git(&top, &["add", "-A"])?;
    git(&top, &["commit", "-q", "-m", "first"])?;
    if let Some(origin) = spec.get("origin").and_then(Value::as_str) {
        git(&top, &["remote", "add", "origin", origin])?;
    }
    for wt in spec["worktrees"].as_array().into_iter().flatten() {
        let wt = wt.as_str().ok_or("a worktree is a path")?;
        git(&top, &["worktree", "add", "-q", wt])?;
    }
    rules(&top, &spec["rules"])?;
    Ok(top)
}

/// Where the gate's repository is: the map's path, else a repository
/// whose directory has the name.
fn located(world: &Value, tops: &[PathBuf], name: &str) -> TestResult<PathBuf> {
    if let Some(path) = world["kb"]["map"].get(name).and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    tops.iter()
        .find(|t| t.file_name().is_some_and(|n| n == name))
        .cloned()
        .ok_or_else(|| format!("gate repository {name} is not in the world").into())
}

fn gate(w: &World, spec: &Value, top: PathBuf) -> TestResult {
    keys(
        spec,
        &["id", "repo", "standing"],
        &["id", "repo", "standing"],
        "gate",
    )?;
    let id = text(spec, "id")?;
    let def = format!(
        "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n"
    );
    put(&w.kb.join(format!(".writfold/gates/{id}.toml")), &def)?;
    let kind = match text(spec, "standing")?.as_str() {
        "met" => Kind::Attested,
        "waived" => Kind::Waived,
        "open" => return Ok(()),
        other => return Err(format!("gate standing {other:?}").into()),
    };
    let say = Say {
        kind,
        id,
        text: "looked at it".into(),
        by: "conformance".into(),
        run: Some(RUN.into()),
        repo: Some(top),
    };
    let exit = writfold_kb::run(&w.request(Verb::Gate(GateVerb::Say(say))))?.exit;
    match exit {
        0 => Ok(()),
        _ => Err(format!("the gate statement was refused ({exit})").into()),
    }
}

/// The current-run pointer for `open` and `closed`; the run is closed
/// after its statements are filed.
fn run_state(w: &World, state: &str, close: bool) -> TestResult {
    match (state, close) {
        ("none", _) | ("open", true) => Ok(()),
        ("open" | "closed", false) => {
            put(&w.kb.join(".git/writfold/current-run"), &format!("{RUN}\n"))
        }
        ("closed", true) => {
            let path = w.kb.join(format!("runs/{RUN}/manifest.json"));
            let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
            manifest["state"] = json!("closed");
            put(&path, &serde_json::to_string_pretty(&manifest)?)
        }
        (other, _) => Err(format!("run {other:?}").into()),
    }
}

fn kb(w: &World, spec: &Value, tops: &[PathBuf], world_spec: &Value) -> TestResult {
    let allowed = ["rules", "run", "map", "gates", "nudged", "unreadable"];
    keys(spec, &allowed, &["run"], "kb")?;
    rules(&w.kb, &spec["rules"])?;
    let mut map = String::new();
    for (name, path) in spec["map"].as_object().into_iter().flatten() {
        map.push_str(&format!("{name} {}\n", path.as_str().ok_or("a map path")?));
    }
    put(&w.kb.join(".git/writfold/repos"), &map)?;
    let run = text(spec, "run")?;
    run_state(w, &run, false)?;
    for g in spec["gates"].as_array().into_iter().flatten() {
        gate(w, g, located(world_spec, tops, &text(g, "repo")?)?)?;
    }
    run_state(w, &run, true)?;
    for session in spec["nudged"].as_array().into_iter().flatten() {
        let session = session.as_str().ok_or("a session is a string")?;
        let mut draft = Map::new();
        draft.insert("nudged".into(), json!("2026-11-11T00:00:00Z"));
        writfold_kb::draft::save(&w.kb.join(".git/writfold"), session, &draft)?;
    }
    if spec["unreadable"].as_bool() == Some(true) {
        std::fs::remove_dir_all(w.kb.join(".git/writfold"))?;
        std::fs::write(w.kb.join(".git/writfold"), "not a directory\n")?;
    }
    Ok(())
}

/// Build the world a case states; returns it and its home directory.
pub fn build(raw: &Value) -> TestResult<(World, PathBuf, Value)> {
    keys(
        raw,
        &["schema", "home", "dirs", "repos", "kb"],
        &["schema", "home", "kb"],
        "world",
    )?;
    if raw["schema"] != "writfold-guard-world/1" {
        return Err("world schema".into());
    }
    let mut paths: Vec<&str> = vec![raw["home"].as_str().ok_or("home")?];
    paths.extend(
        raw["dirs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str),
    );
    for r in raw["repos"].as_array().into_iter().flatten() {
        paths.extend(r.get("top").and_then(Value::as_str));
        paths.extend(
            r["worktrees"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
    }
    paths.extend(
        raw["kb"]["map"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_str()),
    );
    for p in paths {
        placed(p)?;
    }
    let w = world(&[])?;
    let home_text = raw["home"]
        .as_str()
        .ok_or("home")?
        .replace("{root}", &w.base.display().to_string());
    let spec = fill(raw, &w.base, &home_text);
    let home = PathBuf::from(&home_text);
    std::fs::create_dir_all(&home)?;
    for d in spec["dirs"].as_array().into_iter().flatten() {
        std::fs::create_dir_all(d.as_str().ok_or("a dir is a path")?)?;
    }
    let mut tops = Vec::new();
    for r in spec["repos"].as_array().into_iter().flatten() {
        tops.push(repo(r)?);
    }
    kb(&w, &spec["kb"], &tops, &spec)?;
    Ok((w, home, spec))
}
