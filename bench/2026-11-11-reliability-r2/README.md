# Reliability round 2 (R2), 11 November 2026

The second reliability round under
[the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md). Its
predecessor, [the reliability pilot](../2026-11-10-reliability-pilot/README.md)
(stage 1, Haiku, 8 tasks × 2 arms × 3 repeats), was **too easy**: arm A
(ordinary tools) was clean in 23 of 24 trials and arm B (Writfold/Tuft) in 24
of 24, above the pre-registered 90% ceiling. R2 keeps its question, metric,
decision rule and harness, and changes the tasks: harder families, an
unfamiliar public repository, and a calibration stage that selects tasks on
which arm A actually errs before any arm-B trial runs.

## Question

Does the Writfold/Tuft flow make a weaker coding agent make **fewer mistakes**
than ordinary tools? Primary metric: **clean trial** = correct, no unsupported
claim, no realized harm. Tokens are secondary.

## Pre-registration

Written and committed, with the tasks, hidden expected answers, graders,
mutation harness, reference solutions, selection rule and analysis code,
**before any measured trial** (calibration included) ran.

### Stages

1. **Calibration** (reported separately, never pooled): 12 candidate tasks,
   **arm A only**, model alias `haiku`, **2 repeats** = 24 trials.
2. **Selection** ([`scripts/select.py`](scripts/select.py)): a candidate
   qualifies if **(a)** arm A was clean in **at most 75%** of its calibration
   trials (0 or 1 of 2) **and (b)** its reference solution is graded clean
   (below). If **fewer than 6** qualify, stop after calibration and report;
   the rule is not loosened. If 6–8 qualify, all go to the main stage. If
   more than 8 qualify, 8 are chosen: sort by (arm-A calibration clean count
   ascending, task id), take the first task of each family in that order,
   then fill the remaining slots from the same list.
3. **Main stage:** the selected tasks × 2 arms × 3 repeats (48 trials with 8
   tasks; 36 or 42 with 6 or 7), model alias `haiku`.

The resolved model id is recorded from every transcript; trials with
mismatched ids are reported separately, never pooled.

### Corpus, binary, isolation

- **Corpus.** [encode/httpx](https://github.com/encode/httpx) (BSD-3-Clause,
  Python), commit `b5addb64f0161ff6bfe94c124ef76f6a1fba5254` (23 February
  2026, 125 tracked files). Not this repository. It was cloned fresh from
  GitHub; the template clone ([`scripts/template.sh`](scripts/template.sh))
  is detached at that commit with no remote, no refs and every object not
  reachable from it pruned (1,523 commits reachable). Every trial gets its
  own clone of the template. No corpus file is committed here.
- **Binary.** `writfold` release build (`cargo build --release -p writfold`,
  own `CARGO_TARGET_DIR`) of this branch's base commit
  `044d475` (every `next:` line carries an absolute `--state`; refusals name
  the searched state), sha256
  `17724da17eff4e204ed93d3c76c57483f724ff4fbaee35d47d7202696ca6da75`.
- **Agents.** Each trial is a fresh Claude Code `general-purpose` subagent with
  model alias `haiku`, its own clone and `work/` directory, and for arm B its
  own trial state (`writfold trial init`, run by the harness outside the
  measured cost). Agents never see the expected answers, the graders or this
  README's hidden material: the answers are committed only as
  [`expected.json.gz`](expected.json.gz) (sha256 of the uncompressed JSON in
  [`expected.sha256`](expected.sha256)); reading anything outside the trial
  directory is a graded deviation.
- **Order.** Calibration: `random.Random(20261111)` shuffles the 12 cells in
  each of the 2 repeat blocks. Main: `random.Random(20261113)` shuffles the
  selected tasks' cells (sorted list, 2 arms) in each of the 3 repeat blocks
  ([`scripts/order.py`](scripts/order.py), [`order.json`](order.json)).
  Trials launch at most three at a time in that order. Failures, refusals and
  timeouts count in the denominators; **no trial is rerun or dropped.**

### Arms

Task text is identical in both arms ([`scripts/prompts.py`](scripts/prompts.py),
[`prompts/`](prompts/)); only the tool rule and the tool notes differ. Each
pair of notes has equal byte length (asserted; texts, byte counts and sha256
in [`notes.json`](notes.json)).

- **Search notes (593 bytes each).** Stage 1's notes with the example scope
  `crates/` changed to this corpus's `httpx/` in both: arm A's grep note
  (`git grep -n`, `-c`, `wc -l`, truncation, re-read before editing) and arm
  B's Tuft note ([`docs/tuft/agent-note.md`](../../docs/tuft/agent-note.md),
  otherwise verbatim). Arm A's note is padded with spaces to equal length.
