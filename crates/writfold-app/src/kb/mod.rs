//! `writfold kb`: Git-native project continuity reads (KB v1). Argument
//! parsing and exit codes only; every decision lives in `writfold-kb`.
pub mod args;
mod checkpoint;
mod cite;
mod deliver;
mod edit;
mod gate;
mod hook;
mod plan;
mod proposals;
mod set;
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
  repos [--missing]                each cited repository: local, partial or missing, how its clone
                                   was found, its target commit, when it last fetched (exit 2 if missing)
  index status|rebuild|verify      what the derived index holds; rebuild it; check it equals a rebuild
  check [<project>|<path>]         stale, index verify and the current run's gate status in one
                                   summary (the worst exit code of the three)
  runs [--open] [--project P]      every run: state, where it works, last checkpoint
  resume [<run>]                   one run: manifest, coverage, rechecks, checkpoint tail

writes (atomic; they leave the work tree dirty for you to commit):
  checkpoint [--run ID | --new-run --project P --task T [--mode M]] --note TEXT|--from-draft SESSION
  record ticket <ID> --title T [--assets a,b] [--status S] [--summary S] [--prs p,q]
  record decision --title T --target-source S [--project P] [--references a,b]
                  [--context ..] [--evidence ..] [--change ..] [--why ..] [--origin O]
  record link <type> <from> <to> [--status S] [--evidence e1,e2] [--applicability A]
  status <id> --set STATE [--by WHO] [--superseded-by ID] [--note TEXT]
                  [--commit REPO@SHA] [--evidence REF [--run ID] [--repo DIR]]
  cite make <repo> <path> <a-b> [--allow-dirty] [--sym S] [--ttl T]   (prints a token)
  cite make --kind K key=value …   a checked kind's token from its checker's make (prints it)

checkers (kinds declared freshness = \"checked\"; writfold-check-<name> on PATH; only stale runs them):
  checker list                     each checked kind: its checker found or absent, and its describe

proposals (record calls not yet made, in a local untracked queue; at most 10 per session):
  propose ticket|decision|link … [--session S] [--run ID]
                                   the arguments of record; session: --session, else $WRITFOLD_SESSION
  proposals [list] | proposals show|accept|drop <id>
                                   accept replays it through record (refused on a stale citation)

gates (checks with pinned receipts; definitions in .writfold/gates/<id>.toml):
  gate run <id> [--run ID] [--repo DIR]
                                   run the command, record one receipt in the run
  gate status [--run ID] [--repo DIR]
                                   open, satisfied, stale, failed or unknown, per gate
  gate verify <receipt> [--repo DIR]
                                   re-run the recorded argv at the recorded commit: agree or disagree
  gate attest <id> --statement TEXT --by WHO | gate waive <id> --reason TEXT --by WHO
                                   a statement (attested gates) or a waiver: never shown as satisfied
  gate cover <id> <item> supported REF… | unsupported|not-applicable --reason TEXT, with --by WHO
                                   one checklist item: cited evidence re-grounded on every read
  gate lines [--run ID] [--repo DIR]
                                   receipt lines for a commit message (exit 0 only if all are met)

deliveries (offline candidates: the exact payload, checked mechanically; nothing is sent):
  deliver prepare --mode MODE --from FILE [--audience internal|external]
                  [--destination NAME] [--run ID]
                                   MODE: pr-comment, slack-summary, or .writfold/deliveries/MODE.toml
  deliver check [CANDIDATE] [--check NAME] [--run ID]
                                   run the mode's checks and script gates on the candidate's bytes
  deliver attest|waive [CANDIDATE] --check NAME --statement TEXT|--reason TEXT --by WHO
  deliver status [CANDIDATE] [--run ID]
                                   each candidate's checks (exit 0 only if all are met)

plans (the run graph; templates in .writfold/plans/<name>.toml; Writfold records, the agent walks):
  plan set <template>|--from FILE [--run ID] [--repo DIR]
                                   splice a template's steps at the run's root (keeps a snapshot;
                                   adds the required steps of .writfold/plans/required.toml)
  plan use <template> [--again] [--run ID] [--repo DIR]
                                   splice its steps before the current step, as its children;
                                   a repeat is a no-op (exit 0), --again adds another copy
  plan status [--run ID] [--repo DIR]
                                   each step: done (!stale if a gate no longer holds), current with
                                   what it owes, later; !template-moved if its template changed
  plan advance [--run ID] [--repo DIR]
                                   record the current step as done, or refuse (exit 3) naming what it owes
  plan skip --reason TEXT --by WHO [--run ID]
                                   close the current step without calling it done

edits (the Splicery adapter; Splicery for repeated or structural edits, ordinary tools for small ones):
  edit apply RECIPE.spl [RECIPE.spl ...] [--run ID] [--worktree DIR] [--dry-run]
                                   replay recipes against the run's worktree bytes, write each file
                                   atomically, keep recipe and preview as a receipt in the run
  edit recover [--run ID] [--abandon]
                                   finish (or drop) an edit a crash interrupted between files

harness:
  hook session-start|pre-compact|post-compact|stop|session-end|pre-tool-use
                                   a Claude Code hook: event JSON on stdin; always exits 0;
                                   checkpoint drafts stay local until --from-draft confirms one;
                                   pre-tool-use is the optional delivery guard (off without
                                   .writfold/deliveries/guard.toml)

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
