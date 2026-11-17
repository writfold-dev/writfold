# Your policy

> **Status:** built in v0.6.0-dev.3: `kb init`, the research-policy example, the "no policy configured" line, `kb policy`, a gate's `accept`, a checklist's `not_applicable` and `none_found` with the `none-found` outcome, `unskippable` required steps, policy pins and drift, and the plan's summary word. What held already (required steps, evidence bound to its run, the research guard recipe, approval by a person) is described as built. `plan repin`, `kb policy --paths` and `waivable` on a delivery mode are planned.

A team writes down what adequate research, implementation and delivery mean for it: gates, checklists, instructions, plan templates, required steps. Writfold loads those files, shows what is owed, judges the evidence by the mechanism the team chose, and refuses a transition its own verbs control when the policy is not met. It holds no opinion of its own about adequacy, and nothing in this page is a rule Writfold adds.

## From a plain repository to a first resumed run

```text
$ writfold kb init
wrote inventory/assets/.gitkeep
# note: next: writfold kb checkpoint --new-run --project NAME --task TEXT --note TEXT
# note: written to the work tree; commit it to share it
# coverage=complete wrote=1 kept=0 dry_run=false
$ writfold kb checkpoint --new-run --project demo --task "try it" --note "first note"
$ writfold kb resume
run run-… [open] try it @runs/run-…/manifest.json:8 !current
…
# note: policy: none configured (continuity only: checkpoints, records and citations work without one)
```

`writfold kb init [DIR] [--example research-policy] [--dry-run]` is the one `kb` verb that needs no KB. It creates `inventory/assets/.gitkeep`, the directory every verb looks for, and nothing else. Every file is create-new: a path that exists is listed `kept  PATH (exists; not changed)` and left alone; `init` never overwrites, never commits, and writes no project, record or run. `--dry-run` prints the same rows and writes nothing. Exit 0 when every file was written or kept; 3 when `DIR` is inside another KB (`refused: <dir> is inside the KB at <root>; run init there, or pass --kb`); 1 for a directory that cannot be written. With `--json`, one row per path with `status` `wrote` or `kept`.

Every other verb, run outside a KB, now names the fix: `no KB found: pass --kb DIR, set WRITFOLD_KB, run inside a KB, or create one here: writfold kb init`.

**No policy.** When neither the KB root nor the repository in view holds a gate, plan template, required file, guard rule or delivery file (a `.toml` under `.writfold/gates`, `plans`, `guards` or `deliveries`), `resume` adds one note, `policy: none configured (continuity only: checkpoints, records and citations work without one)`, and the session-start context carries the same line once. Checkpoints, records, citations and `resume` need no policy.

## An example to start from: `--example research-policy`

`writfold kb init --example research-policy` also writes four files to the work tree. They are an example: yours to edit or delete, and Writfold reads only what they say.

- `.writfold/gates/research.toml`: a checklist gate with three items (`precedents`, `approach`, `backfill`) over `docs/research/**`, naming its instructions; only `backfill` may be covered `not-applicable`, `precedents` may be covered `none-found` (citing a look that found nothing), and the gate takes no waiver (`accept = ["attested"]`).
- `.writfold/gates/research.md`: the instructions, one heading per question the team answers for itself (which sources count; what makes a precedent comparable; reuse, extend, extract or keep it local, with no option preferred; what a search that finds nothing records; what belongs in the KB afterwards and when "nothing new" is acceptable; how open questions are kept), each with a short starting answer and "Change this.".
- `.writfold/plans/research.toml`: `steps = ["research: research"]`.
- `.writfold/plans/required.toml`: `start = ["research"]`, so every plan set in the KB begins with the research step, with `unskippable = ["research"]` present but commented out.

The same bytes are in [`fixtures/kb/research-policy`](../../fixtures/kb/research-policy/.writfold).

## Same engine, two rubrics

A second team writes its research policy another way, in [`fixtures/kb/research-policy-script`](../../fixtures/kb/research-policy-script/.writfold): a mechanical gate `research-doc` whose script, shipped beside the definition, checks that some `docs/research/*.md` has `Question`, `Found` and `Decision` headings, and an attested gate `research-reviewed` whose instructions say what a person reads the note for.

```toml
schema = "writfold-gate/1"
id = "research-doc"
version = 1
class = "mechanical"
argv = ["sh", "{gate}/research-doc.sh"]
timeout_seconds = 60
scope = ["docs/research/**"]
description = "docs/research/ holds a note with Question, Found and Decision headings"
```

