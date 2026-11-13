//! Argument handling for `writfold tuft`; all executions use a pinned program.
mod args;
mod check;
mod output;
mod program;
mod repos;
#[cfg(test)]
mod tests;

use super::kb::{Options, adapt};
use super::state::Chosen;
use super::{Invocation, execute, execute_kb};
use crate::trial::Result;
use args::{Args, parse};
use std::path::{Path, PathBuf};
use tuft::emit::{Cursor, Format};

pub const USAGE: &str = "\
writfold tuft run FILE|- [--repo DIR|NAME[@COMMIT] ...] [--pinned]
writfold tuft grep PATTERN [--in PATH-GLOB] [--at REF] [--regex] [--limit N]
                         [--repo DIR|NAME[@REF] ...] [--pinned]
writfold tuft check FILE [--json]

Run/grep options: --state DIR --cursor C --verbose|--json --out NEW_FILE
                  --cite --kb DIR --run ID --working-tree --index FILE
                  --print-program (also print the pinned program to stderr)
Saved FILEs and check stay strict: at must be a full lowercase commit ID.
grep and run - accept refs, resolved once and echoed; --pinned forbids refs.
--at is for grep or run - only. Repeated --repo uses the KB repository map;
each repository has its own coverage and cursor, followed by overall coverage.
--working-tree searches tracked and nonignored untracked bytes; changed bytes
are explicitly dirty, never pinned evidence. --index reads universal-ctags JSON
with FILE.meta.json carrying its full commit and sha256; symbols are partial.

State: --state, else $WRITFOLD_STATE, is an initialized trial state. Otherwise
inside a KB Tuft uses <KB git dir>/writfold/tuft-state/; outside a KB it falls
back to ~/.local/share/writfold/trial only when initialized.
--cite emits KB code tokens. An open KB run records pinned retrieval receipts;
--kb/--run require recording to succeed. Notes and programs go to stderr.
Exit codes: 0 successful query/check (including zero hits or partial coverage),
1 invalid arguments/program, refusal or IO error. Always inspect coverage.
Language and JSON contract: docs/tuft/language.md.";

/// Write to a new file, never overwrite.
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

/// Where the KB is discovered, and an optional fixed clock.
pub(crate) struct Context {
    pub cwd: PathBuf,
    pub discover: bool,
    pub now: Option<i64>,
}

fn one(
    parsed: &Args,
    prepared: &repos::Prepared,
    program: &program::Program,
    chosen: &Chosen,
    options: &Options,
) -> Result<(output::Part, Vec<String>)> {
    let cursor = parsed.option("cursor").map(Cursor::parse).transpose()?;
    let (Chosen::Trial(dir) | Chosen::Kb(dir)) = chosen;
    let invocation = Invocation {
        source: &prepared.source,
        name: &program.name,
        repo: &prepared.repo,
        state: dir,
        cursor: cursor.as_ref(),
        format: parsed.format,
        working_tree: parsed.flag("working-tree"),
        index: parsed.option("index").map(Path::new),
    };
    let mut executed = match chosen {
        Chosen::Trial(_) => execute(&invocation)?,
        Chosen::Kb(_) => execute_kb(&invocation)?,
    };
    let mut adapted = adapt(&executed, &prepared.repo, cursor.as_ref(), options)?;
    output::resolved(&mut executed, &mut adapted.printed, prepared)?;
    if parsed.flag("print-program") {
        adapted.notes.push(format!(
            "program repo={} at={}:\n{}",
            prepared.name,
            executed.plan.find.commit,
            std::str::from_utf8(&prepared.source)?
        ));
    }
    Ok((
        output::Part {
            name: prepared.name.clone(),
            printed: adapted.printed,
            trailer: executed.emitted.trailer,
        },
        adapted.notes,
    ))
}

/// Run the language or grep lowering, returning stdout and stderr notes.
pub(crate) fn run_command(args: &[String], context: Context) -> Result<(String, Vec<String>)> {
    let parsed = parse(args)?;
    let program = program::load(&parsed)?;
    if let Some(out) = parsed.option("out").filter(|p| Path::new(p).exists()) {
        return Err(format!("--out {out}: file exists (refusing to overwrite)").into());
    }
    let prepared = repos::prepare(&parsed, &program, &context)?;
    let chosen = parsed.state(&context)?;
    let options = Options {
        cite: parsed.flag("cite"),
        kb: parsed.option("kb").map(PathBuf::from),
        run: parsed.option("run").map(str::to_owned),
        discover: context.discover,
        cwd: context.cwd,
        now: context.now,
    };
    let (mut parts, mut notes) = (Vec::new(), Vec::new());
    for repo in prepared {
        let (part, messages) = one(&parsed, &repo, &program, &chosen, &options)?;
        parts.push(part);
        notes.extend(messages);
    }
    let (printed, trailer) = output::combine(parts, parsed.format)?;
    let printed = match parsed.option("out") {
        None => printed,
        Some(out) => {
            let view = if parsed.format == Format::Compact {
                printed.strip_suffix(&trailer).unwrap_or(&printed)
            } else {
                &printed
            };
            write_new(Path::new(out), view)?;
            trailer
        }
    };
    Ok((printed, notes))
}

pub fn main(args: &[String]) -> Result<String> {
    let parsed = parse(args)?;
    match parsed.positional.first().map(String::as_str) {
        Some("run" | "grep") => {
            let context = Context {
                cwd: std::env::current_dir()?,
                discover: true,
                now: None,
            };
            let (printed, notes) = run_command(args, context)?;
            use std::io::Write as _;
            let mut stderr = std::io::stderr().lock();
            for note in notes {
                writeln!(stderr, "writfold tuft: {note}")?;
            }
            Ok(printed)
        }
        Some("check") => check::check(&parsed),
        _ => Ok(USAGE.to_owned()),
    }
}

/// Process output keeps failed structured checks on stdout, at exit 1.
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

pub fn outcome(args: &[String]) -> Outcome {
    match main(args) {
        Ok(text) => Outcome {
            stdout: format!("{text}\n"),
            stderr: String::new(),
            code: 0,
        },
        Err(error) => match error.downcast_ref::<check::InvalidJson>() {
            Some(json) => Outcome {
                stdout: format!("{}\n", json.0),
                stderr: String::new(),
                code: 1,
            },
            None => Outcome {
                stdout: String::new(),
                stderr: format!("writfold tuft: {error}\n"),
                code: 1,
            },
        },
    }
}
