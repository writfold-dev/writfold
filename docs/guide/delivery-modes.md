# Delivery modes

> **Status:** built in v0.3.0-dev.1; your own modes are built in v0.4.0-dev.1; a delivery as a plan step's gate, `seq` on the manifest and the re-check of cited evidence on every read are built in v0.6.0-dev.1; a mode that declares nothing is never ready, for `deliver status` and the guard as for a plan step, since 16 November 2026; `seq` on check receipts, built in v0.6.0-dev.4.

A mode file, a deny list or `guard.toml` that is wrong is reported fault by fault, each at its line and column ([errors](errors.md)):

```text
$ writfold kb plan set ship
error[value.unknown]: paragraphs.bonded is not a built-in check; list a gate under gates
 --> .writfold/deliveries/release-note.toml:3:11
  |
3 | checks = ["paragraphs.bonded"]
  |           ^^^^^^^^^^^^^^^^^^^ not a built-in check
  |
  = help: did you mean "paragraphs.bounded"?

error[ref.undefined]: unknown gate `reveiw`: define .writfold/gates/reveiw.toml in the KB root
 --> .writfold/deliveries/release-note.toml:4:10
  |
4 | gates = ["reveiw"]
  |          ^^^^^^^^ no such candidate gate

2 errors in 1 file

writfold kb: refused: .writfold/deliveries/release-note.toml: 2 errors
```

A delivery candidate is the exact text of a PR comment or Slack summary, kept in the run and checked before anyone posts it. Writfold never sends it: there is no network code and no Slack or PR adapter. Posting stays with you or your agent.

## Prepare, check, repair

```text
$ writfold kb deliver prepare --mode pr-comment --from comment.md --audience external --destination harbor#42
$ writfold kb deliver check
delivery 20261112T211045Z-ee94cc [blocked] pr-comment audience=external destination=harbor#42 candidate=bb69f10c8fd9 bytes=136 @runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:1 !blocked
  check: paragraphs.bounded [satisfied] mechanical pass for bb69f10c8fd9 (at most 150 words per paragraph) @runs/run-…/checks/paragraphs.bounded-….json:1
  check: references.denied [failed] mechanical fail for bb69f10c8fd9: line 3 internal-host (column 50); line 3 ticket (column 23) @runs/run-…/checks/references.denied-….json:1 !failed
  check: claims.cited-fresh [satisfied] observed pass for bb69f10c8fd9 (0 citation tokens) @runs/run-…/checks/claims.cited-fresh-….json:1
```

`prepare` copies the file byte for byte into `runs/<run>/deliveries/<id>/candidate.md`. `--audience` is `internal` (the default) or `external`; `--destination` is a free one-line name. `check` writes one receipt per check, bound to the candidate's bytes.

To repair a draft, edit `candidate.md` in place (or prepare a new one) and run `deliver check` again. The edit makes the earlier receipts stale, and the row then carries `!edited`:

```text
$ writfold kb deliver status
delivery 20261112T211045Z-ee94cc [ready] pr-comment audience=external destination=harbor#42 candidate=f5d111e329c7 bytes=67 @runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:1 !edited
  check: paragraphs.bounded [satisfied] mechanical pass for f5d111e329c7 (at most 150 words per paragraph) @runs/run-…/checks/paragraphs.bounded-….json:1
  check: references.denied [satisfied] mechanical pass for f5d111e329c7 (2 rules for external) @runs/run-…/checks/references.denied-….json:1
  check: claims.cited-fresh [satisfied] observed pass for f5d111e329c7 (0 citation tokens) @runs/run-…/checks/claims.cited-fresh-….json:1
```

`deliver status` exits 0 only when every obligation of every candidate shown is satisfied, attested or waived.

## The three built-in checks

| Check | Class | Passes when |
|---|---|---|
| `paragraphs.bounded` | mechanical | no prose paragraph has more words than the bound. Code blocks, tables and headings are skipped; each list item is its own paragraph |
| `references.denied` | mechanical | no line matches a rule of the audience's deny list. With no deny list for the audience the outcome is `unknown`, never a pass |
| `claims.cited-fresh` | observed | every citation token in the draft is fresh under `kb stale`. A draft with no tokens passes, and the note says `0 citation tokens` |

