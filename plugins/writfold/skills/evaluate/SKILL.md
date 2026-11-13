---
name: evaluate
description: Evaluate the session that is ending, from the run's records - write the run's reflection entry, propose a few cited KB records or state "no proposal", meet the reflect step's gates and advance the plan. Use as the last step of a run, when the plan's current step is reflect, or when the session-start inbox lists a run as unevaluated.
argument-hint: "[what this session did and learned, in a few lines]"
context: fork
background: false
allowed-tools: Bash(writfold kb resume *) Bash(writfold kb plan status *) Bash(writfold kb plan use *) Bash(writfold kb plan advance *) Bash(writfold kb checkpoint *) Bash(writfold kb propose *) Bash(writfold kb proposals list *) Bash(writfold kb cite make *) Bash(writfold kb gate run *) Bash(writfold kb gate status *) Bash(writfold kb gate attest *)
---

# Evaluate the session that is ending

You are closing one session of work on this project. You run in a fresh context: you do not see the conversation, only what the caller wrote below and what the project's KB records. Judge from the records; Writfold cares what they show, not how the work was done.

The caller's account of the session (it may be empty):

$ARGUMENTS

## Where the run's plan stands

This was printed when the skill loaded. It splices the `reflect` template before the current step, unless the run already has a reflect step (a required end step, or an earlier splice), in which case it changes nothing:

!`writfold kb plan use reflect || true`

## Steps

Run every command with Bash, one per call, with no `;`, `&&` or pipe: only the `writfold kb` commands below are pre-approved, and a compound command needs the user's approval. Add `--kb DIR` if the KB is not found from the working directory.

1. **Read.** Run `writfold kb resume` and `writfold kb plan status`. Note the run, the current step and what it owes, the drafts (`!draft`: a session's last message, often the best account of what happened) and the pending proposals. If there is no current run, stop and report that there is nothing to evaluate in the KB.
2. **Find the reflect step.**
   - The current step ends in `/reflect`: go on.
   - An earlier step is current (its gates are still owed): the session is ending before the work's steps are done. Write the reflection anyway (steps 3 and 4) and report which step the reflect step waits behind. Do not advance, skip or waive the earlier step.
   - The printed answer says the KB has no `reflect` template: write the reflection and the proposals anyway, and report that the KB can add one (a `reflect` plan template with a `reflected` run-record gate; see Writfold's run-graph guide).
   - Every reflect step is already done and this session did more work since: run `writfold kb plan use reflect --again` to add a fresh one.
3. **Write the reflection** as one checkpoint entry whose first line starts `reflection:`:

   ```sh
   writfold kb checkpoint --note "reflection: <one line on the outcome>
   <a few more lines>"
   ```

   If `plan status` or the gate row shows a `read:` line for the reflection gate, read that file first: it is what a good reflection covers in this KB. Otherwise cover four things in under fifteen lines: **what was done** (the commits or files, and the gates or citations it rests on); **what was learned** (what went differently from the plan, a fact not written down before); **what should change** (one change to a template, a gate, the code or the KB, if any); **what was backfilled** (the KB records written or proposed in this run, by ID). Quote `@path:line` for every fact you take from a row. Do not restate the diff or grade the work.
4. **Propose, or say why not.** Look for what the KB lacked during the run: a question it could not answer, a decision made but not recorded, a link between assets that was found. For each, at most three in all:

   ```sh
   writfold kb propose ticket|decision|link … --run <run>
   ```

   with the same arguments as `writfold kb record`, and a citation in it (a `⟦cite …⟧` token from `writfold kb cite make`, a `<repo>@<commit>:<path>:<a-b>` locator in a link's `--evidence`, or a KB row's `@path:line`). Propose only what the records or the code support. If nothing is worth proposing, add a line starting `no proposal:` with the reason to the reflection entry (step 3); a second checkpoint entry is fine if the first is written. Never run `record`, `proposals accept` or `proposals drop`: the user decides.
5. **Meet the reflect step's gates.** For each gate the step owes:
   - a run-record or repository gate (`reflected` in the worked example): `writfold kb gate run <id>`. It counts only when `writfold kb gate status` shows it `[satisfied]`; if it fails, its notes say what is missing (a `reflection:` entry, or a proposal or `no proposal` line). Fix the records, not the gate.
   - an attested gate (`reflection` in the worked example): read its `read:` file, check your entry against it point by point, then `writfold kb gate attest <id> --statement "<what you checked>" --by agent`. Invoking this skill is the request to make that statement; make it only if it is true.

   Never waive a gate, and never edit a receipt or an instructions file.
6. **Advance.** Run `writfold kb plan advance`. If it is refused (exit 3), it names what is still owed: do that, or report it. Do not `plan skip`.
7. **Report** in a few lines: the reflection entry (`@path:line`), each proposal's ID and one-line summary or the `no proposal:` line, each gate of the step with its standing (`[satisfied]`, `[attested]`, or what it still owes), whether the plan advanced and what is current now. End with what the user must do: review the proposals (`writfold kb proposals show <id>`, accepted only by them), and commit the KB so the reflection reaches other clones.
