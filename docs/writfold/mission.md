# Writfold mission: project continuity for agents

Owner decision, 11 November 2026. This document is the current authority for
what Writfold builds next. It supersedes the delivery order in
`agent-boundary-plan.md` and the gate programme in `handovers/revision-5/` as
live obligations; those stay in the repository as history and are not edited.

## Mission

A Git-native project memory. A cold agent session on any machine can answer:

- why did we decide X;
- where do we stand on Y;
- what is the tracker item for Z;
- what does changing X touch.

Every answer is pinned to a source (a path at a commit, with a digest) and
marked when that source has moved. The memory is plain records in Git, so a
new machine needs `git clone` and one static binary, nothing else.

**Acceptance test:** the owner, who uses an earlier private prototype of this
daily, replaces it with the `writfold` binary for real work. No public
benchmark is planned; benchmarking is paused (owner, 10 November 2026).

## Why

- The only strong, first-hand signal of value in the whole project is daily
  use of project continuity: resuming large work across fresh sessions and
  ephemeral machines, and asking why, status, tracker and impact questions.
- Seven pilots of the evidence, check and edit flow (Tuft, Weft, Splicery and
  the trial) found no measurable benefit over ordinary tools with good notes.
  None of them tested continuity.
- None of the continuity verbs were reachable from the `writfold` binary.
- An independent review (Fable 5.2, 10 November 2026) recommended this
  mission, Git as the store, and freezing the rest. Its points adopted and
  declined are listed below.

## Decisions

### Storage

- Git text is the source of truth. One record per file, so concurrent writers
  merge in Git.
- A local SQLite file is a derived index that can always be rebuilt from Git
  at a commit. It holds records, typed edges with provenance and freshness,
  and full-text search (FTS5 with BM25 and a trigram tokenizer for
  typo-tolerant matching). A vector-similarity table may be added later in
  the same index, never as a source of truth.
- Dolt and Omnigraph are paused. Their code, documentation, tests, the TLC
  model of publication and the DINT02 slices stay as they are. This reverses
  the 9 November interim-backend decision. Fuzzy search no longer waits on
  Omnigraph.

### Frozen (working demos, no new development)

- The edit trial flow (`writfold trial`), including `serve`'s trial methods.
- Weft and Splicery, in their own repositories.
- Tuft's query language at v0. Its output contract (budgeted rows, coverage
  trailer, resumable cursor) carries forward into every read verb.
- `writfold.agent/1` and the cross-repository contract pins.
- Dolt publication: the open items (signed candidate binding, the
  same-request rival gap) stay open and are not pursued.
- Known limitation, not fixed while frozen: `trial init` writes the operator
  signing key into the state directory, readable by the agent's own shell.

### Assurance kept and dropped

Kept:
- Aeneas proofs over small pure kernels that ship, where they earn it.
- One TLC model per real protocol with crash points.
- Property tests, kill and fault-injection tests, and the "removing this
  check fails a test" discipline.
- `scripts/gate quality` and `scripts/gate unit` on main before pushing.
- Honest status language and the website's "must not say" list.

Dropped as live obligations:
- the WF00–WF31 gates, the V/M entries, E01–E12 and the 148 scenarios;
- Verus, TLAPS, three-host conformance and the legacy claims map;
- a `qualification/status.json` entry and a receipt-index commit per
  component. New work records gate receipts in its commit message instead.

### Interface

1. Claude Code plugin: a SessionStart brief, PreCompact and Stop checkpoint
   drafts, and a short skill teaching the verbs.
2. The `writfold kb` CLI, which works on a bare machine.
3. A thin MCP server over the same verbs, for other harnesses.

### Record states instead of signatures

Records carry proposed, accepted and superseded states, with the accepting
identity taken from the Git commit. Signing returns only if a real
multi-party need appears.

## Roadmap

The v1 design is [kb-v1-design.md](kb-v1-design.md).

1. ~~Read an existing project knowledge base~~ (M1): `brief`, `context`, `why`, `stale`.
2. ~~The index, search and graph verbs~~ (M2): `find`, `match`, `status`, `impact`, `uses`, `timeline`.
3. ~~Writes~~ (M3): `record`, `checkpoint`, `resume`, `runs`.
4. ~~M4a: the Claude Code plugin~~ (SessionStart brief, checkpoint drafts, one skill; `plugins/writfold`).
5. G1: checks with pinned receipts (below).
6. T1: `writfold watch`, a read-only TUI.
7. G2: offline delivery candidates; G2b: an optional guard hook.
8. A two-week pause: count real catches and waivers from the receipts. If no catch mattered, keep G1 and drop the rest.
9. M4b: MCP, trailer and tracker edges, `--at`. G3 only if the pause justifies it.

## Gates, runs and progress (owner decision, 12 November 2026)

The owner proposed reusable gates, skills and delivery modes. After an independent review, the owner adopted a smaller version, built after M3, with Writfold's own push-to-main flow as its first user.

- **Gate definition:** a small TOML file under the KB repository's `.writfold/` (not `~/.writfold`, which an ephemeral machine lacks) naming a command, an evidence class and a path scope.
- **Receipt:** one JSON record per evaluation in the run directory, pinned to the exact subject (commit or tree, and the paths in scope). It goes stale when its subject moves, using the same machinery as `stale`.
- **Artifact or candidate:** a file or a Git tree. The edit candidate is a plain Git tree in a worktree, judged by a gate.
- **Obligation state** (open, satisfied, stale, failed, waived): derived from definitions and receipts, never stored. `resume` shows it.
- **Evidence classes in G1:** mechanical and observed (commands Writfold runs itself). `gate verify` re-runs the recorded command at the recorded commit; CI doing so is the one check the agent cannot forge.
- **Honest claims:** this resists mistakes, not adversaries. Tamper evidence comes from Git history; a harness hook is a guard, labelled as such. No signing.
- **Runs** are KB runs: they record repo, branch, worktree, base commit and an open or closed state. The worktree is recorded, not created; approval is a record state with the Git identity.
- **Plan assumptions** may use Weft's four-way vocabulary (established, hypothetical, contradicted, unresolved), backed by receipts. Weft itself stays frozen.
- **Not now:** a governed profile, real Slack or PR adapters, effect reconciliation, profile composition and gate groups, authority levels, exception routes, snapshot migration, a Writfold skill format, Weft as an evaluator, a Tuft grammar, Splicery as the edit boundary, signing, and proofs or TLC for gates.

The proposal and review are kept with the session notes; the decision above is the authority.

## Adopted and declined from the review

| Review point | Decision |
|---|---|
| Change the mission to project continuity | Adopted |
| Git as the store, SQLite as a rebuildable index; drop Dolt, shelve Omnigraph | Adopted (paused, not deleted) |
| Freeze trial, Weft, Splicery; Tuft as verbs | Adopted |
| Cut Verus, TLAPS, the gate inventory and receipt ceremony | Adopted |
| Read the existing knowledge-base layout unchanged in v1 | Adopted |
| Derive edges from citations, ID mentions, commit trailers and tracker links | Adopted (milestone 3) |
| Hooks and plugin first, then CLI, then MCP | Adopted |
| Rewrite AGENTS.md and the website around the mission | Adopted |
| Measure continuity before building, with a public evaluation | Declined: benchmarking is paused by the owner; the owner's own use is the acceptance test |
| Move frozen crates to an archive member or branch | Deferred: they stay in the workspace while they build and test cleanly |
