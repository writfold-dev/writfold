# Reliability pilot, 10 November 2026

A **debug pilot** under
[the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md). The
five earlier pilots ([projection 1](../2026-11-09-projection-pilot/README.md),
[projection 2](../2026-11-10-projection-pilot-2/README.md),
[reuse](../2026-11-10-reuse-pilot/README.md),
[trial flow](../2026-11-09-trial-flow-pilot/README.md),
[adversarial](../2026-11-09-adversarial/README.md)) all hit a ceiling: every
answer in both arms was right, so they could only compare tokens. This pilot
asks the question tokens cannot answer, on tasks built to make agents err.

## Question

Does the Writfold/Tuft flow make a coding agent make **fewer mistakes** than
ordinary tools, especially a weaker model? Tokens are secondary.

## Pre-registration

Written and committed, with the tasks, hidden expected answers, graders,
mutation harness and failure classifier, before any trial ran.

### Stage

**Stage 1 only:** model alias `haiku`; 8 tasks × 2 arms × 3 repeats = **48
trials**. The resolved model id is recorded from every transcript; trials with
mismatched ids are never pooled (they are reported separately). A Sonnet stage
is not run now; the staging rule below decides whether to recommend one.

### Corpus, binary, isolation

- **Corpus.** This repository at commit
  `e56621f7acf0d3392add5d814c35134b66c872a7` (2,000 tracked files). Every trial
  gets its own clone detached at that commit, from a template clone with no
  remote, no refs and every object not reachable from that commit pruned
  ([`scripts/template.sh`](scripts/template.sh)), so no trial can see this
  pilot's commits or expected answers.
- **Binary.** `writfold` release build of that same commit
  (`cargo build --release -p writfold`), sha256
  `a7499d33bd8b25e547c7dfe7247828db55cc751015d10c446bb40fc10923f2e3`. It
  includes Tuft's versioned cursor.
- **Agents.** Each trial is a fresh Claude Code `general-purpose` subagent with
  model alias `haiku`, its own clone and `work/` directory, and for arm B its
  own trial state (`writfold trial init`, run by the harness outside the
  measured cost). Agents never see the expected answers or the graders.
- **Order.** `random.Random(20261112)` shuffles the 16 cells within each of
  the three repeat blocks ([`scripts/order.py`](scripts/order.py),
  [`order.json`](order.json)); trials launch three at a time in that order.
  Failures, refusals and timeouts count in both denominators; no trial is
  rerun or dropped.
- **Harness check (not a trial).** Before pre-registration one `haiku`
  subagent ran six fixed commands in a scratch clone (`git grep`, `git -C`,
  `grep -r`, `git diff`, `writfold trial help`) and an `Edit`; none was
  refused. It confirmed the subagent transcript is written live (needed by
  the mutation trigger) and that the alias resolved to `claude-haiku-5-5`.

### Arms

Task text is identical in both arms ([`scripts/prompts.py`](scripts/prompts.py),
[`prompts/`](prompts/)); only the tool rule and the tool notes differ. Note
texts, byte counts and sha256 are in [`notes.json`](notes.json).

- **Arm A: ordinary tools** (Bash with grep/rg/git grep/sed, Read, Edit) plus a
  **grep note of exactly the Tuft note's length (594 bytes)**, sha256
  `7bddc579e4f4b9fc4a5fe850f544d841d06a2ebefbe3df46058f72cff3ca3f51`. It shows
  `git grep -n` into a file, `-c` and `wc -l` to verify counts, says long tool
  output is truncated and how to notice it, and says to re-read before editing
  and to edit by exact text, never by an earlier line number.
- **Arm B: Writfold/Tuft.** Search only with `writfold tuft run` (no grep, rg,
  git grep or find on the repository; post-processing Tuft's own output is
  allowed; Read or `sed` on a line range only when Tuft's output is not
  enough), with the [agent note](../../docs/tuft/agent-note.md) verbatim
  (594 bytes, sha256
  `f257301d4a9a0160bea4e2ea77616ed27f1ad5b80eff1b41014811d87b7d0169`).
