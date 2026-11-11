//! `writfold kb`: Git-native project continuity reads (KB v1). Argument
//! parsing and exit codes only; every decision lives in `writfold-kb`.
pub mod args;

use writfold_kb::Request;

pub const USAGE: &str =
    "usage: writfold kb <verb> [args] [--kb DIR] [--budget-tokens N] [--cursor C] [--json]

verbs:
  brief [<project>|--all]          where do we stand (project inferred from the working directory)
  context <id|alias|locator|ticket> [--depth N]
                                   everything around one record
  why <id|text>                    why did we decide X (accepted decisions first)
  stale [<project>|<path>] [--offline]
                                   which citations have moved (local repositories only)
  index status|rebuild             what the derived index holds; rebuild it from Git

The KB root is --kb, else $WRITFOLD_KB, else the nearest directory above the
working directory holding inventory/assets/. Git is $WRITFOLD_GIT or git on PATH.
Exit codes: 0 complete, 2 partial/unknown/stale, 3 refused, 1 error.";

/// The process outcome: text for stdout, text for stderr, exit code.
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

/// Run `writfold kb <args>` in `cwd`.
pub fn main(args: &[String], cwd: std::path::PathBuf) -> Outcome {
    let request: Request = match args::parse(args, cwd) {
        Ok(Some(request)) => request,
        Ok(None) => {
            return Outcome {
                stdout: format!("{USAGE}\n"),
                stderr: String::new(),
                code: 0,
            };
        }
        Err(message) => {
            return Outcome {
                stdout: String::new(),
                stderr: format!("writfold kb: {message}\n{USAGE}\n"),
                code: 1,
            };
        }
    };
    match writfold_kb::run(&request) {
        Ok(output) => Outcome {
            stdout: output.text,
            stderr: String::new(),
            code: output.exit,
        },
        Err(error) => Outcome {
            stdout: String::new(),
            stderr: format!("writfold kb: {error}\n"),
            code: error.exit_code(),
        },
    }
}
