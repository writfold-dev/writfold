#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

/// `writfold`: `kb` and `watch` take the rest of the line as their own
/// arguments (each parses it itself); clap handles `--help`, `--version`
/// and a mistyped command (#55).
#[derive(clap::Parser)]
#[command(
    name = "writfold",
    version = writfold_app::VERSION,
    about = "Git-native project continuity for agents",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Top,
}

#[derive(clap::Subcommand)]
enum Top {
    /// brief, why, find, impact, resume, checkpoint, record … and the Claude
    /// Code plugin hooks (writfold kb hook <event>); see: writfold kb help
    #[command(disable_help_flag = true)]
    Kb {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        args: Vec<String>,
    },
    /// a read-only terminal view of KB runs, gates and receipts (plain text
    /// when piped): [--kb DIR] [--repo DIR] [--run ID] [--view NAME]
    /// [--theme NAME] [--color auto|truecolor|ansi|none] [--once]
    #[command(disable_help_flag = true)]
    Watch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        args: Vec<String>,
    },
}

fn main() -> ExitCode {
    let Some(all) = env::args_os()
        .map(|a| a.into_string().ok())
        .collect::<Option<Vec<String>>>()
    else {
        return failure("arguments must be valid UTF-8");
    };
    match <Cli as clap::Parser>::try_parse_from(&all) {
        Ok(Cli {
            command: Top::Kb { args },
        }) => kb(&args),
        Ok(Cli {
            command: Top::Watch { args },
        }) => watch(&args),
        Err(error) => {
            let (text, code) = match error.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    (error.render(), 0)
                }
                _ => (error.render(), 1),
            };
            let colour = env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal();
            let text = match colour {
                true => text.ansi().to_string(),
                false => text.to_string(),
            };
            let written = match code {
                0 => write!(io::stdout().lock(), "{text}"),
                _ => write!(io::stderr().lock(), "{text}"),
            };
            match (written, code) {
                (Ok(()), 0) => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            }
        }
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
