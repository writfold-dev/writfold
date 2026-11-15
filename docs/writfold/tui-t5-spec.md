# `writfold watch` T5: the knowledge base in the same frame

Reviewed: main at 024d525 (v0.5.0-dev.7), `crates/writfold-tui/src`, `crates/writfold-kb/src/verbs`,
the guides (`watch.md`, `json.md`, `kb-layout.md`), design §3, §5, §6, §12 and §34–36. Every verb
below was run (text and `--json`) against two scratch copies: the T3 capture KB, and a world built
for this spec from `fixtures/kb/basic` plus a two-commit code repository and real `record`,
`cite make`, `checkpoint` and `stale` calls (section 6). The owner's working KB was cloned and read
from the clone; nothing was run against the original. Read-only review; no repository file changed.

Owner request: extend the viewer from the run story to the knowledge base itself, as one journey in
one tool with one look, instead of a separate reader. T5 adds a second **mode** to `watch`: the run
mode is unchanged, the KB mode shows the same bars, rails, glyphs, chips, calm/`d`/`?` scheme over
the records. The Obsidian reading profile (`docs/writfold/obsidian-reading.md`, `fixtures/views`)
stays in the repository as history and is not touched.

## 1. What the KB holds and which verbs expose it

Record kinds as the index counts them (`kb index status`, capture KB: 51 files, 38 edges, 19
mentions, 7 citations): `project`, `asset`, `relationship`, `dossier`, `decision`, `ticket`,
`investigation`, `signature`, `runbook`, `evidence`, `correction`, `run`, and `doc` for any other
tracked `.md`/`.json`. Status vocabularies are per kind (`verbs/mod.rs::vocabulary`); a word outside
them is kept and flagged `vocab=unknown`. Edges have an origin (`authored`, `field`, `cite`,
`mention`) and a status; mention edges are inferred and not walked by default.

| Question | Verb (JSON) | Rows it gives | Freshness it carries |
|---|---|---|---|
| projects | `brief --all` | `project` rows (id `project:NAME`, status = phase, text = heading) | trailer `counts.stale` (cached verdicts) |
| why | `status project:P` | every record whose field, cite or authored link points at the project: decisions in all states **with titles**, assets, promoted tickets, runs; `section: tracked-by`, a `via` continuation | row flags: `superseded→ID`, `past-ttl`, `vocab=unknown` |
| why, one decision | `why ID` | the live decision (a superseded ID answers with its successor and `via: supersedes`), `decided`, `why`, `cite` continuations; `section: secondary` rows (investigation and ticket outcomes); a draft answers `unknown reason=no-accepted-decision` | each `cite` continuation's flags: none when fresh, `moved→5-7`, `changed`, `gone`, `unchecked`, …; trailer `counts.stale` |
| status | `brief P` | sections `header`, `checkpoint`, `open`, `decisions`, commits, `stale` | `stale` section rows, `counts.stale` |
| tracker / impact | `impact ID --direction in\|out\|both --depth N` | one row per reached record: kind, id, status, `text = depth=N SRC REL DST (ORIGIN)`, `path:line` = the edge's provenance | edge flags (`stale`, `changed`, `moved→…`, `past-ttl`), counts `frontier`, `skipped_retired\|disputed\|inferred`, `stale` |
| one record | `status ID` | the record with `field` continuations at their lines; `tracked-by` rows, then `mentions` rows flagged `inferred` | node flags |
| who cites a file | `uses code:REPO:PATH`, `impact code:REPO:PATH` | referrers with `via` lines; code nodes appear in `impact` rows as kind `code` | as above |
| record to runs | `timeline ID` | dated events; `checkpoint` rows whose **id is the run id** for each checkpoint entry naming the record | flags of linked records |
| run to records | `timeline RUN` (already asked) | rows of kinds other than `run`, `checkpoint`, `plan`: records the run's checkpoints name | their present states |
| stale | `stale --cached` | `cite` rows (id = cited target, `path:line` = the citing file), assets past `ttl`, regressions | status `moved\|changed\|gone\|expired\|unverifiable`, counts `checked`, `stale`, `unverifiable` |

Observed facts that shape the design:

1. **No verb lists records by kind.** `find` refuses an empty text (`find takes some text`) and caps
   at 50 rows; `brief` lists only open items and accepted decisions. Lists therefore come from the
   project: `status project:P` (titles) and the bounded walk `impact project:P --direction in`
   (19 records for `harbor` in the built world, each with its parent edge).
