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

All 24 session-2 trials ran in the pre-registered order, three at a time;
none was dropped or rerun. Every answer in both arms was exactly right.

### Session 1 (shared)

| Task | Tokens | Tool calls | Evidence retained (of complete) | Notes rows (of complete) |
|---|---|---|---|---|
| R1 | 43,989 | 9 | 50 bindings (49 + the definition) | 49 / 49 |
| R2 | 39,021 | 8 | 50 / 50 | 50 / 50 |
| R3 | 41,842 | 10 | 20 / 20 | 20 / 20 |
| R4 | 38,019 | 9 | 297 bindings (every `serde_json::` line; 101 first lines among them) | 101 / 101 |
| R5 | 36,941 | 7 | 20 / 20 | 20 / 20 |
| R6 | 40,356 | 11 | 39 / 39 | 39 / 39 |

240,168 tokens in all ([`session1/`](session1/): notes, Tuft programs,
[`check.json`](session1/check.json), usage; transcripts summarized in
[`raw-session1/`](raw-session1/)). Session 1 retained every base row of every
task, so arm B could answer every follow-up, and the notes held every base
row for arm A.

### Session 2 (measured)

| | Arm A: notes + ordinary tools | Arm B: `trial evidence list/show` |
|---|---|---|
| Correct (exact lines and text) | 12 / 12 | 12 / 12 |
| Total tokens (12 trials) | 449,673 | 457,375 (+1.7%) |
| Mean tokens per trial | 37,473 | 38,115 |
| Tool calls, excluding hand-back | 32 | 51 |
| Tool output returned to the model | 73,695 B | 57,332 B (−22%) |
| Model output tokens | 10,908 | 17,505 |
| Cache-read input tokens | 1,268,708 | 1,453,087 |
| Wall time | 172.7 s | 236.9 s |

Paired by task (mean of each cell's two repeats, B − A; tool calls include
the hand-back):

| Task | Kind | Tokens A | Tokens B | B − A | Tool calls A / B | Output bytes A / B | Output tokens A / B | Wall s A / B |
|---|---|---|---|---|---|---|---|---|
| R1 checker calls (10 of 49) | filter | 38,124 | 36,153 | −1,971 | 3 / 3 | 8,833 / 2,887 | 528 / 575 | 10.5 / 12.9 |
| R2 line after runtime `…Error` enums (14) | context | 38,090 | 39,425 | +1,335 | 5 / 12.5 | 6,994 / 3,434 | 1,046 / 3,356 | 16.5 / 36.0 |
| R3 `&changed(` lines (20) | context | 41,037 | 39,143 | −1,895 | 4.5 / 4 | 11,519 / 6,982 | 1,850 / 1,520 | 21.7 / 17.5 |
| R4 first lines with `serde_json::Value` (27) | filter | 36,078 | 38,539 | +2,461 | 3 / 4 | 3,820 / 5,823 | 633 / 1,345 | 12.0 / 21.3 |
| R5 line after `#[test]` (6) | context | 35,540 | 37,291 | +1,751 | 3.5 / 5 | 2,347 / 3,635 | 836 / 1,313 | 13.9 / 19.3 |
| R6 non-test `MAX_RESPONSE_BYTES` lines (10) | filter | 35,968 | 38,137 | +2,170 | 3 / 3 | 3,337 / 5,906 | 562 / 645 | 11.7 / 11.4 |

Mean paired token difference: **+642 tokens per task (range −1,971 to
+2,461); arm B used fewer tokens on 2 of 6 tasks (R1, R3).** Per-trial
numbers are in [`results.json`](results.json), each trial's tool calls with
scrubbed output heads in [`raw/`](raw/), the grades in
[`grades.json`](grades.json) and every answer in
[`agent-answers/`](agent-answers/).

**Verdict: no token win.** Correctness was equal (both perfect), so the
pre-registered rule applies to tokens alone: arm B used 1.7% more.

## Reading

- **Arm A did not reuse the notes either; it searched again.** All 12 arm-A
  trials answered with `grep`/`git grep` pipelines over the repository (with
  `-A1` or `sed -n` for the next line); 10 of them first printed the head of
  `notes.txt`, which accounts for much of arm A's tool output. Re-searching this repository costs one short command,
  so the earlier session's work saved neither arm much. The pilot therefore
  compares *retained-evidence read-back* against *re-grep*, not against
  reading notes.