Both run through one binary, each to completion under its own rules (`crates/writfold-kb/tests/research_rubrics.rs`): a plan set from a file that names no research step gets the research step first, the advance is refused while the team's own evidence is missing, and the plan completes once it is there. The same test reads every product source file and fails if one names either team's items or gates.

## What the policy may say

Every key is opt-in: a definition without it means what it meant before, and keeps its digest.

| Key | Where | Says | Guide |
|---|---|---|---|
| `accept` | a gate | which standings meet it: `satisfied`, `attested`, `waived` | [gates](gates.md#which-standings-meet-a-gate-accept) |
| `not_applicable`, `none_found` | a checklist | which items take `not-applicable`, which take `none-found` | [gates](gates.md#checklist-gates) |
| `unskippable` | `required.toml` | which required templates' steps `plan skip` refuses | [run-graph](run-graph.md#policy-unskippable-steps-pins-and-how-a-plan-ended) |

Each run pins the policy its plan was set under; a later edit is shown (`!policy-moved`, `policy=moved`, exit 2), and a moved run never reads `complete`. Every plan says how it ended: `complete`, `complete-with-exceptions`, `complete-stale`, `policy-moved`, `incomplete`, `closed-incomplete`.

## Seeing the policy: `kb policy`

```text
$ writfold kb policy
required required start=research end= unskippable=research @.writfold/plans/required.toml:1
gate research checklist accept=attested items=3 (precedents: supported|none-found; approach: supported; backfill: supported|not-applicable) scope=docs/research/** @000910a8d958 @.writfold/gates/research.toml:1
  read: instructions @.writfold/gates/research.md:1
  pin: moved (pinned @c4a42a7b289b, now @000910a8d958)
# coverage=complete records=7/7 shown=2 omitted=0 gates=1 templates=1 guards=0 modes=0 policy=moved identity=sha256:9253ee90… …
```

(Run after the research instructions were edited while the research step was open; restoring them gives `pin: same @runs/run-…/policy/pin-1-….json:1` and `policy=same`.)

Read-only; no command or checker runs. Each gate row shows its class, `accept`, its items' choices and its scope, and, when a run is current, a `pin:` line: `same`, or `moved (pinned @…, now @…)`. Guard rules follow, each `(a guard, not enforcement)`. `identity=` is the SHA-256 over the sorted `<gate>\t<definition digest>` lines and the required file's digest: one value two clones can compare. With nothing configured: `answer=unknown reason=no-policy`, exit 2, and the no-policy note. Exit 2 also while the current run's policy is moved.

## What already holds

- **A plan cannot leave out a required template.** `plan set --from mine.toml`, whose file names no research step, still gives `research#1/research [current]` first: required start and end templates are spliced around any plan ([run-graph](run-graph.md#required-steps)).
- **Evidence never crosses runs.** Receipts, statements and cover rows are read from the run's own directory; a waiver in one run leaves the gate `open` in the next (`no receipt in this run`), and a copied receipt reads `[invalid]`. A done delivery step is judged only on its bound candidate.
- **Research holds source edits, not the research itself.** A guard rule with `tools = ["Edit"]`, `paths = ["src/**"]` and `require = ["run.current", "gate:research"]` denies an edit to `src/a.rs` while the research gate is owed and passes an edit to `docs/research/n.md`, so the research document can be written ([hooks](hooks.md#paths)).

```toml
schema = "writfold-guard-rule/1"
id = "source-after-research"
event = "pre-tool-use"
tools = ["Edit", "Write"]
paths = ["src/**"]
require = ["run.current", "gate:research"]
action = "deny"
description = "edit source only once the research gate is met"
```

- **Approval by a person, and its limit.** A guard rule with `action = "ask"` on `writfold kb gate attest`, `gate waive` and `plan skip` (or the harness's own permission rule, [hooks](hooks.md#guard-rules)) makes the harness ask a person before the statement is written; the statement is then bound to the definition and bytes it was made about. Writfold has no record that the harness asked or who answered: `--by` is text, recorded as given. A policy that needs an authenticated approval is not supported.

## What each mechanism establishes

| | Established by |
|---|---|
| A reference exists | checked: the citation, receipt or observation resolves |
| It is fresh | checked: re-grounded on every read, stale when its bytes moved |
| It is relevant | the judgement of whoever covered or attested, recorded as theirs |
| The conclusion follows | not determined |
