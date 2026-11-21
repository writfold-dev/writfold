//! `writfold watch`: a read-only projection of the KB verbs.
//!
//! The viewer displays recorded plan steps, gate standings, local inbox entries
//! and checkpoint history. It never judges a gate, starts a checker or writes.
//! A pipe or `--once` receives a plain, sectioned frame.
#![forbid(unsafe_code)]

pub mod app;
mod bar_content;
pub mod bars;
pub mod cells;
mod chrome;
pub mod commands;
pub mod frame;
pub mod graph;
pub mod guards;
pub mod help;
pub mod inspect;
mod inspect_view;
pub mod kb_bars;
pub mod kb_entry;
mod kb_follow;
mod kb_help;
mod kb_ink;
mod kb_inspect;
mod kb_keys;
mod kb_list;
mod kb_load;
pub mod kb_plain;
pub mod kb_plain_inspect;
mod kb_projects;
mod kb_rail;
mod kb_sections;
mod kb_source;
pub mod kb_state;
pub mod kb_tree;
mod kb_view;
mod keys;
pub mod layout;
pub mod lines;
mod list_view;
mod map_view;
pub mod measure;
mod nav;
mod paint;
mod palettes;
mod plain_bars;
mod plain_views;
pub mod poll;
pub mod progress;
pub mod rail;
mod rail_fold;
mod rail_text;
mod run_item;
pub mod say;
pub mod say_diag;
pub mod say_draw;
pub mod say_gate;
pub mod say_glossary;
pub mod say_kb;
pub mod say_lists;
pub mod say_next;
pub mod say_read;
pub mod say_row;
pub mod say_shape;
pub mod say_step;
pub mod say_words;
pub mod side;
pub mod source;
mod state;
pub mod style;
mod term;
pub mod theme;
#[cfg(test)]
mod theme_tests;
pub mod view;
mod views;
mod wrap;

use app::{App, View};
use kb_state::KbView;
use source::Source;
use std::path::PathBuf;
use style::{Environment, Look};

pub const USAGE: &str = "usage: writfold watch [--kb DIR] [--repo DIR] [--run ID]
                      [--view NAME] [--theme NAME] [--color auto|truecolor|ansi|none] [--once]

Views: progress (default), gates, timeline, inbox, guards, stale, brief, resume;
knowledge base: kb:why, kb:status, kb:tracker, kb:impact, kb:stale (m switches modes).
--once or a pipe prints one plain frame; --view selects just one section.";

/// The parsed arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub kb: Option<PathBuf>,
    pub repo: Option<PathBuf>,
    pub run: Option<String>,
    pub view: Option<View>,
    pub kb_view: Option<KbView>,
    pub theme: Option<String>,
    pub color: Option<String>,
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
            "--theme" => options.theme = Some(value("--theme")?),
            "--color" => options.color = Some(value("--color")?),
            "--view" => {
                let name = value("--view")?;
                if let Some(kb) = name.strip_prefix("kb:").and_then(KbView::from_name) {
                    options.kb_view = Some(kb);
                    continue;
                }
                options.view = Some(View::from_name(&name).ok_or_else(|| {
                    let mut names: Vec<_> = View::ALL.iter().map(|v| v.name().to_owned()).collect();
                    names.extend(KbView::ALL.iter().map(|v| format!("kb:{}", v.name())));
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

/// The plain result and its optional warning; interactive warnings use the footer.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Option<String>,
    pub warning: Option<String>,
}

fn look(options: &Options, env: &Environment) -> Result<(Look, Option<String>), String> {
    let depth = style::depth(options.color.as_deref(), env)?;
    let config = if options.theme.is_none() && env.theme.is_none() {
        writfold_kb::workspace::user_config("theme")
    } else {
        None
    };
    let (theme, warning) = style::choose_theme(
        options.theme.as_deref(),
        env.theme.as_deref(),
        config.as_deref(),
    );
    Ok((Look { theme, depth }, warning))
}

/// Run `watch` with the environment captured once by the binary.
pub fn main(
    args: &[String],
    cwd: PathBuf,
    terminal: bool,
    env: Environment,
) -> Result<Outcome, String> {
    let options = parse(args)?;
    if options.help {
        let marks = help::marks(cells::Glyphs::Unicode);
        let help: Vec<_> = marks
            .into_iter()
            .chain(frame::help())
            .map(|line| line.text)
            .collect();
        return Ok(Outcome {
            stdout: Some(format!("{USAGE}\n\n{}", help.join("\n"))),
            warning: None,
        });
    }
    let (look, warning) = look(&options, &env)?;
    let source = Source::located(options.kb, cwd, options.repo, None)?;
    let mut app = App::new(source, options.run.as_deref());
    app.look = look;
    app.dumb = env.term.as_deref() == Some("dumb");
    if let Some(view) = options.view {
        app.show(view);
    }
    if let Some(error) = &app.runs.error {
        return Err(error.clone());
    }
    if let Some(view) = options.kb_view {
        app.enter_kb();
        app.kb_show(view);
    }
    if options.once || !terminal {
        let stdout = match options.kb_view {
            Some(view) => frame::once_kb(&mut app, view),
            None => frame::once(&mut app, options.view),
        };
        return Ok(Outcome {
            stdout: Some(stdout),
            warning,
        });
    }
    app.notice = warning;
    term::run(&mut app).map_err(|e| format!("terminal: {e}"))?;
    Ok(Outcome {
        stdout: None,
        warning: None,
    })
}
