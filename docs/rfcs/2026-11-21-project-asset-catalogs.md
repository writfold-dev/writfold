---
rfc: "2026-11-21-project-asset-catalogs"
title: "Project asset catalogs as reviewable Git projections"
status: accepted
implementation: not-started
authors: [astra]
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/84
milestone: v0.6.0-dev.15
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Project asset catalogs as reviewable Git projections

Let a project describe **where its inventory is and how to read it**. Turn that snapshot into a reviewable diff of ordinary Writfold asset files. `find`, `context`, `impact`, `record` and `propose` continue to consume the reviewed records, not a provider. A catalog is not an extensible record grammar or an allowlist of asset subtypes.

## Proof obligations first

In `crates/writfold-core::catalog` (new module), classify one desired/current slot with `action(wanted, ownership, equal) -> Keep | Create | Replace | Remove | Conflict`. Ownership is absent, this catalog, or foreign; a foreign slot with no desired row stays untouched. Fold over the *whole* input, not just successfully parsed rows: `verdict(complete, conflict, changed) -> Invalid | Drift | Clean`. Global collisions and missing declarations for previously managed rows contribute faults. An invalid plan yields **no patch**, including no good prefix.

| Lean theorem to establish | Mutant that must fail |
|---|---|
| `clean_iff`: clean exactly when enumeration/validation was complete, no conflict, no changes | ignore one of the three facts |
| `partial_never_patches`: an unreadable input or an ownership/identity conflict anywhere forbids emitting any patch | treat missing as empty, or emit a good prefix |
| `create_only_absent`, `replace_remove_only_owned`: create requires absence; replacement/removal requires this owner; a foreign collision never becomes a write | overwrite foreign, delete foreign, or let the last claimant win |
| `owned_delta_exact`: for owned slots, absent desired means remove, equal means keep, unequal means replace; absent/absent stays put | skip absent desired, or rewrite unchanged |

Prove the aggregate fold actually preserves faults and changes before wiring it into either verb. Gate: extraction/drift, `lake build`, axiom audit, mutants, refreshed tauten claims. Tested, not proved: TOML/JSON parsing and source spans, Git-tree enumeration, path containment, output comparison, patch encoding/application, and rendering. No product-side tracked writes or new crash protocol; `KbWrite` and the index publication model are unchanged.

## What exists, and what does not

| Surface | Authority today | Decision |
|---|---|---|
| Project asset IDs, subtypes, aliases, locators, owners, projects, verification dates | Git JSON in `inventory/assets/`; [`parse/json.rs`](../../crates/writfold-kb/src/parse/json.rs) retains extra fields | Project data. Do not add Rust variants for services, dashboards or a team's new subtype. |
| Ticket/investigation `assets`, relationship endpoints, ticket IDs, signatures and free-text alerts | [Typed write planners](../../crates/writfold-kb/src/write/); asset references require exact indexed slug IDs, while an investigation can name tickets/signatures that do not exist yet | Keep these distinctions. An alert string is not thereby an asset, nor is a lookup alias a valid write ID. |
| Record shapes, proposal codec, citation tokens, lifecycle/evidence rules | [KB layout](../guide/kb-layout.md), typed writes and proof kernels | Product contracts. Declaring a catalog must not create a verb, change a lifecycle, or grant evidence. [Separate CLI decision](2026-11-21-record-option-ownership.md). |
| Gates, guard rules, deliveries, plan templates, citation kinds and attached stores | Existing `.writfold/` declarations; [`kb policy`](../guide/policy.md) and [store guide](../guide/stores.md) | Reuse the one-TOML-file-per-definition convention, not those loaders' semantics. A store pins cited material; it does not supply asset identities. |
| Lookup/graph index | [Derived SQLite schema](../../crates/writfold-kb/src/index/schema.rs); `claims.path` is a primary key, aliases and edges resolve against winning claims | Rebuild from the same files. No foreign SQLite attachment, provider lookup on a hook, or many-claims-per-path migration. |

