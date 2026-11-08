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
does not read a corpus or write anything yet.

`cargo test --workspace` runs the Rust test suite, including checks that the
Rust kernels agree with cases exported from the Lean model.

## What is in this repository

- `crates/`: the Rust implementation (domain types, pure kernels, wire formats,
  signatures, runtime boundaries and the `writfold` binary).
- `formal/`: the Lean model and proofs the Rust kernels are checked against.
- `spec/`: TLA+ models of the operational protocols.
- `docs/writfold/`: design and boundary documents; [status](docs/writfold/status.md)
  has the detailed implementation and evidence record.
- `handovers/`: the planning contracts the implementation follows.
- `qualification/`: machine-readable status and evidence indexes.

## Feedback

Issues and discussion are welcome on
[GitHub issues](https://github.com/writfold-dev/writfold/issues).

## License

Writfold and its companion website use the [MIT license](LICENSE), selected by Tarek Nawara on 2 November 2026. Third-party materials retain their original notices and licenses; this does not relicense bundled reference material or authorize publication of private playbook content.