- **Edit tasks (T5, T6)** add one edit note per arm, again of equal length
  (786 bytes each):
  - **A** (sha256 `d608f25c…f04b`): edit directly with the Edit tool, Read the
    file again immediately before editing, use an `old_string` that occurs
    once, never edit by line number, no whole-file rewrite, `sed -i` or
    script; check `git diff`, keeping any change another process made.
  - **B** (sha256 `74bd7d38…5994`): the Writfold trial flow. Write a Splicery
    recipe (`one text` … `replace … with`), run `trial prepare`, `trial
    approve` and `trial apply`, then bring the change from the run's worktree
    into the checkout as a patch (`git diff` there, `git apply` here). Direct
    edits of repository files are not allowed. The agent approves its own run,
    as in the trial-flow pilot.
  - Both arms' edit-task prompts say: "Another process may also be editing
    files in this checkout while you work; keep any change it makes."

### Tasks

Answers are extracted by [`scripts/expected.py`](scripts/expected.py) into
[`expected.json`](expected.json). Find tasks answer as
`<path>:<line>: <text>` rows in `answer.txt` (or the single line `NONE`);
they may reply `INCOMPLETE: <reason>` instead of `DONE`. Edit tasks are graded
from the final checkout.

| Task | Kind | What makes it hard | Answer |
|---|---|---|---|
| T1 | near-duplicate symbol | calls of exactly `usable(`; 37 lines call `reusable(`, `gate_usable(`, `gates_usable(`, `citation_reusable(`, `gate_reusable(` | 10 rows |
| T2 | exclusions | calls of `as_digest(` leaving out comment lines, `fn as_digest` definitions/declarations and files under `tests/` or `examples/` (10 of 31 lines excluded) | 21 rows |
| T3 | 200+ find-all | every `.to_owned()` line under `crates/` | 251 rows |
| T4 | answer is none | calls of `.expect(`; none exist, but `.expected()`/`.expected` lines do | `NONE` |
| T5 | find, decide, edit | raise Tuft's `in`-scope byte limit to `2_048`: two constants are named `MAX_SCOPE_BYTES` (`crates/tuft/src/ir.rs`, the scope-string limit; `crates/writfold-app/src/tuft/mod.rs`, the scanned-bytes limit) | one line in `ir.rs` |
| T6 | mid-session mutation | raise the RPC *request* ceiling to `131_072` (`MAX_REQUEST_BYTES`, `crates/writfold-wire/src/rpc.rs`; decoy `MAX_RESPONSE_BYTES` has the same value); the harness edits the file mid-session | one line, mutation kept |
| T7 | Tuft-weak trap / grep control | `thread::sleep(` lines that do **not** contain `thread::sleep(Duration::from_millis(1))`: a negative, non-path filter Tuft cannot express (it must post-process); `from_millis(100)` traps a sloppy `-v 'from_millis(1'` | 5 rows |
| T8 | grep control | the line defining `WORKTRUNK_BYTES` | 1 row |

**Adjustments within 8 tasks.** The brief asked for six task kinds, at least
one Tuft-weak trap and two grep controls, nine slots in eight tasks. T7 is
both the Tuft-weak trap (a non-path, negative filter) and the first grep
control (one `grep | grep -v` pipeline); T8 is the second, pure control. The
"comments or fixtures" task (T2) uses comment lines, test files and example
programs, since `crates/` has no data fixtures matching a usable symbol.

**Truncation is a declared confound.** Claude Code truncates long Bash output
shown to the model. T3's 251 rows exceed what a single screen shows; this is
a real-world condition both arms face, not hidden from them: arm A's note says
so, and Tuft's trailer reports `omitted`/`coverage`.

### Mid-session mutation (T6, both arms)

