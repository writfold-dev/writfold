# Revision-4 contract adoption

The current authority is [`handovers/revision-4/writfold/`](../../handovers/revision-4/writfold/README.md), dated 2 November 2026. The supplied `writfold-rust-handover-version-4.zip` is **47,996,552 bytes**, SHA256 `0b1e43b6b4caaa091187c55511ddc7a69a21b3f681cf8879b71c6bb3b09d853d`. The parent verified and imported those immutable bytes. [Machine-readable adoption](../../qualification/handover-adoption.json) records archive, manifest and registry identities. Package integrity is not product qualification.

Read its AGENT_START_HERE, all 19 stage contracts, and plans 08 and 11–16. The normative stage/gate inventories are copied into the current [status ledger](../../qualification/status.json), including names and dependencies, with **WF00–WF24 required**. Registries and stage files remain immutable; current execution results belong in the separate ledger.

## Approved policy delta

- Omnigraph replaces planned production Dolt as the intended primary structured backend. Start with a supervised native companion and typed transport, not an embedded engine. Dolt remains isolated reference/import tooling, not a second normal writer. Any fallback requires an explicit evidence-backed design revision; uncertain writes never trigger automatic fallback or blind retry.
- P01O/WF24 follows P01 and blocks storage-dependent P04/P06 and downstream integration. Independent domain/calculus work may continue. Smoke fixtures do not pass WF24; integrated WF09/WF11/WF14/WF15 remain independently required.
- Each physical schema epoch owns a separate graph with immutable schema and historical query contracts. A release/read pin binds graph, commit, epoch, query and storage compatibility profile. Retain old readers and complete native roots; independently restore with original storage unavailable.
- Preserve all historical signature bytes and identifiers. New storage/release/approval envelopes have explicit version/domain boundaries and capability negotiation; never cast a graph commit into a legacy Dolt field. Semantic wire 2 and workflow schema 3 remain distinct from storage and RPC versions.
- Mandatory P09V adds prepared document projections, owned generation publication and native Obsidian reading qualification (WF22/WF23). V14/V15/M10/M11 and changed protocol assumptions need actual production/model evidence, not renamed old claims.

The tested candidate is Omnigraph 0.14.0, source `60bfd19db780fdff3a5178f959c1204143420114`, archive SHA256 `02bde3e3a2c6c8cf50dc9739a8b1041df87cfeaa6ab59f620e1d8e32acffd4d9`, CLI SHA256 `978e200a412e7b0db46657417d348569f97529c18b47e64687132e565d19dc64`. This identifies bundled local experiments, not an admitted server/runtime or complete WSL/native campaign. Existing operator storage, Compose/tool-image admission and reserve policies remain unchanged.

## Evidence applicability

`handover/` remains the historical baseline; no file or historical manifest is rewritten. The existing baseline runner and frozen contract/evaluation inventories still verify those historical bytes. All failed attempts and completion records remain retained.

Earlier P00/P01/P02 completion records remain under `historical_completions` in the ledger, with their exact receipt links. They do not imply revision-4 completion: P00 needs current source-bound reconciliation; P01 needs native Omnigraph CLI/server/runtime admission; P02 needs storage identities, new outer wire/signature/profile boundaries and fixtures. Active stage dispositions distinguish these unrun extensions and the P01O blocker. Existing Rust/proof work is neither discarded nor declared requalified.

The runner now recognizes `storage-foundation`→WF24, `omnigraph`→WF09, `legacy-import`→WF14, `views`→WF22, `obsidian`→WF23 and `storage-profile`→WF03. Unimplemented producers refuse. `dolt` is no longer a planned production gate command. `reference`/`native-smoke` retain their historical WF09 association with explicit Dolt-reference-only scope; they cannot qualify current Omnigraph WF09.

Release consumption requires all 25 gates exactly once and binds active contract and fixture bytes. Old receipts, scoped passes, missing producers and unrun obligations cannot qualify the new contract.

Integrated smoke evidence is retained separately: `v4-baseline-001.json` under the external qualification directory passed with scope `baseline-only`; all six newly recognized commands refused their unavailable producers; removing WF22, WF23 or WF24 from an in-memory candidate ledger caused independent aggregation refusal. Exact observations are in `~/.local/share/writfold/evidence/revision-4-gate-smoke-001.json`. These checks do not pass the full revision-4 stages or renew earlier proofs.

`unblocked-baseline-20261103-001.json` additionally exercises current manifest
and registry reconciliation: 258 immutable members, 19 stages, 25 mandatory
gates, 26 V/M obligations and 108 scenarios. The full baseline-only receipt
records 1,322 passing identity checks. The separate
`baseline-calibration-20261103-001/report.json` campaign uses real copied file
closures: the positive control passes, and changed manifest bytes, omitted
WF24, omitted V15 and a weakened P06 dependency each trigger their intended
refusal. No stage, proof or backend status is promoted by those checks.


The public website remains a separate repository, [writfold-dev/writfold-dev.github.io](https://github.com/writfold-dev/writfold-dev.github.io). Its publication state is recorded separately in the root README; site work does not pass backend, installation or release gates.
