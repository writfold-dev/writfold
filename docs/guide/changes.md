# Behaviour changes by release

> **Status:** kept with each release from v0.6.0-dev.6 on; the entries for v0.6.0-dev.1 to dev.5 were gathered from their release notes on 17 November 2026. Each line names something that was built in the release it is listed under.

This page is for someone pinned to one release who wants to upgrade on purpose. It lists only what an existing setup can notice: a different answer, exit code, file or default for input that was already valid. New verbs and new optional keys are not listed unless they change what an old command does; the release notes have those. Within the v0.6.0 pre-releases the entries add up: upgrading from v0.5.0-dev.9 to the newest means reading the whole page.

A line marked **fix** changed an answer that was wrong; anything depending on the old answer was depending on the fault.

## v0.6.0-dev.6

- **The current run is resolved per session, then per work tree, then from the shared pointer.** Before, one pointer per KB clone (`<KB git dir>/writfold/current-run`) decided for every hook and every verb without `--run`. Now a session's own pointer decides first, then the pointer of the work tree the command runs (or acts) in, and the shared pointer only when neither is set ([hooks](hooks.md#the-current-run-per-session-and-per-work-tree)). For one session in one repository nothing changes. What can be noticed:
  - Starting a run in one repository no longer changes the current run of a session or shell in another repository whose own pointer is set. A second session's `checkpoint --new-run` no longer moves the first session's run.
  - A script that set the current run by writing the `current-run` file is no longer read by a session or work tree that has its own pointer. Use `writfold kb resume <run> --here`, or delete `<KB git dir>/writfold/current-run.d/`.
  - A session whose run was closed is told the run is closed, even when the shared pointer names an open run. Before, with one pointer, there was no second run to fall back to, so this is the same answer as before; the change is that it is not replaced by another session's run.
  - `writfold watch` and verbs run inside the KB's own work tree read the shared pointer as before.
- **`checkpoint` writes up to three pointer files** (`current-run.d/session-<id>`, `current-run.d/tree-<hex>`, `current-run`) where it wrote one. They are local and untracked.
- **The `SessionStart` hook writes to `CLAUDE_ENV_FILE`** when Claude Code names one: one `export WRITFOLD_SESSION=<session_id>` line. Verbs run later in that session's shell read it: `propose` then counts its ten per session against that session (before, a shell with no `--session` shared one bucket), and run writes go to the session's run.
- **A checkpoint draft's `run`** is the session's current run, not the clone's.
- **`guard status`** adds one note when a pointer narrower than the shared one decided.
- **`plan repin` is built.** A run that is `policy-moved` still reads so, and still exits 2 from `plan status`, until it is repinned; the note that says so now names the verb. A repinned run never reads plain `complete` (`complete-with-exceptions`). After a repin, the pin in force for a gate (and for the required file) is the newest one naming it; before, the required file was always compared with the first pin. A done step whose record names another digest than its gate's now is kept at its own digest (the new `steps` key of a repin pin); an earlier binary ignores the key and may still read such a run `moved`.
- `find a b c` is unchanged (one phrase); several queries are `find --each a b c`.

## v0.6.0-dev.5

- `deliver match` shows the same candidate on every run when several hold the same bytes: one with a post record about its present bytes, else the newest postable one, else the newest. Counts and exit codes are unchanged.

## v0.6.0-dev.4

- Assessed and delivery check receipts gain `seq` and are ordered by (`seq`, name), not by their recorded time. Older receipts are read as before.
- An older draft no longer meets a later step once a newer draft of that mode is bound.
- A guard route whose posted text cannot be read is denied, naming the fix; before, another ready draft could let it pass.
- `gate lines` ends with the plan's summary word, lists a skipped step's gates as `skipped`, and exits like the summary.
- **fix** A second clone or a linked worktree on the same machine is judged in its own checkout; before, a check could read `satisfied` from the first clone.
- **fix** A delivery mode that declares no checks is never ready.
- **fix** A `*` in a scope pattern matches a literal `*` in a file name.

## v0.6.0-dev.3

- `plan status` exits 2 while a run's policy has moved since it was pinned (`policy=moved`): a gate definition, an instruction file or the required template was edited after the run's plan was set.
- `plan status` gains a plan row and `policy=`, `summary=` and `repinned=` in the trailer; a plan with an attested, waived or skipped step reads `complete-with-exceptions`, not `complete`.
- Each plan write (`plan set`, `plan use`) adds a pin file under `runs/<run>/policy/`.
- Closing a run with `status RUN --set closed --note …` keeps the note and the run's open obligations; a closed run is never reported complete.
- A receipt from another run or another subject counts as absent, not as stale evidence for this one.

## v0.6.0-dev.2

- A faulty definition file (gate, guard rule, plan template, delivery mode, deny list, kind declaration, `queries.jsonl`) reports every fault it has, each with a stable code, on stderr; before, the first fault only, as one line. `--json` answers gain a `diagnostics` array.
- `kb check` gains a `check definitions` row.
- **fix** A gate with a wrong-typed `version` is reported as that, not as a bad `timeout_seconds`.

## v0.6.0-dev.1

- **`gates.met` over an empty set no longer passes.** A guard rule requiring it in a repository with no gate, or in a run whose plan names none, takes its action. Leave such a repository out with `repos`.
- A `commands` rule reads the command line as the shell splits it. A here-document body no longer matches (before: a false refusal); `git -C DIR commit`, `sudo`, `env`, `command`, `time` and a full path to `git` now match (before: a silent miss); `git commit --help` passes.
- Each matched command is judged in the repository it acts on (`cd X &&`, `git -C`), not the event's working directory. A directory that cannot be told takes the rule's action.
- A linked worktree answers to its repository's name in `repos`.
- A passed `claims.cited-fresh` delivery check stands only while its citations are fresh; it is re-checked on every read.
- The `trial`, `serve` and `tuft` subcommands are removed; `--version` prints only the version.
- The KB root is no longer read from an earlier configuration file: use `--kb`, `$WRITFOLD_KB` or the plugin's `kb_root`.
