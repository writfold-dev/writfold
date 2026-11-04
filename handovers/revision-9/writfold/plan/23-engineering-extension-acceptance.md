# Engineering extension execution contract

Revision 5 adds six stages P14-P19 before final product qualification/package/launch P11-P13. Numeric IDs preserve historical references; registry order and dependency edges, not numeric sorting, determine execution order. Existing stages and gates remain mandatory. The new scope does not turn v4 planning evidence into executed Rust assurance.

## Stages and gates

| Stage | Deliverable | Blocking gate |
| --- | --- | --- |
| P14 | Realm layout, ledger/head protocol, canonical identities and fingerprint profile | WF25 |
| P15 | Imported Markdown/script packages, locks, progressive loader, typed procedure composition | WF26 |
| P16 | Qualified effect adapters, shared resource ownership, recovery and cancellation | WF27 |
| P17 | Complete cross-repo author-change procedure with design, review, tests and landing | WF28 |
| P18 | Event-triggered learning, frozen evaluation and authenticated promotion | WF29 |
| P19 | Activity projection/cuts, Obsidian dossiers, backup/restore and migration | WF30 |

WF31 is the final cross-cutting calculus/refinement aggregation gate, owned by P11 only. P14-P19 contribute separately scoped receipts without requiring the future aggregate to pass. It requires E01-E12, V16-V21, M12-M16 and composition receipts on exact production inputs. Each new stage completes its own gate and named local proof/implementation obligations. Shared model bridge evidence may be produced in later dependent stages; the proof registry stays incomplete until all of its obligations exist. A local stage receipt never claims whole-system refinement. Stages may develop concurrently only where their declared dependencies and resource ownership permit. No agent can mark a required gate pass because the diagram or specification exists.

## Implementation ownership

Extend existing boundaries with focused modules: domain/engineering/{identity,package,procedure,run,effect,learning,cut}; kernel/engineering/{typing,enabled,reuse,admission}; protocol/{ledger,ownership,effects,invocation,promotion,projection}; store/{streams,objects,pins}; app/{skills,runs,learning,dossiers}. Effect adapters own Worktrunk/Git/forge/test/rollout IO; runtime owns process and filesystem enforcement. Omnigraph adapters own corpus/activity codecs and immutable epoch pins. View renderers consume prepared values without queries or policy decisions.

A separate crate is justified only by an actual stable dependency boundary. Maintain nominal private-constructor types, 400-line module/80-line function budgets, explicit exceptions, rustfmt/clippy and repository conventions. Do not build a generic distributed scheduler, a new scripting language, a multi-backend abstraction or an agent-host replacement to deliver the first slice.

## Executable first slice

Use three synthetic repositories (API, Rust SDK, CLI) with an additive pagination change, an intentionally strict legacy decoder and a real regression. Provide the author-change skill as Markdown plus a small admitted script, a nested worktree procedure, a conditional language route and independent test/review gates. Exercise a full local run, interruption/resumption, proposed learning improvement, held-out evaluation and new procedure admission. Real forge writes run only in a dedicated authorized qualification repository/profile; local simulation cannot pass remote semantics gates.

Complete the manual-review/explicit-grant path first. Standing grants can automate actions within scope; do not add blanket human prompts for every load or retry. Any automated learning admission still uses the same evidence and policy floor. Deployment support is an explicit adapter capability, not a default implication of merge authority.

## Migration from v4 and existing skills

Inventory the exact existing realm and completed release. Add the activity family in a new declared deployment without changing retained corpus schema epochs in place. Introduce engineering envelopes and ledger format explicitly. Preserve all v4 knowledge identities, approvals, source bytes, selected contexts and historical interpretations. New corpus procedure/learning types require a new corpus schema epoch under plan/16. Never rewrite the old graph to pretend it always had the new schema.

Existing skills import as candidate packages with a compatibility report. Advisory-only packages are usable only under that declared assurance. Sidecars and dependency locks are reviewed artifacts. Source aliases can move only through a new selection; running activations stay pinned. Legacy session hooks become observation/proposal clients; they cannot publish by writing an approved flag.

Upgrade/restore validates all required closure, including packages, run heads, reservations, unresolved effect identities and corpus evidence. Do not resume effects from a stale backup automatically. Establish a new instance incarnation, reconcile remote state and explicitly continue or block. Verify that rebuilding activity from the ledger reproduces exact typed projections and honest completeness metadata.

## Required acceptance evidence

Every new gate needs positive execution, meaningful counterexamples, explicit assumptions, exact production symbols, tool/environment identities and raw result hashes. All required scenarios X109-X148 are individually accounted for; tests may cover several scenarios only with explicit mappings. Required skips fail qualification. A semantic mutant is killed only by the intended property violation.

Package checks in `tools/check_bundle.py` validate this handover's integrity and consistency only. `tools/check_engineering_examples.py` validates illustrative codec/reference fixtures only. Neither passes a product gate or proves the calculus. The independent review record names its examined files, findings, dispositions and unresolved limitations.

## Release claims and human review

The website and atlas must distinguish the three semantic graphs, two logical graph families, independently retained physical epochs, the execution ledger and derived activity index. Publish actual assurance status from receipts; retain `not_run` until implemented. Show the exact current release/procedure/run cut in human dossiers. Preserve the minimal website design and read-only Obsidian channel; add concise engineering examples instead of marketing an autonomous verified agent.

Do not publish private playbook/source references in public Pages output. A new ZIP must use an explicit version name, standard DEFLATE/STORE entries, safe relative paths, no symlinks, a full SHA-256 inventory and independent fresh extraction verification. The atlas must be regenerated from current registries and source contracts, visually reviewed page by page, and included in the ZIP.

Registry semantics: `stages[].gates` and `gates[].stages` are exact forward/reverse membership. Global quality rules still apply to every changed module. `proofs` lists the entries to which a stage contributes; its gate defines the scoped completion obligation. P11 includes all new proof/model/scenario IDs and aggregates whole-system completion.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.