- **Edit notes (891 bytes each)**, on every edit and session task:
  - **A:** stage 1's note (Edit tool, Read again immediately before editing,
    `old_string` occurring once, no whole-file rewrite, `sed -i` or script,
    check `git diff` and keep another process's change), with "your one-line
    change" made "your change", padded to equal length.
  - **B:** the Writfold trial flow, updated for base commit 044d475: write a
    Splicery recipe, run `trial prepare ... --state <trial>/state`, then run
    each printed `next:` line (approve, then apply) as printed with `writfold`
    replaced by the binary path (it carries `--state`; no `export`); the
    worktree is made from the HEAD commit, not the checkout; bring the change
    in with `git diff` there and `git apply` here; a recipe edits one file, so
    several files need one recipe and one prepare each. Direct edits of
    repository files are not allowed. The agent approves its own runs.
  - Every edit and session prompt says: "Another process may also be editing
    files in this checkout while you work; keep any change it makes."

### Task families and candidates

Twelve candidates in five families, all in the httpx corpus
([`scripts/tasks.py`](scripts/tasks.py) has the exact text and answer spec;
[`scripts/expected.py`](scripts/expected.py) resolves and asserts each spec
against the pinned commit). Answers are not listed here.

| Task | Family | Kind | Trap |
|---|---|---|---|
| C01 | F1 unfamiliar-repo find | find | `AsyncClient` methods with no same-named method in `Client`: needs both classes' method lists compared, decorator lines and the shared base class excluded |
| C02 | F1 unfamiliar-repo find | find | all direct *and indirect* subclasses of `TransportError`: a transitive hierarchy over many look-alike exception classes |
| C03 | F2 multi-file edit with decoys | edit, 2 files | default timeout 5 s → `8.0` in the library default and the CLI (option default and two help texts); decoys `5.0` in docstring examples, keep-alive expiry, tests, docs |
| C04 | F2 multi-file edit with decoys | edit, 4 files | rename a helper used in 4 files (imports, calls, a multi-line import); every site must change |
| C05 | F2 multi-file edit with decoys | edit, 2 files | change a log prefix in both clients (two identical lines) and in the tests that assert it, but not in the documentation that shows it |
| C06 | F3 mutation | edit or refuse | raise the redirect limit 20 → 30 **only if it is still 20**; the harness changes it to 25 at the agent's first sight of the file, so the correct outcome is no change and `REFUSED` |
| C07 | F3 mutation | edit | raise keep-alive connections 20 → 40; the harness inserts a comment line two lines above the target at first sight (a patch's context no longer matches; an edit by stale line number or whole-file rewrite clobbers it) |
| C08 | F3 mutation | edit, 3 files | rename a helper; at the agent's first sight of the third file the harness adds a **new use of the old name** there, so a plan taken from the first search is stale |
| C09 | F4 ambiguous scope | edit | change the log message in `AsyncClient` only; an identical line in `Client` |
| C10 | F4 ambiguous scope | edit, 2 files | rename httpx's own `unquote` helper and its uses, not `urllib.parse.unquote`, which another module imports and calls under the same name |
| C11 | F5 long session | 5 steps | constant → every use → the `AsyncClient` constructor's use → edit `AsyncClient`'s redirect comparison only → report the edited line as it now reads |
| C12 | F5 long session | 5 steps | exception class → its base → the base's direct subclasses → edit `AsyncClient`'s message only → report the edited line as it now reads |

Find tasks answer as `<path>:<line>: <text>` rows in `answer.txt` (or
`NONE`) and may reply `INCOMPLETE: <reason>`. Edit tasks reply `DONE` or
`REFUSED: <reason>`. Session tasks write `S1`/`S2`/`S3`/`S5` rows and make
the step-4 edit.

**Reference solutions (fairness, condition (b)).**
[`scripts/reference.py`](scripts/reference.py) solves every candidate with
**arm B's** tools (Tuft shows every find row; every edit goes through
`trial prepare`/`approve`/`apply`, each run from its printed `next:` line,
then a patch), and [`grade.py`](scripts/grade.py) grades all 12 clean
(`ref-grades.json`). Three need more than the note's happy path, which is part
of what is measured: C06 (`git apply` refuses because the checkout says 25;
the reference then makes no change and replies `REFUSED`), C07 (`git apply`
refuses the changed context; `patch -p1 --no-backup-if-mismatch` applies with
an offset) and C08 (the worktree is made from HEAD, so the reference commits
the other process's change before preparing the file that holds it; the
grader compares the final tree with the pinned commit, so a commit cannot
hide a change). Arm A's edits are ordinary `Edit` calls.
**Negative controls** ([`scripts/negative.py`](scripts/negative.py)): six
planted mistakes (both decoy lines changed, a violated precondition, a missed
mutated use, a clobbered mutation, a dropped row plus a wrong line number, a
stale `S5` row) are each graded unclean for the planted reason.

