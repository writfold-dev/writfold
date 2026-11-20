---
rfc: "2026-11-16-user-defined-policy"
title: "User-defined policy: accepted outcomes, unskippable steps, pins and the first-use path"
status: accepted
implementation: partial
authors:
  - tknawara
created: 2026-11-16
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: User-defined policy: accepted outcomes, unskippable steps, pins and the first-use path

> **Status:** built through the cut line in v0.6.0-dev.3, as recorded in [design §44](../writfold/kb-v1-design.md#44-user-defined-policy-as-built-v060-dev3) (deviations there); `plan repin` is built in v0.6.0-dev.6 ([run-graph](../guide/run-graph.md#adopting-a-change-plan-repin); its two decisions are `writfold_core::policy::repin`, proved in the repin round of [aeneas.md](../writfold/aeneas.md); as built it also records, under `steps`, a done step's own digest where it differs from the gate's, and refuses to adopt an edited pin, a gate that does not load, or a lock list that would reopen a skipped step); `kb policy --paths` is not built. The specification below is kept as written. Written against `main` at `b14505b` (v0.6.0-dev.2 plus the roadmap commit). The authority is [mission.md](../writfold/mission.md). Every description of today's behaviour below was checked against the code at that commit, and the rows marked "run" were also run with the release binary on a copy of `fixtures/kb/basic` with a scratch subject repository ([section 8](#8-what-was-checked)).

A team writes down what adequate research, implementation and delivery mean for it: gates, checklists, instructions, plan templates, required steps. Writfold loads that policy, shows what is owed, judges the evidence by the mechanism the team chose, and refuses a transition its own verbs control when the policy is not met. It holds no opinion of its own about adequacy.

Three routes around a configured obligation are open today, each through an ordinary Writfold verb: any step can be skipped, required ones included; any gate that takes a waiver is met by one; any checklist item is covered by `not-applicable`. This round lets the policy say which of those it accepts, pins the policy a run started under so a later edit is visible, and makes every summary say how a plan was completed, not only that it was. It also adds the shortest path from an ordinary repository to a first resumed run.

Two rules shape the design.

- **Recorded history keeps its meaning.** No key added here changes what a file already written says. A policy without the new keys behaves exactly as today; a run started before this release is labelled `unpinned` and judged as today.
- **Decision logic is proved before it is wired** (owner, 16 November 2026). Every decision added or changed here is a pure function in `writfold-core`, translated with Charon and Aeneas and proved in Lean ([section 3](#3-kernels-and-theorems)); the build plan has the proofs gate as a checkpoint before any verb calls the new functions ([section 5](#5-build-plan)).

The trust model is unchanged: cooperating and single-user. An agent with a shell can edit a policy file, write a record by hand or act outside Writfold; Git history shows it and nothing prevents it. "Closing a route" below always means a route through Writfold's own verbs.

## 1. Triage

| # | Need | Verdict | Reason |
|---|---|---|---|
| 1 | Policy authored once: TOML for obligations, Markdown for instructions, bound into one digest; portable in Git; duplicates refused; skills and delivery modes name gate IDs | **Already does** | `gate::def::digest` covers every field but the description, the instructions' bytes and every `{gate}/` script. Definitions live under `.writfold/` in the KB root or the repository; the same ID in both is refused (`gate/discover.rs`). A mode lists gate IDs; a skill names a template in one line. No new language, no Markdown front matter |
| 2 | A research policy the user writes, with a different team able to write another, and no engine change between them | **Already does for the mechanism; adopt two fixtures and a test** | Checklist, attested, mechanical and run-record gates express it today (run: the three-item research checklist loads and is owed). What is missing is a proof by example: two rubrics over one binary |
| 3 | A minimal first-use path | **Adopt: `kb init`** | Run: in a plain repository every verb answers `no KB found`; the fix is `mkdir -p inventory/assets`, which no message names. Nothing else is needed for a first checkpoint and resume |
| 4 | A starter research policy the user adopts or edits | **Adopt: `kb init --example research-policy`**, files written to the work tree and labelled an example | It must be files in Git, never engine behaviour. The binary carries the bytes; after `init` they are the user's |
| 5 | An agent's plan cannot omit a required template | **Already does** | Run: `plan set --from mine.toml`, whose file names no research step, gives `research#1/research [current]` first. `plan/required.rs::order` splices start and end templates around any plan |
| 6 | A required step cannot be skipped when the policy says so | **Adopt: `unskippable` in `required.toml`** | Run: `plan skip --reason "no time" --by owner` closes the required research step and the plan then reads `complete`. A skip is recorded honestly, but nothing lets a policy refuse it |
| 7 | A policy can refuse waivers for an obligation | **Adopt: `accept` on a gate; `waivable` on a delivery mode** | Run: a failing mechanical gate, `gate waive`, then `plan advance` records the step done. `Standing::met` counts a waiver for every class that takes one. The guide lists "per-gate standing sets" as not built |
| 8 | A policy can refuse `not-applicable` for a checklist item | **Adopt: `not_applicable` on a checklist** | Run: all three required items covered `not-applicable` with one-word reasons; the gate stands `attested` and the step advances |
| 9 | A bounded "looked, found nothing" result can be recorded without claiming nothing exists | **Adopt: a fourth cover outcome, `none-found`, accepted only for items the policy names** | Today the only words are `supported` (needs grounded evidence, which an empty search is not), `not-applicable` (the wrong word) and `unsupported` (never covers). Observations with status `no-data` or `not-found` already record the look |
| 10 | "No new reusable finding" as an accepted backfill result | **Adopt an alternative: a checklist item with `not_applicable` or `none_found` allowed; decline extending `research.backfilled`** | A built-in run-record requirement with more branches would be Writfold deciding what a backfill is. The checklist item carries the team's wording and the reason is in the row |
| 11 | Editing instructions, a scope or a gate mid-run does not silently make the run acceptable | **Already does for evidence; adopt policy pins for visibility** | Run: after an instruction edit the done step reads `[done] !stale`, each row `the definition changed since this row`; a removed gate reads `[undefined]` and refuses the advance. What is missing: once the evidence is redone under the edited definition, nothing says the policy changed under the run |
| 12 | An explicit policy-update path | **Adopt: `plan repin --reason --by`**, below the cut line | Editing the file already invalidates the evidence. The pin adds the record that the run adopted the new policy on purpose |
| 13 | Summaries distinguish passed, attested, waived, skipped, stale and unconfigured | **Already does per gate and per step; adopt for the plan as a whole** | Run: `resume` says `plan complete [complete] every step is done or skipped (2 steps)` for a plan with one skipped step and one step done on a waiver. Rows are honest; the one-line summary is not specific |
| 14 | A closed run with open obligations stays visibly incomplete, with its reason | **Already does for the plan position; adopt the word `closed-incomplete` and fix the dropped note** | Run: a closed run still lists its current step and owed gates. `status RUN --set closed --note TEXT` accepts the note and writes nothing of it (`writfold-run.json` gains only `closed`) |
| 15 | Evidence from another run or candidate never discharges this obligation | **Already does (tested); adopt moving the binding into the proved kernel** | Run: a waiver in one run leaves the gate `open` in the next (`no receipt in this run`). Receipts are read from the run's own directory, and a copied one reads `[invalid]`. A done delivery step is judged only on its bound candidate. The binding is decided in the adapter today |
| 16 | A changed delivery draft is re-judged | **Already does** | Receipts bind the candidate's bytes; `claims.cited-fresh` re-checks cited tokens on every read (v0.6.0-dev.1) |
| 17 | `--by` text is never read as an authenticated person | **Already does; document the supported approval route and its limit** | Run: `--by "Human Owner (really)"` is recorded as given. No output infers identity; [assurance](../guide/assurance.md) lists it under "Trusted, not checked" |
| 18 | Guards name what they cover; research gates block the move into implementation without blocking the research itself | **Already does** | Run: a rule with `tools = ["Edit"]`, `paths = ["src/**"]`, `require = ["run.current", "gate:research"]` denies an edit to `src/a.rs` and passes an edit to `docs/research/n.md`. It still denies after the research step is skipped, since the plan's gate stays owed |
| 19 | An invalid definition never yields a passing decision | **Already does for gates, plans, modes and required steps; decline changing the guard-rule case; see [4.1](#41-a-broken-guard-rule-that-denies)** | Run: an undefined or unreadable gate on a step is `[undefined]`, not met. A guard rule that does not parse is skipped by the hook (owner decision, 16 November 2026), reported at session start, by `kb lint` (exit 2) and by `guard status` (exit 2) |
| 20 | Scopes can be inspected | **Adopt: `kb policy --paths GATE`**, below the cut line | `gate status` prints `paths=N` and a digest; the paths themselves are not listed anywhere |
| 21 | A project with no policy says so | **Adopt: one line in `resume`, the session-start context and `kb policy`** | `gate status` already answers `no gates defined`, exit 2. `resume` says nothing about it |
| 22 | One command that creates a run and sets its plan | **Defer; see [4.2](#42-an-atomic-run-start)** | Two idempotent commands do it today, and the state between them is not weaker |
| 23 | A built-in or model-based judge of research quality | **Decline** | Adequacy is the team's. A team that wants a model's opinion runs it as a mechanical gate's `argv`; its receipt says the command exited 0, nothing more |
| 24 | Website wording for "deterministic" and "done, and still true" | **Copy specified in [section 6](#6-website-copy-release-step); a decision for the owner ([7.1](#71-the-word-deterministic-on-the-website))** | The story stays; the copy says what is deterministic |
| 25 | A sandbox, authenticated identity, signing | **Decline** | Outside the cooperative model, and against the mission's "no signing" |

## 2. Adopted items

### 2.1 `kb init` (items 3, 4, 21)

```text
writfold kb init [DIR] [--example research-policy] [--dry-run]
```

`DIR` defaults to the working directory. `init` is the one `kb` verb that needs no KB.

- **Without `--example`** it creates `inventory/assets/.gitkeep` and nothing else. That is the whole of a KB: the directory is the marker every verb looks for.
- **With `--example research-policy`** it also writes the four files of [2.7](#27-the-research-policy-example-items-2-4-10): `.writfold/gates/research.toml`, `.writfold/gates/research.md`, `.writfold/plans/research.toml` and `.writfold/plans/required.toml`.
- Every file is create-new. A path that exists is left alone and listed `kept`; `init` never overwrites, never commits and writes no project name, asset or record.
- `--dry-run` prints the same rows and writes nothing.

```text
$ writfold kb init --example research-policy
wrote inventory/assets/.gitkeep
wrote .writfold/gates/research.toml
wrote .writfold/gates/research.md
wrote .writfold/plans/research.toml
kept  .writfold/plans/required.toml (exists; not changed)
# note: the research policy is an example: these files are yours to edit or delete, and Writfold reads only what they say
# note: next: writfold kb checkpoint --new-run --project NAME --task TEXT --note TEXT
# note: written to the work tree; commit it to share it
```

Exit 0 when at least one file was written or all were kept; 3 when `DIR` is inside an existing KB other than `DIR` itself (`refused: <dir> is inside the KB at <root>; run init there, or pass --kb`); 1 for a directory that cannot be written. JSON: the usual envelope, one row per path with `status` `wrote` or `kept`.

The message for no KB gains the fix: `no KB found: pass --kb DIR, set WRITFOLD_KB, run inside a KB, or create one here: writfold kb init`.

**No policy configured.** When the KB root and the subject repository hold no gate, plan template, required file, guard rule or delivery file, `resume` adds `policy: none configured (continuity only: checkpoints, records and citations work without one)`, and the session-start context adds the same line once. It is one fact, read from directory listings the verbs already make.

`init` holds no decision logic; nothing in it is a kernel.

### 2.2 Accepted outcomes on a gate: `accept` (item 7)

```toml
schema = "writfold-gate/1"
id = "unit"
version = 2
class = "mechanical"
argv = ["cargo", "test", "--locked"]
timeout_seconds = 600
scope = ["src/**", "Cargo.toml"]
accept = ["satisfied"]             # a waiver does not meet this gate
```

`accept` is an optional list of the standings that meet the gate: any of `satisfied`, `attested`, `waived`.

| Class | Standings it can reach | Absent `accept` means | What `accept` may say |
|---|---|---|---|
| `mechanical`, `observed`, `run-record` | `satisfied`, `waived` | both | `["satisfied"]` or both |
| `attested`, `checklist` | `attested`, `waived` | both | `["attested"]` or both |
| `assessed` | `satisfied` | `satisfied` | `["satisfied"]` |

Faults, reported by the definition reader with a line and column like every other ([errors](../guide/errors.md)): a word that is not a standing (`value.unknown`, with the nearest word); a standing the class cannot reach (`value.class`: `a mechanical gate is never attested`); a list without the class's own standing (`value.unmeetable`: `accept must include "satisfied": a mechanical gate that accepts only a waiver can never pass`); a word twice.

The line `accept=<json>` is added to the definition digest only when the key is present, so every existing definition keeps its digest. Adding or changing `accept` is a definition change like any other: earlier receipts, statements and rows go stale and the checks are run again. That is the explicit adoption: the stricter policy starts with fresh evidence.

**One meaning of met.** Wherever a gate is judged (`gate status`, `gate lines`, `plan advance`, `plan status`, `resume`, `kb check`, the `gates.met`, `gate:<id>` and `step.met` requirements, a mode gate inside a delivery), met is `writfold_core::policy::met(standing, accept)` ([3.1](#31-policymet)). The standing itself is computed and shown as today, so the actual outcome is never hidden.

- `gate waive` on a gate that does not accept one is refused, exit 3, nothing written: `gate unit does not accept a waiver (accept = ["satisfied"] @.writfold/gates/unit.toml:8); meet it: writfold kb gate run unit`.
- A waiver that exists anyway (written by hand, or merged from a clone) is shown and counts for nothing:

```text
gate unit [waived] mechanical fail exit=1 at code@fb084405099f scope=5bb42cc7963f paths=1 @runs/run-…/gates/unit-….json:1 !waived !not-accepted
  waived: flaky here (by owner, 2026-11-16T20:45:09Z) @runs/run-…/statements/unit-….json:1
  why: this gate accepts only: satisfied (@.writfold/gates/unit.toml:8); the waiver is recorded and does not meet it
  next: writfold kb gate run unit
```

- `plan advance` refuses with `owes: unit [waived, not accepted]`. A guard refusal reads `gate unit [waived, not accepted]; next: writfold kb gate run unit`.
- The "Past a guard" table gains one clause: a gate whose `accept` leaves out `waived` has no waiver route; meet it, or change the definition in a reviewed commit.

**Delivery modes.** A mode gate is a gate definition and takes `accept` like any other. For the mode's own checks and `attest` names, a mode file gains one optional key, `waivable = ["claims.cited-fresh"]`: the obligations `deliver waive` may set aside. Absent: all of them, as today; an empty list: none. It names obligations the mode declares (`ref.undefined` otherwise), joins the mode's digest only when present, and is judged through the same `policy::met`: `deliver waive --check references.denied` is then refused (exit 3, `pr-comment does not accept a waiver of references.denied (waivable = […] @.writfold/deliveries/pr-comment.toml:6)`), a waiver that exists anyway shows `[waived] !not-accepted`, and the candidate reads `[blocked]`, for `deliver status`, the delivery guard and a `deliver:<mode>` step alike.

JSON (additive): a gate row gains `"accept": ["satisfied"]` when the key is set and `"accepted": false` when its standing is a met word that the policy does not accept. `gate status` exit codes are unchanged: 0 only when every owed gate is met, now by `policy::met`.

### 2.3 Checklist items: `not_applicable` and `none-found` (items 8, 9, 10)

```toml
schema = "writfold-gate/1"
id = "research"
version = 1
class = "checklist"
scope = ["docs/research/**"]
instructions = "research.md"
items = [
  "precedents: investigate applicable prior work",
  "approach: justify the chosen implementation approach",
  "backfill: preserve findings under our research policy",
]
not_applicable = ["backfill"]      # only this item may be covered not-applicable
none_found = ["precedents"]        # this item may be covered none-found
accept = ["attested"]              # and the gate as a whole takes no waiver
```

- **`not_applicable`**: the items that `not-applicable` covers. Absent: every item, as today. An empty list: none.
- **`none_found`**: the items that `none-found` covers. Absent: none.
- Both name declared items only (`ref.undefined` otherwise, with the nearest ID) and each joins the checklist's canonical lines only when present, so existing checklists keep their digest.

**The new outcome.**

```text
writfold kb gate cover <gate> <item> none-found REF… --reason TEXT --by WHO
```

`none-found` says: these recorded looks found nothing. Each `REF` is an observation of this run ([observations](../guide/gates.md#observations-as-evidence-built-in-v050-dev3)) whose status is `no-data` or `not-found`; at least one is required, and `--reason` states what was searched and where the search stopped. A look that did not answer (`timeout`, `denied`, `error`) or answered in part does not ground it. In a `--from` batch the row is `{"item":"precedents","outcome":"none-found","refs":["<file>"],"reason":"…"}`.

```text
$ writfold kb observe --source code-search --locator "rg 'batch.*ledger' in harbor-api@4be03f7" --status no-data --for research.precedents
$ writfold kb gate cover research precedents none-found runs/run-…/observed/20261116T101502Z-3a9c1e.json \
    --reason "searched harbor-api src/ and tests/ for batching of ledger writes; KB find 'ledger batch' also empty" --by agent:model@session
gate research [open] checklist 1/3 required covered at code@fb084405099f scope=9bb2743118af @.writfold/gates/research.toml:1 !open
  item: precedents [none-found] 1 look found nothing: searched harbor-api src/ and tests/ … by agent:model@session @runs/run-…/covers/research.precedents-….json:1 !none-found
# note: recorded … (precedents none-found: the recorded looks found nothing; not a claim that nothing exists)
```

An item stands on its newest row, as today, with one more row:

| Item | When |
|---|---|
| `none-found` | the row is current, says so, and cites at least one observation of this run that still reads as `no-data` or `not-found` |

A gate's roll-up counts an item covered when it is `supported`; or `not-applicable` and the policy lists it (or has no `not_applicable` key); or `none-found` and the policy lists it. The checklist still stands as `attested`, never `satisfied`.

Refusals, exit 3, nothing written:

| Case | Text |
|---|---|
| `not-applicable` for an item the policy does not list | `research.precedents cannot be covered not-applicable (not_applicable = ["backfill"] @.writfold/gates/research.toml:12); cover it supported with evidence` (and `, or none-found citing a look` when the item is in `none_found`) |
| `none-found` for an item not in `none_found` | `research.approach cannot be covered none-found (none_found = ["precedents"] @…:13)` |
| `none-found` with no reference, or one that is not a `no-data`/`not-found` observation of this run | `none-found needs a look that found nothing: <ref> is <what it is>; record one with writfold kb observe --status no-data` |

A row whose outcome the policy does not accept (hand-written, or merged) keeps its word and does not cover: `item: precedents [not-applicable] … !not-accepted`, and the gate stays `open` with that item first owed.

`gate status`, `plan status --gates` and `resume` print each restricted item's choices once, on the owed item's line: `item: precedents [open] investigate applicable prior work (supported or none-found)`. `gate lines` prints a `none-found` item as `precedents: none-found (1 look): <reason>`.

JSON (additive): an item row's `standing` may be `none-found`; item rows gain `"allowed": ["supported","none-found"]` when the policy restricts them and `"accepted": false` when a current row's outcome is not allowed. Cover rows stay `writfold-cover/1` with one more `outcome` value.

**A whole-gate waiver.** `gate waive research` still sets the checklist aside unless the definition says `accept = ["attested"]`. `kb lint` warns once (`policy.waivable`, a warning, exit unchanged without `--deny-warnings`) when a checklist restricts `not_applicable` and still accepts `waived`, because the restriction is then one command from being moot. The warning states the consequence and the fix; it is not an error, since a team may want exactly that.

### 2.4 Unskippable required steps: `unskippable` (item 6)

```toml
schema = "writfold-plan-required/1"
start = ["research"]
end = ["reflect"]
unskippable = ["research"]         # steps of this required template cannot be closed by plan skip
```

`unskippable` names templates that are also in `start` or `end` (`ref.undefined` otherwise). The steps it locks are the steps the required template itself brings at the root: its own steps and those of its `use=` includes. Sub-steps an agent later splices under one with `plan use` are not locked.

**The lock is the run's, taken when the plan is set.** `plan set` writes the locked templates into the run's pin ([2.5](#25-policy-pins-and-drift-items-11-12)). Afterwards the decision reads the pin, never the live file:

- a run pinned with `research` unskippable keeps the lock when `required.toml` is later weakened (until a `plan repin`, which is on record);
- a run set before the key was added, or by an earlier binary, has no lock: its recorded skips stand as they are, and it may still skip. Its plan is labelled `unpinned`.

`plan skip` on a locked step is refused, exit 3, nothing written:

```text
$ writfold kb plan skip --reason "no time" --by owner
step research#1/research [current] owes: research [open] @runs/run-…/plan/splice-research.1.json:1 !locked
  next: writfold kb gate cover research precedents supported REF… --by WHO
# note: not skipped: research#1/research is a required step this run cannot skip (unskippable = ["research"] @.writfold/plans/required.toml:4, pinned @runs/run-…/policy/pin-1.json:1); nothing written
```

A `skip-` record for a locked step that exists anyway counts for nothing: the fold passes it through `plan::skipped(recorded, locked)` ([3.3](#33-planskip_check-and-planskipped)), the step stays open and current, and `plan status` lists the file as `!invalid: a skip of a step this run cannot skip`.

`plan status` flags locked open steps `!locked`. JSON (additive): step rows gain `"locked": true`.

What the lock does not do: it does not force the step's gates to be strict. A locked step whose gate accepts a waiver is still closed by a waiver, as done on a waiver. A policy that wants the research done says both: `unskippable` here and `accept` on the gate.

**Completing the plan.** `plan advance` and `plan skip` already refuse to close a plan's last open step while a required end template is missing. That check moves into the kernel (`plan::close_check`) and covers every required template named by the pin or by the live file, start templates included.

### 2.5 Policy pins and drift (items 11, 12)

**The pin.** With every splice (`plan set`, `plan use`), in the same write plan and ordered before the splice files, Writfold writes `runs/<run>/policy/pin-<seq>.json`:

```json
{
  "schema": "writfold-policy-pin/1",
  "run": "run-20261116T204509Z-0b1b0e",
  "seq": 1,
  "kind": "set",
  "covers": ["plan/splice-research.1.json", "plan/splice-change.2.json"],
  "required": { "source": ".writfold/plans/required.toml", "digest": "sha256:…", "unskippable": ["research"] },
  "gates": [
    { "gate": "research", "definition": "sha256:61ed6fb7…" },
    { "gate": "unit", "definition": "sha256:27977094…" },
    { "gate": "deliver:pr-comment", "definition": "sha256:…" }
  ],
  "recorded": "2026-11-16T20:45:09Z",
  "digest": "sha256:…"
}
```

- `gates` holds each gate the spliced closure names that no earlier pin of the run names, with its definition digest (which already covers class, scope, `accept`, items, instructions and scripts). A `deliver:<mode>` entry is the SHA-256 of the mode file's bytes (the constant `builtin:<mode>` when there is none) followed by each mode gate's definition digest.
- `required` is written by the first pin only (`"digest": "absent"` when there is no file).
- `kind` is `set`, `use` or `repin`. `seq` is one more than the run's highest. The file is create-new and ends with a digest over the rest, like a plan event; an edited pin reads `!invalid` and the run is then judged `moved`, never `same`.
- The directory is new, so an earlier binary does not read it and folds the plan exactly as it does today.
- A crash between the pin and its splices leaves a pin that covers nothing. Rerunning the same `plan set` reuses it when its entries equal what would be written now, and otherwise refuses naming the file to delete.

**Drift.** On every read, each pinned gate is compared ([3.4](#34-policypin_one-and-policypin_all)):

| Step | Compared with the pin |
|---|---|
| open (current or later) | the gate's definition digest now; a gate that no longer loads counts as different |
| done | the `definition` its done record names (delivery entries are not compared once done; their own `!stale` covers a changed mode) |
| the required file | its digest now, for the first pin |

One difference makes the run's policy `moved`. `plan status` then:

```text
step research#1/research [done] research [attested] @runs/run-…/plan/done-research.1~research.json:1 !policy-moved
  why: met under research@9f3c2a17d0e4; this run pinned research@61ed6fb70938 (@runs/run-…/policy/pin-1.json:1)
step change#2/implement [current] owes: unit [open] @runs/run-…/plan/splice-change.2.json:1 !policy-moved
  why: unit is now @c41d0b7a55e2; this run pinned unit@27977094aa10: evidence is judged under the definition as it is now
# note: policy moved since this run pinned it: 2 gates (research, unit); the plan cannot read complete until the definitions are restored or the run adopts them: writfold kb plan repin --reason TEXT --by WHO
```

- The trailer gains `policy=same|moved|unpinned` and `repinned=N`. `plan status` exits 2 while `policy=moved`; `kb check`'s plan row and `resume`'s plan line carry `!policy-moved`.
- Drift blocks no step. Evidence under a changed definition is already stale, and the present definition is what is judged, as today. What drift changes is the summary ([2.6](#26-how-a-plan-ended-items-13-14)): a moved run never reads `complete`.
- Restoring the definition clears the flag for open steps. A step that was *done* under another definition stays flagged until a repin: that is the case of a policy weakened, used and put back.
- A run with no pin (`unpinned`) is never flagged and is judged as today. Its summary carries the word, and one note says why: `this run's plan was set before policy pins were recorded; its steps are judged under the definitions as they are now`.

**Adopting a change: `plan repin`** (below the cut line).

```text
writfold kb plan repin --reason TEXT --by WHO [--run ID] [--repo DIR]
```

It writes a pin of `kind: "repin"` holding every gate of the plan at its digest now (done steps: the digest their record names), the required file's digest and lock list now, the reason and `by`. Later reads compare with the newest repin. It is refused (exit 3) when nothing moved. It invalidates nothing itself: the evidence went stale when the definition changed. A repinned run is never `complete` without qualification again ([2.6](#26-how-a-plan-ended-items-13-14)), and `plan status` keeps one line: `repin: 1 (research, unit) by owner: adopted the stricter review rubric @runs/run-…/policy/pin-3.json:1`. `--by` is text, as everywhere.

`kb delta` classifies the new files as the run artefact `policy-pin`, and `runs --idle` reads their stamps.

**`kb policy`** shows the policy as it is now, and the current run's pin beside it:

```text
$ writfold kb policy
required start=research end=reflect unskippable=research @.writfold/plans/required.toml:1
gate research checklist accept=attested items=3 (precedents: supported|none-found; approach: supported; backfill: supported|not-applicable) scope=docs/research/** paths=4 @.writfold/gates/research.toml:1
  read: instructions @.writfold/gates/research.md:1
  pin: same @runs/run-…/policy/pin-1.json:1
gate unit mechanical accept=satisfied scope=src/**,Cargo.toml paths=41 @.writfold/gates/unit.toml:1
  pin: moved (pinned @27977094aa10, now @c41d0b7a55e2)
guard edit pre-tool-use Edit paths=src/** requires run.current gate:research (a guard, not enforcement) @.writfold/guards/edit.toml:1
# coverage=complete gates=2 templates=3 guards=1 modes=0 policy=moved identity=sha256:5e0c… …
```

`identity` is the SHA-256 over the sorted `<gate>\t<definition digest>` lines and the required file's digest: one value two clones can compare. With nothing configured: `answer=unknown reason=no-policy`, exit 2, and the note of [2.1](#21-kb-init-items-3-4-21). Read-only; no command or checker is run. Below the cut line: `kb policy --paths GATE` lists the paths in a gate's scope at the subject's present state, one per row, with the note `the declared scope; Writfold does not infer what else the check depends on`.

### 2.6 How a plan ended (items 13, 14)

One word for a run's plan, decided by `policy::summary` ([3.5](#35-policyclose-and-policysummary)) and used wherever a plan is summarised: `plan status` (a final `plan:` row), `resume`'s plan line, `runs --plan`, `kb check`'s plan row, the session-start context and `gate lines`' header when the run has a plan.

| Word | When (first that applies) |
|---|---|
| `no-plan` | the run has no step |
| `incomplete` | a step is open, or a required template is missing, and the run is open |
| `closed-incomplete` | the same, and the run is closed |
| `policy-moved` | every step is closed, and the policy differs from the pin |
| `complete-stale` | every step is closed, and a done step's gate no longer holds |
| `complete-with-exceptions` | every step is closed, and a step was skipped, done on a waiver or done on an attestation, or the run was repinned |
| `complete` | every step is done, each on passing checks only |

A step's own word comes from its done record: `passed` when every gate's recorded standing is `satisfied` (a step with no gate is `passed`: nothing was owed and nothing is claimed), `attested` when any is attested and none waived, `waived` when any is waived, `skipped`.

```text
plan complete-with-exceptions: 2 steps: 0 passed, 0 attested, 1 waived, 1 skipped; policy=same @runs/run-…/plan/
plan closed-incomplete: step 2/2 open (mine#2/implement owes unit [open]); closed 2026-11-16T20:50:05Z: abandoned, superseded by HBR-140
plan complete (unpinned): 4 steps: 4 passed
```

The counts are for reading; the word is the decision. Text that changes, named for the release note: `plan complete [complete] every step is done or skipped (N steps)` becomes the line above, and `runs --plan`'s `plan: complete steps=2 done=1 skipped=1` becomes `plan: complete-with-exceptions steps=2 passed=0 attested=0 waived=1 skipped=1`. JSON (additive): the plan row and the trailer gain `summary`, `passed`, `attested`, `waived`; `done` and `skipped` stay. Exit codes: `policy-moved` and `complete-stale` exit 2; the others as today.

**Closing a run.** `status RUN --set closed --note TEXT` writes the note into `writfold-run.json` as `"closed_note"` (today it is accepted and dropped). Closing stays unconditional: an abandoned run must be closable. When the plan is not complete the answer adds `# note: closed with the plan incomplete: step 2/2 open, owing unit [open]; a closed run is not a completed one`, and when no note was given, `# note: no reason recorded: pass --note TEXT to say why`. `runs` shows `[closed] !incomplete` for such a run. Reopening is unchanged.

### 2.7 The research-policy example (items 2, 4, 10)

The four files `kb init --example research-policy` writes. They use the keys of this release; until those are wired, the fixture uses the existing schema without them.

- `.writfold/gates/research.toml`: the checklist of [2.3](#23-checklist-items-not_applicable-and-none-found-items-8-9-10), with a first comment line `# An example. Edit the items, the choices and research.md to say what adequate research means for you.`
- `.writfold/gates/research.md`: headings that are questions for the team, each with a short starting answer and the sentence "Change this." beneath it: which sources count (the KB first, then code and tests); what makes a precedent comparable; when to reuse, extend, extract or keep a change local, with no option preferred; what a search that finds nothing must record (where it looked, where it stopped) and that it is never a claim that nothing exists; what belongs in the KB afterwards and when "nothing new" is an acceptable answer; how open questions are kept.
- `.writfold/plans/research.toml`: `steps = ["research: research"]`.
- `.writfold/plans/required.toml`: `start = ["research"]`, with `unskippable = ["research"]` present but commented out, and a comment saying what it does.

The same bytes live in `fixtures/kb/research-policy/`. A second fixture, `fixtures/kb/research-policy-script/`, is a different team's rubric over the same binary: a mechanical gate `research-doc` whose shipped script checks that `docs/research/<ticket>.md` exists and has three headings (`accept = ["satisfied"]`), beside an attested gate `research-reviewed` with its own instructions. The guide shows both. Neither is read by the engine: the test of [5](#5-build-plan) step 12 runs both policies through one binary and greps the source for the item IDs to show none is known to it.

What each mechanism establishes, stated once in the guide: a reference exists (checked); it is fresh (checked); it is relevant (the judgement of whoever covered or attested, recorded as theirs); the conclusion follows (not determined).

### 2.8 Approval by a person (item 17)

Nothing is built. The guide states the route that exists and its limit in one place: a guard rule with `action = "ask"` on `writfold kb gate attest`, `gate waive`, `plan skip` and `plan repin` (or the harness's own permission rule) makes the harness ask a person before the statement is written; the statement is then bound to the definition and bytes it was made about. Writfold has no record that the harness asked or who answered. A policy that needs an authenticated approval is not supported, and the guide says so in those words.

## 3. Kernels and theorems

Everything below is pure, IO-free Rust in `crates/writfold-core`, written for translation ([aeneas.md](../writfold/aeneas.md#writing-kernels-for-translation)): owned data, explicit `while` loops, one early-returning loop per function, `matches!` instead of derived equality, no casts. The new module `writfold_core::policy` carries no derives and is translated whole; functions added to `plan`, `gate` and `checklist` are named in `[package.metadata.charon] start_from` one by one. The theorems are total-correctness triples over the translation, in the style of `proofs/Writfold/Gate.lean`; the statements below are the claims the axiom audit must find. Names may be adjusted to the translation's; the content may not be weakened.

### 3.1 `policy::met`

```rust
pub struct Accept { pub satisfied: bool, pub attested: bool, pub waived: bool }
pub fn met(standing: Standing, accept: &Accept) -> bool
```

`proofs/Writfold/Policy.lean`:

```lean
theorem met_spec (x : gate.Standing) (a : policy.Accept) :
    policy.met x a ⦃ b => b = true ↔
      (x = .Checked .Satisfied ∧ a.satisfied = true) ∨
      (x = .Attested ∧ a.attested = true) ∨ (x = .Waived ∧ a.waived = true) ⦄

/-- A policy without `accept` is today's rule. -/
theorem met_default (x : gate.Standing) :
    (do let p ← policy.met x ⟨true, true, true⟩; let g ← gate.Standing.met x; ok (p, g))
      ⦃ r => r.1 = r.2 ⦄

/-- A policy only ever narrows. -/
theorem met_narrows (x : gate.Standing) (a : policy.Accept) :
    (do let p ← policy.met x a; let g ← gate.Standing.met x; ok (p, g))
      ⦃ r => r.1 = true → r.2 = true ⦄

/-- A gate that rejects waivers is never met by one, whatever statements exist. -/
theorem waiver_rejected_never_met (ac : Bool) (s : gate.State) (st : gate.Statements)
    (a : policy.Accept) (h : a.waived = false) :
    (do let x ← gate.standing ac s st; policy.met x a) ⦃ b => b = true →
      if ac then st.attested_current = true ∧ a.attested = true
      else s = .Satisfied ∧ a.satisfied = true ⦄
```

`met_default` is the compatibility claim as a theorem: no existing policy changes meaning.

### 3.2 The checklist roll-up

`checklist::item` gains the `NoneFound` outcome and item; `covered` and `rollup` are replaced by

```rust
pub struct Entry { pub item: Item, pub required: bool, pub na: bool, pub none: bool }
pub fn covered_under(item: Item, na: bool, none: bool) -> bool
pub fn rollup_under(entries: &[Entry]) -> Rollup
```

`item`, `covered_under` and `rollup_under` join the translated set. `proofs/Writfold/Checklist.lean`:

```lean
theorem item_supported_iff (r : Option checklist.Row) :
    checklist.item r ⦃ i => i = .Supported ↔ ∃ row, r = some row ∧ row.current = true ∧
      row.outcome = .Supported ∧ 0 < row.evidence.val ∧ row.grounded = true ⦄
theorem item_none_found_iff (r : Option checklist.Row) :
    checklist.item r ⦃ i => i = .NoneFound ↔ ∃ row, r = some row ∧ row.current = true ∧
      row.outcome = .NoneFound ∧ 0 < row.evidence.val ∧ row.grounded = true ⦄
theorem covered_under_spec (i : checklist.Item) (na none : Bool) :
    checklist.covered_under i na none ⦃ b => b = true ↔
      i = .Supported ∨ (i = .NotApplicable ∧ na = true) ∨ (i = .NoneFound ∧ none = true) ⦄
/-- An item whose policy rejects not-applicable is never covered by it. -/
theorem not_applicable_rejected_never_covers (none : Bool) :
    checklist.covered_under .NotApplicable false none ⦃ b => b = false ⦄
theorem rollup_under_spec (es : Slice checklist.Entry) :
    checklist.rollup_under es ⦃ r =>
      (r.covered = true ↔ ∀ e ∈ es.val, e.required = true → coveredB e = true) ∧
      r.first_owed.map (·.val) = leastFailing (fun e => !e.required || coveredB e) es.val ∧
      (r.stale = true ↔ ∃ e ∈ es.val, e.required = true ∧ e.item = .Stale) ⦄
```

(`coveredB e` is the Lean reading of `covered_under e.item e.na e.none`; `leastFailing` is the helper `Plan.lean` already defines.) The checklist kernel was tested and not proved until now; these are new proofs, and its exhaustive Rust tests stay.

### 3.3 `plan::skip_check` and `plan::skipped`

```rust
pub enum SkipRefusal { Complete, NotCurrent(usize), Locked }
pub fn skip_check(nodes: &[Node], at: usize, locked: bool) -> Result<(), SkipRefusal>
pub fn skipped(recorded: bool, locked: bool) -> bool          // recorded && !locked
pub fn close_check(last_open: bool, present: &[bool]) -> Option<usize>
```

`plan skip` today asks `current` and writes; `skip_check` makes the refusal order a kernel decision. The fold builds each `Node` with `skipped: plan::skipped(a skip record exists, the step is locked by the pin)`. `Node`, `closed`, `current`, `advance_check`, `stale`, `splice` and `splice_check` are unchanged, so every existing theorem of `Plan.lean` stands as proved, untouched. Added:

```lean
theorem skip_check_spec (nodes : Slice plan.Node) (pos : Usize) (locked : Bool) :
    plan.skip_check nodes pos locked ⦃ r => match r with
      | .Ok _ => cur nodes.val = some pos.val ∧ locked = false
      | .Err .Complete => cur nodes.val = none
      | .Err (.NotCurrent now) => cur nodes.val = some now.val ∧ now.val ≠ pos.val
      | .Err .Locked => cur nodes.val = some pos.val ∧ locked = true ⦄
theorem skip_ok_iff (nodes : Slice plan.Node) (pos : Usize) (locked : Bool) :
    plan.skip_check nodes pos locked ⦃ r =>
      r = .Ok () ↔ cur nodes.val = some pos.val ∧ locked = false ⦄
/-- A skip record never closes a locked step. -/
theorem locked_skip_never_closes (recorded : Bool) :
    (do let s ← plan.skipped recorded true; plan.closed ⟨false, s⟩) ⦃ b => b = false ⦄
/-- So a locked step that is not done, with every step before it closed, is the current step. -/
theorem locked_stays_current (nodes : Slice plan.Node) (i : Nat) (hi : i < nodes.val.length)
    (hprefix : ∀ j (hj : j < i), closedB nodes.val[j] = true)
    (hopen : nodes.val[i].done = false) (hlock : nodes.val[i].skipped = false) :
    plan.current nodes ⦃ r => r.map (·.val) = some i ⦄
/-- The plan's last step closes only when every required template is present. -/
theorem close_check_spec (last : Bool) (present : Slice Bool) :
    plan.close_check last present ⦃ r => r.map (·.val) =
      if last then firstOwed present.val else none ⦄
```

"A locked required step never leaves the effective plan" is then the existing `splice_keeps_existing` (a splice only adds, and no kernel operation removes a step) together with `locked_skip_never_closes` and `advance_ok_iff`: the only way a locked step closes is an admitted advance, with every gate met under `policy::met`.

### 3.4 `policy::pin_one` and `policy::pin_all`

```rust
pub enum Pin { Unpinned, Same, Moved }
pub fn pin_one(pinned: &Option<Vec<u8>>, now: &Option<Vec<u8>>) -> Pin
pub fn pin_all(pins: &[Pin]) -> Pin
```

`pin_one`: no pinned digest is `Unpinned`; a pinned digest with nothing now, or with different bytes, is `Moved`; equal bytes are `Same` (through `bytes::same`, so `same_spec` carries it). `pin_all`: `Moved` when any is; else `Unpinned` when the list is empty or any is; else `Same`.

```lean
theorem pin_one_same_iff (p n : Option (alloc.vec.Vec U8)) :
    policy.pin_one p n ⦃ r => r = .Same ↔ ∃ a b, p = some a ∧ n = some b ∧ a.val = b.val ⦄
theorem pin_one_unpinned_iff (p n : Option (alloc.vec.Vec U8)) :
    policy.pin_one p n ⦃ r => r = .Unpinned ↔ p = none ⦄
theorem pin_all_moved_iff (ps : Slice policy.Pin) :
    policy.pin_all ps ⦃ r => r = .Moved ↔ ∃ p ∈ ps.val, p = .Moved ⦄
/-- Same only when something was pinned and all of it is unchanged: an empty pin is not "same". -/
theorem pin_all_same_iff (ps : Slice policy.Pin) :
    policy.pin_all ps ⦃ r => r = .Same ↔ ps.val ≠ [] ∧ ∀ p ∈ ps.val, p = .Same ⦄
```

### 3.5 `policy::close` and `policy::summary`

```rust
pub enum How { Passed, Attested, Waived }                       // one gate's recorded standing in a done record
pub enum Close { Open, Passed, Attested, Waived, Skipped }
pub fn close(done: bool, skipped: bool, met: &[How]) -> Close   // done wins over skipped
pub struct Facts { pub run_closed: bool, pub required_present: bool, pub pin: Pin, pub repinned: bool, pub stale: bool }
pub enum Summary { NoPlan, Incomplete, ClosedIncomplete, PolicyMoved, CompleteStale, CompleteWithExceptions, Complete }
pub fn summary(closes: &[Close], facts: &Facts) -> Summary
```

```lean
/-- A step reads `passed` only when it is done and every gate passed a check:
an attestation or a waiver is never read as a pass. -/
theorem close_passed_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Passed ↔ done = true ∧ ∀ h ∈ met.val, h = .Passed ⦄
theorem close_waived_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Waived ↔ done = true ∧ ∃ h ∈ met.val, h = .Waived ⦄
theorem close_open_iff (done skipped : Bool) (met : Slice policy.How) :
    policy.close done skipped met ⦃ c => c = .Open ↔ done = false ∧ skipped = false ⦄

theorem summary_spec (cs : Slice policy.Close) (f : policy.Facts) :
    policy.summary cs f ⦃ r => SummarySpec cs.val f r ⦄      -- the table of 2.6, row by row, in order
theorem summary_complete_iff (cs : Slice policy.Close) (f : policy.Facts) :
    policy.summary cs f ⦃ r => r = .Complete ↔
      cs.val ≠ [] ∧ (∀ c ∈ cs.val, c = .Passed) ∧ f.required_present = true ∧
      f.pin ≠ .Moved ∧ f.repinned = false ∧ f.stale = false ⦄
/-- A run whose policy differs from its pin never reads complete, with or without exceptions. -/
theorem moved_never_complete (cs : Slice policy.Close) (f : policy.Facts) (h : f.pin = .Moved) :
    policy.summary cs f ⦃ r => r ≠ .Complete ∧ r ≠ .CompleteWithExceptions ∧ r ≠ .CompleteStale ⦄
/-- An open step never reads as any kind of complete, whether or not the run is closed. -/
theorem open_never_complete (cs : Slice policy.Close) (f : policy.Facts)
    (h : ∃ c ∈ cs.val, c = .Open) :
    policy.summary cs f ⦃ r => r = (if f.run_closed then .ClosedIncomplete else .Incomplete) ⦄
/-- A skip, a waiver, an attestation or a repin never reads as plain complete. -/
theorem exception_never_clean (cs : Slice policy.Close) (f : policy.Facts)
    (h : f.repinned = true ∨ ∃ c ∈ cs.val, c ≠ .Passed) :
    policy.summary cs f ⦃ r => r ≠ .Complete ⦄
theorem empty_never_complete (f : policy.Facts) :
    policy.summary ⟨[], by simp⟩ f ⦃ r => r = .NoPlan ⦄
```

### 3.6 Evidence bound to its run and subject, in `gate::state`

Today the adapter drops a receipt that is filed in another run, names another repository or candidate, or fails its digest, before the kernel sees the list. The decision moves in:

```rust
pub struct Evidence { pub outcome: Outcome, pub same_definition: bool, pub same_scope: bool, pub bound: bool }
pub fn bound(same_run: bool, same_subject: bool, intact: bool) -> bool   // all three
pub fn stands(same_definition: bool, same_content: bool, bound: bool) -> bool   // for statements and cover rows
```

`state` treats a receipt that is not `bound` exactly as an absent one: the deciding receipt is the newest that is bound and about the current definition and bytes; with none, the newest bound receipt gives `stale`; with no bound receipt, `open`. Behaviour is unchanged: the adapter passes every receipt it finds with its three facts instead of filtering, and `Statements`' flags are computed through `stands`.

**Re-proved, and how.** `Gate.lean`'s `isCurrent` gains the conjunct `e.bound`, and the fallback of `StateSpec` ranges over bound receipts. `state_loop_spec` and `state_spec` are proved again over the new loop pair (the second loop, `newest_bound`, is its own function so each has one early return); `state_satisfied_iff`, `state_other_subject` and `satisfied_only_by_current_pass` keep their statements, with `isCurrent` as redefined, and their proofs are replayed. `outcome_*`, `standing_*`, `met_spec` and `standing_met_iff` do not mention `Evidence` and are untouched. Added:

```lean
theorem bound_spec (r s i : Bool) : gate.bound r s i ⦃ b => b = (r && s && i) ⦄
/-- Receipts from another run, about another repository or candidate, or with a
broken digest never satisfy, whatever they say. -/
theorem state_unbound_never_satisfies (rs : Slice gate.Evidence)
    (h : ∀ e ∈ rs.val, e.bound = false) :
    gate.state rs ⦃ r => r.1 = .Open ⦄
/-- Unbound receipts change nothing: the state is the state of the bound ones alone. -/
theorem state_ignores_unbound (rs : Slice gate.Evidence) :
    gate.state rs ⦃ r => r.1 = stateOf (rs.val.filter (·.bound)) ⦄
```

This step changes no behaviour. If the proofs of 3.1 to 3.5 run long it moves below the cut line whole; it is never wired half-proved.

### 3.7 Audit and mutants

`proofs/Audit.lean`'s `claims` list gains every theorem named in 3.1 to 3.6 (thirty names; the count in [aeneas.md](../writfold/aeneas.md) and the guide moves from thirty-three). `proofs/mutants.py` gains one mutant per decision, each required to break exactly the named theorem:

| Mutant of the Rust | Must break |
|---|---|
| `policy::met` ignoring `accept.waived` | `Policy.met_spec` |
| `policy::met` returning true for a `Checked(Stale)` standing when all are accepted | `Policy.met_default` |
| `covered_under` ignoring `na` | `Checklist.covered_under_spec` |
| `covered_under` counting `NoneFound` without `none` | `Checklist.covered_under_spec` |
| `item` giving `NoneFound` with no evidence | `Checklist.item_none_found_iff` |
| `rollup_under` ignoring `required` | `Checklist.rollup_under_spec` |
| `skip_check` without the lock check | `Plan.skip_check_spec` |
| `skipped` ignoring `locked` | `Plan.locked_skip_never_closes` |
| `close_check` ignoring `present` | `Plan.close_check_spec` |
| `pin_one` reporting `Same` for a missing current digest | `Policy.pin_one_same_iff` |
| `pin_all` reporting `Same` for an empty list | `Policy.pin_all_same_iff` |
| `close` reading a waived gate as `Passed` | `Policy.close_passed_iff` |
| `summary` skipping the `Moved` row | `Policy.summary_spec` |
| `summary` ignoring `run_closed` | `Policy.summary_spec` |
| `summary` returning `Complete` with a skipped step | `Policy.summary_spec` |
| `state` ignoring `bound` | `Gate.state_loop_spec` |

### 3.8 What stays tested, and why

| Not proved | Why | How it is tested |
|---|---|---|
| Reading `accept`, `not_applicable`, `none_found`, `unskippable`, `waivable` from TOML, and their diagnostics | parsers over strings and spans; the translation takes neither | table tests per fault, golden diagnostics, `guide_toml.rs` over every guide block |
| Writing and reading pin files, their digests, choosing the newest repin by `seq` | file IO, JSON, SHA-256 | round-trip and edited-file tests; kill tests at each write-protocol step; order-independence under rotated directory listings |
| The fold from event and pin files to `Node`, `locked`, `How`, `Close` and `Facts` | it gathers the kernel's inputs from files | each adapter has a test that fails when its mapping is removed or inverted ([section 5](#5-build-plan), mutations) |
| Which steps a required template locks (its own closure, not later splices) | derived from splice records | fixture with a `use=` include and a later `plan use` under a locked step |
| The counts in the summary line | display; the word is the decision | golden files |
| `kb init`, `kb policy`, all text and JSON rendering | no decision | golden files, a refusal test per case |
| Candidate selection for a delivery step (`writfold_core::delivery`) | not changed here | its existing exhaustive tests; a candidate for a later proof round |

`spec/RunGraph.tla` gains the lock: `Skip` is enabled only for an unlocked step, with one new invariant (a locked step is never in the skipped set) and one mutant (the guard dropped) that must violate it. Pins add no step state and are not modelled.

## 4. Push-backs

### 4.1 A broken guard rule that denies

Not adopted. A guard rule file that does not parse is skipped, and the call it would have judged goes through. That was decided on 16 November 2026: a typo must not block all work, and a file that cannot be read does not say which commands it governs. It is not silent: session start tells the agent, `kb lint` and `guard status` exit 2, and the hook writes the fault to stderr on every call. Run: a rule requiring `gates.mett` lets `git commit` through, with `guard rule skipped: .writfold/guards/commit.toml:5:12: unknown requirement "gates.mett"; did you mean "gates.met"?`.

The alternative offered: a team that wants a broken policy to stop work runs `writfold kb lint --deny-warnings` in CI and as a pre-commit check, where exit 2 fails the job. That is a decision the team makes in its own tooling, and it holds on every harness. A narrower engine change is listed for the owner in [7.3](#73-a-rule-whose-match-reads-but-whose-requirement-does-not).

Everything else already fails closed: an unreadable or undefined gate is never met, a plan naming one is refused at `plan set`, a mode that does not load holds its step open, and `gates.met` refuses over an empty set or an invalid definition.

### 4.2 An atomic run start

Deferred. `checkpoint --new-run --project P --task T --note N` then `plan set <template>` are both idempotent, and the state between them is a run without a plan, which owes every gate defined for its repository: more than the plan would, never less. A crash there loses nothing and `plan set` completes it. A combined verb would add a second way to create a run and a recovery case, to save one command. If daily use shows the second command being forgotten, the shape is one more flag, `checkpoint --new-run … --plan <template>|--plan-from FILE`, built as one write plan with the pin first; its answer would be the run row, the plan rows and `policy=same identity=…`, and a refused plan would leave no run. Until then, `resume` and the session-start context say, for a run without a plan in a KB that has `required.toml`: `plan: none set; this KB has required steps: writfold kb plan set <template>`.

### 4.3 More built-in research requirements

Not built. `research.backfilled` shows that something was written; it does not grow branches for "nothing new was found". A team says what counts in a checklist item, with `not_applicable` or `none_found` allowed where it accepts them and the reason in the row. Writfold adds an outcome word with a mechanical precondition (a recorded look that found nothing) and nothing about what a good search is.

### 4.4 A judge of relevance or quality

Not built, in any form. A checklist shows that each required item has a row with grounded references or an accepted reason. Whether the references are relevant is the judgement of whoever wrote the row, recorded as theirs. A team that wants a second opinion from a program, a model included, runs it as a mechanical or observed gate; the receipt records its exit status on those bytes and is labelled with the gate's own description.

### 4.5 Identity, approval, sandboxing

Not built. `--by` stays text. Approval by a person is the harness's prompt ([2.8](#28-approval-by-a-person-item-17)); Writfold does not record that it happened and does not claim it. A guarantee that needs control of every effect an agent can cause is outside what a record keeper with a guard hook can give, and the guides keep saying so.

### 4.6 Blocking on drift

Not built. A moved policy stops the plan from reading complete and exits 2; it does not refuse `plan advance`. Evidence under a changed definition is already stale, so an advance is refused for that reason where it matters, and a team that pulls a stricter gate from `main` mid-run is not locked out of its own work. A team that wants a hard stop requires it with a guard rule over a gate, as for anything else.

## 5. Build plan

One implementer, one worktree. Decision logic goes kernel, extraction, proofs, then wiring; nothing in `writfold-kb` calls a new kernel function before step 6 has passed. Sizes are working days. Constraints throughout: no new crates; `writfold-app` modules at most 250 lines and functions at most 80, tests included; no `unwrap` or `expect`; no `#[ignore]`; Tuft, Weft and Splicery untouched; an attested or waived outcome is never worded as passed.

| Step | Work | Size |
|---|---|---|
| 1 | `kb init` without the example; the no-KB message; the "no policy configured" line. The two research fixtures in today's schema, the guide section showing both, and the "same engine, two rubrics" test over them. No decision logic | 1 |
| 2 | Documentation of what already holds: required templates survive any plan, evidence never crosses runs, the guard recipe that holds source edits until research is met, approval by a person and its limit, what each mechanism establishes | 0.5 |
| 3 | **Kernel.** `writfold_core::policy` (`met`, `pin_one`, `pin_all`, `close`, `summary`); `plan::skip_check`, `skipped`, `close_check`; `checklist` `NoneFound`, `covered_under`, `rollup_under`; `gate` `bound`, `stands` and the `state` change. Exhaustive Rust tests for each, in the style of the existing kernel tests. `writfold-kb` keeps its behaviour: the checklist functions being replaced stay until step 8, and the adapter sets `bound: true` on the receipts it already filters until step 7 passes the three facts through | 1.5 |
| 4 | **Extraction.** `start_from` entries; `sh proofs/refresh.sh`; the regenerated `proofs/Extracted/` committed; any rewrite the translation needs, with the kernel tests unchanged | 0.5 |
| 5 | **Proofs.** `Policy.lean`, `Checklist.lean`, the additions to `Plan.lean`, the re-proof of `Gate.lean`'s state theorems; `Audit.lean`'s claims; the sixteen mutants | 3 |
| 6 | **Checkpoint: the proofs gate passes.** `docker compose -f compose.proofs.yaml run --build --rm proofs`: drift check, `lake build`, axiom audit, mutant calibration, ending `proofs gate: OK`. Where Docker cannot create a network, build the image's `gate` target and run it with `docker run --rm --network none`, as earlier rounds did. The result (theorem count, mutant count, time) is recorded in the commit message. `spec/RunGraph.tla` re-run with the lock invariant and its mutant | 0.5 |
| 7 | Wiring `accept`: the definition key and its diagnostics, the digest line, `policy::met` at every judging site, the `gate waive` refusal, `!not-accepted`, guard texts; `waivable` on a mode and the `deliver waive` refusal; the receipt, statement and row facts of 3.6 passed to the kernel instead of filtered | 1.5 |
| 8 | Wiring the checklist: `not_applicable`, `none_found`, the `none-found` outcome in `gate cover` and `--from`, grounding by a `no-data`/`not-found` observation, item lines, `gate lines`, the `policy.waivable` lint warning | 1.5 |
| 9 | Wiring `unskippable` and pins: the `required.toml` key; pin files written with each splice; `locked` in the fold; `plan skip` through `skip_check`; `close_check`; drift flags, `policy=` in trailers, exit 2 | 2 |
| 10 | Wiring the summary: `close` and `summary` behind `plan status`, `resume`, `runs --plan`, `kb check`, session start, `gate lines`; `closed_note`; `[closed] !incomplete` | 1 |
| 11 | `kb policy` (rows, pin state, identity); the example and fixtures moved to the new keys; `init --example` | 1 |
| 12 | Guides (`gates`, `run-graph`, `hooks`, `feeding-back`, `delivery-modes`, `errors`, `assurance`, `validation`, `json`, `watch` for the new words), `aeneas.md`, the as-built section of `kb-v1-design.md`, the scenario tests of the table below that are not yet written | 1.5 |
| | **Cut line: steps 1–12** | **about 16** |
| 13 | `plan repin` | 1 |
| 14 | `kb policy --paths GATE` | 0.5 |
| 15 | The gate-binding kernel wiring of 3.6, if it was moved below the line at step 5 | 1 |

Steps 13 to 15 are independent; each ships when complete and gated or moves to the next release whole. Without step 13 a moved run stays `policy-moved` until its definitions are restored, which is true and usable. If step 5 runs long, 3.6 leaves the kernel commit whole (step 3's `gate` change is reverted, not left unproved) and returns as step 15.

Release steps as usual: version and guide status lines; `scripts/gate quality` and `scripts/gate unit` from main with receipt paths in the commit message; the proofs gate result beside them; website copy only for what was exercised ([section 6](#6-website-copy-release-step)). The release note names the behaviour changes: the plan summary words, exit 2 for `policy-moved`, the closing note now written, and that every new key is opt-in.

**Before calling it done:** one real run on the owner's own research policy, showing one refusal that mattered and one accepted exception (a `none-found` or an allowed `not-applicable`), or a plain statement that no refusal has mattered yet. Then the mission's pause for daily use applies; the next additions come from what that use shows.

### Tests, with the mutation each one catches

| Test | Mutation it must fail under |
|---|---|
| `accept = ["satisfied"]`: `gate waive` refused; a hand-written waiver shows `[waived] !not-accepted`; `plan advance`, `gates.met` and `gate:<id>` all refuse | any judging site calling `Standing::met` instead of `policy::met`; `gate waive` not reading `accept` |
| A definition without `accept` has the digest it had in v0.6.0-dev.2 (pinned literal), and every existing golden file is unchanged | the digest line written unconditionally |
| `accept` faults: unknown word, unreachable standing, unmeetable list | each check removed in the reader |
| `not_applicable = ["backfill"]`: cover of `precedents` refused; a merged row shows `!not-accepted` and the gate stays open | `gate cover` not reading the list; the judge passing `na: true` |
| `none-found`: refused without a reference, with an `ok` or `timeout` look, with another run's look; accepted with a `no-data` look and listed; never printed as `supported` | the status check widened; the run check removed; the word mapped to `supported` in any renderer |
| `unskippable`: `plan skip` refused on the locked step and allowed on the next; a hand-written skip record leaves the step current and is listed `!invalid`; a sub-step spliced under the locked step can be skipped | the fold not passing `locked`; `skip_check` bypassed; the lock applied to later splices |
| A run set before `unskippable` was added keeps skipping, and reads `unpinned` | the lock read from the live file |
| Pin written before its splices; a kill at each write-protocol step leaves either no plan, or a plan with its pin | the op order reversed |
| Instruction edit, scope edit, gate removed, `accept` removed: each gives `policy=moved`, the two digests, exit 2; restoring clears it for open steps and not for a step done meanwhile | the done record's digest not compared; a missing gate read as same |
| Summary words: each row of 2.6 from a fixture; a waived and a skipped step never give `complete`; a closed run with an open step gives `closed-incomplete` | each adapter fact inverted or defaulted |
| `status --set closed --note` writes `closed_note`; without a plan complete the note is printed | the note dropped again |
| `kb init`: create-new only, `kept` for an existing file, `--dry-run` writes nothing, refused inside another KB | overwrite on exists |
| Two rubrics, one binary: both fixtures run to completion under their own rules; the source holds none of their item IDs | an item ID special-cased |
| A fresh clone of a KB with a pinned, half-done run gives the same `plan status`, `kb policy` identity and summary | any read depending on untracked state |

### The behaviour scenarios

| Scenario | Today (v0.6.0-dev.2) | After |
|---|---|---|
| The same engine under two user rubrics | Passes (mechanism); no test | Tested (step 1) |
| A plan that omits required research | **Passes** (run): the template is spliced first, also with `--from FILE` | Unchanged; test kept |
| Research evidence absent or stale | **Passes** (run): advance refused, exit 3, the owed item and its command named; `the definition changed since this row` | Unchanged |
| A bounded no-match result, where the policy permits it | Fails: only `not-applicable` covers, the wrong word | `none-found` (2.3) |
| `not-applicable` rejected for an item | Fails (run): it covers any item | `not_applicable` (2.3) |
| Waivers or skipping rejected | Fails (run): both close the step | `accept` (2.2), `unskippable` (2.4) |
| Instructions or scope changed mid-run | **Passes** for evidence (run): stale, never silently met. Not visible once redone | Pins (2.5) |
| An invalid configured rule | **Passes** for gates, plans, modes (never met). A broken guard rule is skipped, loudly | Unchanged ([4.1](#41-a-broken-guard-rule-that-denies)) |
| A human identity declared in text | **Passes** (run): recorded as given, nothing inferred | Unchanged; one test that no output words it as approval |
| A gate result from another run or candidate | **Passes** (run; tested) | Unchanged in behaviour; proved (3.6) |
| A delivery draft changed after its check | **Passes** (tested: `plan_deliver*.rs`) | Unchanged |
| A run abandoned with open gates | **Passes** in the rows (run); the summary has no word for it and the reason is dropped | `closed-incomplete`, `closed_note` (2.6) |
| A read, or a write that produces evidence, while a phase is pending | **Passes** (run): the research document edits while `src/` is held | Unchanged; the recipe documented |
| A fresh clone resumes a run | **Passes** (tested: a Git clone answers the same) | Extended to pins |
| A crash during a combined run and plan write | Not applicable: there is no combined write; each of the two is kill-tested | Pin and splices in one write plan, kill-tested |
| A shell or hook route Writfold does not see | **Passes**: documented in [hooks](../guide/hooks.md) and [assurance](../guide/assurance.md); no claim of enforcement | Unchanged |

## 6. Website copy (release step)

Copy only; the pages change with the release. Every line describes behaviour that is built at v0.6.0-dev.2 unless marked.

**Hero (recommended, option A: the story kept, the word defined).**

> # Deterministic agent work that resumes where it stopped.
>
> **Deterministic.** Not the model: the process around it. The agent follows a plan; each check leaves a receipt tied to the exact bytes it checked; and what is done, what is owed and whether a step may close are computed from files in Git, the same on every machine.
>
> **Resumable.** A new session reads a short brief and the last checkpoint, not the whole project. Commit and push the run's files, and it resumes on any machine.

**The section heading** stays "A plan the agent walks, with checks that leave receipts", with one sentence added beneath: "What the agent writes is its own; what Writfold says about it is derived from the records, never from a model."

**The legend under the `watch` capture.** "✓ Done, and still true" becomes:

> `✓` **Done, and its checks still hold.** A step is recorded done only when every gate it owes is met. A gate met by a statement or a waiver is shown as attested or waived, never as passed, and a step whose checked bytes have since changed is flagged stale.

**Summaries** (after this release): one line under the legend, "A finished plan says how it finished: `complete`, `complete-with-exceptions`, `policy-moved`."

**A short end-to-end example**, ahead of the configuration vocabulary on `how.html`, six commands with measured output: record a decision and a checkpoint; pass a gate; stop; change a file in the gate's scope; clone the KB elsewhere and `resume`; see the gate `stale` with its `next:` line.

**Must not say** (added to the existing list): "guarantees", "enforces", "verified by a human", "true" of a step or a delivery, "passed" of anything attested or waived, and "deterministic" without the sentence that says of what.

## 7. Open questions for the owner

### 7.1 The word "deterministic" on the website

The story is the owner's: determinism for agents, and work that resumes across sessions. The risk in the present copy is the reading "the agent's output is deterministic", and in "Done, and still true" the reading "Writfold checks truth".

| Option | Headline | Cost |
|---|---|---|
| **A (recommended)** | unchanged, with the definition of [section 6](#6-website-copy-release-step) directly beneath and the legend reworded | none to the story; the claim is exact where it is first read |
| B | "A deterministic process for agent work, resumable where it stopped." | the qualifier is in the headline; a weaker line |
| C | "Agent work that resumes where it stopped, with checks that say what still holds." | drops the word the owner chose |

Recommendation: A. What is deterministic is true and provable: the plan position, what is owed and the advance decision are functions of tracked files, proved in Lean for the kernels, with no clock and no model. Saying that in the second sentence keeps the story and removes the overclaim.

### 7.2 Should `policy-moved` exit 2?

Recommendation: yes. It applies only to runs pinned by this release, so no existing run changes its exit code, and a script that reads `plan status` as "is this run clean" should not get 0 for a run whose rules changed under it.

### 7.3 A rule whose match reads but whose requirement does not

A narrower form of [4.1](#41-a-broken-guard-rule-that-denies): when a guard rule's `commands` or `tools` and its `action` parse and another key does not, the hook could take the rule's action on a matched call, with the fault as the reason. A typo would then stop only the commands that rule names. Recommendation: not now. It reverses a decision made the day this was written, on no observed case; the guard log can show whether skipped rules occur in practice, and the change is small if they do.

### 7.4 Is 3.6 above the cut line?

Moving the run and subject binding into `gate::state` changes no behaviour and costs about a day of re-proof. Recommendation: above the line, as specified, because it turns "evidence from another run never counts" from a tested adapter into a theorem; but it is the first thing to move if the proofs of the new decisions run long.

### 7.5 Is `complete (unpinned)` the right reading for earlier runs?

Runs set before this release have no pin. Recommendation: label them and leave their summary otherwise as computed. Writing pins into old runs after the fact would record a policy they never started under.

## 8. What was checked

Compared with the code at `b14505b`; rows marked "run" were run with the release binary built from it.

| Description of today | Finding |
|---|---|
| A required step can be skipped | Yes (run). `plan/write.rs::skip` checks only that a current step exists; `required.toml` has three keys, `schema`, `start`, `end` |
| A waived or attested gate counts as met for an advance | Yes (run). `judge::Gate::met` is `Standing::met`, proved to be satisfied, attested or waived |
| `not-applicable` covers any item | Yes (run). `checklist::covered` is `Supported \| NotApplicable`; only `optional` distinguishes items |
| `--by` is free text | Yes (run): any non-empty text is recorded |
| A guard skips a rule that does not parse | Yes (run): the call passes, the fault goes to stderr, session start reports it; `guard status` exits 2 |
| A gate definition that does not parse can be met | No (run): `[undefined]`, the advance is refused |
| A required template can be omitted by the plan given | No (run): spliced around `--from FILE` as well |
| Evidence is shared between runs | No (run): `no receipt in this run` |
| Editing a definition mid-run goes unnoticed | No for evidence (run): stale at once, with the reason. Yes for the run's history: the plan templates are snapshotted and flagged `!template-moved`, gate definitions are not pinned |
| `required.toml` changes are detected | Only an end template added later (`plan/required.rs::missing_end`) |
| A closed run with open gates reads as complete | No (run): `[closed]`, with the current step and owed gates listed. `--note` on closing is accepted and not written (run) |
| A complete plan says how it was completed | Per step, yes. The summary line says `every step is done or skipped` (run) |
| A guard can hold implementation until research is met without blocking research | Yes (run): `paths = ["src/**"]` with `gate:research` |
| There is an initializer | No (run): `no KB found`; an empty `inventory/assets/` is enough |
| The checklist, guard and delivery kernels are proved | No: `plan`, `gate` (four functions), `assess` and `bytes` are; thirty-three claims, twenty mutants |

## Decisions (owner, 16 November 2026)

1. **Website wording:** keep the headline "Deterministic agent work that resumes where it stopped" and define beneath it what is deterministic (the recorded process and its checks, not the model's output); the legend becomes "Done, and its checks still hold". Copy as in section 6.
2. `policy-moved` exits 2: yes.
3. Guard rules that only partly parse keep being skipped loudly; no action from a partial parse for now.
4. The gate-binding proof (3.6) stays above the cut line, first to slip.
5. Earlier runs read `complete (unpinned)`: yes.
6. **Order with the proof backfill** ([proof-backfill-spec.md](2026-11-16-proof-backfill.md)): the backfill's first round lands first. Where this spec changes a kernel the backfill proves (gate standing, checklist roll-up, plan skip/advance), it replaces those theorems in place and re-runs the proofs gate; nothing is proved twice.
7. Lean first: every kernel in section 3 passes the proofs gate before it is wired into a verb.
