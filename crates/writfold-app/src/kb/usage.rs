//! The `writfold kb` overview (`kb help`), and each verb's lines of it
//! for that verb's `--help`.

pub const USAGE: &str =
    "usage: writfold kb <verb> [args] [--kb DIR] [--budget-tokens N] [--cursor C] [--json]

verbs:
  brief [<project>|--all]          where do we stand (project inferred from the working directory)
  context <id|alias|locator|ticket> [--depth N]
                                   everything around one record
  why <id|text>                    why did we decide X (accepted decisions first)
  status <id|project>              the state of one record and what tracks it
  find <text> [--kind K[,K]] [--project P] [--signatures] [--limit N]
                                   search in tiers: exact ID or alias, normalized name, name
                                   prefix, section heading, BM25 text, substring, one typo
                                   (JSON matched_on names the tier; nothing found may suggest up
                                   to three near names, exit 2); with --kind and no text,
                                   every record of those kinds by ID
  find --each Q… [--kind K] [--project P]
                                   several queries in one call: a query row, then its first five rows
  find --eval [FILE]               run each query of FILE (.writfold/queries.jsonl) as find does:
                                   hit@1, hit@5 or miss per query (exit 0 only if all hit@5)
  match <text>                     find, testing signature patterns first
  impact <id> [--depth N] [--direction in|out|both] [--include inferred]
                                   what does changing X touch (bounded walk, frontier stated)
  uses <target>                    who relies on a record or a code locator
  timeline <id>                    linked investigations, tickets, decisions, checkpoints by date
  stale [<project>|<path>] [--offline|--cached|--record]
                                   which citations have moved; --cached never starts a checker;
                                   --record writes the checkers' answers to .writfold/checks/
  repos [--missing]                each cited repository: local, partial or missing, how its clone
                                   was found, its target commit, when it last fetched (exit 2 if missing)
  index status|rebuild|verify      what the derived index holds; rebuild it; check it equals a rebuild
  check [<project>|<path>]         stale, index verify and the current run's gate status in one
                                   summary (the worst exit code of the three)
  init [DIR] [--example research-policy|delivery] [--dry-run]
                                   make DIR (default: here) a KB: inventory/assets/.gitkeep, and with
                                   --example an editable example policy; create-new only, never commits
  policy                           the policy as it is now (required steps, gates with accept and
                                   item choices, guard rules) and the current run's pin beside it;
                                   identity= is one digest two clones can compare (exit 2 if moved)
  policy --paths GATE              the paths GATE's scope covers now, one per row (the declared scope)
  lint [PATH…] [--repo DIR] [--deny-warnings] [--records]
                                   every fault of the files you wrote (gates, guards, plans,
                                   delivery modes, kinds, queries.jsonl), each at its line and
                                   column; --records: records' front matter, JSON and citation
                                   tokens. Reads files only (exit 0 clean, 2 errors, 1 usage)
  runs [--open] [--project P] [--plan]
                                   every run: state, place, last checkpoint; --plan adds its position
  runs --idle AGE [--project P] [--plan]
                                   open runs whose last recorded write is older than AGE (7d, 36h)
  delta --mark NAME | --since-mark NAME | --since REV
                                   what differs in the KB since a local mark or a commit (exit 2 if
                                   nothing); a mark is local, and session start takes the session's
  resume [<run>]                   one run: manifest, coverage, rechecks, checkpoint tail
  resume <run> --here              first make it the current run of this session and work tree
  resume [<run>] --packet          the bounded handover: required rows first, cut to --budget-tokens
                                   (local pointers only; a closed run is refused)
  guard status                    would guard rules pass, deny, ask or nudge for the current run?
                                   a read-only judgment, not enforcement; no command is run
  guard log [--rule ID] [--since AGE]
                                   what this machine's hook decided per rule, from the local log;
                                   exit 2 if an audit rule would have refused or never matched

writes (atomic; they leave the work tree dirty for you to commit):
  checkpoint [--run ID | --new-run --project P --task T [--mode M]] --note TEXT|--from-draft SESSION
             [--artifact STORE@REV:PATH …]  pointers to what the run made in an attached store
  record ticket <ID> --title T [--assets a,b] [--status S] [--summary S] [--prs p,q]
  record decision --title T --target-source S [--project P] [--references a,b]
                  [--context ..] [--evidence ..] [--change ..] [--why ..] [--origin O]
  record link <type> <from> <to> [--status S] [--evidence e1,e2] [--applicability A]
  record investigation --title T [--trigger TEXT] [--tickets a,b] [--assets a,b]
                  [--alerts a,b] [--signatures a,b] [--summary S] [--question TEXT]
  status <id> --set STATE [--by WHO] [--superseded-by ID] [--note TEXT]
                  [--commit REPO@SHA] [--evidence REF [--run ID] [--repo DIR]]
  cite make <repo> <path> <a-b> [--allow-dirty] [--sym S] [--ttl T]   (prints a token)
  cite make --kind K key=value …   a checked kind's token from its checker's make (prints it)
  observe --source KIND --locator TEXT --status ok|partial|no-data|not-found|timeout|denied|error
          [--revision R] [--for GATE.ITEM] [--note TEXT] [--cite TOKEN] [--searched TOKEN] [--run ID]
          [--snapshot FILE [--store NAME]]   keep the look's bytes in a pinned store (run, commit, run again)
                                   one look at an outside source, in the run (ok/partial embed a token)

