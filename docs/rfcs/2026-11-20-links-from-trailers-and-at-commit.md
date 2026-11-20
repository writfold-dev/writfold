---
rfc: "2026-11-20-links-from-trailers-and-at-commit"
title: "Links from commit trailers and tracker references, and answers as of a commit (M4b)"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-20
updated: 2026-11-20
issue: https://github.com/writfold-dev/writfold/issues/31
milestone: "v0.6.0"
supersedes: []
superseded_by: []
blocked_on: ["owner: tracker vocabulary (ticket patterns, trailer keys, URL forms) per design §10 question 7"]
---

# RFC: Links from commit trailers and tracker references, and answers as of a commit (M4b)

M4b in the [mission](../writfold/mission.md) roadmap (item 11), and the parts of [design §9 M4](../writfold/kb-v1-design.md#9-milestones) that are not built. It is planned in the README's "Not built" section.

## Summary

Two read-side additions:
1. **Links from history.** The index derives `link` edges from commit trailers (for example `Refs: PROJ-123`) and from tracker references in commit messages and record bodies.
2. **Answers as of a commit.** `--at COMMIT` makes `brief`, `context`, `why`, `uses` and `impact` answer as of a KB commit.

Today the binary refuses the flag with "`--at COMMIT is not built in v1 (planned for M4)`".

## Motivation

The mission's "why" and "impact" questions often lead through a ticket that a commit names but no record cites. "What did we know when we decided this?" needs the KB as it was. Today an agent has to `git checkout` an old tree and rebuild the index by hand.

## Behaviour

**Links from history:**
- A tracked file `.writfold/links.toml` declares:
  - ticket patterns (regexes with one capture group);
  - trailer keys;
  - tracker URL forms.
- The index adds edges that come from history. They are marked as derived (a `via: trailer <commit>` line), never stored as records.
- `uses PROJ-123` lists the commits and records that name the ticket.

**`--at COMMIT`:**
- Reads every file from that commit's tree into a separate, cached index keyed by the commit. The live index is untouched.
- Citations are still judged against the present repositories, and each answer says so.
- Write verbs refuse `--at`.

## Proof obligations

- **Kernel:** none for the edges, which are derived and do not change any standing. If a standing ever reads a derived edge (for example "ticket linked, therefore covered"), that decision needs a kernel and theorems first, and a new RFC section.
- **Tested and unproved:** trailer and pattern parsing, the history walk, the index cache keyed by commit, and the refusals.

## Compatibility and reversibility

Without `links.toml`, there are no new edges. `--at` is new and read-only. The index gains a derived-edge table, and an older binary ignores it.

## Alternatives

- **Recording links by hand only** (today). Many links exist only in commit messages.
- **An MCP server** (dropped on 15 November 2026).

## Rollout

1. Parse `links.toml` and add the trailer edges.
2. Add the tracker references in bodies.
3. Add `--at`.

Each step ships on its own.

## Unresolved questions

- **Owner:** the tracker vocabulary (design §10 question 7).
- **Owner:** does this wait for the "two-week pause" of real use (mission item 10)?

## Decision log

- 2026-11-20: written from the mission roadmap and design §9 when the RFC process was adopted.