2. **`impact` rows have no title and no structured edge.** The edge is in `text`. The row's `id` is
   one end of it, so the other end (the parent) is recoverable without guessing (section 2.4).
3. **`stale P --cached` is scoped by path**, so a project scope misses citations in
   `proposals/…` and `inventory/…` (6 rows against 9 KB-wide in the built world). KB mode asks the
   whole KB.
4. **`why` prints cached verdicts** (`[unchecked]` until someone runs `kb stale`); `stale --cached`
   judges Git-backed citations when asked. The two can disagree, and the verbs' own note says so.
5. **Plan and gate rows name receipts, never record ids.** A checklist cover's evidence is a gate
   reference or a token inside the receipt file; no verb row exposes it. Runs and records meet only
   through checkpoint text (`timeline`, both directions) and the run's `project` field.
6. **The owner's working KB today** (clone at 4f6b974): `project=1`, `run=1`, 51 files, no
   decision, ticket, asset or citation. KB mode there is one project and empty lists; the empty
   states in 2.8 are part of the design, and the mocks use the built world.
7. Reads write only the rebuildable index (`<git dir>/writfold/kb-index-*.sqlite`); `stale --cached`
   does not write `cite_checks`. KB mode adds no other effect.

**Verb additions in T5: two, both additive (see section 4 and section 7).** The section 4 text below describes them as deferred; section 7 overrides that.

## 2. Design

### 2.1 Modes and keys

`App` gains `mode: Mode { Runs, Kb }`. Run mode is today's frame, byte for byte except the tab row
(below). Each mode keeps its own selection, view, focus and scroll, so switching back returns to the
same place. `d`, `?`, `r`, `T`, `q`, the two-second poll and the theme and colour rules are shared.

