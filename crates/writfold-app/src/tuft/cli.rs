//! Argument handling for `writfold tuft ...`; returns the text to print.
use super::{Invocation, compile, run};
use crate::trial::Result;
use std::path::{Path, PathBuf};
use tuft::emit::Format;

pub const USAGE: &str = "\
writfold tuft run FILE --repo DIR [--state DIR] [--cursor C] [--json]
                     run a Tuft v0 program over the pinned commit it names: exact
                     find, retained citation bindings, a budgeted projection
writfold tuft check FILE
                     compile only; print diagnostics or the checked pipeline

Options: --state DIR (default: $WRITFOLD_STATE, else ~/.local/share/writfold/trial),
the trial state that holds the admitted Git executable and retained evidence.
Language: docs/tuft/language.md. Semantic, fuzzy and ranked search, and `follow`,
are refused until the Omnigraph backend exists.";

struct Args {
    positional: Vec<String>,
    options: Vec<(String, String)>,
    json: bool,
}

fn parse(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        positional: Vec::new(),
        options: Vec::new(),
        json: false,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--json" {
            parsed.json = true;
        } else if let Some(name) = arg.strip_prefix("--") {
            if !matches!(name, "repo" | "state" | "cursor") {
                return Err(format!("unknown option --{name}").into());
            }
            let value = iter
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            if parsed.options.iter().any(|(n, _)| n == name) {
                return Err(format!("--{name} given twice").into());
            }
            parsed.options.push((name.to_owned(), value.clone()));
        } else {
            parsed.positional.push(arg.clone());
        }
    }
    Ok(parsed)
}

impl Args {
    fn option(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn file(&self) -> Result<&str> {
        match self.positional.as_slice() {
            [_, file] => Ok(file),
            _ => Err("expected exactly one program FILE".into()),
        }
    }

    fn state(&self) -> Result<PathBuf> {
        if let Some(state) = self
            .option("state")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("WRITFOLD_STATE").map(PathBuf::from))
        {
            return Ok(state);
        }
        let home = std::env::var_os("HOME").ok_or("HOME is not set; pass --state")?;
        Ok(Path::new(&home).join(".local/share/writfold/trial"))
    }
}

pub fn main(args: &[String]) -> Result<String> {
    let parsed = parse(args)?;
    match parsed.positional.first().map(String::as_str) {
        Some("run") => {
            let file = parsed.file()?;
            let source = std::fs::read(file)?;
            let state = parsed.state()?;
            let repo = parsed.option("repo").ok_or("missing --repo")?;
            run(&Invocation {
                source: &source,
                name: file,
                repo: Path::new(repo),
                state: &state,
                cursor: parsed.option("cursor"),
                format: if parsed.json {
                    Format::Json
                } else {
                    Format::Text
                },
            })
        }
        Some("check") => {
            let file = parsed.file()?;
            let plan = compile(&std::fs::read(file)?, file)?;
            let (matcher, pattern) = plan.find.matcher.describe();
            Ok(format!(
                "ok {} program=sha256:{}\n  find @{} {matcher} {:?} in {:?} at {} limit={}\n  retain {}\n  project @{} [{}]\n  emit budget_tokens={}",
                tuft::VERSION,
                plan.program,
                plan.find.binding,
                pattern,
                plan.find.scope,
                plan.find.commit,
                plan.find.budget.positions(),
                plan.retain.as_ref().map_or("none".to_owned(), |r| format!(
                    "@{} around={:?}",
                    r.binding, r.around
                )),
                plan.view.binding,
                plan.view
                    .fields
                    .iter()
                    .map(|f| f.name())
                    .collect::<Vec<_>>()
                    .join(", "),
                plan.emit.budget_tokens
            ))
        }
        Some("help") | None => Ok(USAGE.to_owned()),
        Some(other) => Err(format!("unknown tuft command {other}; see: writfold tuft help").into()),
    }
}