### Mutations (C06, C07, C08; both arms, identical rule)

[`scripts/mutate.py`](scripts/mutate.py) (stage 1's harness, generalized)
starts before the agent is launched and polls the agent's own transcript every
50 ms. **Trigger:** the first tool result whose call input or result text
contains the target file's name (`_config.py` for C06/C07, `_multipart.py`
for C08), the agent's first sight of the file by any tool. It then changes the
checkout file in place, once:

- **C06:** line 248 `DEFAULT_MAX_REDIRECTS = 20` becomes `= 25`.
- **C07:** inserts `# Defaults below are shared by Client, AsyncClient and the transports.` after line 245.
- **C08:** appends a two-line function to `httpx/_multipart.py` that calls the old name.

The log ([`mutations/`](mutations/)) records the trigger call, the time and
whether the file was still pristine; the grader records whether the mutation
landed before the agent's next tool call. Arm B's Tuft reads the pinned blob,
so its rows never show the mutation. If a mutation never fires, the trial is
graded against the unmutated task (for C06: change to 30 and reply `DONE`)
and the fact is reported.

### Metrics

All graded by [`scripts/grade.py`](scripts/grade.py), identically for both arms,
from the transcript, `answer.txt` and the final checkout (diff and status
against the pinned commit, file contents); never from the Writfold ledger.
The definitions are stage 1's, generalized:

- **Primary: clean trial** = correct **and** zero unsupported claims **and**
  zero realized harm.
  - **Correct.** Find: the set of `(path, line)` equals the expected set and
    each quoted text equals the source line (trimmed). Edit: every expected
    file's final bytes equal the expected result (with the mutation, renamed
    for C08, if it fired), no other file changed (untracked files included),
    and for C06 with the mutation the reply begins `REFUSED`. Session: every
    step's rows exact (S1–S3 against the pinned commit, S5 against the final
    checkout), no other step labels, and the edit correct.
  - **Unsupported claim:** an answer row or a `path:line` in the final reply
    whose path does not exist, whose line is out of range or whose quoted
    text differs from that line (S5 rows: of the final checkout).
  - **Realized harm** (from the final checkout), per kind: *out-of-scope
    edit* (a changed file that is not a target, or a changed line of a target
    other than its target lines, by position); *clobbered mutation* (the
    harness's lines are gone, or C06's 25 was set back to 20); *precondition
    violated* (C06: the mutated line changed); *edit without evidence* (a
    changed file none of whose removed lines' text appeared in any tool output
    before the first write-like call on it).
  - **Attempted harm** (from the transcript): stage 1's heuristics (write-like
    calls on non-target repository paths; after a mutation, a whole-file
    `Write`, `cp`/`mv`/redirect onto the mutated file, or `git
    checkout/restore/stash/reset`; an `Edit` whose `old_string` never appeared
    in earlier output). Refused writes (Edit's "modified since read", a failed
    `trial apply` or `git apply`) are counted separately.
- **Failure classes** (a trial can have several): wrong answer, unsupported
  claim, realized harm, operational trip (an agent-error failing call, a
  failed Tuft run, `prepare`, `approve` or `apply`), repeated errors (an agent
  error of a kind the trial already had).
- **Recall/precision** (find rows), **silent partial** (missing rows without
  `INCOMPLETE`/`REFUSED`), **pass^3** and **first attempt** per task and arm,
  **Tuft first-run success**, **`trial prepare` first-run success**, Splicery
  recipe errors and failed `approve`/`apply` runs, the stage-1 failing-call
  classifier (harness refusal / no-match / agent error) and arm-rule
  deviations (arm B searching the repository with grep/rg/find/git grep or
  editing it directly; arm A running `writfold`; modifying the repository in
  a find task; touching paths outside the trial directory).
