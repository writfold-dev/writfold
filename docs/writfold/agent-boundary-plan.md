# Agent boundary and latency plan (9 November 2026)

Owner input: a design supplement, "Unified languages, review sessions and
enforced agent boundary". This document records what Writfold takes from it, what it
declines, and the delivery plan. It is a scoped current plan in the sense of
`AGENTS.md`. It does not amend any handover, renumber gates or claim a pass.

## Accepted

- **One owner, one boundary.** CLI and `serve` share the same typed
  application decisions (ADR-U01, U02). Today `serve` answers only
  `initialize`/`shutdown`; the trial decisions move behind it (S3).
- **Separate languages.** Weft and Splicery keep their own languages and IRs.
  There is no shared VM or merged grammar (U04).
- **Honest profiles.** UI mode, policy profile and runtime containment are
  different things (U07, U08). The cooperating profile *is*
  `writfold-trial-worktree/1`; no third profile is added. A strict profile
  must refuse when containment cannot be established. It is deferred to the
  existing WF18/WF27/P16 work and is never claimed for the trial.
- **One vertical slice first** (U10), starting from the flow that runs today
  rather than from search.
- **Independent status badges** instead of one aggregate green (from §5.3).
- **New pure decisions are proved over the shipped Rust** (U09, as amended by
  the 9 November Aeneas decision): `crates/writfold-core` plus `proofs/`.
  System properties (non-promotion) belong to `formal/`, and interleavings to
  TLA+.
- §4.2's table of what policy can and cannot force, §4.5's threat list as a
  test backlog, §6.4's replay/recheck/reevaluate/observe distinction (already
  in `shared-client-references.md`), and §11's non-goals.

## Declined or deferred

- **Tuft: accepted as v0 over the pinned Git tree; corpus search deferred.**
  The owner kept Tuft for two reasons. Budgeted, source-mapped projections are
  the one untested lever on the token cost the pilots measured. Running
  query → evidence → check → edit in one go cuts round trips and keeps every
  handoff typed.
  - **What v0 has:**
    - `find` is exact text or regex over **one pinned commit**, with no ranking.
      Over a pinned tree, exact search is complete, so zero results means zero.
    - `retain` keeps exact bytes as `writfold-citation/1` bindings.
    - `project`/`emit` select fields and give source-mapped excerpts under a
      token budget, with explicit omissions and a cursor.
  - **What waits for Omnigraph** (P01O/WF24, no fallback): `follow` over graph
    relations, semantic or hybrid `find`, fuzzy/similarity matching and
    ranking. They arrive behind the same grammar and the B0 storage trait.
  - **Overlap:** v0 maps onto the admitted revision-9 vocabulary
    (`CandidateHandle`, `ScopeCut`, `Coverage`, `NoticeCursor`) rather than
    inventing parallel types.
  - **Where it lives:** Tuft starts as crates in this repository, because it
    queries Writfold's own retained store. It moves to its own repository if the
    owner creates one.
- **Review Session / `review {}` block: deferred.** A fourth grammar embedding three
  others is the "admitted faster than connected" failure the 8 November review
  named. Run Watch (revision-9 plan 27) already owns durable run views. The
  badges land in `trial show` first.
- **MCP adapter and LSP: deferred.** They have no methods to adapt yet. Revisit
  after S3.
- **The proposal's speculative RPC method list: declined.** Methods are added
  only when the owner publishes them with tests (S3 publishes `writfold.trial/1`).
- **M7 "full composed qualification": declined as a new milestone.** It is P11/WF31.

## Latency budget

Agents call these operations many times per task, so latency compounds. Every
agent-facing operation has a **hard ceiling of 5 seconds** and a **target of
1 second at p95** on the reference machine. Read-only views (`show`,
`status`) target 250 ms. The ceiling applies to the operation as the agent
sees it: process start to exit, or request to response.

The budget does not apply to qualification gates, proofs or builds. Those are
developer and CI work, not the agent loop. Any future operation that cannot
meet the ceiling, such as indexing, a heavy Weft check or a corpus rebuild, must
return a durable handle at once and be observed, never block the call.

**Measured before any change** (release binary, WSL2, warm cache):

| Operation | Small repository | 100,000-file repository (200 MB) |
|---|---|---|
| `trial init` | 72 ms | — |
| `trial prepare` | 183 ms | **7,772 ms** |
| `trial approve` | 189 ms | 196 ms |
| `trial apply` | 412 ms | 431 ms |
| `trial show` | 161 ms | 167 ms |

