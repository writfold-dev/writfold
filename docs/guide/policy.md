# Your policy

> **Status:** built in v0.6.0-dev.3: `kb init`, the research-policy example, the "no policy configured" line, `kb policy`, a gate's `accept`, a checklist's `not_applicable` and `none_found` with the `none-found` outcome, `unskippable` required steps, policy pins and drift, and the plan's summary word. What held already (required steps, evidence bound to its run, the research guard recipe, approval by a person) is described as built. `waivable` on a delivery mode, a mode gate's `accept` holding for a delivery, `kb init --example delivery`, and `kb policy`'s mode and route rows are built in v0.6.0-dev.4. `plan repin` is built in v0.6.0-dev.6. `kb policy --paths` is built in v0.6.0-dev.11.

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

`writfold kb init [DIR] [--example research-policy|delivery] [--dry-run]` is the one `kb` verb that needs no KB. It creates `inventory/assets/.gitkeep`, the directory every verb looks for, and nothing else. Every file is create-new: a path that exists is listed `kept  PATH (exists; not changed)` and left alone; `init` never overwrites, never commits, and writes no project, record or run. `--dry-run` prints the same rows and writes nothing. Exit 0 when every file was written or kept; 3 when `DIR` is inside another KB (`refused: <dir> is inside the KB at <root>; run init there, or pass --kb`); 1 for a directory that cannot be written. With `--json`, one row per path with `status` `wrote` or `kept`.

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
| `waivable` | a delivery mode | which of its built-in checks and `attest` names a waiver may set aside (absent: all; `[]`: none); a mode gate's own `accept` decides for the gate | [delivery modes](delivery-modes.md#what-a-mode-takes-waivable-and-a-mode-gates-accept) |
| `audience`, `rendering` | a delivery mode | the audience the mode is for; whether citation tokens are stripped from what is posted | [delivery modes](delivery-modes.md#a-mode-file) |

`writfold kb init --example delivery` writes an example delivery policy to edit: two deny lists, a `commit-message` and a `release-note` mode that take no waiver, the `commit-format` gate with `accept = ["satisfied"]`, guard routes, and a plan whose comment step is held until the post is recorded ([delivery modes](delivery-modes.md#an-example-to-start-from-kb-init---example-delivery)). `kb lint` warns (`policy.waivable`) when a mode lists `references.denied` or a mechanical gate and still takes a waiver of it.

Each run pins the policy its plan was set under; a later edit is shown (`!policy-moved`, `policy=moved`, exit 2), and a moved run never reads `complete`. The run either has the definitions restored or adopts the policy as it is now, on the record, with `writfold kb plan repin --reason TEXT --by WHO` ([run-graph](run-graph.md#adopting-a-change-plan-repin)); a repinned run reads at best `complete-with-exceptions`. Every plan says how it ended: `complete`, `complete-with-exceptions`, `complete-stale`, `policy-moved`, `incomplete`, `closed-incomplete`.

## Seeing the policy: `kb policy`

```text
$ writfold kb policy
required required start=research end= unskippable=research @.writfold/plans/required.toml:1
gate research checklist accept=attested items=3 (precedents: supported|none-found; approach: supported; backfill: supported|not-applicable) scope=docs/research/** @000910a8d958 @.writfold/gates/research.toml:1
  read: instructions @.writfold/gates/research.md:1
  pin: moved (pinned @c4a42a7b289b, now @000910a8d958)
# coverage=complete records=7/7 shown=2 omitted=0 gates=1 templates=1 guards=0 modes=0 policy=moved identity=sha256:9253ee90… …
```

(Run after the research instructions were edited while the research step was open; restoring them gives `pin: same @runs/run-…/policy/pin-1-….json:1` and `policy=same`, and so does `plan repin`, against the pin it writes.)

Read-only; no command or checker runs. Each gate row shows its class, `accept`, its items' choices and its scope, and, when a run is current, a `pin:` line: `same`, or `moved (pinned @…, now @…)`. Guard rules follow, each `(a guard, not enforcement)`, then (built in v0.6.0-dev.4) one `mode` row per delivery mode, as [`deliver modes`](delivery-modes.md#what-a-mode-asks-for-deliver-modes) prints it, and one `route` row per guard route (`mode=<mode> body=… file=…` or `field=…`). The `modes=` count is unchanged. `identity=` is the SHA-256 over the sorted `<gate>\t<definition digest>` lines and the required file's digest: one value two clones can compare. With nothing configured: `answer=unknown reason=no-policy`, exit 2, and the no-policy note. Exit 2 also while the current run's policy is moved.

`writfold kb policy --paths GATE` (built in v0.6.0-dev.11) lists the paths that gate's `scope` covers at the subject's present state, one `path` row each: committed files, dirty and untracked ones in scope, deleted ones gone, then each `repo:` store's as `<store>:<path>`. It reads what `gate status` reads to count `paths=N`. The note says what it is: the declared scope; Writfold does not infer what else the check depends on. A gate with no path scope (a run-record or candidate gate) is refused; a store that cannot be read is named in a note and the answer is partial.

```text
$ writfold kb policy --paths unit
path src/ledger.rs
path src/retry.rs
# note: gate unit scope=src/**,!src/batch.rs in demo-service: the declared scope; Writfold does not infer what else the check depends on
# coverage=complete … paths=2 …
```

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

- **Approval by a person, and its limit.** A guard rule with `action = "ask"` on `writfold kb gate attest`, `gate waive`, `plan skip` and `plan repin` (or the harness's own permission rule, [hooks](hooks.md#guard-rules)) makes the harness ask a person before the statement is written; the statement is then bound to the definition and bytes it was made about. Writfold has no record that the harness asked or who answered: `--by` is text, recorded as given. A policy that needs an authenticated approval is not supported. For deliveries, the same holds for `deliver waive` and `deliver withdraw` (built in v0.6.0-dev.4); the delivery example's `guard.toml` carries the permission rule for `.claude/settings.json` in a comment:

  ```json
  {"permissions": {"ask": ["Bash(writfold kb deliver waive:*)", "Bash(writfold kb deliver withdraw:*)"]}}
  ```

## What each mechanism establishes

| | Established by |
|---|---|
| A reference exists | checked: the citation, receipt or observation resolves |
| It is fresh | checked: re-grounded on every read, stale when its bytes moved |
| It is relevant | the judgement of whoever covered or attested, recorded as theirs; `gate status` notes a look recorded for another item and a reference shared across items; a checklist may require that an item's evidence be its own (`own_evidence`) or come from named gates (`evidence_from`), checked mechanically, which still says nothing of what the evidence shows |
| The conclusion follows | not determined |