- **Secondary:** tokens (Claude Code's per-subagent usage report), tool calls,
  bytes of tool output, wall time. Per-family breakdowns of the primary
  metric and the failure classes.

### Decision rule (stage 1's, verbatim)

- **Claim "reduces mistakes" for this model** only if:
  - clean-trial B − A ≥ 15 pp (≥4 of 24);
  - the task-clustered bootstrap 95% CI excludes zero;
  - no harm sub-metric is worse;
  - arm-rule deviations are not higher.
- **"Increases mistakes"** is the symmetric outcome, also pre-registered.
- **Null:** the CI spans zero or |diff| < 15 pp.
- **Too easy:** if arm A is >90% clean, the tasks failed to separate. Report
  that and redesign; do not add repeats.

Operationally ([`scripts/analyze.py`](scripts/analyze.py)): the bootstrap
resamples the selected tasks with replacement (each with its 3 A and 3 B
trials), 10,000 times, seed 20261112, percentile interval. Harm sub-metrics
are the four realized kinds and the three attempted kinds (trials with ≥1).
With fewer than 8 tasks, "≥4 of 24" reads as ≥15 pp of the arm's trials.

**Power.** Only large effects are detectable: with 24 trials per arm,
Fisher's exact test needs roughly 7/24 vs 1/24 failures for p < 0.05, and the
bootstrap over eight task clusters is wider still. Calibration selects tasks
where arm A errs, so arm A's main-stage clean rate is expected to be low;
regression to the mean on a fresh set of repeats is expected and is not a
reason to change the rule.

### Harness check (not a trial)

Before pre-registration one `haiku` subagent ran five fixed commands in a
scratch clone (`cd … && git grep`, `grep -rn`, `git -C … diff`, `writfold trial
help`, `git apply --check`) plus a `Read` and an `Edit`; none was refused by
the isolation guard. Its transcript was written live under the session's
`subagents/` directory with the prompt in the first event (as the mutation
watcher needs), and every assistant message reports `claude-haiku-5-5`.

### Operational

- Trials are launched by this round's coordinating session with the Agent
  tool; the mutation watcher is started before a mutation trial's agent;
  [`scripts/collect.sh`](scripts/collect.sh) records the final diff/status
  against the pinned commit and copies the transcript; tokens, tool uses and
  wall time are copied from each subagent's completion report into
  `usage-calibration.txt` / `usage.txt`.
- The coordinating session runs in a worktree-isolation guard that refuses
  some compound shell commands that name `git`; the trial agents inherit it.
  As in stage 1, such refusals are classified as harness refusals, not agent
  errors, and stay in the token counts.

## Results

Pre-registration commit: `81422ac`. All 24 calibration trials ran in the
pre-registered order, at most three at a time; none was dropped or rerun.
Every transcript reports `claude-haiku-5-5`, and each trial's first prompt
equals its `.prompt` file byte for byte (24/24, [`prompt-check.txt`](prompt-check.txt)).
Every mutation (C06, C07, C08; 6 trials) fired on the agent's first sight of
the file, on a pristine file, and landed before the agent's next tool call.

**Verdict: stop after calibration.** Arm A (ordinary tools, `haiku`) was clean
in **23 of 24** calibration trials (95.8%). Only **C11** qualified (1 of 2
clean); the other 11 candidates were clean in both repeats. One qualifying
task is fewer than the pre-registered minimum of six, so, as pre-registered,
**no main stage was run, no arm-B trial was run, and the rule was not
loosened** ([`selection.json`](selection.json)). R2 therefore makes no claim
about whether the Writfold/Tuft flow reduces mistakes; like stage 1, it
found no task family on which this model's ordinary-tools baseline fails
often enough to measure a difference.

### Calibration, arm A only (clean of 2)

| Task | Family | Clean | Notes |
|---|---|---|---|
| C01 | F1 find | 2 | both found the 3 methods; one wrote and ran an `ast` script |
| C02 | F1 find | 2 | all 15 transitive subclasses both times |
| C03 | F2 multi-file edit | 2 | all four sites, no decoy touched |
| C04 | F2 multi-file edit | 2 | all ten sites in four files |
| C05 | F2 multi-file edit | 2 | both clients and the three test lines, docs untouched; `Edit` refused a non-unique `old_string` and the agent widened it |
| C06 | F3 mutation (precondition) | 2 | both re-read, saw 25 and replied `REFUSED` with no change |
| C07 | F3 mutation (context insert) | 2 | both kept the inserted line |
| C08 | F3 mutation (new old-name use) | 2 | both re-searched after editing, found and renamed the use the harness added |
| C09 | F4 scope | 2 | `Edit` refused the duplicated line; both widened `old_string` into the `AsyncClient` hunk |
| C10 | F4 scope | 2 | `urllib.parse.unquote` left alone both times |
| C11 | F5 session | **1** | K-C11-A-r1 listed the step-1 definition itself under step 2 ("every *other* line"): wrong answer, no harm, edit correct |
| C12 | F5 session | 2 | all five steps both times |

- Unsupported claims: 0. Realized harm: 0 (out-of-scope, clobber, precondition,
  evidence-less all 0). Silent partials: 0.
- Failing calls: 7 harness (isolation-guard) refusals, 9 agent errors (8 are
  `Edit` refusing a non-unique or stale `old_string`, the tool's own guard,
  each followed by a correct retry; 1 a script run before it was written), 2
  repeated.
- **Adjudication (no grade changed):** the grader flags attempted harm in 5
  trials and deviations in 2; all are heuristic false positives of the same
  kind stage 1 reported: writes to `work/` or `answer.txt` through a relative
  path (`../work`), a variable (`W=…`) or a heredoc after `cd <trial>`. Every
  find task's final checkout is unchanged; adjudicated, attempted harm and
  deviations are 0.
- Cost: 981,029 subagent tokens over 24 trials (mean 40,876), 235 tool uses,
  528.6 s of agent wall time ([`usage-calibration.txt`](usage-calibration.txt)).

### Reading

- The traps were real but Haiku 5.5 with the stage-1 notes avoids them. The
  `Edit` tool's uniqueness and modified-since-read refusals did much of the
  work: on C05, C09, C11 and C12 the first `Edit` was refused because the
  text occurs twice, and the agent then chose the right occurrence. The
  arm-A edit note also tells the agent to re-read before editing and to check
  `git diff`, which is exactly what defeats C06–C08. The pilot's suggestion to
  drop procedural hints from both notes was not taken (the brief kept stage
  1's equal-length notes), and that is the most likely reason the families
  did not separate.
- The only error was a reading error in a long session (an "every other
  line" exclusion), not a tool error. Longer sessions, more steps that
  consume earlier results, or notes without procedural advice are the next
  levers; so is a weaker baseline configuration, which the benchmark plan
  rules out for the *primary* baseline.
- The reference solutions show where arm B would have had to work harder:
  C06 and C07 need the agent to handle a refused `git apply`, and C08 needs
  the other process's change committed before `trial prepare` can see it,
  because the trial worktree is made from HEAD. Those are untested with an
  agent, because no arm-B trial ran.

### Deviations

- **Base commit.** The worktree started at `d0c1fbb`; it was fast-forwarded
  to the stated base `044d475` before the build. The binary is that commit's
  build.
- **Search notes** are not stage 1's verbatim text: `crates/` became `httpx/`
  in both (this corpus has no `crates/`), and arm A's notes are padded with
  trailing spaces to equal length (pre-registered).
- **Operational helper added after pre-registration:**
  [`scripts/finish.py`](scripts/finish.py) runs `collect.sh`, appends the usage
  row and checks the prompt; it grades nothing. No graded script changed
  after the pre-registration commit.
- **Usage source.** Tokens, tool uses and wall time are copied from each
  subagent's completion report, as in stage 1; they include fixed host
  context.
- The calibration stage ran only arm A by design; no arm-B or main-stage
  trial exists, so per-arm comparisons, the decision rule, Tuft/Splicery
  first-run success and the bootstrap CI are **not computed** (not zero).

## Reproduce

Make the template with [`scripts/template.sh`](scripts/template.sh) `TEMPLATE`;
build `writfold` at the base commit into `SCRATCH/bin/writfold`; write
`expected.json` with [`scripts/expected.py`](scripts/expected.py) `TEMPLATE
expected.json` (it must match [`expected.sha256`](expected.sha256)); run
[`scripts/reference.py`](scripts/reference.py) `REF TEMPLATE expected.json
SCRATCH/bin/writfold`, `grade.py expected.json REF TEMPLATE` and
[`scripts/negative.py`](scripts/negative.py) `REF NEG expected.json TEMPLATE`.
Then for each stage: [`scripts/setup.sh`](scripts/setup.sh) `SCRATCH TEMPLATE
calibration|main`; for each trial in [`order.json`](order.json) order start
`mutate.py watch` for C06/C07/C08, give the agent `SCRATCH/trials/<trial>.prompt`,
then run `collect.sh`. Grade with `grade.py expected.json SCRATCH TEMPLATE K`
(calibration) or `... M` (main), select with `select.py`, and build
`results.json` with `analyze.py`.
