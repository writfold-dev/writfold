---
name: kb
description: Answer why / status / tracker / impact questions about this project from its Git-native KB with the `writfold kb` CLI, and record checkpoints and decisions there. Use when resuming work, when asked why something was decided, where something stands, which ticket tracks it, or what a change touches, and before ending a long task.
---

# Project continuity with `writfold kb`

The project KB is plain records in Git. `writfold kb <verb>` answers from it in budgeted rows. Every row that states a fact ends in its source, `@path:line`. Run the verbs with Bash, from the code repository or the KB. Add `--kb DIR` when the KB lives elsewhere. At session start a hook has already injected `brief` (and `resume` and the plan position when a run is current, an inbox, and a line if the KB clone is behind its remote), so don't re-run them just to orient yourself. After a compaction the hook injects the plan position again.

## Which verb

| Question | Verb |
|---|---|
| Where do we stand? (project inferred from the directory) | `brief [<project>\|--all]` |
| Why did we decide X? | `why <id\|text>` (accepted decisions first, supersession followed) |
| Everything about one asset, ticket or record | `context <id\|alias\|ticket>` |
| What is the state of X, and what tracks it? | `status <id\|project>` |
| Find a record by words, a partial ID or a typo | `find <text>` (`match <text>` also tests signature patterns) |
| What does changing X touch? | `impact <id> [--depth N] [--direction in\|out\|both]` |
| Which citations have moved? | `stale [<project>\|<path>]` |
| Is everything still valid: citations, the index, the run's gates? | `check [<project>\|<path>]` (one row per area; worst exit code) |
| Pick up a run: its checkpoint, coverage and rechecks | `resume [<run>]` (list runs with `runs --open`) |
| Save where the work stands | `checkpoint --note TEXT` (or `--new-run --project P --task T --note TEXT`) |
| Write down a ticket, decision or link | `record ticket\|decision\|link …` (see `writfold kb help`) |
| Suggest a record for the user to review, without writing it | `propose ticket\|decision\|link …` (same arguments); `proposals [list]`, `proposals show <id>` |
| Run a check and record its receipt in the run | `gate run <id>` (definitions in `.writfold/gates/`; an `assessed` gate runs Weft on the step's source) |
| Are the run's checks done, and still valid? | `gate status` (`open`, `satisfied`, `stale`, `failed`, `unknown`) |
| Does a receipt reproduce at its commit? | `gate verify <receipt>` |
| Receipt lines for a commit message | `gate lines` |
| Set a gate aside, or state an attested gate | `gate waive <id> --reason TEXT --by WHO`, `gate attest <id> --statement TEXT --by WHO` |
| Cover one item of a checklist gate | `gate cover <id> <item> supported REF… --by WHO`, or `unsupported\|not-applicable --reason TEXT --by WHO` |
| Prepare a PR comment, Slack summary or other draft for review (nothing is sent) | `deliver prepare --mode MODE --from FILE [--audience external]` (built in: `pr-comment`, `slack-summary`; others from `.writfold/deliveries/<mode>.toml`) |
| Check a delivery candidate, and see what it still owes | `deliver check [CANDIDATE]`, `deliver status` |
| Where does the run's plan stand, and may it move on? | `plan status`, `plan advance` (`plan set <template>` to start one; `plan use <template>` splices a template's steps before the current step, once) |
| Apply a Splicery recipe in the run's worktree, keeping a receipt | `edit apply RECIPE.spl… [--dry-run]` (`--worktree DIR` once if the run records none) |

Writes (`checkpoint`, `record`, `status --set`) change the work tree only. They never commit; commit the KB yourself when the user wants it shared.

## Reading an answer

The last line is always the trailer:

```text
# coverage=complete records=28/28 shown=7 omitted=0 [stale=1] kb=d5db758bda80 index=fresh cursor=none q=…
```

- `coverage=partial` means the answer is incomplete. Records were unreadable, the walk hit its depth (`frontier=`), or nothing matched. Say so; don't present it as the whole picture.
- `answer=unknown reason=…` means the KB has no answer. Its absence from the KB is not evidence about the world. Look in the sources, then `record` what you found.
- `stale=N` / `unverifiable=N`: N cited sources have moved or could not be checked. A row flagged `!stale`, `!moved→a-b`, `!changed`, `!gone`, `!expired` or `!superseded→<id>` must be re-checked at its source before you act on it.
- `omitted=N` with `cursor=v2.…`: more rows exist. Re-run with `--cursor <value>` (same query) or a larger `--budget-tokens`.
- `kb=<commit>+dirty:N` means N KB files have uncommitted changes.

Exit codes: 0 complete; 2 partial, unknown or stale (still a usable answer; read the trailer); 3 refused (an ambiguous target: pick one of the listed candidates; or a write conflict); 1 error.

## Rules

- Quote the `@path:line` of every row you rely on.
- Re-check any row carrying a `!` flag before acting on it.
- On `answer=unknown`, look in the sources, then `record` what you found.

## Gates

A gate is a command with a path scope. Only `gate run` writes a receipt; never write or edit one by hand (a hand-written receipt reads as invalid). Say a check passed only when `gate status` shows the gate `[satisfied]`: `!stale` means an in-scope file changed since the receipt, `!failed` means the newest receipt for these bytes failed, and `!unknown` (a timeout or crash) is never a pass. Each such row has a `next:` line with the command to run. `[waived]` (with a reason) and `[attested]` (a statement, for gates of class `attested`) are statements, not checks: report them as waived or attested, never as passed. Waive or attest only when the user asks you to, and use `--by` honestly (`--by agent` when it is you). `gate status` and `gate lines` exit 0 only when every gate is satisfied, attested or waived.