| Key | Run mode | KB mode |
|---|---|---|
| `m` | open KB mode (first time: the selected run's project, else the first project; view `why`) | back to run mode |
| `f` (follow) | KB mode on the first record the run's checkpoints name (`timeline RUN`, first row whose kind is not `run`, `checkpoint` or `plan`): in `why` when it is listed there, otherwise `impact` rooted on it. None: enter KB mode with the notice `this run's checkpoints name no record` | on a record with a `runs` section, or an entry of kind `run`: run mode, that run selected, view Timeline (Progress for a `run` entry). No listed run: notice `no listed run names this record`, nothing changes |
| `a` | unbound | impact of the selected record: view `impact` rooted on it (in `impact`, re-root) |
| `v` | unbound | in `impact`: cycle direction `in` → `out` → `both`; elsewhere nothing |
| `1`–`5`, Tab, Shift-Tab | views 1–8 as today | the five KB views |
| `p g t i u s b e` | that run view | **the same**: leave KB mode for that run view. Letters never change meaning |
| `j k`, arrows, PgUp/PgDn | as today | select an entry; scroll a focused inspector or the status text |
| `h l`, Enter, Esc | focus | focus: projects → list → inspector |
| `[` `]` | previous/next run | previous/next project |
| `n` `N` | next/previous flagged step | next/previous entry carrying a mark, wrapping |
| `0` | runs list (narrow) | projects list (narrow) |
| `x X o P .` | as today | nothing (no notice) |

`c` stays unbound; no key writes. Free keys used: `m`, `f`, `a`, `v`. `--view kb:why|kb:status|
kb:tracker|kb:impact|kb:stale` starts in KB mode (the project of `--run`, else of the current run,
else the first).

### 2.2 Views

| # | View | Asks | List pane |
|---|---|---|---|
| 1 | `why` | `status project:P` | rows of kind `decision`, verb order, one rail under the header `decisions`; label = the row's `text` (title), word = `status` |
| 2 | `status` | `brief NAME` (`NAME` = the project id without its `project:` prefix; `brief` does not take the prefixed form) | the answer as text through the existing `lines::reply`, scrolled; not selectable. The inspector shows the project record |
| 3 | `tracker` | `impact project:P --direction in --depth 2` | the tree of 2.4 filtered to kinds `ticket`, `investigation`, `signature`; a kept row's parent becomes a group header (`┬ harbor-db`), so the pane reads "tracker items for Z" |
| 4 | `impact` | `impact SUBJECT --direction D --depth 2` | the unfiltered tree; `SUBJECT` is the record `a` was pressed on, initially `project:P`; `D` initially `in` |
| 5 | `stale` | `stale --cached` (whole KB, asked once per stamp) | rows grouped under their citing file (`┬ harbor/STATUS.md`); label = `id`, word = `status`; `a` on a `code:` row asks the impact of its id without the trailing `:A-B` range |

The inspector (2.5) is the same in every view and always describes the selected entry. The left
pane lists projects from `brief --all`: two lines each, the heading, then the phase and name
(`build  harbor`); `d` adds the id and `@path:line`.

### 2.3 Entries, glyphs and marks

Every list is a `Vec<Entry>`: `id`, `kind`, `status`, `label`, `word`, `path`, `line`, `flags`,
`depth` (0 or 1 below a root or group header). A row is: rail, state glyph, label, mark column (two
cells), one dim word. Labels are padded to the widest label up to the pane width less 20; ids are cut
from the left with `…`, titles from the right (the existing `rail_text::left` and `clip`).

No new shapes. The state glyph is redundancy for a word that is always shown (in the row in lists,
in the inspector and at `d` in trees); it is a lookup on the exact word, never a judgement:

| Glyph | ASCII | Words |
|---|---|---|
| `●` settled | `[x]` | accepted, applied, verified, done, n/a, concluded, fixed, active, closed |
| `○` open | `[ ]` | draft, proposed, landing, deferred, todo, in-progress, blocked, investigating, open, monitoring, suspected, known-error, fix-in-progress, regressed |
| `–` set aside | `[-]` | rejected, superseded, retired, abandoned, deprecated |
| `·` none | `[.]` | no status, or any other word (a project phase, `sunsetting`, `confirmed`, `explicit`) |

Marks say that a source moved; they are the KB's staleness, read from flags only:

- `!` when a flag is one of `stale`, `changed`, `gone`, `expired`, `past-ttl`, `regression`,
  `marked`, `due`, or starts with `moved`;
- `?` when a flag is `unverifiable` or `unchecked`;
- the flags considered are the entry's own, plus those of every `stale --cached` row whose `path`
  equals the entry's `path` (file equality between two verb answers; nothing is read from disk).
  At most one `!` and one `?` per row; the inspector and the `stale` view have the detail.

The citation strip in the inspector is one cell per `cite` continuation of `why`, in verb order:
`●` no flag, `!`, `?` by the same rule (ASCII `+ ! ?`). All other flags (`superseded→…`, `draft`,
`inferred`, `self`, `vocab=unknown`) are printed as words at `d` and in the inspector.

### 2.4 The tree (tracker and impact)

`impact` text is `depth=N SRC REL DST (ORIGIN)`. With the row's own `id` known: strip the `depth=N `
prefix and the final ` (ORIGIN)`; if the remainder starts with `id` and a space, the row is the
source, the next word is the relation and the rest is the parent; else if it ends with a space and
`id`, the rest splits at its last space into parent and relation. A depth-1 row hangs under the
root; a depth-2 row hangs under the entry whose id is its parent, on a dim bar. Order: depth-1 rows
in verb order, each followed by its children in verb order. The dim word is the relation; `d` adds
the status word and a second dim line `@path:line (ORIGIN)`.

If any row fails the rule (a relation with a space, a parent not in the answer), the whole answer is
drawn flat under `┬ depth 1` and `┬ depth 2` headers with the verb's text as the label. Nothing is
dropped either way. The last pane line quotes the trailer: `N reached`, then
`not followed: A retired, B disputed, C inferred` for non-zero counts and `frontier F` when
non-zero, in the warning tone when `coverage` is `partial`. A walk shows each record once, under the
first edge that reached it; its other links are in the inspector's `linked from`.

### 2.5 Inspector

Asked for the selected entry and cached by id until a stamp moves: `status ID`; `why ID` when the
kind is `decision`; `timeline ID`. A `code:` entry asks `uses ID` instead. Calm content, top down:

1. label and a status chip; the full id, dim;
2. decision: `decided …`, the `why` lines (wrapped with a hanging indent);
3. `cites N` with the strip, then only the cites that carry a flag, each with its flag text;
4. other records: `field` lines whose value is neither empty nor `[]`;
5. `see also`: `why`'s `secondary` rows (glyph, title, status);
6. `linked from`: `status`'s `tracked-by` rows (glyph, title or id, the `via` relation);
7. `runs`: one line per distinct run id among `timeline`'s `checkpoint` rows, shown by the title
   the runs list has for that id, else the id;
8. the verbs' notes (for example the no-cached-verdict note), dim;
9. the source, `@path:line`, cut from the left to one line;
10. the hint `d sources and fresh cites   f run   a impact`.

`d` adds `@path:line` to every line, every cite including fresh ones, outcome lines under
`see also`, the `mentions` rows (flagged `inferred`), every field, checkpoint times under `runs`,
all flags as words, and each verb's `coverage` and `records` from its trailer. An `unknown` answer
(a draft's `why`) prints its reason and the verb's boundary note, never an empty section.

### 2.6 Bars

- **Header**: project heading (bold), phase chip, `knowledge base` (muted); the project id
  right-aligned (`project 1/2` when narrow). `knowledge base` drops first.
- **Tabs**: five tabs with the same chip, gap and narrow rules as today. Only `stale` carries a
  badge, from its own trailer: `!N` (`counts.stale`), `?N` (`counts.unverifiable`). The right end of
  the row holds the mode switch in both modes: `m kb` in run mode, `m runs` in KB mode (key in the
  accent tone, label muted). It is the first unit dropped when the row is short and sits before
  `i/N` in the narrow form. This is the only change to run-mode frames.
- **Footer**: unchanged left group (`kb` and `index` from the `brief --all` trailer). Hints: list
  `j/k record   l inspect   a impact`; inspector `j/k scroll   h back   d detail`; projects
  `j/k project   l records`; status view `j/k scroll   Tab view`. Narrow drops the first hint.
- **Help** (`?`) is per mode: KB mode shows the marks of 2.3, "This frame", the keys of 2.1 and the
  verb beside each view.

### 2.7 Frames

Calm, 136×38 (Medium: 22, 57 and 57 columns), clay, list focused, the third decision selected. Chips
are shown as padded text; blank pane rows and the blank lines between inspector sections are
trimmed; `asked` is the view clock.

```text
 Harbor status   build   knowledge base                                                                                   project:harbor
  1 why    2 status    3 tracker    4 impact    5 stale !9                                                                       m runs 
╭ projects (2) ──────╮╭ why (5) ──────────────────────────────────────────────╮╭ decision 3/5 ─────────────────────────────────────────╮
│> Harbor status     ││ ┬ decisions                                           ││Batch ledger writes in the API  accepted               │
│  build  harbor     ││ – One writer process for the ledger     superseded    ││proposal/decision/2026-03-02-batch-ledger-writes       │
│  Lantern console   ││ – Drop client retries on flush          rejected      ││                                                       │
│  lantern           ││ ● Batch ledger writes in the API     !  accepted      ││decided by ops-review on 2026-03-09                    │
│                    ││ ○ Shard the ledger by tenant            draft         ││                                                       │
│                    ││ ○ Cap replay batches at 500 rows     !  draft         ││why  Group ledger writes into batches of up to 500     │
│                    ││ ┴                                                     ││     rows per transaction.                             │
│                    ││                                                       ││why  Batching keeps the flush under the store's fsync  │
│                    ││                                                       ││     budget.                                           │
│                    ││                                                       ││cites 2  ● !                                           │
│                    ││                                                       ││  ! code:demo-service:src/batch.rs:2-4 [moved]         │
│                    ││                                                       ││    moved→5-7                                          │
│                    ││                                                       ││see also                                               │
│                    ││                                                       ││  ● Flush stalls on the ledger store  concluded        │
│                    ││                                                       ││  ● Ledger flush stalls on slow disk  done             │
│                    ││                                                       ││linked from                                            │
│                    ││                                                       ││  – One writer process for the ledger  superseded-by   │
│                    ││                                                       ││runs                                                   │
│                    ││                                                       ││  Add --plan to kb runs                                │
│                    ││                                                       ││@…/2026-03-02-batch-ledger-writes/proposal.md:5        │
│                    ││decisions that name harbor as their project            ││d sources and fresh cites   f run   a impact           │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 20cd29c9466b   index fresh   asked 00:21:18Z                        j/k record   l inspect   a impact   ? help   q quit
```

The `impact` pane after `a` on `harbor-db` (direction `in`), same size:

```text
╭ impact ───────────────────────────────────────────────╮
│in  what cites or links to harbor-db                   │
│                                                       │
│ ┬ harbor-db                                           │
│ ○ HBR-101                           concerns          │
│ │ · project:harbor               !  cites             │
│ ○ HBR-102                           concerns          │
│ │ ○ …-replay-batches-at-500-rows !  references        │
│ ● HBR-99                            concerns          │
│ ● harbor-api                     !  depends-on        │
│ │ · dossier:harbor-api           !  dossier-of        │
│ │ · runbook:ops/restart-harbor      concerns          │
│ ● …:2026/2026-02-10-flush-stalls    concerns          │
│ ○ …2026-06-01-flush-stalls-again !  concerns          │
│ ● …026-03-02-batch-ledger-writes !  references        │
│ │ – …on/2026-01-10-single-writer    superseded-by     │
│ · quay-worker                       depends-on        │
│ ● sig:fsync-stall                   concerns          │
│ ┴                                                     │
│14 reached   not followed: 1 disputed, 6 inferred      │
╰───────────────────────────────────────────────────────╯
```

`out` reads `out  what harbor-db cites or links to`; `both` reads `both  every link of harbor-db`.
Calm, 80×24 (narrow; `l` swaps the list for the inspector, `0` shows projects):

```text
 Harbor status   build                                               project 1/2
  1 why    5 stale !9                                               m runs   1/5
╭ why (5) ─────────────────────────────────────────────────────────────────────╮
│ ┬ decisions                                                                  │
│ – One writer process for the ledger           superseded                     │
│ – Drop client retries on flush                rejected                       │
│ ● Batch ledger writes in the API           !  accepted                       │
│ ○ Shard the ledger by tenant                  draft                          │
│ ○ Cap replay batches at 500 rows           !  draft                          │
│ ┴                                                                            │
│decisions that name harbor as their project                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 20cd29c9466b             l inspect   a impact   ? help   q quit
```

Wide (140×32 and up): projects 40, list, inspector 48; the run mode's side stack has no KB
counterpart. Degraded forms follow the existing rules (`--color none`, `NO_COLOR`, `TERM=dumb`:
ASCII glyphs and rails, bracket chips, no background):

```text
 Harbor status  [build]  knowledge base                                                                                   project:harbor
 [1 why]   2 status    3 tracker    4 impact    5 stale !9                                                                       m runs 
 [x] Batch ledger writes in the API     !  accepted
 [read-only]  kb 20cd29c9466b   index fresh   asked 00:21:18Z                        j/k record   l inspect   a impact   ? help   q quit
```

### 2.8 Empty, refused and plain forms

- No project rows: the left pane reads `no project records`, the list pane prints `brief --all`'s
  notes, and the footer hint is `m runs`.
- A list with no entry of its kind: one dim line, `harbor names no decisions` (tracker:
  `nothing tracked within 2 links of harbor`; stale: `nothing stale; N checked`), then the verb's
  notes. A refusal stays in its pane as `refused (exit N): …`, as in run mode.
- `--once --view kb:NAME` (or a pipe) prints `== projects ==`, the one section as the verb's text
  through `lines::reply`, and the footer line. `--once` without a `kb:` view is unchanged.
- The honesty rules of run mode hold: no percentage, no relative time, no liveness, no reworded
  verb text. `asked` is still only the view clock.

### 2.9 Refresh

KB answers are cached per project and view, the inspector's per record id, `stale --cached` once.
The existing poll (KB pin and local stamp every two seconds) drops them all and keeps the selection
by id, falling back to the first entry. The subject stamp does not affect KB mode. A cited code
repository moving is not polled: press `r`. `watch` still starts no checker and never runs
`kb stale`.

## 3. Honest limits

| Not shown | Why | What would lift it |
|---|---|---|
| Records outside a project's reach: a decision without `project`, anything further than two links from a project, relationships and plain documents as rows | no verb lists records by kind | deferred addition A (section 4) |
| Titles in `tracker` and `impact` rows | `impact` rows carry the edge, not the title; the inspector has it | deferred addition B, or a second verb call per row (not done) |
| A jump from a plan step or gate to the record it cites | step and gate rows name receipts and gate references, not record ids | a `cite` continuation on `plan status` items; not planned here |
| Runs that cite a record in cover evidence or retrieval receipts | only checkpoint text links a run to a record (`timeline`); those links are mentions, not citations, and the frame says `named`, never `cites` | the same |
| Opening a record or file by typing | `watch` has no text entry; `find` is not reachable | a `/` prompt over `find`, deferred |
| Verdict agreement between the mark and the `why` line | `why` reads cached verdicts, `stale --cached` judges at ask time (fact 4) | the user runs `writfold kb stale`; the verb's note is shown |
| Mention (inferred) links in the tree, depth other than 2, history at a commit | kept out of the key map for size; the skipped count is shown | a later toggle |
| Record bodies, dossier sections, relationship evidence text | the viewer formats verb answers and reads no file | open the path the inspector ends with |
| Much to look at in the owner's KB today | fact 6: one project, one run | recording decisions and tickets in real work |

## 4. Deferred verb additions (specified, not built in T5)

- **A. `find --kind K[,K] [--project P]` with no text** lists every record of those kinds in id
  order, each with a `match: listed` continuation, paged by the budget. Today that call is an
  argument error, so no reader of the current output changes; the envelope stays at version 1.
  About 40 lines in `verbs/find.rs` and the argument parser, one golden, one sentence in `json.md`.
  With it, `why` and `tracker` could list the whole KB and a sixth view `all` becomes possible.
- **B. An `edge` continuation on `impact` rows** (`label: edge`, text `SRC REL DST`, the edge's
  position) would replace the parse of 2.4. It changes the text form (one more indented line per
  row), so it needs the owner's agreement and new goldens. The parse has a tested fallback, so T5
  does not wait for it.

## 5. Implementation plan (one long session)

Budgets: `app.rs` 198, `state.rs` 208, `bar_content.rs` 212 and `layout.rs` 203 lines are near the
250-line limit, so KB code lives in new modules and existing files gain only dispatch lines.
Functions stay under 80 lines; no `unwrap`/`expect`; no new crate; `writfold-kb` is not edited.

| # | Slice | Files |
|---|---|---|
| 1 | Fixture world and capture test (section 6) | `tests/support/kb_world.rs`, `tests/support/mod.rs` |
| 2 | Mode and state: `Mode`, `KbView`, `KbState { project, view, focus, selected: per-view id, subject, direction, scroll, answers, records, stale }`; `App.mode`, `App.kb`; `reload` clears the KB caches; loaders. `Source` methods `projects`, `tracked`, `walk`, `why`, `record`, `uses`, `named`, `stale_all` | new `kb_state.rs` (≈180), new `kb_source.rs` (≈90), `app.rs`, `state.rs` |
| 3 | Entries: the model, the glyph table, marks with the path join, projections for `why` and `stale` | new `kb_entry.rs` (≈150) |
| 4 | Tree: the edge rule, ordering, the kind filter with group headers, the flat fallback, the trailer line | new `kb_tree.rs` (≈130) |
| 5 | List pane: rows, columns, clipping, window with two rows of context, detail lines, projects pane, empty and refused forms | new `kb_rail.rs` (≈170), new `kb_view.rs` (≈150), `view.rs` (dispatch), `layout.rs` (`kb_areas`, ≈25) |
| 6 | Inspector, calm and detailed | new `kb_inspect.rs` (≈200) |
| 7 | Keys: `m`, `f`, `a`, `v`, KB motion, `[ ]`, `n N`, digits, the letters leaving KB mode | new `kb_keys.rs` (≈140), `keys.rs` (dispatch, ≈12) |
| 8 | Bars and help: KB header, tabs, footer hints; the `m kb` / `m runs` unit; KB help | new `kb_bars.rs` (≈120), new `kb_help.rs` (≈50), `bar_content.rs`, `chrome.rs`, `help.rs` |
| 9 | `--view kb:NAME`, the plain frame, usage text | `lib.rs`, `frame.rs` |
| 10 | Goldens, a tmux pass (80×24, 100×30, 136×38, 160×40; truecolor, `--color ansi`, `--color none`, `TERM=dumb`), the gates from main | `tests/golden/*` |
| 11 | Docs | below |

Order: 1 → 2 → 3 → 5 (the `why` list on screen) → 6 → 7 → 4 (tracker, impact) → 8 → 9 → tests →
10 → 11. Size: about 1,250 lines of source in nine new modules and 60 changed lines, about 550 of
tests and support, 6 new goldens, and the interactive run-mode goldens re-blessed for the tab row.
This is roughly twice T4. **Cut line** if the session runs short, in this order: the plain frame
(9), the `status` view (run mode's Brief already shows the same answer), `a` from a `stale` row,
then the `stale` view itself (its marks and badge stay). Modes, `why`, the inspector, the tree and
`m`/`f` are the slice that answers the request and are not cut.

### Tests (each fails under the named mutation)

New `tests/kb_mode.rs`, `tests/kb_tree.rs`, `tests/kb_frames.rs`; unit tests beside `kb_tree.rs`.

- `run_mode_gains_only_the_mode_unit`: at 160×40, 136×38 and 80×24 a fresh `App` shows `m kb` on
  the tab row and nowhere else, and no KB pane title; blanking that unit in the buffer gives the
  pre-T5 golden text (kept as `tests/golden/pre-t5/` copies for this test). Mutation: drawing any
  KB pane, hint or header segment in run mode.
- `m_round_trips_selection`: run 2 and a map node selected, `m`, move, `m`: run, node, view and
  scroll as before; and the reverse for KB mode. Mutation: resetting either mode's state on switch.
- `letters_keep_their_meaning`: in KB mode `g` yields run mode on Gates; `3` yields `tracker`.
  Mutation: mapping `t` to tracker.
- `why_lists_exactly_the_verbs_decisions`: labels, words and order equal the `decision` rows of
  `status project:harbor`; a decision added through `record decision` appears after a poll.
  Mutation: sorting or filtering by state.
- `glyph_is_a_lookup_on_the_word`: every vocabulary word of `ticket`, `decision`, `investigation`
  and `signature` maps as the table says; an invented word and a missing status give `·`, and the
  word is still printed. Mutation: defaulting unknown words to `●`.
- `marks_come_from_flags_and_the_path_join`: the decision citing `src/batch.rs` shows `!`; with the
  code repository restored to its first commit and `r`, it shows none; an unresolvable repository
  shows `?`, never nothing. Mutation: dropping the join, or treating `unverifiable` as fresh.
- `tree_parents_follow_the_edge`: for `impact harbor-db --direction in`, each depth-2 entry sits
  under the id its text names; the entry count equals the verb's `shown`. Mutation: nesting by id
  prefix. Unit: 200 generated answers (ids with `:`, `/`, `#`; the row as source or destination)
  reproduce the parent that generated them.
- `unparsable_edges_fall_back_flat`: one row with a two-word relation gives depth headers and every
  row once. Mutation: skipping the unparsed row.
- `walk_limits_are_on_screen`: the last pane line holds the trailer's reached, skipped and frontier
  numbers; a `partial` answer takes the warning tone. Mutation: omitting zero-suppressed counts
  that are non-zero.
- `inspector_calm_and_detail`: calm has no `@` except the final source line, lists only flagged
  cites, and its strip has one cell per `cite` continuation; after `d` every cite, `mentions`, each
  line's `@path:line` and the trailer coverage are present; a second `d` restores the first buffer.
  Mutation: unbinding `d` in KB mode.
- `draft_why_says_unknown`: the inspector of a draft prints `no-accepted-decision` and the boundary
  note. Mutation: hiding an `unknown` answer.
- `follow_both_ways`: `f` on the run whose checkpoint names the accepted decision lands on it in
  `why`; `f` there returns to that run on Timeline; a run naming nothing gives the notice; with
  `o` filtering the run out, `f` gives `no listed run names this record`. Mutation: taking the run
  from the runs list's order instead of the `checkpoint` row's id.
- `impact_of_a_file`: `a` on the `code:` entry under a decision roots the tree on it and lists both
  citing decisions. `v` cycles three directions and the verb is re-asked.
- `stale_badge_equals_the_verb`: `!N`/`?N` equal `stale --cached`'s counts; zero prints no badge.
- `kb_bars_never_cut_a_token` (widths 60..=200) and `kb_chips_degrade_to_brackets`, extending the
  T4 bar tests; `honest()` runs over every KB buffer at both levels.
- `kb_mode_starts_no_process_and_writes_nothing`: the KB work tree status and the `cite_checks`
  row count are equal before and after visiting every view. Mutation: asking `stale` without
  `cached`.
- `empty_kb_mode`: a KB with one project and one run (the shape of fact 6) renders the empty forms
  and exits cleanly on `q`.

Goldens: `t5-why-136x38.txt`, `t5-why-80x24.txt`, `t5-why-detail-136x38.txt`,
`t5-impact-136x38.txt`, `t5-tracker-100x30.txt`, `t5-once-kb-why.txt`; bless with
`WRITFOLD_KB_BLESS=1` and read each diff. The re-blessed run-mode goldens may differ on row 2 only.

### Docs

- `docs/guide/watch.md`: the status line; a new section "The knowledge base" (modes, the five views
  with their JSON sources in the existing table form, entries and marks, the tree and its limits,
  the inspector, `m`/`f`/`a`/`v`, the two calm frames); the controls table; "Not built".
- `docs/guide/README.md`: the `watch` row and the status line.
- `docs/writfold/kb-v1-design.md`: new **§37 T5 as built**, written by the implementer: 37.1 modes
  and keys, 37.2 sources per view, 37.3 entries, glyph table and marks, 37.4 the tree rule and
  fallback, 37.5 inspector, 37.6 bars and degraded forms, 37.7 tests with their mutation checks,
  37.8 deviations from this spec, 37.9 not done (section 3 and the cut line, as they turned out).
- `docs/writfold/mission.md`: one roadmap line for T5. The website gets a KB frame only after the
  capture exists; until then its copy says planned.

## 6. Fixture world

`tests/support/kb_world.rs::atlas(name)`, also used for captures. Records that a verb can write are
written by that verb; assets, projects, dossiers and the older decisions come from
`fixtures/kb/basic`, because no verb creates them. Clock: the suite's `NOW` and offsets from it.

1. `map::nested(name)`: the existing world (the `basic` copy, the run with its nested plan, gates).
2. A code repository `demo-service` beside the KB with the five files the fixture cites
   (`src/ledger.rs`, `batch.rs`, `retry.rs`, `legacy.rs`, `db.rs`), one commit; its path written to
   `<git dir>/writfold/repos`.
3. Each `⟦cite kind=code … {{…}}⟧` placeholder token in the copied files is replaced by the token
   that `cite make demo-service PATH A-B` prints (`Write::CiteMake`); a bare `{{REV}}` becomes the
   commit. No digest is computed by test code.
4. `record ticket HBR-102 --title "Replay too slow" --assets harbor-api,harbor-db --status
   in-progress`; `record decision --title "Cap replay batches at 500 rows" --project harbor
   --references harbor-api,HBR-102 --why … --evidence TOKEN` with a `cite make` token for
   `src/batch.rs 2-4` (it stays `draft`: `status --set proposed` refuses a decision without its
   review sections, which is the product's rule and is left alone); `record link depends-on
   quay-worker harbor-db --evidence demo-service@REV:src/ledger.rs:3-5`; `checkpoint --run RUN
   --note` naming the new decision and `HBR-102`. Commit the KB.
5. A second `demo-service` commit: two comment lines above `batch.rs` (moved), an edit in
   `retry.rs` (changed), `legacy.rs` deleted (gone), `db.rs` renamed (moved to a new path);
   `ledger.rs` untouched (fresh).
6. One `stale` (not cached) so `why` has verdicts. Checked in the scratch build: 11 checked, 9
   stale, 0 unverifiable; verdicts `moved→5-7` twice, `changed`, `gone`, `moved→src/store/db.rs:1-3`,
   plus the fixture's expired web citation, changed record citation, the asset past its `ttl` and
   the regression.

Tokens hold the code repository's local path in `url=`; no calm or detailed frame prints a token,
and a test asserts that no buffer contains the world's base path. For website captures an ignored
test `capture_world` builds the same world under `target/t5-pty/kb` and leaves it; the frames are
taken with `writfold watch --kb target/t5-pty/kb --view kb:why` at 136×38 after T5 is merged.

## 7. Decisions (15 November 2026)

1. KB mode opens on `why`; the impact tree is one key away (`a`).
2. The state glyph is the fixed word table of 2.3, with `·` for anything else; the word is always printed.
3. Marks join `stale --cached` rows to entries by file path, as specified.
4. **The viewer does not parse verb text.** This overrides sections 1 (fact 2), 2.4, 3 and 4 where they say otherwise. The JSON guide already tells readers to use structured fields, not to parse the text form, and the viewer must follow its own rule. Both additions in section 4 are built in T5, as the first slice, in `writfold-kb`:
   - **A. `find --kind K[,K] [--project P]` with no text** lists records of those kinds as section 4 describes (today an argument error, so nothing that works today changes). The `why` view lists the project's decisions from `find --kind decision --project P` when that gives at least what `status project:P` gives, and keeps titles; the projects pane gains a last entry, `all records`, whose `why` and `tracker` lists come from `find --kind` without `--project`. If `find` already supplies a title in `text`, use it.
   - **B. A structured edge on `impact` rows, in JSON only.** Each `impact` row gains an optional key `edge` (an object: `from`, `rel`, `to`, `origin`, plus `depth`), absent (not null) on rows of other verbs. The text form does not change, so every text golden stays. Document it in `docs/guide/json.md` next to the `status` key rule. The tree of 2.4 reads `edge` only; the text-parsing rule and its flat fallback are dropped. A row without `edge` (an older binary is impossible here, but be total) is drawn flat under its depth header.
   - Each addition has its own tests (argument handling, ordering, budget/paging, JSON shape, the text golden unchanged) and a mutation that fails them.
5. Titles in `tracker`/`impact` rows: if `edge` work makes the target's title available cheaply from the index, add it as an optional `title` key on `impact` rows in the same JSON-only way; otherwise leave titles to the inspector and record it under Not done.
6. The cut line of section 5 stands, with the verb slice added at the front and not cuttable.
