//! Check receipt helpers: declaring `max_age`, a second clone with no
//! checker on `PATH`, listing receipts, and hand-written receipts (token
//! digests from `sha256sum`, times from `date`).
use super::{KIND, Result, Root, git, put};
use std::path::PathBuf;
use std::process::Command;

pub const DIR: &str = ".writfold/checks/src";

/// Declare `src` with this `max_age` (or none) and commit it.
pub fn max_age(w: &Root, age: Option<&str>) -> Result {
    let text = match age {
        Some(age) => format!("{KIND}max_age = \"{age}\"\n"),
        None => KIND.to_owned(),
    };
    put(&w.kb().join(".writfold/sources/src.toml"), &text)?;
    commit(w, "kind")
}

/// Commit everything in the work tree.
pub fn commit(w: &Root, message: &str) -> Result {
    git(&w.kb(), &["add", "-A"])?;
    git(&w.kb(), &["commit", "-q", "--allow-empty", "-m", message])
}

/// A clone of `w`'s KB with an empty `bin`: no checker on `PATH`, no index.
pub fn reader(w: &Root, name: &str) -> Result<Root> {
    let dir = std::env::temp_dir().join(format!("writfold-checker-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin"))?;
    let root = Root(std::fs::canonicalize(&dir)?);
    let from = w.kb().display().to_string();
    let to = root.kb().display().to_string();
    git(&root.0, &["clone", "-q", &from, &to])?;
    Ok(root)
}

/// The receipt files of kind `src`, by name.
pub fn files(w: &Root) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(w.kb().join(DIR))
        .into_iter()
        .flatten()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    found.sort();
    found
}

/// The text of the only receipt.
pub fn only(w: &Root) -> Result<String> {
    match files(w).as_slice() {
        [one] => Ok(std::fs::read_to_string(one)?),
        other => Err(format!("{} receipts", other.len()).into()),
    }
}

fn output(program: &str, args: &[&str], input: Option<&str>) -> Result<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = command.spawn()?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        std::io::Write::write_all(&mut stdin, text.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

/// `sha256:<hex>` of a token's exact text.
pub fn digest(token: &str) -> Result<String> {
    let line = output("sha256sum", &[], Some(token))?;
    let hex = line.split_whitespace().next().unwrap_or_default();
    Ok(format!("sha256:{hex}"))
}

/// The UTC time `offset` seconds from now, as a receipt writes it.
pub fn stamp(offset: i64) -> Result<String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let at = i64::try_from(now.as_secs())? + offset;
    output(
        "date",
        &["-u", "-d", &format!("@{at}"), "+%Y-%m-%dT%H:%M:%SZ"],
        None,
    )
}

/// Write `name` holding `(token, status)` entries, checked `offset`
/// seconds from now by `checker`.
pub fn write(w: &Root, name: &str, offset: i64, entries: &[(&str, &str)], checker: &str) -> Result {
    let mut lines = Vec::new();
    for (token, status) in entries {
        let moved = match *status {
            "moved" => ",\"moved\":{\"first\":12,\"last\":18}",
            _ => "",
        };
        lines.push(format!(
            "    {{\"token\":\"{}\",\"status\":\"{status}\"{moved}}}",
            digest(token)?
        ));
    }
    let text = format!(
        "{{\n  \"schema\": \"writfold-check-receipt/1\",\n  \"kind\": \"src\",\n  \"checker\": {{\"name\":\"{checker}\",\"version\":\"0.9\"}},\n  \"checked_at\": \"{}\",\n  \"verdicts\": [\n{}\n  ]\n}}\n",
        stamp(offset)?,
        lines.join(",\n")
    );
    put(&w.kb().join(DIR).join(name), &text)
}
