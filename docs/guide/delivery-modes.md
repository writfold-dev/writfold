# Delivery modes

> **Status:** built in v0.3.0-dev.1; your own modes are built in v0.4.0-dev.1; a delivery as a plan step's gate, `seq` on the manifest and the re-check of cited evidence on every read are built in v0.6.0-dev.1; a mode that declares nothing is never ready, for `deliver status` and the guard as for a plan step, since 16 November 2026; `seq` on check receipts, built in v0.6.0-dev.4. The second round ([spec](../rfcs/2026-11-17-delivery-modes-second-round.md), [design §47](../writfold/kb-v1-design.md#47-delivery-modes-second-round-as-built-17-november-2026-v060-dev4)) is built in v0.6.0-dev.4: a mode's policy (`waivable`, a mode gate's `accept`), withdrawing a candidate, `rendering` and `deliver payload`, guard routes, `deliver match`, post records and `posted:<mode>`, findings drawn against the candidate, `deliver modes`, a mode's `audience`, `--from -`, and `kb init --example delivery`. The `length.bounded` check and a reminder after a guarded call passes are built in v0.6.0-dev.11.

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

A delivery candidate is the exact text of a PR comment, a Slack summary or a commit message, kept in the run and checked before anyone posts it. Writfold never sends it: there is no network code and no Slack or PR adapter. Posting stays with you or your agent.

## Prepare, check, repair

```text
$ writfold kb deliver prepare --mode pr-comment --from comment.md --audience external --destination harbor#42
$ writfold kb deliver check
delivery 20261112T211045Z-ee94cc [blocked] pr-comment audience=external destination=harbor#42 candidate=bb69f10c8fd9 bytes=136 @runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:1 !blocked
  check: paragraphs.bounded [satisfied] mechanical pass for bb69f10c8fd9 (at most 150 words per paragraph) @runs/run-…/checks/paragraphs.bounded-….json:1
  check: references.denied [failed] mechanical fail for bb69f10c8fd9: line 3 internal-host (column 50); line 3 ticket (column 23) @runs/run-…/checks/references.denied-….json:1 !failed
  check: claims.cited-fresh [satisfied] observed pass for bb69f10c8fd9 (0 citation tokens) @runs/run-…/checks/claims.cited-fresh-….json:1
  next: edit runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md (line 3: internal-host, ticket), then: writfold kb deliver check 20261112T211045Z-ee94cc
```

`prepare` copies the file byte for byte into `runs/<run>/deliveries/<id>/candidate.md`. `--from -` reads standard input, with the same limits; the manifest's source name is then `-`. `--audience` is `internal` (the default) or `external`, unless the mode fixes it ([below](#a-modes-audience)); `--destination` is a free one-line name. `check` writes one receipt per check, bound to the candidate's bytes.

**Findings drawn.** For each failed or unknown check whose receipt is about the present bytes, `deliver check` and `deliver status` draw the findings on stderr, in the form of [file diagnostics](errors.md), up to ten, then `… and N more: <receipt path>`. The rows stay on stdout; with `--json` nothing is drawn. The span is found again by matching the rule on that line of the present payload; receipts copy no matched text. Nothing is drawn for a stale receipt.

```text
failed[references.denied]: line 3 matches the deny rule `internal-host` for audience external
 --> runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:3:50
  |
3 | …
  |                                                  ^^^^^^^^^^^^^^^^^ internal-host
  |
  = note: rule `internal-host: \.corp\.example\.com` @.writfold/deliveries/deny-external.toml:3
  = help: reword or remove the match, then: writfold kb deliver check 20261112T211045Z-ee94cc
```

A paragraph over the bound is drawn as `failed[paragraphs.bounded]: the paragraph at line 7 has 170 words; this mode takes at most 150` with `help: split or shorten it (paragraph_words = 150 @built-in pr-comment)`; a citation that moved as `failed[claims.cited-fresh]: the citation at line 1 no longer matches its source (…)`; a missing deny list as `unknown[references.denied]: no deny list for audience internal` with `help: the deny list is the team's policy: ask before writing .writfold/deliveries/deny-internal.toml (rules = [] says nothing is denied)`. A script gate's finding is drawn with the script's text.

**`next:`** under a candidate follows its first obligation not met under its policy:

| First unmet | `next:` |
|---|---|
| a check or script gate with no receipt about these bytes, or a stale one | `writfold kb deliver check <id>` |
| a failed check or script gate | `edit <path to candidate.md> (line 3: internal-host, ticket), then: writfold kb deliver check <id>` |
| `references.denied` unknown for want of a deny list | `ask for .writfold/deliveries/deny-<audience>.toml (the team's policy), then: writfold kb deliver check <id>` |
| an attested obligation | `writfold kb deliver attest <id> --check <name> --statement TEXT --by WHO` |
| a statement the policy does not take | no command; `why:` names the key and its place |
| none (ready), and no post recorded | `post <payload file> to <destination>, then: writfold kb deliver posted <id> --ref URL --by WHO` |

To repair a draft, edit `candidate.md` in place (or prepare a new one) and run `deliver check` again. The edit makes the earlier receipts stale.

**`!edited`** means the candidate differs from the file it was prepared from. It stays after an in-place repair, also on a `[ready]` candidate: checks are about the candidate, not the source. `deliver status` says so once in its notes when the flag is shown (`!edited: the candidate differs from the file it was prepared from; checks are about the candidate`).

```text
$ writfold kb deliver status
delivery 20261112T211045Z-ee94cc [ready] pr-comment audience=external destination=harbor#42 candidate=f5d111e329c7 bytes=67 @runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:1 !edited
  check: paragraphs.bounded [satisfied] mechanical pass for f5d111e329c7 (at most 150 words per paragraph) @runs/run-…/checks/paragraphs.bounded-….json:1
  check: references.denied [satisfied] mechanical pass for f5d111e329c7 (2 rules for external) @runs/run-…/checks/references.denied-….json:1
  check: claims.cited-fresh [satisfied] observed pass for f5d111e329c7 (0 citation tokens) @runs/run-…/checks/claims.cited-fresh-….json:1
  next: post runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md to harbor#42, then: writfold kb deliver posted 20261112T211045Z-ee94cc --ref URL --by WHO
```

`deliver status` exits 0 only when every obligation of every live candidate shown is met under its policy. Its trailer counts `candidates`, `ready`, each standing, `invalid` and `withdrawn`.

## Which candidate a verb means

`deliver check`, `attest`, `waive`, `withdraw`, `payload` and `posted` take a candidate ID (or the six hex digits that end it). Without one:

- with `--mode M`, the candidate that decides for `M`: the newest live one, if no done step is bound to it ([on a plan step](#on-a-plan-step));
- else the run's only live, unbound candidate;
- else the verb refuses, listing each live, unbound candidate with its mode and state: `2 live, unbound candidates; name one, or give --mode: 20261117T103311Z-e948b4 (pr-comment, ready), 20261117T103516Z-336665 (commit-message, blocked)`.

`deliver prepare` adds a note when the run already has a live, unbound candidate of the mode: `<older id> is an older pr-comment candidate; this one now decides. Withdraw the older one if it is not meant: writfold kb deliver withdraw <older id> --reason TEXT --by WHO`.

## The built-in checks

| Check | Class | Passes when |
|---|---|---|
| `paragraphs.bounded` | mechanical | no prose paragraph has more words than the bound. Code blocks, tables and headings are skipped; each list item is its own paragraph |
| `references.denied` | mechanical | no line matches a rule of the audience's deny list. With no deny list for the audience the outcome is `unknown`, never a pass |
| `claims.cited-fresh` | observed | every citation token in the draft is fresh under `kb stale`. A draft with no tokens passes, and the note says `0 citation tokens` |
| `length.bounded` | mechanical | (built in v0.6.0-dev.11; only in a mode that lists it, never a default) the whole payload has at most `max_chars` characters and `max_lines` lines. A finding names the line where the bound is crossed: `failed[length.bounded]: 3 lines; this mode takes at most 2 (max_lines)`. Listing it without either bound is refused |
| `prose.rules` | mechanical | (built in v0.6.0-dev.20; only in a mode that lists it with `prose = "<set>"`, never a default) no prose line matches a **blocking** rule of the rule set. Matches of **advice** rules are shown and recorded and never hold the draft back. A rule set with no rules is `unknown`, never a pass ([below](#prose-rules)) |

The checks count words, match patterns and run `stale`. They do not judge tone or truth, and they miss a claim made in prose without a citation token.

## A mode file

Two modes are built in: `pr-comment` (all three checks, 150 words per paragraph) and `slack-summary` (all three, 60). Their definitions did not change in v0.6.0-dev.4, so no receipt or done step of an earlier run moves. A file in the KB root narrows a built-in mode, changes the bound or adds statements it needs. Unknown keys are refused.

```toml
# .writfold/deliveries/slack-summary.toml
schema = "writfold-delivery-mode/1"
mode = "slack-summary"             # must be the file's name, and a built-in mode
checks = ["paragraphs.bounded", "references.denied", "claims.cited-fresh"]
paragraph_words = 60               # 1 to 10000
attest = ["tone.reviewed"]         # obligations met only by deliver attest
waivable = ["claims.cited-fresh"]  # absent: every waiver is taken; []: none
audience = "internal"              # absent: --audience decides
rendering = "verbatim"             # or "strip-citations"
```

A mode that lists `length.bounded` in `checks` also sets `max_chars` (1 to 1000000) or `max_lines` (1 to 100000), or both (built in v0.6.0-dev.11).

An attested obligation is met by a statement, and a check can be set aside by a waiver the mode takes. Both are shown as such, never as satisfied:

```sh
writfold kb deliver attest --check tone.reviewed --statement "Read aloud; neutral" --by owner
writfold kb deliver waive --check claims.cited-fresh --reason "cited repository not cloned here" --by owner
```

When a mechanical or observed obligation of the candidate is not met, `attest` and `waive` still write the statement and add a note: `references.denied is [failed]; repairing the candidate will make this statement stale`. Attest last.

### What a mode takes: `waivable` and a mode gate's `accept`

`waivable` lists the built-in checks and `attest` names of the mode that a waiver may set aside. Absent, every waiver is taken, as before; `[]` takes none. A name the mode does not declare is `ref.undefined`, with a suggestion. A mode gate carries its own policy: naming one in `waivable` is `value.conflict` (`` `commit-format` is a mode gate: set accept in .writfold/gates/commit-format.toml ``), and the gate's `accept` ([gates](gates.md#which-standings-meet-a-gate-accept)) decides for it.

`deliver waive` on an obligation the policy does not take is refused, exit 3, nothing written:

```text
refused: commit-message does not accept a waiver of references.denied (waivable = ["tone.reviewed"] @.writfold/deliveries/commit-message.toml:7); meet it: writfold kb deliver check 20261117T103516Z-336665
refused: gate commit-format does not accept a waiver (accept = ["satisfied"] @.writfold/gates/commit-format.toml:12); meet it: writfold kb deliver check 20261117T103516Z-336665
```

A waiver that exists anyway (written before the key, by hand, or merged from a clone) is shown and counts for nothing. The line reads `[waived] … !waived !not-accepted`, a `why:` line names the key, the candidate is `[blocked]` and `deliver status` exits 2:

```text
  check: references.denied [waived] mechanical by agent: meh (check: failed) @runs/run-…/statements/references.denied-….json:1 !waived !not-accepted
  why: references.denied does not accept waived (waivable = [] @.writfold/deliveries/commit-message.toml:16)
```

The guard, `deliver match` and a `deliver:<mode>` step read it the same way; the step owes `deliver:commit-message [waived, not accepted]`. `kb lint` warns (`policy.waivable`; exit unchanged without `--deny-warnings`) when a mode lists `references.denied` or a mechanical gate and still takes a waiver of it, since the deny list or the gate is then one `deliver waive` from moot. `--by` is text: to have a person asked before a waiver or a withdrawal, add the harness's permission rule ([policy](policy.md#what-already-holds)).

### A mode's audience

`audience = "internal"` or `"external"` in a mode file fixes the audience: `prepare` without `--audience` takes it, and a different `--audience` is refused (`commit-message is for audience external (@.writfold/deliveries/commit-message.toml:15); prepare without --audience`). Absent, `--audience` decides, as before.

## A deny list

One file per audience. A rule is `name: pattern`; the pattern is a regular expression, ASCII and case-insensitive, matched against every line, code blocks included. A finding names the line, the rule and the column, and does not copy the matched text.

```toml
# .writfold/deliveries/deny-external.toml
schema = "writfold-deny/1"
audience = "external"
rules = ["internal-host: \\.corp\\.example\\.com", "ticket: \\bOPS-[0-9]+"]
```

Editing the deny list or the bound makes the matching receipts stale and leaves the others standing. An explicit empty list (`rules = []`) passes as `0 rules for internal`: the team saying nothing is denied.

## Prose rules

Built in v0.6.0-dev.20 ([#127](https://github.com/writfold-dev/writfold/issues/127), [RFC](../rfcs/2026-11-25-prose-rules-and-advice.md)). You keep a rule set as data beside the deny lists, and a mode names it:

```toml
# .writfold/deliveries/prose-plain.toml
schema = "writfold-prose-rules/1"
name = "plain"
advise = ["filler: \\b(just|simply|basically)\\b", "hedge: \\b(I think|probably)\\b"]
block = ["placeholder: \\b(TODO|TBD|FIXME)\\b"]
```

```text
# in a mode file
checks = ["paragraphs.bounded", "prose.rules"]
prose = "plain"
```

Rules use the deny list's `name: pattern` form: ASCII regular expressions, case-insensitive. They are matched line by line against **prose only**. Lines inside code fences and indented code blocks are skipped, and inline code spans and URLs are blanked first, so a rule never fires on code.

- **A blocking match** fails the check, like a deny-list match. It is drawn against the line with the rule and column, and it is waivable only if the mode says so.
- **Advice** is recorded in the receipt (`result.advice`, `advice_total`) and shown, but it never changes the outcome, the exit status or a `next:` line. `deliver status` adds `2 advisory: line 3 filler, line 5 hedge`, and `deliver check` draws each piece as `advice[prose.rules]: filler (column 9)` with its location. Advice from a receipt about other bytes is not shown.
- **No rules** (both lists empty) is `unknown`, never a pass.

Editing a pattern, renaming a rule or moving a rule between `advise` and `block` makes earlier receipts stale. `prose` is required when `checks` lists `prose.rules` and refused otherwise. A missing or broken rule file is refused when the mode loads.

The decision (advice never decides, any blocking match fails, no rules is unknown) is proved in Lean (`writfold_core::prose`). Reading the rules and matching them are tested.

These are rules you write. They do not do grammar or spelling, they do not judge tone or truth, and they give no quality score. `kb init --example delivery` writes a short example set, `prose-plain.toml`, to edit. An external linter (Vale, Harper, anything) can give advice through a candidate script gate (below); a recipe for Vale is planned ([#117](https://github.com/writfold-dev/writfold/issues/117)).

## What a mode asks for: `deliver modes`

`deliver modes [MODE]` is read-only. It prints one row per mode, built-in or file: its audience, rendering, paragraph bound and source; one `check:` line per obligation with its class and whether the mode takes a waiver of it; a `read:` line for a gate's instructions; the deny rule names per audience; and the guard routes that post it. Read it before writing the draft.

```text
$ writfold kb deliver modes commit-message
mode commit-message audience=external rendering=verbatim paragraph_words=150 source=.writfold/deliveries/commit-message.toml @.writfold/deliveries/commit-message.toml:1
  check: references.denied mechanical (no waiver)
  check: commit-format mechanical (no waiver)
  deny: external: 2 rules (internal-host, ticket)
  route: git commit
```

`kb policy` prints the same mode rows after the gates, then one row per guard route. Its `modes=` count is unchanged. A mistyped mode is refused with the KB's modes and a suggestion: `no delivery mode pr-coment: add .writfold/deliveries/pr-coment.toml (…); the modes are: pr-comment, slack-summary, commit-message, release-note; help: did you mean "pr-comment"?`.

## Withdrawing a candidate

```text
$ writfold kb deliver withdraw 20261117T103607Z-2d4f65 --reason "pasted the wrong text" --by agent
delivery 20261117T103607Z-2d4f65 [withdrawn] pr-comment … @runs/run-…/deliveries/20261117T103607Z-2d4f65/candidate.md:1 !withdrawn
  withdrawn: by agent: pasted the wrong text @runs/run-…/deliveries/20261117T103607Z-2d4f65/statements/withdrawn-….json:1
# note: recorded …/statements/withdrawn-….json (a statement); a withdrawn candidate decides no step and the guard never passes it
```

A withdrawal is a statement (`writfold-delivery-withdrawn/1`), one create-new file, bound to the candidate's ID, not its bytes: an edit does not undo it. Prepare again to go on. It is refused (exit 3) for a candidate a done step is bound to (`… is bound to ship#1/comment [done]; done is history`), for one already withdrawn, and without `--reason` or `--by`.

A withdrawn candidate decides no step, is never passed by the guard, and is not counted by the guard's lenient rule. `deliver status` leaves it out unless it is named or `--all` is given (a note says how many are hidden), and it is not counted in the standing counts or the exit code; the trailer counts it as `withdrawn`. `deliver check`, `attest`, `waive` and `posted` on it are refused.

## The payload: `rendering` and `deliver payload`

The payload is the bytes posted. With `rendering = "verbatim"` (the default) it is the candidate. With `rendering = "strip-citations"` it is the candidate with every complete, one-line citation token removed, together with one space or tab directly before it; nothing else changes and no line is added or removed, so line numbers hold. A citation token then proves a claim to `claims.cited-fresh` without reaching the outside reader. There is no redaction, no link rewriting and no template.

- `deliver check` writes `payload.md` beside the candidate when the rendering is not `verbatim`. `paragraphs.bounded`, `references.denied` and script gates read the payload; `claims.cited-fresh` reads the candidate's tokens.
- In a mode gate's `argv`, `{candidate}` is the payload file (`payload.md` under `strip-citations`) and `{source}` is `candidate.md`. `{source}` is refused outside a candidate gate.
- A rendering other than `verbatim` joins every obligation's digest, so changing it makes earlier receipts and statements stale.
- The candidate row gains `payload=<12 hex>` when the payload differs from `candidate=`, and every path a `next:` line or a denial offers for posting is the payload's.
- `deliver payload [CANDIDATE] [--mode M] [--to FILE]` prints the payload of the present candidate on stdout, or writes it to `FILE`. It writes nothing to the KB.
- The guard, `deliver match` and `deliver posted` compare with the payload computed from the present candidate, never with the stored `payload.md`. Bytes equal to a `payload.md` older than the candidate are denied with `payload.md is older than the candidate: writfold kb deliver check <id>`.

## The guard

An optional Claude Code `PreToolUse` hook, `writfold kb hook pre-tool-use`. It is off unless this file exists in the KB:

```toml
# .writfold/deliveries/guard.toml
schema = "writfold-guard/1"
commands = ["gh issue comment"]          # the lenient rule (below)
tools = []
routes = [
  "gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F",
  "mcp__slack__post_message: mode=slack-summary field=text",
]
```

### Routes

A route is `<command words or tool name>: key=value …`. `mode` (optional) names a delivery mode, which must load (`ref.undefined` otherwise). `body` lists flags whose value is the text; `file` lists flags whose value is a path; `field` names the key of a tool's input that holds the text, and makes the route a tool route. A long flag is also read as `--flag=value`. `field` with `body` or `file`, a route with none of the three, and a name also listed under `commands` or `tools` are `value.conflict`; a key other than these is `value.form`; each is reported at its line and column. A `guard.toml` without `routes` reads as before. The init example ([below](#an-example-to-start-from-kb-init---example-delivery)) routes four posts:

```text
gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F
git commit: mode=commit-message body=-m,--message file=-F,--file
gh release create: mode=release-note body=--notes,-n file=--notes-file,-F
mcp__slack__post_message: mode=slack-summary field=text
```

Under a route, the call passes exactly when a candidate of the run is **postable**: its payload is the bytes being posted (trailing whitespace ignored), it is ready, it is not withdrawn, it is of the route's mode, and no post record about its present bytes exists. A route without `mode` takes a candidate of any mode.

A body the guard **cannot read** is denied under a route. It is unread when: no listed flag is present; a listed flag appears more than once; a flag has no value; the value holds a shell expansion; a file value is `-` (standard input) or cannot be read; the tool input has no string at `field`; or the command acts in another directory and the file is relative. A command acts elsewhere when a `cd` or `pushd` before it in the line, `git -C`, `--work-tree`, `--git-dir` or a `GIT_DIR`/`GIT_WORK_TREE` assignment moves it ([how a command is read](hooks.md#how-a-command-is-read)). A relative body file is then never read from the hook's directory, which the shell would not have posted.

### Listed commands and tools: the lenient rule

A Bash call that begins with a `commands` entry is read by `--body`, `-b`, `--body-file` or `-F`; a `tools` entry is guarded as a whole, its body never read. A listed call keeps the rule it had:

- bytes read: the call passes when a postable candidate of any mode holds them;
- bytes not read (a tool, a shell expansion, no body flag, or a relative file after `cd`): the call passes when the run has at least one live candidate and every live candidate is ready.

Two things changed for listed calls in v0.6.0-dev.4: a withdrawn candidate no longer counts, and a candidate recorded as posted no longer passes a second time.

### Denials

Either way, no current run is a denial (`no current KB run: start one, then writfold kb deliver prepare --mode M --from FILE`). Each denial names the repair. The reasons, after the fixed prefix `writfold delivery guard (a guard, not enforcement): `:

| Case | Reason |
|---|---|
| unread under a route, a ready candidate of the mode exists | `cannot read what gh pr comment would post (the body holds a shell expansion). Post the checked file itself: --body-file runs/run-…/deliveries/<id>/candidate.md` |
| unread under a route, none ready | the same first sentence, then `No pr-comment candidate of run <run> is ready: writfold kb deliver status` |
| unread, a listed command or tool | `candidate <id> owes tone.reviewed [open] (tool mcp__x is guarded as a whole, so every live candidate of the run must be ready)` |
| bytes held by no candidate, a ready one exists | `these bytes (sha256:5f93…) are not a candidate's. The ready pr-comment candidate is runs/run-…/deliveries/<id>/candidate.md (sha256:d367…); post that file, or prepare these bytes: writfold kb deliver prepare --mode pr-comment --from FILE` |
| bytes held by no candidate, none ready | `no delivery candidate of run <run> holds the bytes being posted (sha256:5f93…); prepare and check one: writfold kb deliver prepare --mode pr-comment --from FILE` |
| same bytes, wrong mode | `candidate <id> holds these bytes but is a slack-summary candidate; gh pr comment posts pr-comment` |
| same bytes, not ready | `candidate <id> owes references.denied [failed]; next: edit … then: writfold kb deliver check <id>` |
| same bytes, recorded as posted | `candidate <id> was recorded as posted (by agent: https://…/pull/42#issuecomment-1). To post these bytes again, prepare a new candidate` |
| same bytes, withdrawn | `candidate <id> was withdrawn (by agent: pasted the wrong text)` |
| a stale `payload.md` | `payload.md is older than the candidate: writfold kb deliver check <id>` |

As Claude Code receives one:

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"writfold delivery guard (a guard, not enforcement): cannot read what gh pr comment would post (the body holds a shell expansion). Post the checked file itself: --body-file runs/run-…/deliveries/20261117T103623Z-95bb2e/candidate.md"}}
```

When it lets a call through it prints nothing, so Claude Code's own permission prompt still applies. It is a guard, not enforcement: it sees only the calls the harness shows it, and it passes any command or tool it is not configured for. A missing KB, an unreadable configuration or a timeout lets the call proceed. Once a call matches, though, a KB it cannot read denies the call ("cannot read the KB") rather than letting it through. The same hook also evaluates [guard rules](hooks.md#guard-rules), after this guard; a denial here wins.

A candidate is ready when its mode declares at least one check, gate or attestation and every one is met under its policy. A mode that declares nothing is never ready: its candidate reads `[blocked]` with `why: mode <name> declares nothing to check; a delivery is not met by an empty set: …`, `deliver status` exits 2, and the guard refuses its bytes. Readiness is one proved kernel function for `deliver status`, the guard and the plan (`writfold_core::delivery::ready`, [assurance](assurance.md#proved)), and so is the guard's decision over the run's candidates (`writfold_core::delivery::guard`). One body per call is checked: a PR's title and body are not checked together.

## Recording a post: `deliver posted`

Writfold sees no post. After posting, record it:

```text
$ writfold kb deliver posted --ref "https://github.com/acme/harbor/pull/42#issuecomment-1" --by agent
delivery 20261117T103623Z-95bb2e [ready] pr-comment audience=external destination=harbor#42 … !posted
  …
  posted: by agent: https://github.com/acme/harbor/pull/42#issuecomment-1 (a statement; Writfold sent nothing and saw no post) @runs/run-…/deliveries/20261117T103623Z-95bb2e/posts/post-….json:1
# note: recorded runs/run-…/posts/post-….json (a statement; Writfold sent nothing and saw no post)
```

- `deliver posted [CANDIDATE] [--mode M] --ref TEXT --by WHO`. `--ref` is one line of at most 2000 characters, recorded as given and never fetched. The record (`writfold-delivery-post/1`, one create-new file under `posts/`) holds the run, the candidate ID and its content digest, the payload digest, `ref`, `by`, `recorded`, a `seq`, and whether the candidate was ready when found. Several records may exist.
- A record is about the present exactly when its content digest is the candidate's present one. After an edit it reads `posted: [stale] by …: … (the candidate was edited after this record)` and counts for nothing.
- A record made while the candidate was not ready still counts as a post, since it says what happened; its line adds `(recorded while the candidate was not ready)` and the flag `!unready-when-recorded`.
- A posted candidate is spent: the guard does not pass its bytes again. Posting the same text elsewhere is a new candidate with its own destination.
- Refused (exit 3): a withdrawn candidate; no `--ref` or `--by`.

It is a statement: `--by` is text, and a session that stops between the post and the record leaves no trace of the post.

With the delivery guard configured, the plugin's `PostToolUse` hook (built in v0.6.0-dev.11) narrows that gap: after a `Bash` or MCP call whose bytes are a ready, unposted candidate's payload, it adds one line to the agent's context, `writfold: if gh pr comment posted it, record the post: writfold kb deliver posted <id> --ref URL --by WHO`. It is a reminder, not a record; it cannot tell whether the call posted anything.

## Outside the hook: `deliver match`

`deliver match --from FILE|- [--mode M] [--run ID]` asks the guard's question of some bytes: are they the payload of a postable candidate of mode `M` (any mode without `--mode`)? It is read-only and runs no check. It exits 0 exactly when the guard would pass these bytes under a route for `M`, except that a candidate recorded as posted still matches (the text has been posted; that is what is being confirmed) and shows its `posted:` line. Otherwise it exits 2 with the guard's reason as a note.

```text
$ git log -1 --format=%B | writfold kb deliver match --from - --mode commit-message
delivery 20261117T103528Z-25dde5 [ready] commit-message … @runs/run-…/deliveries/20261117T103528Z-25dde5/candidate.md:1
  match: these bytes are this candidate's payload (sha256:d3673c62c45d…)
# coverage=complete … matched=1 postable=1
```

A Git `commit-msg` hook asks the same question for every commit made in the repository, with or without Claude Code:

```sh
#!/bin/sh
# .git/hooks/commit-msg
exec writfold kb deliver match --mode commit-message --from "$1"
```

Git strips comment lines and trailing blank lines before the hook runs; a message whose bytes then differ from the payload in more than trailing whitespace does not match. CI can run the same command over a PR body, and an agent without the hook can run it before posting.

## On a plan step

> Built in v0.6.0-dev.1; withdrawing, the rule for older drafts and `posted:<mode>` built in v0.6.0-dev.4.

A plan step can owe a delivery: the word `deliver:<mode>` in the step string (`"comment: deliver:pr-comment"`). The step cannot be advanced until a candidate of that mode, prepared in the run, is ready, exactly as `deliver status` says `[ready]`. The [run graph guide](run-graph.md#a-delivery-as-a-steps-gate) shows the plan's side; this is the delivery's side.

- **Which candidate decides.** For the current step: scanning the run's candidates of the mode from the newest, withdrawn ones are passed over; the first other one decides if no done step is bound to it, and nothing decides if one is. So once a step has bound a candidate, every older draft of its mode decides no step, and the next step of that mode needs a newer candidate. Newest is by the manifest's `seq`, then by ID, never by a clock. A candidate prepared before the step became current counts: drafting the comment during `implement` is ordinary work.
- **An older draft.** An unbound, live candidate older than a bound one of its mode keeps its row and gains `step: decides no step: 20261117T103623Z-95bb2e, a newer pr-comment candidate, is bound to ship#1/comment [done]`. It is not withdrawn; the guard treats it as any other candidate.
- **`seq`.** `deliver prepare` writes `"seq": N` in `delivery.json`, one more than the highest among the run's candidates of that mode. A manifest written before v0.6.0-dev.1 has none, counts as 0 and keeps ID order.
- **Which check receipt decides.** `deliver check` writes `"seq": N` in each receipt, one more than the highest among the candidate's receipts of that check; the newest is by `seq`, then by file name, never by the receipt's `recorded` time (built in v0.6.0-dev.4). A receipt written before carries none, reads as 0 and comes first, in name order.
- **Binding.** `plan advance` writes the candidate's path and digest into the done record. That candidate is bound to the step for ever: the step is judged on it alone, it never decides another step, and it cannot be withdrawn. A newer candidate never turns a done step stale; an edit to the bound candidate, or a change to its mode, deny list or a mode gate, does (`!stale`), and the current step does not move back.
- **`deliver status`** adds one line under a candidate a step is judged on: `step: decides deliver:pr-comment at change#1/deliver [current]`, `step: bound to change#1/deliver [done]`, or the `decides no step` line above.
- **Not built:** `deliver prepare --step`, a step window for candidates, waiving a delivery as a whole (waive a check the mode takes a waiver of, or close the step with `plan skip --reason --by`), and `posted:<mode>` on a later step than its `deliver:<mode>`.

### Holding the step until the post is recorded: `posted:<mode>`

`posted:<mode>` may stand beside `deliver:<mode>` on the same step, and only there (`plan template: step comment: posted:pr-comment needs deliver:pr-comment on the same step`):

```toml
schema = "writfold-plan/1"
name = "ship"
steps = [
  "implement:",
  "comment: deliver:pr-comment posted:pr-comment",
]
```

It is judged on the step's deciding (or bound) candidate: `[open]` until a post record about that candidate's present bytes exists, then `[attested]`, never `[satisfied]`. The order on the step is: `deliver prepare`, `deliver check` (and `attest` what the mode asks for), post the payload file, `deliver posted`, then `plan advance`.

```text
step ship#1/comment [current] owes: posted:pr-comment [open] @runs/run-…/plan/splice-ship.1.json:1
  candidate: 20261117T103623Z-95bb2e [ready] @runs/run-…/deliveries/20261117T103623Z-95bb2e/candidate.md:1
  next: post runs/run-…/deliveries/20261117T103623Z-95bb2e/candidate.md to harbor#42, then: writfold kb deliver posted 20261117T103623Z-95bb2e --ref URL --by WHO
```

The done record gains an entry with `gate` `posted:pr-comment`, `standing` `attested`, `evidence` the post record's path and `content` the candidate's digest; an edit to the candidate after the advance reads the step `[done] !stale`. `resume` and the session-start context carry the candidate's `posted:` line, so a cold session reads "posted, by whom, where" or its absence before it acts. Without `posted:` on the step, a ready delivery still reads `next: writfold kb plan advance`, after one line: `note: nothing on this step records a post; add posted:pr-comment to the template to hold the step until one is recorded`.

## Cited evidence that moved

> Built in v0.6.0-dev.1.

A `claims.cited-fresh` receipt is bound to the candidate's bytes. A passed receipt also stands only while every citation token in the candidate is still fresh: on every read (`deliver status`, `resume`, the delivery guard and the plan) the tokens are run through `stale` again, locally, with nothing fetched, the same call `deliver check` makes. When any is no longer fresh the check reads `stale`, its line ends `; cited evidence moved: <n> of <m> citation tokens no longer fresh`, and the candidate is `[blocked]` until `deliver check` runs again on evidence that holds. When the evidence comes back, the receipt stands again. A waiver on the check still stands if the mode takes it: it is bound to the bytes and the check's definition, not to the receipt. A candidate with no citation token costs nothing more.

Limits: a mode gate of class `observed` is not re-run on read and does not age (`ages_as` is for repository gates), and a claim made in prose without a citation token is not seen.

## Your own modes

Any `.writfold/deliveries/<name>.toml` defines a mode. The two built-in modes (`pr-comment`, `slack-summary`) stay as defaults. A mode lists built-in `checks`, and `gates`: script gates run against the payload through `{candidate}`, and attested gates whose instructions are shown. Unknown keys, unknown gates and repository gates are refused when the mode is loaded.

```toml
# .writfold/deliveries/commit-message.toml
schema = "writfold-delivery-mode/1"
mode = "commit-message"
description = "checked before git commit; the guard binds the checked bytes under its route, and a commit made outside the hook is not seen"
audience = "external"
checks = ["references.denied"]
gates = ["commit-format", "language-guidelines"]
waivable = []
```

```sh
writfold kb deliver modes commit-message   # what it asks for
writfold kb deliver prepare --mode commit-message --from MSG
writfold kb deliver check                  # references.denied and commit-format
writfold kb deliver attest --check language-guidelines --statement TEXT --by WHO
git commit -F runs/<run>/deliveries/<id>/candidate.md
writfold kb deliver posted --ref "$(git rev-parse HEAD)" --by WHO
```

**What binds the bytes to the commit.** With a `git commit` route in `guard.toml`, the guard reads the message from `-F`/`--file` or `-m`/`--message` and passes the commit only when it is the payload of a postable `commit-message` candidate; a message it cannot read (`-m "$(cat …)"`, no flag, the editor) is denied, naming the file to commit with. A commit made outside the hook (another terminal, CI, an agent without the hook) is not seen; a Git `commit-msg` hook running [`deliver match`](#outside-the-hook-deliver-match) covers those. The mode's description, printed with every status, says so: `checked before git commit; the guard binds the checked bytes under its route, and a commit made outside the hook is not seen` (`fixtures/kb/commit-message/`).

## An example to start from: `kb init --example delivery`

`writfold kb init --example delivery` writes, create-new only, the files of [`fixtures/kb/delivery-example`](../../fixtures/kb/delivery-example/.writfold): `deny-external.toml` (two example rules), `deny-internal.toml` (`rules = []`), `commit-message.toml` (audience `external`, `references.denied` and the `commit-format` gate, `waivable = []`), `release-note.toml` (all three checks, an attested `notes.reviewed`, `waivable = []`, `rendering = "strip-citations"`), `guard.toml` with the four routes above and, in a comment, the permission rule that makes Claude Code ask before `deliver waive` and `deliver withdraw`, the `commit-format` gate with its script and `accept = ["satisfied"]`, and a plan template `ship` whose last step is `"comment: deliver:pr-comment posted:pr-comment"`. Each file says in a comment that it is an example to edit; the built-in modes are unchanged. A test runs the commit-message flow on these bytes.

## Behaviour changes in v0.6.0-dev.4

- A waiver a mode gate's `accept` or a mode's `waivable` rejects no longer makes a candidate ready: it reads `[waived] !not-accepted` with a `why:` line, the candidate `[blocked]`, exit 2; `deliver waive` refuses to write one, exit 3. Before, `accept` on a candidate gate was ignored for a delivery.
- An older draft decides no step once a newer candidate of its mode is bound; it gains `step: decides no step: …`. A run that drafted two candidates ahead of two steps, expecting the older to meet the second, now finds that step `[open]` with `next: … deliver prepare`.
- A withdrawn candidate is not counted by the guard's lenient rule, the trailer's standing counts or the exit code.
- A candidate recorded as posted no longer passes the guard a second time.
- The denial for unknown bytes, when a ready candidate exists, names that candidate's file.
- `next:` after a failed check names the repair: `edit <path> (line N: rules), then: writfold kb deliver check <id>`.
- A ready delivery whose step has no `posted:` word adds `note: nothing on this step records a post; add posted:<mode> to the template …` before `next: writfold kb plan advance`.
- `deliver status`'s trailer gains `withdrawn=`. The JSON gains keys ([json](json.md#keys-only-some-verbs-add)); `writfold_kb` stays 1.
