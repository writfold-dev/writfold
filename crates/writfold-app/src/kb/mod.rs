//! `writfold kb`: Git-native project continuity reads (KB v1). Argument
//! parsing and exit codes only; every decision lives in `writfold-kb`.
pub mod args;
mod checkpoint;
mod gate;
mod hook;
mod verbs;
mod writes;

use writfold_kb::Request;

pub const USAGE: &str =
    "usage: writfold kb <verb> [args] [--kb DIR] [--budget-tokens N] [--cursor C] [--json]

verbs:
  brief [<project>|--all]          where do we stand (project inferred from the working directory)
  context <id|alias|locator|ticket> [--depth N]
                                   everything around one record
  why <id|text>                    why did we decide X (accepted decisions first)
  status <id|project>              the state of one record and what tracks it
  find <text> [--kind K[,K]] [--project P] [--signatures]
                                   search: exact ID, BM25 text, substring, one-typo
  match <text>                     find, testing signature patterns first
  impact <id> [--depth N] [--direction in|out|both] [--include inferred]
                                   what does changing X touch (bounded walk, frontier stated)
  uses <target>                    who relies on a record or a code locator
  timeline <id>                    linked investigations, tickets, decisions, checkpoints by date
  stale [<project>|<path>] [--offline]
                                   which citations have moved (local repositories only)
  index status|rebuild|verify      what the derived index holds; rebuild it; check it equals a rebuild
  runs [--open] [--project P]      every run: state, where it works, last checkpoint
  resume [<run>]                   one run: manifest, coverage, rechecks, checkpoint tail

writes (atomic; they leave the work tree dirty for you to commit):
  checkpoint [--run ID | --new-run --project P --task T [--mode M]] --note TEXT|--from-draft SESSION
  record ticket <ID> --title T [--assets a,b] [--status S] [--summary S] [--prs p,q]
  record decision --title T --target-source S [--project P] [--references a,b]
                  [--context ..] [--evidence ..] [--change ..] [--why ..] [--origin O]
  record link <type> <from> <to> [--status S] [--evidence e1,e2] [--applicability A]
  status <id> --set STATE [--by WHO] [--superseded-by ID] [--note TEXT]
  cite make <repo> <path> <a-b> [--allow-dirty] [--sym S] [--ttl T]   (prints a token)

gates (checks with pinned receipts; definitions in .writfold/gates/<id>.toml):
  gate run <id> [--run ID] [--repo DIR]
                                   run the command, record one receipt in the run
  gate status [--run ID] [--repo DIR]
                                   open, satisfied, stale, failed or unknown, per gate
  gate verify <receipt> [--repo DIR]
                                   re-run the recorded argv at the recorded commit: agree or disagree
  gate lines [--run ID] [--repo DIR]
                                   receipt lines for a commit message (exit 0 only if all satisfied)

harness:
  hook session-start|pre-compact|post-compact|stop|session-end
                                   a Claude Code hook: event JSON on stdin; always exits 0;
                                   checkpoint drafts stay local until --from-draft confirms one

The KB root is --kb, else $WRITFOLD_KB, else the nearest directory above the
working directory holding inventory/assets/. Git is $WRITFOLD_GIT or git on PATH.
Exit codes: 0 complete, 2 partial/unknown/stale (or a gate not satisfied), 3 refused
(or write conflict), 1 error.";

/// The process outcome: text for stdout, text for stderr, exit code.
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

/// Run `writfold kb <args>` in `cwd`.
pub fn main(args: &[String], cwd: std::path::PathBuf) -> Outcome {
    if let Some((first, rest)) = args.split_first()
        && first == "hook"
    {
        return hook::main(rest, cwd);
    }
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