- **Where arm B saved tokens.** When the follow-up was a filter that `list
  --path` expresses directly (R1) or a context lookup that `show
  --context-lines` answers for a few bindings (R3), arm B read less: R1-B used
  2,887 bytes of tool output against 8,833 (arm A printed 50 lines of notes
  first), and R3-B read only the six multi-line calls with context instead of
  `grep -A3` over every `decode_pin(`.
- **Where it cost tokens.**
  - Filters that are not a path prefix (R4, R6) mean listing *every* binding
    and filtering in the shell; R4's session 1 retained all 297 `serde_json::`
    lines, so arm B first had to redo the first-per-file reduction. A
    `list` with a text filter, or session 1 retaining only the answer rows,
    would remove that.
  - Rows carry a 12-hex binding and `@c568505` that the agent must strip
    with `sed` before answering, and the longer command lines (absolute state
    paths) add output tokens: 17,505 against 10,908.
  - `show` takes one binding per call. Only half the context trials used it
    (R3-B both, R5-B-r2); R2-B and R5-B-r1 read the next line with `sed -n` or
    `Read` instead, and R2-B-r1 opened 14 files one `Read` at a time after
    three refused shell commands (19 tool calls, 41,364 tokens, the costliest
    session-2 trial).
- **Fixed host context dominates again.** About 34,000 tokens of every trial
  are the subagent's own context; the per-task differences are 5% or less.

## Deviations

- **Session 1 for R4 and R5 was run twice.** In the first runs the agents'
  single shell command that wrote `notes.txt` was refused by the harness's
  worktree-isolation guard (an environment check on command shape, not a task
  rule), and both agents handed back `DONE` without notes. The pre-registration
  said session 1 is not rerun, but these were tool refusals, not content gaps,
  and an arm A with no notes would not test the design. Both failed attempts
  are kept (`S1-R4-failed`, `S1-R5-failed` in
  [`raw-session1/`](raw-session1/) and [`session1/usage.txt`](session1/usage.txt),
  72,000 tokens, not counted above); the reruns used identical prompts in fresh
  directories, and `scripts/setup_s1.sh` gained an optional task list for that.
- **The same guard refused some session-2 commands** in both arms: 7 refusals
  in 4 arm-A trials and 6 in 5 arm-B trials. Every refused trial recovered and
  answered correctly; the refusals and retries stay in the token counts.
- **R5-B-r2** changed directory outside its trial directory (to the
  coordinating worktree) to retry a refused command; it ran only the evidence
  commands there, with absolute `--state` and `--repo` paths. **R4-B-r2** read
  the harness's own saved copy of its previous `list` output. Both stay in both
  denominators. No arm-B trial searched the repository.

## Limits

Two repeats, six tasks, one repository and one model: no statistical power.
The model alias does not pin a backend revision, although every transcript
reports `claude-sonnet-5-5`. Token totals are Claude Code's per-subagent usage
reports and include fixed host context. Trials ran three at a time. The tasks
are cheap to re-search with grep, which is exactly what arm A did; the pilot
does not test evidence that is expensive to find again (a judgement across
many files, a corpus outside the checkout, a commit no longer checked out),
or a hand-over that must carry citations (a Weft report).

## Next

Three pilots now agree that on this repository, with grep available and
cheap, Tuft and its retained evidence cost a few percent more tokens than
grep at equal correctness. The read-back saved tokens only where its own
filters fit the question (path prefix, a few bindings with context). Two
small changes would test that more fairly: a text filter on `evidence list`
and a multi-binding `show`. A token advantage remains a hypothesis for
evidence that is expensive to re-derive, not a result.

Reproduce: make the template with [`scripts/template.sh`](scripts/template.sh),
build `writfold` at the binary commit, run
[`scripts/setup_s1.sh`](scripts/setup_s1.sh) `SCRATCH TEMPLATE`, give each
agent `SCRATCH/s1/S1-<task>.prompt`, run
[`scripts/check_session1.py`](scripts/check_session1.py), then
[`scripts/setup_s2.sh`](scripts/setup_s2.sh) `SCRATCH TEMPLATE`, give each
agent `SCRATCH/trials/<trial>.prompt` in [`order.json`](order.json) order,
`python3 grade.py expected.json SCRATCH/trials`, and rebuild
`results.json` with [`scripts/analyze.py`](scripts/analyze.py) from the
grades, [`usage.txt`](usage.txt) and [`raw/`](raw/).
