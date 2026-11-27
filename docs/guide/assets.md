# Assets: what a citation points at, and who judges it

> **Status:** built in v0.6.0-dev.28 for Git ([RFC](../rfcs/2026-11-27-asset-boundary.md), [#199](https://github.com/writfold-dev/writfold/issues/199)); providers other than Git are planned.

An **asset** is anything a citation points at as evidence: a span of a source file, a gate receipt, a file in an attached store, a ticket, a page. Citing one involves five things:

| | Question | In a code token | Where it lives |
|---|---|---|---|
| **Identity** | which bytes | `blob=`, `span=` | the citation |
| **Name** | where in what | `repo=`, `path=`, `L=` | the citation |
| **Snapshot** | as of which state | `rev=` | the citation |
| **Replica** | where a copy is on this machine | the local map, `$WRITFOLD_KB_REPOS`, a path | this machine, untracked |
| **Line** | which moving reference counts as now | a declaration's `ref`, the map's `ref=`, else the remote that names the repository | `.writfold/repos/` (in Git) or the local map |

Only the first three are in the citation. The **line** is a fact of the team, so it belongs in Git text: declare the repository in `.writfold/repos/<slug>.toml` ([validation](validation.md#which-commit-is-judged)). The **replica** is a fact of one machine.

## Who judges what

A provider (Git today) finds the replica, resolves the line (`writfold_core::target::choose_in`, proved), says how the cited snapshot relates to it, and observes the bytes there. It never supplies a verdict: the verdict is `writfold_core::referent::judge` of those facts, which only withholds (an unresolved line, or one behind the cited snapshot, supports no change). Both are laws of the calculus ([laws](../writfold/laws.md)).

| Kind | Identity | Line | Judged by |
|---|---|---|---|
| `code` | `blob=`, `span=` | the repository's line | the verdict kernel, through `referent` |
| `gate` | the receipt's commit | the repository's line | the verdict kernel |
| a store's `code` | the pinned commit | the store's pin | `store::pinned` (proved) |
| `record`, `state` | the record's state | the KB | the verdict kernel |
| a checker kind (`web`, `ticket`, …) | the checker's | the checker's | the checker's word, mapped by the verdict kernel |
| `ttl=`, `seen=` | none | time | the lifetime rules |

## Where Git is required

- **The KB:** records, runs and receipts are Git text, and the index is rebuilt from a commit.
- **A run's subject:** gates bind to a commit of the repository they check.
- **Cited code:** through the Git provider, the first. A cited asset outside Git is judged by a checker today; built-in providers for other kinds are planned ([#200](https://github.com/writfold-dev/writfold/issues/200)–[#202](https://github.com/writfold-dev/writfold/issues/202)).