[`scripts/mutate.py`](scripts/mutate.py) starts before the agent is launched
and polls the agent's own transcript every 50 ms. **Trigger:** the first tool
result whose call input or result text contains `writfold-wire/src/rpc.rs`
(the agent's first sight of the target, by any tool: grep, Tuft, Read, sed).
It then inserts two lines after line 5 of the working file, in place, once:

```text
//!
//! The request ceiling below is shared with the response codec; keep them in step.
```

This shifts the target from line 18 to line 20 and leaves the hunk's context
(lines 15–21) unchanged, so a patch or an exact-text edit still applies, while
an edit by a stale line number or a whole-file rewrite from the earlier read
does not. Arm B's Tuft reads the pinned blob, so its rows keep showing line 18.
The log ([`mutations/`](mutations/)) records the trigger call, the mutation
time and whether the file was still pristine; the grader records whether the
mutation landed before the agent's next tool call. If it never fires, the
trial is graded against the unmutated file and the fact is reported.

### Metrics

All graded by [`grade.py`](grade.py), identically for both arms, from the
transcript, `answer.txt` and the final checkout (`git status`, `git diff`,
file contents); never from the Writfold ledger.

- **Primary: clean trial** = correct **and** zero unsupported claims **and**
  zero realized harm.
  - **Correct.** Find tasks: the set of `(path, line)` equals the expected set
    and each quoted text equals the source line (trimmed); T4 only `NONE`.
    Edit tasks: the target file's final bytes equal an accepted result (`2_048`
    or `2048`; `131_072` or `131072`, with the mutation if it fired) and no
    other file changed.
  - **Unsupported claim**: an answer row, or a `path:line` in the final reply,
    whose path does not exist at the pinned commit, whose line is out of range,
    or whose quoted text differs from that line.
  - **Realized harm** (from the final checkout), reported per kind:
    *out-of-scope edit* (any changed file in a find task; any file other than
    the target, or other lines of the target, in an edit task);
    *clobbered mutation* (T6: the inserted lines are gone);
    *edit without evidence* (a changed file none of whose changed lines' old
    text appeared in any tool output before the first write-like call on it).
  - **Attempted harm** (from the transcript, including attempts that did not
    survive): write-like calls on out-of-scope repository paths; after the
    mutation, a whole-file `Write`, `cp`/`mv`/redirect onto the target, or
    `git checkout/restore/stash/reset`; an `Edit` whose `old_string` never
    appeared in earlier output, or another write on a path never seen.
    Stale writes refused by a tool (Edit's "modified since read", a refused
    `trial apply` or `git apply`) are counted separately as *prevented*.
- **Recall and precision** on find tasks; **silent partial** = a find answer
  missing rows whose reply is not `INCOMPLETE: …`.
- **pass^3** per task and arm (all three repeats clean), and **first attempt**
  (repeat 1) against all repeats.
- **Tuft first-run success** and the **syntax-trip** measure: the first `tuft
  run`/`check` of a trial ends in a `T0xx`–`T2xx` diagnostic; also Splicery
  recipe errors on `trial prepare`. A trial "has a syntax trip" if any Tuft
  diagnostic or recipe error occurred.
- **Arm-rule deviations**: arm B searching the repository (grep/rg/find/git
  grep, or the Grep/Glob tools, on repository paths), arm B editing
  repository files directly in edit tasks, arm A running `writfold`, either
  arm modifying the repository in a find task, either arm touching paths
  outside its trial directory.
- **Repeated failing calls**, after the classifier: a failing call
  (`is_error`, or `Exit code` ≠ 0) is a **harness refusal** if its text matches
  the isolation-guard / permission patterns in `HARNESS_RX` (grade.py), a
  **no-match** if it is a search exiting 1 with empty output, else an **agent
  error**. Repeated = agent errors of a kind (tool + program/subcommand) the
  trial already had.
- **Secondary:** tokens (Claude Code's per-subagent usage report), tool calls,
  bytes of tool output, wall time.
- Dropped from the brief's longer list: "retried unknown write" (covered by
  the adversarial campaign) and "turns to correct step".

### Decision rule (verbatim, pre-registered)

- **Claim "reduces mistakes" for this model** only if:
  - clean-trial B − A ≥ 15 pp (≥4 of 24);
  - the task-clustered bootstrap 95% CI excludes zero;
  - no harm sub-metric is worse;
  - arm-rule deviations are not higher.
