# Projection pilot 2, 10 November 2026

The second projection pilot, a **debug pilot** under
[the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md).
It repeats [the first projection pilot](../2026-11-09-projection-pilot/README.md)
with tasks that have large result sets, and with Tuft's lean output (compact
`path:line: text` rows, `--out FILE` and the 594-byte
[agent note](../../docs/tuft/agent-note.md)). Two repeats per cell, six tasks,
one model: not a headline result.

## Question

Does lean Tuft output let a coding agent answer evidence-gathering questions
with **large result sets** using fewer tokens than grep-and-read, at equal or
better correctness?

## Pre-registration

Written and committed before any trial ran. The methodology is the first
pilot's; only the tasks, the arm-B tool note and the arm-B post-processing rule
differ, as stated below.

- **Corpus.** This repository at commit
  `c568505742d00859cad758924c98d8d0a6e98648` (1,771 tracked files; 644 under
  `crates/`). Each trial gets its own clone detached at that commit, made from a
  template clone created before this pre-registration was committed, with no
  remote and no branches, so trials cannot see later commits or the expected
  answers ([`scripts/setup.sh`](scripts/setup.sh)).
- **Binary.** `writfold` release build of that same commit
  (`cargo build --release -p writfold`), sha256
  `8813db29b305bd18728776747f612860ab5459b44cd3a60b1bfdcc34e31a345b`.
- **Six tasks** ([`prompts/`](prompts/)), each answered as
  `<path>:<line>: <full line text>` lines in `answer.txt`. "Matching lines" is
  the line count of the obvious single search; "answer" is the expected row
  count ([`expected.json`](expected.json), extracted by
  [`scripts/expected.py`](scripts/expected.py)):

  | Task | Shape | Matching lines | Answer rows |
  |---|---|---|---|
  | P1 | every call line of `claim_matches_spec(` under `crates/`, not its definition | 50 | 49 |
  | P2 | files under `crates/` mentioning `serde_json::`, first such line per file | 297 | 101 |
  | P3 | `decode_pin` calls whose first argument is `&changed(...)`; six calls span lines, so the `&changed(` is on the next line | 52 | 20 |
  | P4 | **handful of hundreds:** the last 6 `.to_owned()` lines under `crates/` in (path, line) order; Tuft's default `limit 100` stops the search early | 224 | 6 |
  | P5 | every `pub enum` under `crates/` whose name ends in `Error` (a regex) | 50 | 50 |
  | P6 | **control, grep should win:** the definition line of `PIN_BYTES` | 5 | 1 |

- **Arm A:** ordinary tools (Bash with grep/rg/git grep/sed, and Read), with
  the first pilot's wording.
- **Arm B:** `writfold tuft run` for searching the repository; grep, rg, git
  grep, find and other search commands on the repository are not allowed.
  Reading a file (Read, or sed on a line range) is allowed when Tuft's output
  is not enough. **Changed from the first pilot:** post-processing Tuft's own
  output (for example the `--out` file) with ordinary shell tools is allowed,
  because the agent note invites it. The tool note is
  [`docs/tuft/agent-note.md`](../../docs/tuft/agent-note.md)'s fenced text
  verbatim, with `<trial>`, `<commit>` and `<writfold>` filled in; it does not
  mention the language reference. Trial state is created with
  `writfold trial init` by the harness, outside the measured cost.
- **Identical task text** in both arms; only the tool rule and note differ
  ([`scripts/prompts.py`](scripts/prompts.py)).
- **Runs:** 6 tasks × 2 arms × 2 repeats = 24 trials, each a fresh Claude Code
  `general-purpose` subagent with model alias `sonnet` (the earlier pilots'
  alias), its own clone, `work/` directory and (arm B) trial state. Order is
  randomized within each repeat block with `random.Random(20261110)`; trials
  launch three at a time in that order ([`order.json`](order.json)).
- **Grader:** [`grade.py`](grade.py), unchanged from the first pilot: a trial
  succeeds only if its set of `(path, line)` pairs equals the hidden expected
  set exactly and every quoted line equals the source line after trimming
  surrounding whitespace. Missing, extra, wrong-text and unparsable lines all
  fail. Checked before the runs against a perfect answer, a missing row, an
  extra row and a wrong text for every task (6 pass, 18 fail, as intended).
  Agents never see the expected answers or the grader.
