//! Argument handling for `writfold tuft ...`; returns the text to print.
use super::kb::{Options, adapt};
use super::{Invocation, compile, execute};
use crate::trial::Result;
use crate::trial::state_dir::{StateDir, Uses};
use std::path::{Path, PathBuf};
use tuft::emit::{Cursor, Format};

pub const USAGE: &str = "\
writfold tuft run FILE --repo DIR [--state DIR] [--cursor C] [--verbose|--json]
                  [--out NEW_FILE] [--cite] [--kb DIR] [--run ID]
                     run a Tuft v0 program over the pinned commit it names: exact
                     find, retained citation bindings, a budgeted projection.
                     Default rows are `path:line: text` plus one `# coverage=...`
                     trailer; --out writes the view to a new file and prints
                     only the trailer
writfold tuft check FILE
                     compile only; print diagnostics or the checked pipeline

Options: --state DIR (default: $WRITFOLD_STATE, else ~/.local/share/writfold/trial),
the trial state that holds the admitted Git executable and retained evidence.
--cite prints, under each shown row, the code citation token `writfold kb cite make`
would print for its lines at the pinned commit. When the KB (found as `writfold kb`
finds it) has an open current run, or with --kb DIR / --run ID, the run also
records a retrieval receipt (program, commit, trailer, row counts, tokens) in
runs/<id>/retrievals/; `writfold kb resume` shows it. Notes go to standard error.
Language: docs/tuft/language.md. Semantic, fuzzy and ranked search, and `follow`,
are refused until the Omnigraph backend exists.";

struct Args {
    positional: Vec<String>,
    options: Vec<(String, String)>,
    format: Format,
    cite: bool,
}

fn parse(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        positional: Vec::new(),
        options: Vec::new(),
        format: Format::Compact,
        cite: false,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let format = match arg.as_str() {
            "--json" => Some(Format::Json),
            "--verbose" => Some(Format::Verbose),
            _ => None,
        };
        if let Some(format) = format {
            if parsed.format != Format::Compact {
                return Err("give at most one of --verbose and --json".into());
            }
            parsed.format = format;
        } else if arg == "--cite" {
            if parsed.cite {
                return Err("--cite given twice".into());
            }
            parsed.cite = true;
        } else if let Some(name) = arg.strip_prefix("--") {
            if !matches!(name, "repo" | "state" | "cursor" | "out" | "kb" | "run") {
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

    fn state(&self) -> Result<StateDir> {
        let dir = StateDir::resolve(self.option("state"))?;
        dir.require_initialized(Uses::AnyState)?;
        Ok(dir)
    }
}

/// Write `view` to a file that must not exist yet; never overwrite.
pub(crate) fn write_new(path: &Path, view: &str) -> Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("--out {}: {e} (refusing to overwrite)", path.display()))?;
    file.write_all(view.as_bytes())?;
    Ok(file.sync_all()?)
}

/// How the KB adapter runs: where to look for a KB, and the clock.
pub(crate) struct Context {
    pub cwd: PathBuf,
    pub discover: bool,
    pub now: Option<i64>,
}

/// `writfold tuft run`: the text to print and notes for standard error.
pub(crate) fn run_command(args: &[String], context: Context) -> Result<(String, Vec<String>)> {
    let parsed = parse(args)?;
    let file = parsed.file()?;
    let source = std::fs::read(file)?;
    let dir = parsed.state()?;
    let repo = parsed.option("repo").ok_or("missing --repo")?;
    if let Some(out) = parsed.option("out").filter(|o| Path::new(o).exists()) {
        return Err(format!("--out {out}: file exists (refusing to overwrite)").into());
    }
    let cursor = parsed.option("cursor").map(Cursor::parse).transpose()?;
    let executed = execute(&Invocation {
        source: &source,
        name: file,
        repo: Path::new(repo),
        state: &dir.path,
        cursor: cursor.as_ref(),
        format: parsed.format,
    })?;
    let options = Options {
        cite: parsed.cite,
        kb: parsed.option("kb").map(PathBuf::from),
        run: parsed.option("run").map(str::to_owned),
        discover: context.discover,
        cwd: context.cwd,
        now: context.now,
    };
    let adapted = adapt(&executed, Path::new(repo), cursor.as_ref(), &options)?;
    let printed = match parsed.option("out") {
        None => adapted.printed,
        Some(out) => {
            let emitted = &executed.emitted;
            let view = match emitted.format {
                Format::Compact => adapted
                    .printed
                    .strip_suffix(emitted.trailer.as_str())
                    .unwrap_or(&emitted.view)
                    .to_owned(),
                Format::Verbose | Format::Json => adapted.printed,
            };
            write_new(Path::new(out), &view)?;
            emitted.trailer.clone()
        }
    };
    Ok((printed, adapted.notes))
}

pub fn main(args: &[String]) -> Result<String> {
    let parsed = parse(args)?;
    match parsed.positional.first().map(String::as_str) {
        Some("run") => {
            let context = Context {
                cwd: std::env::current_dir()?,
                discover: true,
                now: None,
            };
            let (printed, notes) = run_command(args, context)?;
            if !notes.is_empty() {
                use std::io::Write as _;
                let mut stderr = std::io::stderr().lock();
                for note in notes {
                    let _ = writeln!(stderr, "writfold tuft: {note}");
                }
            }
            Ok(printed)
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
