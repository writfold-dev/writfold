# Projection pilot, 9 November 2026

The third **debug pilot** under [the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md),
and milestone T2 of [the agent-boundary plan](../../docs/writfold/agent-boundary-plan.md).
Two repeats per cell, six tasks, one model: not a headline result.

## Question

Does a Tuft projection (budgeted, source-mapped excerpts over a pinned commit)
let a coding agent answer evidence-finding questions with fewer tokens than
ordinary grep-and-read, at equal or better correctness?

## Pre-registration

Written and committed before any trial ran.

- **Corpus.** This repository at commit
  `9e0c8d648852e2e6f27c6f9517c71d8e5ea67a49` (1,664 tracked files, about
  49 MB), cloned per trial and checked out detached at that commit. No
  separate fixture: the pinned commit is the fixture, and the grader's
  expected answers were extracted from it with `git show`.
- **Six tasks** ([`prompts/`](prompts/)), each asking for exact source lines
  with locations, written as `<path>:<line>: <full line text>` to
  `answer.txt`:
  - P1: definition of `retain_read` and every call site (excluding an import
    and a documentation mention);
  - P2: definition of the `record` function taking `tree: bool` (eight other
    functions share the name) and only the call sites that pass `true`;
  - P3: every line under `crates/` mentioning `Refusal::Pin`;
  - P4: every line under `crates/` containing `writfold-citation/1`,
    including one 291-byte JSON line and lines full of escapes;
  - P5: every `pub struct` whose name ends in `Backend` (a regex question);
  - P6, **the control**: the one definition line of `trial_blob`, where a
    single grep is the obvious answer.
- **Arm A:** ordinary tools (Bash with grep/rg/git grep/sed, and Read).
- **Arm B:** `writfold tuft run` for searching, reading files only when the
  Tuft output is not enough; grep, rg, git grep and find are not allowed. The
  tool note carries a seven-line Tuft quick reference and points at the full
  language reference, which the agent may read (that reading counts). The
  trial state was created with `writfold trial init` by the harness, outside
  the measured cost, as in the trial-flow pilot.
- **Identical task text** in both arms; only the tool-note bullet differs.
- **Runs:** 6 tasks × 2 arms × 2 repeats = 24 trials, each a fresh Claude Code
  `general-purpose` subagent with model alias `sonnet` (the earlier pilots'
  model), its own clone, `work/` directory and (arm B) trial state. Order is
  randomized within each repeat block with `random.Random(20261109)`; trials
  launch three at a time in that order ([`order.json`](order.json)).
- **Grader:** [`grade.py`](grade.py) succeeds a trial only if the set of
  `(path, line)` pairs equals the hidden expected set exactly and every quoted
  line equals the source line after trimming surrounding whitespace. Missing
  answers, extra lines, wrong text and unparsable lines all fail. The grader
  was checked against a perfect answer and three wrong ones before the runs.
  Agents never see the expected answers or the grader.
- **Metrics per trial:** success; total tokens as reported by the subagent's
  usage; tool calls; bytes of tool output returned to the model (from the
  subagent transcript); wall time.
- **Analysis:** raw per-trial counts and per-arm totals first, then the paired
  difference B − A per task (mean of each cell's two repeats) with its mean
  and range over the six tasks. Failures stay in both denominators. A **token
  win is claimed only if** arm B's mean tokens are lower **and** its success
  count is not lower than arm A's. With six tasks this can at most be a
  direction, not a significant effect; the result is published either way.

## Results

Not yet run.
