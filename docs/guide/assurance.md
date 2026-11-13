# Assurance

> **Status:** built in v0.4.0-dev.1. The run-graph, gate and assess kernels are proved over the shipped Rust, and the run graph is model-checked with TLC. Kernels added since (investigation, checklist and guard in v0.5.0-dev.1; decision states and checker verdicts in v0.5.0-dev.2; observations in v0.5.0-dev.3) are tested exhaustively and not proved.

What stands behind `writfold kb`, sorted by how strong the claim is. The short version: the write protocol and the run graph are model-checked, the write protocol is kill-tested; the decisions about plans and gates (which step is current, whether a step may be recorded as done, how a splice changes the plan, what a gate's receipts and statements add up to) are proved for every input over the shipped Rust; everything that turns files into those decisions' inputs is tested.

## Tested

| What | How |
|---|---|
| Gate scopes (`writfold_core::scope`) | glob semantics and exclusions; not proved (it uses closures and iterators the translation does not take) |
| The plan fold (`writfold-kb`) | every rotation and reversal of the event listing folds to the same steps and current step; a Git clone answers the same; the kernel's `splice` agrees with the fold step for step |
| Citation verdicts (`writfold_core::verdict`) | 3,000 random files, ranges and edits: fresh means unchanged lines, moved names a range carrying the same digest, changed lines are never fresh |
| Citation lifetimes (`writfold_core::verdict::lifetime`, v0.5.0-dev.1) | every combination of an immutable kind, a token `ttl=` and a declared default; 4,000 random `seen`, lifetimes and clocks: the verdict `stale` gives equals the kernel's over the lifetime it chooses |
| The `--json` envelope | its top-level, row and trailer keys are pinned, and each is named in [json](json.md) |
| Bounded walk (`writfold_core::walk`) | compared with a reference breadth-first search |
| Writes | the test binary kills itself at each of the seven protocol steps for a checkpoint, a new run, a ticket and a status change; afterwards the file holds the old or the complete new bytes and `index verify` is equal. The same for gate receipts, edits and delivery candidates |
| The index | `index verify` equals a rebuild after each group of writes |
| Receipts | a hand-edited outcome, a wrong digest and a receipt copied under another gate's name each read as invalid |
| Output | golden files for every verb, the hooks and `watch --once` |
| These guides | every `toml` block is parsed by the real parser (`crates/writfold-kb/tests/guide_toml.rs`) |

## Model-checked

[`spec/KbWrite.tla`](../../spec/KbWrite.tla) models the write protocol and the index refresh: two appenders and a creator, two refreshers, a reader, and a power loss at any moment. TLC checks five invariants over about 18 million states: a published record is complete, no acknowledged write is lost, create-new never overwrites, the stored index is the derivation of its pin, and a reader told `index=fresh` sees exactly that derivation. Five mutants, each dropping one protocol step, each fail their invariant.

[`spec/RunGraph.tla`](../../spec/RunGraph.tla) models the run graph (`plan set|use|advance|skip`): two sessions, either in two clones that merge or in one clone, a crash between any two writes, the subject's bytes moving under a gate, and a template edited mid-run. TLC checks that a step has at most one `done-` record, that every done record names a receipt about the bytes it judged, that every splice has its snapshot, that two clones holding the same files agree on the plan, that the current step is the first neither done nor skipped, that a splice repeats no template already spliced on its chain unless `--again` asked, that the done set only grows, that a step adds nodes only before the current step, and that a stale step never becomes current again. The bounds are small (two templates, splices two levels deep, two or three decisions per session): with drift and a crash the check is complete for two sessions in one clone (about 44 million states), and for two merging clones it is complete without them (about 2 million states) and was stopped unfinished with them. Ten mutants, each changing one rule, each fail their property. Details and numbers: [design §27](../writfold/kb-v1-design.md#27-model-specrungraphtla).

The run-graph model found two gaps, recorded and not yet fixed:

- **Within one clone,** a plan write decides from a fold read outside the write lock and only checks, under the lock, that its file name is new. Two sessions in one clone can interleave so that a step is done while steps spliced before it are open. The model shows that re-checking the decision under the lock (and, for `plan use`, that nothing was spliced under the step meanwhile) closes this.
- **Across clones,** a merge can put open steps before a done one (a splice under a step another clone has closed, or two different plans set on one run). Nothing is lost and both clones agree, but `plan status` does not flag it.

The models are hand-written and bounded. They are evidence about the design, not a refinement of the Rust: `KbWrite.tla` is tied to the code by the kill tests at the same steps, and `RunGraph.tla` by the plan tests (order independence, kill tests, the kernel's exhaustive tests), not by proof.

## Proved

With Charon and Aeneas, over the shipped Rust, for every input ([`aeneas.md`](../writfold/aeneas.md), `proofs/Writfold/`):

| Kernel | Proved |
|---|---|
| The run graph (`writfold_core::plan`) | the current step is the first step neither done nor skipped; `plan advance` is admitted exactly for the current step with every gate met, and otherwise refused in a fixed order (complete, not current, the first owed gate); a stale step is done and is never the current step, so `!stale` never moves the plan back; a splice keeps every existing step in order with its state and the done prefix, inserts open steps, and makes the first of them current; a splice is `already` exactly when its key is already spliced at the current step or above it, so repeating it changes nothing; an admitted splice is never deeper than 4 |
| Gate obligations (`writfold_core::gate`) | only exit status 0 on a subject that did not move is a pass; the state is decided by the newest receipt about the current definition and the current bytes in scope, so a receipt about another definition or other bytes never satisfies; an attestation or a waiver never yields `satisfied`, only standing as itself; met is exactly satisfied, attested or waived |
| The assessed gate (`writfold_core::assess`) | a check meets the gate exactly when Weft entailed it and every assumption it used is grounded; a hypothetical entailment is never a pass |
| The edit trial flow | the ledger, grant, revocation, recovery and required-check kernels; that flow is kept working and is not where new development happens |

Put together: a step is recorded as done only when it is the current step and each of its gates stands satisfied by a passing receipt about those exact bytes and that definition, attested, or waived. That sentence is proved for the kernels; that the kernels are given the right facts (which receipts are valid, which digests match, which gates a step names) is tested.

Each proved claim has at least one mutant of the Rust source that the proofs must reject, and every theorem's axioms are audited (Lean's three standard axioms only).

**Not proved:** the fold from event files to the list of steps, template and gate-definition parsing, digests, Git and file reads, the write protocol's code, scope globs, citation verdicts (`writfold_core::verdict`) and the bounded walk (`writfold_core::walk`). They are tested (above).

## Trusted, not checked

1. **The agent cooperates.** It can skip a gate, edit a tracked file or write a receipt by hand. Git history shows it; nothing prevents it.
2. **`--by` is text.** A statement records who was named, not who typed it. The Git commit that adds the file is the record of who committed it.
3. **Hooks are best effort.** They may not fire, they time out open, and the harness can change them ([hooks](hooks.md)).
4. **The model's premises:** rename and link are atomic, fsync makes content durable, SQLite transactions are atomic and durable, a digest identifies content.
5. **Not modelled:** another tool editing a KB file without the write lock (the re-check narrows that window and does not close it), and a file reverted to earlier bytes between two observations.
6. **For the proofs:** Charon and Aeneas at their pins translate the Rust faithfully; the Aeneas library's models of the Rust operations used are right; the Lean kernel and its three standard axioms. The proofs are about the kernel functions; the code that calls them is tested, not proved. `splice` is proved for plans whose new length fits in `usize`.
7. **Git and the file system** behave as documented.

## Not determined at all

Model output, the quality of a statement or a reflection, whether a recorded decision is right, and whether a delivery draft is true. The delivery checks count words, match patterns and re-check citations; they do not read for meaning.

## What "derived" buys

Gate standing, delivery standing and the index are recomputed from tracked files on every read and never stored. Two clones at the same commit with the same local repositories give the same answer, and there is no state to migrate or repair. Where several gate receipts cover the same bytes, the newest is chosen by a `seq` counter and the file name, never the clock ([design 21.7](../writfold/kb-v1-design.md#217-receipt-order-without-a-clock)). One caveat today: delivery receipts are still ordered by their recorded time, which comes from the machine's clock.

## Planned

- **Ordering without a clock for delivery receipts**, which still order by recorded time.

Until the model lands, describe concurrent sessions and merges on one run as tested, not model-checked.
