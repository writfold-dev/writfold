---
rfc: "2026-11-20-citations-as-links"
title: "Citations rendered as links in posted text"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-20
updated: 2026-11-20
issue: https://github.com/writfold-dev/writfold/issues/5
milestone: "v0.6.0"
supersedes: []
superseded_by: []
blocked_on: ["owner: what an unmapped citation does (proposed: refuse)"]
---

# RFC: Citations rendered as links in posted text

Item U2 of [Attached stores, next](2026-11-18-attached-stores-next.md#32-citations-rendered-as-links-u2-worth-doing-after-an-owner-answer). It is listed as planned in the README's "Not built" section.

## Summary

A delivery mode may declare `rendering = "links"`. Its posted text then shows each citation token as a link to the cited lines, built from a `web` template, and the deny lists judge the rendered bytes. A mode may also declare `unmapped = "refuse"` (the default) or `"strip"`, which decides what happens to a citation whose repository has no template.

## Motivation

Text an agent posts to a tracker or a review currently carries raw `⟦cite …⟧` tokens, which a human reader cannot follow. Two of the three earlier blockers have moved:
- Stores carry a tracked `url`, and every `code` token carries `url=`.
- The delivery checks already judge the payload that will be posted.

The remaining question is what to do with a citation that has no template.

## Behaviour

- **Without `rendering`:** the bytes are exactly today's.
- **With `rendering = "links"`:** each citation whose kind has a template and whose repository has an address becomes a link. A citation without a template is handled by `unmapped`:
  - `refuse`: the candidate is refused, and the message names the repository to map.
  - `strip`: the token is removed and its text is kept.
- **Where templates live:** in the store declaration (`web = "https://…/blob/{rev}/{path}#L{start}-L{end}"`), and for ordinary repositories in a tracked file of the same shape.

## Proof obligations

- **Kernel:** `writfold_core::render::link(kind_has_template, repo_has_address, unmapped) -> Rendered`.
- **Theorems:**
  - `render_off_unchanged`: a mode without `links` produces today's bytes.
  - `render_link_iff`: a citation becomes a link exactly when the kind has a template and the repository has an address.
  - `render_unmapped`: an unmapped citation is refused under `refuse`, stripped under `strip`, and never a link.
  - `render_checked_bytes`: the bytes the deny lists judge are the bytes bound to the candidate.
- **Mutants:** one per theorem.
- **Tested and unproved:** template expansion, URL escaping and the template file's codec.

## Design

Rendering happens before the candidate is bound, so binding, the checks and `deliver posted` all see the rendered bytes. The unrendered text stays in the draft.

## Compatibility and reversibility

The feature is opt-in per mode, and an older binary refuses the unknown key, as it does every unknown mode key. Reverting means removing the key.

## Alternatives

- **Render at post time, outside Writfold:** the checks would then judge bytes that are never posted.
- **A local URL map:** it is not reviewed, and it differs per machine.

## Evidence and tests

Golden outputs for each combination; a deny-list check over a rendered link; a refusal that names the repository.

## Rollout

1. Kernel and proofs.
2. Templates on stores.
3. A tracked template file for ordinary repositories.
4. Mode keys, guide and `changes.md`.

## Unresolved questions

- **Owner:** should an unmapped citation be refused (proposed) or stripped by default?

## Decision log

- 2026-11-20: written from attached-stores-next §3.2.
- 2026-11-21: owner answer ([#40](https://github.com/writfold-dev/writfold/issues/40), question 5): an unmapped citation is refused by default; `strip` is opt-in. The RFC stays a draft until it is accepted as a whole.
