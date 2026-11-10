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

Not yet run.

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
