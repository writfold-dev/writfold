# Implementation status

What this repository holds and how it is checked, as of the public/archive
split of 16 November 2026. The current authority is [mission.md](mission.md);
its [Component status](mission.md#component-status-15-november-2026) table is
the standing of each component. What you can run today, with the evidence
for each verb, is in the README's
[What works today](../../README.md#what-works-today) table and in the
[guides](../guide/README.md).

## The binary

`writfold` has two commands, `kb` (the knowledge-base verbs and the Claude
Code plugin hooks) and `watch` (a read-only terminal view), plus `--version`
and `--help`. `writfold --version` prints `writfold <version>` only.

## The workspace

| Crate | Role |
|---|---|
| `writfold` | the binary |
| `writfold-app` | the `kb` verbs' argument handling and output |
| `writfold-kb` | the KB: layout, index, reads, writes, runs, gates, plans, deliveries, guards and hooks |
| `writfold-tui` | `writfold watch` |
| `writfold-core` | pure decision kernels; part of it is translated to Lean and proved ([aeneas.md](aeneas.md)) |
| `writfold-runtime` | native Git reads and the KB's Git calls, contained filesystem reads, owned subprocess groups |
| `writfold-sources` | citation, front matter and Markdown codecs, and the authored-material revision hash |
| `writfold-domain` | shared checked values (Git object IDs, digests, byte counts) |
| `writfold-xtask` | development-only source, dependency-graph and budget checks |

Models: `spec/KbWrite.tla` (KB writes) and `spec/RunGraph.tla` (the run
graph), each with mutants and probes; see the
[assurance guide](../guide/assurance.md).

## Gates

`scripts/gate quality` (rustfmt, both clippy runs, the xtask checks and their
calibrations) and `scripts/gate unit` (the workspace tests) run on main
before a push; [quality-gate.md](quality-gate.md) describes both. The proofs
gate (`docker compose -f compose.proofs.yaml run --build --rm proofs`) runs
when `crates/writfold-core` or `proofs/` changes.

## Archived

The edit trial flow and `serve`, `writfold tuft` and its adapter,
`writfold.agent/1`, Dolt publication and Omnigraph, with their crates, Lean
models, TLA+ models, qualification records, benchmark pilots, earlier
handovers and the earlier text of this page, moved to the private archive
`writfold-dev/writfold-archive`: under `public-split-2026-11-16/` with their
history, and as the last buildable public tree on the archive's branch
`public-main-pre-split` (tagged `pre-split-2026-11-16` in this repository).
The last release that includes them is `v0.5.0-dev.9`.

### Removed on 21 November 2026 (#54)

| Removed | Where it lives now |
|---|---|
| Tuft (the `tuft` crate; its resume cursor moved into `writfold-kb`, unchanged) | the archive's branch `public-pre-cleanup-2026-11-21` (commit `85611cf`, tag `pre-cleanup-2026-11-21`) |
| Splicery's `kb edit apply` / `edit recover` and the vendored kernel | Splicery: `writfold-dev/splicery`; the adapter: the same archive branch |
| Weft's assessed gate, the `assess` kernel and `proofs/Writfold/Assess.lean` | Weft: `writfold-dev/weft`; the gate, kernel and Lean: the same archive branch |
| The Verus formalization of `writfold-domain`, xtask's Verus audit and the proof-trust manifest | the same archive branch |
| The runtime's staged replacements, writer locks and synced writes | the same archive branch |

Edit and retrieval receipts already in a KB stay readable (`resume`, and as checklist evidence); nothing writes them now. The tools image still carries Verus until its next re-bootstrap.
