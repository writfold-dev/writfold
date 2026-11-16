# Writfold mission: project continuity for agents

Owner decision, 11 November 2026. This document is the current authority for
what Writfold builds next. It supersedes the earlier delivery order and gate
programme as live obligations; those records are kept, unedited, in the
private archive (see "The public/archive split" below).

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

### Component status (15 November 2026)

The current standing of each component. Earlier sentences below that say otherwise are marked superseded in place; the decisions they record are unchanged.

| Component | Status |
|---|---|
| Tuft | frozen (owner decision, 15 November 2026). The `writfold tuft` subcommand and its adapter were archived on 16 November 2026; the `tuft` crate stays in the workspace only for the citation cursor `writfold-kb` uses. Retrieval receipts that earlier runs recorded are still read |
| Weft, Splicery | frozen (15 November 2026): kept building and tested through their adapters (the Weft assessed gate, `kb edit apply`), and no new development. The `writfold kb` verb groups cover each stage of the flow; the two are revisited only if a use case emerges. They are no longer presented on the website |
| The trial flow and `serve`, `writfold.agent/1`, Dolt publication, Omnigraph | archived (16 November 2026): moved to the private archive and removed from the binary and the workspace |

### The public/archive split (owner decision, 16 November 2026)

The public repository holds the product: the `kb` verbs, `watch`, the crates
they use, their proofs and models, and the guides. The frozen components
that the binary no longer runs (the trial flow, `serve`, `writfold tuft`,
`writfold.agent/1`, Dolt, Omnigraph) and their models, evidence, pilots and
history moved to the private archive repository `writfold-dev/writfold-archive`:
the files with their history under `public-split-2026-11-16/`, and the exact
last tree, buildable, as branch `public-main-pre-split` (tag
`pre-split-2026-11-16` here). The public history is unchanged.

### Storage

- Git text is the source of truth. One record per file, so concurrent writers
  merge in Git.
- A local SQLite file is a derived index that can always be rebuilt from Git
  at a commit. It holds records, typed edges with provenance and freshness,
  and full-text search (FTS5 with BM25 and a trigram tokenizer for
  typo-tolerant matching). A vector-similarity table may be added later in
  the same index, never as a source of truth.
- Dolt and Omnigraph are archived (16 November 2026; paused since 11
  November). Their code, documentation, tests, the TLC model of publication
  and the DINT02 slices are in the private archive. This reverses the 9
  November interim-backend decision. Fuzzy search no longer waits on
  Omnigraph.
- Why they were not resumed (owner, 12 November 2026): both were only ever
  projections of the Git records, so their versioning and branching
  duplicated what Git already provides. The index needs no history of its
  own: it is rebuilt from Git at any commit, and SQLite gives the queries,
  full-text and typo-tolerant search it needs in one file inside the binary.

### Available tools, not a priority for expansion (owner, 12 November 2026)

Tuft, Weft and Splicery are available to the agent as alternatives within the
flow, maintained and kept building, but not a priority for new language
features. (Superseded for Tuft: see Component status.) The agent chooses per task: Tuft or `kb find` for retrieval,
Splicery recipes or ordinary Edit/sed for edits, Weft for plan verification.
Common edit or query patterns may be added later if real use shows the need.

### Archived (16 November 2026; frozen since 11 November)

- The edit trial flow (`writfold trial`) and `serve`.
- `writfold.agent/1` and the cross-repository contract pins.
- Dolt publication: its open items (signed candidate binding, the
  same-request rival gap) were not pursued.

### Assurance kept and dropped

Kept:
- Aeneas proofs over small pure kernels that ship, where they earn it.
- One TLC model per real protocol with crash points.
- Property tests, kill and fault-injection tests, and the "removing this
  check fails a test" discipline.
- `scripts/gate quality` and `scripts/gate unit` on main before pushing.
- Honest status language and the website's "must not say" list.

Dropped as live obligations (their records are in the private archive):
- the WF00–WF31 gates, the V/M entries, E01–E12 and the 148 scenarios;
- Verus, TLAPS, three-host conformance and the legacy claims map;
- a per-component status entry and a receipt-index commit. New work records
  gate receipts in its commit message instead.

### Interface

