# Writfold implementation

Writfold is the Rust successor to an earlier private Python prototype, kctx. This public repository contains only the Rust product, its Lean/TLA+ models and current planning; the prototype code, earlier handovers and their reference material are retained in a private archive.

## Mission and authority

- **The current authority is [`docs/writfold/mission.md`](docs/writfold/mission.md)** (owner decision, 11 November 2026): Git-native project continuity for agents. A cold session on any machine answers why / status / tracker / impact questions, each answer pinned to a source and marked when that source has moved. Git text is the source of truth; a local SQLite file is a derived index that can always be rebuilt. The v1 design is [`docs/writfold/kb-v1-design.md`](docs/writfold/kb-v1-design.md).
- **Acceptance test:** the owner replaces the private prototype with the `writfold` binary for real work. Benchmark pilots are paused; do not start new ones unless the owner asks.
- **Frozen, working demos, no new development:** the edit trial flow and `serve`'s trial methods, Weft, Splicery, Tuft's query language (v0), `writfold.agent/1` and cross-repository contract pins, the Dolt writer and DINT02 publication, and Omnigraph. Keep them building and their tests passing; fix them only when a change elsewhere breaks them.
- **History, not obligations:** `handovers/` (revision 5 and revision 9) stays immutable and is not edited. The WF00–WF31 gates, V/M entries, E01–E12, the 148 scenarios, `docs/writfold/agent-boundary-plan.md`'s delivery order and `qualification/status.json` record what was done; they are not live work items. Never relabel an earlier receipt or convert a skipped, blocked or unrun obligation into a pass.
- Adopt private `private:playbook@e7d991e990e3dff9c5d70e4327c0e90298e2f67e` only at that pin. Do not publish private reference material or edit the playbook. The prototype's knowledge-base layout may be read for compatibility; describe formats in Writfold's own words and never copy its code or prose.
- Preserve historical identifiers, signed bytes, schema namespaces, citations and `refs/kctx/*`.

## Engineering and builds

- Linux/WSL x86_64. Target a static musl Rust executable; rustls-only dependencies; Git remains an explicit native dependency. No server or companion process on the product path.
- Keep pure decision logic IO-free in `crates/writfold-core`; prove a kernel with Aeneas (`proofs/`, `docs/writfold/aeneas.md`) only where it earns it. One TLC model per real protocol with crash points. Prefer property tests, kill/fault-injection tests and the "removing this check fails a test" discipline.
- Apply the size budgets in `crates/writfold-xtask` (writfold-app modules ≤250 lines, functions ≤80, tests included); no `unwrap`/`expect`.
- Before pushing main, run `scripts/gate quality` and `scripts/gate unit` from main (never from agent worktrees; run `git ls-files -z | xargs -0 touch` first). Record the receipt paths in the commit message; do not add per-component `qualification/status.json` entries or receipt-index commits. Running them as `writfold kb gate run quality|unit` (definitions in `.writfold/gates/`) records KB receipts, and `writfold kb gate lines` prints the commit-message lines.
- Build locally through Compose/BuildKit. Reuse the operator's storage binding and admission floors; never relocate storage, prune unrelated resources or install publishing secrets on build workers.
- Update current documentation alongside implemented changes. Public website copy may describe only exercised capabilities; anything else is labelled planned.
