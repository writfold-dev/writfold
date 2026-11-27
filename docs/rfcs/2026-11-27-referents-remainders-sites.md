---
rfc: "2026-11-27-referents-remainders-sites"
title: "Three laws the calculus lacks: a resolved referent, an accounted universe, a site with a neutral event and a posture"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/178
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Three laws the calculus lacks: a resolved referent, an accounted universe, a site with a neutral event and a posture

Extends [the calculus](2026-11-25-agent-vocabulary/calculus.md) (§3.11 to
§3.13 below). It is the foundation for seven focused RFCs of the same day,
each of which carries its own proof obligations:

| RFC | Law | Closes |
|---|---|---|
| [citation target](2026-11-27-citation-target.md) | referent | a fork clone's false `gone` |
| [run directory accounting](2026-11-27-run-directory-accounting.md) | remainder | `packet=complete` over unread files |
| [partial clones](2026-11-27-partial-clone-identity-first.md) | referent, remainder | every blobless clone `partial-clone` |
| [guard check and posture](2026-11-27-guard-check-and-posture.md) | site | no guard outside one harness; everything fails open |
| [session ledger](2026-11-27-session-ledger.md) | site | no "X happened before Y"; no actor on a rule |
| [outbound remainder](2026-11-27-outbound-remainder.md) | remainder | an unlisted external write passes; one body per call |
| [claims in text](2026-11-27-claims-in-text.md) | remainder | a draft with no citation passes |

## Summary

