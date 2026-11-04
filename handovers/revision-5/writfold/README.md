# Writfold — Rust implementation handover

Prepared for Tarek and the implementing agent · **3 November 2026 · revision 5**

**Writfold is settled; writfold.dev is registered by Tarek.** This refreshed package targets kctx `1c17b94be37af8f82f222baddf31435f14172e1a` and adopts the engineering playbook at `e7d991e990e3dff9c5d70e4327c0e90298e2f67e`. It supersedes the 31 October handover's baseline and single-context assumptions while retaining its Rust, Scribe, website and WSL requirements.

Start with `AGENT_START_HERE.md`, then `plan/11-workflow3-contract.md` and `plan/12-playbook-adoption.md`. The full contract is in `plan/`; execute the **25 stage files**, including P01O storage foundation, P03P portfolios, P08D design binding, P08S scribes and P09V views. Open `review-atlas.pdf` for the **48-page** visual companion or `index.html` for the offline navigator.

## What changed

**Revision 5 adds context, execution and learning as connected engineering graphs.** Read plans/17-23 for two Omnigraph graph families, the operational ledger, package composition, exact fingerprints, cross-repository execution, reviewed learning and the proposed conservative calculus. E01-E12 are new proof targets, not completed Lean declarations. Execute P14-P19 before final P11-P13.

The package has 25 stages, 32 gates, 37 Verus/protocol entries, 12 new Lean theorem targets and 148 adversarial scenarios. All product obligations remain not_run.


**Revision 4 selects Omnigraph as the primary backend target.** P01O/WF24 is an early blocking foundation gate. Plan/16 specifies immutable schema epochs, exact storage pins, native lifecycle, retention, signed migration and legacy import. Dolt is reference/import tooling and a fallback after an explicit evidence-backed design decision, not a second shipping writer. This supersedes revision 3's retain-Dolt recommendation.

The follow-up versioned-type probe is retained alongside the original comparison. It demonstrates preserved old reads and the cross-schema branch refusal; full epoch/WSL/server/recovery qualification remains not_run.


Revision 3 adds mandatory P09V: Obsidian/Markdown views, core Bases plus a pinned Dataview profile, and a renderer-neutral document contract for future views. Plans/13–16 define publication, readability and the updated storage decision/contract. A real local comparison ran 19 commands / 12 focused assertions; it does not qualify WSL, performance or persistence. The synthetic vault is in `examples/obsidian-vault/`.


- Inference semantics remain `2.0.0` and core reference wire 2. Knowledge workflows use schema 3; baseline RPC initialization uses protocol `"2"`; plan/16 requires explicit storage-profile negotiation or a versioned outer protocol before shipping.
- Named portfolios authorize several contexts without pooling evidence. Added claims have exact context/witness assignments, context retirement is explicit, and queries/history preserve context identity.
- Accepted design and concrete implementation binding are separate. Rebinding preserves design approval but cannot reuse another target's gate passes. Material changes require reopening and new acceptance; finalized history requires supersession.
- Historical schema-2 signed releases remain authenticated under their original bytes/domain. Pending old writes cannot resume through the new writer.
- Typed fix references, local diff-impact views and native reader descendant ownership are now baseline behaviors.
- The playbook adds an explicit adoption pin, shared engine-prepared CLI/RPC views, stamped asynchronous results, cancellation resource ownership, and companion continuity. No shared framework is introduced. Revision 3 admits a human view channel under the same prepared-view boundary.

## Current review evidence

The retained Lean report matches current source: **353 audited declarations**, 159 added, still within T01–T22. All listed inputs match the **334-case** conformance report, which records 11/11 decision mutants killed. The retained **740/740** behavioral report also matches the current source/test digest. These are checked report identities, not a fresh Lean proof run.

The original Dolt warning regression was fixed in `2da7036`; the package no longer tells the next agent to rediscover that completed fix. Stronger executable digest admission remains required for Rust. A later native reader fix disables detached event flushers in its private subprocess environment. The new storage evidence covers 13 selected knowledge cases; the full storage gate is deliberately false for that subset. The recorded one-file reader delta and regression hashes match current bytes.