After the change (`bench/2026-11-09-latency/results.json`, 5 repeats each),
`prepare` on the 100,000-file repository runs at a median of 163 ms and a p95
of 239 ms. Every operation's p95 is under 0.5 s on both repositories.

`prepare` broke the ceiling because `git worktree add` checks out the whole
tree (4.1 s alone). The fix: add the worktree without a checkout and fill it
with one sparse `read-tree -mu` over exactly the edited paths. Other entries are
skip-worktree, so `git status`, `git diff` and commits behave normally, and
`core.sparseCheckout` is passed for that one command so the user's repository
config is unchanged. Measured standalone: 4.1 s → 83 ms. The latency bench
(`bench/latency/run.sh`) records every operation against both repositories,
and the ceiling is checked there.

Through `serve` (S3, `bench/latency/serve.py`, 10 calls each,
`bench/2026-11-09-serve-latency/results.json`), the p95 on the 100,000-file
repository is 27 ms for `trial/retain`, 166 ms for `trial/prepare` and 53 ms for
`trial/show`. The revision adding `trial/run` and `trial/evidence-*`
(`bench/2026-11-10-serve-run-evidence-latency`, stub Weft fixed by
`serve --weft`) measured p95 393 ms for `trial/run`, 29 ms for
`trial/evidence-list` over 1,001 bindings and 12 ms for `trial/evidence-show`
with three context lines. The real Weft step (up to 30 s) is outside the budget
and returns no handle yet.

## Delivery plan

| # | Milestone | Exit test |
|---|---|---|
| L0 | Latency budget and sparse trial worktree | The bench shows every trial operation ≤ 1 s p95 on the 100k-file repository; worktree status/diff/commit stay correct; the repository config is unchanged |
| S0 | Weft report is read, not just hashed: `prepare --report` requires a complete `weft.report/1`; `approve --require <check>` refuses unless each named check is `entailed` | Refuted, undetermined, malformed and incomplete reports refuse; entailed proceeds; removing the check fails a test |
| K1 | Pure trial decisions in `writfold-core`, proved: recovery outcome (`Applied ⇔ observed = candidate`, `Abandoned ⇔ observed = source`, else Unknown, given source ≠ candidate) and the required-verdict gate; `prepare` refuses a no-change candidate | Aeneas theorems, axiom audit and calibrated mutants |
| S2 | Five independent badges in `trial show`: evidence, assessment, approval, claim, effect | Golden output for each ledger shape |
| S1 | `trial retain`: exact bytes of a pinned Git blob range, retained and bound as `writfold-citation/1`; a report citing unretained or mismatched bytes refuses `prepare` | Equal text at two positions gives two bindings; HEAD is never substituted for the pinned commit |
| S3 | `serve` capability `writfold.trial/1` with show/retain/prepare; **approve stays CLI-only** so an agent session cannot approve its own edit | Negotiation, MethodNotFound for approve, refreshed stdio-control receipt, Weft's vendored contract updated. Implemented with engine, wire and real-process tests ([trial-profile.md](trial-profile.md#through-serve)); apply and recover also stay CLI-only. Fresh stdio-control receipt for contract `0baf69b2…` |
| S4 | Two-phase adversarial pilot: injected file change, SIGKILL after install, expired approval, refuted required check, direct checkout write | Every injected case ends in its expected ledger outcome; results retained |
| T0 | Tuft v0: grammar, typed IR, `find`/`retain`/`project`/`emit` over a pinned commit, deterministic output and budget | Golden cases; zero-result completeness on a pinned tree; budget overflow emits omissions and a cursor, never silent truncation; excerpts resolve to retained bytes |
| T1 | One-go run file: Tuft → Weft check → Splicery prepare, stopping at approval | One command produces evidence, report and prepared edit bound by digest; approval stays a separate CLI step |
| T2 | Projection pilot: Tuft projections vs grep-and-read, same tasks and model | Tokens and correctness for both arms, published either way |
| B0 | Swappable storage-backend boundary with Dolt as the interim backend (owner, 9 November; replaces the earlier D0 "retire Dolt" row, which was reversed before anything was removed). See [storage-backend.md](storage-backend.md) | The Dolt adapter passes the trait-only conformance suite against a real disposable Dolt database; fuzzy, semantic and ranked lookups refuse by type; undeclared writes refuse `Unavailable`; quality and unit gates green |
| S5 | Source pre-release tag `v0.1.0-dev.1` ("trial preview"), README "what works" table, website | Fresh-clone commands reproduce; fresh gate receipts at the tag |

Formal follow-ups, in order:

