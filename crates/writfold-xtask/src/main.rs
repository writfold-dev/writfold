#![forbid(unsafe_code)]
mod attributes;
mod graph;
mod json_macro;
mod macro_audit;
mod macros;
mod matches_macro;
mod rust_attributes;
mod rust_json_macro;
mod rust_macro_audit;
mod rust_matches_macro;
mod rust_safety;
mod rust_syntax;
mod safety;
mod source;
mod syntax;
mod tokens;

use serde_json::json;
use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("quality") {
        return Err(
            "usage: writfold-xtask quality --root PATH --metadata PATH --report PATH".into(),
        );
    }
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !matches!(key.as_str(), "--root" | "--metadata" | "--report") {
            return Err(format!("unknown option {key}").into());
        }
        let value = args.next().ok_or("option value missing")?;
        if options.insert(key, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let root = PathBuf::from(options.get("--root").ok_or("--root required")?).canonicalize()?;
    let metadata = fs::read(options.get("--metadata").ok_or("--metadata required")?)?;
    let metadata = serde_json::from_slice(&metadata)?;
    let report = options.get("--report").ok_or("--report required")?;
    let mut diagnostics = Vec::new();
    graph::check(&metadata, &mut diagnostics)?;
    source::check(&root, &metadata, &mut diagnostics)?;
    let success = !diagnostics.iter().any(|item| item["severity"] == "error");
    fs::write(
        report,
        serde_json::to_vec_pretty(&json!({"schema":1,"complete":true,"success":success,
        "scope":"all workspace Rust files, macro templates, Verus executable and ghost bodies, declared and resolved Cargo dependencies",
        "diagnostics":diagnostics}))?,
    )?;
    Ok(success)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("quality infrastructure error: {error}");
            ExitCode::FAILURE
        }
    }
}
