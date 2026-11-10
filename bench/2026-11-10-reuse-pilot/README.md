# Evidence reuse pilot, 10 November 2026

A **debug pilot** under
[the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md), after
[projection pilot 2](../2026-11-10-projection-pilot-2/README.md) found Tuft
within about 3% of grep's tokens for one-off lookups. It tests the remaining
plausible token story: **reuse**. A later session reads evidence an earlier
session retained, with `writfold trial evidence list/show`, instead of
searching and re-reading. Two repeats per cell, six tasks, one model: not a
headline result.

## Question

Does reading evidence retained by an earlier session let a fresh agent answer
follow-up questions about that evidence using fewer tokens than the earlier
session's plain-text notes plus ordinary tools, at equal or better correctness?

## Pre-registration

Written and committed before any trial ran. The methodology is the projection
pilots'; what differs is the two-session design below.

- **Corpus.** This repository at commit
  `c568505742d00859cad758924c98d8d0a6e98648` (pilot 2's corpus; 1,771 tracked
  files). Every session gets its own clone detached at that commit, made from a
  template clone created before this pre-registration was committed, with no
  remote, no refs and every object not reachable from that commit pruned
  ([`scripts/template.sh`](scripts/template.sh)), so no session can see later
  commits or the expected answers.
- **Binary.** `writfold` release build of commit
  `aea2ded502016de9751cd2d738d307b7de427cc9` (the commit adding
  `trial evidence`), sha256
  `bba0dd133d1eaf5f6a46ff5683c6cebc1a4bde66bc000e1c2f3922ae60d0e609`.
- **Session 1 (shared, run once per task, not part of the comparison).** A
  fresh agent gathers the evidence for a task with Tuft
  ([`prompts/S1-*.txt`](prompts/)): it retains every evidence line
  (`retain @h around lines 0`) in its trial state **and** writes plain-text
  notes (`notes.txt`, evidence as `path:line: text` plus anything it thinks a
  later session needs). Search rules and tool note are pilot 2's arm B
  (the [agent note](../../docs/tuft/agent-note.md) verbatim). Its cost is
  reported separately and is the same for both arms. Session 1 is not rerun;
  after it, [`scripts/check_session1.py`](scripts/check_session1.py) records how
  much of each task's complete evidence was retained and noted, before
  session 2 starts. Gaps are shared by both arms and stay in the results.
- **Session 2 (measured).** A fresh agent answers a follow-up question about
  the same evidence, as `<path>:<line>: <full line text>` lines in
  `answer.txt`. Task text is identical in both arms
  ([`scripts/prompts.py`](scripts/prompts.py)); only the tool rule and the
  handed-over material differ.
  - **Arm A:** a copy of session 1's `notes.txt`, plus ordinary tools (Bash
    with grep/rg/git grep/sed, and Read), with pilot 2's arm-A wording.
  - **Arm B:** a copy of session 1's trial state (no notes), read with
    `writfold trial evidence list/show` through a 7-line tool note in the
    prompt. grep, rg, git grep, find and other searches on the repository are
    not allowed; reading a repository file (Read, or sed on a line range) is
    allowed when the evidence output is not enough; post-processing the
    evidence commands' own output is allowed.
- **Six tasks.** Three follow-ups are filters over the evidence, three need
  the line next to it (which `show --context-lines` gives in arm B, and which
  arm A must read from the files). Answer rows are extracted by
  [`scripts/expected.py`](scripts/expected.py) into
  [`expected.json`](expected.json):

  | Task | Session-1 evidence | Rows | Session-2 follow-up | Answer rows |
  |---|---|---|---|---|
  | R1 | calls of `claim_matches_spec(` under `crates/` | 49 | those under `crates/writfold-kernel/src/checker/` | 10 |
  | R2 | `pub enum …Error` declarations | 50 | for those under `crates/writfold-runtime/`, the line after the declaration | 14 |
  | R3 | `decode_pin(&changed(…))` calls (six span lines) | 20 | the line holding each call's `&changed(` | 20 |
  | R4 | first `serde_json::` line per file | 101 | those whose text contains `serde_json::Value` | 27 |
  | R5 | `#[test]` lines in `crates/writfold-app/src/trial/` | 20 | in `retain_tests.rs`/`recovery_tests.rs`, the line after each | 6 |
  | R6 | lines mentioning `MAX_RESPONSE_BYTES` | 39 | those in non-test files | 10 |

- **Runs:** 6 session-1 runs, then 6 tasks × 2 arms × 2 repeats = 24
  session-2 trials, each a fresh Claude Code `general-purpose` subagent with
  model alias `sonnet` (the earlier pilots' alias), its own clone and `work/`
  directory. Session-2 order is randomized within each repeat block with
  `random.Random(20261111)`; trials launch three at a time in that order
  ([`order.json`](order.json), [`scripts/setup_s2.sh`](scripts/setup_s2.sh)).
- **Grader:** [`grade.py`](grade.py), unchanged from the projection pilots: a
  trial succeeds only if its set of `(path, line)` pairs equals the hidden
  expected set exactly and every quoted line equals the source line after
  trimming surrounding whitespace. Agents never see the expected answers or the
  grader.
- **Positive control:** the unit tests of `trial evidence` show that `list`
  returns each retained evidence row and that `show --context-lines` returns
  the neighbouring lines from the pinned blob; `check_session1.py` shows
  whether session 1 retained each task's base rows.
- **Metrics per session-2 trial:** success; total tokens from the subagent's
  usage report; output and cache tokens from the transcript; tool calls; bytes
  of tool output returned to the model; wall time
  ([`scripts/summarize.py`](scripts/summarize.py) and
  [`scripts/analyze.py`](scripts/analyze.py), pilot 2's with the labels
  changed).
- **Analysis:** raw per-trial counts and per-arm totals first, then the paired
  difference B − A per task (mean of each cell's two repeats) with its mean and
  range over the six tasks. Failures stay in both denominators. A **token win
  is claimed only if** arm B's mean tokens are lower **and** its success count
  is not lower than arm A's. With six tasks this can at most show a direction;
  the result is published either way. Arm-rule deviations are recorded and kept
  in both denominators.

## Results

Pending.