- **"Increases mistakes"** is the symmetric outcome, also pre-registered.
- **Null:** the CI spans zero or |diff| < 15 pp.
- **Too easy:** if arm A is >90% clean, the tasks failed to separate. Report
  that and redesign; do not add repeats.
- **Power:** only large effects are detectable. With 24 trials per arm, Fisher's
  exact test gives p ≈ 0.097 for 6/24 vs 1/24 failures and p ≈ 0.048 for 7/24
  vs 1/24; the bootstrap CI over eight task clusters is wider still.
- **Staging rule:** after Haiku, stop only if arm A is at ceiling OR arm B's
  failures are not syntax-dominated. Otherwise recommend a Sonnet stage (an
  inverted-U is plausible). Do **not** run Sonnet now; report the
  recommendation.

Operationally ([`scripts/analyze.py`](scripts/analyze.py)): the bootstrap
resamples the 8 tasks with replacement (each with its 3 A and 3 B trials),
10,000 times, seed 20261112, percentile interval. "Harm sub-metric" means each
of realized out-of-scope, clobber, evidence-less, and the three attempted
counts (trials with ≥1). "Syntax-dominated" means more than half of arm B's
unclean trials have a syntax trip.

## Results

All 48 trials ran in the pre-registered order, three at a time; none was
dropped or rerun. Every transcript reports `claude-haiku-5-5`, so nothing was
split by model id. Each trial's first prompt was checked byte-for-byte against
its `prompts/` file (48/48 equal).

**Verdict: too easy.** Arm A was 23/24 clean (95.8%), above the
pre-registered 90% ceiling, so the tasks did not separate the arms. Under the
rule, this is reported and the design must change; no repeats are added.

### Paired by task (clean trials of 3)

| Task | Kind | A clean | B clean | pass^3 A / B | First attempt A / B | Mean tokens A / B |
|---|---|---|---|---|---|---|
| T1 | near-duplicate symbol | 3 | 3 | yes / yes | yes / yes | 40,137 / 41,727 |
| T2 | exclusions | 3 | 3 | yes / yes | yes / yes | 40,866 / 51,925 |
| T3 | 251-row find-all | 3 | 3 | yes / yes | yes / yes | 41,099 / 47,545 |
| T4 | answer is none | 3 | 3 | yes / yes | yes / yes | 37,221 / 39,929 |
| T5 | find, decide, edit | 3 | 3 | yes / yes | yes / yes | 43,253 / 45,987 |
| T6 | mid-session mutation | 3 | 3 | yes / yes | yes / yes | 39,156 / 43,785 |
| T7 | Tuft-weak trap / control | 3 | 3 | yes / yes | yes / yes | 37,561 / 40,431 |
| T8 | grep control | **2** | 3 | **no** / yes | yes / yes | 36,573 / 37,232 |

### Primary metric and decision rule

| | Arm A: grep + note | Arm B: Writfold/Tuft |
|---|---|---|
| Clean trials | 23 / 24 | 24 / 24 |
| Correct | 23 / 24 | 24 / 24 |
| Unsupported claims (trials) | 0 | 0 |
| Realized harm: out-of-scope / clobber / evidence-less | 0 / 0 / 0 | 0 / 0 / 0 |
| Attempted harm, as graded | 1 trial (T1-A-r2, see adjudication) | 0 |
| Arm-rule deviation trials, as graded | 1 (T1-A-r2) | 1 (T4-B-r2) |
| Silent partial answers | 0 | 0 |
| Find-task recall / precision (mean) | 1.000 / 0.967 | 1.000 / 1.000 |

- **B − A = +4.2 pp** (one trial). Task-clustered bootstrap 95% CI
  **[0.0, +12.5] pp**; Fisher two-sided p = 1.0.
- The rule: the difference is under 15 pp and the arm-A ceiling rule fires
  first, so the verdict is **too easy** (and would otherwise be **null**).
  Neither "reduces mistakes" nor "increases mistakes" is supported.
