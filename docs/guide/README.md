# Writfold guides

> **Status:** built in v0.6.0-dev.1 (guard rules read a command as the shell splits it and judge the repository it acts on, `gates.met` over an empty set refuses, `mode = "audit"` and the guard log, guard conformance cases, see [Hooks](hooks.md); the `normalized`, `prefix` and `heading` lookup tiers, `matched_on` and `find --eval`, see [kb-layout](kb-layout.md) and [validation](validation.md); `runs --idle` and `kb delta`, see [feeding-back](feeding-back.md); a delivery as a plan step's gate, `deliver:<mode>`, and the re-check of cited evidence on every read, see [run-graph](run-graph.md#a-delivery-as-a-steps-gate) and [delivery-modes](delivery-modes.md#on-a-plan-step); `writfold watch` in plain words by default, with the exact wording on `d`, see [Watching a run](watch.md)). Diagnostics for the files you write (the file, line and column, the source line, and a fix when one can be named) and `kb lint` built in v0.6.0-dev.2, see [errors](errors.md). Each page below says what is built and what is planned. Weft and Splicery are frozen: their adapters (the assessed gate, `kb edit apply`) keep building and their tests keep passing, with no new development. Tuft's `writfold tuft` subcommand was archived on 16 November 2026; retrieval receipts that earlier runs recorded are still read.

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
| [watch](watch.md) | Recorded run progress in eight read-only views, and the knowledge base (decisions, status, tracker, impact, stale) as a second mode; layouts, controls, refresh and colour themes | built (T5) |
| [skills](skills.md) | The bundled `writfold:kb` and `writfold:evaluate` skills, and skills that splice a plan template | built |
| [hooks](hooks.md) | What each Claude Code hook reads, writes and prints | built |
| [errors](errors.md) | When a file you wrote is wrong: reading a diagnostic, the codes, `kb lint`, what hooks show | built in v0.6.0-dev.2 (definitions; delivery and kind files planned) |
| [json](json.md) | The `--json` envelope: keys, exit codes and the compatibility rule | built in v0.5.0-dev.1 |
| [assurance](assurance.md) | What is proved, model-checked, tested and trusted | built; run-graph, gate and assess kernels proved |

A `toml` block in these pages is parsed by the real parser in a test (`crates/writfold-kb/tests/guide_toml.rs`). A block under a `<!-- planned -->` marker is a sketch of a format that is not built; the same test checks that today's parser still refuses it, so the marker cannot outlive the feature.

The design record, with every deviation and test, is [`docs/writfold/kb-v1-design.md`](../writfold/kb-v1-design.md). The direction is [`docs/writfold/mission.md`](../writfold/mission.md).
