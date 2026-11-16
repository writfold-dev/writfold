# Writfold

Project continuity for coding agents. A fresh session on any machine can ask
why something was decided, where a piece of work stands, which tracker item it
belongs to and what changing it touches, and every answer points at the file
and line it came from, flagged when that source has moved. The knowledge base
is plain text in Git; a local SQLite index is derived from it and can always be
rebuilt. A new machine needs `git clone` and one binary.

> **Pre-release (`v0.6.0-dev.1`).** Built in public; formats and verbs may still
> change. The [release page](https://github.com/writfold-dev/writfold/releases)
> carries a static Linux x86_64 binary, `writfold-x86_64-linux-musl`, with its
> `.sha256`. See [writfold.dev](https://writfold.dev/) and the
> [guides](docs/guide/README.md). The binary has two commands, `kb` and
> `watch`; the earlier trial flow, `serve` and `writfold tuft` were removed
> on 16 November 2026 (the last release with them is `v0.5.0-dev.9`).

## The family

| Tool | Job | Try it |
|---|---|---|
| **Writfold** (this repo) | Answers continuity questions from a Git knowledge base; runs, checkpoints, gates with receipts, plans and delivery checks | [Try it](#try-it), below |
| [**Weft**](https://github.com/writfold-dev/weft) (frozen) | Checks what declared assumptions actually support: entailed, refuted or open; read by the assessed gate | [Weft: Try it](https://github.com/writfold-dev/weft#try-it) |
| [**Splicery**](https://github.com/writfold-dev/splicery) (frozen) | Describes byte-exact edits and previews the result without touching the file; replayed by `kb edit apply` | [Splicery: Try it](https://github.com/writfold-dev/splicery#try-it) |

Tuft, Weft and Splicery are frozen (no new development). Tuft's `writfold
tuft` subcommand is archived; the `tuft` crate here only supplies the
citation cursor the KB uses.

## Try it

Download the binary from the
[release page](https://github.com/writfold-dev/writfold/releases) and check it:

```sh
sha256sum -c writfold-x86_64-linux-musl.sha256
install -m 0755 writfold-x86_64-linux-musl ~/.local/bin/writfold
```

Or build it on Linux x86_64 with [rustup](https://rustup.rs) (the pinned
toolchain installs on first build):

```sh
cargo build --release -p writfold
```

### Project continuity with `writfold kb`

`writfold kb` reads a knowledge base of plain records in Git (assets,
relationships, decisions, ticket records, investigations, signatures, runs and
project folders) and answers where a project stands, what surrounds a record,
why something was decided, what state a record is in, what changing it
touches, who relies on it, and which citations have moved. A derived SQLite
index lives under the repository's Git directory and is refreshed in place for
the files that changed since the commit (and dirty files) it was built at;
`index verify` checks it equals a rebuild. The write verbs record checkpoints,
tickets, decisions, links and status changes as ordinary files and leave them
uncommitted for you (or your agent) to review and commit.

```sh
cd /path/to/kb/some-project            # or pass --kb DIR, or set WRITFOLD_KB
target/release/writfold kb brief       # project inferred from the directory
target/release/writfold kb context HBR-101
target/release/writfold kb why "batch ledger writes"
target/release/writfold kb status HBR-101
target/release/writfold kb find "harbr-api"          # exact, BM25, substring, one typo
target/release/writfold kb match "fsync took 30s"    # signature patterns first
target/release/writfold kb impact harbor-api --depth 2 --direction both
target/release/writfold kb uses demo-service@<commit>:src/ledger.rs:3-5
target/release/writfold kb timeline harbor-db
target/release/writfold kb stale       # code citations need local clones (below)
target/release/writfold kb index status    # or: index verify, index rebuild
target/release/writfold kb check           # stale, index verify, gate status, plan status: one summary, worst exit
target/release/writfold kb runs --open      # runs: state, branch/worktree, last checkpoint
target/release/writfold kb resume           # the current run: checkpoint tail, rechecks
```

Writes (each one file, or one directory, published atomically; an existing
ID or a file changed since it was read is a conflict, exit 3):

```sh
writfold kb checkpoint --new-run --project harbor --task "Close HBR-101" --note "Start: replay numbers."
writfold kb checkpoint --note "Replay takes 41 s; next: batch size 250."   # the current run
writfold kb record ticket HBR-102 --title "Replay too slow" --assets harbor-api,harbor-db
writfold kb record decision --title "Replay in batches" --target-source harbor-api --project harbor
writfold kb record link depends-on quay-worker harbor-db --evidence demo-service@<commit>:src/ledger.rs:3-5
writfold kb status HBR-102 --set done
writfold kb status <run-id> --set closed
writfold kb cite make demo-service src/batch.rs 2-4    # prints a citation token; writes nothing
```

A record call can also wait for review as a proposal: `writfold kb propose
ticket|decision|link …` takes the same arguments, writes nothing tracked and
queues the call in the KB's Git directory (local, lost with the clone; at most
ten per session). `writfold kb proposals [list] | show | accept | drop <id>`
reviews the queue; `accept` replays the call through `record`, and refuses,
keeping the proposal, when a citation in it is no longer fresh.

The files keep the layout the KB already uses, so tools that read it keep
working: a run's repository, branch, worktree (recorded, never created), base
commit and `closed` state are kept in `runs/<id>/writfold-run.json` beside the
manifest; a decision starts as a draft, and `status --set` refuses the
decision transitions that need design-binding fields it does not compute
(accepted, landing, applied, verified, regressed).

Gates are checks with pinned receipts. A gate is a small TOML file,
`.writfold/gates/<id>.toml`, in the KB root or in the repository it checks:
an argv (never a shell string), a class (`mechanical` or `observed`), a
timeout, and a path scope. `gate run` runs it on the repository and records
one JSON receipt in the run, bound to the definition's digest, the commit and
the content of the paths in scope. `gate status` derives each gate's state
(`open`, `satisfied`, `stale`, `failed`, `unknown`) on every read; `resume`
shows it too. A receipt goes stale when an in-scope path changes, so a docs
edit leaves a test gate satisfied while a code edit does not. A timeout, crash
or failure to start is `unknown`, never a pass.

```sh
writfold kb gate run unit             # this repository defines quality and unit (wrapping scripts/gate)
writfold kb gate status               # exit 0 only when every gate is satisfied
writfold kb gate verify runs/<run>/gates/<receipt>.json   # re-run at the recorded commit: agree or disagree
writfold kb gate lines                # receipt lines for a commit message
```

This resists mistakes (a forgotten check, a check of other bytes, a stale
"tests pass"), not adversaries: receipts are tracked files, so Git history
shows an edited or hand-written one, and a receipt whose digest does not match
reads as invalid, but nothing is signed. See the
[G1 design as built](docs/writfold/kb-v1-design.md#16-g1-as-built).

Two obligation classes are statements, not checks. A gate of class
`attested` runs nothing and is met by `gate attest <id> --statement TEXT --by
WHO`; `gate waive <id> --reason TEXT --by WHO` sets any gate aside. Both are
shown as `attested` and `waived`, never as `satisfied`, and stop standing when
the content in the gate's scope moves.

A gate may name an `instructions` Markdown file beside its definition (the
guideline it checks, or, on an attested gate, what is being attested to) and
ship its own script through the `{gate}` placeholder, which is the
definition's directory: `argv = ["sh", "{gate}/check.sh"]`. Both files' bytes
are part of the definition digest, so editing the guideline or the script
makes earlier receipts and attestations stale; a comment in the TOML does
not. `gate status`, `gate run` and `resume` print the file in a `read:` line.
Paths that leave the definition's directory are refused, and nothing is run
when definitions are read.

A gate of class `assessed` (frozen with Weft: kept working, not developed further) is met by a [Weft](https://github.com/writfold-dev/weft)
assessment of a plan step. Its definition names a Weft source (`weft =
"{gate}/step.weft"`, or `"{run}/<path>.weft"` for a file the agent submits
in the run), the checks that must hold, and the SHA-256 of the `weft`
binary; the binary's path comes only from `$WRITFOLD_WEFT` or
`~/.config/writfold/config.toml`, and one with another digest is refused.
Each assumption cites its evidence with Weft's own `claim <name> = citation
"…";` declared directly above it: a KB citation token, `gate:<id>`, or a
gate, retrieval or edit receipt of the run. `gate run` resolves every
citation (a token must be fresh, a gate satisfied or attested), runs Weft,
and records a receipt with each check's Weft result and basis. A check is
met only when Weft entails it and every assumption it rests on is grounded;
one resting on an ungrounded assumption is shown `hypothetical` and never
counted as met. The receipt goes stale when the source or any cited
evidence moves; no binary is `unknown`, never a pass. See the
[Weft assessed gate as built](docs/writfold/kb-v1-design.md#23-weft-assessed-gate-as-built).

Delivery candidates are offline. `kb deliver prepare --mode
MODE --from FILE [--audience internal|external]
[--destination NAME]` copies a draft into the run as `candidate.md`, the exact
payload that would be posted, with a manifest of its mode, audience,
destination and digests. Nothing is ever sent: there is no network and no
adapter. `kb deliver check` runs the mode's built-in checks on the candidate's
bytes and records one receipt per check: `paragraphs.bounded` (words per
prose paragraph), `references.denied` (the audience's deny list in
`.writfold/deliveries/deny-<audience>.toml`, reporting line and rule) and
`claims.cited-fresh` (every citation token fresh under `stale`). `kb deliver
status` shows each candidate's checks and exits 0 only when all are
satisfied, attested or waived; editing the candidate makes its receipts
stale, and one candidate's checks never count for another. The checks are
mechanical: they do not judge tone or truth, and a claim made without a
citation token is not seen.

```sh
writfold kb deliver prepare --mode pr-comment --from comment.md --audience external --destination harbor#42
writfold kb deliver check             # the run's only candidate (or name it)
writfold kb deliver status            # [ready] or [blocked], with each check's line and rule
```

`pr-comment` and `slack-summary` are built in; any
`.writfold/deliveries/<name>.toml` defines another mode, listing built-in
`checks` and `gates`. A mode's gates declare `subject = "candidate"`: a
script gate gets the payload file as `{candidate}` and reports findings as
`<line>: <text>` on standard output; an attested gate shows its instructions
and is met only by `deliver attest`. Unknown keys and gates are refused when
the mode is loaded. The fixture KB's worked `commit-message` mode
(`fixtures/kb/commit-message/`) checks a subject and body layout with a
shipped script, denied references for an external audience, and an attested
language guideline. Its limit is printed with every status: nothing binds the
checked message to the commit that is then made. See
[instruction gates and delivery modes as built](docs/writfold/kb-v1-design.md#20-instruction-gates-and-delivery-modes-as-built).

A run can follow a plan: `kb plan set <template>` splices the steps of
`.writfold/plans/<template>.toml` (each written `id: gate gate …`) into the
run, keeping a snapshot of the template. `kb plan status` derives each step
from the run's event files on every read (done, `!stale` when a gate that met
it no longer holds, current with what it owes, later); `kb plan advance`
records the current step as done, naming the receipt or statement that met
each gate, or refuses with exit 3 and writes nothing. A run with a plan owes
only its plan's gates. Gates of class `run-record` judge the run's own records
(`research.backfilled`, `run.reflected`) instead of code. Writfold records
the plan; the agent walks it. See
[the run graph as built](docs/writfold/kb-v1-design.md#21-run-graph-linear-as-built).

`kb plan use <template>` splices a template's steps before the current step,
as its children; a repeat writes nothing and exits 0, so a skill can splice
itself on load with one line (`` !`writfold kb plan use review || true` ``
in its `SKILL.md`). A step may include another template with `use=<name>`.
A template edited after it was spliced shows `!template-moved`; the run keeps
its snapshot. `.writfold/plans/required.toml` names templates spliced into
every new plan, at the start and at the end (for example a closing `reflect`
step); the plan cannot complete without them. See
[splicing and required steps as built](docs/writfold/kb-v1-design.md#24-splicing-and-required-steps-as-built).

The plugin's optional `PreToolUse` hook (`writfold kb hook pre-tool-use`) is
a guard, not enforcement. It is off unless the KB has
`.writfold/deliveries/guard.toml` listing publishing commands (such as `gh pr
comment`); then such a command is denied unless the current run has a ready
candidate holding the bytes being posted. See
[G2 as built](docs/writfold/kb-v1-design.md#19-g2-as-built). The same hook
evaluates guard rules (`.writfold/guards/<id>.toml`, v0.5.0-dev.1): a rule can
deny, or ask about, a named command or tool, such as `git commit` or `Edit`,
until the run's gates are met, naming each unmet gate and its `next:` command.
A rule can be scoped to file paths (`paths`, v0.5.0-dev.3); a `stop` rule adds context once per session, or once per turn with `repeat = "turn"`. See
[guard rules as built](docs/writfold/kb-v1-design.md#30-guard-rules-and-the-stop-nudge-as-built).

Splicery recipes apply in a run's worktree (frozen: kept working, not developed further). `kb edit apply` replays one or
more [Splicery](https://github.com/writfold-dev/splicery) recipes (each
`edit "<path>" { … }` names one file) through the pinned kernel against the
worktree's current bytes, uncommitted edits included, and writes the results
there: each file atomically, after checking it still holds the bytes the
recipe was replayed against. A refused recipe (with Splicery's match sites
and counts) or a file changed in the meantime writes nothing. The recipe,
its splice sites, the before and after blob IDs and the preview diff are kept
as a receipt in `runs/<id>/edits/`, and `resume` shows it. The change is an
ordinary uncommitted Git change: gates whose scope covers it go stale until
they run again. `--dry-run` prints the preview diff and writes nothing; a run
that records no worktree needs `--worktree DIR` once, which is then recorded.

```sh
writfold kb edit apply rename.spl docs.spl --dry-run   # the preview diff; nothing written
writfold kb edit apply rename.spl docs.spl             # apply both files, record the receipt
writfold kb edit recover                               # after a crash between two files: finish the batch
```

Use Splicery for repeated or structural edits (every matching line, a block
between two anchors, an exact expected count); ordinary edits are simpler for
one small change. This is a cooperating, single-user tool: nothing stops the
agent's shell from editing the worktree directly, and nothing is signed or
approved. See the
[adapter as built](docs/writfold/kb-v1-design.md#17-splicery-adapter-as-built).

`writfold watch` shows recorded run progress, not an estimate of work left.
Its eight read-only views project the KB verbs: a nested plan map and node
inspector, Gates, Timeline, Inbox, Guards, Stale, Brief and Resume.
The runs list includes each plan's position. Guards concern the current-run
pointer, not whichever run is selected. The frame reads in plain words by
default (`next up`, `out of date`, `needs review-list and signoff`); `d` shows
the verbs' exact wording, `c` the ids and commands to copy, and `?` each plain
phrase beside its exact word. No key writes, runs a command or starts a checker.

`h`/`l` move between runs, map and inspector; `j`/`k` select a node or scroll.
`.` selects the recorded current step, `n`/`N` the next/previous flag. `x` switches
to the text list, where `X` expands detail. `g` selects Gates and `e` Resume.
`o` filters open runs and `P` the selected project. The view polls KB and
queue/pointer changes every two
seconds, and the selected subject every ten seconds; `--repo` selects that
subject without changing KB discovery. Wide, medium and narrow layouts share
six themes, with ANSI and no-colour modes. See [Watching a run](docs/guide/watch.md)
for the views, keys, refresh limits and colour selection.

```sh
writfold watch                            # h/l focus, j/k selection, ? help, q quit
writfold watch --repo /path/to/repository # gate subject
writfold watch --run RUN_ID --once        # one expanded plain frame; also automatic in a pipe
writfold watch --view gates --theme paper # one view at start; --once limits the printed sections
```

KB read answers keep their source positions; flags (`!moved→5-7`,
`!past-ttl`, `!draft` …) mark what to re-check, and their trailer is always last:

```text
decision proposal/decision/2026-03-02-batch-ledger-writes [accepted] Batch ledger writes in the API @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:5
  decided: by ops-review on 2026-03-09 @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:11
  why: Group ledger writes into batches of up to 500 rows per transaction. @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:24
  cite: code:demo-service:src/batch.rs:2-4 [moved] @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:31 !moved→5-7
# coverage=complete records=28/28 shown=3 omitted=0 stale=1 kb=d5db758bda80 index=fresh cursor=none q=206e21d1c60f
```

`impact` states the edges it left unwalked (`frontier=`) and the ones it
skipped as retired, disputed or inferred; it prints `coverage=complete` only
when both are zero. Prose mentions of IDs are inferred links, followed only
with `--include inferred`. `--budget-tokens N` (default 1000) bounds the rows
and `--cursor` resumes the rest; `--json` prints the same rows as one object. Exit codes: 0 complete,
2 partial, unknown or stale, 3 refused (such as an ambiguous name), 1 error.
`stale` checks code citations against local clones only and never fetches:
list them in `.git/writfold/repos` of the KB repository (`<repo-name> <path>`
per line) or point `WRITFOLD_KB_REPOS` at directories holding them. The
invented example KB the tests use is in [`fixtures/kb/basic`](fixtures/kb/basic);
the format and plan are in the [KB v1 design](docs/writfold/kb-v1-design.md).

The [guides](docs/guide/README.md) hold the full skeletons: the KB layout,
what each stage writes back, validation, gates, delivery modes, hooks and
what is planned next, each marked built or planned.

### The Claude Code plugin

[`plugins/writfold`](plugins/writfold) gives a Claude Code session the KB
without a tool call:

- **Session start:** the hook injects `brief` for the project inferred from
  the session's directory, `resume` when a run is current, the plan position
  (the current step and the gates it still owes), and an inbox of unconfirmed
  checkpoint drafts, pending proposals and runs that ended without their
  `reflect` step. A line says when the KB clone is behind its upstream,
  read from local refs only (the hook never fetches). After a compaction it
  re-injects only the plan position. The whole is cut to 9,000 characters,
  brief first (Claude Code's limit is 10,000). Nothing in it calls a model.
- **Compaction, stop and session end:** the hooks keep a local checkpoint
  draft per session in the KB's Git directory. It holds the repository, branch
  and HEAD, the compaction summary and your last message. Before compaction the
  hook also asks the summary to keep decisions, open questions and the next
  step. A draft is never committed. `writfold kb checkpoint --from-draft
  <session>` turns it into a checkpoint entry and removes it.
- **Skill `writfold:kb`:** which verb answers which question, and how to read
  the trailer and exit codes.
- **Skill `writfold:evaluate`:** the run's wrap-up, run in a forked
  subagent: it splices the `reflect` template unless the run has one
  (`` !`writfold kb plan use reflect || true` ``), writes a `reflection:`
  checkpoint entry, proposes up to three cited records or writes
  `no proposal:`, meets the reflect step's gates and advances the plan. No
  hook runs it; the agent, the user or the next session's inbox does. See
  [the evaluation skill as built](docs/writfold/kb-v1-design.md#26-evaluation-skill-as-built).

Each hook runs `writfold kb hook <event>` and always exits 0. With no
`writfold` on `PATH`, no KB, no Git or unreadable input, it prints nothing.

Try it from a clone, with the binary on `PATH`:

```sh
cargo build --release -p writfold && export PATH="$PWD/target/release:$PATH"
cd /path/to/kb/some-project            # or set WRITFOLD_KB
claude --plugin-dir /path/to/writfold/plugins/writfold
```

To install it for every session, add this repository as a plugin
marketplace, then install the plugin:

```sh
claude plugin marketplace add writfold-dev/writfold   # or a local clone's path
claude plugin install writfold@writfold
```

The plugin has one optional setting, `kb_root`: Claude Code asks for it when
it enables the plugin, or pass `--config kb_root=/path/to/kb` to `install`.
Set it when your sessions start in code repositories rather than in the KB. Without
it, the hooks use `$WRITFOLD_KB`, then the nearest KB above the session's
directory.

### What works today

| Command | What it does | Evidence |
|---|---|---|
| `writfold kb brief / context / why / status / find / match / impact / uses / timeline / stale / index` | Reads a Git KB of plain records; every row pinned to its file and line, moved citations flagged; the index is refreshed incrementally and verified equal to a rebuild | [KB v1 design](docs/writfold/kb-v1-design.md#11-m1-as-built) (sections 11 and 12), golden and property tests in `crates/writfold-kb/tests` |
| `writfold kb checkpoint / resume / runs / record / status --set / cite make` | Writes checkpoints, runs, tickets, decisions, links and status changes atomically (old or new bytes after a kill at any step); prints citation tokens | [KB v1 design](docs/writfold/kb-v1-design.md#13-m3-as-built) (section 13), kill and fault tests in `crates/writfold-kb/tests/faults.rs`, TLC model `spec/KbWrite.tla` |
| Claude Code plugin (`plugins/writfold`), `writfold kb hook`, `checkpoint --from-draft` | A KB brief at session start with no tool call, with the plan position, an inbox and a behind-remote line from local refs; the plan position again after compaction; local checkpoint drafts at compaction, stop and session end; one skill teaching the verbs, and `writfold:evaluate` closing a run with a reflection, cited proposals or an explicit none, and its gates | [KB v1 design](docs/writfold/kb-v1-design.md#14-m4a-as-built) (sections 14 and [25](docs/writfold/kb-v1-design.md#25-deterministic-hooks-as-built)), golden tests in `crates/writfold-kb/tests/hook.rs`, `hook_start.rs`, `crates/writfold/tests/kb_hook.rs`; the evaluation loop in `crates/writfold-kb/tests/evaluate.rs` ([section 26](docs/writfold/kb-v1-design.md#26-evaluation-skill-as-built)), skill front matter in `crates/writfold/tests/plugin_skills.rs`; one `claude -p` run loaded `/writfold:evaluate` forked, without a KB |
| `writfold kb gate run / status / verify / lines` | Runs a gate's argv on a repository and records one receipt in the KB run; derives open, satisfied, stale, failed or unknown from definitions and receipts; re-runs a receipt at its commit; prints receipt lines for a commit message | [KB v1 design](docs/writfold/kb-v1-design.md#16-g1-as-built) (section 16), golden, stale-transition and kill tests in `crates/writfold-kb/tests/gate.rs` and `gate_faults.rs`; the real `scripts/gate` has not yet been run through it |
| `writfold kb gate attest / waive`, `writfold kb deliver prepare / check / attest / waive / status`, `writfold kb hook pre-tool-use` | Statements recorded as attested or waived, never shown as satisfied; offline delivery candidates (the exact payload, never sent) checked mechanically for paragraph length, denied references and stale citations, with receipts bound to the candidate's bytes; an optional guard hook, off by default | [KB v1 design](docs/writfold/kb-v1-design.md#19-g2-as-built) (section 19), golden, repair, staleness, statement, guard and kill tests in `crates/writfold-kb/tests/deliver.rs`, `deliver_faults.rs`, `gate_statements.rs` and `guard.rs`; no real PR comment or Slack summary has gone through it yet |
| `writfold kb gate cover`, gates of class `checklist` | A gate declares named items; each is covered `supported` with references Writfold grounds (citation tokens, locators, gates and receipts), or `unsupported` / `not-applicable` with a reason. Item standings are derived on every read: a moved citation makes only the items citing it stale, an edit in scope makes every item stale; a fully covered checklist stands as `attested`, never `satisfied` | [KB v1 design](docs/writfold/kb-v1-design.md#31-checklist-gates-and-gate-cover-as-built-v050-dev1) (section 31), exhaustive kernel tests in `writfold-core/src/checklist.rs`, golden, staleness, refusal, plan and kill tests in `crates/writfold-kb/tests/gate_checklist.rs` and `gate_checklist_faults.rs`; no real review has been covered yet |
| Kind declarations (`.writfold/sources/<kind>.toml`), the stale count at session start, the `--json` envelope | A citation kind gets a default lifetime, `immutable` freshness or required fields; undeclared kinds behave as before. Session start counts non-fresh citations from the last `kb stale`. The `--json` envelope is [documented](docs/guide/json.md) and its keys pinned by a test | [KB v1 design](docs/writfold/kb-v1-design.md#29-kind-declarations-the-stale-count-at-start-and-the-json-envelope-v050-dev1) (section 29), kernel tests in `writfold-core/src/verdict.rs`, `crates/writfold-kb/tests/kinds.rs`, `json.rs`, `hook_start.rs` |
| `status <investigation> --set concluded` | Concludes an investigation once it has `Checked against` and `Scope and limits` text and no open hypothesis; otherwise refuses, naming what is missing | exhaustive kernel test in `writfold-core/src/investigation.rs`, `crates/writfold-kb/tests/investigation_conclude.rs` |
| `status <decision> --set accepted\|applied\|verified` (v0.5.0-dev.2) | `accepted` needs `--by` and the review readiness `proposed` needs, and records the design revision; `applied` needs `--commit REPO@SHA` that resolves in a local clone and is in the target ref's history, and appends it to `landed`; `verified` needs `--evidence` naming a gate receipt that grounds now. Each refusal exits 3 and writes nothing; `landing` and `regressed` stay unwritten | [KB v1 design](docs/writfold/kb-v1-design.md#32-decision-states-as-built-v050-dev2) (section 32), exhaustive transition test in `writfold-core/src/decision.rs`, `crates/writfold-kb/tests/decision_states.rs` |
| External checkers: `freshness = "checked"` kinds, `cite make --kind`, `kb checker list` (v0.5.0-dev.2) | `kb stale` runs `writfold-check-<name>` from `PATH` once per kind over `writfold-checker/1` (JSON lines) and caches each verdict; every other reader uses the cache. An absent, failing, hung or malformed checker leaves its tokens `unverifiable`, never fresh. `cite make --kind` builds a token through the checker, refusing fields the kind does not allow | [KB v1 design](docs/writfold/kb-v1-design.md#33-external-checkers-phase-1-as-built-v050-dev2) (section 33), [guide](docs/guide/checkers.md), exhaustive kernel tests in `writfold-core/src/verdict.rs`, `crates/writfold/tests/kb_checker.rs` and `kb_checker_reads.rs` with a fake checker; no real external system has been checked yet |
| Following a rename with edits (v0.5.0-dev.2) | A code citation whose path is gone is followed through Git's rename detection (one `diff-tree` per cited revision, at most 64 per run); it reads `moved` only when the cited span's digest is found at the new path, otherwise `gone` with the reason | [KB v1 design](docs/writfold/kb-v1-design.md#62-algorithm-for-a-code-citation) (section 6.2), `crates/writfold-kb/tests/rename.rs` |
| `writfold kb repos [--missing] [--json]` (v0.5.0-dev.3) | One row per repository the KB's code and gate citations name: `local`, `partial` or `missing`, how it was found (map, search root, path), its target commit and when it was last fetched; exit 2 when any is missing. A partial (`--filter`) clone's code citations read `unverifiable: partial-clone`, and a Git failure in one repository no longer fails the whole `kb stale` run | [validation guide](docs/guide/validation.md) (with a clone-and-fetch recipe), `crates/writfold-kb/tests/repos.rs` with a real blobless clone, `crates/writfold/tests/kb_repos.rs` |
| `writfold kb observe` (v0.5.0-dev.3) | Records one lookup in the run (`runs/<run>/observed/`) with its status: `ok`, `partial`, `no-data`, `not-found`, `timeout`, `denied` or `error`. Only an `ok` look whose citation is fresh can support a checklist item; the others show under the item as `looked:` and may be cited on `unsupported` and `not-applicable` rows | [gates guide](docs/guide/gates.md), exhaustive kernel test in `writfold-core/src/observe.rs`, `crates/writfold-kb/tests/observe.rs` |
| `gate cover <gate> --from FILE\|-` (v0.5.0-dev.3) | Covers several checklist items from JSON lines with one `--by`, each row checked as the single-item form; one bad row refuses the batch and writes nothing | `crates/writfold-kb/tests/gate_checklist_batch.rs`, kill test in `gate_checklist_faults.rs` |
| Tracked proposals (v0.5.0-dev.3) | `propose … --tracked`, or the marker `.writfold/proposals/.tracked`, queues a proposal in the KB's work tree to commit with it, so it survives losing the clone; `proposals` reads both queues and accept or drop removes every copy | [feeding back](docs/guide/feeding-back.md), `crates/writfold-kb/tests/proposals_tracked.rs` |
| Guard `paths` and `repeat = "turn"`; checker `max_age` (v0.5.0-dev.3) | A tool rule can be scoped to file globs (`["**", "!**/*.md"]`), matched before the index is opened; a stop rule can remind once per turn; a checked kind's cached verdict older than `max_age` reads `unverifiable: check-expired` | [hooks guide](docs/guide/hooks.md), [checkers guide](docs/guide/checkers.md), `crates/writfold-kb/tests/guard_paths.rs`, `checker_cache.rs` |
| `writfold kb stale --record`: check receipts (v0.5.0-dev.9) | Writes what external checkers said in one run to tracked, write-once files under `.writfold/checks/<kind>/`; any clone that has them, with no checker or credentials, reads the newest verdict per token for as long as the kind's `max_age` allows, always labelled `recorded` with when it was checked, and never caches it locally | [KB v1 design](docs/writfold/kb-v1-design.md#38-check-receipts-as-built-v050-dev9) (section 38), [checkers guide](docs/guide/checkers.md#sharing-verdicts-check-receipts), exhaustive kernel tests in `writfold-core/src/verdict/recorded.rs`, a recording and a reading clone in `crates/writfold/tests/kb_checker_record.rs` and `kb_checker_receipts.rs`, kill test in `crates/writfold-kb/tests/check_receipt_faults.rs`; no real external checker has recorded yet |
| `writfold kb record investigation` and `propose investigation` (v0.5.0-dev.9) | Opens an investigation in `investigations/<year>/<date>-<slug>/` with its edges and the sections `status --set concluded` checks, empty | [KB v1 design](docs/writfold/kb-v1-design.md#391-record-investigation-and-propose-investigation) (section 39.1), `crates/writfold-kb/tests/record_investigation.rs` |
| `writfold kb edit apply -`; guard rules naming `writfold:edit` (v0.5.0-dev.9) | One Splicery recipe from stdin, kept in the receipt as `stdin`; a guard rule listing `writfold:edit` with `paths` is checked by `edit apply` itself against each recipe's target before anything is written, on any harness | [KB v1 design](docs/writfold/kb-v1-design.md#179-as-built-v050-dev9-a-recipe-on-stdin) (sections 17.9 and 30.10), [hooks guide](docs/guide/hooks.md#edits-made-by-writfold-kb-edit-apply), `crates/writfold-kb/tests/edit_stdin.rs` and `edit_guard.rs` |
| Guard rules: how a command is read, `gates.met` over nothing (v0.6.0-dev.1) | A `commands` rule matches through wrappers (`sudo`, `env`, `command`, `time`, `if`, `{`, `!`), a path to the binary and `git`'s global options (`git -C DIR`, `--no-pager`, `-c`), never inside a here-document's body or before a lone `--help`, and judges each matched command in the repository it acts on (`cd DIR &&`, `git -C DIR`); a directory that cannot be told takes the rule's action with "name the directory literally"; a linked worktree answers to its repository's name; `gates.met` with no gate owed refuses | [hooks guide](docs/guide/hooks.md#how-a-command-is-read), [design section 40](docs/writfold/kb-v1-design.md#40-v060-dev1-as-built), table and corpus tests in `writfold-core/src/command/tests.rs`, `crates/writfold-kb/tests/guard_dirs.rs` and `guard_empty.rs` |
| Guard conformance cases (v0.6.0-dev.1) | 59 language-neutral cases (a world, an event, the expected decision) run through the real hook entry point under `cargo test`; an outside suite is converted into the format, with each difference recorded | [`fixtures/guard-conformance/`](fixtures/guard-conformance/README.md), `crates/writfold-kb/tests/guard_conformance.rs` |
| `writfold kb find` tiers and `find --eval` (v0.6.0-dev.1) | `normalized` (`harbor api`, `HAPI`), `prefix` (`quay work`) and `heading` tiers between exact and full text, the tier in each row's `matched_on`, up to three suggested names on an empty answer (still exit 2), and `find --eval` measuring a tracked query file (hit@1, hit@5, miss, recall per tier) | [kb-layout guide](docs/guide/kb-layout.md#what-a-lookup-matches), [validation guide](docs/guide/validation.md#lookups-find---eval), `crates/writfold-kb/tests/find_tiers.rs` and `find_eval.rs` |
| `mode = "audit"`, `writfold kb guard log` (v0.6.0-dev.1) | An audit rule is judged and logged but never decides the call; `guard status` flags it `!audit` and a pass resting on a waiver `!waived`; the local guard log records what each rule would have refused, and `guard log` counts it per rule version and says when an audit rule can be enforced | [hooks guide](docs/guide/hooks.md#trying-a-rule-audit-mode), `crates/writfold-kb/tests/guard_audit.rs`; no real session has been audited yet |
| `writfold kb runs --idle AGE`, `writfold kb delta` (v0.6.0-dev.1) | Open runs whose last recorded write (from tracked file names and checkpoints, never modification times) is older than `AGE`; what differs in the KB since a commit or a local mark, by blob ID and classified as records, run artefacts or configuration, with the session's mark taken at session start | [feeding back](docs/guide/feeding-back.md#what-was-written-since-kb-delta), `crates/writfold-kb/tests/runs_idle.rs` and `delta.rs` |
| A delivery as a plan step's gate, `deliver:<mode>` (v0.6.0-dev.1) | A step that owes `deliver:pr-comment` is not advanced until the newest candidate of that mode no done step is bound to is ready (every check satisfied, attested or waived for its present bytes); the done record binds the candidate, and an edit to it or to its mode turns the step `!stale`. A passed `claims.cited-fresh` check stands only while its citation tokens are still fresh, in `deliver status`, the delivery guard and the plan. `gate status` and `gates.met` do not count deliveries; `step.met` does. Nothing is sent: done means a candidate was ready, not that it was posted | [run-graph guide](docs/guide/run-graph.md#a-delivery-as-a-steps-gate), [delivery modes](docs/guide/delivery-modes.md#on-a-plan-step), [design section 42](docs/writfold/kb-v1-design.md#42-delivery-modes-as-a-steps-gates-as-built), exhaustive kernel tests in `writfold-core/src/delivery.rs`, `crates/writfold-kb/tests/plan_deliver.rs`, `plan_deliver_pick.rs`, `plan_deliver_cited.rs` and `plan_deliver_faults.rs`; no real PR comment has been held by a plan step yet |
| Guard rules (`.writfold/guards/<id>.toml`), the Stop nudge | `writfold kb hook pre-tool-use` denies or asks about a named command or tool until the current run, its gates (as `gate status` judges them) or its plan step meet the rule's requirements, naming each unmet gate and its `next:` command; a matched rule that cannot read the KB takes its action; with no rule directory nothing is opened. A `stop` rule adds context once per session (or once per turn with `repeat = "turn"`, v0.5.0-dev.3), never while `stop_hook_active` is set; `paths` globs scope a tool rule to the files it edits | [KB v1 design](docs/writfold/kb-v1-design.md#30-guard-rules-and-the-stop-nudge-as-built) (section 30), kernel tests in `writfold-core/src/guard.rs`, hook tests in `crates/writfold-kb/tests/guard_rules.rs` and `crates/writfold/tests/kb_hook.rs`; no real session has been guarded by a rule yet |
| Instruction gates and user-defined delivery modes | A gate's `instructions` file and `{gate}`-shipped scripts are part of its digest and shown as `read:` lines; any `.writfold/deliveries/<name>.toml` defines a delivery mode whose script gates run on the candidate through `{candidate}` and whose attested gates show their instructions | [KB v1 design](docs/writfold/kb-v1-design.md#20-instruction-gates-and-delivery-modes-as-built) (section 20), golden, staleness, escape, refusal, never-a-pass and kill tests in `crates/writfold-kb/tests/gate_instructions.rs`, `deliver_modes.rs` and `deliver_faults.rs`; the `commit-message` mode is a fixture and has not yet checked a real commit |
| `writfold kb plan set / status / advance / skip`, run-record gates | A plan's steps spliced from a template snapshot into the run as create-new event files; the current step, what it owes and `!stale` done steps derived on every read, independent of file order and clock; advance refused (exit 3, nothing written) until every gate of the current step is met; `research.backfilled` and `run.reflected` judged over the run's records | [KB v1 design](docs/writfold/kb-v1-design.md#21-run-graph-linear-as-built) (section 21), golden, refusal, stale, order-independence and kill tests in `crates/writfold-kb/tests/plan.rs` and `plan_faults.rs`, kernel tests in `writfold-core/src/plan.rs`; no real run has followed a plan yet |
| `writfold kb plan use`, `use=` includes, `!template-moved`, required steps | A template's steps spliced before the current step as its children, idempotent on the template and digest at the current step or an ancestor (`--again` adds another); includes resolved at splice time with their closure snapshotted, cycles and nesting beyond four levels refused; an edited template flagged without changing the run; required start and end templates spliced into every new plan, which cannot complete without them | [KB v1 design](docs/writfold/kb-v1-design.md#24-splicing-and-required-steps-as-built) (section 24), golden, idempotence, include, refusal, moved and order-independence tests in `crates/writfold-kb/tests/plan_splice.rs`, required-steps tests in `plan_required.rs` over `fixtures/kb/required-steps`, kill tests in `plan_faults.rs`, kernel `splice` tests in `writfold-core/src/plan.rs`; the skill injection line has run once in a `claude -p` session (`/writfold:evaluate`, without a KB) |
| `writfold kb propose`, `writfold kb proposals list / show / accept / drop`, `writfold kb check` | Queues `record ticket\|decision\|link` calls locally (one file per call, at most ten per session, linked to the run) and accepts one through the ordinary write path, refusing a stale citation or a taken ID with the proposal kept; runs `stale`, `index verify` and the current run's `gate status` as one summary exiting with the worst code | [KB v1 design](docs/writfold/kb-v1-design.md#22-proposals-and-kb-check-as-built) (section 22), golden, dedup, cap, stale-refusal and kill tests in `crates/writfold-kb/tests/proposals.rs` and `proposal_faults.rs`; `writfold:evaluate` fills it at the end of a run; no real run has yet |
| `writfold kb gate run` on a gate of class `assessed` | Grounds each Weft assumption's citations (fresh KB tokens, satisfied or attested gates and receipts), runs the pinned `weft` binary on the step's source, and records each check's Weft result with a grounded or hypothetical basis; met only when every required check is entailed from grounded assumptions | [KB v1 design](docs/writfold/kb-v1-design.md#23-weft-assessed-gate-as-built) (section 23), kernel property tests in `writfold-core/src/assess.rs`, golden, staleness, refusal and kill tests in `crates/writfold-kb/tests/gate_assessed.rs` and `gate_assessed_faults.rs` (a labelled test double, plus one test against a real Weft build); no real plan step has been assessed yet |
| `writfold kb edit apply / recover` | Replays Splicery recipes against a KB run's recorded worktree and writes each file atomically after re-checking its bytes; keeps the recipes, splice sites, blob IDs and preview as a receipt in the run; a crash between files is finished by `recover` from a local journal | [KB v1 design](docs/writfold/kb-v1-design.md#17-splicery-adapter-as-built) (section 17), golden, refusal, gate-staleness and kill tests in `crates/writfold-kb/tests/edit.rs` and `edit_faults.rs` |
| `writfold watch [--kb DIR] [--repo DIR] [--run ID] [--view NAME] [--theme NAME] [--color MODE] [--once]` | Read-only recorded progress: steps, gates, checklist items, timeline, pending drafts/proposals and current-run guards; responsive layouts, six themes, cached checker verdicts and plain frames | [Watching a run](docs/guide/watch.md), [as built](docs/writfold/kb-v1-design.md#34-t2-as-built) (section 34); projection, layout, colour, refresh and read-only tests in `crates/writfold-tui/tests/` and binary checker-counter tests |

`cargo test --workspace` runs the Rust test suite.

## What is in this repository

- `crates/`: the Rust implementation. `writfold` is the binary (`kb`,
  `watch`); `writfold-app` holds the `kb` verbs' argument handling;
  `writfold-kb` the knowledge base, index, runs, gates, plans, deliveries and
  hooks; `writfold-tui` the `watch` view; `writfold-core` the pure decision
  kernels; `writfold-runtime` the native Git, filesystem and process effects;
  `writfold-sources` the pinned Splicery kernel behind `kb edit apply`;
  `writfold-domain` shared types; `tuft` the citation cursor; and
  `writfold-xtask` the source, dependency-graph and budget checks.
- `proofs/`: `crates/writfold-core` translated to Lean by Aeneas and proved
  there ([how](docs/writfold/aeneas.md)).
- `spec/`: the TLA+ models `KbWrite` (KB writes) and `RunGraph` (the run
  graph), with their mutants and probes.
- `docs/guide/`: the guides; `docs/writfold/`: the mission, the
  [v1 design](docs/writfold/kb-v1-design.md) with its as-built sections,
  release specifications and the [status](docs/writfold/status.md) page.
- `plugins/` and `.claude-plugin/`: the Claude Code plugin and its
  marketplace entry.
- `fixtures/kb/` and `fixtures/guard-conformance/`: test knowledge bases and
  guard conformance cases.
- `vendor/splicery/`: the vendored Splicery source `writfold-sources` builds.
- `scripts/`, `compose.yaml`, `docker/`, `qualification/` and
  `toolchains.lock.json`: the inputs of `scripts/gate quality|unit`
  ([quality gate](docs/writfold/quality-gate.md)).

The trial flow, `serve`, `writfold tuft`, Dolt, Omnigraph, the Lean
models of the edit flow, the benchmark pilots, the earlier handovers and their
qualification records are in a private archive; the last public tree with
them is tagged `pre-split-2026-11-16`.

## Feedback

Issues and discussion are welcome on
[GitHub issues](https://github.com/writfold-dev/writfold/issues).

## License

Writfold and its companion website use the [MIT license](LICENSE), selected by Tarek Nawara on 2 November 2026. Third-party materials retain their original notices and licenses; this does not relicense bundled reference material or authorize publication of private playbook content.