Independent execution passed **737/740** tests; three native-reader paths were blocked by this host’s socket/process restrictions. The suite remains a recorded non-pass, with mandatory WSL/native follow-up. Independent checks and their limitations are recorded in `plan/01-review-findings.md` and `evidence/current/`. Older evidence is retained under `evidence/baseline-3134c8c/` and must not be treated as current. Lean/Verus/TLC/TLAPS and the complete Compose qualification were not executed by this refresh. Every Rust product gate remains `not_run`.

## Package map

| Path | Purpose |
| --- | --- |
| `AGENT_START_HERE.md` | Execution prompt and completion rules |
| `plan/01-review-findings.md` | Current evidence, closed findings and remaining gaps |
| `plan/02-architecture.md` | Crates, nominal types, ownership and surfaces |
| `plan/03-formal-assurance.md` | Lean/Verus/TLA+/TLAPS obligations and trusted boundaries |
| `plan/04-storage-protocols.md` | Publication, recovery, retention, replicas and restore |
| `plan/05-build-and-quality.md` | WSL/external BuildKit, pins, offline execution, no monster files |
| `plan/06-compatibility-and-tests.md` | Contract parity and 148 additional scenarios |
| `plan/07-brand-and-website.md` | Organization/site/Porkbun implementation runbook |
| `plan/08-execution-and-acceptance.md` | 32 mandatory gates and receipt admission |
| `plan/09-scribes-and-writs.md` | Supervised external scripts, capabilities and proofs |
| `plan/10-minimal-website-outline.md` | Minimal sections, copy and vector icon brief |
| `plan/11-workflow3-contract.md` | Authoritative portfolio/design/version/migration delta |
| `plan/12-playbook-adoption.md` | Pinned S1–S3/I1–I3 adoption and PB-Q gate crosswalk |
| `plan/13-view-channels.md` | View protocol, owned publication, V14 and M10 |
| `plan/14-obsidian-reading-design.md` | Reading design, plugin setup and native acceptance |
| `plan/15-storage-decision.md` | Approved Omnigraph-first decision and early qualification |
| `plan/16-omnigraph-storage-contract.md` | Schema epochs, signed storage profiles, migration and V15/M11 |
| `plan/17-engineering-graphs-and-layout.md` | Three semantic graphs, two graph families and physical layout |
| `plan/18-identities-fingerprints-and-cuts.md` | Nominal identities, canonical fingerprints and read cuts |
| `plan/19-skill-packages-and-composition.md` | Existing Markdown/scripts, dependencies and checked calls |
| `plan/20-execution-and-cross-repository.md` | Durable run/effect lifecycle and cross-repo changes |
| `plan/21-learning-and-promotion.md` | Scoped evaluation, review and publication |
| `plan/22-engineering-calculus.md` | Proposed judgments, E01-E12 and proof composition |
| `plan/23-engineering-extension-acceptance.md` | P14-P19, gates, migration and release requirements |
| `examples/obsidian-vault/` | Synthetic readable vault, Base and Dataview queries |
| `qualification/storage/` | Reproducible native comparison fixture and script |
| `stages/`, `registries/`, `templates/` | Executable work slices, obligation inventories and receipt schemas |
| `diagrams/`, `brand/` | Editable Mermaid and SVG/PNG diagrams; folded-W SVG variants |
| `reference/` | Exact current kctx archive, requirement sources and playbook source identities |
| `evidence/` | Retained review facts, logs, hash audits and atlas review |
| `tools/` | Plan/diagram generation and offline package validation |

`reference/kctx-1c17b94-source.tar.gz` contains the exact reviewed Git snapshot. It is the existing Python/Lean implementation, not a stub or completed Rust product. Private playbook material is not a website publication input. No repository push, domain change or deployment was made by this refresh.

## Check and continue

Run `python3 tools/check_engineering_examples.py` and `python3 tools/check_bundle.py` from the extracted directory (Python 3.9+). It validates file hashes, registry references, stage diagrams, local links and atlas structure. This is package validation; it cannot pass product gates. P00 checks whether the actual destination repo has advanced beyond the pinned source and reconciles the delta without discarding newer work.
