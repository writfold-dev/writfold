# Writfold — Rust implementation handover

Prepared for Tarek and the implementing agent · **4 November 2026 · revision 9**

**Writfold is settled; writfold.dev is registered by Tarek.** This refreshed package targets kctx `1c17b94be37af8f82f222baddf31435f14172e1a` and adopts the engineering playbook at `e7d991e990e3dff9c5d70e4327c0e90298e2f67e`. It supersedes the 31 October handover's baseline and single-context assumptions while retaining its Rust, Scribe, website and WSL requirements.

**Revision 9 corrects the scope of discovery.** [Plan 30](plan/30-corpus-discovery.md) defines one read-only find across authorized, qualified corpus kinds. Incidents are the first benchmark. Atlas pages 67–68 show a mixed design/review/procedure query and explain Metamath.

**Revision 8 adds [similar-incident discovery](plan/28-similar-incident-discovery.md), a [pinned Omnigraph capability review](plan/29-omnigraph-capability-fit.md), and a [formal qualification packet](qualification/similar-cases/README.md).** The early citation queue still governs work. Atlas pages 64–66 cover the design and evidence.

**Revision 7 records Tarek’s design approval for the five Run Watch screens.** [Plan 27](plan/27-run-watch-and-session-handoff.md) defines the proposed session UX, run discovery, crash-safe intake and reader boundary. The [agent driver candidate](examples/run-watch/session-driver/SKILL.md) and [approved screens](examples/run-watch/approved-screens.pdf) are packaged. Atlas pages 57–63 follow the flow from chat to WSL.

Start with [PRIORITIES.md](PRIORITIES.md). The first new deliverable is **P03C / WF32**, the Lean citation and admission rules. Next comes **P05C / WF33**, the compact reader and checked production admission path. Read plans/24-26 for those two slices. The 27 stage files describe the full release, including inherited work. For human review, start on pages 2-7 of the [56-page atlas](review-atlas.pdf).

## What changed

**Revision 6 places proof before admission.** Every authoritative proposal transition requires an independently selected, Lean-qualified rule profile and a Lean-checked instance certificate and witness for that exact proposal. The revision introduces CIT01-CIT08 and V22/V23, includes a reproducer for ambiguous copied lines and puts presentation experiments in the later queue.

The current inventory contains **27 stages, 34 gates, 39 Verus/protocol entries, 164 scenarios, 12 engineering Lean targets and 8 citation/admission Lean targets**. Proof and product receipts for the new work are pending.


**Revision 5 adds context, execution and learning as connected engineering graphs.** Read plans/17-23 for two Omnigraph graph families, the operational ledger, package composition, exact fingerprints, cross-repository execution, reviewed learning and the proposed conservative calculus. E01-E12 are new proof targets, not completed Lean declarations. Execute P14-P19 before final P11-P13.

The revision-5 baseline had 25 stages, 32 gates, 37 Verus/protocol entries, 12 new Lean theorem targets and 148 adversarial scenarios. All product obligations remain not_run.


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
| `plan/06-compatibility-and-tests.md` | Contract parity and 164 additional scenarios |
| `plan/07-brand-and-website.md` | Organization/site/Porkbun implementation runbook |
| `plan/08-execution-and-acceptance.md` | 34 mandatory gates and receipt admission |
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
| `PRIORITIES.md` | Immediate task, stop conditions and deferred experiments |
| `plan/24-citation-calculus.md` | Typed binding, comparisons, conservative extension and Lean statements |
| `plan/25-proof-first-admission.md` | Common admission floor, per-kind profiles, receipts and failure handling |
| `plan/26-compact-citations-and-followups.md` | Reader contract, duplicate-line regression, benchmark and later views |
| `examples/citations/` | Reproducible historical-versus-current fixture |
| `examples/obsidian-vault/` | Synthetic readable vault, Base and Dataview queries |
| `qualification/storage/` | Reproducible native comparison fixture and script |
| `stages/`, `registries/`, `templates/` | Executable work slices, obligation inventories and receipt schemas |
| `diagrams/`, `brand/` | Editable Mermaid and SVG/PNG diagrams; folded-W SVG variants |
| `reference/` | Exact current kctx archive, requirement sources and playbook source identities |
| `evidence/` | Retained review facts, logs, hash audits and atlas review |
| `tools/` | Plan/diagram generation and offline package validation |

`reference/kctx-1c17b94-source.tar.gz` contains the exact reviewed Git snapshot. It is the existing Python/Lean implementation, not a stub or completed Rust product. Private playbook material is not a website publication input. No repository push, domain change or deployment was made by this refresh.

## Check and continue

Run `python3 tools/check_revision6.py`, `python3 tools/check_engineering_examples.py`, `python3 tools/check_run_watch.py`, and `python3 tools/check_bundle.py` from the extracted directory (Python 3.9+). It validates file hashes, registry references, stage diagrams, local links and atlas structure. This is package validation; it cannot pass product gates. P00 checks whether the actual destination repo has advanced beyond the pinned source and reconciles the delta without discarding newer work.
