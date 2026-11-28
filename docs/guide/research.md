# Research contracts: scopes, review units and dispositions

> **Status:** built in v0.6.0-dev.23 ([#153](https://github.com/writfold-dev/writfold/issues/153), [RFC](../rfcs/2026-11-25-kb-research-contracts.md)): `kb impact --manifest`, a research scope as a mechanical gate, checklists generated from its manifest (`items_from`), `delivery:` cover references, `agent prepare --for` and `needs`. Built in v0.6.0-dev.24 ([#163](https://github.com/writfold-dev/writfold/issues/163), [RFC](../rfcs/2026-11-26-research-contracts-second-slice.md)): a judged universe, unit-local staleness, withdrawn reports grounding nothing, and `--for` checked against the items a checklist has now. Planned: unit rules other than all pairs, and `gate cover --from` for generated units ([#162](https://github.com/writfold-dev/writfold/issues/162)). See [Known limits](#known-limits).

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
- the members, sorted by id, each with its path and the digest of its bytes;
- when the seed is a record, its own path and digest (`seed_path`, `seed_digest`, from v0.6.0-dev.24).

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

**Whether there are units at all** is a proved decision (`units::universe`, [#165](https://github.com/writfold-dev/writfold/issues/165)). There is a universe only when every one of these holds, and the `why:` line names the first that does not:

1. the scope gate is defined;
2. it stands satisfied by a check: attested, waived, open, stale, failed or unknown gives no units;
3. its kept file reads;
4. the file is a `writfold-manifest/1` whose members each have an id, a path and a digest;
5. its `coverage` is `complete`, so a manifest that says `partial` gives no units even when its command exited 0;
6. every member's path, and the seed's, is inside both the scope gate's `scope` and the checklist's, so a change to any of them makes the scope stale;
7. no two members make one unit name: no id holds `~`, and none is listed twice;
8. there are at most 64 members; with more, the count is named.

`gate cover` refuses with the same reason (`has no units: …`). With fewer than two members there is no unit, which is not covered unless the checklist says `empty = "allowed"`.

**Each unit's row is about four things** ([#166](https://github.com/writfold-dev/writfold/issues/166)):
- the checklist's definition;
- the seed;
- the unit's two members.

A row records their digests. When the scope runs again, a row stays current while those four are unchanged (`units::unit_moved`):
- editing one member stales only the units that name it (`decision-a changed since this row`);
- editing the seed stales every unit (`the seed changed since this row`);
- adding a member leaves the earlier units covered and owes the new ones.

The reviewer was handed only the pair, so a third member cannot change what the pair review claims. A profile whose instructions read the whole scope makes a different claim; that would be another unit rule ([#162](https://github.com/writfold-dev/writfold/issues/162)). A row written before v0.6.0-dev.24 has no unit digests and is stale whenever the manifest's members move.

Each unit takes evidence of its own: a worker's report prepared for that unit. Prepare the worker with `--for`, deliver its report bound to the invocation, then cover the unit with the report:

```text
$ writfold kb agent prepare consistency-reviewer --input pair.txt --out pkg --for pair-review.<a>~<b>
$ writfold kb deliver prepare --mode pair-report --from report.md --invocation <invocation>
$ writfold kb deliver check <candidate>
$ writfold kb gate cover pair-review <a>~<b> supported delivery:<candidate> --by agent
```

`delivery:<candidate>` grounds a row only while the report is not withdrawn, is ready and passes `invocation.bound`; it is judged again on every read. A withdrawn report makes its row stale (`cited evidence moved: delivery:… was ready, now withdrawn`), and checking a report again after its step is done gives the same answer ([#164](https://github.com/writfold-dev/writfold/issues/164)). `gate cover` refuses:
- a report that names no invocation, as `unbound`;
- a report prepared at another step, which fails `invocation.bound` (also `unbound`);
- a bound report that is not ready, for instance one whose mode still owes a statement (`not ready`);
- a withdrawn report (`withdrawn`);
- a report prepared `--for` another unit (`takes its own evidence (a report prepared with agent prepare --for …)`). A unit takes only such a report: no look can be recorded `--for` a unit, since `observe --for` takes a slug and a unit's name holds `~`.

`agent prepare --for` itself refuses a gate that is not defined or not a checklist, and an item the checklist does not have now, with the scope gate's reason when there are no units ([#167](https://github.com/writfold-dev/writfold/issues/167)).

`gate status` shows `8/10 units covered`, with every unit, each owed one marked `[open]`, and a `next:` line for the first. A plan step that owes the checklist is not advanced until every unit is covered.

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

## Notes whose every paragraph says how it stands

A research note delivered under a mode with `claims.accounted` (since v0.6.0-dev.31) cannot carry an unmarked claim: each paragraph is cited with fresh tokens or ends with one of the mode's status tags, such as `[inferred]` or `[open]` ([delivery modes](delivery-modes.md#claims-inside-text)). With `min_citations`, a note with no citation at all fails `claims.cited-fresh` instead of passing it.

## What this does not do

- **Correctness:** a report that is wrong and well formed passes `invocation.bound` and covers its unit. Coverage is not correctness.
- **A statement a harness cannot make:** one such as `context.submitted` stays open, and the report is not ready. Nothing turns a statement into a check.
- **Harness shape:** no judgment depends on which harness ran the worker. A harness that reads only `instructions.md` and `input` gives the same standings as one that reads `invocation.json`.
- **Forged records:** these, and a bypassed hook, are not resisted inside a run. The boundary that resists a worker that does not cooperate is a receiver ([`handoff check`](policy.md), [#64](https://github.com/writfold-dev/writfold/issues/64)). Whether an effect outside Writfold happened once is not something Writfold knows.

## Known limits

Each of these is how the binary behaves today ([RFC](../rfcs/2026-11-26-research-contracts-second-slice.md)).

- **A record that does not exist yet.** A member outside the scope is refused (above), but no manifest can name a record added later outside the scope gate's `scope`. Only `scope = ["**", "!runs/**"]` makes "a newly related record cannot leave an old complete standing" true; a narrower `scope` is the author's claim, and `kb lint` says so (`policy.narrow-scope`, a warning).
- **The seed is not a member.** It is part of every unit (its bytes are), but it is never paired with a member.
- **`others` is more than code locators.** A KB document that is not a record (a note that mentions the seed) is counted there and is not a member.
