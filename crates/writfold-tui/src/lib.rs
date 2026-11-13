//! `writfold watch`: a read-only projection of the KB verbs.
//!
//! The viewer displays recorded plan steps, gate standings, local inbox entries
//! and checkpoint history. It never judges a gate, starts a checker or writes.
//! A pipe or `--once` receives a plain, sectioned frame.
#![forbid(unsafe_code)]

pub mod app;
pub mod frame;
mod keys;
pub mod layout;
pub mod lines;
mod paint;
pub mod poll;
pub mod progress;
pub mod side;
pub mod source;
mod state;
mod term;
pub mod view;

use app::{App, View};
use source::Source;
use std::path::PathBuf;

pub const USAGE: &str = "usage: writfold watch [--kb DIR] [--repo DIR] [--run ID]
                      [--view NAME] [--once]

Views: progress (default), gates, timeline, inbox, stale, brief, resume.
--once or a pipe prints one plain frame; --view selects just one section.";

/// The parsed arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub kb: Option<PathBuf>,
    pub repo: Option<PathBuf>,
    pub run: Option<String>,
    pub view: Option<View>,
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
            "--view" => {
                let name = value("--view")?;
                options.view = Some(View::from_name(&name).ok_or_else(|| {
                    let names: Vec<_> = View::ALL.iter().map(|v| v.name()).collect();
                    format!("unknown view {name:?}; views: {}", names.join(", "))
                })?);
            }
            "--once" => options.once = true,
            "--help" | "-h" | "help" => options.help = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

/// One frame as plain text.
pub fn once(source: Source, run: Option<&str>, view: Option<View>) -> String {
    frame::once(&mut App::new(source, run), view)
}

/// Run `writfold watch <args>` in `cwd`: the text to print (for `--once`,
/// a pipe or `--help`), or `None` after an interactive session.
pub fn main(args: &[String], cwd: PathBuf, terminal: bool) -> Result<Option<String>, String> {
    let options = parse(args)?;
    if options.help {
        let help: Vec<_> = frame::help().into_iter().map(|line| line.text).collect();
        return Ok(Some(format!("{USAGE}\n\n{}", help.join("\n"))));
    }
    let source = Source::located(options.kb, cwd, options.repo, None)?;
    let mut app = App::new(source, options.run.as_deref());
    if let Some(view) = options.view {
        app.show(view);
    }
    if let Some(error) = &app.runs.error {
        return Err(error.clone());
    }
    if options.once || !terminal {
        return Ok(Some(frame::once(&mut app, options.view)));
    }
    term::run(&mut app).map_err(|e| format!("terminal: {e}"))?;
    Ok(None)
}