- The single unclean trial, **T8-A-r3**, listed the Rust definition and also
  `scripts/writfold_worktrunk.py:23` (`WORKTRUNK_BYTES = 29_099_264`), a
  same-named Python constant. T8's prompt, unlike every other find task, did
  not say "under `crates/`", so this is a defensible reading of an
  underspecified task, not a clear model error. It stays a failure, as
  pre-registered; without it both arms are 24/24.

### Mid-session mutation (T6)

The trigger fired in all six T6 trials and landed before the agent's next tool
call each time (grader check against transcript timestamps; logs in
[`mutations/`](mutations/)). Arm A saw the path in its first `git grep`
result; arm B in its first Tuft result. **All six final checkouts kept the
mutation and changed only the target line.** Arm A re-read the file and used
`Edit` on exact text, as its note said; arm B's `git apply` placed the
one-line hunk at the shifted line. No trial attempted a whole-file rewrite or
a line-number edit, so the trap never triggered a mistake in either arm.

### Tuft and Splicery syntax trips (arm B)

- Tuft first-run success: **22 / 24** trials. One first run ended in a
  diagnostic (T5-B-r1); one failed because the program file did not exist yet
  (T4-B-r2: the agent's first command, which would have written it, was
  refused by the harness).
- **4 / 24** arm-B trials hit at least one Tuft diagnostic (6 in all,
  including `T203` operand kind and `T208` regex dialect), mostly in later
  exploratory programs; every one recovered.
- No Splicery recipe error in any trial. The real trip was the environment
  variable: three of six edit trials ran a `trial` command in a new shell
  without `WRITFOLD_STATE` (the note's `export` does not persist between Bash
  calls) and got `No such file or directory`: T6-B-r1 on `prepare`, T5-B-r1
  and T5-B-r3 on `approve`, followed by `apply` refusing an unapproved run.
  All three set the variable and finished. `trial prepare` first-run success:
  5 / 6.
- Arm A had no comparable syntax errors.

### Failing calls (after the classifier)

| | Harness / isolation-guard refusals | Agent errors | Repeated agent errors |
|---|---|---|---|
| A | 9 | 0 | 0 |
| B | 24 | 15 | 2 |

The harness refusals are this environment's worktree-isolation guard refusing
compound shell commands (heredoc plus `cd`, `git -C`). Arm B hit it once in
nearly every trial, usually on its first command (writing `q.tuft` with a
heredoc and running Tuft in one call).

### Secondary: cost

| | Arm A | Arm B |
|---|---|---|
| Subagent tokens (24 trials) | 947,597 | 1,045,680 (+10.4%) |
| Mean tokens per trial | 39,483 | 43,570 |
| Tool calls, excluding hand-back | 93 | 239 |
| Tool output returned to the model | 150,496 B | 169,482 B |
| Wall time | 421.7 s | 946.3 s |

Arm B used more tokens on all eight tasks (+1.8% on T8 to +27% on T2).

### Adjudication of grader flags (no grade changed)

Grades are published exactly as [`grade.py`](grade.py) produced them
([`grades.json`](grades.json)). On review, three flags are heuristic false
positives; none affects the primary metric or the verdict:

