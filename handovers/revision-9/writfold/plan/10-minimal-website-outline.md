# Minimal website: section outline and design brief

**Selected brand:** Writfold · **registered domain:** writfold.dev. Use the exact spelling everywhere. The supplied folded-W icon is the brand mark. This is a site implementation brief and visual outline; the product is not implemented or deployed by this handover.

## Direction

Use the current Gripsack home page's restraint: compact top navigation, one central hero, useful capability descriptions, a practical example and a small documentation/footer area. Keep Writfold even shorter. One page, maximum six content sections, narrow readable measure, generous space and little decoration. No dashboard preview, carousel, fake customer logos, scrolling badges, giant architecture graph or marketing statistics on the landing page.

Default dark palette: background `#101922`, primary text `#EEF3F4`, muted text `#A8B6BF`, border `#2B3B46`, accent `#6DD7C4`. Light palette: background `#F7F8F4`, text `#172D38`, muted `#536772`, border `#D9E2DF`, accent `#087D72`. Choose accessible text contrast in final CSS; the accent is mainly for controls/marks, not tiny body text. One system sans family and one monospace family. Avoid a theme picker with many palettes; a compact light/dark toggle is enough.

Desktop content width ~960px; prose ~620px; 24px mobile side padding. At 360px width everything is single-column and code scrolls horizontally inside its own region. Use real headings, visible keyboard focus and a skip link. No animation is necessary.

## Header

Left: icon + lowercase wordmark `writfold`. Right: **start**, **scribes**, **docs**, **GitHub**. The first three link to useful local sections/pages; GitHub points to the real organization/repository once created. Do not show an invented live repository link while it is uncreated. Keep the bar compact and non-sticky unless the final page needs it.

## 1. Hero

Kicker: `a local knowledge engine for agents`

Headline:

> Keep the context.  
> Know what supports it.

Supporting copy:

> Bring evidence from your tools into one durable knowledge base. Keep its source, history and approval attached, so the next session can pick up with context it can inspect.

Primary action: **get started** -> quickstart. Secondary: **read the design** until a real release exists, then **view source**. Platform line: `Linux x86_64 · WSL2 · local-first`. Display maturity from release metadata. Before release, show “in development” and no fake installation command.

Use the small folded-W mark above or beside the wordmark, not a giant logo illustration. No claims that the engine makes all knowledge true or that an LLM cannot hallucinate.

## 2. Three useful things

Exactly three short columns on desktop, stacked on mobile:

1. **Keep the evidence.** Retain the source, exact revision and bytes behind a claim.
2. **Review what changes.** Propose, approve and publish knowledge with an inspectable history.
3. **Carry context forward.** Retrieve a bounded view with supporting evidence, contradictions and gaps.

Each card is mostly text with a small link to the relevant guide. Avoid stock icons if the text already communicates the meaning.

## 3. One small workflow

Heading: **start with one source**.

One tab-free terminal panel, not a wall of code. Use actual tested commands once P09/P12 land. Proposed command shape for implementation review:

```text
writfold init
writfold cite make ...
writfold knowledge propose ...
writfold knowledge context ...
```

The published version must show a complete runnable quickstart against a checked-in synthetic demo, with valid flags/fixtures and a brief approval step where needed. These ellipses are **not publication-ready copy**. The quickstart must not imply `cite make` automatically admits claims. Link to the full capture -> propose -> approve -> publish example for the real workflow.

Show one compact result excerpt: a claim, source identity, assessment and explicit gap. Use synthetic data and label it. The code and result come from the released binary's test fixture. No hand-maintained fake output.

## 4. Scribes

Heading: **connect a tool with a small script**.

Copy:

> A scribe is a small program that helps Writfold find and read a source. Write one for a repository, a document service or your team's tools. Writfold manages the process and keeps the evidence it returns as a writ.

One compact diagram in docs (not necessarily on the homepage): agent -> Writfold -> scribe; scribe -> validated writ -> proposed claim. Below the copy: **write a scribe** and **browse the catalog**. A small Python handler excerpt can show the author experience once the SDK exists. Keep protocol version numbers, ports, sandbox internals and proof tooling on the author/architecture pages.

Clarify in one quiet sentence: “Scribes acquire evidence; approval stays with your knowledge policy.” Do not present generated scripts as automatically trusted or execute them from a download button.

## 5. Install and documentation

Small install section from exact release metadata: supported Linux/WSL artifact, version, checksums, prerequisites/offline bundle link. State that Writfold manages its pinned Omnigraph companion; list Git and any selected scribe runtime prerequisites accurately in the install guide. No claim that the whole system is one dependency-free binary.

Then five documentation links: **quickstart**, **evidence & history**, **scribes & catalog**, **backup & recovery**, **assurance & architecture**. Maintainer proof/build details live behind the last link. Keep the homepage assurance statement to one modest line: “A small verified core, with explicit assumptions and tested storage protocols.” Only show it after the corresponding release gates pass; otherwise say “Formal verification is part of the implementation plan.”

## 6. Footer

Name/icon, source, changelog and license **after license selection is confirmed**. Do not infer Writfold's license from Gripsack's MIT badge. Add no social feed, newsletter, signup or analytics by default. The site can be completely static.

## Pages and source-of-truth map

| URL | Page | Source |
| --- | --- | --- |
| `/` | Minimal landing page | site source + exact release manifest |
| `/docs/start/` | Runnable synthetic quickstart | tested demo and CLI schema |
| `/docs/evidence/` | Writs, citations, approval, corrections and history | product contracts + fixtures |
| `/docs/scribes/` | Single-script guide and capability admission | SDK examples + protocol schemas |
| `/docs/catalog/` | Kinds, providers and locators | versioned catalog descriptors |
| `/docs/recovery/` | Backup, migration, recovery and retention | tested operation recipes |
| `/docs/assurance/` | Proof scope, trusted base, exact evidence | generated proof/gate registry |
| `/docs/architecture/` | Product and protocol diagrams | source-owned SVG/Mermaid |
| `/docs/install/` | Online/offline install and prerequisites | release artifact inventory |
| `/changelog/` | Actual delivered changes | release notes |

## Icon handoff

`brand/writfold-icon.svg` is transparent and optimized for light surfaces; `writfold-icon-dark.svg` uses a pale main mark for dark surfaces; `writfold-icon-mono.svg` uses currentColor; `favicon.svg` has a dark tile. The faceted W folds at its upper-right edge, tying the name to a compact retained record without using a shield/checkmark that implies universal correctness.

All are editable SVG vectors with a 96x96 viewBox, no embedded raster, no scripts, no network references and no font dependency. Keep their aspect ratio, use at least 16px display size and leave clear space of roughly 12/96 of the mark width. PNG previews are supplied. The contact sheet/atlas shows dark/light/favicon uses. Do not change the mark shape across themes; only swap defined colors.

## Acceptance

Test at 360/768/1440px; keyboard navigation/focus; light/dark contrast; reduced-motion behavior; local and deployed root/deep links; consistent version/install/checksum/source links; HTML escaping of code/source excerpts; no secret/internal corpus data; no failed or unrun proof represented as passed. The exact rendered site commit is the deployment input. Use the current Gripsack publish-only pattern, with the improvements in `07-brand-and-website.md`.

## Workflow-3 copy and examples

Keep the same minimal six sections. The quickstart now discovers/selects an explicit named context and shows approval before publication. The evidence/history guide explains separate context-claim results; the workflow guide distinguishes accepted design, implementation binding and execution gates. Do not imply one context's support authorizes another. Keep these details in the worked example/docs rather than adding homepage sections. Public assurance comes from Writfold's own release receipts; private playbook material is not published.
