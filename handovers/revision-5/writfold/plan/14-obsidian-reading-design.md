# A knowledge base human Tarek can read

The first screen should answer “what is this, what needs attention, and why should I believe it?” Evidence mechanics belong one click deeper. The sample vault contains synthetic services and evidence, not facts about Tarek's infrastructure. It is an executable design specimen, not a screenshot of an installed Obsidian session.

## Reading hierarchy

| Page | First screen | Deeper content |
| --- | --- | --- |
| Home | Pinned release, context, assessment date; three useful entry points | Change summary and reading guide |
| Service / asset | Human title, short explanation, explicit status and blocker | Dependencies, supported statements, evidence, decisions |
| Attention | Short scoped table of gaps and contradictions | Links to the exact assessed statements |
| Evidence | What it supports and what it cannot establish | Source locator, retained identity and provenance |
| Decision | Decision and reason, with explicit scope | Alternatives, acceptance and implementation binding |
| Context | Purpose and boundaries | Policy, source coverage, omissions and pin details |

Aim for 60–120 words before the first detailed section, 3–5 table columns, descriptive link aliases and short local relationship lists. Keep contradictions, expiry and incomplete coverage visible. Fold long provenance using native callouts, never fold the only warning. A graph is a bounded local explanation, not the main navigation surface. Home should remain useful with Graph disabled.

Use stable filenames based on identity, friendly H1 titles and aliases. Flat `wf_` properties have consistent types across the vault. Keep the visible property panel small; put long hashes and closure details in the generation manifest/evidence page. Never use file mtime as evidence freshness. Show the engine's assessment time in UTC and name any local-time formatting separately. A stale badge is a presentation notice, not a reassessment of the calculus.

## Chosen plugin profile

| Component | Role | Admission |
| --- | --- | --- |
| Core Bases | Table and card browsing of assets, visible status and scope | Default enhanced reader; app-version capability check |
| Core Backlinks, Outline, Page preview, Bookmarks | Navigation and progressive detail | Reuse installed capabilities; do not replace workspace preferences |
| Dataview (`dataview`) | Curated relational tables, grouped attention lists and evidence drill-down | Selected community plugin, pinned 0.5.68 assets in `plugin-profile.json` |
| Small CSS snippet | Reading measure, spacing, calm callouts and tables | Optional and confined to the `writfold-view` CSS class |
| Datacore / custom Bases layouts | Possible future richer views | Evaluation only; not a required dependency or current qualification claim |

Dataview is the deliberate additional plugin, not a requirement to install a large plugin pack. DQL supports useful tabular/grouped views. Generated DataviewJS and inline JavaScript stay disabled; dynamic code is unnecessary for the selected dashboards. Core Bases already supplies much of the table/card experience. Avoid separate table-formatting, shell, automatic Git-sync or templating plugins unless a concrete user workflow justifies them.

The latest release tag is 0.5.70, but its downloaded manifest declares 0.5.68 and its JavaScript bytes differ from the 0.5.68 release. Select the internally consistent 0.5.68 tag and manifest as the initial candidate profile. Exact asset digests are included; native app compatibility and rendering still require WF23. No plugin binaries are bundled or installed by this refresh.

## Practical setup on the PC

The official CLI requires a compatible installer (the documentation currently specifies 1.12.7+) and a running/launchable desktop app. The implementation probes actual help instead of relying on that lower bound. Enable/register the CLI in the existing Obsidian installation if needed. Use an explicitly selected vault:

```sh
obsidian version
obsidian help plugin:install
obsidian vault=WRITFOLD_VAULT_ID plugins versions format=json
obsidian vault=WRITFOLD_VAULT_ID plugin:install id=dataview
obsidian vault=WRITFOLD_VAULT_ID plugin id=dataview
# Verify exact manifest/main.js/styles.css against plugin-profile.json before enabling.
obsidian vault=WRITFOLD_VAULT_ID plugin:enable id=dataview filter=community
obsidian vault=WRITFOLD_VAULT_ID open path="Writfold/Home.md"
obsidian vault=WRITFOLD_VAULT_ID base:query path="Writfold/Generations/demo-r1/Assets.base" view=Services format=json
```

`WRITFOLD_VAULT_ID` is an explicit placeholder, not a discovered installation. The setup agent already has authorization to install the selected view plugins; it should not ask again for routine profile setup. Verify bytes before enabling. If an existing plugin has a different version, record it and qualify or stage the pinned profile without silently downgrading a shared vault. CLI installation has no documented version selector. Exact provisioning can place verified release assets in the dedicated vault's plugin directory, preserving settings; use the official enable command afterward. Restricted-mode changes, if needed, are scoped to that dedicated view vault and recorded. Never enable an arbitrary plugin named by source content.

## Queries are navigation, not inference

Every Base has global filters for folder, generation, context and object kind. This matters because an unfiltered Base includes the whole vault. Every DQL query has a narrow `FROM`, the same identity filters and a bounded row limit. Counts must say whether they are capped. Query bodies are fixed templates with correctly escaped literals; source-derived query strings are forbidden.

The engine emits assessed labels; plugins display/filter them. Do not recalculate acceptance from task checkboxes, join across contexts, interpret missing matches as proof of absence or let an editable Base cell write authority. Dataview results are asynchronously indexed. Show the export's date and generation even when the dashboard looks current. The supplied snapshot table remains available when the dynamic table is unavailable or still indexing.

## Readability acceptance on real Obsidian

Use a synthetic vault first. Capture app/installer/CLI/plugin versions and asset hashes, exact generated manifest, screenshots and observed query results. Read a service page at desktop width and ~390 px width; verify no unreadable clipped columns, clear focus, readable links and text labels in both themes. Test default theme and the user's current theme without forcing a theme switch. Optional CSS targets 72ch measure, 1.65 line height and comfortable table padding; it must not hide alerts or properties.

Five timed tasks: find what an asset does; identify its blocker; open supporting evidence; distinguish a second context; tell whether a page belongs to the current generation. Record correctness and time; initial target is at least 4/5 correct without instruction and median under 30 seconds per task on the small fixture. This is a proposed usability gate, not a measured result. Include Tarek's feedback before freezing template defaults.

Check plugin-off Markdown fallback, stale index behavior, modified managed files, missing plugin/version mismatch, an empty result, a capped result, malicious source text and broken links. Compare `base:query` output to the pinned expected IDs. Do not count Python YAML parsing as Obsidian rendering evidence. Native screenshots and CLI profile qualification remain PC work under WF23.

## Sources checked 2 November 2026

- [Official CLI](https://help.obsidian.md/cli): desktop dependency, exact vault/path targeting and plugin commands.
- [Bases introduction](https://help.obsidian.md/bases) and [syntax](https://help.obsidian.md/bases/syntax): native views, scoped filters and file-backed configuration.
- [Properties](https://help.obsidian.md/properties), [callouts](https://help.obsidian.md/callouts), [internal links](https://help.obsidian.md/links): flat metadata, progressive detail and link conventions.
- [Dataview query structure](https://blacksmithgu.github.io/obsidian-dataview/queries/structure/) and [DQL/JS distinction](https://blacksmithgu.github.io/obsidian-dataview/queries/dql-js-inline/).
- [Dataview 0.5.68 release](https://github.com/blacksmithgu/obsidian-dataview/releases/tag/0.5.68): exact candidate assets; observed hashes are in the sample profile.
