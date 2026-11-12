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

### The Claude Code plugin

[`plugins/writfold`](plugins/writfold) gives a Claude Code session the KB
without a tool call:

- **Session start:** the hook injects `brief` for the project inferred from
  the session's directory, `resume` when a run is current, and any unconfirmed
  checkpoint drafts. It is cut to 9,000 characters (Claude Code's limit is 10,000).
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
| Claude Code plugin (`plugins/writfold`), `writfold kb hook`, `checkpoint --from-draft` | A KB brief at session start with no tool call; local checkpoint drafts at compaction, stop and session end; one skill teaching the verbs | [KB v1 design](docs/writfold/kb-v1-design.md#14-m4a-as-built) (section 14), golden tests in `crates/writfold-kb/tests/hook.rs`, `crates/writfold/tests/kb_hook.rs` |
| `writfold kb gate run / status / verify / lines` | Runs a gate's argv on a repository and records one receipt in the KB run; derives open, satisfied, stale, failed or unknown from definitions and receipts; re-runs a receipt at its commit; prints receipt lines for a commit message | [KB v1 design](docs/writfold/kb-v1-design.md#16-g1-as-built) (section 16), golden, stale-transition and kill tests in `crates/writfold-kb/tests/gate.rs` and `gate_faults.rs`; the real `scripts/gate` has not yet been run through it |
| `writfold kb edit apply / recover` | Replays Splicery recipes against a KB run's recorded worktree and writes each file atomically after re-checking its bytes; keeps the recipes, splice sites, blob IDs and preview as a receipt in the run; a crash between files is finished by `recover` from a local journal | [KB v1 design](docs/writfold/kb-v1-design.md#17-splicery-adapter-as-built) (section 17), golden, refusal, gate-staleness and kill tests in `crates/writfold-kb/tests/edit.rs` and `edit_faults.rs` |
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
