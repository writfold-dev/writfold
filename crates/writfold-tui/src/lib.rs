//! `writfold watch` (T1): a read-only terminal view of KB runs and their
//! progress.
//!
//! It is strictly a projection. Each pane is the JSON answer of one
//! existing verb, asked in-process through [`writfold_kb::run`]: the runs
//! list is `kb runs`, the run pane is `kb resume <run>` headed by `kb gate
//! status --run <run>`'s counts, and the other panes are `kb stale` and
//! `kb brief` for the run's project. Nothing here derives a state, counts
//! an obligation or reads a KB file, and no key writes anything. There is
//! no daemon, so the view never claims that a run is live and shows no
//! progress percentage. The KB pin is polled every two seconds and the
//! verbs are asked again only when it moves. When stdout is not a terminal
//! (or with `--once`) one frame is printed as plain text.
#![forbid(unsafe_code)]

pub mod app;
pub mod lines;
pub mod poll;
pub mod source;
mod term;
pub mod view;

use app::App;
use source::Source;
use std::path::PathBuf;

pub const USAGE: &str = "usage: writfold watch [--kb DIR] [--repo DIR] [--run ID] [--once]

A read-only view of the KB: runs on the left (open first), the selected run's
resume on the right (gates, receipts, drafts, checkpoint tail), the KB pin at
the bottom. Keys: j/k or arrows, Enter, Tab (resume, stale, brief), g s b,
r, ? for help, q to quit. KB and local state are polled every 2 s; the gate
subject every 10 s. --repo selects that subject without changing KB discovery.
--once (automatic when stdout is not a terminal) prints one plain frame.";

/// The parsed arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub kb: Option<PathBuf>,
    pub repo: Option<PathBuf>,
    pub run: Option<String>,
    pub once: bool,
    pub help: bool,
}

/// Parse `watch`'s arguments.
pub fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = |name: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--kb" => options.kb = Some(PathBuf::from(value("--kb")?)),
            "--repo" => options.repo = Some(PathBuf::from(value("--repo")?)),
            "--run" => options.run = Some(value("--run")?),
            "--once" => options.once = true,
            "--help" | "-h" | "help" => options.help = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

/// One frame as plain text.
pub fn once(source: Source, run: Option<&str>) -> String {
    lines::once(&App::new(source, run))
}

/// Run `writfold watch <args>` in `cwd`: the text to print (for `--once`,
/// a pipe or `--help`), or `None` after an interactive session.
pub fn main(args: &[String], cwd: PathBuf, terminal: bool) -> Result<Option<String>, String> {
    let options = parse(args)?;
    if options.help {
        return Ok(Some(USAGE.into()));
    }
    let source = Source::located(options.kb, cwd, options.repo, None)?;
    let mut app = App::new(source, options.run.as_deref());
    if let Some(error) = &app.runs.error {
        return Err(error.clone());
    }
    if options.once || !terminal {
        return Ok(Some(lines::once(&app)));
    }
    term::run(&mut app).map_err(|e| format!("terminal: {e}"))?;
    Ok(None)
}
