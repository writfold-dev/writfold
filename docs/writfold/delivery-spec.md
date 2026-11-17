# Delivery modes, second round: specification

> **Status:** built in v0.6.0-dev.4 above the cut line (steps 1 to 8, D1 to D13, with D14's skill text and the guides); D18 and D19, below the cut line, are not built ([design §47](kb-v1-design.md#47-delivery-modes-second-round-as-built-17-november-2026-v060-dev4)). Proposed 17 November 2026, written against `main` at `83d5ded` (v0.6.0-dev.3); sections 1 to 4 describe the state before this round. Every description of today's behaviour was run with the v0.6.0-dev.3 release binary on a scratch KB (section 1), or read from the code at that commit. The authority is [mission.md](mission.md). This document stands beside [deliver-gate-spec.md](deliver-gate-spec.md) (a delivery as a step's gate, built) and [policy-spec.md](policy-spec.md) (whose delivery `waivable` is specified here in full and built by this round).

A delivery mode is how an agent prepares outward text (a PR comment, a Slack summary, a commit message, a release note) and has it checked before anyone posts it. Writfold never sends anything: no network code, no adapter. That does not change. What changes is how well an agent that has never seen this repository can get from a draft to posted text without a wrong answer on the way, and what a cold session can tell afterwards.

## 1. What an agent meets today

One run, one scratch KB (`writfold kb init`, one checkpoint), the four deliveries an agent makes most: a PR comment, a Slack summary, a commit message and a mode of the team's own. Paths are shortened to `runs/run-…`; nothing else in the quoted output is edited.

### 1.1 Wrong answers

**W1. A mode gate's `accept` is ignored for a delivery.** `commit-format.toml` says `accept = ["satisfied"]`. The check fails, `deliver waive` records the waiver, the candidate reads `[ready]` and the guard lets `git commit -F` through with an 88-character subject:

```text
  check: commit-format [failed] mechanical fail for 69cdd8d14dc6: line 1 commit-format (subject has 88 characters; at most 72); line 2 commit-format (the second line must be blank) …
$ writfold kb deliver waive 20261117T103915Z-4d85ce --check commit-format --reason "meh" --by agent
  check: commit-format [waived] mechanical by agent: meh (check: failed) @runs/run-…/statements/commit-format-….json:1 !waived
$ writfold kb deliver attest … --check language-guidelines --statement ok --by agent
delivery 20261117T103915Z-4d85ce [ready] commit-message audience=external …
```

The same gate named on a plan step would refuse the waiver. One word, two rules.

**W2. Any check can be waived by whoever runs the command.** A 170-word paragraph against a bound of 150, and a deny-list failure, are each one command from `[ready]`; `--by` is text. `waivable` ([policy-spec.md](policy-spec.md) section 2.2) is the answer and is not built.

```text
$ writfold kb deliver waive … --check paragraphs.bounded --reason "I like long paragraphs" --by agent
  check: paragraphs.bounded [waived] mechanical by agent: I like long paragraphs (check: failed) … !waived
```

**W3. An abandoned draft meets a later step.** The newest candidate that no done step is bound to decides an open step. So once a step has bound the candidate that was meant, every older draft of that mode is still in line. Here `20261117T103607Z-2d4f65` is a draft that holds a pasted error message; a better candidate was prepared, checked and bound to `ship#1/comment`; the next step of the same mode is then met at once, by the abandoned draft:

```text
step ship#1/announce/two#1/first [current] met: deliver:pr-comment [satisfied] @runs/run-…/plan/splice-ship.1~announce~two.1.json:1
  candidate: 20261117T103607Z-2d4f65 [ready] @runs/run-…/deliveries/20261117T103607Z-2d4f65/candidate.md:1
  next: writfold kb plan advance
```

The same happens to a step whose mode has only a scratch draft from earlier in the run: `next:` tells the agent to attest tone on a file that says `short note`. A candidate cannot be withdrawn.

**W4. The guard does not tie the command to the mode.** A ready `pr-comment` candidate lets `git commit -F` through, and a ready candidate of a team's weakest mode lets `gh pr comment` through. The guard asks only whether some ready candidate holds the bytes.

**W5. When the guard cannot read the body, it checks nothing about the bytes.** The rule is "every candidate of the run is ready". With that true, anything is passed:

```text
$ # tool mcp__slack__post_message, text "anything at all, OPS-9 on ledger-01.corp.example.com"
[allowed]
$ gh pr comment 42 --body "$(cat /etc/hostname) OPS-9"
[allowed]
```

With it false, nothing is, and the blocker may be an unrelated draft that can never be removed:

```text
writfold delivery guard (a guard, not enforcement): candidate 20261117T103321Z-fbea34 owes tone.reviewed [open]; candidate 20261117T103516Z-336665 owes references.denied [unknown], commit-format [failed], language-guidelines [open] (no body flag the guard can read, so every candidate of the run must be ready)
```

The body is unreadable more often than not: an MCP tool's text is never read from its input (`tool … is guarded as a whole`), `git commit -m`, `gh release create --notes-file` and `--body-file -` have "no body flag the guard can read", and the form agents use most, `--body "$(cat <<'EOF' … EOF)"`, "holds a shell expansion".

**W6. The body file is read from the hook's directory, whatever the command did first.** `cd /tmp && gh pr comment 42 -F runs/…/candidate.md` is allowed on the file under the hook's working directory; the shell would post the file under `/tmp`.

**W7. A citation token is part of the payload.** `claims.cited-fresh` needs tokens in the candidate, the candidate is "the exact payload", and the guard passes only those bytes. A PR comment that cites one line therefore carries this, local path included, to the outside reader:

```text
The entry point prints one line ⟦cite kind=code repo=kb rev=f4d4c6a87019b9339e094ad3d148544ea93e9863 path=src/main.rs L=1-1 blob=5681f3509cb6b3e1e1dbacfa923ccf43abd08096 span=55559a7adc9d38cd seen=2026-11-17T10:36Z url=/tmp/…/deliver2/kb⟧.
```

An agent either posts that, or strips the tokens and loses the check (and the guard, which no longer finds the bytes). The one check that ties outward text to evidence cannot be used on outward text.

**W8. `next:` sends the agent round a loop.** After a failed or unknown check the plan says to run the check again, which cannot change the outcome:

```text
step ship#1/commit [current] owes: deliver:commit-message [unknown] …
  candidate: 20261117T103516Z-336665 [blocked] owes references.denied [unknown], commit-format [failed], language-guidelines [open] …
  next: writfold kb deliver check 20261117T103516Z-336665
```

**W9. A ready delivery reads as "advance".** Once the candidate is ready the step says `met:` and `next: writfold kb plan advance`. Nothing says the text is still to be posted, and nothing records that it was. A session that stops between the post and the advance leaves the next session no way to tell which side of the post it is on: it either posts twice or never.

### 1.2 Friction

| # | What happens | Output |
|---|---|---|
| F1 | A failing line is named by number and rule only: no source line, no carets, no rule text, no fix. The agent opens the receipt, then the deny list, then the candidate | `references.denied [failed] mechanical fail for b9a6e771b3fe: line 5 internal-host (column 25); line 5 ticket (column 48)` |
| F2 | The built-in modes are never ready in a new KB: no deny list gives `unknown`. The row names the file to add, which is the team's policy, and the agent writes it itself or waives | `references.denied [unknown] mechanical unknown for … (no deny list for audience internal: add .writfold/deliveries/deny-internal.toml)` |
| F3 | `deliver check` with no ID refuses as soon as the run holds two candidates, although one of them decides the current step. The list has no mode and no state | `refused: 3 candidates; name one: 20261117T103311Z-e948b4, 20261117T103321Z-fbea34, 20261117T103516Z-336665` |
| F4 | The audience is chosen at `prepare` and defaults to `internal`. A commit message for a public repository prepared without `--audience external` is checked against the wrong list (here: none). A mode cannot say which audience it is for; the fix is a second candidate | `commit-message audience=internal … references.denied [unknown]` |
| F5 | Nothing lists the modes or what one will ask for before the draft is written. `kb policy` counts them (`modes=4`) and prints none; a mistyped mode gets no suggestion and only the built-in names | `refused: no delivery mode pr-coment: add .writfold/deliveries/pr-coment.toml (…), or use a built-in mode: pr-comment, slack-summary` |
| F6 | An attestation is accepted while a mechanical check of the same candidate fails; the repair then makes it stale and it is made twice | `language-guidelines [attested]` beside `references.denied [failed]`, then `language-guidelines [stale]` |
| F7 | `deliver status` for the run exits 2 for ever once one draft is abandoned, and pages after three candidates | `shown=3 omitted=6 omitted_by=deliveries:6 candidates=9 ready=5 …` |
| F8 | A guard denial for unknown bytes does not say where the checked bytes are. The agent posted `-F MSG`; the candidate was repaired in place, so `MSG` is the old text | `no delivery candidate of run … holds the bytes being posted (sha256:5f93…); prepare and check one with writfold kb deliver` |
| F9 | The commit-message example prints "nothing binds the checked bytes to the commit that is made" on every status, while the guard does bind them for `git commit -F`, and nothing offers a check outside a Claude Code hook (CI, a `commit-msg` hook, another agent) | `# note: mode commit-message: checked before git commit; nothing binds …` |
| F10 | JSON carries mode, audience, destination, digest and each finding only inside `text` | `"text": "references.denied [failed] mechanical fail for a9fb9d965673: line 1 internal-host (column 15); line 1 ticket (column 5)"` |
| F11 | `--from -` is read as a file name; the draft must be written to a file first | `--from -: No such file or directory (os error 2)` |
| F12 | `!edited` stays on a candidate for ever after one in-place repair, also when it is `[ready]`; it means "differs from the file it was prepared from" and reads as "edited since the check" | `delivery … [ready] … !edited` |
| F13 | `claims.cited-fresh` passes on a draft with no token (`0 citation tokens`); a mode cannot ask for at least one | |
| F14 | The only length bound is words per paragraph. A Slack summary of forty short paragraphs passes | |
| F15 | The skill text agents read describes prepare, check and status only: nothing on which file to post from, on attesting last, on waivers being the owner's, or on recording a post | |

### 1.3 What works and stays

Receipts bound to bytes; statements shown as statements; the re-check of cited evidence on read; one readiness rule for `deliver status`, the guard and the plan; an empty mode never ready; binding of a done step to its candidate; an explicit empty deny list (`rules = []`) passing as `0 rules for internal`, which is the team saying nothing is denied; `deliver attest` refusing a mechanical check; refusals that name the next command when nothing is prepared.

## 2. Triage

Adopt only what any agent, team or CI job would use. Writfold never sends.

| # | Item | Decision | Reason |
|---|---|---|---|
| D1 | A mode gate's `accept` and a mode's `waivable` decide readiness (W1, W2) | **Adopt** | One meaning of a waiver across gates and deliveries; the unbuilt half of the policy round |
| D2 | Withdraw a candidate; an older draft decides no step once a newer one of its mode is bound (W3, F7) | **Adopt** | The plan must not be met by text nobody meant. Withdrawal is a statement with who and why, like a skip |
| D3 | Guard routes: a posting command or tool names its mode and where its body is; an unread body under a route is denied; the any/all over candidates moves into the kernel (W4, W5, W6, F8) | **Adopt** | This is what "the guard binds the exact bytes" has to mean. The lenient lists stay for compatibility |
| D4 | `deliver match`: is this text a ready candidate's payload? (F9) | **Adopt** | The guard's question as a verb, for CI, a Git `commit-msg` hook and agents without the hook. Read-only |
| D5 | A post record (`deliver posted`), and a step word `posted:<mode>` (W9) | **Adopt** | "Resumes where it stopped" needs to know which side of the post a run is on. It is a statement: Writfold saw nothing |
| D6 | A posted candidate is spent: the guard does not pass it again | **Adopt** | The record's use at the point where a double post happens. Posting the same text elsewhere is a new candidate |
| D7 | Rendering `strip-citations`, and `deliver payload` (W7) | **Adopt, this one rendering only** | Without it `claims.cited-fresh` and exact-bytes posting exclude each other. No redaction, no templates, no rewriting |
| D8 | Findings drawn against the candidate like file diagnostics; `next:` per obligation that names the repair; additive JSON (F1, F10, W8) | **Adopt** | How an agent learns what failed and what to do, in the style of [errors](../guide/errors.md) |
| D9 | `deliver modes`, mode rows in `kb policy`, a suggestion for a mistyped mode (F5) | **Adopt** | An agent reads the bound, the deny rule names and the instructions before it writes |
| D10 | A mode's `audience` key (F4) | **Adopt** | The audience of a commit message is a property of the repository, not of one command line |
| D11 | Which candidate a verb means without an ID; `--from -` (F3, F11) | **Adopt** | Small, and each one is a refusal agents hit on their second draft |
| D12 | `kb init --example delivery` (F2) | **Adopt** | The examples are the team's to edit; the binary gains no opinion and the built-in modes keep their digests |
| D13 | A note when attesting or waiving ahead of a failing mechanical check (F6) | **Adopt** | One line; saves a statement made twice |
| D14 | The skill text (F15) | **Adopt** | It is the only delivery documentation most agents read |
| D15 | `commit-message` or `release-note` as built-in modes | **Alternative: the example of D12** | Their format rules are script gates and a team's taste. A built-in mode would be an opinion in the binary |
| D16 | Changing the built-in modes' defaults (audience `external`, a length check) | **Decline** | It changes their definition digests: every receipt and every done delivery step of existing runs would read stale |
| D17 | Renaming `!edited` (F12) | **Decline; explain it** | A flag is a JSON value scripts match. The guide and `deliver status` say what it means |
| D18 | `length.bounded` (total characters and lines) as a fourth built-in check (F14) | **Defer (below the cut line)** | Generalizes, but a script gate does it today |
| D19 | A reminder after a guarded call passes (a `PostToolUse` hook line naming `deliver posted`) | **Defer (below the cut line)** | Useful, harness-specific, and D5 does not depend on it |
| D20 | `citations_min` on a mode (F13) | **Defer** | A script gate can count tokens; no second team has asked |
| D21 | Audiences beyond `internal` and `external` | **Defer** | No evidence yet that two do not fit; the file layout (`deny-<audience>.toml`) already allows it |
| D22 | Several checked parts in one call (a PR's title and body) | **Defer** | One body per call covers the four deliveries walked. The limit is documented |
| D23 | The hook writing the post record itself | **Decline** | It would write `--by` for someone, and "the tool call returned" is not "it was posted" |
| D24 | Sending, destination adapters, URL validation, reading the posted text back | **Decline** | Writfold never sends and never fetches |
| D25 | Judging tone, truth or meaning | **Decline** | Attested gates with instructions carry that, as statements |

## 3. Theorems first

D1, D2, D3, D5 and D6 change decisions. `writfold_core::delivery` is proved ([assurance](../guide/assurance.md#proved)); its theorems are replaced in place, the proofs gate passes (drift check, `lake build`, axiom audit, mutants), and only then is anything wired into a verb.

### 3.1 The kernel after this round

```rust
// crates/writfold-core/src/delivery.rs (no IO, alloc only)
pub struct Owed { pub standing: Standing, pub accept: Accept }       // one obligation, with what its policy takes

pub fn unmet(o: &Owed) -> bool;                  // !policy::met(o.standing, &o.accept)
pub fn first_unmet(os: &[Owed]) -> Option<usize>;
pub fn standing(os: &[Owed]) -> Standing;        // empty: Checked(Open); else the first unmet obligation's
                                                 // standing; else Waived, Attested, Checked(Satisfied)
pub fn ready(os: &[Owed]) -> bool;               // !os.is_empty() && first_unmet(os).is_none()
pub fn refused(os: &[Owed]) -> bool;             // the first unmet obligation stands on a met word its policy rejects
pub fn fact(os: &[Owed]) -> GateFact;            // Stands(standing(os), every word if ready(os), none otherwise)

pub enum Place { Open, Bound, Withdrawn }        // one candidate of the mode, oldest first by (seq, id)
pub fn deciding(places: &[Place]) -> Option<usize>;

pub fn posted(recorded: bool) -> Standing;       // Attested if recorded, else Checked(Open)

pub enum Body { Known, Unknown }
pub enum Unread { AllReady, Deny }               // a listed command or tool; a route
pub struct Seen { pub same: bool, pub ready: bool, pub live: bool, pub routed: bool, pub spent: bool }
pub fn postable(c: &Seen) -> bool;               // same && ready && live && routed && !spent
pub fn guard(body: Body, unread: Unread, cands: &[Seen]) -> bool;
```

`is_waived`, `is_attested` and `rechecked` are unchanged. `ready` is no longer `standing(os).met()`: a waiver the policy rejects is the first unmet obligation, its word is `waived`, and a step must not read that word as met. `fact` is what the plan is given, so the proved `plan::fact_met` and `done_only_when_met` hold of a delivery with no change to the plan kernel.

`deciding` scans from the newest candidate, passes over withdrawn ones, and answers the first other candidate if it is open and nothing if it is bound.

`guard`: known bytes pass exactly when some candidate is postable. Unknown bytes never pass under `Deny`; under `AllReady` they pass when at least one candidate is live and every live candidate is ready (today's rule, with withdrawn candidates left out). `live` is "not withdrawn"; `routed` is "of the route's mode", true when the call matched a plain list entry; `spent` is "has a post record about its present bytes".

### 3.2 Theorems

Replaced in place (same names, new statements), in `proofs/Writfold/Delivery.lean`:

| Theorem | Statement after this round |
|---|---|
| `unmet_spec` | `unmet o = !Policy.metB o.standing o.accept` |
| `standing_loop_spec`, `standing_spec` | the result is `Checked(Open)` for `[]`; else the standing of the first `o` with `unmet o`; else `Waived` if any is waived, else `Attested` if any is attested, else `Checked(Satisfied)` |
| `standing_satisfied_iff` | `standing os = Checked(Satisfied)` ↔ `os ≠ []` ∧ every `o` is `Checked(Satisfied)` and its policy takes `satisfied` |
| `standing_first_unmet` | if `os[i]` is unmet and no earlier one is, `standing os = os[i].standing` |
| `standing_met_iff` | `ready os = true → Gate.Met (standing os)`; the converse is false and is no longer claimed (W1 is the counterexample) |
| `ready_iff` | `ready os = true ↔ os ≠ [] ∧ ∀ o ∈ os, Policy.Met o.standing o.accept` |
| `empty_never_ready` | unchanged in words, restated over `Owed` |
| `deciding_loop_spec`, `deciding_spec` | `deciding ps = some i ↔ ps[i] = Open ∧ ∀ j > i, ps[j] = Withdrawn`; `none` otherwise |
| `guard_allow_iff` | `guard b u cs = true ↔ (b = Known ∧ ∃ c ∈ cs, Postable c) ∨ (b = Unknown ∧ u = AllReady ∧ (∃ c ∈ cs, c.live) ∧ ∀ c ∈ cs, c.live → c.ready)` |

New, each a required claim of the axiom audit:

| Theorem | Statement |
|---|---|
| `first_unmet_loop_spec`, `first_unmet_spec` | `some i` exactly for the least `i` with `unmet os[i]`; `none` exactly when none is unmet |
| `refused_iff` | `refused os = true ↔ ∃ i, first_unmet os = some i ∧ Gate.Met os[i].standing` |
| `waiver_rejected_never_ready` | if some `o ∈ os` is `Waived` with `o.accept.waived = false`, `ready os = false`, whatever the other obligations are |
| `default_accept_is_before` | when every `o.accept` takes all three words, `ready os` and `standing os` equal the functions they replace, on the standings alone. An absent `accept` or `waivable` changes nothing |
| `accept_only_narrows` | `ready os = true →` `ready` of the same standings under all-three `accept` is true |
| `fact_met_iff_ready` | `plan.fact_met (fact os) = true ↔ ready os = true` |
| `deciding_withdrawn_irrelevant` | inserting or removing `Withdrawn` entries does not change which candidate decides |
| `deciding_none_after_bound` | if some `j` has `ps[j] = Bound` and every later entry is `Withdrawn`, `deciding ps = none`: an older draft never decides once a newer candidate of its mode is bound |
| `posted_spec`, `posted_never_satisfied`, `posted_met_iff` | `posted r` is `Attested` when `r`, `Checked(Open)` otherwise; never `Checked(Satisfied)`; met exactly when `r` |
| `postable_spec` | `postable c = (c.same ∧ c.ready ∧ c.live ∧ c.routed ∧ ¬c.spent)` |
| `guard_any_loop_spec`, `guard_all_loop_spec` | the two scans, for every prefix |
| `guard_deny_unread` | `guard Unknown Deny cs = false` for every `cs` |
| `guard_withdrawn_irrelevant` | adding candidates with `live = false` never changes the decision |
| `guard_spent_never_passes` | if every `c` with `c.same` has `c.spent`, `guard Known u cs = false` |

Unchanged: `is_waived_spec`, `is_attested_spec`, `rechecked_spec`. `done_only_when_met` is unchanged and now covers a delivery through `fact_met_iff_ready`.

### 3.3 Mutants

The nine delivery mutants stay, re-aimed at the functions as they now are (`delivery-empty-met`, `delivery-waived-satisfied`, `delivery-unmet-ignored`, `deciding-ignores-bound`, `deciding-first-unbound`, `ready-accepts-empty`, `rechecked-keeps-satisfied`, `guard-allows-no-candidate`, `guard-known-any-ready`). `deciding-ignores-bound` now reads "continue past a bound candidate", which is today's rule and the cause of W3. New:

| Mutant | Change | Fails |
|---|---|---|
| `delivery-accept-ignored` | `unmet` uses `!standing.met()` | `unmet_spec` |
| `ready-from-standing` | `ready` is `standing(os).met()` | `ready_iff` |
| `refused-any-unmet` | `refused` drops the "met word" test | `refused_iff` |
| `fact-accepts-when-unready` | `fact` gives every word regardless | `fact_met_iff_ready` |
| `deciding-counts-withdrawn` | a withdrawn candidate is treated as open | `deciding_spec` |
| `deciding-stops-at-withdrawn` | a withdrawn candidate ends the scan | `deciding_spec` |
| `posted-always` | `posted` ignores its input | `posted_spec` |
| `posted-satisfied` | a recorded post reads `Checked(Satisfied)` | `posted_never_satisfied` |
| `postable-drops-same`, `-ready`, `-live`, `-routed`, `-spent` | one conjunct removed (five mutants) | `postable_spec` |
| `guard-known-all` | known bytes need every candidate postable | `guard_allow_iff` |
| `guard-unread-deny-allows` | `Deny` falls through to the `AllReady` rule | `guard_deny_unread` |
| `guard-all-counts-withdrawn` | a withdrawn candidate must be ready | `guard_withdrawn_irrelevant` |
| `guard-all-skips-last` | the scan stops one short | `guard_all_loop_spec` |

"Unknown bytes allowed when one candidate is ready", which could not be built as a kernel mutant while `all_ready` was an input, is `guard-all-skips-last` and its neighbours now.

### 3.4 Tested, not proved

Reading `waivable`, `audience`, `rendering` and `routes` from TOML and their diagnostics; splitting a command line and reading a body flag or a tool field (the existing reader, `writfold_core::command`); the rendering itself; digests; which statements and post records are about the present (the callers use the proved `gate::about`); drawing findings; JSON; the files written. No new protocol: `deliver posted` and `deliver withdraw` are each one create-new file under the existing write protocol, so `KbWrite.tla` and `RunGraph.tla` are unchanged and the kill tests gain these two writes.

## 4. Each item

Exit codes are the existing ones: 0, 2 (not met), 3 (refused, nothing written), 1 (usage or IO). Every JSON change is additive; `writfold_kb` stays 1. A mode key joins the mode's digest only when present, so no existing receipt, statement or done step changes standing.

### D1. `accept` and `waivable`

```toml
# .writfold/deliveries/pr-comment.toml
schema = "writfold-delivery-mode/1"
mode = "pr-comment"
checks = ["paragraphs.bounded", "references.denied", "claims.cited-fresh"]
attest = ["tone.reviewed"]
waivable = ["claims.cited-fresh"]      # absent: every obligation, as today; []: none
```

- `waivable` names obligations the mode declares (`ref.undefined` otherwise, with a suggestion). It applies to the mode's `checks` and `attest` names. A mode gate carries its own `accept`; naming a mode gate in `waivable` is refused (`value.conflict`: "set accept in .writfold/gates/<id>.toml").
- Each obligation gets an `Accept`: a built-in check or `attest` name takes `waived` only when waivable; a mode gate takes what its definition's `accept` says.
- `deliver waive --check references.denied` on a mode that rejects it: exit 3, nothing written, `refused: pr-comment does not accept a waiver of references.denied (waivable = ["claims.cited-fresh"] @.writfold/deliveries/pr-comment.toml:6)`; for a mode gate, the gate's own refusal text from `gate waive`.
- A waiver that exists anyway (written before the key, or by hand): the line reads `[waived] … !waived !not-accepted`, the candidate `[blocked]`, with `why: commit-format does not accept waived (accept = ["satisfied"] @.writfold/gates/commit-format.toml:9)`. The same for `deliver status`, the guard, `deliver match` and a `deliver:<mode>` step, whose gate line reads `deliver:commit-message [waived] !not-accepted`.
- JSON: flag `not-accepted` on the check continuation; `waivable` (array) on a mode row of `deliver modes`.
- Compatibility: a KB with `accept` on a candidate gate and a standing waiver of it reads `[blocked]` where it read `[ready]`. That is W1 corrected, and is listed as a behaviour change.
- Tests: W1 as a failing test first; the refusal; the stale-by-policy line; `guide_toml.rs` over the new block. Mutations: dropping the `Accept` at the call site (passing all three) must fail the W1 test; reading `waivable` absent as `[]` must fail an existing waiver test.
- Docs: delivery-modes, policy (remove "planned"), gates (`accept` on a candidate gate), errors (codes used), design as-built.

### D2. Withdrawing, and older drafts

```text
$ writfold kb deliver withdraw 20261117T103607Z-2d4f65 --reason "pasted the wrong text" --by agent
delivery 20261117T103607Z-2d4f65 [withdrawn] pr-comment … @runs/run-…/deliveries/20261117T103607Z-2d4f65/candidate.md:1 !withdrawn
  withdrawn: by agent: pasted the wrong text @runs/run-…/deliveries/20261117T103607Z-2d4f65/statements/withdrawn-….json:1
# note: recorded …/statements/withdrawn-….json (a statement); a withdrawn candidate decides no step and the guard never passes it
```

- The statement is bound to the candidate ID, not its bytes: it cannot be undone by an edit. Prepare again to go on.
- Refused (exit 3): a candidate a done step is bound to (`… is bound to ship#1/comment [done]; done is history`), one already withdrawn, a missing `--reason` or `--by`.
- A withdrawn candidate: status `withdrawn`, flag `withdrawn`; left out of `deliver status` unless named or the new `--all` is given; not counted in the trailer's `ready`…`unknown` counts or the exit code (a new count `withdrawn`); never deciding; never postable; not counted by the `AllReady` rule; `deliver check`, `attest`, `waive` and `posted` on it are refused.
- An unbound candidate older than a bound one of its mode keeps its row and gains `step: decides no step: 20261117T103623Z-95bb2e, a newer pr-comment candidate, is bound to ship#1/comment [done]`. It is not withdrawn; the guard still treats it as any other candidate.
- `deliver prepare` adds a note when the run already has a live, unbound candidate of the mode: `note: 20261117T103311Z-e948b4 is an older pr-comment candidate; this one now decides. Withdraw the older one if it is not meant: writfold kb deliver withdraw 20261117T103311Z-e948b4 --reason TEXT --by WHO`.
- Compatibility: a run that drafted two candidates of one mode ahead of two steps, expecting the older to meet the second step, now finds that step `[open]` with `next: … deliver prepare`. Done steps are unaffected.
- Tests: W3 as a failing test first; the fold from statements to `Place` (a withdrawn statement in another run's directory counts for nothing); kill tests for the write. Mutations: reading a bound candidate as open must fail the W3 test; counting a withdrawn candidate in the exit code must fail a status test.

### D3 and D6. The guard

```toml
# .writfold/deliveries/guard.toml
schema = "writfold-guard/1"
commands = ["gh issue comment"]        # as today
tools = []                             # as today
routes = [
  "gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F",
  "git commit: mode=commit-message body=-m,--message file=-F,--file",
  "gh release create: mode=release-note body=--notes,-n file=--notes-file,-F",
  "mcp__slack__post_message: mode=slack-summary field=text",
]
```

- A route is `<command words or tool name>: key=value …`, the form a deny rule and a plan step already use. `mode` (optional) names a delivery mode that must load (`ref.undefined` otherwise). `body` lists flags whose value is the text; `file` lists flags whose value is a path; `field` names the key of a tool's input that holds the text, and makes the route a tool route. `field` with `body` or `file`, a route with none of the three, and a name that is also under `commands` or `tools` are refused (`value.conflict`), each at its line and column.
- The body under a route is **unread**, and the call is denied, when: no listed flag is present; a listed flag appears twice; the value holds a shell expansion or is `-`; the file cannot be read; the tool input has no string at `field`; or the simple command is preceded in its line by `cd`, `pushd` or an assignment to `GIT_DIR`/`GIT_WORK_TREE`, or carries `-C` (W6). The last rule also applies to plain `commands` entries, where such a body becomes unknown rather than read from the wrong place.
- Known bytes pass exactly when a candidate is postable: same payload bytes (trailing whitespace ignored, as today), ready, not withdrawn, of the route's mode, with no post record about its present bytes.
- Denials name the repair. The texts, after the fixed prefix:

| Case | Reason |
|---|---|
| unread, a ready candidate of the mode exists | `cannot read what gh pr comment would post (the body holds a shell expansion). Post the checked file itself: --body-file runs/run-…/deliveries/<id>/candidate.md` |
| unread, none ready | the same first sentence, then `no pr-comment candidate of run <run> is ready: writfold kb deliver status` |
| bytes held by no candidate, a ready one exists | `these bytes (sha256:5f93…) are not a candidate's. The ready pr-comment candidate is runs/run-…/candidate.md (sha256:d367…); post that file, or prepare these bytes: writfold kb deliver prepare --mode pr-comment --from FILE` |
| same bytes, wrong mode | `candidate <id> holds these bytes but is a slack-summary candidate; gh pr comment posts pr-comment` |
| same bytes, not ready | as today (`candidate <id> owes …`), then `next:` as D8 words it |
| same bytes, spent | `candidate <id> was recorded as posted (by agent: https://…/pull/42#issuecomment-1). To post these bytes again, prepare a new candidate` |
| same bytes, withdrawn | `candidate <id> was withdrawn (by agent: pasted the wrong text)` |

- Compatibility: `commands` and `tools` keep their rule, except that a withdrawn candidate no longer counts and a posted candidate no longer passes a second time. A `guard.toml` without `routes` reads as today; a binary older than this round refuses a file with `routes` by its unknown key and, as today, lets the call proceed.
- Tests: conformance cases under `fixtures/guard-conformance/` for each row above and each unread cause; the commit-message, PR, Slack-tool and release-note routes end to end through the hook entry point; `guide_toml.rs`. Mutations: computing `routed` as true for a route with a mode must fail the wrong-mode case; reading the body file before looking for `cd` must fail the W6 case; dropping `spent` at the call site must fail the spent case.
- Docs: delivery-modes ("The guard"), hooks, the commit-message fixture's description (it no longer says nothing binds the bytes; it says what does and what does not: a commit made outside the hook).

### D4. `deliver match`

```text
$ git log -1 --format=%B | writfold kb deliver match --from - --mode commit-message
delivery 20261117T103528Z-25dde5 [ready] commit-message … @runs/run-…/deliveries/20261117T103528Z-25dde5/candidate.md:1
  match: these bytes are this candidate's payload (sha256:d3673c62c45d…)
# coverage=complete … matched=1 postable=1
```

`deliver match --from FILE|- [--mode M] [--run ID]`. Read-only; no check runs. Exit 0 exactly when the guard would pass these bytes under a route for `M` (any mode when `--mode` is absent), except that a spent candidate matches (the text has been posted; that is what is being confirmed) and carries its `posted:` line. Exit 2 with the guard's reason otherwise. It is the same kernel call. Uses: `.git/hooks/commit-msg` (`writfold kb deliver match --mode commit-message --from "$1"`), CI over a PR body, an agent without the hook. JSON: one `delivery` row or none, counts `matched` and `postable`, `reason` in the trailer. Tests: the answers equal the guard's on the conformance cases' bytes; mutation: answering from `ready` alone must fail the wrong-mode case.

### D5. A post record, and `posted:<mode>`

```text
$ writfold kb deliver posted --ref "https://github.com/acme/harbor/pull/42#issuecomment-1" --by agent
delivery 20261117T103623Z-95bb2e [ready] pr-comment audience=external destination=harbor#42 … !posted
  …
  posted: by agent: https://github.com/acme/harbor/pull/42#issuecomment-1 (a statement; Writfold sent nothing and saw no post) @runs/run-…/deliveries/20261117T103623Z-95bb2e/posts/post-….json:1
```

- `deliver posted [CANDIDATE] --ref TEXT --by WHO`. `--ref` is one line of at most 2000 characters, recorded as given and never fetched. The record (`writfold-delivery-post/1`) holds the candidate ID and content digest, the payload digest, `ref`, `by`, `recorded`, a `seq`, whether the candidate was ready as found, and its own digest. Several records may exist. Refused (exit 3): a withdrawn candidate; no `--ref` or `--by`.
- A record is about the present exactly when its content digest is the candidate's present one (the proved `gate::about`). After an edit it reads `posted: [stale] … (the candidate was edited after this record)` and counts for nothing. A record made while the candidate was not ready keeps `!unready-when-recorded` and still counts as a post: it says what happened.
- **On a plan step.** `posted:<mode>` may stand beside `deliver:<mode>` on the same step, and only there (`plan template: step <id>: posted:<mode> needs deliver:<mode> on the same step`). It is judged on the step's deciding (or bound) candidate: `[open]` until a post record about its present bytes exists, then `[attested]`, never `[satisfied]`. The done record gains an entry `gate: "posted:pr-comment"`, `standing: "attested"`, `evidence` the post record's path, `content` the candidate's digest.

```text
step ship#1/comment [current] owes: posted:pr-comment [open]; met: deliver:pr-comment [satisfied] @runs/run-…/plan/splice-ship.1.json:1
  candidate: 20261117T103623Z-95bb2e [ready] @runs/run-…/deliveries/20261117T103623Z-95bb2e/candidate.md:1
  next: post runs/run-…/deliveries/20261117T103623Z-95bb2e/candidate.md to harbor#42, then: writfold kb deliver posted 20261117T103623Z-95bb2e --ref URL --by WHO
```

- Without `posted:` on the step, a ready delivery still reads `next: writfold kb plan advance`, preceded by one line: `note: nothing on this step records a post; add posted:pr-comment to the template to hold the step until one is recorded`.
- `resume` and the session-start context carry the `posted:` line of a current step's candidate, so a cold session reads "posted, by whom, where" or its absence before it acts.
- Limits, stated wherever the line is printed: it is a statement. A session that stops between the post and the record leaves no trace; D19 narrows that and nothing closes it.
- JSON: flag `posted`; a `posted` continuation with `status` (`attested` or `stale`), `text`, `path`; row key `posts` (integer).
- Tests: W9 as a failing test first (advance refused while `posted:` is open; admitted after the record; `[done] !stale` after an edit); two clones recording a post each merge to two records; kill tests. Mutations: judging `posted:` on the newest record regardless of bytes must fail the edit test; accepting `posted:` without `deliver:` must fail the template test.
- Docs: delivery-modes ("Posting under the guard" is rewritten: the attested repository gate recipe is replaced by `posted:<mode>`), run-graph, json, assurance (the proved row; "Trusted, not checked" gains "a post record is text").

### D7. Rendering

```toml
rendering = "strip-citations"          # absent: "verbatim", as today
```

- The payload is the candidate with every citation token removed, together with one space or tab directly before it. Nothing else changes, no line is added or removed, so line numbers hold. With `verbatim` the payload is the candidate.
- `deliver check` writes `payload.md` beside the candidate when the rendering is not `verbatim`, and records the payload digest in each receipt. `deliver payload [CANDIDATE] [--to FILE]` prints the payload from the present candidate. `paragraphs.bounded`, `references.denied` and script gates read the payload (`{candidate}` expands to the payload file; `{source}` is new and names `candidate.md`); `claims.cited-fresh` reads the candidate's tokens. The guard, `deliver match` and `posted` compare with the payload computed from the present candidate, never with the stored `payload.md`; a `payload.md` older than the candidate is denied with `payload.md is older than the candidate: writfold kb deliver check <id>`.
- The candidate row gains `payload=<12 hex>` when it differs from `candidate=`, and every path a `next:` or a denial offers for posting is the payload's.
- Not built: redaction, link rewriting, templates, any rendering that adds text.
- Tests: W7 as a failing test first (the payload holds no `⟦`; the guard passes the payload and denies the candidate); a property test that stripping keeps the line count and every non-token byte in order; receipts go stale when `rendering` changes. Mutation: comparing the guard's bytes with the candidate under `strip-citations` must fail the W7 test.

### D8. Findings, `next:`, JSON

`deliver check` and `deliver status` keep their rows on stdout. For each failed or unknown check whose receipt is about the present bytes they draw the findings on stderr, in the form of [errors](../guide/errors.md), up to ten, then `… and N more: <receipt path>`:

```text
failed[references.denied]: line 5 matches the deny rule `internal-host` for audience external
 --> runs/run-…/deliveries/20261117T103311Z-e948b4/candidate.md:5:25
  |
5 | Tested against ledger-01.corp.example.com; see OPS-4412 for the rollout plan and the retry bug it fixes.
  |                         ^^^^^^^^^^^^^^^^^ internal-host
  |
  = note: rule `internal-host: \.corp\.example\.com` @.writfold/deliveries/deny-external.toml:3
  = help: reword or remove the match, then: writfold kb deliver check 20261117T103311Z-e948b4

failed[paragraphs.bounded]: the paragraph at line 7 has 170 words; this mode takes at most 150
 --> runs/run-…/candidate.md:7:1
  = help: split or shorten it (paragraph_words = 150 @built-in pr-comment)

failed[claims.cited-fresh]: the citation at line 1 no longer matches its source (changed)
 --> runs/run-…/candidate.md:1:33
  = note: code:kb:src/main.rs:1-1
  = help: read the source again, correct the sentence if it no longer holds, and replace the token: writfold kb cite make kb src/main.rs 1-1

unknown[references.denied]: no deny list for audience internal
  = help: the deny list is the team's policy: ask before writing .writfold/deliveries/deny-internal.toml (rules = [] says nothing is denied)
```

The span is found by matching the rule on that line of the present payload; receipts are unchanged and still copy no matched text. A script gate's finding is drawn at its line with the script's text. Nothing is drawn for a stale receipt.

`next:` under a candidate (in `deliver status`, `plan status`, `resume`, the hooks) follows the first unmet obligation:

| First unmet | `next:` |
|---|---|
| open or stale check or script gate | `writfold kb deliver check <id>` |
| failed check or script gate | `edit <payload source path> (line 5: internal-host, ticket), then: writfold kb deliver check <id>` |
| unknown, no deny list | `ask for .writfold/deliveries/deny-<audience>.toml (the team's policy), then: writfold kb deliver check <id>` |
| attested, open or stale | `read: <instructions>` then `writfold kb deliver attest <id> --check <name> --statement TEXT --by WHO` |
| a statement the policy rejects | no command; `why:` names the key and its place |
| none (ready), unposted | D5's line |

JSON, all additive: on a `delivery` row `mode`, `audience`, `destination`, `digest`, `payload_digest`, `bytes`, `ready` (boolean), `seq`, `posts`, `next` (string or absent); on a `check` continuation `check`, `class`, `outcome` (`pass`, `fail`, `unknown` or absent) and `findings`, an array of `{line, column, width, rule, detail}` (`column` and `width` absent when the span is not known). Readers use these and stop splitting `text`.

Tests: golden files for each drawing; the JSON keys pinned in the envelope test and named in json.md; W8 as a failing test first. Mutation: drawing from a receipt about other bytes must fail a golden test.

### D9 to D14

- **D9.** `deliver modes [MODE]`: read-only, one row per mode (built-in or file), with its audience, rendering, bound, each obligation with class and whether it may be waived, `read:` lines for instructions, the deny rule names per audience, and the guard routes that post it. `kb policy` prints the same mode rows after the gates, and the guard's routes. A mistyped `--mode` gets `help: did you mean "pr-comment"?` under the suggestion rule of [errors](../guide/errors.md), and the refusal lists the KB's modes with the built-in ones.
- **D10.** `audience = "external"` in a mode file fixes it. `prepare` without `--audience` takes it; a different `--audience` is refused (`commit-message is for audience external (@.writfold/deliveries/commit-message.toml:4)`). Absent: as today.
- **D11.** A verb without a candidate ID means: with `--mode M`, the candidate that decides for `M` (the newest live one, if unbound); else the only live, unbound candidate; else it refuses, listing each live, unbound candidate with its mode, state and the step it decides. `prepare --from -` reads standard input, with the same size and UTF-8 limits; the manifest's source name is `-`.
- **D12.** `kb init --example delivery` writes, create-new only: `deny-external.toml` (two example rules), `deny-internal.toml` (`rules = []`), `commit-message.toml` and `release-note.toml` with their gates and scripts, `guard.toml` with the four routes above, and a plan template whose last step is `"comment: deliver:pr-comment posted:pr-comment"`. Each file says in a comment that it is an example to edit. The same bytes are a fixture, and a test runs the whole flow on it.
- **D13.** `deliver attest` and `deliver waive` add `note: references.denied is [failed]; repairing the candidate will make this statement stale` when a mechanical or observed obligation of the candidate is not met. The statement is still written.
- **D14.** The "Delivery candidates" section of `plugins/writfold/skills/kb/SKILL.md` is rewritten around the order an agent follows: read `deliver modes MODE`; prepare; check; repair the candidate file in place and check again; attest last, after reading the `read:` file; never waive or withdraw for the user unless asked, and say so when a waiver is what would be needed; post the payload file itself with the command's file flag, never a retyped body; record the post; advance. It also says what a guard denial means and that the repair is in the denial. The `evaluate` skill is unchanged.

`deliver status` explains `!edited` once in its notes when the flag is shown: `note: !edited: the candidate differs from the file it was prepared from; checks are about the candidate`.

## 5. Build plan

Proofs round 2 and a bug-fix round land first. Rebase on both before step 1 and re-run the walk of section 1; a finding either round has closed is struck from this document rather than built twice.

Files this round shares with them, and should expect conflicts in: `crates/writfold-core/src/delivery.rs`, `plan.rs` (`GateFact`) and `policy.rs`; `proofs/Writfold/Delivery.lean`, `proofs/Audit.lean`, `proofs/mutants.py`, `proofs/extract.sh` and the regenerated `proofs/Extracted/WritfoldCore.lean`; `crates/writfold-kb/src/deliver/{mod,status,step,receipt,recheck,mode,mode_check,config,config_check}.rs` (receipt ordering without a clock is planned for the proof round and touches `receipt.rs` and `status.rs`; `mod.rs` is at 376 lines and must be split before a verb is added); `crates/writfold-kb/src/guard/delivery.rs`, `plan/delivery.rs`, `hook/start.rs`, `init.rs`, `verbs/policy.rs`; the usage text; `docs/guide/{delivery-modes,run-graph,hooks,policy,gates,json,errors,assurance}.md`, `docs/writfold/{aeneas,kb-v1-design}.md`; `plugins/writfold/skills/kb/SKILL.md`; the golden files of `deliver`, `plan status`, `resume` and the hooks; `fixtures/guard-conformance/`.

| # | Step | Depends on | Days |
|---|---|---|---|
| 1 | The kernel of 3.1, its theorems and mutants; the proofs gate passes. Callers adapted with every `Accept` all-three, `Place` never withdrawn, `Unread::AllReady`, `spent` false: no behaviour changes yet, and the existing tests pass unchanged except W3's | | 2 |
| 2 | D1: `waivable`, a mode gate's `accept`, the refusal and `!not-accepted` | 1 | 1 |
| 3 | D2 and D11: withdraw, the candidate a verb means, the `prepare` note, `--from -` | 1 | 1 |
| 4 | D8: findings drawn, `next:` per obligation, the JSON keys | | 1.5 |
| 5 | D7: `rendering`, `deliver payload`, `{source}` | 4 | 1 |
| 6 | D3, D6 and D4: routes, unread causes, the denial texts, `deliver match` | 1, 3, 5 | 2 |
| 7 | D5: `deliver posted`, `posted:<mode>`, the lines in `resume` and session start | 1, 6 | 1.5 |
| 8 | D9, D10, D12, D13: `deliver modes`, `kb policy` rows, `audience`, the init example and its fixture | 2, 6, 7 | 1.5 |
| 9 | D14 and the guides, the as-built section of the design, assurance; gates; release | all | 1 |
| | **Cut line** | | |
| 10 | D18: `length.bounded` (`max_chars`, `max_lines`), not in the built-in modes' defaults | 4 | 0.5 |
| 11 | D19: a `post-tool-use` hook line after a routed call returns: `record the post: writfold kb deliver posted <id> --ref URL --by WHO` | 6, 7 | 0.5 |
| 12 | D20 to D22, each when a second team asks | | |

Each of steps 2 to 8 starts from its failing test (the W or F number it closes), lands with its mutations shown to fail, and updates its guide in the same commit. Step 1 alone may land without a guide change; it changes one behaviour (W3) and says so in the commit message and the guide.

## 6. Open questions

| # | Question | Recommendation |
|---|---|---|
| 1 | Is an unread body under a route always denied, or may a route opt back into "every candidate ready"? | Always denied. The lenient rule stays available by listing the command under `commands`. Two rules per route is one more thing to misconfigure |
| 2 | Does a posted candidate stop passing the guard (D6)? | Yes. A second post of the same text to another place is a second candidate with its own destination; a duplicate is the failure worth stopping |
| 3 | Must a candidate be ready for `deliver posted` to record it? | No. Record what was found, flag `!unready-when-recorded`. Refusing would leave a real post with no record |
| 4 | With `waivable` absent, every obligation may still be waived. Should a mode with `references.denied` warn? | `kb lint` warns once (`policy.waivable`, as for a checklist) when a mode lists `references.denied` or a mechanical gate and takes waivers for it. The init example sets `waivable = []`. The default does not change |
| 5 | Who may waive, attest, withdraw or record a post? | `--by` stays text. The init example's comments and the guide give the guard rule that makes the harness ask a person before `writfold kb deliver waive` and `deliver withdraw`, as [policy](../guide/policy.md) does for gates |
| 6 | Should an older draft of a mode decide no step once a newer one is bound (the W3 change), given that it changes a run that drafted two candidates ahead of two steps? | Yes. That run gets an open step and a `prepare` command; today's rule gives a met step on text nobody chose |
| 7 | Should `strip-citations` leave a short reference (`src/main.rs@f4d4c6a`) where the token was? | No. Whatever is left is outward text the deny list must then judge; a team that wants a reference writes it in the sentence |
| 8 | Should `{candidate}` in a script gate become the payload, as specified, or stay the source with `{payload}` new? | The payload: a gate checks what will be posted. With `verbatim` nothing changes, and no existing mode has another rendering |
| 9 | Should `posted:<mode>` be allowed on a later step than `deliver:<mode>`? | Not in this round. It needs a second binding rule; the same step covers the flow walked |
| 10 | Should the built-in modes gain an audience or a length bound? | No (D16). Their digests are in every existing run. The example modes carry the better defaults |

## Decisions (owner, 17 November 2026)

1. All ten recommendations in section 6 are adopted as written.
2. Order: this round is built after proof backfill round 2 and the bug-fix round have merged, and rebased on them; the delivery kernel's theorems are replaced in place and pass the proofs gate before any verb is rewired (Lean proofs first).
3. Steps above the cut line must land; below it, each item ships whole or moves to a later release.
