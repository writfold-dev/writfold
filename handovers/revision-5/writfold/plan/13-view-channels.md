# View channels: Obsidian first, other readers next

Decision, 2 November 2026: admit a human view channel and an optional Obsidian CLI companion. This adds presentation, not an alternative knowledge authority. The supported engine target remains Linux/WSL x86_64. An optional bridge to an installed Windows Obsidian does not introduce a Windows Rust build.

## One prepared document, several renderers

Use the playbook's I1 prepared presentation, I2 stamped lifetimes and I3 controlled publication. Extend the existing application view rather than inventing a general frontend framework. Keep acquisition and presentation separate: a Scribe obtains evidence; a view renderer presents admitted knowledge. Neither publishes claims.

`ViewRequest -> AuthorizedSnapshot -> PreparedDocument -> RenderedBundle -> PublishReceipt`

The immutable prepared value contains a release identity, selected context, policy and visibility identities, exact source closure, query identity, assessment clock, completeness/truncation information, and a typed document tree. Nodes are headings, paragraphs, links, evidence references, relationships, tables and explicit notices. A renderer cannot query storage, reassess claims, fetch citations or read the wall clock. An HTML renderer can later consume the same tree without parsing Obsidian Markdown. No HTML service is required by this revision.

Suggested ownership: `writfold-view-model` (pure prepared values), `writfold-view-markdown` (escaping and portable rendering), `writfold-view-obsidian` (properties, Bases, DQL templates), and `writfold-view-publish` (filesystem plan/recovery). The existing supervisor owns CLI child lifetimes. Split modules by these responsibilities; enforce the existing 400-line module and 80-line function budgets. Avoid a single `views.rs` with rendering, storage and process effects.

Use private-constructor nominal types for `ViewChannelId`, `ViewGenerationId`, `ViewSchemaVersion`, `VaultId`, `VaultRelativePath`, `ExportGrantId`, `RendererDigest` and `OutputDigest`. Reuse existing `ReleaseId`, `ContextId`, `ClaimId` and `WritId`; do not duplicate their encodings. Distinguish `PreparedView`, `ValidatedBundle`, `PublishedGeneration` and `ActivatedView` through explicit state transitions. JSON is only a versioned external encoding.

## Capability contract

The channel catalog declares protocol version, accepted document schema, renderer identity, limits, supported formats and required publication capability. Initial channels are compiled-in portable Markdown and Obsidian. A later stdio renderer must use a separate view protocol: a bounded prepared payload in, bounded relative output descriptors out, no direct vault access. Reuse supervisor mechanics, not the Scribe method names or evidence permissions. Reject unknown required capabilities; explicitly negotiate optional ones.

An export grant admits a specific audience, scope and destination. Filter before rendering, including summaries, filenames, aliases, backlinks, table counts, attachments and query metadata. Redaction must not leak a hidden object's existence through a link. A summary is either deterministic from admitted statements or explicitly labelled non-authoritative editorial text with provenance. Never infer an owner, certainty level or complete absence from missing data.

Generated content is data. Escape arbitrary source text against Markdown links, YAML, callout directives, HTML, transclusion, Dataview fences and formula injection. Only fixed renderer-owned templates emit active query blocks. Source snippets use literal fenced blocks with delimiter lengths exceeding contained fences. Copy only admitted attachments; do not silently embed remote resources.

## Publication and honest refresh semantics

Use a dedicated vault or an explicitly owned subtree in an existing vault. Stage outside the watched vault, validate all names/links/hashes, then install a new immutable generation under `Writfold/Generations/<generation>/`. Every link within that generation names its exact generation; every query filters its generation and context. Update `Writfold/Home.md` last, conditionally on the expected previous index digest. An open old page stays a valid old snapshot. Its visible timestamp prevents it being mistaken for a live assessment.

This is **not an atomic Obsidian refresh**. Obsidian and plugins index files asynchronously. The filesystem publisher can make closure-before-index visible and avoid cross-generation references, but it cannot prove the GUI has refreshed. Wait for declared CLI/index observations before reporting “visible”; otherwise report “published; reader refresh pending.” Never count plugin indexing as authority. On unsupported rename/durability semantics, retain the old index and report a failed activation rather than advertising atomicity.