The checks count words, match patterns and run `stale`. They do not judge tone or truth, and they miss a claim made in prose without a citation token.

## A mode file

Two modes are built in: `pr-comment` (all three checks, 150 words per paragraph) and `slack-summary` (all three, 60). A file in the KB root narrows a built-in mode, changes the bound or adds statements it needs. Unknown keys are refused.

```toml
# .writfold/deliveries/slack-summary.toml
schema = "writfold-delivery-mode/1"
mode = "slack-summary"             # must be the file's name, and a built-in mode
checks = ["paragraphs.bounded", "references.denied", "claims.cited-fresh"]
paragraph_words = 60               # 1 to 10000
attest = ["tone.reviewed"]         # obligations met only by deliver attest
```

An attested obligation is met by a statement, and a check can be set aside by a waiver. Both are shown as such, never as satisfied:

```sh
writfold kb deliver attest --check tone.reviewed --statement "Read aloud; neutral" --by owner
writfold kb deliver waive --check claims.cited-fresh --reason "cited repository not cloned here" --by owner
```

## A deny list

One file per audience. A rule is `name: pattern`; the pattern is a regular expression, ASCII and case-insensitive, matched against every line, code blocks included. A finding names the line, the rule and the column, and does not copy the matched text.

```toml
# .writfold/deliveries/deny-external.toml
schema = "writfold-deny/1"
audience = "external"
rules = ["internal-host: \\.corp\\.example\\.com", "ticket: \\bOPS-[0-9]+"]
```

Editing the deny list or the bound makes the matching receipts stale and leaves the others standing.

## The guard

An optional Claude Code `PreToolUse` hook, `writfold kb hook pre-tool-use`. It is off unless this file exists in the KB:

```toml
# .writfold/deliveries/guard.toml
schema = "writfold-guard/1"
commands = ["gh pr comment", "gh pr review", "gh issue comment"]
tools = ["mcp__slack__post_message"]
```

When a Bash call begins with a listed command, or a listed tool is called, the hook reads the body from `--body`, `-b`, `--body-file` or `-F`. The command line is read as guard rules read it, so a listed command quoted in a here-document's body is data and is not matched (built in v0.6.0-dev.1; [how a command is read](hooks.md#how-a-command-is-read)):

- body readable: the call goes on when a ready candidate of the current run holds exactly those bytes; otherwise it is denied, with the reason;
- body not readable: the call goes on only when the run has a candidate and every candidate is ready;
- no current run: denied.

A candidate is ready when its mode declares at least one check, gate or attestation and every one is met. A mode that declares nothing is never ready: its candidate reads `[blocked]` with `why: mode <name> declares nothing to check; a delivery is not met by an empty set: …`, `deliver status` exits 2, and the guard refuses its bytes. Before 16 November 2026 such a candidate read `[ready]` and the guard let it through, while a plan step on the same mode was refused; the rule is now one proved kernel function for all three (`writfold_core::delivery::ready`, [assurance](assurance.md#proved)).

A denial, as Claude Code receives it:

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"writfold delivery guard (a guard, not enforcement): no delivery candidate of run run-20261112T211025Z-7fe148 holds the bytes being posted (sha256:2221aeb6…); prepare and check one with writfold kb deliver"}}
```

When it lets a call through it prints nothing, so Claude Code's own permission prompt still applies. It is a guard, not enforcement: it sees only the calls the harness shows it, and it passes any command or tool it is not configured for. A missing KB, an unreadable configuration or a timeout lets the call proceed. Once a call matches, though, a KB it cannot read denies the call ("cannot read the KB") rather than letting it through. The same hook also evaluates [guard rules](hooks.md#guard-rules) (built in v0.5.0-dev.1), after this guard; a denial here wins.

## On a plan step

> Built in v0.6.0-dev.1.

A plan step can owe a delivery: the word `deliver:<mode>` in the step string (`"deliver: deliver:pr-comment"`). The step cannot be advanced until a candidate of that mode, prepared in the run, is ready, exactly as `deliver status` says `[ready]`. The [run graph guide](run-graph.md#a-delivery-as-a-steps-gate) shows the plan's side; this is the delivery's side.

- **Which candidate decides.** For the current step, the newest candidate of the mode that no done step is bound to. Newest is by the manifest's `seq`, then by ID, never by a clock. A candidate prepared before the step became current counts: drafting the comment during `implement` is ordinary work.
- **`seq`.** `deliver prepare` writes `"seq": N` in `delivery.json`, one more than the highest among the run's candidates of that mode. A manifest written before this release has none, counts as 0 and keeps ID order.
- **Which check receipt decides.** `deliver check` writes `"seq": N` in each receipt, one more than the highest among the candidate's receipts of that check; the newest is by `seq`, then by file name, never by the receipt's `recorded` time (built in v0.6.0-dev.4). A receipt written before carries none, reads as 0 and comes first, in name order, which is the order of the time in its name.
- **Binding.** `plan advance` writes the candidate's payload path and digest into the done record. That candidate is bound to the step for ever: the step is judged on it alone, and it never decides another step. Two steps naming one mode each need their own candidate. A newer candidate never turns a done step stale; an edit to the bound candidate, or a change to its mode, deny list or a mode gate, does (`!stale`), and the current step does not move back.
- **`deliver status`** adds one line under a candidate a step is judged on: `step: decides deliver:pr-comment at change#1/deliver [current]` or `step: bound to change#1/deliver [done]`. Other candidates read as before.
- **Not built:** `deliver prepare --step`, a step window for candidates, withdrawing a candidate, waiving a delivery as a whole (waive a check with `deliver waive`, or close the step with `plan skip --reason --by`).