**Observed**, released `v0.6.0-dev.13`, scratch Git KB: `find --kind asset` listed `mix-station` with free-form subtype `fermenter`; `context brewdeck` resolved its alias. An object and an array under `catalog/` counted as files, not assets. An investigation naming `mix-station` succeeded; adding `not-in-catalog` refused (exit 3). There is no `record asset`; today's [import guide](../guide/kb-layout.md#importing-existing-material) says to generate the files and validate them. The missing capability is a *shared, reproducible mapping*, not storage of a new kind of entity.

## Declaration and source boundary

Example of the proposed `.writfold/catalogs/brewery.toml` (name is the file stem, a Writfold slug):

```toml
schema = "writfold-asset-catalog/1"
input = ".writfold/catalog-inputs/equipment.json"
items = "/equipment"

[fields]
id = "/tag"
name = "/label"
kind = "/classification"
aliases = "/search/names"
```

For `{"equipment":[{"tag":"mix-station","label":"Mix station","classification":"fermenter","search":{"names":["brewdeck"]}}]}`, the target is `inventory/assets/mix-station.json`, with `id`, `name`, `kind`, `aliases` and the ownership metadata below. No filename-to-ID guessing or slugification.

- `input` is either one Git-tracked regular JSON file, or a directory of directly contained tracked `.json` files (one object each, no recursion). In single-file mode absence of `items` means one object; presence selects an array (empty pointer means the root array). `[]` in a valid aggregate explicitly declares zero assets. A directory with no eligible files is **invalid**, not permission to retire a fleet; use an explicit empty array to clear one. An absent declared/tracked input, malformed entry or partially read directory is not an empty inventory.
- Consult the tracked paths including staged additions and read worktree bytes, so edits can be reviewed before commit. An untracked `.json` in a declared directory is diagnosed, not silently omitted; ignore Git-ignored files. For a new declaration require staging the declaration and inputs before projecting. Reject absolute paths, `..`, Git internals, symlink components, submodules, output directories, declarations as inputs, and URLs; no shell, network, globs or plugins. Operate inside one KB Git worktree. Do not follow arbitrary references in data.
- Fields use [RFC 6901 JSON Pointer](https://www.rfc-editor.org/rfc/rfc6901#section-4), relative to each object. Required mapping/value: `id`, an existing-form slug. Optional mapped strings: `name`, `kind`, `owner`, `lifecycle`, `locator`, `verified_at`, `ttl`, `notes`; optional string arrays: `aliases`, `projects`. Omit unmapped fields; a configured pointer missing or of the wrong type is an error, not a fallback. Reject duplicate JSON keys and unknown declaration keys. No transformations or inferred dates. Adding a subtype is a data edit; adding a mapping primitive needs a demonstrated format, not a script evaluator.
- A team with YAML, CSV, a database or an API may export JSON itself and commit that snapshot, or continue generating native asset files directly. Document one working non-JSON export recipe. Writfold verifies the **committed input relationship**, not whether the producer fetched everything or its remote is current. Never derive a fresh check receipt or set `verified_at` from the time of import.

## Review, ownership and failure

1. `writfold kb catalog diff brewery > brewery.patch` emits only a standard Git patch on stdout, exit 0 for a valid plan (empty patch for no difference). Sort targets by path, use deterministic pretty JSON/key order and LF, no timestamps, absolute paths or local Git configuration in the patch. `--json` uses the normal envelope: one row per affected asset (`kind=asset`, `id`, `path`, `status=create|replace|remove`), not a patch embedded in JSON. Failure emits no patch bytes. Report declaration path/key or source path/pointer and reason on stderr. Bad usage exits 1; invalid/conflicting input exits 3.
2. The user inspects the patch, uses `git apply --check` then `git apply` on a nonempty patch, reviews the Git diff and commits inputs, declaration and outputs together. No `sync`, hidden auto-apply, automatic staging, or deletes on a read. The KB's working asset inventory is the last **applied** projection plus hand-authored assets. Editing an input alone does not change lookup results; `catalog check` exposes the difference.
3. The generated record carries an extra `writfold_catalog` object with `name` and KB-relative `source`; match the stable `id` inside that source using the declaration. No line number or array ordinal as identity. Git history pins which declaration and bytes produced the record; diagnostic pointers locate a bad entry at the inspected revision. Do not reformat or adopt hand-owned assets. Validate global ID/path ownership across all declarations, managed files and existing KB record claims (including losing duplicates, not just the winning record). Ambiguity, invalid target placement, malformed ownership or orphaned owner is a conflict; an invalid catalog cannot authorize a partial diff for another. Catalog removal requires explicit review: empty its input and apply its removal patch before removing its declaration, or explicitly remove ownership to keep hand-maintained assets. Hand edits to a managed record appear as drift; a replacement patch shows the exact lines it would undo. No automatic takeover, including on a catalog rename.
4. `kb catalog check [NAME]` reports the same rows without a patch: 0 clean, 2 drift, 3 invalid/conflict, 1 usage. `kb lint` diagnoses malformed declarations. `kb check` includes catalog drift/invalid inventory as non-clean when configured, using its existing aggregate exit convention; never run converters. No declarations and no managed assets: preserve existing `kb check` output and exit. Treat a directory that cannot be listed as invalid, **not** as the no-config case. Existing read-only verbs and the write planner do not run this check in the hot path.
5. Derive from a consistent inventory of sources, definitions, claims and targets. Recheck inputs/outputs before publishing a diff/check result; changes during the read invalidate it, never a partly valid patch. Stage potentially large output to a private temporary file until validated rather than buffering the KB or streaming an irrevocable prefix. Git applying a patch is outside Writfold's protocol; a later input edit is caught by `check` on the candidate tree (e.g. CI on a clean checkout).

## Alternatives and compatibility

[Backstage's software catalog](https://backstage.io/docs/features/software-catalog/) separates descriptor locations from entity metadata and allows external sources. Adopt that separation, **not** its runtime integrations. Direct registration of only native-shaped files under another directory would be simpler, but would not read a team's aggregate index or renamed fields. Projecting foreign rows directly into SQLite would require multi-record file provenance, collision and refresh changes and would hide records from older readers. A general-purpose importer or per-team plugin on the read path would make a cold clone depend on executable code. Doing nothing preserves a separate, unvalidated converter per team. `record asset` alone would just make hand entry easier.

This narrows the [mission's earlier bulk-import exclusion](../writfold/mission.md): no generic write/import verb, no changed lifecycles. Only a **read-only patch** for this owner-requested inventory mapping. Native records remain one per file; the SQLite cache remains disposable. Older binaries can use the projected assets and ignore the extra ownership key, but cannot attest `catalog check`; require a supporting binary in CI to check drift. Removing the declarations and ownership markers leaves ordinary records. No citation-token, proposal, or existing record schema name changes. [#46](https://github.com/writfold-dev/writfold/issues/46) concerns batch writes and observation provenance; it is not a prerequisite.

## Rollout and acceptance

- **P1, start here:** [#84](https://github.com/writfold-dev/writfold/issues/84), kernel, Lean properties and killing mutants. **P1, then** [#86](https://github.com/writfold-dev/writfold/issues/86), IO/CLI/lint/check and guides. Both target dev.15; parser fixtures can start before wiring. Do not change the existing index/recovery work to implement this.
- In two clean clones, exercise the directory and nested-array forms, escaped pointers, explicit empty vs incomplete inventory, collisions with hand-owned/other-kind records, symlink escapes, malformed keys, edited declarations, renamed/deleted/dirty source and target, and deterministic patch/recheck. Reorder an input array without changing records: no diff. Run the documented non-JSON export, not a mock adapter.
- Apply a reviewed patch; list with `find --kind asset`, resolve an alias with `context`, write a ticket, propose/accept an investigation by exact asset ID, inspect its impact link, and verify a rebuild with no producer available. Include an older reader and a no-config control. Record actual commands/receipts in the implementation issues; quality/unit on main before closure. No claims of implementation or source freshness from this planning exercise.

## Decision log

2026-11-21 — Accepted under the owner's standing default. Git-local input, explicit review, native assets on output; no general schema/plugin system. No unresolved acceptance questions.
