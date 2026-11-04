# Current repository review — 1 November 2026

## Exact scope

Reviewed six commits between kctx `3134c8c722e46ea4bef4459b4b5517e985d34b78` and `1c17b94be37af8f82f222baddf31435f14172e1a` through GitHub comparison and a detached local checkout. Inspected the added Lean definitions/theorems/codecs/examples, Python twins and production callers, signatures, schemas, migration, authored projection, diff adapters, native reader changes and current reports. No source fixes or remote writes were made.

Also inspected private `private:playbook@e7d991e990e3dff9c5d70e4327c0e90298e2f67e` through GitHub. Its adopted patterns and exact source identities are recorded separately. The Gripsack build/site observations remain those pinned in revision 1; they were not requalified by this refresh.

This is a targeted contract and implementation-handover review, not an exhaustive security audit or new end-to-end proof. The original findings and failed native trace remain under `evidence/baseline-3134c8c/`. The current source archive replaces the old baseline archive; the earlier complete handover remains a prior file version.

## Changes and disposition

| Commit | Change | Handover consequence |
| --- | --- | --- |
| `2da7036` | Graph identity selects the Dolt version line | Original warning-contamination failure is repaired; retain regression, strengthen admitted tool identity for Rust. |
| `11f8911` | Typed fix references and local diff impact | Preserve the closed reference grammar, exact local endpoints, unmapped coordinates and retrieval-only meaning. |
| `6ea4831` | First-admission review and priorities | Carry explicitly open adapter/replica acceptance criteria forward. |
| `ac000fc` | Lean portfolios and design bindings | Map the expanded declarations, examples, strict codecs and exact assumptions. |
| `cbc3e63` | Workflow-3 production integration | Port named contexts, exact assignments, retirement, design binding/reopen, migration and protocol 2. |
| `1c17b94` | Private Dolt reader descendant lifetime | Own all children and private-root writes, not only the SQL-server process. |

## R2-F01: the extended model is concrete and versioned

`formal/Kctx/Portfolio.lean`, `DesignWorkflow.lean`, `WorkflowV3Codec.lean` and `WorkflowV3Reference.lean` have production counterparts in `semantics/portfolio.py`, `design_workflow.py`, `workflow_models.py`, `workflow_wire.py` and `workflow_reference.py`. The production knowledge/proposal services call these contracts. The inference model/checker constructors were not replaced.

The new proof report has 353 audited declarations, 159 more than revision 1, with no reported open obligations. The recomputed formal source digest is `a2ccd0a20dfe4c4e0e410cbee6554729e3fe73e523bc935857524078087c9fb3`, exactly matching the report. T01–T22 remain the registry families; the new entries include examples and codec laws, not 159 independent top-level soundness theorems.

All recorded inputs to the conformance report match: 32 Lean/tool files, 18 Python/harness files and three fixture files. It records 334 cases (203 fixture,112 generated,19 stateful), zero mismatches and 11/11 killed mutants. The current 740/740 behavioral report matches recomputed digest `e7fa422508aaf434619f1a04fbcc2ae79cdfa5c756b315cf0ecbd9062ace8777`. Some reports name an earlier parent commit because they were produced before commit; exact file hashes, not the parent label alone, establish correspondence.

**Limit:** Lean was not rerun here. The old EvidenceSound, authenticated-adapter, source-resolution and storage assumptions remain. Differential agreement is not cross-prover equivalence or whole-Python refinement. Writfold needs V12/V13 and the affected production/model bridges, not merely a port of old theorem names.

## R2-F02: superseded blocker, retained tool-identity hardening

The graph no longer hashes the entire warning-bearing stdout. `graph/inputs.py` now extracts `^dolt version \\S+`; warning/no-warning regression coverage is present. Do not keep revision 1's instruction to fix that already-fixed behavior.

The current regex selects the first matching line. It does not establish a unique, fully validated version or bind executable bytes. `services/knowledge.py:_toolchain_bytes` still captures raw Dolt version output. These remain reasons for Writfold's one strict admitted engine-identity adapter and tool digest, not evidence that the original graph failure persists. Reject ambiguous output and changed executable bytes even when version spelling matches. Preserve the actual source-change guard.

## R2-F03: better native evidence, still scoped

The workflow-3 storage report contains 13 passed case rows and `executed_cases_passed: true`, with stable source during execution. It ran selected section `knowledge`, so `gate_pass: false` is intentional. Do not turn it into a full-storage pass or describe its selected cases as failing.

Exactly one listed runtime file differs at current HEAD: `adapters/dolt/read_session.py`. The runtime linkage records its old/new digests and the subreaper regression; both current source and test hashes match. It reports eight focused native passes and an installed CLI/RPC/EOF check. Both temporary smoke harnesses are explicitly **not retained**; their hashes and summaries support historical observations but are insufficient for an exact rerun. Writfold must retain its executable smoke harnesses.