- TLA+ trial-stream interleavings (apply, recover, an external writer and
  crash points; at most one outcome; no redispatch after a claim).
- The authority-timing decision: a claim made under valid authority may
  complete after expiry.
- After S3, concurrent `serve` sessions.

## Progress (9 November 2026)

Each row's evidence is the `qualification/status.json` entry named here:

| Row | Status | Evidence |
|---|---|---|
| L0 | done | `latency_budget_component` |
| S0, K1, S2 | done | `trial_assessment_component` |
| S1 | done | `trial_retain_component` |
| S3 | done | `serve_trial_component` (incl. WF13 `stdio-control` and Weft's `control` re-pin) |
| S4 | done | `trial_adversarial_campaign`. Not covered: expired approval, a kill between claim and install, and an agent writing the checkout itself |
| T0 | done | `tuft_v0_component` |
| B0 | done | `storage_backend_component`. The Dolt conditional writer is built (`dolt_writer_component`; quality and unit gates pass, including the writer suite against real Dolt 2.6.1) |
| S5 | done | tag `v0.1.0-dev.1`; the fresh-clone flow was reproduced |
| T2 | done: no token win | `bench/2026-11-09-projection-pilot`. Correctness was 12/12 in both arms; Tuft used 5.3% more tokens, more on all six tasks. The tasks had 1–9 matches, so the budget and omissions never engaged. Agents also retyped Tuft's quoted rows by hand instead of piping them, and its header lines doubled tool output |
| T2b | done: no token win | `bench/2026-11-10-projection-pilot-2`. Lean output, tasks with 50–297 matches. Correctness 12/12 in both arms; Tuft used 3.1% more tokens, more on all six tasks. Transcription and output volume no longer cost anything (arm-B tool output was 5.6% lower); the gap is the agent writing longer programs and making more calls. Budget and cursor still never engaged |
| T1 | done | `trial_run_component`; `writfold trial run` ([one command](trial-profile.md#one-command)); `bench/2026-11-10-trial-run-latency` (median 319 ms on the 100k-file repository with a stub Weft, within 10 ms of its parts) |
| T2c | done: no token win (`trial_evidence_reuse`) | `bench/2026-11-10-reuse-pilot`. A later session reads evidence an earlier one retained (`trial evidence list/show`) against that session's notes plus grep. Correctness 12/12 in both arms; read-back used 1.7% more tokens (+642 per task, range −1,971 to +2,461), fewer on 2 of 6 tasks (a path filter and a few context lookups). Arm A re-grepped instead of reusing its notes |

Next after T1. T2 suggested a compact Tuft row format and an `--out` flag. Both are built: `path:line: text` rows with a one-line coverage trailer, `--out` that writes the rows to a new file, and a 594-byte [agent note](../tuft/agent-note.md). T2b measured them on larger result sets and found no token win. Next:

- ~~the TLA+ trial-stream model~~ and ~~the edit-flow validator in `writfold-core`~~: done (`trial_formal_component`);
- ~~the cursor moved onto `NoticeCursor`~~: assessed, not a fit. `NoticeCursor` is a borrowed, in-process position over a `NoticeQuery` with no serialized form, and Tuft's cursor crosses processes. It is now a nominal `tuft::cursor::Cursor` with a versioned `v2.P.C.OFFSET.K` encoding bound to program, commit, offset and result, refusing with typed errors ([language](../tuft/language.md#revision-9-vocabulary));
- ~~serve methods for `trial/run` and `trial/evidence-*`~~: built as a revision of
  `writfold.trial/1` ([trial-profile.md](trial-profile.md#through-serve); contract
  `0baf69b2…` → `bfa436ba…`; the Weft binary is fixed by `serve --weft`,
  never named by the client; `bench/2026-11-10-serve-run-evidence-latency`). Still
  needed: a fresh WF13 `stdio-control` receipt for the new contract, and Weft's
  `control` re-pin to it and its executable;
- DINT02 publication over the Dolt writer.

## Website wording after the slice

**May say:**

- the trial records and approves edits through a durable ledger;
- a Weft report can be required, with Weft's verdicts recorded as
  hypothetical, not grounded;
- exact retained excerpts are cited by digest;
- the ledger, grant and trial decisions are proved over the shipped Rust;
- measured latency and token overhead.

**Must not say:**

- sandbox, containment, strict;
- "cannot bypass";
- grounded, corpus, semantic search, ranking;
- "unified interface" before S3;
- token savings (unless T2 measures them), alpha, installable;
- "govern what the agent may know".
