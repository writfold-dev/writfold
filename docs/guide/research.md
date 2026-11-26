# Research contracts: scopes, review units and dispositions

> **Status:** built in v0.6.0-dev.23 ([#153](https://github.com/writfold-dev/writfold/issues/153), [RFC](../rfcs/2026-11-25-kb-research-contracts.md)): `kb impact --manifest`, a research scope as a mechanical gate, checklists generated from its manifest (`items_from`), `delivery:` cover references, `agent prepare --for` and `needs`. Planned: unit rules other than all pairs, keeping earlier reviews of unchanged pairs when a member is added, and `gate cover --from` for many rows at once ([#156](https://github.com/writfold-dev/writfold/issues/156)).

"Inspect all related records for inconsistencies" is a prose instruction an agent can skip. A research contract turns it into three things Writfold can hold, each with its own standing:

1. **Scope:** which records are related. A mechanical gate keeps the list.
2. **Coverage:** every pair of them reviewed. A checklist with one item per pair, each covered by a worker's report.
3. **Disposition:** what will be done about the findings. A person's statement, an attested gate.

None of the three is derived from another. Covering every unit says nothing about whether the reviews were right, and the disposition is still owed after coverage.

The two rehearsals in `docs/guide/recipes/` run every case on this page against the real binary: `research-scope-rehearsal.sh` and `kb-consistency-rehearsal.sh`, checked by the tests `kb_research_scope.rs` and `kb_consistency.rs`. The definitions are in `fixtures/kb/kb-consistency/`.

## The scope: a kept walk

`writfold kb impact <id> --manifest FILE` writes what the walk reached as `writfold-manifest/1`:
- the seed, the direction and the depth;
- `coverage`, `complete` or `partial`, the same word and exit status as the answer (exit 2 when partial);
- the members, sorted by id, each with its path and the digest of its bytes.

Only KB records are members. A code locator is counted in `others`. With `--manifest` the walk is over records: an edge declared in a run's own files (a report, a receipt, an invocation record) is neither walked, listed nor counted. So the reviews a scope leads to never make the scope itself partial.

"Complete" means complete for this seed, direction and depth over typed links. It is not a claim about records related in meaning and not linked.

A scope gate keeps that file with its receipt:

```toml
# .writfold/gates/related-scope.toml
schema = "writfold-gate/1"
id = "related-scope"
version = 1
class = "mechanical"
argv = ["writfold", "kb", "impact", "SEED-1", "--direction", "in", "--depth", "1", "--include", "inferred", "--manifest", "{artifacts}/manifest.json"]
timeout_seconds = 120
scope = ["proposals/**", "decisions/**", "ktlo/**", "investigations/**"]
retain = ["manifest.json"]
accept = ["satisfied"]
```

What the rehearsal shows:

| Case | The gate reads |
|---|---|
| the walk read in full | `satisfied`, and the manifest lists the members |
| the walk stopped at its work limit (`WRITFOLD_KB_WORK`) | `failed`: the manifest says `partial` and the command exits 2 |
| the command timed out | `unknown` |
| a record added that references the seed | `stale`, naming the new files; running it again lists the new member |
| a clone without the kept manifest | every receipt `[invalid]`: never met |
| a seed nothing references | a manifest with no members and `coverage = "complete"` |

`--include inferred` follows mentions as well as declared links. Without it a mention is skipped, and a skipped edge makes the walk partial.

## Review units: a generated checklist

```toml
# .writfold/gates/pair-review.toml
schema = "writfold-gate/1"
id = "pair-review"
version = 1
class = "checklist"
scope = ["proposals/**", "decisions/**", "ktlo/**", "investigations/**"]
items_from = "related-scope/manifest.json"
units = "pairs"
```

The items are not written in the file. They are made, on every read, from the manifest that the scope gate's current passing receipt kept: one per unordered pair of members, named `<id>~<id>` with the ids in byte order. Five members make ten units; six make fifteen.

- **No universe:** without a current pass of the scope gate, or when its manifest does not read, there are no units and the checklist is not covered. A `why:` line names the scope gate.
- **Too many members:** a manifest of more than 64 members is refused with the count.
- **Fewer than two members:** this leaves no unit, which is not covered unless the checklist says `empty = "allowed"`.
- **New members:** when the scope runs again over other members, every earlier row is stale ("the scope's members or the definition changed since this row"). Reviews of unchanged pairs are not kept yet.

Each unit takes evidence of its own: a worker's report prepared for that unit. Prepare the worker with `--for`, deliver its report bound to the invocation, then cover the unit with the report:

```text
$ writfold kb agent prepare consistency-reviewer --input pair.txt --out pkg --for pair-review.<a>~<b>
$ writfold kb deliver prepare --mode pair-report --from report.md --invocation <invocation>
$ writfold kb deliver check <candidate>
$ writfold kb gate cover pair-review <a>~<b> supported delivery:<candidate> --by agent
```

`delivery:<candidate>` grounds a row only while the report is ready and passes `invocation.bound`; it is judged again on every read. `gate cover` refuses:
- a report that names no invocation, as `unbound`;
- a report prepared at another step, which fails `invocation.bound`, so it is not ready;
- a report prepared `--for` another unit (`takes its own evidence`).

`gate status` shows `8/10 units covered`, with each unit and the next one owed. A plan step that owes the checklist is not advanced until every unit is covered.

## Needs and the disposition

The skill that does the reviews says `needs = ["related-scope"]`, so `agent prepare` refuses while the scope is open, failed, stale or cannot be judged ([agents](agents.md#needs-gates-that-must-be-met-when-a-worker-is-set-up)).

The findings are disposed of by a person, as an attested gate with `accept = ["attested"]`:

```toml
# .writfold/gates/findings-disposition.toml
schema = "writfold-gate/1"
id = "findings-disposition"
version = 1
class = "attested"
accept = ["attested"]
scope = ["proposals/**", "decisions/**", "ktlo/**", "investigations/**"]
```

## What this does not do

- **Correctness:** a report that is wrong and well formed passes `invocation.bound` and covers its unit. Coverage is not correctness.
- **A statement a harness cannot make:** one such as `context.submitted` stays open, and the report is not ready. Nothing turns a statement into a check.
- **Harness shape:** no judgment depends on which harness ran the worker. A harness that reads only `instructions.md` and `input` gives the same standings as one that reads `invocation.json`.
- **Forged records:** these, and a bypassed hook, are not resisted inside a run. The boundary that resists a worker that does not cooperate is a receiver ([`handoff check`](policy.md), [#64](https://github.com/writfold-dev/writfold/issues/64)). Whether an effect outside Writfold happened once is not something Writfold knows.
