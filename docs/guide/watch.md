# Watching a run

> **Status:** built (T6, built in v0.5.0-dev.10: plain words by default, the verbs' exact wording on `d`, commands to copy on `c`). Implementation, tests and deviations: [design §41](../writfold/kb-v1-design.md#41-t6-as-built-v050-dev10); the knowledge-base mode is recorded in [§37](../writfold/kb-v1-design.md#37-t5-as-built), the calm frame it replaced in [§36](../writfold/kb-v1-design.md#36-t4-as-built), the plan map in [§35](../writfold/kb-v1-design.md#35-t3-as-built) and the earlier text view in [§34](../writfold/kb-v1-design.md#34-t2-as-built).

`writfold watch` is a read-only view of the KB verbs. It shows recorded steps,
gate standings, checklist items, pending drafts and proposals, guard rules and
checkpoint history. It does not schedule work or claim that an agent is working.

```sh
writfold watch
writfold watch --kb /path/to/kb --repo /path/to/repository
writfold watch --run RUN_ID --view gates --once
writfold watch --theme paper --color truecolor
writfold watch --view kb:why
```

The KB is discovered from the original working directory before `--repo` changes
the subject working directory. `--repo` must name a directory in a Git work tree.
`--run` selects a run; otherwise the current-run pointer wins, then the first
listed run. Runs that are not closed come first, preserving each group's verb order.

## Panes and their sources

Each answer is requested in-process as the verb's JSON, with the full output
budget. The viewer formats the answer; it does not read records or judge gates.

| View | Key | JSON source | Scope |
|---|---|---|---|
| Runs | left column; `0` in narrow layouts | `writfold kb runs --plan [--open] [--project P] --json` | Listed runs, including recorded plan positions |
| Progress | `p`, `1` | `writfold kb plan status --run R --gates --json` | Plan map and selected-node inspector; `x` switches to the text list |
| Gates | `g`, `2` | `writfold kb gate status --run R --json` | Selected run's gate counts, receipts and next commands |
| Timeline | `t`, `3` | `writfold kb timeline R --json` | Checkpoints, folded plan events and mentioned records with their present states |
| Inbox | `i`, `4` | `writfold kb resume R --json` | Draft and proposal rows, including confirm/accept commands |
| Guards | `u`, `5` | `writfold kb guard status --json` | **Current run**, not the selected run; rules judged as if matched |
| Stale | `s`, `6` | `writfold kb stale P --cached --json` | Selected run's project; the whole KB when it names none |
| Brief | `b`, `7` | `writfold kb brief P --json` | Selected project; the verb infers it when absent |
| Resume | `e`, `8` | `writfold kb resume R --json` | The complete resume answer |

`g` now means **Gates**; Resume moved to `e`. The Guards view explicitly says when
its current run differs from the selection. It is a guard, not enforcement, and
does not evaluate the separate delivery guard.

A run without a plan shows the verb's no-plan note followed by its Gates content.
A refusal stays in its pane as `refused (exit N): …`; it does not end the view.
Failure to discover the KB or obtain the runs list exits with an error.

## Recorded progress, not estimates

Progress defaults to a bracket-rail map: one step per row in the verb's order.
Group headings quote `splice`; nesting follows `under`, never the spelling of
an ID. Children precede their parent. Each step retains its own recorded state,
including a done step after open children following a merge.

| Step state | Unicode | ASCII |
|---|---|---|
| Done | `✓` | `[x]` |
| Done with stale evidence | `!` | `[!]` |
| Skipped | `–` | `[-]` |
| Recorded current | `▶` | `[>]` |
| Later | `○` | `[ ]` |
| Invalid flag | `×` | `[#]` |

At the exact level (`d`), gate chips follow each gate continuation's `status`: `●` satisfied, `◆` attested
or waived, `○` open, `!` stale, `×` failed, `?` unknown, and `·` later, not judged.
ASCII chips are `+` satisfied, `*` attested/waived, `o` open, `!` stale, `x`
failed, `?` unknown and `.` later; every chip shape differs, and the inspector
retains the exact state words. ASCII rails use `|`, `+`, `\`, `/`.
`--color none`, automatic `NO_COLOR`, and `TERM=dumb` select ASCII map shapes;
`TERM=dumb` keeps those shapes even when colour is explicitly enabled.

The marker means **the next step to pick up**: the recorded current step (the
first step neither done nor skipped), not a liveness claim. Nothing records
whether an agent is working; a run whose agent stopped shows the same marker.
The line above the rail reads `step N of M` with the verb's done and skipped
counts and, in words, the counts that need a look (`1 out of date`); only
those take a warning tone. Below the rail are the newest plan event from
Timeline in words (`last recorded   2026-11-10 23:59 · prepare done`; the
verb's line, with the run list's last checkpoint, at the exact level) and a
one-line caption for the marker. Without a subject repository the gate counts
stay missing, the line says `gates not judged (no repository)`, the header says
`no repository: gates not judged`, and the verb's note is quoted when it reads
as words; the position still comes from the row the verb marks current.
Done stays done when its evidence becomes stale; the marker never moves back.

Selection starts at the recorded current step, survives refresh by ID, and falls
back to the current step or first row if its ID disappears. The rail keeps it
visible with two rows of context where space permits. Group headers name the
template (`findings`); inside a group a row shows its id after the group's name
(`check` under `findings`), and the exact level shows the full id. IDs truncate
from the left, and text that does not fit, headers included, ends in `…`. Rail
bars are always dim; only the state glyph takes a state colour. A row carries
words only when it is the recorded current step (`needs review-list and
signoff`), the selected row (`done 2026-11-10 23:47`) or flagged (`done; lint
is now out of date`, `done out of order`); the state glyph says the rest. The
legend is in help (`?`, "Marks" and "Words").

The inspector reads top down: the step's name and `next up` (or `done`,
`skipped`); where it sits (`step 7 of 11 · in findings`); what its state means
(`Needs 2 gates before it can be marked done.`, from the plan's owed count;
`Done on 2026-11-10 23:50.`; `It stays done, but 1 of its gates is now out of
date.`); each gate with its standing (`review-list   out of date`) and its
class (`checklist · 2 of 4 required items covered`, `decided by a person's
statement; no command is run`); the checklist items still open or out of date
(`□ docs    not covered · The guide names every new flag`); then `Read first`
and `Suggested` (`give evidence for tests in review-list`). At the exact level
it shows the splice and parent, the gate chips, the segment strip (`■`
supported, `◆` not-applicable/attested, `!` stale, `□` open, then `(+N
optional)`, one per required item, never scaled), every item including covered
ones, every receipt path and the run's last plan event and checkpoint. `g`
opens the Gates view; no gate-status rows are joined into the inspector.

### Plain and exact

The frame starts **plain**: the product's nouns (run, step, gate, checklist,
guard, draft, proposal) and the names people chose, with no notation. `d`
toggles the **exact** level for the whole frame and back: the verbs' own
wording, ids, flags, receipts and stamps, exactly as the frame showed them
before plain words existed. The level is never written, so a fresh start is
plain. `--once`, `--view NAME --once`, a pipe and `--once --view kb:NAME` print
the exact text, unchanged; the JSON verbs are untouched.

- **States are words.** `out of date` is `stale` everywhere (map, inspector,
  Gates, the tab badge, KB lists); `passed` is only ever `satisfied`;
  `signed off` (attested) and `waived` say that no check passed; `nothing
  recorded yet` is `open`; `no verdict` is `unknown`; `not judged yet` is
  `later`; a guard `would block`, `would ask first` or `would remind`, never
  blocks. `next up` is the recorded current step, not a claim that anyone is
  working. A word with no phrase is printed as itself. `?` prints the whole
  vocabulary under **Words**, each phrase beside its exact word.
- **Whole or nothing.** A verb's sentence is shown unchanged or not at all;
  when a pane leaves one out it ends with `N lines in exact wording (d)` (or
  `N notes …`). Authored text (titles, checkpoint text, checklist
  requirements, `why` lines, field values) is always shown whole.
- **Times are respelt, not computed.** `2026-11-10T23:59:50Z` prints as
  `2026-11-10 23:59` (UTC, as recorded); there is no relative time.
- **The bars speak when something is not normal.** The header's right end is
  `run 1 of 3`; `current run` is a chip. Badges say `4 not met` (gates not
  satisfied, attested or waived), `3 pending`, `1 would block`, and shorten to
  their number before a tab drops. The footer says `KB has 19 uncommitted
  changes` and an index that is not fresh, and nothing else; the stamps are in
  help (`?`, "This frame") and at the exact level.
- **Runs entries** are two lines (title; state when not open, project and the
  plan position); `d` restores the five-line entry with id, fields, plan counts
  and last checkpoint.

#### Words

`?` prints this vocabulary from the tables the frame uses, so help and the
frame cannot disagree. The main rows:

| Plain | Exact word | Where |
|---|---|---|
| `next up` | `current` | a step: the next step to pick up, as recorded |
| `done; lint is now out of date` | `done` with `!stale` | a step that stays done while a gate no longer holds |
| `done out of order`, `two plans on this run`, `spliced again`, `template changed` | `!out-of-order`, `!two-plans`, `!repeated`, `!template-moved` | step flags (the run graph guide's merge disorder and template checks) |
| `passed` | `satisfied` | a gate; the only phrase that says passed |
| `signed off`, `waived` | `attested`, `waived` | a gate decided by a person; no check passed |
| `nothing recorded yet`, `out of date`, `failed`, `no verdict`, `not judged yet` | `open`, `stale`, `failed`, `unknown`, `later` | gate standings |
| `checked by a command`, `decided by a person's statement; no command is run` | `mechanical`, `attested` | gate classes |
| `covered`, `marked not applicable`, `not covered`, `optional ·` | `supported`, `not-applicable`, `open`, `(optional)` | checklist items |
| `would block`, `would ask first`, `would remind`, `would allow` | `would-deny`, `would-ask`, `would-nudge`, `would-pass` | guards: a guard advises, it does not enforce |
| `needs every gate of the run met`, `needs the current step's gates met` | `gates.met`, `step.met` | guard requirements (see [hooks](hooks.md#guard-rules)) |
| `draft`, `pending`, `kept only on this machine`, `in the work tree` | `draft`, `pending`, `!local`, `!tracked` | Inbox |
| `moved`, `changed`, `gone`, `expired`, `overdue`, `came back` | `moved→X`, `changed`, `gone`, `expired`, `past-ttl`, `regression` | cited sources and records in KB mode |
| `about it`, `in project`, `replaced by it`, `cites it` | `concerns`, `project`, `superseded-by`, `cites` | a link, read from the linking record's side |

The plain wide frame (136×38, map focused, blank pane rows trimmed):

```text
 Add --plan to kb runs   open   current run   harbor                                                                         run 1 of 3
  1 progress   2 gates 4 not met   3 timeline   4 inbox 3 pending   5 guards 1 would block   6 stale   7 brief   8 resume          m kb
╭ runs (3) ──────────╮╭ plan ─────────────────────────────────────────────────╮╭ step 7 of 11 ─────────────────────────────────────────╮
│> Add --plan to kb …││step 7 of 11    5 done  1 skipped  1 out of date       ││check   next up                                        │
│  harbor  step 7/11 ││                                                       ││step 7 of 11 · in findings                             │
│  Review the lanter…││ ┬ change                                              ││                                                       │
│  lantern           ││ ✓ research                                            ││Needs 2 gates before it can be marked done.            │
│  Retire the legacy…││ ! implement     done; lint is now out of date         ││                                                       │
│  closed  harbor  c…││ – benchmarks                                          ││review-list   out of date                              │
│                    ││ ├─╮ review                                            ││  checklist · 2 of 4 required items covered            │
│                    ││ │ ├─╮ notes                                           ││  ! tests   out of date · the evidence it cites has    │
│                    ││ │ │ ✓ read                                            ││            changed                                    │
│                    ││ │ ├─╯                                                 ││  □ docs    not covered · The guide names every new    │
│                    ││ │ ✓ read-diff                                         ││            flag                                       │
│                    ││ │ ├─╮ findings                                        ││  □ style   optional · not covered · Names follow the  │
│                    ││ │ │ ✓ prepare                                         ││            module                                     │
│                    ││ │ │ ▶ check     needs review-list and signoff         ││                                                       │
│                    ││ │ ├─╯                                                 ││signoff   nothing recorded yet                         │
│                    ││ │ ○ findings                                          ││  decided by a person's statement; no command is run   │
│                    ││ ├─╯                                                   ││                                                       │
│                    ││ ○ review                                              ││Read first   instructions for signoff                  │
│                    ││ ○ deliver                                             ││Suggested    give evidence for tests in review-list    │
│                    ││ ○ reflect                                             ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││last recorded   2026-11-10 23:59 · prepare done        ││                                                       │
│                    ││▶ the next step to pick up, as recorded                ││d exact wording   c commands   g gates                 │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                           j/k step   l inspect   d detail   ? help   q quit
```

At 80×24 the map has the frame to itself; `l` swaps it for the inspector, whose
footer hint becomes `h back`. Committed buffers:
[plain 136×38](../../crates/writfold-tui/tests/golden/t4-calm-136x38.txt),
[plain 80×24](../../crates/writfold-tui/tests/golden/t4-calm-80x24.txt),
[inspector 80×24](../../crates/writfold-tui/tests/golden/t6-inspector-80x24.txt),
[exact 136×38](../../crates/writfold-tui/tests/golden/t4-detail-136x38.txt).

The text views say the same things in rows. Gates at 136×38:

```text
 Add --plan to kb runs   open   current run   harbor                                                                         run 1 of 3
 1 progress   2 gates  4 not met   3 timeline   4 inbox 3 pending   5 guards 1 would block   6 stale   7 brief   8 resume          m kb
╭ runs (3) ──────────────────╮╭ gates ─────────────────────────────────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││6 gates · 2 passed · 2 out of date · 2 with nothing recorded yet · 4 not met                            │
│  harbor  step 7/11         ││                                                                                                        │
│  Review the lantern guide  ││○ bench         nothing recorded yet · checked by a command                                             │
│  lantern                   ││! lint          out of date · changed in scope: harbor/lint.txt                                         │
│  Retire the legacy flush p…││● notes         passed                                                                                  │
│  closed  harbor  complete  ││○ signoff       nothing recorded yet · decided by a person's statement; no command is run               │
│                            ││● unit          passed                                                                                  │
│                            ││! review-list   out of date · checklist · 2 of 4 required items covered                                 │
│                            ││    ■ errors    covered · 1 reference by agent                                                          │
│                            ││    ! tests     out of date · the evidence it cites has changed                                         │
│                            ││    □ docs      not covered · The guide names every new flag                                            │
│                            ││    ◆ perf      marked not applicable · no hot path touched by agent                                    │
│                            ││    □ style     optional · not covered · Names follow the module                                        │
│                            ││Standings are worked out from the run's recorded results each time this view asks; nothing is run.      │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                          j/k scroll   Tab view   d detail   ? help   q quit
```

Inbox leads with `3 pending · 1 draft · 2 proposals` and one entry per draft or
proposal (`confirming it records a checkpoint`, `pending · kept only on this
machine`); Guards with `2 rules · 1 would block · 1 would remind` and one rule
per row with what it needs (`needs every gate of the run met`); Timeline is
`when   what   name` (`2026-11-10 23:51   step skipped   benchmarks · (by
owner) no hot path touched`) and ends `Times are UTC, as recorded.` Stale,
Brief and Resume show each row as glyph, title, `kind · state` and its
readable lines. Committed:
[gates](../../crates/writfold-tui/tests/golden/t6-gates-136x38.txt),
[gates 80×24](../../crates/writfold-tui/tests/golden/t6-gates-80x24.txt),
[inbox](../../crates/writfold-tui/tests/golden/t6-inbox-136x38.txt),
[guards](../../crates/writfold-tui/tests/golden/t6-guards-136x38.txt),
[timeline](../../crates/writfold-tui/tests/golden/t6-timeline-136x38.txt).

### Commands to copy

`c` replaces the bottom rows of the frame with borderless, full-width lines a
terminal selection copies cleanly: the run and step ids and the files to read
as `#` comment lines, then each suggested command whole, exactly as the verb
gave it. A command longer than the frame breaks at a space and ends in ` \`,
so the copy still runs. In the Gates, Inbox, Guards and Resume views the rows
list every `next`, `read`, `confirm` and `accept` of the view under a `# name`
line; in KB mode they name the record and its file and the verbs the inspector
asked. At most ten rows; `j`/`k` scroll; any other key closes. `watch` runs
none of them, and the first row says so:

```text
 commands for step check · copy into a shell; watch never runs them · UPPERCASE words are placeholders
 # run   run-20261101T090000Z-a1b2c3
 # step  change#1/review/review#1/findings/findings#1/check
 # read  .writfold/gates/signoff.md
 writfold kb gate cover review-list tests supported REF… --by WHO
 any key closes
```

[Committed](../../crates/writfold-tui/tests/golden/t6-commands-136x38.txt).

`x` switches to the earlier text list. There, a clean done step collapses to its
heading and completion time, and a later step to its heading; flagged, current
and skipped steps retain detail. `X` expands or collapses all text steps.
The text strip remains one cell per recorded step, never a completion bar.
Gates, Resume, the inspector and `--once` retain evidence positions.

The viewer adds no progress percentage, ETA, elapsed/relative time, spinner or
run-liveness claim. Authored text and verb states are not censored or rewritten.
Recorded times remain recorded times; `asked HH:MM:SSZ` is when the view last
asked a verb, not an activity timestamp for the run.

## The knowledge base

`m` switches between two modes of the same frame. **Run mode** is everything
above; **KB mode** shows the knowledge base's records with the same bars,
rails, plain and exact levels, command rows and help. Each mode keeps its own view,
selection, focus and scroll, so `m` twice returns to the same place. The tab row
ends in the switch: `m kb` in run mode, `m runs` in KB mode (the first unit a
short row drops). `--view kb:why|kb:status|kb:tracker|kb:impact|kb:stale`
starts in KB mode on the project of `--run`, else of the current run, else the
first project.

The left pane lists the project records of `brief --all` (heading, then phase
and name; `d` adds the id and position), then **all records**, whose `why` and
`tracker` lists come from `find --kind` without a project.

| View | Key | JSON source | List |
|---|---|---|---|
| why | `1` | `writfold kb find --kind decision --project P --json`, plus any decision `writfold kb status project:P --json` names that the listing does not | the project's decisions in ID order: title, state glyph, marks, status word |
| status | `2` | `writfold kb brief P --json` (`brief --all` for all records) | the answer as text, scrolled; the inspector shows the project record |
| tracker | `3` | `writfold kb impact project:P --direction in --depth 2 --json` | tickets, investigations and signatures within two links, under the record their edge came from (`┼ Harbor ledger store`; the id at the exact level) |
| impact | `4` | `writfold kb impact ID --direction D --depth 2 --json` | the walk as a tree: root, depth-1 records in verb order, each followed by its depth-2 records on a dim bar |
| stale | `5` | `writfold kb stale --cached --json` (the whole KB) | stale citations, assets past their TTL and regressions, under the file that holds them |

The views read structured fields only. The tree follows each `impact` row's
`edge` key (its `from`, `to` and `depth`, see [JSON](json.md#keys-only-some-verbs-add));
a row without an edge, or whose parent is not in the answer, is drawn flat
under a `depth N` header, so nothing is dropped. A tree row's label is the
`title` key when the record has one, else its id. The dim word is the
relation, read from the row's side (`about it`, `in project`, `replaced by
it`; the exact level prints the edge's word, `concerns`); in the tracker it is
the record's own status. The last line is the walk's trailer in words:
`19 records reached · not followed: 1 disputed link, 22 inferred links`, in
the warning tone when the verb's coverage is partial; a caption names the
marks the list carries (`! cites a source that has moved`). Mention
edges are not walked; the count says how many were skipped.

### Entries, glyphs and marks

A row is rail, state glyph, label, a two-cell mark column and one dim word.
The glyph is a lookup on the exact status word, and the word is always shown:

| Glyph | ASCII | Words |
|---|---|---|
| `●` settled | `[x]` | accepted, applied, verified, done, n/a, concluded, fixed, active, closed |
| `○` open | `[ ]` | draft, proposed, landing, deferred, todo, in-progress, blocked, investigating, open, monitoring, suspected, known-error, fix-in-progress, regressed |
| `–` set aside | `[-]` | rejected, superseded, retired, abandoned, deprecated |
| `·` none | `[.]` | no status, or any other word (a project phase, `sunsetting`, `confirmed`) |

Marks say that a source moved, from flags only: `!` for `stale`, `changed`,
`gone`, `expired`, `past-ttl`, `regression`, `marked`, `due` or `moved…`; `?` for
`unverifiable` or `unchecked`. The flags are the entry's own plus those of every
`stale --cached` row about the same file (the row's `path`), so a decision whose
cited lines moved carries `!` in `why`. `n`/`N` select the next or previous
marked entry. `why` prints verdicts `kb stale` cached; `stale --cached` judges
Git-backed citations when asked, so the two can differ until someone runs
`writfold kb stale`. The KB tab carries a badge only on `stale`: `9 moved` and
`N unchecked` (`!N` and `?N` at the exact level) from that verb's own counts.
In the plain stale list every row is there because something moved, so the
mark column goes and the word is the reason: `moved`, `changed`, `gone`,
`expired`, `overdue` (past its TTL), `came back` (a regression); a cited
source is named by its place (`src/batch.rs:2-4`). A verdict read from a [check receipt](checkers.md#sharing-verdicts-check-receipts)
(built in v0.5.0-dev.9) sets no mark of its own: its row carries the extra flag
`recorded`, which the stale inspector shows beside the verdict.

### The inspector

The inspector always describes the selected entry: `status ID`, `why ID` for a
decision and `timeline ID` (for a code node, `uses ID`). Plain, top down: the
title and status; what it is (`decision · decided by ops-review on
2026-03-09`, `ticket HBR-101`, `a citation of code`); for a decision `Why` and
each line whole, then `Sources · 1 of 2 has moved` with only the sources that
moved, by place (`! src/batch.rs:2-4 in demo-service`, `moved to 4-6`); for
other records their fields as `key   value` (the status is the header's); `See
also`, `Linked from` (with the relation from the linking record's side,
`replaced by it`), `Cited by`, `Runs that name it` (by title), and the verbs'
notes when they read as words, else `N notes in exact wording (d)`. `d` shows
the id, every position, fresh citations, outcome lines, inferred mentions,
checkpoint text under `runs`, all flags as words and each verb's coverage. A
draft decision's `why` says `answer unknown · no-accepted-decision` and quotes
the verb's boundary note. `why` on a superseded decision answers with its
successor, and the inspector says so.

### Following between runs and records

`f` in run mode opens KB mode on the first record the run's checkpoints name
(the first `timeline RUN` row that is not a run, checkpoint or plan event): in
`why` when it is listed there, otherwise `impact` rooted on it. A run that names
none opens KB mode with `this run's checkpoints name no record`. `f` in KB mode
returns to the first listed run whose checkpoint names the selected record
(`timeline ID`'s `checkpoint` rows carry the run's id), on its Timeline; on a
`run` entry, its Progress. When no listed run names it (a filter hides it), the
footer says `no listed run names this record` and nothing changes. Runs and
records meet only through checkpoint text; the frame says `named`, never `cites`.

`a` shows the impact of the selected record (in `impact` it re-roots; on a
`stale` code citation it walks the cited file); `v` cycles the direction
`in` → `out` → `both`, and the verb is asked again.

The plain KB frame (136×38, list focused, the third decision selected, blank
pane rows trimmed):

```text
 Harbor status   build   knowledge base                                                                                  project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                  m runs
╭ projects (2) ──────╮╭ why (5) ──────────────────────────────────────────────╮╭ decision 3 of 5 ──────────────────────────────────────╮
│> Harbor status     ││ ┬ decisions                                           ││Batch ledger writes in the API   accepted              │
│  build  harbor     ││ – One writer process for the ledger    superseded     ││decision · decided by ops-review on 2026-03-09         │
│  Lantern console   ││ – Drop client retries on flush         rejected       ││                                                       │
│  lantern           ││ ● Batch ledger writes in the API    !  accepted       ││Why                                                    │
│  all records       ││ ○ Shard the ledger by tenant           draft          ││  Group ledger writes into batches of up to 500 rows   │
│  by kind           ││ ○ Cap replay batches at 500 rows    !  draft          ││  per transaction.                                     │
│                    ││ ┴                                                     ││  Batching keeps the flush under the store's fsync     │
│                    ││                                                       ││  budget.                                              │
│                    ││                                                       ││Sources · 1 of 2 has moved                             │
│                    ││                                                       ││  ! src/batch.rs:2-4 in demo-service                   │
│                    ││                                                       ││    moved to 4-6                                       │
│                    ││                                                       ││See also                                               │
│                    ││                                                       ││  ● Flush stalls on the ledger store   concluded       │
│                    ││                                                       ││  ● Ledger flush stalls on slow disk   done            │
│                    ││                                                       ││Linked from                                            │
│                    ││                                                       ││  – One writer process for the ledger   replaced by it │
│                    ││                                                       ││Runs that name it                                      │
│                    ││                                                       ││  Add --plan to kb runs                                │
│                    ││harbor's decisions   ! cites a source that has moved   ││d exact wording and file   f run   a impact            │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                         j/k record   l inspect   a impact   ? help   q quit
```

The impact pane after `a` on `harbor-db` (direction `in`), same size, blank rows trimmed:

```text
╭ impact ───────────────────────────────────────────────╮
│what points at Harbor ledger store                     │
│                                                       │
│ ┬ Harbor ledger store                                 │
│ ○ Slow ledger flush at month end         about it     │
│ │ · Harbor status                     !  cites it     │
│ ○ Replay too slow                        about it     │
│ │ ○ Cap replay batches at 500 rows    !  refers to it │
│ ● Ledger flush stalls on slow disk       about it     │
│ ● Harbor billing API                  !  depends on it│
│ …                                                     │
│14 records reached · not followed: 1 disputed link, 6  │
│inferred links   ! cites a source that has moved       │
╰───────────────────────────────────────────────────────╯
```

The asset `harbor-api` and its dossier share the title "Harbor billing API";
`d` shows each row's position, and the inspector its id.

Committed buffers:
[why 136×38](../../crates/writfold-tui/tests/golden/t5-why-136x38.txt),
[why 80×24](../../crates/writfold-tui/tests/golden/t5-why-80x24.txt),
[why exact](../../crates/writfold-tui/tests/golden/t5-why-detail-136x38.txt),
[status](../../crates/writfold-tui/tests/golden/t6-kb-status-136x38.txt),
[stale](../../crates/writfold-tui/tests/golden/t6-kb-stale-136x38.txt),
[a citation 80×24](../../crates/writfold-tui/tests/golden/t6-kb-stale-inspector-80x24.txt),
[impact](../../crates/writfold-tui/tests/golden/t5-impact-136x38.txt),
[tracker 100×30](../../crates/writfold-tui/tests/golden/t5-tracker-100x30.txt) and the plain
[`--once --view kb:why`](../../crates/writfold-tui/tests/golden/t5-once-kb-why.txt).

KB footer hints: list `j/k record   l inspect   a impact`; inspector
`j/k scroll   h back   d detail` (its hint line: `d exact wording and file   f run
a impact`); projects `j/k project   l records`; status
`j/k scroll   Tab view`. A KB with no project shows only `all records`; an empty
list says so in one dim line (`harbor names no decisions`, `nothing tracked
within 2 links of harbor`, `nothing stale; N checked`) followed by the verb's
notes. A refusal stays in its pane as `refused (exit N): …`.

KB answers are cached per project and view, the inspector's per record id, and
`stale --cached` once; the two-second poll drops them all with the run answers
and keeps the selection by id. A cited code repository moving is not polled:
press `r`.

## Controls

| Key | Action |
|---|---|
| `j` / `k`, Up / Down | Select a run or map node; scroll text or a focused inspector |
| `[` / `]` | Previous/next run regardless of focus; in KB mode, previous/next project |
| `h` / `l`, Left / Right | Move focus left/right: runs → map → inspector |
| Enter / Esc | Cycle focus forward / step left; from the wide map, Esc restores the side stack |
| Tab / Shift-Tab | Next/previous view |
| PgDn / PgUp | Page map nodes, or scroll text by the pane's height less one |
| `0` | Show the full-frame runs list in a narrow layout; paging does not change selection |
| `.` | Select the recorded current step |
| `n` / `N` | Next/previous flagged step, wrapping |
| `d` | Plain words / exact wording for the whole frame: the verbs' own text, ids, receipts, covered items, splice text, chips, full runs entries and stamps; never written, so a fresh start is plain |
| `c` | Commands to copy: ids, files and the suggested commands of what is on screen, as borderless rows at the bottom; `j`/`k` scroll, any other key closes; `watch` runs none of them |
| `x` / `X` | In Progress: map/text toggle / expand or collapse the text list |
| `o` | Toggle `kb runs --open` |
| `P` | Toggle the selected project's `kb runs --project`; without a project, show a notice and change nothing |
| `r` | Drop cached answers and ask again |
| `T` | Cycle the eight themes in truecolor (the footer names the new theme for one key); otherwise explain how to enable it |
| `m` | Run mode ↔ KB mode; each keeps its own view, selection, focus and scroll |
| `f` | Follow: from a run to the first record its checkpoints name; from a record to a listed run whose checkpoint names it |
| `a` / `v` | KB mode: impact of the selected record / cycle its direction in → out → both |
| `1`–`5` | KB mode: why, status, tracker, impact, stale; a run view's letter (`p g t i u s b e`) leaves KB mode for that view |
| `?` | Help: the marks, this frame's full stamps, the Words (each plain phrase beside the verbs' exact word), keys and verbs (per mode); any key except `q` closes it |
| `q`, Ctrl-C | Quit and restore the terminal |

No key writes, and no key runs anything. `c` lists the verb's `confirm` or
`accept` command; copy it to another terminal when you intend a write. Local-only inboxes say they are local;
when a tracked proposal is present, the caption instead describes pending
records. `!tracked` means the work-tree proposal store, not acceptance or proof
that the file has been committed.

## Layout and refresh

| Class | Minimum size | Layout |
|---|---|---|
| Wide | 140×32 | Runs, map and side stack; map/inspector focus replaces the stack with the inspector |
| Medium | 90×16, when not wide | With a map: runs 22 columns, map and inspector split the rest (map at least 32) |
| Narrow | 60×12, when not medium | Map or inspector alone; `l`/Enter opens the inspector, `h`/Esc returns; `0` reveals runs |
| Too small | Otherwise | `writfold watch needs 60x12 …`; controls still work |

Without a plan, or in another view, the earlier layouts remain. The wide stack
contains Gates/Inbox/Timeline, plus Guards from height 38; its panes are not
focusable. At the plain level each holds its count line and one clipped line
per row (`! lint   out of date`, `stop-reflect   would remind`). Text and evidence wrap with hanging indentation; maps and lists
truncate with `…`. The map reserves the recorded facts below the rail and drops
the caption first, then the oldest record line, when space is short.

Every layout has the same three bars, and every pane starts on the third row:

- **Header**: the run title, its status chip, a `current run` chip on the
  current run and a project chip; `no repository: gates not judged` (or
  `repository unreadable`) only when no subject is judged; `run 1 of 3` at the
  right. The exact level shows `current`, `subject NAME` and the run id
  (`run N/M` in a narrow layout). When the row is short, the subject warning
  then the project drop.
- **Tabs**: eight views; the active one is a filled chip. A tab carries a badge
  only for a non-zero count from its own verb: Gates `N not met` (gates not
  satisfied, attested or waived; the bad tone when one is out of date, failed
  or invalid), Inbox `N pending`, Guards `N would block`, `N would ask`. A
  badge shortens to its number, from the right, before the mode unit or a tab
  drops. The exact level shows `!N`, `×N`, `○N`, `?N`, one inbox number and
  `deny N`, `ask N`. Narrow layouts, or a row too short for all eight, show the
  active tab, the tabs with badges and `i/8`.
- **Footer**: a `read-only` chip, then only what is not the normal case: `KB
  has N uncommitted changes` and `index STATE` (warning tone) when not fresh;
  at most three hints for the focused pane, then `? help   q quit`, on the
  right. The exact level shows `kb STAMP`, `index STATE`, `asked HH:MM:SSZ` and
  the subject stamp, dropping whole from the end when they do not fit. A notice
  (theme cycling, a refused filter) replaces the left group for one key.

A chip is its text plus one cell each side: padding on a background in
truecolor, brackets (`[open]`, `[1 progress]`, `[read-only]`) otherwise, so
widths never change with depth. The full subject stamp, theme and colour depth
are in help ("This frame").

Every two seconds the viewer compares the KB pin and a stat-only stamp of the
current-run pointer, drafts and **both** proposal stores. Tracked proposals need
this stamp too: their dot directory is not an index input. Every fifth tick
(10 seconds) it observes the selected subject's HEAD and dirty-path metadata.
Selecting a different run observes its subject immediately. Changed stamps drop
cached answers while retaining selection; Guards is cached per stamp, not per run.
The Guards view shows each rule's flags as `guard status` gives them, so an audit rule reads `!audit` and a rule passing on a waiver `!waived`, and its count line adds `audit N` when an audit rule would refuse (v0.5.0-dev.10).

Dirty subject files are not hashed by the poll. Same-size edits that preserve
mtime, time-only expiry and other changes outside these stamps can require `r`.
The view is not a freshness guarantee between asks.

`watch` never starts a checker, gate command, hook or Weft process. Git reads are
permitted. `stale --cached` checks Git-backed citations normally and reads cached
external verdicts without writing `cite_checks`; an unchecked token remains
`not-checked`. Run `writfold kb stale` explicitly to ask external checkers. Opening
an ordinary read session may refresh the rebuildable index.

## Themes and colour

Themes cycle in this order: **clay** (the default), **clay-light**, **hone**,
**slate**, **carbide**, **vellum**, **paper**, **paper-dark**. Clay is neutral
charcoal with a terracotta accent; clay-light is its ivory counterpart. Clay-light,
vellum and paper are light; the others are dark. Panes have rounded corners
except under `--color none`. The selected name comes from `--theme`, then
`WRITFOLD_THEME`, then the existing user configuration, then clay. The configuration is `$XDG_CONFIG_HOME/writfold/config.toml`, otherwise
`~/.config/writfold/config.toml`; its quoted line is:

```text
theme = "paper"
```

An unknown highest-precedence name keeps clay and warns with the source and all
eight choices. A lower-precedence name that was overridden produces no warning.
The interactive warning occupies the footer until the next key; plain output
sends it to stderr. Cycling never writes the configuration.

| Colour mode | Behaviour |
|---|---|
| `auto` | `NO_COLOR` set to any value, or `TERM=dumb`: none; otherwise `COLORTERM=truecolor` or `24bit`: truecolor; otherwise ANSI |
| `truecolor` | Force palette foregrounds and surfaces |
| `ansi` | Force named terminal colours, with no painted background |
| `none` | No foreground/background colours; selected entries reverse and key headings retain emphasis |

Explicit modes override environment detection. There is no 256-colour palette
quantisation. Most Unicode marks (`● ○ ◆ ■ □ ▶ × – … ·` and box drawing) are
East Asian Ambiguous width: a terminal set to draw them wide will misalign the
columns. `--color none` or `TERM=dumb` selects the ASCII marks. Contrast tests cover every palette's inks, surface ladder, accent
and selected-entry text.

## Plain frames and scripts

`--once --view kb:NAME` prints `== projects ==` (the project rows, `>` on the
selected one, then `all records`), the view's verb answers as their text under
`== kb:NAME PROJECT ==`, and the footer line. No rail, glyph or mark enters it.

These frames are the exact wording: the plain words of the terminal frame
never enter them, so a script or an agent reads the verbs' own text (there is
no `d` to press in a pipe).

`--once`, or a non-terminal stdout, prints one expanded, unwrapped plain frame:
Runs, Progress, Gates, Inbox, Guards, Timeline, then the footer. It contains no
view clock or terminal colour/style sequences, regardless of theme or depth.
`--view NAME` prints Runs, only that section, and the footer. `--view resume`
retains the gate-count heading of the earlier plain resume frame.
Progress gains only the verb's `splice:`/`under:` continuations, and Timeline its
plan event rows. No map rails or new graphical glyphs enter plain frames.

`watch --json` is not a second API. Ask the JSON verb listed beside the pane above.
Not built: write keys, text search over runs or records (`find` is not reachable
from a key), cross-KB views, mouse interaction, focusable side-stack panes,
user-defined themes, persistence of the theme choice; in KB mode, walking
mention edges, a depth other than two, history at a commit, record bodies and
dossier sections (open the path `d` shows in the inspector), and a jump from a
plan step or gate to the record it cites (their rows name receipts, not
records); a `--level` flag to start at the exact level, a layout of its own for
Stale, Brief and Resume beyond the row form, and a `change · 2nd time` header
for a template spliced twice (it shows the template's name).
