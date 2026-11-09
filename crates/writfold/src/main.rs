#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "Usage: writfold serve\n       writfold trial <command> (see: writfold trial help)\n       writfold --version\n       writfold --help\n\nserve: bounded JSON-RPC 2.0 over newline-delimited stdio; initialize/shutdown only.\ntrial: prepare, approve and apply one Splicery edit in a Writfold-owned worktree\n       (cooperating single-user profile writfold-trial-worktree/1).";

fn main() -> ExitCode {
    let Some(all) = env::args_os()
        .skip(1)
        .map(|a| a.into_string().ok())
        .collect::<Option<Vec<String>>>()
    else {
        return failure("arguments must be valid UTF-8");
    };
    if all.first().map(String::as_str) == Some("trial") {
        return match writfold_app::trial::cli::main(&all[1..]) {
            Ok(text) => {
                let _ = writeln!(io::stdout().lock(), "{text}");
                ExitCode::SUCCESS
            }
            Err(error) => failure(&error.to_string()),
        };
    }
    let mut arguments = env::args_os().skip(1);
    let command = arguments.next();
    if arguments.next().is_some() {
        return failure("expected exactly one command; use --help");
    }
    match command.as_ref().and_then(|value| value.to_str()) {
        Some("serve") => {
            let input = io::stdin();
            let output = io::stdout();
            match writfold_app::serve(input.lock(), output.lock()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    let _ = writeln!(io::stderr().lock(), "writfold: {error}");
                    ExitCode::from(error.exit_code())
                }
            }
        }
        Some("--version") => {
            if writeln!(
                io::stdout().lock(),
                "writfold {}\nprofile={}\nscope=transport-control-only",
                writfold_app::VERSION,
                writfold_app::PROFILE
            )
            .is_ok()
            {
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

fn failure(message: &str) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "writfold: {message}");
    ExitCode::FAILURE
}
