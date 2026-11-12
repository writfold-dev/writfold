#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

const USAGE: &str = "Usage: writfold kb <verb> (see: writfold kb help)\n       writfold watch [--kb DIR] [--run ID] [--once]\n       writfold serve [--state DIR] [--weft PATH]\n       writfold trial <command> (see: writfold trial help)\n       writfold tuft <command> (see: writfold tuft help)\n       writfold --version\n       writfold --help\n\nserve: bounded JSON-RPC 2.0 over newline-delimited stdio; initialize/shutdown, plus\n       trial/show|retain|prepare|run|evidence-list|evidence-show when writfold.trial/1\n       is negotiated (state: --state or $WRITFOLD_STATE; trial/run Weft checks use the\n       binary fixed by --weft). Approve, apply and recover are CLI-only.\ntrial: prepare, approve and apply one Splicery edit in a Writfold-owned worktree\n       (cooperating single-user profile writfold-trial-worktree/1).\ntuft: run a Tuft v0 retrieval and projection program over one pinned Git commit.\nkb: Git-native project continuity (brief, why, find, impact, resume, checkpoint, record …)\n    and the Claude Code plugin hooks (writfold kb hook <event>).\nwatch: a read-only terminal view of KB runs, gates and receipts (plain text when piped).";

fn main() -> ExitCode {
    let Some(all) = env::args_os()
        .skip(1)
        .map(|a| a.into_string().ok())
        .collect::<Option<Vec<String>>>()
    else {
        return failure("arguments must be valid UTF-8");
    };
    let subcommand: Option<fn(&[String]) -> _> = match all.first().map(String::as_str) {
        Some("trial") => Some(writfold_app::trial::cli::main),
        Some("tuft") => Some(writfold_app::tuft::cli::main),
        _ => None,
    };
    if let Some(subcommand) = subcommand {
        return match subcommand(&all[1..]) {
            Ok(text) => {
                let _ = writeln!(io::stdout().lock(), "{text}");
                ExitCode::SUCCESS
            }
            Err(error) => failure(&error.to_string()),
        };
    }
    if all.first().map(String::as_str) == Some("serve") {
        return serve(&all[1..]);
    }
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

fn serve(args: &[String]) -> ExitCode {
    let options = match writfold_app::serve_options(args) {
        Ok(options) => options,
        Err(message) => return failure(message),
    };
    // The Weft binary is resolved and digested once, before any request.
    let weft = match options
        .weft
        .as_deref()
        .map(writfold_app::trial::run::weft::pin)
    {
        None => None,
        Some(Ok(pinned)) => Some(pinned),
        Some(Err(error)) => return failure(&format!("--weft: {error}")),
    };
    let input = io::stdin();
    let output = io::stdout();
    let state = options.state.as_deref();
    match writfold_app::serve(state, weft.as_ref(), input.lock(), output.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "writfold: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

/// `kb` has its own branch: it exits 0, 2 (partial, unknown or stale),
/// 3 (refused) or 1 (error), where other subcommands only succeed or fail.
fn kb(args: &[String]) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return failure(&format!("working directory: {error}")),
    };
    let outcome = writfold_app::kb::main(args, cwd);
    let _ = io::stdout().lock().write_all(outcome.stdout.as_bytes());
    let _ = io::stderr().lock().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code)
}

/// `watch` prints one frame when stdout is not a terminal (or `--once`).
fn watch(args: &[String]) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return failure(&format!("working directory: {error}")),
    };
    match writfold_tui::main(args, cwd, io::stdout().is_terminal()) {
        Ok(Some(text)) => {
            let _ = writeln!(io::stdout().lock(), "{}", text.trim_end());
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => failure(&format!("watch: {error}")),
    }
}

fn failure(message: &str) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "writfold: {message}");
    ExitCode::FAILURE
}
