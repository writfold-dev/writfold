# Writfold

A local knowledge engine for coding agents: exact evidence, explicit approval,
reproducible context.

**Finding something is not proof. A citation is not approval. A database join is
not inference.** Writfold keeps evidence, support, freshness and permission to act
as separate, recorded steps, so an agent cannot quietly turn "I found a document"
into "I am allowed to change the code".

> **Preview (`v0.1.0-dev.1`).** Writfold is built in public. A cooperating,
> single-user trial runs end to end today: find and retain exact evidence, bind a
> Weft report, prepare a Splicery edit, approve, apply and recover. It is not a
> sandbox, not an installable release and not a knowledge service yet. See
> [writfold.dev](https://writfold.dev/) and [status](https://writfold.dev/status.html).

## The family

| Tool | Job | Try it |
|---|---|---|
| **Writfold** (this repo) | Holds exact evidence and the approval policy; the only part that may write files | trial flow, below |
| **Tuft** (this repo, `crates/tuft`) | Finds and projects exact evidence from a pinned commit under a token budget | [Tuft](#find-and-cite-exact-evidence-with-tuft), below |
| [**Weft**](https://github.com/writfold-dev/weft) | Checks what declared assumptions actually support: entailed, refuted or open | [Weft: Try it](https://github.com/writfold-dev/weft#try-it) |
| [**Splicery**](https://github.com/writfold-dev/splicery) | Describes byte-exact edits and previews the result without touching the file | [Splicery: Try it](https://github.com/writfold-dev/splicery#try-it) |

## Try it

Linux x86_64 with [rustup](https://rustup.rs) (the pinned toolchain installs on
first build):

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

A gate of class `assessed` is met by a [Weft](https://github.com/writfold-dev/weft)
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
[G2 as built](docs/writfold/kb-v1-design.md#19-g2-as-built).

Splicery recipes apply in a run's worktree. `kb edit apply` replays one or
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
approved. The edit trial's signing, approval and own-worktree flow is
separate and frozen. See the
[adapter as built](docs/writfold/kb-v1-design.md#17-splicery-adapter-as-built).

`writfold watch` keeps the runs in view in a terminal pane. It is read-only
and only shows what the verbs print: the runs of `kb runs` on the left (open
runs first, with their place and last checkpoint), the selected run's
`kb resume` on the right (gate rows with their states, edit and retrieval
receipts, drafts, the checkpoint tail) headed by `kb gate status`'s counts,
and the KB pin and index state at the bottom. Failed, stale and invalid show
in red. Tab (or `s`, `b`, `g`) switches the right pane to `kb stale` or
`kb brief` for the run's project. It polls the KB pin every two seconds and
asks the verbs again only when it moves. There is no daemon, so it never says
a run is running and shows no progress percentage; no key writes anything.
Gate states are for the repository you start it in, as with `gate status`.

```sh
writfold watch                    # j/k or arrows, Enter, Tab, ? for help, q to quit
writfold watch --run <id> --once  # one frame as plain text (also when stdout is not a terminal)
```

Every row read from a file ends with its path and line, flags (`!moved→5-7`,
`!past-ttl`, `!draft` …) mark what to re-check, and the trailer is always last:

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
directory, then the prototype configuration's `root`.

Tuft results share this vocabulary. `writfold tuft run … --cite` prints,
under each shown row, the token `kb cite make` would print for those lines
at the program's pinned commit, so a token pasted into a record checks
`fresh` under `kb stale` until the lines move or change. While the KB has an
open current run (or with `--kb DIR` / `--run ID`), each `tuft run` also
records a retrieval receipt in `runs/<id>/retrievals/` (program digest,
commit, trailer, row counts and tokens), and `kb resume` shows the newest
ones. Standard output stays as it was; the receipt's path goes to standard
error. See [Tuft's KB adapter](docs/tuft/language.md#kb-adapter).

### What works today

| Command | What it does | Evidence |
|---|---|---|
| `writfold kb brief / context / why / status / find / match / impact / uses / timeline / stale / index` | Reads a Git KB of plain records; every row pinned to its file and line, moved citations flagged; the index is refreshed incrementally and verified equal to a rebuild | [KB v1 design](docs/writfold/kb-v1-design.md#11-m1-as-built) (sections 11 and 12), golden and property tests in `crates/writfold-kb/tests` |
| `writfold kb checkpoint / resume / runs / record / status --set / cite make` | Writes checkpoints, runs, tickets, decisions, links and status changes atomically (old or new bytes after a kill at any step); prints citation tokens | [KB v1 design](docs/writfold/kb-v1-design.md#13-m3-as-built) (section 13), kill and fault tests in `crates/writfold-kb/tests/faults.rs`, TLC model `spec/KbWrite.tla` |
| Claude Code plugin (`plugins/writfold`), `writfold kb hook`, `checkpoint --from-draft` | A KB brief at session start with no tool call, with the plan position, an inbox and a behind-remote line from local refs; the plan position again after compaction; local checkpoint drafts at compaction, stop and session end; one skill teaching the verbs | [KB v1 design](docs/writfold/kb-v1-design.md#14-m4a-as-built) (sections 14 and [25](docs/writfold/kb-v1-design.md#25-deterministic-hooks-as-built)), golden tests in `crates/writfold-kb/tests/hook.rs`, `hook_start.rs`, `crates/writfold/tests/kb_hook.rs` |
| `writfold kb gate run / status / verify / lines` | Runs a gate's argv on a repository and records one receipt in the KB run; derives open, satisfied, stale, failed or unknown from definitions and receipts; re-runs a receipt at its commit; prints receipt lines for a commit message | [KB v1 design](docs/writfold/kb-v1-design.md#16-g1-as-built) (section 16), golden, stale-transition and kill tests in `crates/writfold-kb/tests/gate.rs` and `gate_faults.rs`; the real `scripts/gate` has not yet been run through it |
| `writfold kb gate attest / waive`, `writfold kb deliver prepare / check / attest / waive / status`, `writfold kb hook pre-tool-use` | Statements recorded as attested or waived, never shown as satisfied; offline delivery candidates (the exact payload, never sent) checked mechanically for paragraph length, denied references and stale citations, with receipts bound to the candidate's bytes; an optional guard hook, off by default | [KB v1 design](docs/writfold/kb-v1-design.md#19-g2-as-built) (section 19), golden, repair, staleness, statement, guard and kill tests in `crates/writfold-kb/tests/deliver.rs`, `deliver_faults.rs`, `gate_statements.rs` and `guard.rs`; no real PR comment or Slack summary has gone through it yet |
| Instruction gates and user-defined delivery modes | A gate's `instructions` file and `{gate}`-shipped scripts are part of its digest and shown as `read:` lines; any `.writfold/deliveries/<name>.toml` defines a delivery mode whose script gates run on the candidate through `{candidate}` and whose attested gates show their instructions | [KB v1 design](docs/writfold/kb-v1-design.md#20-instruction-gates-and-delivery-modes-as-built) (section 20), golden, staleness, escape, refusal, never-a-pass and kill tests in `crates/writfold-kb/tests/gate_instructions.rs`, `deliver_modes.rs` and `deliver_faults.rs`; the `commit-message` mode is a fixture and has not yet checked a real commit |
| `writfold kb plan set / status / advance / skip`, run-record gates | A plan's steps spliced from a template snapshot into the run as create-new event files; the current step, what it owes and `!stale` done steps derived on every read, independent of file order and clock; advance refused (exit 3, nothing written) until every gate of the current step is met; `research.backfilled` and `run.reflected` judged over the run's records | [KB v1 design](docs/writfold/kb-v1-design.md#21-run-graph-linear-as-built) (section 21), golden, refusal, stale, order-independence and kill tests in `crates/writfold-kb/tests/plan.rs` and `plan_faults.rs`, kernel tests in `writfold-core/src/plan.rs`; no real run has followed a plan yet |
| `writfold kb plan use`, `use=` includes, `!template-moved`, required steps | A template's steps spliced before the current step as its children, idempotent on the template and digest at the current step or an ancestor (`--again` adds another); includes resolved at splice time with their closure snapshotted, cycles and nesting beyond four levels refused; an edited template flagged without changing the run; required start and end templates spliced into every new plan, which cannot complete without them | [KB v1 design](docs/writfold/kb-v1-design.md#24-splicing-and-required-steps-as-built) (section 24), golden, idempotence, include, refusal, moved and order-independence tests in `crates/writfold-kb/tests/plan_splice.rs`, required-steps tests in `plan_required.rs` over `fixtures/kb/required-steps`, kill tests in `plan_faults.rs`, kernel `splice` tests in `writfold-core/src/plan.rs`; the skill injection line has not been run in a live Claude Code session |
| `writfold kb propose`, `writfold kb proposals list / show / accept / drop`, `writfold kb check` | Queues `record ticket\|decision\|link` calls locally (one file per call, at most ten per session, linked to the run) and accepts one through the ordinary write path, refusing a stale citation or a taken ID with the proposal kept; runs `stale`, `index verify` and the current run's `gate status` as one summary exiting with the worst code | [KB v1 design](docs/writfold/kb-v1-design.md#22-proposals-and-kb-check-as-built) (section 22), golden, dedup, cap, stale-refusal and kill tests in `crates/writfold-kb/tests/proposals.rs` and `proposal_faults.rs`; no evaluation skill fills the queue yet |
| `writfold kb gate run` on a gate of class `assessed` | Grounds each Weft assumption's citations (fresh KB tokens, satisfied or attested gates and receipts), runs the pinned `weft` binary on the step's source, and records each check's Weft result with a grounded or hypothetical basis; met only when every required check is entailed from grounded assumptions | [KB v1 design](docs/writfold/kb-v1-design.md#23-weft-assessed-gate-as-built) (section 23), kernel property tests in `writfold-core/src/assess.rs`, golden, staleness, refusal and kill tests in `crates/writfold-kb/tests/gate_assessed.rs` and `gate_assessed_faults.rs` (a labelled test double, plus one test against a real Weft build); no real plan step has been assessed yet |
| `writfold kb edit apply / recover` | Replays Splicery recipes against a KB run's recorded worktree and writes each file atomically after re-checking its bytes; keeps the recipes, splice sites, blob IDs and preview as a receipt in the run; a crash between files is finished by `recover` from a local journal | [KB v1 design](docs/writfold/kb-v1-design.md#17-splicery-adapter-as-built) (section 17), golden, refusal, gate-staleness and kill tests in `crates/writfold-kb/tests/edit.rs` and `edit_faults.rs` |
| `writfold watch [--kb DIR] [--run ID] [--once]` | A read-only terminal view of runs, gate states, receipts and checkpoints, each pane the answer of one `kb` verb; one plain-text frame when piped | [KB v1 design](docs/writfold/kb-v1-design.md#18-t1-as-built) (section 18), golden, layout and projection tests in `crates/writfold-tui/tests/watch.rs` |
| `writfold trial prepare / approve / apply / recover / show` | One edit through a durable, validated ledger in Writfold's own sparse worktree; never your checkout | [trial profile](docs/writfold/trial-profile.md), [adversarial campaign](bench/2026-11-09-adversarial/) |
| `writfold trial approve --require CHECK` | Refuses unless each named check of the bound Weft report is `entailed` (recorded as hypothetical, never as grounded) | proved: `required_entailed_spec` |
| `writfold trial retain` | Keeps exact bytes of a file at a full commit id as a citation; reports citing anything else are refused | [trial profile](docs/writfold/trial-profile.md) |
| `writfold trial evidence list/show` | Reads retained evidence back in a later session: compact rows, and exact bytes verified against their digest before printing (tampered evidence refuses); context lines come from the pinned blob, marked apart | [trial profile](docs/writfold/trial-profile.md#reusing-evidence-in-a-later-session) |
| `writfold tuft run` | Exact find, retain, project and emit over one pinned commit; `--cite` adds KB citation tokens, and a run with a current KB run records a retrieval receipt there | [Tuft v0](docs/tuft/language.md), [adapter as built](docs/writfold/kb-v1-design.md#15-tuft-adapter-as-built) |
| `writfold trial run` | Tuft evidence → a Weft report from the binary you name → `prepare`, in one command; stops at approval | [one command](docs/writfold/trial-profile.md#one-command) |
| `writfold serve` | JSON-RPC 2.0 on stdio; with `writfold.trial/1`, `trial/show`, `trial/retain` and `trial/prepare`. Approval stays CLI-only | [trial profile](docs/writfold/trial-profile.md#through-serve) |

Every agent-facing operation is budgeted at 1 s p95 (5 s ceiling); on a
100,000-file repository every trial operation's p95 is under 0.5 s
([bench](bench/2026-11-09-latency/)). The ledger, grant, revocation, recovery and
required-check decisions are proved over the shipped Rust ([how](docs/writfold/aeneas.md)).

### Prepare, approve and apply one edit

The [trial profile](docs/writfold/trial-profile.md) runs the connected flow on
any Git repository. Writfold makes its own worktree of your repository's HEAD,
replays a [Splicery](https://github.com/writfold-dev/splicery) recipe through
the pinned kernel, and installs the result only after a signed approval and a
current grant are verified. Every step is recorded in a validated ledger:

```sh
export WRITFOLD_STATE=/tmp/writfold-trial
target/release/writfold trial init
target/release/writfold trial prepare --repo /path/to/repo --file src/lib.rs --recipe edit.spl
target/release/writfold trial approve <run>
target/release/writfold trial apply <run>
git -C "$WRITFOLD_STATE/worktrees/<run>" diff
```

Your own checkout is never written. `apply` records a durable claim before it
touches the file, and `writfold trial recover <run>` settles an interrupted
apply from the file's exact bytes. This is a cooperating single-user trial, not
the hardened alpha path.

### Find and cite exact evidence with Tuft

[Tuft v0](docs/tuft/language.md) searches one pinned commit exactly, retains
each hit as a citation a Weft report can cite, and prints a compact view
under a token budget, with explicit omissions and a resume cursor:

```sh
printf '@h := find text "trial_blob(" in "crates/" at "%s";\n@e := retain @h around lines 0;\n@v := project @e [path, line, excerpt(max_chars: 200)];\nemit @v budget_tokens 300;\n' "$(git rev-parse HEAD)" > q.tuft
target/release/writfold tuft run q.tuft --repo .
```

It prints one `path:line: text` row per hit, as `grep -n` would, and a
trailer that says whether anything is missing:

```text
crates/writfold-app/src/trial/retain.rs:131:     let (blob, content) = trial_blob(executable, &git_dir, &commit, path, budgets()?, &CANCELLED)?;
crates/writfold-runtime/src/git/trial.rs:145: pub fn trial_blob(
# coverage=complete files=640/640 hits=2 shown=2 omitted=0 cursor=none program=3ff0993dee6a
```

`--out hits.txt` writes the rows to a new file and prints only the trailer.
Add `binding` to the projection for each row's citation, or pass `--verbose`
or `--json` for the full envelope.

Search is exact text or a bounded regex; semantic, fuzzy and ranked search
are refused until the Omnigraph backend exists.

### From evidence to a prepared edit in one command

`writfold trial run` runs a Tuft program, a Weft review and `prepare` in
order and stops before approval. In the review, `citation "tuft:@v#1"` is a
Writfold-side template (not Weft syntax) for the first row of view `@v`;
Writfold substitutes its exact binding digest before Weft reads the source.
Weft is not bundled: name its binary, and its sha256 is printed.

```sh
target/release/writfold trial run --repo /path/to/repo --file src/client.rs \
    --recipe edit.spl --tuft q.tuft --weft review.weft --weft-bin /path/to/weft
target/release/writfold trial approve <run> --require <entailed-check>
target/release/writfold trial apply <run>
```

It prints the retained bindings, each Weft check (hypothetical), the run and
the prepared-edit digest. Any failure names its step; see
[one command](docs/writfold/trial-profile.md#one-command).

`cargo test --workspace` runs the Rust test suite, including checks that the
Rust kernels agree with cases exported from the Lean model.

## What is in this repository

- `crates/`: the Rust implementation (domain types, pure kernels, wire formats,
  signatures, runtime boundaries and the `writfold` binary).
- `formal/`: the Lean model (the specification) and its proofs.
- `crates/writfold-core` and `proofs/`: the pure kernels production calls, translated to Lean by
  Aeneas and proved there ([how](docs/writfold/aeneas.md)).
- `spec/`: TLA+ models of the operational protocols.
- `docs/writfold/`: design and boundary documents; [status](docs/writfold/status.md)
  has the detailed implementation and evidence record.
- `bench/`: agent benchmark pilots with fixtures, prompts, graders and raw results.
- `handovers/`: the planning contracts the implementation follows.
- `qualification/`: machine-readable status and evidence indexes.

## Feedback

Issues and discussion are welcome on
[GitHub issues](https://github.com/writfold-dev/writfold/issues).

## License

Writfold and its companion website use the [MIT license](LICENSE), selected by Tarek Nawara on 2 November 2026. Third-party materials retain their original notices and licenses; this does not relicense bundled reference material or authorize publication of private playbook content.