1. Claude Code plugin: a SessionStart brief, PreCompact and Stop checkpoint
   drafts, and a short skill teaching the verbs.
2. The `writfold kb` CLI, which works on a bare machine.
3. ~~A thin MCP server~~: deferred (12 November 2026), then dropped (15 November 2026): the CLI is the interface.

### Record states instead of signatures

Records carry proposed, accepted, applied, verified and superseded states, with the accepting
identity taken from the Git commit. Signing returns only if a real
multi-party need appears.

## Roadmap

The v1 design is [kb-v1-design.md](kb-v1-design.md).

1. ~~Read an existing project knowledge base~~ (M1): `brief`, `context`, `why`, `stale`.
2. ~~The index, search and graph verbs~~ (M2): `find`, `match`, `status`, `impact`, `uses`, `timeline`.
3. ~~Writes~~ (M3): `record`, `checkpoint`, `resume`, `runs`.
4. ~~M4a: the Claude Code plugin~~ (SessionStart brief, checkpoint drafts, one skill; `plugins/writfold`).
5. ~~Tuft adapter~~: `tuft run` rows could carry KB citation tokens, and a search was recorded as a retrieval receipt in the current run. (Archived 16 November 2026; existing retrieval receipts are still read.)
6. ~~G1: checks with pinned receipts~~ (below; `gate run`, `gate status`, `gate verify`, `gate lines`; [as built](kb-v1-design.md#16-g1-as-built)).
7. ~~Splicery adapter~~: a recipe applies directly in a run's recorded worktree; the result is an ordinary Git change judged by a gate, with the recipe and preview kept as a receipt (`edit apply`, `edit recover`; [as built](kb-v1-design.md#17-splicery-adapter-as-built)).
8. ~~T1: `writfold watch`, a read-only TUI~~ ([as built](kb-v1-design.md#18-t1-as-built)).
9. ~~G2: offline delivery candidates; G2b: an optional guard hook~~ (`deliver prepare|check|attest|waive|status`, `gate attest|waive`, `hook pre-tool-use`; [as built](kb-v1-design.md#19-g2-as-built)). Then, from the order of work below, ~~instruction gates and user-defined delivery modes~~ (`instructions`, `{gate}`, `.writfold/deliveries/<mode>.toml`; [as built](kb-v1-design.md#20-instruction-gates-and-delivery-modes-as-built)).
10. A two-week pause: count real catches and waivers from the receipts. If no catch mattered, keep G1 and drop the rest.
11. M4b: trailer and tracker edges, `--at`. G3 only if the pause justifies it. (MCP was dropped on 15 November 2026: see Interface.)
12. ~~T2: run progress in `writfold watch`~~: plan steps, owed gates, checklist items, guards, inbox and timeline; responsive layouts, subject/queue polling and six themes; read-only and never starting a checker ([as built](kb-v1-design.md#34-t2-as-built)).
13. ~~T3: the plan map and node inspector in `writfold watch`~~ ([as built](kb-v1-design.md#35-t3-as-built)): the recorded spine, nested splices, per-step state glyphs and gate chips, persistent node selection, and an inspector with evidence and checklist segments. Read-only; the marker is the recorded position, never a liveness claim.
14. ~~T5: the knowledge base as a second mode of `writfold watch`~~ ([as built](kb-v1-design.md#37-t5-as-built)): `m` shows a project's decisions, status, tracker, impact tree and stale citations with the run mode's look, a record inspector, and `f` between a run and the records its checkpoints name; built on `find --kind` and the JSON-only `edge` of `impact` rows. Read-only.

**v0.5.0-dev.1** (13 November 2026): enforcement and evidence that a team configures, not code it writes.

- ~~Guard rules and the Stop nudge~~ (`.writfold/guards/<id>.toml`; [as built](kb-v1-design.md#30-guard-rules-and-the-stop-nudge-as-built)): a rule can refuse a named command, such as `git commit`, until the run's gates are met. The rule reads gates; it is a guard, not enforcement.
- ~~Checklist gates and `gate cover`~~ (items covered with grounded evidence or a reason, re-grounded on every read; [as built](kb-v1-design.md#31-checklist-gates-and-gate-cover-as-built-v050-dev1)).
- ~~Kind declarations (a default lifetime, `immutable`, required fields per citation kind), the stale-citation count at session start, and the documented `--json` envelope~~ ([as built](kb-v1-design.md#29-kind-declarations-the-stale-count-at-start-and-the-json-envelope-v050-dev1)).
- ~~Concluding an investigation~~ once its write-up is complete ([record states](../guide/feeding-back.md#record-states)).

Considered for this release and deferred, with the reason:

- ~~**External source checkers**~~ (a program Writfold runs to check a citation kind it cannot check from Git, such as a file in another source-control system): built in v0.5.0-dev.2 as phase 1 ([as built](kb-v1-design.md#33-external-checkers-phase-1-as-built-v050-dev2); [guide](../guide/checkers.md)). The concern that kept it back is answered by running checkers in `kb stale` only: every other reader uses the cached verdict, and an absent or failing checker leaves its tokens `unverifiable`, never fresh. Not built: built-in kinds through the protocol, fetching, render and redaction, a server mode, an SDK, token migration, a checker with a TTL, tracked check receipts and a flag requiring checkers. (Clarified 15 November 2026: "a checker with a TTL" means a checked kind that also ages by `ttl=` or `default_ttl`, which is still not built; `max_age` limits how old a cached verdict may be, not how old the token is.)
- ~~**A separate observation record**~~ (what was looked at, with what result, including failed lookups) as evidence for checklist items: built in v0.5.0-dev.3 in reduced form as `kb observe` (below).
- ~~**Writing a decision's `accepted`, `applied` and `verified` states.**~~ Built in v0.5.0-dev.2: `accepted` records the reviewed design revision, `applied` a commit on the target ref, `verified` a passing gate receipt of the run ([as built](kb-v1-design.md#32-decision-states-as-built-v050-dev2)).
- ~~**Following a rename with edits** for code citations.~~ Built in v0.5.0-dev.2 ([as built](kb-v1-design.md#62-algorithm-for-a-code-citation)).
- **Not planned:** user-declared record lifecycles, origin-URL matching on guard rules (path globs were built in v0.5.0-dev.3), per-audience render and redaction templates for citations, fetching cited repositories, and bulk inventory import (a loop over `record` reviewed as a Git diff does this).

**v0.5.0-dev.2** (13 November 2026): three of the deferred items above are built (external checkers as phase 1, the later decision states, following a rename with edits). Checker phase 2 (tracked check receipts readable on a machine without the checker, a flag that requires checkers, an explicit refresh that proposes new tokens as a diff) waits on real use of phase 1. (15 November 2026: ~~tracked check receipts~~ built in v0.5.0-dev.9 as `kb stale --record`, [as built](kb-v1-design.md#38-check-receipts-as-built-v050-dev9); a flag that requires checkers and an explicit refresh still wait.)

**v0.5.0-dev.3** (13 November 2026): an observation record built in reduced form (`kb observe`: one lookup with its status, in `runs/<run>/observed/`; only an `ok` look with a fresh citation supports a checklist item; no derivation chains and no independent-origins policy); `kb repos` to list cited repositories and their clones, with partial clones and per-repository Git failures degraded to `unverifiable`; tracked proposals; guard `paths` globs (moved out of "Not planned") and per-turn stop rules; checker `max_age`; batch `gate cover`. Still not planned: fetching cited repositories (`kb repos --missing --json` feeds a user's own clone script instead), handing the built-in `code` kind to a checker, render and redaction, reading other tools' observation files, and a checker SDK (the checkers guide has a complete template).

**v0.6.0-dev.1** (16 November 2026): guards that match exactly the command they name and judge the repository it acts on, and lookups that hit on the first try ([dev10-spec.md](dev10-spec.md); [as built](kb-v1-design.md#40-v060-dev1-as-built)). Built: the shared command reader (here-document bodies are data; wrappers, a path to the binary and `git`'s global options are seen through; `--help` passes; each matched command judged where it acts, an unresolvable directory taking the rule's action; a linked worktree named after its repository); `gates.met` over an empty set refuses; guard conformance cases; `normalized`, `prefix` and `heading` lookup tiers with `matched_on`, suggestions on an empty answer and `find --eval`; `mode = "audit"`, waivers shown in the guard's answers, and the local guard log with `guard log`; `runs --idle`; `kb delta` with marks and the session's mark. Three behaviour changes for existing users: a rule requiring `gates.met` in a repository that owes no gate now refuses (leave it out with `repos`); `commands` rules now match through `git -C` and wrappers; here-document bodies are no longer matched. Not built, with the reasons in the specification: a `warn` mode, a rule-level waiver, skipping an unresolvable directory, closing runs automatically, `delta --since <timestamp>`, semantic search, rank fusion, aliases on Markdown records and learned aliases. In the same release, ~~delivery modes as a step's gates~~ ([deliver-gate-spec.md](deliver-gate-spec.md); [as built](kb-v1-design.md#42-delivery-modes-as-a-steps-gates-as-built)): a step owes `deliver:<mode>` and cannot be advanced until a candidate of the mode prepared in the run is ready; the done record binds that candidate. Two more behaviour changes: a candidate whose cited evidence moved after its check reads `[blocked]` (in `deliver status`, the delivery guard and the plan alike), and `delivery.json` gains `seq`. Nothing is sent: a done delivery step says a candidate was ready, never that it was posted.

## Gates, runs and progress (owner decision, 12 November 2026)

The owner proposed reusable gates, skills and delivery modes. After an independent review, the owner adopted a smaller version, built after M3, with Writfold's own push-to-main flow as its first user.

- **Gate definition:** a small TOML file under the KB repository's `.writfold/` (not `~/.writfold`, which an ephemeral machine lacks) naming a command, an evidence class and a path scope.
- **Receipt:** one JSON record per evaluation in the run directory, pinned to the exact subject (commit or tree, and the paths in scope). It goes stale when its subject moves, using the same machinery as `stale`.
- **Artifact or candidate:** a file or a Git tree. The edit candidate is a plain Git tree in a worktree, judged by a gate.
- **Obligation state** (open, satisfied, stale, failed, waived): derived from definitions and receipts, never stored. `resume` shows it.
- **Evidence classes in G1:** mechanical and observed (commands Writfold runs itself). `gate verify` re-runs the recorded command at the recorded commit; CI doing so is the one check the agent cannot forge.
- **Honest claims:** this resists mistakes, not adversaries. Tamper evidence comes from Git history; a harness hook is a guard, labelled as such. No signing.
- **Runs** are KB runs: they record repo, branch, worktree, base commit and an open or closed state. The worktree is recorded, not created; approval is a record state with the Git identity.
- **Plan assumptions** may use Weft's four-way vocabulary (established, hypothetical, contradicted, unresolved), backed by receipts. (Superseded: Weft verifies plans; see below.)
- **Not now:** a governed profile, real Slack or PR adapters, effect reconciliation, profile composition and gate groups, authority levels, exception routes, snapshot migration, a Writfold skill format, Weft beyond the assessed gate and plan checks, a Tuft grammar, Splicery as the edit boundary, signing, and proofs or TLC for gates.

The proposal and review are kept with the session notes; the decision above is the authority.

### Instruction gates, delivery modes, plans and Weft (owner decision, 12 November 2026)

The owner's working flow: gates are atomic checks (instructions in `.md`, optionally with scripts); skills and delivery modes (commit message, PR, Slack) consume gates; evidence recorded by Writfold meets a gate and lets the agent advance to the next step of its plan. After an independent review, the owner adopted, ahead of the two-week pause:

1. **Instruction gates and user-defined delivery modes.** A gate may name an `instructions` file whose digest is part of its definition, and ship its own scripts (a `{gate}` placeholder; `{candidate}` for delivery checks). Any `.writfold/deliveries/<name>.toml` defines a mode listing its gates; the two built-in modes stay as defaults; a `commit-message` mode needs no code of its own.
2. **Plan and advance in the KB run.** `plan set` copies a plan into the run; `plan status` shows the current step; `plan advance` refuses until the step's gates are satisfied, attested or waived, and records which receipt or statement met each one. Steps are derived from records, never stored as state. Gates outside the plan stop being owed by runs that never reach them.
3. **Plan templates** under `.writfold/plans/`, which a skill names in one line of its text. Writfold does not parse `SKILL.md`.
4. **~~Weft verifies plans~~ (owner decision, against the review's advice; the gate is [as built](kb-v1-design.md#23-weft-assessed-gate-as-built)).** A plan step may require a Weft assessment. The agent submits a Weft file stating the step's assumptions and checks; each assumption may cite a Writfold receipt or citation. Writfold resolves those citations (fresh, satisfied, or not), runs the pinned `weft` binary, and records an assessed receipt. A check counts as met only when Weft entails it from assumptions Writfold has grounded; a check that rests on an ungrounded assumption is reported as hypothetical and does not meet the gate. Weft's own vocabulary (entailed, refuted, undetermined, inconsistent; basis hypothetical or grounded) is kept as Weft's.

The agent submits drafts, statements and plans; Writfold writes every receipt and every verdict. The trust model is unchanged: cooperating and single-user; receipts resist mistakes, not adversaries.

### The run graph, hooks and order of work (owner decision, 12 November 2026)

After a fourth independent review, the owner adopted:

- **The run graph is the form plan/advance takes.** Writfold records the graph and the agent walks it; Writfold does not schedule. v1 is one ordered spine of steps, each owing gates. Templates live in `.writfold/plans/` (steps written as `id: gates…`, optional `use=` includes). Loading a skill splices its template's steps before the current step as its children, recording the template snapshot by digest so later edits cannot rewrite the run; a splice is idempotent on (parent, template, digest, ordinal), `--again` adds another. Node identity is a path; the current node is the first neither done nor skipped; a done node whose gate no longer holds shows `!stale` and never moves the current node back. A changed template shows `!template-moved`; there is no migration. No branching or general DAG in v1. Weft verification is one gate kind on a node; the advance decision only sees met or not met.
- **Hooks stay model-free.** Session start shows the plan position, an inbox (drafts, proposals, sessions that ended without evaluation) and whether the KB clone is behind its remote, from local refs only; after compaction it re-injects the plan position. Evaluation is a `writfold:evaluate` skill and a template's last wrap-up step, not a hook that runs a model. Proposals are `record` calls not yet made, kept in a local queue (`kb proposals list|show|accept|drop`), accepted through the normal write path.
- **No MCP server** (owner decision, 15 November 2026; earlier: deferred). The CLI and the Claude Code plugin are the interface; any harness with a shell uses the CLI.
- **Proved and modelled later:** `writfold_core::plan` (current, advance check, splice) and the existing gate standing kernels move into the Aeneas-translated set; `spec/RunGraph.tla` models two sessions, merges, drift and crashes. Receipt ordering must not depend on a wall clock where it decides state.
- **Order of work:** (1) these records, the site slimmed with the full skeletons moved to `docs/guide/*.md`; (2) ~~instruction gates and user-defined delivery modes~~ ([as built](kb-v1-design.md#20-instruction-gates-and-delivery-modes-as-built)); (3) ~~the linear run graph (`plan set|status|advance`)~~ ([as built](kb-v1-design.md#21-run-graph-linear-as-built)); (4) ~~splicing~~ ([as built](kb-v1-design.md#24-splicing-and-required-steps-as-built), with the required steps); (5) ~~deterministic hooks~~ ([as built](kb-v1-design.md#25-deterministic-hooks-as-built)); (6) one week of real use; then ~~proofs~~ ([as built](aeneas.md#the-run-graph-and-gate-kernels-13-november-2026); done ahead of its place), ~~`RunGraph.tla`~~ ([as built](kb-v1-design.md#27-model-specrungraphtla); two gaps found, fixed for v0.4.0-dev.1), ~~`kb check` and proposals~~ ([as built](kb-v1-design.md#22-proposals-and-kb-check-as-built)), ~~the evaluation skill~~ ([as built](kb-v1-design.md#26-evaluation-skill-as-built), with the closing `reflect` step), and ~~the Weft assessed gate~~ ([as built](kb-v1-design.md#23-weft-assessed-gate-as-built); built ahead of its place, the gate only: naming it on a plan step waits for the run graph).

### Required steps and run-record checks (owner, 12 November 2026)

Writfold is a record keeper: it never schedules the agent or its subagents, and does not care how a step was achieved, only what the records show. The owner's "enforcement points" are expressed with gates, not harness hooks:

- **Required steps:** a template under `.writfold/plans/` that every run includes is spliced in automatically at the start or end of each run, like any other step (for example a closing `reflect` step).
- **Run-record checks:** a mechanical gate kind evaluated over the run's own records rather than code. Examples: `research.backfilled` (the step wrote at least one KB record, citation or retrieval receipt) and `run.reflected` (the run holds a reflection entry, and either a proposal or an explicit "no proposal"). A run-record check may be paired with an attested gate whose instructions say what a good reflection or backfill covers.

So "every research step backfills the KB" and "every run ends with self-reflection" are required steps whose gates are run-record checks. The word "hook" is kept only for Claude Code hooks.

### Narrow adapters for Tuft and Splicery (owner decision, 12 November 2026; the Tuft adapter archived 16 November 2026)

Splicery is not a priority for new language features (Tuft is, since 13 November 2026: see "Tuft expansion" below), but each gains one narrow adapter so agents have two interfaces and can choose by task: Splicery for repeated or structural edits, ordinary tools for small ones; `kb` verbs or Tuft for retrieval. The Tuft adapter emits KB citations and run receipts. The Splicery adapter applies in a run's worktree without the trial's signing and approval flow, which stays frozen. Weft contributes only its four-way vocabulary for plan assumptions; making it read KB citations as premises waits for the two-week pause. (Superseded: see Component status.)

### Tuft expansion (owner decision, 13 November 2026; superseded: frozen 15 November, the adapter archived 16 November 2026)

Tuft is taken off the "not a priority" list. Retrieval is where an agent spends most of its reads, and Tuft's value is the citation and the coverage line, not token counts: a search that says it was complete, and rows that arrive as KB citations. The expansion makes that cheap to reach: refs accepted where a person would type them (resolved to a commit and echoed), a one-line form for the common search, one program over several repositories of the KB's repo map, symbol-aware finds with honest partial coverage, diagnostics that name the fix, and a documented, pinned `--json` and exit codes. Stored programs and receipts stay pinned to full commit IDs; the binary stays static and network-free. Weft and Splicery stay as the section above says.

**As built (Tuft expansion):** one-shot stdin and `grep` resolve refs and echo
the full commits; files/check/`--pinned` remain strict. `grep` lowers to the
ordinary language, with glob scopes and printable pinned programs. Repeated
`--repo NAME@REF` uses the KB map, with per-repository and overall coverage and
pinned receipts. Diagnostics collect recoverable parse errors and name fixes;
`check --json`, the result JSON and exit codes have contract tests. Worktrees
and submodule checkouts are accepted. Working-tree search labels uncommitted
bytes dirty and never retains them as pinned evidence. `find def|refs` imports
universal-ctags JSON with a verified commit/digest sidecar and always-partial
coverage, without new crates. Not included: SCIP, tree-sitter, enclosing-block
windows (not exact without language-specific parsing), parameters, grouping,
set operations and a recipes page. (The language and CLI reference moved
to the private archive with the adapter on 16 November 2026.)
`writfold tuft index` (v0.5.0-dev.9) builds the ctags index and its sidecar
from the bytes of one commit, exported under the Tuft state directory, with
universal-ctags found on `PATH` or `$WRITFOLD_CTAGS`; the user's work tree,
index and worktree list are untouched.

## Adopted and declined from the review

| Review point | Decision |
|---|---|
| Change the mission to project continuity | Adopted |
| Git as the store, SQLite as a rebuildable index; drop Dolt, shelve Omnigraph | Adopted (paused, then archived on 16 November 2026) |
| Freeze trial, Weft, Splicery; Tuft as verbs | Adopted (Superseded: see Component status) |
| Cut Verus, TLAPS, the gate inventory and receipt ceremony | Adopted |
| Read the existing knowledge-base layout unchanged in v1 | Adopted |
| Derive edges from citations, ID mentions, commit trailers and tracker links | Adopted (milestone 3) |
| Hooks and plugin first, then CLI, then MCP | Adopted |
| Rewrite AGENTS.md and the website around the mission | Adopted |
| Measure continuity before building, with a public evaluation | Declined: benchmarking is paused by the owner; the owner's own use is the acceptance test |
| Move frozen crates to an archive member or branch | Deferred, then done on 16 November 2026: the crates the binary no longer runs moved to the private archive; Weft's and Splicery's adapters stay |