The manifest records sorted relative paths and content hashes, schema/renderer/template versions, source release/context/grant, assessment time and generation identity. The generation identity is derived from prepared inputs and renderer configuration; it must not depend circularly on output files that embed it. Output hashes then form a separate manifest digest. Fsync files and destination directories where required by the admitted Linux storage profile. Persist a recovery journal and conditional activation intent. Do not reuse a partially built generation directory.

Check both lexical and resolved containment; reject symlink traversal, existing hard-linked destinations, `..`, reserved Windows basenames, trailing dots/spaces, separators in names, case-fold and Unicode normalization collisions. Route using stable safe IDs, with human names as H1/aliases. Detect user-modified managed files against the previous manifest and preserve them with an explicit conflict outcome. Never overwrite human notes, plugin settings or another channel's files.

Keep `Human/` user-owned. Drafts can cite managed notes, but Markdown edits and editable Base cells do not update Writfold's authority. A future import bridge must show a diff, issue a new authenticated proposal and run normal admission; this revision does not implement silent bidirectional sync. Cleanup requires manifest ownership, retention/reference checks and unchanged bytes. Retain old generations deliberately: they may have personal backlinks. Do not delete them merely because Home moved.

## CLI companion and plugin administration

File publication works without the app. Desktop commands use the official CLI, an explicitly admitted executable and an exact `VaultId`; never ambient active-vault/active-file defaults. Build argv directly, never a shell command. Read installed `help` and `version`, then admit required verbs. The official CLI controls the desktop app and may launch it; Headless Sync is a different facility.

Normal view capability permits exact-path open, bounded search, backlinks, unresolved-link checks and Base query. Plugin installation/enabling is a separate setup capability authorized by Tarek for this view profile. It is not accessible to a document renderer or source citation. Pin and hash selected plugin assets; inspect installed versions, preserve unrelated settings, and require a version match before enabling the profile. The documented `plugin:install` command has no version argument: install, inspect and verify, or provision exact release assets through the managed setup adapter. Never claim the command itself pins a release.

Do not expose arbitrary `eval`, generic command execution, shell/terminal plugins or unattended plugin updates through the channel. Installed community plugins execute with Obsidian's capabilities; a DQL-only profile reduces generated script surface but does not sandbox the plugin. Register timeouts, bounded output, cancellation and child reaping in the existing supervisor. Capacity remains owned until physical teardown. Stale completions cannot activate a different generation or vault.

WSL qualification must cover native Linux Obsidian where used, and separately the optional WSL-to-Windows launcher with explicit path conversion. Keep authoritative Omnigraph/Git/writ state on the qualified Linux filesystem. A Windows-visible vault is an export destination, with its own rename, case-folding, watcher and latency tests. No guessed drive letter or relocated BuildKit directory.

## Formal and execution gates

**V14, production Verus:** prove the pure projection preserves release/context identities, outputs only the admitted subset, keeps status/coverage labels faithful, and validates output-path identities and manifest membership. Callers establish parser/normalization assumptions; Unicode libraries and IO remain explicit trusted boundaries. Do not claim a proof of arbitrary natural-language summaries or third-party rendering.

**M10, ViewGeneration:** model Prepare, Validate, Stage, Install, Activate, Observe, Cancel, Crash, Recover and Retire. State includes grant/request stamps, source pin, output closure, previous Home digest, owned paths, live child handles and retained generations. Prove with TLAPS that activation names a validated closed generation, stale completions cannot activate, no unowned path is mutated, and cancellation never releases a live process slot. Specify cross-model bridges to publication/retention and Scribe lifetime ownership. TLC explores two overlapping refreshes, interruption at every write/rename, a revoked grant and cancellation/exit races; deliberately broken closure/stamp checks must yield counterexamples. Liveness requires stated fairness/IO assumptions and is separate from safety.

**WF22:** V14/M10 evidence plus deterministic output, hostile text/path fixtures, context isolation, edited-file preservation, recovery and generation-scoped query tests. **WF23:** actual Obsidian/CLI/plugin profile, light/dark and narrow-screen reading, plugin-off fallback, native file watcher and stale-index checks. None of these proofs or product gates has been executed in this planning refresh.

See [reading design](14-obsidian-reading-design.md), [backend decision](15-storage-decision.md), the sample at `../examples/obsidian-vault/`, and mandatory stage P09V.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
