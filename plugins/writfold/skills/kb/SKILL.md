---
name: kb
description: Answer why / status / tracker / impact questions about this project from its Git-native KB with the `writfold kb` CLI, and record checkpoints and decisions there. Use when resuming work, when asked why something was decided, where something stands, which ticket tracks it, or what a change touches, and before ending a long task.
---

# Project continuity with `writfold kb`

The project KB is plain records in Git. `writfold kb <verb>` answers from it in budgeted rows. Every row that states a fact ends in its source, `@path:line`. Run the verbs with Bash, from the code repository or the KB. Add `--kb DIR` when the KB lives elsewhere. At session start a hook has already injected `brief` (and `resume` when a run is current), so don't re-run them just to orient yourself.

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
| Pick up a run: its checkpoint, coverage and rechecks | `resume [<run>]` (list runs with `runs --open`) |
| Save where the work stands | `checkpoint --note TEXT` (or `--new-run --project P --task T --note TEXT`) |
| Write down a ticket, decision or link | `record ticket\|decision\|link …` (see `writfold kb help`) |
| Run a check and record its receipt in the run | `gate run <id>` (definitions in `.writfold/gates/`) |
| Are the run's checks done, and still valid? | `gate status` (`open`, `satisfied`, `stale`, `failed`, `unknown`) |
| Does a receipt reproduce at its commit? | `gate verify <receipt>` |
| Receipt lines for a commit message | `gate lines` |
| Set a gate aside, or state an attested gate | `gate waive <id> --reason TEXT --by WHO`, `gate attest <id> --statement TEXT --by WHO` |
| Prepare a PR comment or Slack summary for review (nothing is sent) | `deliver prepare --mode pr-comment\|slack-summary --from FILE [--audience external]` |
| Check a delivery candidate, and see what it still owes | `deliver check [CANDIDATE]`, `deliver status` |
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

## Delivery candidates

`deliver prepare` copies your draft into the run as the exact payload that would be posted; Writfold never sends it. `deliver check` runs the mode's mechanical checks on those bytes: paragraph length, an audience deny list (`.writfold/deliveries/deny-<audience>.toml`), and whether every citation token is fresh. A failing check names the line and rule: fix the candidate (edit `candidate.md` in place, or prepare a new one) and run `deliver check` again, since any edit makes earlier receipts stale. Say a candidate is ready only when `deliver status` shows it `[ready]`. Checks are mechanical: they don't judge tone or truth, and they miss claims made without a citation token. The optional guard hook may deny a configured publishing command (such as `gh pr comment`) until a ready candidate holds the bytes being posted; it is a guard, not enforcement.

## Splicery or Edit

`edit apply` replays Splicery recipes against the run's worktree and keeps the recipe and preview as a receipt in the run. Choose by task:

- **Splicery** for repeated or structural edits: every line matching a pattern (`each @r in line_matches rx"…" expect N`), a block between two anchors, the same change across several files in one batch. The `expect` count and exact selectors refuse instead of guessing, and the receipt records what changed.
- **Ordinary Edit or sed** for one small change. Don't write a recipe for it.

Run `--dry-run` first and read the diff. A refusal names the recipe, the match sites and counts; fix the recipe, don't loosen it blindly. A file you changed after the replay is refused, never overwritten. After a crash, `edit recover` finishes the batch. The result is an uncommitted change: review it, run `gate status` (in-scope gates are now stale), and commit it. Nothing stops ordinary edits to the same files; this records what Splicery did, it does not guard the worktree.

## Checkpoint drafts

The plugin's hooks keep a local draft per session: the compaction summary, else your last message, plus the repository, branch and HEAD. A draft is never committed. Before ending a long task, checkpoint with a short note: the task, decisions with reasons, open questions and the exact next step. To keep a draft instead, ask the user first, then run `writfold kb checkpoint --from-draft <session>`. `resume` and the session-start brief list drafts (`!draft`) with their session IDs.