**Posting under the guard.** With [the guard](#the-guard) on, the order on a posting step is: `deliver prepare`, `deliver check` (and `attest` or `waive` what the mode asks for), post (the guard passes the ready candidate's bytes), then `plan advance`. The done record says a candidate was ready; it never says the comment was posted, because Writfold sends nothing and sees no post. A team that wants "it was posted" on record adds an attested repository gate to the same step (`"deliver: deliver:pr-comment posted"`). It must not be an `attest` name of the mode: the guard requires the candidate ready before the post, so a mode statement made after posting could never be made in time.

## Cited evidence that moved

> Built in v0.6.0-dev.1.

A `claims.cited-fresh` receipt is bound to the candidate's bytes. From this release a passed receipt also stands only while every citation token in the candidate is still fresh: on every read (`deliver status`, `resume`, the delivery guard and the plan) the tokens are run through `stale` again, locally, with nothing fetched, the same call `deliver check` makes. When any is no longer fresh the check reads `stale`, its line ends `; cited evidence moved: <n> of <m> citation tokens no longer fresh`, and the candidate is `[blocked]` until `deliver check` runs again on evidence that holds. When the evidence comes back, the receipt stands again. A waiver on the check still stands: it is bound to the bytes and the check's definition, not to the receipt. A candidate with no citation token costs nothing more.

Limits: a mode gate of class `observed` is not re-run on read, and a claim made in prose without a citation token is not seen.

**A behaviour change.** A candidate whose citations moved after its check read `[ready]` before this release; it now reads `[blocked]`, `deliver status` exits 2, and the delivery guard refuses its bytes.

## Your own modes

Any `.writfold/deliveries/<name>.toml` defines a mode. The two built-in modes (`pr-comment`, `slack-summary`) stay as defaults. A mode lists built-in `checks`, and `gates`: script gates run against the candidate through `{candidate}`, and attested gates whose instructions are shown. Unknown keys, unknown gates and repository gates are refused when the mode is loaded.

```toml
# .writfold/deliveries/commit-message.toml
schema = "writfold-delivery-mode/1"
mode = "commit-message"
description = "checked before git commit; nothing binds the checked bytes to the commit that is made"
checks = ["references.denied"]
gates = ["commit-format", "language-guidelines"]
```

```sh
writfold kb deliver prepare --mode commit-message --audience external --from MSG
writfold kb deliver check                  # references.denied and commit-format
writfold kb deliver attest --check language-guidelines --statement TEXT --by WHO
writfold kb deliver status                 # exit 0 only when every check is met
git commit -F MSG
```

The limit is printed with every status: nothing ties the checked bytes to the commit that is then made. Commit with `-F` on the candidate file itself. The worked example, including the deny list that keeps internal hosts and ticket numbers out of public messages, is in `fixtures/kb/commit-message/`.