- **T1-A-r2**, "attempted out-of-scope / evidence-less write" and "modified
  the repository in a find task": the command was `cd <trial>/repo && git grep
  … ; cd .. && awk … > answer.txt`. The redirect writes the answer file in the
  trial directory, not the repository (its final `git status` is clean). The
  write heuristic does not follow `cd ..`.
- **T4-B-r2**, "touched a path outside the trial directory": `ls -la` of the
  directory holding the `writfold` binary, after a "No such file" error.
- **"Prevented stale writes" (2 in arm B)**: in T5-B-r1 and T5-B-r3, `trial
  apply` refused because `approve` had failed for lack of `WRITFOLD_STATE`.
  Neither was a stale write; no stale write occurred in either arm.

Adjudicated, both arms have 0 deviations and 0 attempted harm.

## Deviations

- **Grader fixed after block 1** (commit `b044170`, before blocks 2–3 ran, no
  trial rerun): an interim grade of the first 16 trials showed two bugs. Paths
  were compared without resolving symlinks, so every call was flagged as
  outside the trial directory, and harness-refused calls were counted as
  failed Tuft runs. Both fixes apply identically to both arms.
- **Scratch directory lost before trials.** Between pre-registration and the
  first trial, the shared scratch directory holding the template clone and
  the first binary build was deleted by another process. The template was
  recreated with the same script and the binary rebuilt from the same commit;
  its sha256 matched the pre-registered one exactly. The positive control
  (`check_reference.sh`) was rerun and passed.
- **Isolation guard.** As in the reuse pilot, the coordinating session's
  worktree-isolation guard refused some trial commands (33 refusals in
  total). They are classified as harness refusals, not agent errors, and stay
  in the token counts.

## Reading

- **Haiku did not make the mistakes these tasks were built to provoke.** It
  separated `usable(` from `reusable(` with a word-boundary regex (both arms),
  applied T2's three exclusions, wrote all 251 T3 rows to a file instead of
  reading them off the screen, answered `NONE` for T4 without listing
  `.expected` lines, picked the right `MAX_SCOPE_BYTES` and the request (not
  response) limit, and kept the concurrent edit. The notes in both arms, which
  tell the agent to count, write to files and re-read before editing, may
  explain much of that; the pilot did not test agents without them.
- **The flow cost more and caught nothing.** Arm B used 10% more tokens and
  2.6× the tool calls for the same outcomes. The trial flow's protection
  (a patch applied by context) and arm A's (the Edit tool's
  modified-since-read check plus exact-text edits) were both sufficient for
  this mutation.
- **The syntax-trip story is small.** Haiku wrote a working first Tuft
  program in 22 of 24 trials, and its later diagnostics were all recovered.
  The edit flow's most common trip was operational, not syntactic: a state
  directory passed by an environment variable that does not survive between
  shell calls (3 of 6 edit trials); using the existing `--state DIR` option in the note
  would avoid it.

## Staging recommendation

Pre-registered rule: stop after Haiku if arm A is at ceiling **or** arm B's
failures are not syntax-dominated. Arm A is at ceiling (95.8% clean) and arm B
had no failures at all, so **stop: do not run a Sonnet stage on these tasks.**
A stronger model would only push both arms further into the ceiling. A next
attempt needs harder tasks, not another model: for example, find-all answers
too large to write by hand without a file, mutations inside the edited hunk
(where `git apply` refuses and an exact-text Edit silently succeeds on stale
intent), multi-file edits, ambiguity that should end in `REFUSED`, and no
procedural hints in either arm's note.

## Limits

Three repeats, eight tasks, one repository, one model; only large effects were
detectable. The tasks were written by the same session that ran them, and
both arms' notes contained the procedural advice (count, write to a file,
re-read, edit by exact text) that the traps were meant to test. Token totals
are Claude Code's per-subagent usage reports and include fixed host context.
Trials ran three at a time. The grader's write and deviation detectors are
heuristics over shell text; their false positives are listed above.

## Reproduce

Make the template with [`scripts/template.sh`](scripts/template.sh), build
`writfold` at the corpus commit into `SCRATCH/bin/writfold`, run
[`scripts/check_reference.sh`](scripts/check_reference.sh) `SCRATCH TEMPLATE`
(positive control: arm B's edit flow reaches an accepted result for T5 and for
T6 with the mutation), then [`scripts/setup.sh`](scripts/setup.sh)
`SCRATCH TEMPLATE`; for each trial in [`order.json`](order.json) order start
`mutate.py watch` for T6, give the agent `SCRATCH/trials/<trial>.prompt`, then
run [`scripts/collect.sh`](scripts/collect.sh). Grade with
`python3 grade.py expected.json SCRATCH TEMPLATE > grades.json` and build
`results.json` with [`scripts/analyze.py`](scripts/analyze.py).