Two outside reports of 27 November 2026 (tracked in
[#178](https://github.com/writfold-dev/writfold/issues/178)) list
ten gaps. Each was checked against the code and the built binary at
`9afa5942`. Nine are real on `main`. None of them is a wrong kernel: every
proved decision is right about the facts it is given. All of them sit where
the calculus stops, in three places it never named:

1. **Which thing a fact is about.** The caller picks the referent of a fact
   by convention (`origin`, the working directory, `~/.claude`, the tool
   named `Bash`), outside the proved kernel, with no way to answer "I cannot
   tell".
2. **What a universal word ranges over.** `complete`, `pass`, `ready` and
   "no rule matched" are said over what Writfold recognised, not over what
   is there. What it did not recognise is outside the judgment and is not
   counted.
3. **Where a decision is asked.** A use site is written as one harness's
   transport. Its input has no harness-neutral form, no issuer and no
   memory, and a failure of the site itself is a pass that nobody declared.

This RFC adds one law for each, with the kernel that carries it, and says
what stays impossible. It builds nothing by itself. Nothing shipped is
withdrawn: the five layers, binding, "one record decides", "a class is
never promoted" and "unknown never admits" stand, and the trust tiers
(recorded, guarded, received) do not move. The approach is not redone from
scratch, because the faults are not in it; they are at its edges.

## Motivation: the verdicts

Reproduced with the binary built from `9afa5942` (`writfold 0.6.0-dev.20`
plus dev.21 to dev.25) unless marked "read".

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1.1 | A fork clone (`origin` a stale fork, `upstream` the cited repository) turns live evidence into `gone` | **true on main** | reproduced: `gone at 66bea466a0d4 (origin/HEAD) path absent; no rename since the cited rev` while the file is at `upstream/main`. `stale/repos.rs:198-214` takes `origin/HEAD`, else `HEAD`; the kernel (`verdict.rs`) is handed "the target" and never asks which. `cite make` has the same rule (`write/cite.rs:51-68`) and writes the fork's URL as `url=` (`:148`) |
| 1.2 | No harness-neutral guard since `kb edit apply` went | **partly true** | guard rules are judged only inside `kb hook pre-tool-use`, which reads Claude Code's event (`guard/matched.rs:98-116`, the shell tool is the literal `"Bash"`). `guard status` judges "as if matched" and takes no command. The delivery guard does have a neutral form, `deliver match` |
| 1.3 | `resume --packet` says `complete` with unread files in the run | **true on main** | reproduced: three files under `runs/<id>/observations/` and one stray file, `packet=complete required_omitted=0`. `layout.rs:113` skips the directories; `verbs/runs.rs:214-250` reads named subdirectories and never lists the run; `verbs/runs/packet.rs:57` counts only rows that were tried and failed |
| 1.4 | A blobless clone is always `partial-clone` | **true, by design, and coarser than it needs to be** | `stale/code.rs:92-94`. Reproduced on a real `--filter=blob:none` clone. With lazy fetch disabled, `git ls-tree` and `rev-parse <rev>:<path>` still answer the blob id, so "same blob" needs no blob and no network |
| 1.5 | The installer says the plugin step writes under `~/.claude` | **mostly false** | the printed plan already names `$CLAUDE_CONFIG_DIR` when it is set (`install.sh:268`, `:331` in the site repository; dry run reproduced). The header comment and the README still say `~/.claude`, and the plan does not say where the directory came from |
| 2.a | Obligations attach to command names, not effects; an unlisted external write passes | **true** | reproduced: `gh gist create notes.md` and `bash -c "git commit"` both pass a `git commit` rule in silence |
| 2.b | A skill load, a read or a spawn "never reaches a rule" | **partly true; the inference is wrong** | the plugin's matcher leaves them out (`hooks.json`), but the binary judges any tool name it is sent: a rule with `tools = ["Skill", "Agent", "Read"]` denied all three. What cannot be said is "X was seen earlier": the requirement set is closed (`guard/rules.rs:17-23`) and nothing remembers an event |
| 2.c | "No run or no plan means no obligation" | **mostly false** | `run.current` is unmet with no run; `gates.met` is unknown with no run and owes every defined gate with no plan; a `step.met` Stop rule does nudge a session with no run (reproduced). Only `step.met` is met by a run without a plan (`writfold-core/src/guard/require.rs:28-41`). The guide says so at `hooks.md:170`, one sentence after the line quoted |
| 2.d | Everything fails open | **true before a rule matches, false after** | every hook exits 0; a missing binary, no KB, bad input, an unparsable rule and a timeout pass (reproduced, `hook/mod.rs:88-133`). A matched rule whose requirement cannot be judged never passes (`unknown_never_passes`) |
| 2.e | The shipped Stop only nudges | **true** | `guard/mod.rs:136-172` prints `additionalContext`; a blocking stop is a recipe |
| 2.f | One harness | **true for the sending side** | hooks are Claude Code's. `handoff check`, `gate lines`, `deliver match` and the receiver's pre-receive hook (dev.25) take no harness |
| 2.g | Agents mark their own gates | **true, and stated** | `--by` is text. The receiver accepts no statement (`strict_accepted_iff`); the pre-receive recipe (dev.25) runs the gates itself |
| 2.h | A draft with no citation passes | **true** | `claims.cited-fresh` over zero tokens passes (`deliver/checks.rs:165-`); nothing looks at prose without a token |
| 2.i | Rules have no actor; a worker's calls cannot be told apart | **true in Writfold** | Claude Code's event carries `agent_id` and `agent_type` (`hooks.md:515`); no rule key reads them |
| 2.j | One body per call; a multi-part deliverable has no single candidate | **true** | a route reads one body; a candidate is one file |

Report 2 quoted the guides accurately: every quoted sentence is at the
line it names on `main`. Where it went wrong is in four inferences, marked
above, each drawn from a sentence that is true and easy to misread (#191).

**What dev.22 to dev.25 already close** (report 2 listed them as not
verified):

- "Workers inherit the obligations, and their reports are evidence only
  after a check": for a worker set up with `agent prepare`, yes. `needs`
  are judged at prepare, the report is a candidate, `invocation.bound`
  binds it to what the worker was handed, and a cover row may cite it.
  Not for a worker the harness spawns by itself.
- "Review evidence is a statement": a review can now be a bound report from
  a prepared reviewer invocation (`agent = "reviewer"` on a mode), which is
  stronger than `--by`. It is still not a principal.
- Self-marking: the receiver hook refuses a push whose required gates do
  not pass when the receiver runs them, whatever the worker recorded.
- Not closed by them: the guard's harness, fail-open, `saw`, the catch-all,
  claims without tokens.

## §3.11 Referent: resolution is a decision

Every fact is about something: a commit of a repository, a run, a
directory, a session, an actor. Today the caller chooses it and hands the
kernel the result.

> **Law R.** The referent of a fact is chosen by a kernel from the
> candidates the caller found, and the choice has three outcomes: one
> candidate, with the rule that chose it; **ambiguous**; or none. A fact
> about an ambiguous or absent referent is unknown. A verdict that asserts
> change or absence (`gone`, `changed`, `moved`, `failed`, `unmet`) needs a
> resolved referent that could have shown the opposite. The referent and
> the rule that chose it are printed wherever the verdict is.

Shipped instances, each found the hard way: the subject repository
(`subject::pick`, dev.4, after "target not cwd"), the current run
(`pointer`, dev.6), missing history (`absence_never_evidence`, dev.17).
Unshipped instances, found by reading for the literal `origin`:

| Site | Convention | RFC |
|---|---|---|
| `stale/repos.rs:207` | the target of a code citation is `origin/HEAD` | citation target |
| `write/cite.rs:60`, `:148` | `cite make` cites `origin/HEAD` and records `remote.origin.url` | citation target |
| `write/decide.rs:249` | a decision's commit is checked against `origin/HEAD` | citation target (same kernel) |
| `gate/subject.rs:96`, `verbs/brief.rs:61`, `write/run.rs:265`, `store/mod.rs:154` | a repository is named by `remote.origin.url` | audit (#190) |
| `guard/matched.rs:106`, `guard/delivery_body.rs:80` | the shell tool is `"Bash"` | guard check |
| the installer | the plugin directory is `~/.claude` in the text | #185 |

The kernel shape is `subject::pick`'s: small enums in, one enum out, a table
proved row by row. The new one is `writfold_core::target`.

## §3.12 Remainder: a universal word is over a declared universe

Law 3.5 says unknown never admits and an empty set is never "all met". It
is applied to members Writfold tried to read. It is silent about members
it never tried.

> **Law U.** A word that claims all of something (`complete`, `ready`,
> `pass`, `covered`, "nothing matched") states its universe by what is
> there, not by what was recognised. Every member is exactly one of
> *judged*, *declared out* (by a rule in Git text, with a count) or
> *remainder*. The word holds only when the remainder is zero, and the
> remainder is printed. A partial universe may prove that something
> exists. It never proves that something is absent.

Shipped instances: an unlistable definition root (`inventory`, #56), a run
directory without a manifest (#98), an absent manifest (`generated`, #156),
"a work limit never certifies completeness" (#60). Unshipped:

| Universe | Recognised today | Remainder, uncounted | RFC |
|---|---|---|---|
| the files of a run directory | the subdirectories Writfold writes | another tool's files | run directory accounting |
| the blobs a verdict needs | "is this a partial clone?" asked once per repository | the blobs actually missing | partial clones |
| the calls a session makes | the commands and tools a rule names | every other call | outbound remainder |
| the prose blocks of a draft | the citation tokens present | blocks with no token | claims in text |
| the parts of a deliverable | one body | the other parts | outbound remainder |
| the events of a session | the events the hook was sent | events no harness reports | session ledger (stated, not closed) |

The kernel is `writfold_core::account`:

```rust
pub enum Member { Judged, DeclaredOut, Remainder }
pub fn remainder(members: &[Member]) -> usize;   // how many are Remainder
pub fn whole(members: &[Member]) -> bool;        // remainder == 0
```

Proved once (`remainder_counts`, `whole_iff_no_remainder`,
`declared_out_never_counts`, `judged_never_counts`), then used by each
RFC's own kernel, which proves its word implies `whole`. The first user is
the handover packet.

Law U has a mirror, and partial clones show it: unknown is assigned to the
question, not to its container. "This repository is a partial clone" made
970 citations unverifiable when the unknown fact was "this blob's bytes",
and most verdicts never needed the bytes.

## §3.13 Site: a neutral event, an issuer, a posture, and what is remembered

The calculus lists four use sites as verbs (§3.8). Three take arguments
any shell can pass. The fourth, "before a tool call", is
`hook pre-tool-use`: Claude Code's JSON in, Claude Code's JSON out. The
[enforcement RFC](2026-11-25-evidence-governed-execution.md) said "no
judgment depends on an event format, so there is nothing to normalize".
That is true of `agent prepare` and `deliver`. It is false of the guard.

> **Law S.** A site decides over an **event** in Writfold's own form:
> what kind of call (`shell`, `file.write`, `file.read`, `skill`, `spawn`,
> `tool`, `stop`), its subject (the command words, the path, the name),
> where, in which session, by which actor, and **who reported it** (the
> adapter). A harness is a binding from its event format to that form,
> written as data. A site has a declared **posture**: what a call gets
> when the site could not decide. An event the site was sent may be
> **remembered**, and a remembered event is a fact of the *reported*
> class: it shows that a harness reported a call, never that a model read,
> understood or obeyed anything, and by §3.3 it is never promoted.

Two kernels:

- `writfold_core::site::decide(basis, posture)`. The basis is one of
  no rules, no match, judged (with the rules' decision) or undecided.
  Under the default posture `open` it is today's behaviour, exactly
  (`open_is_today`). Under `ask` or `deny`, a pass needs a decision
  (`closed_pass_iff`). See [guard check and posture](2026-11-27-guard-check-and-posture.md).
- `writfold_core::guard::require::saw_fact(ledger, found, stale)`: met
  only by a witness; unmet only over a whole ledger; unknown when the
  ledger is partial, unreadable or the binding does not report that kind.
  See [session ledger](2026-11-27-session-ledger.md).

No obligation type is added. `saw:` is one more name in the fixed
requirement vocabulary (L3), and the ledger is local session state beside
the drafts, marks, pointers and guard log: never committed, never read by
`plan advance` or `handoff check`. This amends decision 1 of the
enforcement RFC in one respect only: the guarded tier gains a local record
of reported events. The recorded and received tiers are unchanged.

## What Writfold cannot enforce, and how it says so

These are limits of principle. No slice of this work removes them, and
each guide page that touches one says it in these words.

| Cannot | Why | What is offered instead, and its class |
|---|---|---|
| That a model read or understood a file, a skill or an instruction | reading is not an event | "the harness reported a read of these bytes since the context was last cut" (*reported*); "this package was written" (first hand); a review of the result (attested) |
| Anything a harness never reports: a call made with no hook, a skill injected without a tool call, another harness with no binding | Writfold is not on the path of a call it is not shown | the binding declares the kinds it reports; a requirement over an unreported kind is `unknown`, never quiet |
| What an arbitrary program does: `bash -c`, `eval`, a script, `ssh` | deciding it is running it | under an outbound policy such a call is *unclassified*, and the policy says what unclassified gets; by default it passes, as today |
| Holding back text shown to the user in chat | it is displayed before any hook can refuse it | a deliverable is what goes through `deliver prepare`; a Stop check of the last message is a recipe, after the fact |
| Keeping a session from ending | a harness caps continuations (eight in Claude Code) and a crash ends it anyway | a blocking stop with a stated limit; the inbox at the next start; the receiver |
| Failing closed when the hook process never starts, or is killed at the harness's timeout | the harness lets that call through | a registration that exits non-zero when the binary is missing; a deadline inside the binary below the harness's timeout |
| Who made a statement | no server, no signing: `--by`, a session id and an actor are recorded as given | the Git author; a bound report from a prepared invocation; at a receiver, re-execution (#64) and signed evidence (#170) |
| A worker that edits its own rules, ledger or records | anyone who can write the clone can write them | detection in the recorded tier (`policy moved`), refusal in the received tier |

The sentence "a guard, not enforcement" stays on every guard output. A
posture of `deny` does not change it: it narrows what a cooperating
session can do by mistake.

## Keeping future gaps of the same kind out

Three checks, all cheap, tracked in #190:

1. **A claims register** in [assurance](../guide/assurance.md): one row per
   word a verb can print that claims all, none or change (`complete`,
   `fresh`, `gone`, `changed`, `ready`, `met`, `pass`, `covered`,
   "no rule matched"), with its referent and who chooses it, its universe,
   where the remainder is printed, and the theorem. A word without a row
   fails the docs check; a row without a theorem is marked tested-only.
2. **A convention-literal check** in `writfold-xtask`: the literals
   `origin`, `refs/remotes/origin`, `"Bash"`, `.claude` and `HEAD` used as a
   default may appear in product code only inside a module the register
   names as a resolver.
3. **A foreign-member test** per reader of a directory or a text: put one
   member the reader does not know into the universe; the verb's universal
   word must change, or the member must be counted. This is the
   "removing this check fails a test" discipline turned on inputs.

The RFC template gains three lines under Proof obligations: the referent
and its resolver, the universe and its remainder, the site and its
posture.

## Proof obligations

This RFC adds one shared kernel; every other kernel is in the RFC that
uses it.

- **Kernel:** `writfold_core::account` (`Member`, `remainder`, `whole`), as
  above.
- **Theorems** (`proofs/Writfold/Account.lean`): `remainder_spec`
  (`remainder` is the count of `Remainder` members, for every list);
  `whole_iff_no_remainder`; `declared_out_never_counts`;
  `judged_never_counts`; `append_remainder` (the remainder of a
  concatenation is the sum, so accounting can be done per directory).
- **Mutants:** `remainder-skips-last` and `declared-out-counts`, each
  breaking `remainder_spec`.
- **Tested and unproved:** classifying a member (a file by its place in a
  run, a call by its words, a block by its text) is the caller's and is
  tested with the foreign-member test above.

No protocol changes, so no TLA+ model. The ledger of the session RFC is
one append per event; that RFC says why it needs none.

## Compatibility and reversibility

Nothing changes until a focused RFC lands. Each of them keeps today's
answer for input that was judged correctly today and changes only an
answer that was wrong or silent; each lists its own changes.md lines.

## Alternatives

- **Patch the five asks.** That is what #56, #63, #90 and #98 each did for
  one instance. The class came back four times in two weeks.
- **Rewrite as a general policy engine over session events.** It would make
  Writfold a resident component on every tool call, and the mission keeps
  the product a record keeper with no companion process. The removed
  assessed gate was a rule language over evidence; this is not a second
  one. The requirement vocabulary stays fixed.
- **Move every guard to the receiver.** The receiver is the only boundary
  that resists a worker that does not cooperate, and it stays the answer
  to self-marking. It cannot see a gist or a chat message, which never
  reach it.

## Unresolved questions

None blocks. Owner questions, with the default taken, are collected in
#193. Those that touch this RFC:

1. **Owner:** the mission (12 November 2026) says enforcement points are
   gates, not harness hooks. The site law keeps guard rules as guards and
   adds a neutral form, a posture and a local ledger. *Default: proceed as
   guarded-tier work, opt-in, and add a dated mission line when the first
   site slice lands.*
2. **Owner:** order of work. *Default: referent (citation target) first,
   then the two remainder fixes of report 1, then `guard check`; the
   ledger, outbound and claims after those, each on its own milestone.*

## Decision log

- 2026-11-27: drafted from the two reports after each claim was checked
  against `9afa5942` and the built binary. Accepted under the owner's
  standing acceptance (21 November 2026); not yet reviewed by the owner.
  It amends decision 1 of `2026-11-25-evidence-governed-execution` for the
  guarded tier only, and corrects that RFC's sentence that no judgment
  depends on an event format.
