# Name, repositories and website

## Selected name: Writfold

Tarek confirmed that writfold.dev is registered at Porkbun. The following shortlist/checks are retained as decision history; they are not a pending name choice. The spelling is **Writfold**, without an extra e.

**Writfold** (pronounced “rit-fold”) combines a durable written record with gathering and preserving context. It sits comfortably next to Gripsack, Strop and Rootle without promising that software can prove arbitrary prose true.

Proposed tagline: **Knowledge with a record.**

One-line product description: “A local knowledge engine for agents, with exact evidence, explicit approval and reproducible context.”

| Choice | Domain / org | Assessment |
| --- | --- | --- |
| **Writfold (selected)** | `writfold.dev` / `writfold-dev` | Selected; compact, easy CLI spelling, records/provenance association |
| Tracewold | `tracewold.dev` / `tracewold-dev` | More distinctive and evocative; meaning and spelling need more explanation |

Proposed CLI is the full `writfold`, avoiding a common ambiguous abbreviation. New Rust packages use `writfold-*`. Keep protocol/storage namespaces versioned independently of branding.

## What was checked

On the review date, the .dev registry RDAP endpoint returned HTTP 404/no object for writfold.dev and tracewold.dev. GitHub's public user/org lookup returned 404 for writfold-dev and tracewold-dev. crates.io returned 404 for writfold. These are preliminary namespace observations, **not** a guarantee the registrar/host will permit registration, not a reservation and not a trademark clearance. Porkbun price, premium/reserved status and final availability were not checked at checkout. No purchase or account mutation was attempted. Raw checks and URLs are retained in evidence.

Exact web searches found no obvious Writfold developer-product collision; one unrelated legal use of the word appeared. Rejected candidates included Writloom (existing Android app), Tallyroot (existing finance product), Siftmark (existing developer tool/app) and Loreweft (existing software project). Avoid them even where a `.dev` registration was absent. Check final names again immediately before registration.

References: [Writloom publisher listing](https://play.google.com/store/apps/details?id=com.develoepralamin.writloom), [Tallyroot](https://tallyroot.com/), [Siftmark repository](https://github.com/xiaokillua/siftmark), [Loreweft repository](https://github.com/EXwhitewood/wanxiangpu-loreweft). This is brand-conflict screening, not legal advice or a legal search.

## Organization layout

| Repository | Contents |
| --- | --- |
| `<name>-dev/<name>` | Rust workspace, source contracts, Lean reference, TLA+/TLAPS, tests, catalog defaults, docs, release tooling |
| `<name>-dev/<name>-dev.github.io` | Static site sources, local renderer, prebuilt publication branch, publish-only Pages workflow |
| Optional later `<name>-dev/catalogs` | Only when independently versioned reviewed catalogs need a separate release cadence |

Do not create five repositories before one vertical slice works. Keep the calculus alongside the implementation initially so a semantic change must update consumers/fixtures/proofs together. Preserve Git history when moving the project; record the old kctx source link. A transfer or new repo choice depends on what Tarek creates; do not force-push over it.

## Match the actual Gripsack publishing pattern

The inspected Gripsack website lives in its own organization Pages repository. `website/build.py` reads an explicitly selected local core commit; `website/publish.py` pushes a prebuilt static commit and nominates it to a Pages deployment job. Reuse that separation. Build/test docs and the website locally against exact release artifacts. The Pages job only uploads/deploys the nominated static files. It must not rebuild the Rust product or fetch mutable documentation at deployment time.

Improve the new implementation with immutable action pins, exact commit validation, branch/ancestry/manifest checks, least-privilege deployment permissions and a content manifest. Reject a nominated commit not generated/approved for the intended site and product release. Do not invent a signing requirement for the site; integrity/qualification and publication authority are distinct.

## Visual and information design brief

A restrained technical tool site: charcoal/navy background, warm off-white text, teal evidence accents and amber uncertainty accents; a high-quality light theme too. Use readable sans-serif body text and monospaced command snippets. The supplied SVG icon is a folded W, with light, dark, monochrome and favicon variants in `brand/`. Do not present a “fully verified AI knowledge” badge.

Landing page: product name, concise value statement, honest maturity/platform badge, one runnable workflow, install command only when the release actually exists, and three demonstrated benefits: inspect why a claim is usable; retain exact evidence/history; recover a failed publication. Provide keyboard navigation, visible focus, responsive layout, reduced-motion support, accessible contrast and meaningful alt text. No account/signup flow or hosted-backend promise.

Docs: install/offline install; quickstart with a small synthetic KB; citations and captures; proposal/approval; current versus historical context; corrections; source/catalog integration; migration; backup/restore; troubleshooting/doctor; agent protocol; architecture; formal assurance and trusted assumptions; release notes. Generate CLI/schema examples from actual artifacts and test them. Keep product flows free of SQL ports, build cache paths and proof-tool setup unless the reader opens maintainer docs.

Assurance page: show proved model obligations, verified executable functions, tested adapters and explicit assumptions as separate categories. Derive status from source-bound receipts. A pending proof is visibly pending. A release must not inherit a badge from another commit.

## Porkbun and GitHub Pages runbook

The name is selected and domain registration is complete per Tarek. The agent prepares the exact records and site next. Before changing DNS, inspect existing records and preserve email/DKIM/SPF/verification records. Domain registration/paid purchase and account creation are owner actions unless separately authorized.

1. Use the registered writfold.dev; verify its account/domain state only when authorized to configure it. Check or use the GitHub organization Tarek creates. Do not re-purchase or reopen the naming decision.
2. Create/use `<name>-dev` and its organization Pages repository. Build the reviewed static artifact locally; test it at root and nested documentation URLs.
3. Follow GitHub's domain verification flow for the organization and add the custom domain in Pages settings **before** changing routing DNS. Record the exact verification TXT value from GitHub; never invent it.
4. For apex `<name>.dev`, use supported ALIAS/ANAME or the current GitHub A/AAAA set obtained from official docs at execution. For `www`, use a CNAME to `<name>-dev.github.io`, not to a repository path. Choose one canonical host and redirect the other.
5. Configure only the required records at Porkbun. Its GitHub quick configuration can change records; preview its impact rather than blindly replacing the zone.
6. Verify authoritative DNS, Pages ownership, certificate issuance and HTTPS enforcement; `.dev` requires HTTPS in normal browsers. Test apex/www, redirects, deep links, 404, mobile width, cached assets and download links.
7. Publish the exact reviewed site commit through the upload-only workflow; record deployment URL, commit and artifact hash. Retain the previous static commit for rollback.

GitHub's custom-workflow deployment uses the Pages domain setting; a CNAME file alone does not configure that setting. A CNAME artifact may be retained for consistency, but do not claim it performs the account operation. See [GitHub custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site) and [Porkbun DNS guide](https://kb.porkbun.com/article/231-how-to-add-dns-records-on-porkbun). Verify exact addresses and provider screens at execution rather than freezing them here.

## Brand handoff

Keep one `branding.toml` containing display name, CLI, crate prefix, org, repository and domain. Use it for new docs/site/installer URLs. Historical schema tags and signed bytes are excluded from automatic replacement. Branding is settled. Use the supplied SVG assets and `10-minimal-website-outline.md`; deployment still requires the appropriate account authorization.
