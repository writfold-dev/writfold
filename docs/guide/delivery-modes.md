# Delivery modes

> **Status:** built in v0.3.0-dev.1; your own modes are built in v0.4.0-dev.1.

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

When a Bash call begins with a listed command, or a listed tool is called, the hook reads the body from `--body`, `-b`, `--body-file` or `-F`:

- body readable: the call goes on when a ready candidate of the current run holds exactly those bytes; otherwise it is denied, with the reason;
- body not readable: the call goes on only when the run has a candidate and every candidate is ready;
- no current run: denied.

A denial, as Claude Code receives it:

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"writfold delivery guard (a guard, not enforcement): no delivery candidate of run run-20261112T211025Z-7fe148 holds the bytes being posted (sha256:2221aeb6…); prepare and check one with writfold kb deliver"}}
```

When it lets a call through it prints nothing, so Claude Code's own permission prompt still applies. It is a guard, not enforcement: it sees only the calls the harness shows it, it passes any command or tool it is not configured for, and a missing KB, an unreadable configuration or a timeout lets the call proceed.

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