A row with a `read:` line names the gate's instructions file. Read it before you run, repair or attest that gate: it is what the gate checks, and on an attested gate it is what your statement says you followed. Editing it (or a script the gate ships beside itself) makes earlier receipts and attestations stale, so they must be earned again.

A gate of class `assessed` is met by a Weft assessment of a plan step. Write the step's assumptions and checks in the Weft file its definition names (its `next:` line shows where; `{run}/…` is under the run's directory). Cite each assumption's evidence with `claim <name> = citation "<reference>";` directly above its `assume`: a KB citation token, `gate:<id>`, or a receipt of the run (path or name). Then `gate run <id>`. Each check shows `[<Weft result> <basis>]`: only `entailed grounded` counts as met. `!hypothetical` means it rests on an assumption whose citation is stale, failed, missing or absent; a `cite:` line names it. Fix the evidence (rerun the gate, refresh the citation), never the wording of the check, and do not report a hypothetical check as passed. You cannot choose the Weft binary; if the row says there is none or its digest is refused, tell the user.

A gate of class `checklist` shows one `item:` line per item. Cover an item only after you checked it: `supported` with the references you relied on (a citation token, `gate:<id>`, a receipt of the run), each one argument; `not-applicable` or `unsupported` with a reason. A reference that is not grounded now is refused; fix the evidence, not the row. A covered checklist reads `[attested]`: report it as covered by you, never as passed. An item shown `[stale]` lost its evidence or its content moved; check it again before covering it again.

## Plans

When the run has a plan, `resume` shows one `plan` line: the current step and the gates it still owes. Work the current step, meet its gates (`gate run`, or `gate attest` for an attested gate), then `plan advance`. A refusal (exit 3) names what is still owed, first owed gate first, with the command to run; nothing is written. Start a plan with `plan set <template>` (templates in `.writfold/plans/`) when the user or a skill names one. `plan skip --reason TEXT --by WHO` closes a step without calling it done: only when the user asks. A done step shown `!stale` stays done; if the check matters now, run it again. A step shown `!template-moved` comes from a template edited since it was spliced: the run keeps its old steps; tell the user rather than re-splicing on your own. Required steps (`.writfold/plans/required.toml`, such as a closing `reflect`) are in the plan like any other: the plan is not complete until they are done. Gates of class `run-record` judge what the run wrote: `research.backfilled` wants a KB record, a retrieval receipt or a citation in a checkpoint since the step began; `run.reflected` wants a checkpoint starting `reflection:` plus a proposal or a line starting `no proposal`. To close a run (its `reflect` step, or a run the inbox lists as `unevaluated`), invoke the `writfold:evaluate` skill: it writes the reflection, proposes or says `no proposal`, meets the step's gates and advances.

## Delivery candidates

`deliver prepare` copies your draft into the run as the exact payload that would be posted; Writfold never sends it. `deliver check` runs the mode's mechanical checks on those bytes: paragraph length, an audience deny list (`.writfold/deliveries/deny-<audience>.toml`), and whether every citation token is fresh. A failing check names the line and rule: fix the candidate (edit `candidate.md` in place, or prepare a new one) and run `deliver check` again, since any edit makes earlier receipts stale. Say a candidate is ready only when `deliver status` shows it `[ready]`. Checks are mechanical: they don't judge tone or truth, and they miss claims made without a citation token. The optional guard hook may deny a configured publishing command (such as `gh pr comment`) until a ready candidate holds the bytes being posted; it is a guard, not enforcement.

A KB may define its own modes (a commit message, a release note) in `.writfold/deliveries/<mode>.toml`. Their script gates run on the candidate during `deliver check` and report `line N` findings like the built-in checks; their attested gates show a `read:` line and are met only by `deliver attest`, after you have read that file and checked the draft against it. For a commit message, prepare and check the draft, then commit with exactly those bytes (`git commit -F` the candidate); nothing ties the checked draft to the commit you make.

## Splicery or Edit

`edit apply` replays Splicery recipes against the run's worktree and keeps the recipe and preview as a receipt in the run. Choose by task:

- **Splicery** for repeated or structural edits: every line matching a pattern (`each @r in line_matches rx"…" expect N`), a block between two anchors, the same change across several files in one batch. The `expect` count and exact selectors refuse instead of guessing, and the receipt records what changed.
- **Ordinary Edit or sed** for one small change. Don't write a recipe for it.

Run `--dry-run` first and read the diff. A refusal names the recipe, the match sites and counts; fix the recipe, don't loosen it blindly. A file you changed after the replay is refused, never overwritten. After a crash, `edit recover` finishes the batch. The result is an uncommitted change: review it, run `gate status` (in-scope gates are now stale), and commit it. Nothing stops ordinary edits to the same files; this records what Splicery did, it does not guard the worktree.

## Checkpoint drafts

The plugin's hooks keep a local draft per session: the compaction summary, else your last message, plus the repository, branch and HEAD. A draft is never committed. Before ending a long task, checkpoint with a short note: the task, decisions with reasons, open questions and the exact next step. To keep a draft instead, ask the user first, then run `writfold kb checkpoint --from-draft <session>`. `resume` and the session-start brief list drafts (`!draft`) with their session IDs.

## Proposals

When you find something worth recording but the user has not asked you to write it, `propose` it: the same arguments as `record`, kept in a local queue (never committed, lost with the clone), one file per call, at most ten per session (pass `--session` or set `WRITFOLD_SESSION`). `resume` and the session-start context list pending proposals. Accept (`proposals accept <id>`) or drop one only when the user asks. Accept runs the real `record`; it is refused, keeping the proposal, when a citation in it is no longer fresh or the ID is taken: re-cite and propose again, or drop it.
