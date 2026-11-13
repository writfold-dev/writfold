# T2: run progress in `writfold watch` — specification

Status: design, not built. Written 13 November 2026 against `main` at `b1ef5c2` (v0.5.0-dev.2); the owner's answers to its open questions are folded in below (section 0a, which overrides any sentence it contradicts). Branch from current `main`: v0.5.0-dev.3 work (guard `paths`/`repeat`, `kb observe`, tracked proposals, checker `max_age`, `kb repos`, batch `gate cover`) lands there in parallel; re-read the files named here before editing them. Nothing in this document has been implemented; every "T2 does" below is a requirement for the implementer.

Audience: an agent implementing T2 in its own worktree. Read first: `AGENTS.md`, `docs/writfold/mission.md`, `docs/writfold/kb-v1-design.md` §18 (T1 as built, including 18.6), §21 and §24 (run graph), §30 (guard rules), §31 (checklist gates), §32 (decision states), §33 (checkers), `docs/guide/json.md`, `docs/guide/run-graph.md`.

## 0. Summary of decisions

| Question | Decision |
|---|---|
| What is "progress" | Facts the records state: steps done, skipped and total; which step is current; which gates it owes and how each stands; which checklist items are covered, open or stale. No percentage, no bar that fills, no ETA, no elapsed or relative time, no liveness word. |
| Where the facts come from | Existing verbs' JSON, in-process (`writfold_kb::run`). Where a verb lacks a field, the verb gains it (section 3); the TUI computes nothing. |
| New verb surface | `plan status --gates`, `runs --plan`, `stale --cached`, `timeline <run>` lists the run's own checkpoint entries and what they mention, and one new read verb `guard status`. The envelope stays `writfold_kb: 1` with no new key. |
| Checkers | `watch` never starts one. T1's stale pane calls `kb stale` without `--offline`, which since §33 does start checkers; T2 fixes this by asking `stale --cached`. A test counts checker starts. |
| Layout | Three columns at 140×32 and above (runs, progress, a side stack of gates, inbox, guards, timeline); two columns from 90 columns; one column from 60; a "too small" line below 60×12. |
| Refresh | T1's KB pin poll (2 s), plus a stat-only stamp of the local state directory (2 s) and a stamp of the subject repository (10 s). Any moved stamp re-asks the verbs. `--repo DIR` is added. |
| Runs list | Each run shows its plan position (`kb runs --plan`). Owed-gate counts are shown for the selected run only, because they depend on a subject repository. |
| Filters | `o` open runs only, `P` the selected run's project only; both are arguments of `kb runs`. |
| Writes | None. `c` (promote a draft) is not bound; the pane prints the verb's own `confirm:` and `accept:` commands. |
| `--once` | A plain frame with every T2 section, golden-tested, never coloured. `--view NAME` picks one section. |
| `watch --json` | Not added. The JSON is the verbs' own; the help lists the verb behind each pane. |
| Themes | Six palettes (hone default, slate, carbide, vellum, paper, paper-dark) ported by value from the owner's editor project strop (MIT, same author), with Writfold's own role mapping and the same contrast minimums. `--theme`, `WRITFOLD_THEME`, `theme = "…"` in the existing `~/.config/writfold/config.toml`; `T` cycles. Palettes need truecolor; otherwise T1's named colours. |

## 0a. Owner decisions (13 November 2026)

These answer section 15 and override any sentence elsewhere in this document that says otherwise.

