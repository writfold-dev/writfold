# Writfold guides

> **Status:** built in v0.3.0-dev.1. Each page below says what is built and what is planned.

These pages hold the full skeletons the website only sketches: file layouts, complete configuration files and real command output. The output comes from an invented demo knowledge base (`harbor`); run IDs and digests are real, local paths are cut to `…`.

| Page | What it covers | Status |
|---|---|---|
| [kb-layout](kb-layout.md) | The knowledge base directory, records, citation tokens, the trailer, `.writfold/`, tracked and local state | built |
| [feeding-back](feeding-back.md) | What each stage writes, with which verb, where it lands and who commits | built, with proposals and the backfill and reflection checks |
| [validation](validation.md) | Checking that what you read still holds: `stale`, `index`, `gate status`, `deliver check`, and `kb check` for all at once | built, with the plan line of `kb check` |
| [gates](gates.md) | Gate definitions, receipts, standing, waivers and attestations, run-record checks | built |
| [delivery-modes](delivery-modes.md) | Delivery candidates, mode files, deny lists and the guard hook | built; your own modes planned |
| [run-graph](run-graph.md) | Plans as a spine of steps that owe gates | built (linear); splicing planned |
| [skills](skills.md) | The bundled `writfold:kb` skill, and skills that name a plan template | built; templates planned |
| [hooks](hooks.md) | What each Claude Code hook reads, writes and prints | built |
| [assurance](assurance.md) | What is proved, model-checked, tested and trusted | built; KB kernel proofs planned |

A `toml` block in these pages is parsed by the real parser in a test (`crates/writfold-kb/tests/guide_toml.rs`). A block under a `<!-- planned -->` marker is a sketch of a format that is not built; the same test checks that today's parser still refuses it, so the marker cannot outlive the feature.

The design record, with every deviation and test, is [`docs/writfold/kb-v1-design.md`](../writfold/kb-v1-design.md). The direction is [`docs/writfold/mission.md`](../writfold/mission.md).
