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

All 24 trials ran in the pre-registered order; none was dropped or rerun.

| | Arm A: grep and read | Arm B: Tuft |
|---|---|---|
| Correct (exact lines and text) | 12 / 12 | 12 / 12 |
| Total tokens (12 trials) | 408,734 | 430,351 (+5.3%) |
| Mean tokens per trial | 34,061 | 35,863 |
| Tool calls, as reported (incl. the final hand-back) | 34 | 38 |
| Tool calls, excluding hand-back | 22 | 26 |
| Tool output returned to the model | 11,183 B | 24,579 B |
| Model output tokens (from transcripts) | 5,559 | 9,682 |
| Wall time | 110.3 s | 134.2 s |

Paired by task (mean of each cell's two repeats, B − A):

| Task | Tokens A | Tokens B | B − A | Tool calls A / B | Output bytes A / B | Wall s A / B |
|---|---|---|---|---|---|---|
| P1 definition + callers | 34,171 | 35,640 | +1,470 | 3 / 3 | 909 / 1,328 | 10.2 / 12.5 |
| P2 `record`, `true` only | 34,993 | 37,870 | +2,877 | 4 / 3.5 | 2,209 / 6,028 | 12.0 / 12.8 |
| P3 `Refusal::Pin` | 33,716 | 35,490 | +1,774 | 2 / 3 | 696 / 1,337 | 7.6 / 9.6 |
| P4 `writfold-citation/1` | 33,942 | 36,000 | +2,058 | 2 / 3 | 1,317 / 2,015 | 6.4 / 10.8 |
| P5 `pub struct …Backend` | 33,847 | 35,105 | +1,258 | 2.5 / 3 | 288 / 808 | 9.8 / 9.9 |
| P6 control | 33,699 | 35,073 | +1,374 | 3.5 / 3.5 | 174 / 775 | 9.1 / 11.5 |

Mean paired token difference: **+1,801 tokens per task (range +1,258 to
+2,877); arm B used fewer tokens on 0 of 6 tasks.** Per-trial numbers are in
[`results.json`](results.json), each trial's tool calls with scrubbed output
heads are in [`raw/`](raw/), and every answer and Tuft program the agents
wrote is in [`agent-answers/`](agent-answers/).

**Verdict: no token win.** Correctness was equal (both perfect), so the
pre-registered rule applies to tokens alone, and arm B used more on every task,
including the control.

## Reading

- **Correctness was at ceiling in both arms.** Every agent found exactly the
  right lines, including the disambiguation traps (the `use` import in P1, the
  eight other `record` functions and the `false` call sites in P2). These
  tasks did not separate the arms on quality.
- **Where arm B's extra tokens came from.** Three measurable sources:
  - the tool note: arm B's prompt is about 1.9 KB longer (the quick reference
    and the longer command line), roughly a third of the gap;
  - transcription: 8 of 12 arm A agents piped `grep -n | sed` straight into
    `answer.txt` without retyping a line. Arm B rows are JSON-quoted with byte
    ranges, so all 12 arm B agents retyped the answer by hand: 9,682 output
    tokens against 5,559;
  - output size: the Tuft envelope (program digest, find/retain/view lines,
    byte ranges) roughly doubled the tool output bytes for the same hits.
- **The quick reference was enough.** No arm B agent opened the full
  language reference, and every Tuft program ran on the first attempt.
- **Fixed host context dominates.** About 33,000 tokens of every trial are
  the subagent's own context, so a 5% difference is the size of a tool note.
  At this scale neither tool can save much.
- **Protocol deviation.** In P2-B-r2 the agent piped one Tuft run's output
  through `grep write_objects`, filtering tool output rather than searching
  the repository, which the arm rule forbids. The trial stays in both
  denominators. Its tokens (36,904) are the highest of all 24 trials, so
  excluding it would not change the verdict.

## Limits

Two repeats, six tasks, one repository and one model: no statistical power, and
the tasks have 1 to 9 hits each, where grep output is already compact. Tuft's
budget, omission counts and resume cursor never triggered, so this pilot does
not test them. Nor does it value the `writfold-citation/1` bindings Tuft
retains; arm A produces no citations at all. Token totals are Claude Code's
per-subagent usage reports and include fixed host context. Bytes and output
tokens come from the subagent transcripts
([`scripts/summarize.py`](scripts/summarize.py)). The model alias does not
pin a backend revision, although the transcripts report `claude-sonnet-5-5`.
Trials ran three at a time, so wall times include contention.

## Next

A token advantage for projection is plausible only where grep is not already
compact: queries with hundreds of hits where a budgeted, sorted, limited view
replaces paging through output, or tasks that must hand on cited evidence (a
Weft report, a later session), where arm A would also have to retain bytes.
A plain-text row format that pipes straight to a file would remove the
transcription cost seen here. Those are hypotheses for a later pilot, not
results.

Reproduce: per trial, clone this repository into `<trial>/repo` detached at
the pinned commit. For arm B, also run `writfold trial init --state <trial>/state`.
Give the agent `prompts/<task>-<arm>.txt` with `<trial>`, `<commit>` and
`<writfold>` filled in, then run `python3 grade.py expected.json <trials-dir>`
with one directory per trial named `<task>-<arm>-r<n>` containing
`answer.txt`. [`reference-programs/`](reference-programs/) holds a Tuft
program per task that finds every expected line, the arm-B positive
control. [`scripts/analyze.py`](scripts/analyze.py) rebuilds `results.json`
from the grades, the harness usage reports and the transcript summaries.