1. **`status` key on `more` entries.** `gate` and `item` lines (3.1) carry their state in a new optional `status` key on the `more` entry (`"status": "stale"`), in addition to the `<id> [<state>]` text. It is additive under `writfold_kb: 1`: `crates/writfold-kb/tests/json.rs`'s `MORE` set gains `status` as an optional key, present only where a verb sets it; every existing golden stays byte-identical because text output does not change. The TUI reads the key, not the text; `docs/guide/json.md` documents it. Where cheap, also set it on existing `gate status` item lines so both verbs agree (same test as 3.1's equality check).
2. **`guard status`** is kept, as the last slice (3.5); if it slips, T2 ships without the guards pane, as 3.5 says.
3. **Keys:** `g` is gates and resume moves to `e`, as section 6 specifies.
4. **Subject poll:** 10 s, as section 7 specifies.
5. **Unknown `--theme`:** a warning that keeps hone and lists the six names, as 8.4 specifies.
6. **`timeline <run>`** changes by default, with no flag (today it prints only the run's own row).
7. **Gates and merge.** The implementer does not run `scripts/gate` or `writfold kb gate run` (step 7 of section 12): commit on the worktree branch with local `cargo test`, clippy, fmt and the xtask quality report clean (use a private `CARGO_TARGET_DIR` inside the worktree), write §34 and the docs, and report. The integrator reviews, merges to `main`, runs the gates from `main` and releases.
8. **Naming.** No document, comment or commit mentions anything outside Writfold's own terms except the credited theme source in `theme.rs`'s module doc comment.

## 1. Principles

1. **A projection.** Every line on screen is a row, a `more` line, a note or a trailer field of one verb's JSON answer, printed in the verb's own wording, or a fixed label. The TUI may choose which rows to show, in which pane, in what colour, and may hide `more` lines; it does not derive a state, judge a gate, read a KB file or parse a record. The one piece of text it reads inside a field is the bracketed state of a `gate:` or `item:` line (3.1), whose grammar the verb fixes and a test pins.
2. **Honest progress.**
   - Shown: `2 done, 1 skipped of 6 steps`, `current step 4/6`, `owes 2 of 2 gates`, `checklist 2/4 required covered`, each a count the verb printed.
   - Shown: a step strip, one cell per step in plan order (`[x] [!] [-] [>] [ ] [ ]`). Each cell is one step's recorded state. It is not scaled to a width and is omitted when it does not fit.
   - Not shown: a percentage, a filling bar, an ETA, "N minutes ago", elapsed time, a spinner, or any of the words `running`, `active`, `in progress`, `live` about a run. Steps differ in size and nothing records effort, so a ratio would claim something no record says. There is no daemon, so nothing can say an agent is working now.
   - Times are printed only as recorded (`recorded 2026-11-12T10:02:11Z`, checkpoint headings). The footer's `asked HH:MM:SSZ` is the wall time the verbs were last asked, which is a fact about the view, not about a run.
   - A done step is history: `[done] !stale` is shown as the verb shows it and never moves the current step back (run-graph guide, "Done and stale").
   - `attested` and `waived` are drawn in a different colour from `satisfied`: both meet a gate, but one is a statement and the other a check that ran.
3. **Read-only.** No key writes to the KB, the local state directory or the index cache (`stale --cached` skips the verdict cache write that `kb stale` makes). Opening an index session may refresh the derived index, as every read verb does.
4. **Spawns.** Git only, through the verbs and the stamps. Never a checker, a gate command, Weft or a hook.

## 2. Command line

```text
writfold watch [--kb DIR] [--repo DIR] [--run ID] [--view NAME] [--theme NAME]
               [--color auto|truecolor|ansi|none] [--once]
```

- `--kb`, `--run`, `--once`: as T1 (§18.1, §18.3).
- `--repo DIR`: the gate subject. Implemented without touching any verb: the KB is discovered first from the real working directory (`writfold_kb::workspace::discover(kb, cwd)`), then every verb is asked with `kb: Some(<that root>)` and `cwd: DIR` (canonicalised; a directory that does not exist or is not in a Git work tree is a usage error naming it). This is exactly "start `watch` in DIR with the same KB", so `resume`, `plan status`, `gate status` and `guard status` all agree on the subject.
- `--view NAME`: the main pane at start, and the one section `--once` prints after the runs. Names: `progress` (default), `gates`, `timeline`, `inbox`, `guards`, `stale`, `brief`, `resume`. An unknown name is a usage error listing the eight.
- `--theme NAME`, `--color`: section 8.
- An unknown argument is refused, as in T1. `USAGE` in `crates/writfold-tui/src/lib.rs` and the usage text in `crates/writfold/src/main.rs` (line 9) are updated.

Exit codes are T1's: 0 after a session or a printed frame; 1 with the verb's error when the KB cannot be found or `kb runs` errors; a usage error as T1 reports it.

## 3. Verb changes in `writfold-kb` (all additive, envelope unchanged)

Rules for all of them: no key is added to the envelope, a row or a `more` entry (`crates/writfold-kb/tests/json.rs` keeps its `TOP`, `ROW`, `MORE`, `TRAILER` sets unchanged); new information is carried by new `more` labels, new row kinds and new `counts` names, which json.md already allows ("the verb's own counts"). Default output of every existing verb is byte-identical unless stated, so no existing golden changes. Each new flag is parsed in `crates/writfold-app/src/kb/` and listed in `kb help` (`crates/writfold-app/src/kb/mod.rs`). New code goes in new files where the touched file is near its budget.

### 3.1 `plan status --gates`

`PlanVerb::Status { run, repo, gates: bool }` (`crates/writfold-kb/src/plan/mod.rs`); CLI `plan status [--run ID] [--repo DIR] [--gates]` (`crates/writfold-app/src/kb/plan.rs`, switch list in `writes.rs` line 82). Without `--gates` nothing changes. With it:

- Every `step` row that was judged (done steps and the current step; `plan/judge.rs::steps`) gains, after its existing `why:` line and before `read:`, `next:` and `done:`, one line per gate the step names, in the template's order:
  - `label`: `gate`
  - `text`: `<id> [<standing>] <class>`; a checklist gate appends ` <covered>/<required> required covered` (`gate/checklist/judge.rs::View::counts`). `<standing>` is `judge::Gate::shown()` (`satisfied`, `open`, `stale`, `failed`, `unknown`, `attested`, `waived`, `undefined`). `<class>` is the definition's class name; omitted for `undefined`.
  - `path`, `line`: the evidence path (`judge::Gate::evidence`: the standing statement, else the deciding receipt, else the newest covering row) with line 1; absent when there is none.
  - `flags`: `[<standing>]` when the gate is not met, empty when met. This matches `gate status`, whose rows carry the standing as a flag; a met gate carries none here so a clean done step has no flag on any line.
- Directly under a checklist gate's `gate` line, one `item` line per declared item: exactly the `More` that `gate status` prints for it (`gate/checklist/show.rs::item_line`: same text, path, line and flags). To carry them, `plan::judge::Gate` gains `class: &'static str` (or `String`), `counts: Option<(usize, usize)>` and `items: Vec<More>`, filled in `judge::gate` from `Judged::checklist`; `item_line` becomes `pub(crate)`.
- A `later` step gains one `gate` line per gate it names with `text` `<id> [later]`, no path, no flags. Later steps are not judged (they owe nothing yet), and the line says so instead of inventing a standing.
- A `skipped` step gains no `gate` lines.
- Trailer `counts` gain, when a subject was found and a current step exists: `current` (the 1-based position of the current step among the steps), `current_gates` (gates the current step names) and `current_owed` (those not met). With every step closed, `current=0` and the other two are absent. Without a subject (gates not judged) none of the three is printed and the existing note says why.
- Grammar the TUI may rely on, to be stated in `docs/guide/json.md` and pinned by a test: a `gate` or `item` line's `text` starts with `<id> [<state>]`, `<id>` holding no space and `<state>` no `]`.

Put the new lines in a new file `crates/writfold-kb/src/plan/detail.rs` (`plan/show.rs` is 227 lines; do not grow it past 250). `resume_row`, `plan advance`'s refusal, the hooks and `kb check` call `show::rows` without detail and are unchanged.

Tests (`crates/writfold-kb/tests/plan.rs` or a new `plan_detail.rs`, golden `plan-status-gates.txt` and `.json`): the default output is byte-identical to today's golden; with `--gates` a done step lists each gate with its standing, a done step with a stale gate carries `!stale` on the row and `[stale]` in the gate line's flags, the current step lists met and owed gates alike (today's row text names only the owed ones), a checklist gate's `item` lines equal `gate status`'s `item` lines for the same gate (compare the JSON `more` entries), later steps read `[later]`, the three counts are right, and a run-record gate of a done step shows the standing of that step's own window (not the current step's, which is what `gate status` shows).

### 3.2 `runs --plan`

`Verb::Runs { open, project, plan: bool }` (`crates/writfold-kb/src/lib.rs`, `verbs/runs.rs::list`); CLI `runs [--open] [--project P] [--plan]` (`writes.rs` line 50 and 118). With `--plan`, each run whose folded graph is planned (`plan::fold::read(root, runs::dir(&rec), &rec.id)`, `Graph::planned`) gains one `more` line after its `field` line:

- `label`: `plan`
- `text`: `step <i>/<n> done=<d> skipped=<s> current=<step path>`, or `complete steps=<n> done=<d> skipped=<s>` when no step is open
- `path`, `line`: the current step's splice file, line 1 (as `plan status`' current row); absent when complete
- `flags`: the merge-disorder kinds (`plan::disorder::find(&graph).kinds()`), and `invalid` when `graph.invalid` is not empty

The fold only reads the run's `plan/` files: no subject, no Git spawn, no gate is judged. That is why the line carries no owed-gate count: a gate's standing depends on a subject repository, and one list can hold runs of several. A run without a plan gains no line. The trailer gains `counts.planned` (runs listed that hold a plan).

Tests (`crates/writfold-kb/tests/plan.rs`): default `kb runs` output unchanged; `--plan` on a world with a planned run, an unplanned run and a complete one; `<i>/<n>`, `done=` and `skipped=` equal `plan status`'s `counts` for the same run; the line appears in the JSON as a `more` entry with label `plan`.

### 3.3 `stale --cached`

`Verb::Stale { scope, offline, cached: bool }` (about 20 uses across the crates; `check.rs::parts` passes `false`); CLI `stale [<project>|<path>] [--offline] [--cached]`, `--offline` with `--cached` refused as a usage error. With `--cached`, `verbs/stale.rs::stale` calls `stale::check` instead of `stale::check_asking` (`crates/writfold-kb/src/stale/mod.rs` lines 122 and 128), so Git-checked kinds are judged as usual and a checked kind reads the cached verdict or `unverifiable: not-checked` (§33 "Who runs it"); `remember` is skipped, so nothing is written to `cite_checks`; and the answer gains the note `cached: no checker was started; a checked kind shows the verdict the last kb stale cached`. Query digest args gain `cached=<bool>`.

Tests: in `crates/writfold/tests/kb_checker_reads.rs` (the fake checker with a counter file) `stale --cached` starts no checker and leaves `cite_checks` unchanged, while plain `stale` starts one; a checked token never checked reads `not-checked`.

### 3.4 `timeline <run>`: the run's own entries and what they mention

`verbs/timeline.rs::timeline`, only when the resolved record's kind is `run` (other targets unchanged):

- One row per checkpoint entry of the run (`runs::entries(session, &rec)`): `kind` `checkpoint`, `id` the entry's heading (its timestamp), `text` `entry <k> of <n>: <first non-empty line, at most 100 characters>`, `path` the checkpoint file, `line` the entry's first line. They sort with the other events by that timestamp.
- `related` also takes the destination of every edge whose source is a file of the run (`src` starting `doc:runs/<run id>/`, the form `checkpoints()` already reads), so the investigations, tickets and decisions the run's checkpoint entries mention or cite are listed with their dates. Their rows are `record_row`s, so a decision carries its present `[status]` (`accepted`, `applied`, `verified`, …; §32). Add a small query by source prefix in `index/query.rs`.
- `counts` gain `checkpoints` (entries listed).

Tests (`crates/writfold-kb/tests/graph.rs` or where `timeline` is tested; golden `timeline-run.txt`): a run with three entries, one mentioning a decision ID, lists three `checkpoint` rows in order and the decision with its status; a non-run target's output is unchanged.

### 3.5 `guard status` (new read verb)

"Would a guard rule refuse now?" is a progress question (it is what stands between the work and `git commit`), and the facts are already gathered by `guard/require.rs::evaluate(ws, cwd, now, rules)`, which opens the index and spawns Git but never a checker (`gate/ground.rs` reads the cache). There is no verb for it, so T2 adds one rather than evaluating rules in the TUI.

```text
writfold kb guard status [--json]
```

`Verb::Guard(GuardVerb::Status)`; new file `crates/writfold-kb/src/guard/status.rs`; CLI in a new `crates/writfold-app/src/kb/guard.rs`. It reads the rule files as the hook does (`guard::rules::all(&ws.root, &request.cwd)`), evaluates every rule as if it had matched, and decides with the kernel (`writfold_core::guard::decide(true, action, facts)` for `pre-tool-use` rules, `nudge(false, false, facts)` for `stop` rules). It takes no `--run`: guard rules are about the current-run pointer and the working directory, as in the hook.

- One row per rule, in ID order: `kind` `guard`, `id` the rule ID, `status` `would-pass`, `would-deny`, `would-ask`, `would-nudge` or `quiet` (a stop rule that would not nudge), or `not-here` when the rule's `repos` leave this repository out (`Evaluated`'s `None`); `text` `<event> <commands and tools, space-separated> requires <requirements>`; `path` the rule file, line 1; `flags` `deny`, `ask` or `nudge` when not passing. One `owes` line per requirement that is not `Met`, with `require::Owed::text` (it already carries the `next:` command); an `Unknown` fact's line is flagged `unknown`.
- One row per rule file that does not parse: `kind` `guard-rule`, `status` `invalid`, the reason, flag `invalid` (the hook skips these).
- `counts`: `rules`, `deny`, `ask`, `nudge`, `invalid`. Exit 2 when any of the last four is not zero, else 0. With no rule file: `answer=unknown reason=no-guards`, a note naming `.writfold/guards/`, exit 0.
- Notes, always: `judged for the current run <id or none> in <cwd> as if each rule had matched; a guard, not enforcement; nothing was run`. The delivery guard (`guard/delivery.rs`, `.writfold/deliveries/guard.toml`) is a separate mechanism and is not covered and the note says so when `.writfold/deliveries/guard.toml` exists.

Tests (`crates/writfold-kb/tests/guard_rules.rs`, golden `guard-status.txt`): for each world that file already builds, `guard status` says `would-deny` exactly when the hook denies a matching command, and `would-pass` when it lets it through (the same fixture, both asked); `would-ask`; `would-nudge` and `quiet`; `not-here`; an unparsable rule; no rule directory; no checker is started (add it to the counter-file test in `kb_checker_reads.rs`). Document it in `docs/guide/hooks.md` under guard rules and in §30's as-built addendum.

If this verb cannot land with the rest, T2 ships without the guards pane (the view name, key and side pane are simply absent) and §34 lists it under Not done. Nothing else depends on it.

### 3.6 Stamps for a polling viewer

New module `crates/writfold-kb/src/watch.rs` (pure observation; no verb, no index, nothing written):

- `pub fn local(ws: &Workspace) -> String`: a digest of the local state directory (`proposal::state_of(ws)`, i.e. `<git dir>/writfold`): the bytes of `current-run`, and for each file under `draft::dir(state)` and `proposal::dir(state)` its name, length and modification time. Stats and one small read only. Drafts, proposals and the current-run pointer are untracked, so the KB pin never moves when they change; T1 misses them until the next KB change.
- `pub fn subject(ws: &Workspace, tool: &GitTool, cwd: &Path, recorded: Option<&str>) -> Result<Option<Stamp>>` with `Stamp { name, head, dirty: usize, digest }`: locates the subject as the verbs do (`gate::subject::locate` with no explicit repository), runs one `git status` there (the call `index::observe` uses, `kb_status`), and digests HEAD with each dirty path's name, length and modification time. Dirty files are not hashed: a false "moved" costs one re-ask, and `r` remains. `recorded` is the selected run's `worktree=` place field from its `kb runs` row.

Unit tests: `local` changes when a draft or proposal file is added, changed or removed and when `current-run` changes, and not otherwise; `subject` changes on a commit, on an edit to a tracked file and on a new untracked file, and is `None` outside a repository.

### 3.7 The `theme` key of the user configuration

`crates/writfold-kb/src/gate/assessed/weft.rs::configured` already reads `weft = "<path>"` from `$XDG_CONFIG_HOME/writfold/config.toml` (else `~/.config/writfold/config.toml`) with a line reader. Move that reader to `pub fn user_config(key: &str) -> Option<String>` in `crates/writfold-kb/src/workspace.rs` (same file lookup, same `key = "value"` line rule), have `weft.rs` call it for `weft` (behaviour and `tests/assessed` unchanged), and have `writfold-tui` call it for `theme`. No new file, format or dependency.

## 4. Panes

The verb behind each pane, asked with `budget: MAX_BUDGET`, `json: true`, as T1's `Source::call` does.

| Pane | Key | Verb | Shown |
|---|---|---|---|
| Runs | (left) | `kb runs --plan [--open] [--project P]` | T1's entry (state, ID, flags, task, place, last checkpoint) plus the `plan:` line. Not-closed runs first, each group in the verb's order, as T1. |
| Header | (top) | the selected run's `kb runs` row; `watch::subject` | `<id> [state] !flags  <task>  project=…  subject=<name>` |
| Progress | `p`, `1` | `kb plan status --run R --gates` | 4.1 |
| Gates | `g`, `2` | `kb gate status --run R` | T1's `gates:` counts line, then every row as the CLI prints it (receipt, `item:`, `why:`, `read:`, `next:` lines). |
| Timeline | `t`, `3` | `kb timeline R` | Every row in the verb's date order: the run's dates, its checkpoint entries, the records they mention with their present status. |
| Inbox | `i`, `4` | `kb resume R` (rows of kind `draft` and `proposal`) | Each row with its `confirm:` or `accept:` line, under the fixed label `local; not in the KB until confirmed or accepted`. |
| Guards | `u`, `5` | `kb guard status` | Every row and `owes:` line, headed by the counts and `current run <id>`; when the selected run does not carry `!current`, a first dim line `about the current run <id>, not the selected run`. |
| Stale | `s`, `6` | `kb stale <project> --cached` | As T1, plus the verb's `cached:` note. |
| Brief | `b`, `7` | `kb brief <project>` | As T1. |
| Resume | `e`, `8` | `kb resume R` | T1's run pane unchanged (everything `resume` prints), without the `gates:` head line, which moved to Gates. |
| Footer | (bottom) | `kb runs` trailer; the stamps | `kb=<pin> index=<state>  subject=<name>@<head 12>+dirty:<n>  asked <HH:MM:SSZ>  theme=<name>  read-only; no daemon` |

Refusals and errors are T1's: a verb that refuses becomes one `refused (exit N): …` line in its pane and never stops the view. `plan status` with `reason=no-plan` prints the verb's note (`run … has no plan: writfold kb plan set <template> …`); the Progress pane then shows, under it, the Gates pane's content, since a run without a plan owes every defined gate (run-graph guide, first paragraph).

### 4.1 The Progress pane

Lines, top to bottom, all from `plan status --gates`:

1. **Facts line**, from the trailer `counts`: `plan <done> done, <skipped> skipped of <steps> steps; current step <current>/<steps>; owes <current_owed> of <current_gates> gates`. Variants: `…; every step is done or skipped` when `current=0`; `…; gates not judged (no subject repository)` when the three `current*` counts are absent; then ` stale=<n>`, ` moved=<n>`, ` invalid=<n>`, ` disorder=<n>` for each of those counts that is not zero. Toned `Bad` when `stale`, `invalid` or `disorder` is not zero, `Warn` when `current_owed` is, else `Good` only when `current=0`, else `Plain`.
2. **Step strip**: one cell per `step` row in order: `[x]` done, `[!]` done and flagged `stale`, `[-]` skipped, `[>]` current, `[ ]` later. Printed only when `4 × steps + 5` fits the pane's inner width; otherwise omitted (the facts line stands alone). Always printed in `--once`.
3. **One block per step row**, in the verb's order:
   - head: ` <mark> <id> [<status>] <text> !flags` where `<mark>` is `x`, `-`, `>` or a space as above, and the rest is the row as the CLI prints it minus the `@path:line` (the path is shown in the Resume and Gates views and in `--once`; here the width goes to the states);
   - its `more` lines, indented, `gate` lines as `gate <text> !flags` and `item` lines one level deeper as `item <text> !flags`; other labels as `<label>: <text>`.
   - Collapsing: a done step whose row and `more` lines carry no flag shows its head and its `done:` line (when it was recorded); a later step shows only its head. A flagged step, the current step and a skipped step show every line. `x` toggles "expand all". `--once` is always expanded.
4. `plan-event … [invalid]` rows as printed, in `Bad`.
5. The notes, dimmed, as `# note: …`.

