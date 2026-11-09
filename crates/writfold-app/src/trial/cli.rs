//! Argument handling for `writfold trial ...`; returns the text to print.
use super::flow::{Event, Premises};
use super::{Result, Trial, apply, approve, prepare, stream};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
writfold trial init                     create the private trial state directory
writfold trial prepare --repo DIR --file PATH --recipe FILE [--report FILE]
                                        worktree at HEAD + Splicery replay; records Prepared
writfold trial approve RUN              sign a grant and an approval; records Approved
writfold trial apply RUN                re-verify, check the file, install; records Applied
writfold trial show RUN                 print the run's validated ledger history

Options: --state DIR (default: $WRITFOLD_STATE, else ~/.local/share/writfold/trial).
Profile writfold-trial-worktree/1: a cooperating single-user trial. Edits land only
in Writfold's own worktree of your repository, never in your checkout.";

struct Args {
    positional: Vec<String>,
    options: Vec<(String, String)>,
}

fn parse(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        positional: Vec::new(),
        options: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(name) = arg.strip_prefix("--") {
            let value = iter
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
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
    fn required(&self, name: &str) -> Result<&str> {
        self.option(name)
            .ok_or_else(|| format!("missing --{name}").into())
    }
    fn run(&self) -> Result<&str> {
        match self.positional.as_slice() {
            [_, run] => Ok(run),
            _ => Err("expected exactly one RUN".into()),
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

fn show(state: &Path, run: &str) -> Result<String> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let loaded = trial
        .ledger(&lease, &executable)?
        .load::<Event, Premises>(&stream(run)?)?;
    let mut out = format!(
        "run {run}: {} record(s), history validated\n",
        loaded.records.len()
    );
    for record in &loaded.records {
        let line = match &record.payload {
            Event::Prepared {
                path,
                base,
                candidate,
                ..
            } => format!(
                "prepared  {path} at {base}, candidate {} bytes",
                candidate.bytes
            ),
            Event::Approved { .. } => {
                "approved  operator grant + signed approval retained".to_owned()
            }
            Event::Applied { .. } => format!(
                "applied   candidate installed in {}",
                trial.worktree(run).display()
            ),
        };
        out.push_str(&format!("  {}. {line}\n", record.sequence.get()));
    }
    Ok(out)
}

pub fn main(args: &[String]) -> Result<String> {
    let parsed = parse(args)?;
    let state = parsed.state()?;
    match parsed.positional.first().map(String::as_str) {
        Some("init") => {
            Trial::init(&state)?;
            Ok(format!("initialized trial state in {}", state.display()))
        }
        Some("prepare") => {
            let report = parsed.option("report").map(Path::new);
            let p = prepare(
                &state,
                Path::new(parsed.required("repo")?),
                parsed.required("file")?,
                Path::new(parsed.required("recipe")?),
                report,
            )?;
            let mut out = format!(
                "prepared run {}\n  file {} at {}\n  prepared edit {}\n",
                p.run, p.path, p.base, p.prepared
            );
            for (lo, hi, inserted) in &p.edits {
                out.push_str(&format!("  splice [{lo},{hi}) -> {inserted} bytes\n"));
            }
            out.push_str(&format!("next: writfold trial approve {}", p.run));
            Ok(out)
        }
        Some("approve") => {
            let run = parsed.run()?;
            let expires = approve(&state, run)?;
            Ok(format!(
                "approved run {run} until {expires} (unix seconds)\nnext: writfold trial apply {run}"
            ))
        }
        Some("apply") => {
            let run = parsed.run()?;
            let worktree = apply(&state, run)?;
            Ok(format!(
                "applied run {run}\n  inspect: git -C {} diff",
                worktree.display()
            ))
        }
        Some("show") => show(&state, parsed.run()?),
        _ => Ok(USAGE.to_owned()),
    }
}
