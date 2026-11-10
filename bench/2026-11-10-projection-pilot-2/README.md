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

Not yet run.
