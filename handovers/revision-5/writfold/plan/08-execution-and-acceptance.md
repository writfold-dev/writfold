# Execution order and acceptance

## Critical path

P00 -> P01 -> P02 -> P03 -> P03P. P01O qualifies the Omnigraph foundation after P01. Protocol design P04 requires P02 and P01O; storage-independent domain/calculus work may proceed while foundation failures are investigated. Adapters P05/P06 require the typed/build boundary and may develop before complete kernel proofs, but authoritative integration P07 requires P03/P03P/P04/P05/P06. P08 implements workflows/catalog; P08D implements design binding/reopening; mandatory P08S adds supervised external scribes and protocol proofs before P09 completes retrieval. P09V adds prepared views and the native Obsidian profile after P09. P10 closes the baseline migration/retention/transport. P14-P19 then implement engineering identities/ledger, skills, effects, cross-repo execution, learning and exact-cut views. P11 qualifies the complete product. P12 packages/cuts over; P13 finishes the site and launch. Site design may proceed earlier without claiming product readiness.

This is a dependency graph, not authorization to spawn additional agents or run conflicting mutations concurrently. Use the local repository's actual collaboration policy. Keep each commit reviewable; proof difficulty is not a reason to build an enormous one-shot rewrite.

## Mandatory gates

`registries/gates.json` is the normative machine-readable inventory. Every gate starts `not_run`; the implementing agent records receipts as work completes. WF01/WF02 quality rules apply to every later code stage. A stage cannot pass with skipped required tools or unimplemented planned command names.

| Gate | Meaning | Typical owner stage |
| --- | --- | --- |
| WF00 | Fixed, source-identified baseline and reconciled reports | P00 |
| WF01 | Admitted tools, Compose build, offline execution and storage preflight | P01 |
| WF02 | Nominal values, architecture/size/lint/unsafe/dependency rules | P02 and every later stage |
| WF03 | Strict wire, canonical signature bytes, lossless compatibility | P02 |
| WF04 | Lean registry and complete three-host correspondence | P03 |
| WF05 | Selected production Rust functions verified by Verus | P03, P07–P10 |
| WF06 | Positive TLC models and calibrated negative cases | P04, protocol changes |
| WF07 | TLAPS invariant/bridge obligations complete | P04, protocol changes |
| WF08 | Source, citation, filesystem and Git adapters | P05 |
| WF09 | Native Omnigraph, epoch/commit reads, provision/upgrade/supervision | P06 |
| WF10 | Exact authenticated plans and candidate admission | P07 |
| WF11 | Real-process publication/recovery/race qualification | P07, P10 |
| WF12 | Authored workflows, catalog and provider neutrality | P08 |
| WF13 | Complete bounded CLI/RPC/query behavior | P09 |
| WF14 | Accounted migration, retention, sync and independent restore | P10 |
| WF15 | Matched resource/outcome qualification and parity | P11 |
| WF16 | Installed artifact, offline bundle, cutover and release provenance | P12 |
| WF17 | Tested exact-release static website and controlled publication | P13 |
| WF18 | Scribe protocol, sandbox/broker, real plugins and M07/M08/V11 proofs | P08S |
| WF19 | Exact portfolio authority, contexts, history and V12 integration | P03P, P07, P09, P10 |
| WF20 | Design binding/reopen, material projection, V13 and M09 | P08D |
| WF21 | Pinned playbook adoption and current Writfold companion | Every affected stage, P11/P13 |
| WF22 | Faithful view projection, V14/M10 and owned publication | P09V |
| WF23 | Real Obsidian/CLI/plugin and reading profile | P09V/P11 |
| WF24 | Mandatory Omnigraph foundation campaigns; smoke probes insufficient | P01O/P06/P11 |
| WF25 | Engineering identities, exact fingerprints and durable ledger | P14/P11 |
| WF26 | Skill packages and checked procedure composition | P15/P11 |
| WF27 | Effect adapters, resource ownership and honest recovery | P16/P11 |
| WF28 | Cross-repository author-change lifecycle | P17/P11 |
| WF29 | Frozen learning evaluation and authenticated promotion | P18/P11 |
| WF30 | Engineering read cuts, dossiers and restoration | P19/P11 |
| WF31 | Final composed engineering calculus/refinement | P11 only |

## Gate runner contract

Create `scripts/gate` with explicit subcommands: `baseline`, `quality`, `unit`, `lean`, `verus`, `model`, `tlaps`, `conformance`, `sources`, `storage-foundation`, `omnigraph`, `legacy-import`, `publication`, `workflows`, `transport`, `migration`, `evaluate`, `runtime`, `package`, `site`, `scribes`, `portfolio`, `design`, `companion`, `views`, `obsidian`, `storage-profile`, `engineering-ledger`, `skills`, `effects`, `cross-repo`, `learning`, `engineering-cuts`, `engineering-calculus`, `release-required`. Each launches the correct Compose service or executes admitted tooling inside it. CLI examples in this plan name future contracts; do not paste them into the old Python repo and call failure a product defect.

Every receipt records gate ID, exact command/argv, source commit plus content-tree identity, dirty status, lock/tool/fixture hashes, target/features, start/end time and clock source, exit, assertion/obligation counts, skips, artifacts and raw log hashes. Store out-of-tree attempt directories so report generation does not dirty the tested source. Keep failed attempts immutable and link successful replacements. A summary generator may not convert nonzero into pass or hide skips.

The release aggregator evaluates **all** mandatory gates for the candidate's dependency closure. It rejects missing receipts, unknown schema, source mismatch, stale assumption registries, artifact mismatch, unapproved waiver or any required blocked/failed/not-run gate. Fresh formatting and unit tests do not renew a stale persistence receipt. A doc-only change can reuse evidence if the assessed closure is unchanged and that linkage is recorded.

## Decision and deviation policy

Routine module layout, helper naming and test implementation are agent decisions within this contract. Resolve missing information from source/config before asking Tarek. Material unresolved choices: account/repository destination; real production credentials/enterprise host; an incompatible semantic/storage migration; a proposed relaxation of a required proof or supported safety profile. Prepare the concrete change/evidence before asking for approval when that approval is actually needed.

Every deviation entry identifies requirement, reason, safe supported behavior, affected consumers/proofs, migration, test coverage and acceptance effect. No silent deferral of a required proof. A scoped local release may explicitly leave enterprise transport and unavailable real-corpus evaluation unqualified, but its docs must not advertise those guarantees. Failure of a public-product gate is not fixed by calling it “optional”.

## Completion report format

Report product/artifact versions; final source and bundle hashes; platform/runtime dependency closure; All 25 stages, including P14-P19 before final P11-P13, and WF00–WF31 disposition; requirement Q/D/R crosswalk; Lean/Verus/TLA/TLAPS summary with assumptions; real-process/restore/migration results; performance/outcome observations; remaining limitations; installed-use example; and website build/deployment state. Link exact receipts, not a wall of transient terminal output.

No final claim of complete Rust implementation until all required in-scope gates pass. A compiled CLI scaffold, static site, verified leaf or passing reference corpus alone does not meet that criterion.

Revision 4: P01O/WF24 admits the early contract; integrated WF09/WF11/WF14/WF15 and V15/M11 still require their own evidence. A failed foundation blocks storage-dependent integration. Backend fallback is an explicit design revision, never a hidden runtime retry path.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
