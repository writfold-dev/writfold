# Assurance

> **Status:** describes v0.3.0-dev.1. Proofs of the KB kernels and a run-graph model are planned and marked so.

What stands behind `writfold kb`, sorted by how strong the claim is. The short version: the write protocol is model-checked and kill-tested, the decision kernels are small pure functions with exhaustive or property tests, and nothing about gates or plans is proved yet.

## Tested

| What | How |
|---|---|
| Gate state and standing (`writfold_core::gate`) | every sequence of up to three receipts over all outcome and match combinations; every input of `standing`: no statement ever yields `satisfied` |
| Citation verdicts (`writfold_core::verdict`) | 3,000 random files, ranges and edits: fresh means unchanged lines, moved names a range carrying the same digest, changed lines are never fresh |
| Bounded walk (`writfold_core::walk`) | compared with a reference breadth-first search |
| Writes | the test binary kills itself at each of the seven protocol steps for a checkpoint, a new run, a ticket and a status change; afterwards the file holds the old or the complete new bytes and `index verify` is equal. The same for gate receipts, edits and delivery candidates |
| The index | `index verify` equals a rebuild after each group of writes |
| Receipts | a hand-edited outcome, a wrong digest and a receipt copied under another gate's name each read as invalid |
| Output | golden files for every verb, the hooks and `watch --once` |
| These guides | every `toml` block is parsed by the real parser (`crates/writfold-kb/tests/guide_toml.rs`) |

## Model-checked

[`spec/KbWrite.tla`](../../spec/KbWrite.tla) models the write protocol and the index refresh: two appenders and a creator, two refreshers, a reader, and a power loss at any moment. TLC checks five invariants over about 18 million states: a published record is complete, no acknowledged write is lost, create-new never overwrites, the stored index is the derivation of its pin, and a reader told `index=fresh` sees exactly that derivation. Five mutants, each dropping one protocol step, each fail their invariant.

The model is hand-written and bounded. It is tied to the code by the kill tests at the same steps, not by proof.

## Proved

With Aeneas, over the shipped Rust, for every input: the ledger, grant, revocation, recovery and required-check kernels of the earlier edit trial flow ([`aeneas.md`](../writfold/aeneas.md)). That flow is kept working and is not where new development happens.

**No KB kernel is proved today.** `gate`, `verdict` and `walk` live in the same dependency-free crate and are written to be translated, but they are not in the translated set and no theorem is claimed for them.

## Trusted, not checked

1. **The agent cooperates.** It can skip a gate, edit a tracked file or write a receipt by hand. Git history shows it; nothing prevents it.
2. **`--by` is text.** A statement records who was named, not who typed it. The Git commit that adds the file is the record of who committed it.
3. **Hooks are best effort.** They may not fire, they time out open, and the harness can change them ([hooks](hooks.md)).
4. **The model's premises:** rename and link are atomic, fsync makes content durable, SQLite transactions are atomic and durable, a digest identifies content.
5. **Not modelled:** another tool editing a KB file without the write lock (the re-check narrows that window and does not close it), and a file reverted to earlier bytes between two observations.
6. **For the proofs:** Charon and Aeneas at their pins, the Aeneas library's models of Rust operations, the Lean kernel and its three standard axioms.
7. **Git and the file system** behave as documented.

## Not determined at all

Model output, the quality of a statement or a reflection, whether a recorded decision is right, and whether a delivery draft is true. The delivery checks count words, match patterns and re-check citations; they do not read for meaning.

## What "derived" buys

Gate standing, delivery standing and the index are recomputed from tracked files on every read and never stored. Two clones at the same commit with the same local repositories give the same answer, and there is no state to migrate or repair. One caveat today: where several receipts cover the same bytes, the newest is chosen by its recorded time, which comes from the machine's clock.

## Planned

- **Proofs:** the plan kernel (current step, the advance check, splicing) and the existing gate standing kernel move into the Aeneas-translated set.
- **A model:** `spec/RunGraph.tla`, covering two sessions on one run, merges, drift and crashes.
- **Ordering without a clock:** receipt ordering must not depend on a wall clock where it decides state.

These follow one week of real use of the run graph ([run-graph](run-graph.md)). Until they land, describe gates and plans as tested, not proved.
