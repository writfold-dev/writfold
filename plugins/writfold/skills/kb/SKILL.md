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

## Checkpoint drafts

The plugin's hooks keep a local draft per session: the compaction summary, else your last message, plus the repository, branch and HEAD. A draft is never committed. Before ending a long task, checkpoint with a short note: the task, decisions with reasons, open questions and the exact next step. To keep a draft instead, ask the user first, then run `writfold kb checkpoint --from-draft <session>`. `resume` and the session-start brief list drafts (`!draft`) with their session IDs.