- **Positive control:** [`reference-programs/`](reference-programs/) holds a
  Tuft program per task whose `--out` rows contain every expected line with
  its exact text ([`scripts/check_reference.py`](scripts/check_reference.py)),
  so arm B can succeed. `P3-context.tuft` shows the one-line context that
  resolves the six multi-line calls.
- **Metrics per trial:** success; total tokens from the subagent's usage
  report; input, cache and output tokens from the transcript; tool calls; bytes
  of tool output returned to the model; wall time.
- **Analysis:** raw per-trial counts and per-arm totals first, then the paired
  difference B − A per task (mean of each cell's two repeats) with its mean and
  range over the six tasks. Failures stay in both denominators. A **token win
  is claimed only if** arm B's mean tokens are lower **and** its success count
  is not lower than arm A's. With six tasks this can at most show a direction;
  the result is published either way. Arm-rule deviations are recorded and kept
  in both denominators.

## Results

All 24 trials ran in the pre-registered order, three at a time; none was
dropped or rerun, and no arm-rule deviation was found in the transcripts
(every arm-B `grep`, `awk`, `sort` or `sed` ran on Tuft's own output, and the
one `sed -n` on a source file was a permitted line-range read).

| | Arm A: grep and read | Arm B: Tuft |
|---|---|---|
| Correct (exact lines and text) | 12 / 12 | 12 / 12 |
| Total tokens (12 trials) | 424,110 | 437,387 (+3.1%) |
| Mean tokens per trial | 35,343 | 36,449 |
| Tool calls, as reported (incl. the final hand-back) | 33 | 42 |
| Tool calls, excluding hand-back | 21 | 30 |
| Tool output returned to the model | 43,435 B | 40,994 B (−5.6%) |
| Model output tokens (from transcripts) | 5,667 | 12,403 |
| Cache-creation input tokens (from transcripts) | 232,210 | 226,449 |
| Cache-read input tokens (from transcripts) | 881,755 | 1,238,579 |
| Wall time | 129.0 s | 234.2 s |

Uncached input tokens were 2–4 per API call in every trial, so they are not
tabulated; they are in [`results.json`](results.json).

Paired by task (mean of each cell's two repeats, B − A):

| Task | Tokens A | Tokens B | B − A | Tool calls A / B | Output bytes A / B | Output tokens A / B | Wall s A / B |
|---|---|---|---|---|---|---|---|
| P1 callers (49 of 50) | 36,978 | 38,225 | +1,248 | 3 / 3 | 7,427 / 7,840 | 467 / 880 | 9.4 / 14.3 |
| P2 first per file (101 of 297) | 33,616 | 34,893 | +1,277 | 2 / 3 | 367 / 668 | 287 / 705 | 6.6 / 24.8 |
| P3 `&changed` calls (20 of 52) | 38,134 | 38,742 | +609 | 3.5 / 4.5 | 8,655 / 6,301 | 765 / 1,926 | 17.9 / 30.7 |
| P4 last 6 of 224 | 34,283 | 35,263 | +981 | 3 / 3.5 | 1,197 / 1,301 | 552 / 833 | 11.2 / 16.1 |
| P5 `…Error` enums (50) | 35,305 | 37,056 | +1,751 | 2 / 4 | 3,725 / 4,124 | 324 / 1,230 | 7.0 / 18.8 |
| P6 control (1) | 33,741 | 34,515 | +774 | 3 / 3 | 347 / 264 | 441 / 630 | 12.4 / 12.4 |

Mean paired token difference: **+1,106 tokens per task (range +609 to
+1,751); arm B used fewer tokens on 0 of 6 tasks.** Per-trial numbers are in
[`results.json`](results.json), each trial's tool calls with scrubbed output
heads in [`raw/`](raw/), and every answer and Tuft program the agents wrote in
[`agent-answers/`](agent-answers/) (arm A agents wrote no scratch files).

**Verdict: no token win.** Correctness was equal (both perfect), so the
pre-registered rule applies to tokens alone, and arm B used more on every task.
The gap narrowed from the first pilot's +5.3% (+1,801 per task) to +3.1%
(+1,106 per task), but the direction did not change.

## Reading

- **Correctness was again at ceiling.** Every agent in both arms got every
  task exactly right, including the six multi-line calls in P3, the
  first-per-file reduction over 297 lines in P2 and the 224-line tail in P4.
- **The lean output fixed transcription.** Every arm-B agent wrote its answer
  by piping Tuft's rows (or an `--out` file, 8 of 12 trials) through
  `awk`/`sed`/`sort`/a short Python filter, as arm A piped grep. Nobody retyped
  rows. Tool output bytes were now slightly *lower* in arm B (−5.6%), lowest
  relative to arm A on P3, where Tuft's one-line context replaced a separate
  `grep -A` pass.
- **Where arm B's extra tokens came from now.**
  - Model output: arm B wrote 12,403 output tokens against 5,667. The Tuft
    program (four lines with a 40-digit commit) and the longer command line
    cost more to write than a one-line grep pipeline, in every trial.
  - More calls: 30 tool calls against 21. Arm-B agents often ran Tuft once to
    look, then again with `--out` or a filter (P1, P5); arm-A agents usually
    did both in one pipeline. Each extra call re-reads the context, which shows
    as 40% more cache-read input.
  - The tool note: arm-B prompts are about 860 bytes longer (1,749 vs 888
    bytes for P1), a fifth to a quarter of the gap.
- **Large result sets did not change the economics.** The model never had to
  page through hundreds of rows in either arm: arm A filtered with
  `awk '!s[$1]++'`, `tail -6` or `grep -A1` inside one pipeline, so its output
  stayed as small as Tuft's. Budget and cursor still did not engage: arm-B
  agents raised `limit` (100 to 5,000) and `budget_tokens` (4,000 to 200,000)
  up front, every observed trailer read `coverage=complete … omitted=0
  cursor=none`, and no agent used `--cursor`.
- **One Tuft trial beat both arm-A trials on its task.** P3-B-r1 used a
  multi-line regex, `decode_pin\(\s*&changed\(`, that matched across the line
  break and reported the opening line directly (36,208 tokens against 38,418
  and 37,849). Its repeat P3-B-r2 took the long way (six calls, 41,276
  tokens), so the cell mean is still higher.
- **Fixed host context dominates.** About 33,000 tokens of every trial are the
  subagent's own context; the per-task differences are 2–5% of a trial.
- **No agent opened the language reference**; the 594-byte note was enough for
  every program to run on the first attempt.

## Limits

Two repeats, six tasks, one repository and one model: no statistical power.
The model alias does not pin a backend revision, although every transcript
reports `claude-sonnet-5-5`. Token totals are Claude Code's per-subagent usage
reports and include fixed host context; bytes, call counts and output/cache
tokens come from the transcripts
([`scripts/summarize.py`](scripts/summarize.py), unchanged from the first
pilot). Trials ran three at a time, so wall times include contention. Arm B's
post-processing rule differs from the first pilot's (now allowed, as the note
invites), so the two pilots' arm-B numbers are not strictly comparable. The
pilot still does not value the `writfold-citation/1` bindings Tuft retains, and
it does not test tasks where grep output cannot be reduced inside one shell
pipeline.

## Next

Two pilots now agree that, for single-shot evidence lookups that a shell
pipeline can reduce, Tuft costs a few percent more tokens than grep at equal
correctness, and the remaining cost is the agent writing a longer program, not
output volume. A token advantage is still plausible only where arm A cannot
reduce output in one pipeline (results that must be read and judged across many
files) or where evidence must be handed on with citations (a Weft report, a
later session). Those are hypotheses, not results.

Reproduce: make a template clone detached at the pinned commit (no remote, no
branches), build `writfold` at that commit, run
[`scripts/setup.sh`](scripts/setup.sh) `SCRATCH TEMPLATE`, give each agent
`SCRATCH/trials/<trial>.prompt` in [`order.json`](order.json) order, copy each
`answer.txt` into `<dir>/<trial>/answer.txt`, then
`python3 grade.py expected.json <dir>`. [`scripts/analyze.py`](scripts/analyze.py)
rebuilds `results.json` from the grades, the harness usage reports
(`trial tokens tool_uses duration_ms` per line) and `raw/`.
