# Writfold

A local knowledge engine for coding agents: exact evidence, explicit approval,
reproducible context.

**Finding something is not proof. A citation is not approval. A database join is
not inference.** Writfold keeps evidence, support, freshness and permission to act
as separate, recorded steps, so an agent cannot quietly turn "I found a document"
into "I am allowed to change the code".

> **Early development.** Writfold is being built in public. Most of the system
> exists as separately tested parts; the connected prepare → approve → apply flow
> is not wired up yet. See [writfold.dev](https://writfold.dev/) for the overview
> and [status](https://writfold.dev/status.html) for what runs today.

## The three tools

| Tool | Job | Try it |
|---|---|---|
| **Writfold** (this repo) | Holds exact evidence and the approval policy; the only part that may write files | `writfold serve` session, below |
| [**Weft**](https://github.com/writfold-dev/weft) | Checks what declared assumptions actually support: entailed, refuted or open | [Weft: Try it](https://github.com/writfold-dev/weft#try-it) |
| [**Splicery**](https://github.com/writfold-dev/splicery) | Describes byte-exact edits and previews the result without touching the file | [Splicery: Try it](https://github.com/writfold-dev/splicery#try-it) |

## Try it

Linux x86_64 with [rustup](https://rustup.rs) (the pinned toolchain installs on
first build):

```sh
cargo build --release -p writfold
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2"}}' \
  '{"jsonrpc":"2.0","id":2,"method":"shutdown"}' \
  | target/release/writfold serve
```

`writfold serve` speaks newline-delimited JSON-RPC 2.0 on stdio. Today it
implements session start and shutdown only and refuses every other method; it
does not read a corpus yet.

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
printf '@h := find text "trial_blob(" in "crates/" at "%s";\n@e := retain @h around lines 0;\n@v := project @e [path, line, binding, excerpt(max_chars: 72)] limit 6;\nemit @v budget_tokens 300;\n' "$(git rev-parse HEAD)" > q.tuft
target/release/writfold tuft run q.tuft --repo .
```

Search is exact text or a bounded regex; semantic, fuzzy and ranked search
are refused until the Omnigraph backend exists.

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
