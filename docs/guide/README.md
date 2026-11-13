# Writfold guides

> **Status:** built in v0.5.0-dev.4 (the Tuft expansion: refs, `tuft grep`, several repositories, working-tree mode and ctags symbols; see [Tuft](../tuft/language.md)). Each page below says what is built and what is planned.

These pages hold the full skeletons the website only sketches: file layouts, complete configuration files and real command output. The output comes from an invented demo knowledge base (`harbor`); run IDs and digests are real, local paths are cut to `…`.

| Page | What it covers | Status |
|---|---|---|
| [kb-layout](kb-layout.md) | The knowledge base directory, records, citation tokens and kind declarations, the trailer, `.writfold/`, knowledge bases not written by Writfold, tracked and local state | built |
| [feeding-back](feeding-back.md) | What each stage writes, with which verb, where it lands and who commits | built, with proposals and the backfill and reflection checks |
| [checkers](checkers.md) | External checkers for citation kinds Git cannot check: the kind file, the `writfold-checker/1` protocol, a minimal checker, versions | built in v0.5.0-dev.2 |
| [validation](validation.md) | Checking that what you read still holds: `stale`, `index`, `gate status`, `deliver check`, and `kb check` for all at once | built, with the plan line of `kb check` |
| [gates](gates.md) | Gate definitions, receipts, standing, waivers and attestations, run-record checks | built |
| [delivery-modes](delivery-modes.md) | Delivery candidates, mode files, deny lists and the guard hook | built, with your own modes |
| [run-graph](run-graph.md) | Plans as a spine of steps that owe gates | built (splicing and required steps included) |
| [skills](skills.md) | The bundled `writfold:kb` and `writfold:evaluate` skills, and skills that splice a plan template | built |
| [hooks](hooks.md) | What each Claude Code hook reads, writes and prints | built |
| [json](json.md) | The `--json` envelope: keys, exit codes and the compatibility rule | built in v0.5.0-dev.1 |
| [assurance](assurance.md) | What is proved, model-checked, tested and trusted | built; run-graph, gate and assess kernels proved |

A `toml` block in these pages is parsed by the real parser in a test (`crates/writfold-kb/tests/guide_toml.rs`). A block under a `<!-- planned -->` marker is a sketch of a format that is not built; the same test checks that today's parser still refuses it, so the marker cannot outlive the feature.

The design record, with every deviation and test, is [`docs/writfold/kb-v1-design.md`](../writfold/kb-v1-design.md). The direction is [`docs/writfold/mission.md`](../writfold/mission.md).