Older general storage/remote inventories have 43 modified listed runtime files plus nine new runtime modules at this HEAD. Their historical results remain useful precedents but are not current workflow-3 transport qualification. The new campaign does not resolve the separately documented ordinary unborn-branch restore case.

## R2-F04: migration is an authority boundary

The new signature domain is `kctx-operation-v3` plus NUL; historical v2 uses only its original domain. Schema-2 completed history is readable as the `legacy` context, but pending v2 writes cannot resume, and v2 operations cannot follow v3 history. Authored physical schema 1 is distinct from workflow version 3; design-v3 material identities cannot reinterpret legacy custom metadata. Policy/backup/migration containers do not all become schema 3.

A Rust port with one loose 'version' field would break this separation. P02, P03P, P08D and P10 now require explicit variants, byte fixtures, refusals and migration paths. Rebranding cannot change old signed inputs.

## R2-F05: open adapter requests remain open

The current first-admission record marks FR5/FR6 implemented and FR4 diagnostics improved, with C1 migration guidance split by actual status. Its older paragraphs are historical dispositions; do not read them as proof that portfolios are still absent.

Independent work remains for canonical new-bootstrap policy identity (FR3), aggregate malformed-incident isolation (FR8), ordinary unborn restore and owned-ref cleanup (FR9), typed missing native generations (FR10), stable managed locks (FR11), and incremental published-closure replication/read-only replicas (FR7/FR12). This review confirmed the corresponding code shape: bootstrap hashes raw bytes, aggregate readers omit the tolerant diagnostic path, read construction claims local state/ownership, and restore has the described direct-fetch structure. Plan/11 gives their required Rust acceptance criteria. No request permits a weaker inference or admission rule.

## R2-F06: refactoring need increased

`services/knowledge.py` is now 1,834 raw lines; `services/proposals.py` is 1,082. The new pure workflow modules are useful boundaries, but production orchestration remains broad. Keep the handover's domain/kernel/wire/protocol/adapters split, explicit portfolio/design modules and mechanical size/dependency gates. Porting these classes wholesale would recreate the problem.

## R2-F07: playbook adoption strengthens integration details

The playbook supports the existing pure kernel and explicit protocol approach. It changes no inference rule. This refresh adds a pinned adoption record, prepared immutable presentation views, exact preview-to-approval identity, owner/session/request-generation stamps, cancellation capacity ownership, and evidence-producer versus evidence-admitter separation. No new general framework, frontend or wholesale proof fleet is required.

The upstream companion gate applies when changing the playbook itself. Writfold maintains its own atlas and diagrams under WF21. Its public website must not accidentally publish the private playbook. No upstream playbook changes were made.

## Independent execution in this refresh

Strict Pyright with the selected interpreter, Ruff lint/format over 136 runtime files, the positive consumer and all 12 intended ID/enum misuse sites passed. Package/version contract checks passed; those checks inspect the retained proof report, not execute Lean.

An initial Pyright invocation selected the host interpreter and lacked package dependencies. It was corrected with the explicit isolated interpreter; the successful run is the one qualified above. No product code or lint configuration was changed.

The independent full behavioral run used the locked existing Python environment, current source, actual Dolt 2.6.1 and TERM=xterm-256color. Exact counts, failures and environmental limitations are recorded in `evidence/current/behavior.json` and its raw log. This host run is supplemental; it does not substitute for the destination's Compose/WSL/native campaign. See the final execution note appended below.

### Final independent execution result

**737 of 740 tests passed; one failure, two errors, zero skips.** The suite exits nonzero and is recorded as such. Two private-reader tests encounter `PermissionError(EPERM)` while constructing an AF_UNIX socket. The descendant-cleanup test also cannot inspect its `/proc/<pid>/task/<pid>/children` path in this host. A focused RPC rerun captures `E101: Dolt read-session executable or transport unavailable` at `graph.uses`; the upstream test then raises `KeyError('result')` because it expects a successful envelope. A direct socket-construction probe independently confirms EPERM.

These observations identify host restrictions, not evidence of a newly demonstrated production regression. They also do not qualify the native behavior beyond the blocked operation. Keep the full native acceptance campaign mandatory on the admitted WSL/Linux Compose host; do not silently skip these paths or promote this run to a complete pass. The original version-warning regression and portfolio/design tests passed in this run. No Lean, Docker, Verus, TLC or TLAPS run is claimed. Exact logs, executed diagnostic harness and assessment are retained in `evidence/current/`.

Final GitHub comparisons on 1 November found both reviewed pins identical to their repositories' `main` heads: kctx `1c17b94` and playbook `e7d991e`. Recheck at implementation start.

## Revision 4 decision

This chapter describes the unchanged kctx baseline and its Dolt evidence. The new Rust target is Omnigraph under plans/15–16. Retained baseline passes are reference evidence only, not qualification of the replacement engine.