checkers (kinds declared freshness = \"checked\"; writfold-check-<name> on PATH; only stale runs them):
  checker list                     each checked kind: its checker found or absent, and its describe

proposals (record calls not yet made, in a local untracked queue; at most 10 per session):
  propose ticket|decision|link|investigation … [--session S] [--run ID] [--tracked|--local]
                                   the arguments of record; session: --session, else $WRITFOLD_SESSION
                                   --tracked: .writfold/proposals/ in the work tree, to commit
                                   (the default when .writfold/proposals/.tracked exists)
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
                                   one checklist item: cited evidence re-grounded on every read;
                                   none-found OBS… --reason TEXT: looks of this run that found nothing
  gate cover <id> --from FILE|- --by WHO
                                   several items from JSON lines, one plan: all rows or none
  gate lines [--run ID] [--repo DIR]
                                   receipt lines for a commit message (exit 0 only if all are met)

deliveries (offline candidates: the exact payload, checked mechanically; nothing is sent):
  deliver modes [MODE]             what each mode asks for: audience, checks, waivers, deny rules, routes
  deliver prepare --mode MODE --from FILE|- [--audience internal|external]
                  [--destination NAME] [--run ID]
                                   MODE: pr-comment, slack-summary, or .writfold/deliveries/MODE.toml
  deliver check [CANDIDATE] [--mode MODE] [--check NAME] [--run ID]
                                   run the mode's checks and script gates on the payload
  deliver attest|waive [CANDIDATE] [--mode MODE] --check NAME --statement TEXT|--reason TEXT --by WHO
  deliver withdraw [CANDIDATE] [--mode MODE] --reason TEXT --by WHO
                                   the candidate is not meant: it decides no step, the guard never passes it
  deliver payload [CANDIDATE] [--mode MODE] [--to FILE]
                                   the bytes to post (citation tokens stripped when the mode says so)
  deliver posted [CANDIDATE] [--mode MODE] --ref TEXT --by WHO
                                   record that the payload was posted (a statement; nothing is fetched)
  deliver match --from FILE|- [--mode MODE] [--run ID]
                                   are these bytes a ready candidate's payload? (exit 0 only if so)
  deliver status [CANDIDATE] [--all] [--run ID]
                                   each candidate's checks (exit 0 only if all are met)

handoff (a receiver's check; nothing is run):
  handoff check --policy REV --candidate REV [--run ID] [--repo DIR]
                                   every gate .writfold/handoff.toml requires at the policy revision,
                                   judged on the candidate commit from the run's receipts under the
                                   policy's definitions: accepted only on a current mechanical pass
                                   for each (exit 0); a waiver, a skip or a changed definition refuses

catalogs (a project's JSON inventory as reviewable asset records; .writfold/catalogs/<name>.toml):
  catalog diff NAME                a Git patch of the asset records the catalog's input wants
                                   (stdout only; nothing is written: review it, then git apply);
                                   an incomplete or conflicting input prints no patch (exit 3)
  catalog check [NAME]             the same without a patch: exit 0 clean, 2 drift, 3 invalid

plans (the run graph; templates in .writfold/plans/<name>.toml; Writfold records, the agent walks):
  plan set <template>|--from FILE [--run ID] [--repo DIR]
                                   splice a template's steps at the run's root (keeps a snapshot;
                                   adds the required steps of .writfold/plans/required.toml)
  plan use <template> [--again] [--run ID] [--repo DIR]
                                   splice its steps before the current step, as its children;
                                   a repeat is a no-op (exit 0), --again adds another copy
  plan status [--run ID] [--repo DIR] [--gates]
                                   each step: done (!stale if a gate no longer holds), current with
                                   what it owes, later; --gates lists gate and checklist item standings
  plan again [--run ID] [--repo DIR]
                                   record the next round of the current loop step (loop=<template>
                                   max=N [min=N]); refused once its gates are met or its rounds used
  plan advance [--run ID] [--repo DIR]
                                   record the current step as done, or refuse (exit 3) naming what it owes
  plan skip --reason TEXT --by WHO [--run ID]
                                   close the current step without calling it done
  plan repin --reason TEXT --by WHO [--run ID] [--repo DIR]
                                   adopt the policy as it is now when it moved since the run pinned it

harness:
  hook session-start|pre-compact|post-compact|stop|session-end|pre-tool-use|post-tool-use
                                   a Claude Code hook: event JSON on stdin; always exits 0;
                                   checkpoint drafts stay local until --from-draft confirms one;
                                   pre-tool-use and stop apply guard rules in .writfold/guards/
                                   and the delivery guard (.writfold/deliveries/guard.toml);
                                   with neither, they open nothing; post-tool-use names deliver posted

The KB root is --kb, else $WRITFOLD_KB, else the nearest directory above the
working directory holding inventory/assets/. Git is $WRITFOLD_GIT or git on PATH.
Without --run, the current run is the session's ($WRITFOLD_SESSION; the session-start
hook exports it), else the enclosing work tree's, else the clone's shared pointer.
Exit codes: 0 complete, 2 partial/unknown/stale (or a gate not satisfied), 3 refused
(or write conflict), 1 error.";

/// The lines of [`USAGE`] that describe `verb`: each entry starting with
/// it, with its continuation lines.
pub fn section(verb: &str) -> String {
    let mut out = Vec::new();
    let mut inside = false;
    for line in USAGE.lines() {
        let entry = line.strip_prefix("  ").filter(|l| !l.starts_with(' '));
        match entry {
            Some(text) => inside = text.split([' ', '|']).next() == Some(verb),
            None if !line.starts_with("   ") => inside = false,
            None => {}
        }
        if inside {
            out.push(line);
        }
    }
    out.join("\n")
}