When the pane opens or the run changes, the scroll offset is set so the current step's head line is the third visible line when the content is taller than the pane (the line index is known while the lines are built).

## 5. Layout

All sizes are terminal cells; `W×H` is the frame.

| Class | When | Arrangement |
|---|---|---|
| Wide | `W ≥ 140` and `H ≥ 32` | Row 0 header, row 1 view tabs, body, last row footer. Body: runs 40 wide; main pane (the selected view) takes the rest; side stack 48 wide: gates 9 high, inbox 9, guards 7 (only when `H ≥ 38`), timeline the rest (at least 5). The side panes show compact forms: gates as the counts on two lines then one line per `gate` row (`<id> [<status>] !flags`, the verb's order); inbox as the rows of kind `draft` and `proposal`, each on two lines; guards as counts then each row with its first `owes:` line; timeline as the rows, newest at the bottom, scrolled to the end. |
| Medium | `W ≥ 90` and `H ≥ 16`, not wide | Header, tabs, one summary line, body, footer. Body: runs 30 wide; main pane the rest. Summary line: `gates: <each non-zero count other than satisfied> of <gates> \| inbox: drafts=<n> proposals=<n> \| guards: deny=<n> ask=<n> of <rules>` (the first and third from the trailers; the second the number of rows of those kinds). |
| Narrow | `W ≥ 60` and `H ≥ 12`, not medium | Header (`run <k>/<n> <id> [state]`), tabs shortened to the current view's name and `<k>/8`, body the main pane alone, footer `kb=<pin> ? q`. `[` and `]` change the run; `0` shows the runs list full-frame as a ninth view. |
| Too small | otherwise | One line: `writfold watch needs 60x12 (this terminal is WxH)`. Keys still work. |

- In the wide class the side panes are not focusable; their full content is the same-named view (`g`, `i`, `u`, `t`). They are asked for the selected run when it changes, so a wide frame asks five verbs per run per stamp (`plan status`, `gate status`, `resume`, `guard status`, `timeline`); `guard status` does not depend on the run and is cached per stamp.
- Text wraps in the main pane (`Wrap { trim: false }`, as T1); side panes and the runs list truncate.
- Layout selection is a pure function `layout::class(width, height) -> Class` with the rectangles computed by `layout::areas`, unit-tested at the boundaries (139/140, 31/32, 37/38, 89/90, 59/60, 11/12).

### 5.1 Mockup, 160×40 (wide), hone

Schematic: borders are ratatui's box lines; long lines wrap in the main pane rather than cut as drawn here. The mock world is the fixture of section 10.

```text
 run-20261112T090000Z-a1b2c3 [open] !current  Add --plan to kb runs   project=harbor  subject=harbor
 [1 progress] 2 gates 3 timeline 4 inbox 5 guards 6 stale 7 brief 8 resume
+- runs (4) open first ----------------++- progress run-20261112T090000Z-a1b2c3 -------------------------------++- gates (subject harbor) ---------------------+
|> [open] run-20261112T090000Z-a1b2c3 !||plan 2 done, 1 skipped of 6 steps; current step 4/6;                  ||gates=5 satisfied=2 open=2 stale=1 failed=0   |
|    Add --plan to kb runs             ||     owes 2 of 2 gates stale=1                                        ||unknown=0 attested=0 waived=0 invalid=0       |
|    project=harbor worktree=/w/harbor ||     [x] [!] [-] [>] [ ] [ ]                                          || lint [stale] !stale                          |
|    plan: step 4/6 done=2 skipped=1 cu||                                                                      || notes [satisfied]                            |
|    last: 2026-11-12T14:10:00Z Review || x change#1/research [done] notes [satisfied]                         || review-list [open] !open                     |
|  [open] run-20261110T080000Z-0f9e8d  ||     done: recorded 2026-11-12T10:02:11Z                              || signoff [open] !open                         |
|    Rotate the ledger signing key     || x change#1/implement [done] unit [satisfied], lint [stale] !stale    || unit [satisfied]                             |
|    project=harbor                    ||     why: no longer holds: lint [stale] (done stays done; list the gat|+----------------------------------------------+
|    plan: step 1/3 done=0 skipped=0 cu||          again on a later step if it must hold there)                |+- inbox --------------------------------------+
|    last: 2026-11-10T08:30:00Z Read th||     gate unit [satisfied] mechanical                                 ||drafts=1 proposals=2 (local; not in the KB)   |
|  [open] run-20261101T090000Z-77aa10  ||     gate lint [stale] mechanical !stale                              || draft s-41c0 [open] Review started; lint     |
|    Batch ledger writes rollout       ||     done: recorded 2026-11-12T12:40:03Z                              ||   went stale after the rename. !draft        |
|    project=harbor                    || - change#1/benchmarks [skipped] no hot path touched (by owner,       || proposal 9f2c11d0a4b7e6c3 [pending] ticket   |
|    last: 2026-11-03T16:00:00Z Batchin||     2026-11-12T12:41:00Z) !skipped                                   ||   Lint gate scope misses crates/app          |
|  [closed] run-20261104T080000Z-d4e5f6|| > change#1/review [current] owes: review-list [open], signoff [open] || proposal 0b77e2a19c44d5f0 [pending] link     |
|    Retire the legacy flush path      ||     gate review-list [open] checklist 2/4 required covered !open     ||   run -> decision 2026-11-02-runs-plan       |
|    project=harbor                    ||       item errors [supported] 1 reference by agent                   |+----------------------------------------------+
|    plan: complete steps=4 done=4 skip||       item tests [stale] 1 reference by agent; gate:lint is stale !st|+- guards -------------------------------------+
|    last: 2026-11-04T09:00:00Z Legacy ||       item docs [open] The guide names every new flag                ||rules=2 deny=1 ask=0 nudge=1 (current run)    |
|                                      ||       item perf [not-applicable] no hot path touched by agent        || commit-needs-gates [would-deny] git commit   |
|                                      ||       item style [open] (optional) Names follow the module           ||   owes: gate lint [stale]; next: writfold    |
|                                      ||     gate signoff [open] attested !open                               ||     kb gate run lint                         |
|                                      ||     read: instructions for signoff @.writfold/gates/signoff.md:1     || stop-reflect [would-nudge] stop              |
|                                      ||     next: writfold kb gate cover review-list tests supported REF... -|+----------------------------------------------+
|                                      ||   change#1/deliver [later] gates: unit                               |+- timeline -----------------------------------+
|                                      ||   change#1/reflect [later] no gates                                  ||decision proposal/decision/2026-11-02-runs-   |
|                                      ||                                                                      ||  plan [accepted] 2026-11-02 created Show     |
|                                      ||# note: plan change @94d8a4f70bc4 from kb; steps are derived from     ||  plan position in kb runs                    |
|                                      ||  runs/run-20261112T090000Z-a1b2c3/plan/ on every read, never stored  ||checkpoint 2026-11-12T09:00:00Z entry 1 of 3: |
|                                      ||                                                                      ||  Started; read the runs verb.                |
|                                      ||                                                                      ||checkpoint 2026-11-12T12:45:00Z entry 2 of 3: |
|                                      ||                                                                      ||  Implemented --plan; unit passes.            |
|                                      ||                                                                      ||checkpoint 2026-11-12T14:10:00Z entry 3 of 3: |
|                                      ||                                                                      ||  Review started.                             |
|                                      ||                                                                      ||                                              |
+--------------------------------------++----------------------------------------------------------------------++----------------------------------------------+
 kb=be3c99236219+dirty:4 index=fresh  subject=harbor@4c1d2e9a77b0+dirty:1  asked 14:12:07Z  theme=hone  read-only; no daemon  ? help  q quit
```

### 5.2 Mockup, 100×30 (medium)

```text
 run-20261112T090000Z-a1b2c3 [open] !current  Add --plan to kb runs
 [1 progress] 2 gates 3 timeline 4 inbox 5 guards 6 stale 7 brief 8 resume
 gates: open=2 stale=1 of 5 | inbox: drafts=1 proposals=2 | guards: deny=1 ask=0 of 2
+- runs (4) -----------------++- progress ---------------------------------------------------------+
|> [open] ..090000Z-a1b2c3 !c||plan 2 done, 1 skipped of 6 steps; current step 4/6;                |
|    Add --plan to kb runs   ||     owes 2 of 2 gates stale=1                                      |
|    project=harbor worktree=||     [x] [!] [-] [>] [ ] [ ]                                        |
|    plan: step 4/6 done=2 sk||                                                                    |
|    last: 2026-11-12T14:10:0|| x change#1/research [done] notes [satisfied]                       |
|  [open] ..080000Z-0f9e8d   ||     done: recorded 2026-11-12T10:02:11Z                            |
|    Rotate the ledger signin|| x change#1/implement [done] unit [satisfied], lint [stale] !stale  |
|    project=harbor          ||     why: no longer holds: lint [stale] (done stays done; list the g|
|    plan: step 1/3 done=0 sk||          again on a later step if it must hold there)              |
|    last: 2026-11-10T08:30:0||     gate unit [satisfied] mechanical                               |
|  [open] ..090000Z-77aa10   ||     gate lint [stale] mechanical !stale                            |
|    Batch ledger writes roll||     done: recorded 2026-11-12T12:40:03Z                            |
|    project=harbor          || - change#1/benchmarks [skipped] no hot path touched (by owner,     |
|    last: 2026-11-03T16:00:0||     2026-11-12T12:41:00Z) !skipped                                 |
|  [closed] ..080000Z-d4e5f6 || > change#1/review [current] owes: review-list [open], signoff [open|
|    Retire the legacy flush ||     gate review-list [open] checklist 2/4 required covered !open   |
|    project=harbor          ||       item errors [supported] 1 reference by agent                 |
|    plan: complete steps=4 d||       item tests [stale] 1 reference by agent; gate:lint is stale !|
|    last: 2026-11-04T09:00:0||       item docs [open] The guide names every new flag              |
|                            ||       item perf [not-applicable] no hot path touched by agent      |
|                            ||       item style [open] (optional) Names follow the module         |
|                            ||     gate signoff [open] attested !open                             |
|                            ||     read: instructions for signoff @.writfold/gates/signoff.md:1   |
|                            ||     next: writfold kb gate cover review-list tests supported REF...|
+----------------------------++--------------------------------------------------------------------+
 kb=be3c99236219+dirty:4 index=fresh  subject=harbor@4c1d2e9a77b0+dirty:1  hone  read-only  ? q
```

## 6. Keys

| Key | Does |
|---|---|
| `q`, Ctrl-C | quit |
| `?` | help (any key but `q` closes it) |
| `j` `k` `↓` `↑` | move the run selection; with the main pane focused, scroll it one line |
| PgDn, PgUp | scroll the main pane by its height less one, whichever pane has the focus |
| `[` `]` | previous, next run, whichever pane has the focus |
| Enter | move the focus between the runs and the main pane; Esc returns it to the runs |
| Tab, Shift-Tab | next, previous view |
| `1`–`8`; `p` `g` `t` `i` `u` `s` `b` `e` | progress, gates, timeline, inbox, guards, stale, brief, resume |
| `0` | the runs list as a view (narrow class only) |
| `x` | Progress: expand or collapse every step |
| `o` | runs: open only, on and off (`kb runs --open`) |
| `P` | runs: only the selected run's project, on and off (`kb runs --project <p>`); no project on the run: no-op with a footer notice |
| `r` | ask the verbs again now |
| `T` | next theme (hone → slate → carbide → vellum → paper → paper-dark → hone) |

- Changed from T1: `g` was "resume" and is now "gates"; resume is `e`. The help and README say so.
- **`c` is not bound, and no key writes.** T1 listed promoting a draft as the one candidate write key. T2 keeps the view read-only: the view is trusted because it cannot change what it shows; a promote key would need the write protocol, the lock, a confirmation dialog and fault tests inside a viewer; and the Inbox already prints the exact command (`confirm: writfold kb checkpoint --from-draft <id>`, `accept: writfold kb proposals accept <id>`) for the terminal next to it. A test asserts every key leaves the KB tree, the local state directory and the index's `cite_checks` table as they were.
- Filters are arguments of `kb runs`, so a filtered list is still the verb's answer. The runs title shows them: `runs (3) open project=harbor`. When a filter removes the selected run, the selection moves as on a reload (current run, else first). A text search over runs is not built.

## 7. Refresh

- Tick every 2 s (T1's `POLL`). On each tick: the KB pin (`Source::pin`, unchanged) and `watch::local`. On every fifth tick (10 s): `watch::subject` for the selected run. The subject is slower because one `git status` in a large code repository costs more than in the KB; the help states both intervals.
- When any stamp differs from the one the answers were asked at, every cached answer is dropped, `kb runs` and the verbs of the visible panes are asked again, the selection is kept by run ID, the scroll offset is kept if the view did not change, and the frame is redrawn. Otherwise nothing is asked and nothing drawn (T1's rule).
- Changing the selected run takes a new subject stamp at once (its recorded worktree may differ) without dropping other runs' cached answers.
- `r` drops everything and asks again.
- Answers are cached per (run, view) at the current stamps, as T1; `guard status` and the runs list per stamps only.
- The footer shows `asked HH:MM:SSZ` (`writfold_kb::time::stamp` of the wall clock, time part), interactive frames only. It is not in `--once`, whose output must not depend on the clock.
- This closes both refresh items of §18.6: `--repo`, and a subject change showing without `r`.

## 8. Themes and colour

### 8.1 Port

New module `crates/writfold-tui/src/theme.rs`, whose doc comment credits the source: "Palettes ported by value from strop (MIT, same author), `crates/strop-core/src/theme/`; the role mapping and the tests are Writfold's." No dependency on any strop crate; only the tokens below are copied (the editor's syntax, mode, diff-background and ANSI-16 tokens are not needed). Names, order and descriptions follow the source registry.

```rust
pub struct Rgb(pub u8, pub u8, pub u8);
pub struct Palette {
    pub base: Rgb, pub surface: Rgb, pub raised: Rgb, pub selection: Rgb,
    pub text: Rgb, pub secondary: Rgb, pub muted: Rgb, pub faint: Rgb, pub on_accent: Rgb,
    pub accent: Rgb, pub error: Rgb, pub warning: Rgb, pub info: Rgb,
    pub good: Rgb,     // source: diff.add
    pub vouched: Rgb,  // source: mode.visual
}
pub enum ThemeId { Hone, Slate, Carbide, Vellum, Paper, PaperDark }  // Default = Hone
// ALL, name(), description(), is_light(), from_name() (trimmed, ASCII case-insensitive),
// choices() -> "hone, slate, carbide, vellum, paper, paper-dark", palette(), next()
```

| Token | hone | slate | carbide | vellum | paper | paper-dark |
|---|---|---|---|---|---|---|
| base | `15171c` | `1a1c20` | `0b0c0f` | `f6f2e9` | `f8f8f3` | `151d19` |
| surface | `1c1f25` | `202329` | `14161b` | `fdfbf6` | `eef1e8` | `1c2720` |
| raised | `23272f` | `272b32` | `1d2027` | `e5dfd0` | `dbe2d6` | `243229` |
| selection | `30405a` | `343a45` | `2f4a6e` | `d1c9b3` | `c2d2ba` | `2f4b3a` |
| text | `e6e1d6` | `dcd9d2` | `f4f2ec` | `262a30` | `20362d` | `e3e9dd` |
| secondary | `a8adb8` | `aeb1b7` | `cfd3da` | `4b525c` | `3a4d43` | `bfccbe` |
| muted | `858c9a` | `858a93` | `a3a9b4` | `656d7b` | `475951` | `9daa9e` |
| faint | `4a5160` | `545962` | `6a7280` | `aeb3ba` | `a9b8a8` | `53695b` |
| on_accent | `15171c` | `1a1c20` | `0b0c0f` | `f6f2e9` | `f8f8f3` | `151d19` |
| accent | `e8a848` | `c9b27e` | `ffb74a` | `8a510f` | `1a6650` | `9bc7ac` |
| error | `e8707c` | `d98a90` | `ff7b86` | `b8323f` | `9c3f38` | `df9992` |
| warning | `d9a85a` | `cfb27a` | `f0c060` | `8a5d13` | `7c5a20` | `d3b477` |
| info | `7fb2d4` | `92b2c8` | `8fc8ff` | `2a6a94` | `25677a` | `9abac5` |
| good | `8fc48a` | `a5bd9a` | `9be08f` | `3f7a3a` | `3b7a4a` | `a8bb87` |
| vouched | `b79ce6` | `b8aacb` | `cfaaff` | `6a3faa` | `6d5a8c` | `b7a6c9` |

Light palettes: vellum and paper. The implementer re-reads each value from `/home/tarek/workspace/strop/crates/strop-core/src/theme/{hone,slate,carbide,vellum,paper}.rs` when typing them in; the contrast test below catches a mistyped ink but not a mistyped background within the ladder's range.

### 8.2 Writfold's roles

`lines::Tone` becomes the role set; `style.rs` maps a role to a ratatui `Style` for the current look. Tones still only colour what the verb flagged (T1's rule): the role is chosen from the row's `status` and `flags`, never from a count the TUI made.

| Role (`Tone`) | Chosen when | Truecolor | ANSI (T1's colours) |
|---|---|---|---|
| `Bad` | a flag starting `failed`, `stale`, `invalid`, `changed`, `gone`, `expired`, `moved`, `regression`, `disagree` (T1's list), or `unsupported`, `out-of-order`, `two-plans`, `repeated`, `deny`; status `would-deny` | `error`, bold | red, bold |
| `Warn` | a flag starting `open`, `unknown`, `unverifiable`, `draft`, `past-ttl` (T1's list), or `template-moved`, `undefined`, `proposal`, `ask`, `nudge`, `skipped`; status `would-ask`, `would-nudge`, `proposed`, `draft` | `warning` | yellow |
| `Good` | status `satisfied`, `supported`, `verified`, `would-pass`, `clean` and no `Bad`/`Warn` flag | `good` | green |
| `Vouched` | status `attested`, `waived`, `not-applicable` and no `Bad`/`Warn` flag | `vouched` | magenta |
| `Current` | the current step's head line; the `!current` run's ID | `accent`, bold | cyan, bold |
| `Done` | a done step's head with no flag | `secondary` | default |
| `Skipped` | a skipped step's head (it carries `!skipped`, shown in `muted` rather than `Warn`: a skip is recorded with a reason, not owed) | `muted` | dark grey |
| `Later` | a later step; `[later]` gate lines | `muted` | dark grey |
| `Dim` | closed runs, `more` lines without a tone of their own, notes, the footer | `muted` | dark grey |
| `Info` | `next:`, `confirm:`, `accept:`, `read:` lines | `info` | blue |
| `Head` | pane titles, the header's run ID, the facts line when `Plain` | `text`, bold | bold |
| `Plain` | everything else | `text` | default |
| `Key` | key names in the tabs line, the help and the footer | `accent`, bold, on `raised` | bold |

A `more` line under a `Bad` row is `Bad` unless it has a tone of its own (T1's rule, kept). `muted` is used for dimmed text because it clears 4.5:1; `faint` does not and is only for borders.

Surfaces (truecolor only): the frame and the panes are `base`; the header, tabs, summary and footer lines are `surface`; the selected view's tab and the help popup are `raised`; an unfocused border is `faint` on `base`, the focused pane's border and title are `accent`; the selected run's entry is `text` on `selection` for all its lines (its state colours are dropped there, the words stay), with T1's `> ` marker. State is never carried by colour alone: every state is also the word in brackets.

### 8.3 Contrast test (ported)

`crates/writfold-tui/src/theme_tests.rs` (unit tests, `#[cfg(test)] mod`), with the source's WCAG relative-luminance and ratio functions and the same minimums, for all six palettes:

- on `base` and on `surface`: `text` ≥ 10, `secondary` ≥ 7, and `muted`, `accent`, `error`, `warning`, `info`, `good`, `vouched` ≥ 4.5;
- `on_accent` on `accent` ≥ 4.5; `accent` on `raised` ≥ 4.5 (the `Key` role);
- the ladder: `base`→`surface` and `surface`→`raised` each within 1.07–1.30; `raised` over `base` ≥ 1.18;
- `selection` over `base` ≥ 1.45 on light palettes, ≥ 1.55 on dark ones, 1.49 for slate (the source's documented floor);
- Writfold's addition: `text` on `selection` ≥ 7 (the selected run's entry). If a palette fails this, do not change the palette: draw the selected entry on `raised` for every palette and note it in §34;
- the registry: the default is hone; every name round-trips through `from_name` (also upper-cased); `nord` and `dark` are unknown; `choices()` is the six names in order.

All ratios compare with a `1e-9` tolerance, as the source does.

### 8.4 Selection and unknown names

Precedence: `--theme NAME`, then `$WRITFOLD_THEME`, then `theme = "NAME"` in `~/.config/writfold/config.toml` (3.7), then hone. The environment variable follows the existing `WRITFOLD_*` settings (`WRITFOLD_KB`, `WRITFOLD_WEFT`, `WRITFOLD_KB_REPOS`); the configuration file already exists for `weft`, so no new file is introduced.

An unknown name, from any of the three, is never fatal: the default theme (hone) is used and one warning names the source and the six names: `watch: unknown theme "nord" (--theme); using hone; themes: hone, slate, carbide, vellum, paper, paper-dark`. In an interactive session it is the footer's notice until the next key; for `--once` or a pipe it goes to stderr and stdout is unchanged. An unknown name in a lower-precedence source that a valid higher one overrides is not reported.

`T` changes the running view only; nothing is written, and the footer shows `theme=<name>`. The help lists the six names with their descriptions.

### 8.5 Colour depth

The source project always emits 24-bit colour and defers degraded terminals (its plan 0088 §4.4: "detect truecolor first; quantise only when the probe says so"). T2 detects and does not quantise:

| Depth | When (`--color auto`) | Drawing |
|---|---|---|
| truecolor | `COLORTERM` is `truecolor` or `24bit` | the palette, with surfaces |
| ansi | otherwise, when colour is allowed | T1's named colours (third column of 8.2), no background painted, so the terminal's own palette and background apply; the footer shows `theme=ansi`; `T` shows the notice `themes need 24-bit colour: COLORTERM=truecolor or --color truecolor` |
| none | `NO_COLOR` is set (any value), or `TERM` is `dumb` | no colour; bold for `Bad`, `Head`, `Current`, `Key`; the selected entry reversed |

`--color truecolor|ansi|none` forces a depth (for a terminal that supports 24-bit colour without saying so, such as inside a multiplexer). Quantising the palettes to 256 colours is not built: the tonal ladder (steps of 1.07–1.30:1) does not survive the 6×6×6 cube, and a half-applied theme would break the contrast minimums.

`--once`, and any run whose stdout is not a terminal, prints plain text with no escape sequence at all, whatever `--theme`, `--color` and the environment say (T1's "a pipe gets one plain frame"; a test asserts no `\x1b`).

## 9. `--once`

One frame as plain text, exit 0. Sections in this order, each a `== title ==` line; rows as the CLI prints them, with `@path:line` and `!flags` (T1's `lines::row`), so every line keeps its source position:

```text
== runs (4 listed, 3 not closed) ==              T1's entries plus the plan: line; filters none
== progress <run> ==                             facts line, strip, every step expanded with gate and item lines, notes
== gates <run> ==                                the counts line, then gate status' rows
== inbox <run> ==                                draft and proposal rows from resume; "(none)" when empty
== guards (current run <id|none>) ==             guard status' rows; "(no guard rules)" for reason=no-guards
== timeline <run> ==                             timeline's rows
== kb=<pin> index=<state> subject=<name>@<head>+dirty:<n>  read-only; no daemon, nothing here claims a run is live ==
```

- `--view NAME` prints the runs, that one section and the footer. `--view resume` is T1's frame (runs, `== resume <run> ==` with the `gates:` line, footer) apart from the footer's `subject=` and the runs' `plan:` lines. `--view stale` and `--view brief` print those panes.
- Step rows are printed as the CLI prints them (with the `step` kind word and the source position), prefixed by the mark; the interactive pane drops the kind word and the position to save width.
- No width is involved: lines are not wrapped or cut. No clock: no `asked`. No colour.
- A verb's refusal is its `refused (exit N): …` line inside its section.

The Progress section over the fixture of section 10 (IDs shortened here; the golden holds the real ones):

```text
== progress run-20261112T090000Z-a1b2c3 ==
plan 2 done, 1 skipped of 6 steps; current step 4/6; owes 2 of 2 gates stale=1
     [x] [!] [-] [>] [ ] [ ]
 x step change#1/research [done] notes [satisfied] @runs/…/plan/done-change.1~research.json:1
     gate notes [satisfied] mechanical @runs/…/gates/notes-…json:1
     done: recorded 2026-11-12T10:02:11Z
 x step change#1/implement [done] unit [satisfied], lint [stale] @runs/…/plan/done-change.1~implement.json:1 !stale
     why: no longer holds: lint [stale] (done stays done; list the gate again on a later step if it must hold there)
     gate unit [satisfied] mechanical @runs/…/gates/unit-…json:1
     gate lint [stale] mechanical @runs/…/gates/lint-…json:1 !stale
     done: recorded 2026-11-12T12:40:03Z
 - step change#1/benchmarks [skipped] no hot path touched (by owner, 2026-11-12T12:41:00Z) @runs/…/plan/skip-change.1~benchmarks.json:1 !skipped
 > step change#1/review [current] owes: review-list [open], signoff [open] @runs/…/plan/splice-change.1.json:1
     gate review-list [open] checklist 2/4 required covered !open
       item errors [supported] 1 reference by agent @runs/…/covers/review-list.errors-…json:1
       item tests [stale] 1 reference by agent; … @runs/…/covers/review-list.tests-…json:1 !stale
       item docs [open] The guide names every new flag
       item perf [not-applicable] no hot path touched by agent @runs/…/covers/review-list.perf-…json:1
       item style [open] (optional) Names follow the module
     gate signoff [open] attested !open
     read: instructions for signoff @.writfold/gates/signoff.md:1
     next: writfold kb gate cover review-list tests supported REF… --by WHO
   step change#1/deliver [later] gates: unit @runs/…/plan/splice-change.1.json:1
     gate unit [later]
   step change#1/reflect [later] no gates @runs/…/plan/splice-change.1.json:1
# note: plan change @… from kb; steps are derived from runs/run-20261112T090000Z-a1b2c3/plan/ on every read, never stored
```

The exact wording of each row is whatever the verbs print; the golden is produced by blessing (`WRITFOLD_KB_BLESS=1`, as T1's test does) and reviewed by hand once.

### 9.1 `watch --json`: not added

A JSON form of the frame would be a second envelope to version and keep compatible, holding only what the verbs' own `--json` already gives in full. Instead the help (`?` and `--help`) lists, per pane, the exact command whose JSON it shows, so a script asks those. `--once` stays the human form.

## 10. Module layout, budgets, tests

### 10.1 `crates/writfold-tui`

`crates/writfold-xtask/src/source.rs` (lines 143–151) gives this crate the ordinary budget: a file over 250 lines is a warning and over 400 an error; a function over 50 lines is a warning and over 80 an error; test files are inspected too. T2 targets no warning: every file at most 250 lines and every function at most 50, which also brings `lines.rs` (293 today) back under. No `unwrap`, no `expect`, `#![forbid(unsafe_code)]` kept. No new dependency: `ratatui` 0.29 (with its `crossterm` 0.28 re-export), `serde_json` and `writfold-kb` only; `crates/writfold-xtask/src/graph.rs` needs no change.

| File | Holds |
|---|---|
| `lib.rs` | `Options`, `parse`, `USAGE`, `main` (now returning the text for stdout and an optional warning for stderr), `once` |
| `source.rs` | `Source`, `Reply`, `field`, `flags`, and one method per verb: `runs(open, project)`, `plan`, `gates`, `timeline`, `resume`, `guards`, `stale`, `brief` |
| `poll.rs` | the three stamps (`pin` moved here from `source.rs`; `local`, `subject` through `writfold_kb::watch`) and `Stamps` with equality |
| `app.rs` | `App`: selection, view, focus, scroll, filters, expand-all, look, the answer caches, `reload`, `poll`, `load` |
| `keys.rs` | `Key` (adds `BackTab`, `PageUp`, `PageDown`) and `App::key` |
| `layout.rs` | `Class`, `class(width, height)`, `areas` (pure) |
| `lines.rs` | `Line`, `Tone`, `tone(row)`, `row`, `reply`, `run_item`, `state_of(text)` (the `<id> [<state>]` reader for `gate`/`item` lines) |
| `progress.rs` | the Progress pane's lines: facts line, strip, step blocks, collapsing |
| `side.rs` | the compact side panes and the summary line; `gate_summary` (moved from `lines.rs`), the inbox, guards and timeline lines |
| `frame.rs` | `once` (the plain frame), `header`, `footer`, `HELP` |
| `theme.rs` | `Rgb`, `Palette`, the six palettes, `ThemeId` (data only; about 150 lines) |
| `theme_tests.rs` | the contrast and registry tests |
| `style.rs` | `Depth`, `Look { theme, depth }`, `depth(color_flag, env)` (pure, the environment passed in), `style(look, tone)`, the surfaces |
| `view.rs` | `draw`: picks the class and renders the panes with ratatui |
| `term.rs` | the event loop (key mapping gains the new keys; the subject tick) |

`crates/writfold/src/main.rs`'s `watch` branch prints the warning to stderr and reads the environment (`COLORTERM`, `NO_COLOR`, `TERM`, `WRITFOLD_THEME`) once, passing it in, so the crate's decisions stay testable without touching the process environment.

### 10.2 Fixture world

`crates/writfold-tui/tests/support/` gains `plan.rs` (keep `mod.rs` as is for T1's world; share its helpers). It builds, with fixed `now` values so every recorded time is stable, as `crates/writfold-kb/tests/gate_checklist.rs` and `tests/plan.rs` build theirs:

- gates: `notes`, `unit`, `lint`, `bench` (mechanical, `argv` a trivial command, scopes chosen so one file edit makes only `lint` stale), `review-list` (checklist; items `errors`, `tests`, `docs`, `perf` required, `style` optional), `signoff` (attested, with instructions);
- template `change`: `research: notes`, `implement: unit lint`, `benchmarks: bench`, `review: review-list signoff`, `deliver: unit`, `reflect:`;
- run A (`!current`, open): `plan set change`; run `notes`, advance; run `unit` and `lint`, advance; `plan skip --reason "no hot path touched" --by owner`; cover `errors` supported with `gate:unit`, `tests` supported with `gate:lint`, `perf` not-applicable with a reason; then edit a file in `lint`'s scope only. Result: research `[done]`, implement `[done] !stale`, benchmarks `[skipped]`, review `[current]` owing both gates with items supported / stale / open / not-applicable / open-optional, deliver and reflect `[later]`;
- three checkpoint entries on run A, the second naming a decision ID of the fixture KB;
- one draft and two proposals in the local state directory (one proposal for run A, one for no run);
- two guard rules in `.writfold/guards/`: `commit-needs-gates` (`pre-tool-use`, `commands = ["git commit"]`, `require = ["gates.met"]`, `action = "deny"`) and `stop-reflect` (`stop`, `require = ["step.met"]`);
- run B: open, no plan; run C: closed, with a completed two-step plan.

### 10.3 Tests (`crates/writfold-tui/tests/`)

Keep `watch.rs` (T1) passing with the smallest edits (its resume view is now `View::Resume` via `e`; its `--once` golden is re-blessed under `--view resume`). Add `progress.rs`, `layout.rs`, `look.rs`, each at most 250 lines.

1. **`--once` goldens**: `tests/golden/progress-once.txt` (the full frame over the fixture) and `progress-once-view-gates.txt`. Assertions beside the golden: the six sections in order; no `\x1b`; none of `%`, `ETA`, `running`, `in progress`, `active`, ` ago` anywhere; the strip is `[x] [!] [-] [>] [ ] [ ]`.
2. **Layout** (`TestBackend`): at 160×40 the runs, progress, gates, inbox, guards and timeline blocks exist at the columns and rows `layout::areas` gives, the header is row 0 and the footer the last row; at 150×34 there is no guards block; at 100×30 two blocks and the summary line; at 70×20 one block and `run 1/3` in the header; at 50×10 the "needs 60x12" line. Two buffer-text goldens, `frame-160x40.txt` and `frame-100x30.txt` (symbols only, trailing spaces trimmed), which replace the mockups of 5.1 and 5.2 in §34 once built.
3. **Roles** (`TestBackend`, look forced to truecolor hone): the `lint [stale]` gate line's cells are `Rgb(0xe8,0x70,0x7c)` and bold; `[satisfied]` is `good`; an `[attested]` gate (attest `signoff` in a second world) is `vouched`, not `good`; the current step's head is `accent`; the skipped step is `muted`; the selected run's cells have the `selection` background; with depth `ansi` the failed cells are `Color::Red` and no cell has a background; with depth `none` no cell has a colour.
4. **Themes**: `theme_tests.rs` (8.3); selection precedence (flag over environment over configuration over default, each with the others set); each unknown-name case keeps hone and yields the warning with the six names, and `--once` output is byte-identical with and without the bad name; `T` walks the six and wraps; `depth()` over the table of 8.5.
5. **Keys**: every view key and digit reaches its view; Tab and Shift-Tab cycle eight; `[` `]` move the run with either focus; `x` expands and collapses; `o` and `P` change the `kb runs` request and the title, and the list equals `kb runs --open` / `--project`; `r` re-asks; `c` and every unbound letter change nothing; `q` quits.
6. **Projection equals the verbs**: for the fixture, every `step` row's ID, status and flags in the Progress pane equal `kb plan status --gates --json`'s; every `gate` and `item` line's text appears verbatim; the facts line's numbers are that trailer's `counts`; the runs entries' `plan:` text equals `kb runs --plan --json`'s `more` line and its `done=`/`skipped=` equal `plan status`'s counts for that run; the gates pane's states equal `kb gate status`'s (T1's test, kept); the guards pane's statuses equal `kb guard status`'s; the timeline rows equal `kb timeline <run>`'s. A property-style loop over the three fixture runs and all eight views asserts each displayed row text is a substring of that view's verb text output.
7. **No checker is started**: the fixture declares a checked citation kind (`.writfold/sources/<kind>.toml`, schema `writfold-source/1`, with `freshness = "checked"`, `checker = "probe"` and `fields`, as `crates/writfold/tests/kb_checker.rs` declares one) cited by a record of the run's project, and puts a `writfold-check-probe` script on `PATH` that appends a line to a counter file and exits. After building the app, visiting every view of every run, forcing three reloads and printing `--once` for every `--view`, the counter file does not exist. A control in the same test asks plain `kb stale` once and sees the counter at one, so the probe is known to work. (If the test process cannot change `PATH` safely in-process, place this test in `crates/writfold/tests/` beside `kb_checker.rs` and drive the binary with `--once`, as those tests do.)
8. **Read-only**: a digest of the KB work tree (`git status --porcelain` plus the bytes of dirty files), of the local state directory and of the index's `cite_checks` rows is equal before and after test 5's key sequence and test 7's tour.
9. **Refresh**: `poll` is false with nothing changed; true after a KB file change (T1), after adding a proposal file to the local state directory, and after editing a tracked file of the subject on the subject tick; the selection is kept; the stale gate turns up without `r` after the subject edit.
10. **A pipe** gets one plain frame; `--repo` pointing at a second repository makes `gate status`'s subject that repository; a bad `--repo` and a bad `--view` are usage errors.

The kb-side tests are listed with each verb in section 3. "Removing the check fails a test" is checked by hand for: `--cached` skipping the checker call (test 7 fails), the collapsing rule keeping flagged steps expanded, the unknown-theme fallback, and the `NO_COLOR` branch.

## 11. Documentation to update with the change

- `docs/writfold/kb-v1-design.md`: a new section after §33, "## 34. T2 as built", created by the implementer with these subsections filled from what was actually built: 34.1 Verb and panes; 34.2 Verb additions (`plan status --gates`, `runs --plan`, `stale --cached`, `timeline <run>`, `guard status`, the stamps, `user_config`); 34.3 Layout classes, with the two buffer goldens; 34.4 Keys and refresh; 34.5 `--once`; 34.6 Themes and colour depth (crediting the source of the palettes); 34.7 Build and dependencies (none new); 34.8 Tests; 34.9 Deviations from this specification; 34.10 Not done. §18 is not rewritten; add one line at the top of §18.6 pointing to §34 for the items it closes (`--repo`, the subject poll, the plan position in the runs list, the run filters) and the one it declines (`c`). §30 and §33 each get a one-line pointer (`guard status`; `stale --cached`). Fix the `### 32.x` subsection numbers under §33 only if the owner asks; it is outside this change.
- `docs/writfold/mission.md`, Roadmap: add under the v0.5.0-dev.2 items, struck through when built: `T2: run progress in writfold watch (plan steps, owed gates, checklist items, guards, inbox; themes) ([as built](kb-v1-design.md#34-t2-as-built))`.
- `docs/guide/`: a new page `watch.md` ("Watching a run": what each pane shows and the verb behind it, the keys, the layout classes, `--once` and `--view`, themes and colour, what it never shows and why), linked from `docs/guide/README.md`; `run-graph.md` line 69 updated (`watch` shows the steps, not one line) and `plan status --gates`, `runs --plan` added to its command list; `json.md` gains a short "Labels the views rely on" paragraph (the `gate`/`item` grammar of 3.1, the `plan` label, the new counts) with no change to its key tables; `checkers.md` line 33 adds `stale --cached` to the readers that start no checker; `hooks.md` documents `guard status`.
- `README.md`: the `watch` paragraph (line 255) and the command table row (line 385); the status line follows the README's convention for built versus planned.
- `plugins/writfold` skill text only if it names `watch`'s keys (check; do not widen).
- Public website copy: nothing until the capability is exercised.

## 12. Order of work

Each slice builds and tests on its own; commit per slice.

1. `stale --cached` and switching T1's stale pane to it, with the no-checker test. This fixes a present fault and is worth landing even if the rest slips.
2. `plan status --gates`, `runs --plan`, `timeline <run>`; their tests and goldens.
3. `writfold_kb::watch` stamps; `--repo`; the refresh changes.
4. The TUI: split of `lines.rs`, layout classes, Progress pane, side panes, keys, filters, `--once` sections and `--view`.
5. Themes, depth, `user_config`.
6. `guard status` and the guards pane.
7. Documentation and §34 (the integrator runs the gates from `main` after merging; see 0a.7).

## 13. Acceptance checklist

- [ ] `writfold watch` on a run with a plan shows every step with its state, the current step's gates with their standings, and each checklist item's standing, all equal to `kb plan status --gates`.
- [ ] The facts line holds only counts printed by the verb; no frame, in any view or size, contains a percentage, an ETA, a relative time or a liveness word; the test of 10.3 (1) passes.
- [ ] The runs list shows each planned run's position from `kb runs --plan`; owed-gate counts appear only for the selected run.
- [ ] `o` and `P` filter through `kb runs`; `[` `]`, the eight views, `x`, `r`, `T` work; `c` does nothing; the read-only test passes.
- [ ] A subject edit that makes a gate stale shows within one subject tick without `r`; a new proposal shows within one tick; `--repo` selects the subject.
- [ ] No checker is ever started by `watch` (test 7), and the control shows the probe would have counted one.
- [ ] 160×40, 150×34, 100×30, 70×20 and 50×10 render as section 5 says; the two buffer goldens are committed.
- [ ] `--once` prints the sectioned plain frame with no escape sequence; `--view` selects a section; the goldens are committed and reviewed by hand.
- [ ] Six themes; the contrast test passes for all six with the source's minimums; `--theme`, `WRITFOLD_THEME` and the configuration key select in that order; an unknown name keeps hone and lists the six; `NO_COLOR`, a non-truecolor terminal and `--color` behave as 8.5; `theme.rs` credits the source in its doc comment and no strop crate is a dependency.
- [ ] Every existing golden of `writfold-kb` is byte-identical; `crates/writfold-kb/tests/json.rs` is unchanged and passes; `json.md` documents the new labels and counts.
- [ ] No new crate in `Cargo.lock`; the xtask inspection reports no error and no new warning for `writfold-tui`; no `unwrap`/`expect`.
- [ ] The static musl binary builds offline and its `watch` was driven through a pseudo-terminal at 160×40 and 100×30 (as §18.4 did), in truecolor and with `NO_COLOR=1`.
- [ ] §34 is written from what was built, including deviations and Not done; mission roadmap line, guide page, README updated.
- [ ] `scripts/gate quality` and `scripts/gate unit` pass from main; receipt paths are in the commit message.

## 14. Not in T2

- Any write key, including promoting a draft and accepting a proposal.
- A percentage, ETA, elapsed or relative time, spinner or liveness indicator; a daemon or file watcher (the poll stays).
- Owed-gate counts for every run in the list (they need a subject per run).
- The session-start inbox's "unevaluated run" notices: they are computed in the hook (`crates/writfold-kb/src/hook/`), no verb prints them, and adding one is a separate change.
- The delivery guard and delivery rows beyond what `resume` already prints.
- Cross-KB views; a text search over runs; mouse support; focusable side panes.
- 256-colour quantisation of the palettes; user-defined themes; writing the chosen theme back to the configuration.
- `watch --json`.

## 15. Open questions for the owner (resolved: see 0a)

1. **`gate`/`item` lines carry their state inside `text`.** The alternative is a `status` key on `more` entries: cleaner for readers, still additive under `writfold_kb: 1`, but it changes the pinned `MORE` key set for every verb. This specification avoids it; say if the key is preferred.
2. **`guard status` is a new verb**, the only one here. It is specified as the last slice and can be dropped without affecting the rest.
3. **`g` moves from "resume" to "gates"** (resume becomes `e`). Keep T1's binding instead?
4. **Subject poll at 10 s.** Faster costs one `git status` in the code repository per tick.
5. **Unknown `--theme` is a warning, not a refusal**, although T1 refuses unknown arguments; this follows the source project's behaviour for unknown names.
6. **`timeline <run>` changes by default** (no flag) to list the run's own entries and mentions; a flag would keep today's output for run targets.
