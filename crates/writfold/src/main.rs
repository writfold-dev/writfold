#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

const USAGE: &str = "Usage: writfold kb <verb> (see: writfold kb help)\n       writfold watch [--kb DIR] [--repo DIR] [--run ID] [--view NAME] [--theme NAME] [--color auto|truecolor|ansi|none] [--once]\n       writfold --version\n       writfold --help\n\nkb: Git-native project continuity (brief, why, find, impact, resume, checkpoint, record …)\n    and the Claude Code plugin hooks (writfold kb hook <event>).\nwatch: a read-only terminal view of KB runs, gates and receipts (plain text when piped).";

fn main() -> ExitCode {
    let Some(all) = env::args_os()
        .skip(1)
        .map(|a| a.into_string().ok())
        .collect::<Option<Vec<String>>>()
    else {
        return failure("arguments must be valid UTF-8");
    };
    if all.first().map(String::as_str) == Some("kb") {
        return kb(&all[1..]);
    }
    if all.first().map(String::as_str) == Some("watch") {
        return watch(&all[1..]);
    }
    let mut arguments = env::args_os().skip(1);
    let command = arguments.next();
    if arguments.next().is_some() {
        return failure("expected exactly one command; use --help");
    }
    match command.as_ref().and_then(|value| value.to_str()) {
        Some("--version") => {
            if writeln!(io::stdout().lock(), "writfold {}", writfold_app::VERSION).is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("--help" | "-h") => {
            if writeln!(io::stdout().lock(), "{USAGE}").is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => failure("unknown or missing command; use --help"),
    }
}

/// `kb` has its own branch: it exits 0, 2 (partial, unknown or stale),
/// 3 (refused) or 1 (error).
fn kb(args: &[String]) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return failure(&format!("working directory: {error}")),
    };
    let plain = env::var_os("NO_COLOR").is_some() || env::var("TERM").is_ok_and(|t| t == "dumb");
    let style = |terminal: bool| writfold_app::kb::Style {
        colour: terminal && !plain,
        width: match terminal {
            true => writfold_tui::measure::columns().unwrap_or(100).max(40),
            false => 100,
        },
        cell: writfold_tui::measure::cell,
    };
    let styles = writfold_app::kb::Styles {
        stdout: style(io::stdout().is_terminal()),
        stderr: style(io::stderr().is_terminal()),
    };
    let outcome = writfold_app::kb::main_styled(args, cwd, &styles);
    #[expect(
        clippy::let_underscore_must_use,
        reason = "exiting; no channel is left to report to"
    )]
    let _ = io::stdout().lock().write_all(outcome.stdout.as_bytes());
    #[expect(
        clippy::let_underscore_must_use,
        reason = "exiting; no channel is left to report to"
    )]
    let _ = io::stderr().lock().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code)
}

/// `watch` prints one frame when stdout is not a terminal (or `--once`).
fn watch(args: &[String]) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return failure(&format!("working directory: {error}")),
    };
    let environment = writfold_tui::style::Environment {
        colorterm: env::var("COLORTERM").ok(),
        no_color: env::var_os("NO_COLOR").is_some(),
        term: env::var("TERM").ok(),
        theme: env::var("WRITFOLD_THEME").ok().filter(|t| !t.is_empty()),
    };
    match writfold_tui::main(args, cwd, io::stdout().is_terminal(), environment) {
        Ok(outcome) => {
            if let Some(warning) = outcome.warning {
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "stderr itself failed; nowhere to report"
                )]
                let _ = writeln!(io::stderr().lock(), "{warning}");
            }
            if let Some(text) = outcome.stdout {
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "exiting; no channel is left to report to"
                )]
                let _ = writeln!(io::stdout().lock(), "{}", text.trim_end());
            }
            ExitCode::SUCCESS
        }
        Err(error) => failure(&format!("watch: {error}")),
    }
}

fn failure(message: &str) -> ExitCode {
    #[expect(
        clippy::let_underscore_must_use,
        reason = "stderr itself failed; nowhere to report"
    )]
    let _ = writeln!(io::stderr().lock(), "writfold: {message}");
    ExitCode::FAILURE
}
