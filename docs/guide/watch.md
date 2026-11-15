# Watching a run

> **Status:** built (T4). Implementation, tests and deviations: [design §36](../writfold/kb-v1-design.md#36-t4-as-built); the plan map is recorded in [§35](../writfold/kb-v1-design.md#35-t3-as-built) and the earlier text view in [§34](../writfold/kb-v1-design.md#34-t2-as-built).

`writfold watch` is a read-only view of the KB verbs. It shows recorded steps,
gate standings, checklist items, pending drafts and proposals, guard rules and
checkpoint history. It does not schedule work or claim that an agent is working.

```sh
writfold watch
writfold watch --kb /path/to/kb --repo /path/to/repository
writfold watch --run RUN_ID --view gates --once
writfold watch --theme paper --color truecolor
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

Gate chips follow each gate continuation's `status`: `●` satisfied, `◆` attested
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
counts; only stale, moved, invalid and disorder counts take a warning tone.
Below the rail are the newest plan event from Timeline (one clipped line; whole,
with the run list's last checkpoint, at the detailed level) and a one-line
caption for the marker. Without a subject repository the gate counts stay
missing, the line says `gates not judged` and the verb's note is shown; the
position still comes from the row the verb marks current.
Done stays done when its evidence becomes stale; the marker never moves back.

Selection starts at the recorded current step, survives refresh by ID, and falls
back to the current step or first row if its ID disappears. The rail keeps it
visible with two rows of context where space permits. Group headers name the
splice; inside a group a row shows its id after the group's name (`check` under
`findings#1`), and the inspector shows the full id. Chips are aligned, IDs
truncate from the left, and text that does not fit, headers included, ends in
`…`. Rail bars are always dim; only the state glyph takes a state colour. The
row end repeats only disorder and invalid flags; the state glyph already says
stale or skipped. The legend is in help (`?`, "Marks").

The inspector leads with the step's short id and a status chip, then the full
id. It shows reasons, what the step owes, each gate as the verb words it with
its segment strip, the checklist items still open or stale, completion or skip
records and the read/next commands. Checklist segments are one per required
item, in source order: `■` supported, `◆` not-applicable/attested, `!` stale,
`□` open, then `(+N optional)`; their count never scales with width. `d` adds
splice and parent, the gate chips, every item including covered ones, every
receipt path (wrapped with a hanging indent) and the run's last plan event and
checkpoint. `g` opens full gate receipts. No gate-status rows are joined into it.

### Calm and detailed

The frame starts **calm**. `d` toggles the **detailed** level for the whole
frame and back; the level is never written, so a fresh start is calm. Calm
removes lines and clips at word boundaries; it never rewrites a verb's wording,
and everything it omits is one key away (`d`, `?`, `g`, `x` or a view).

- A rail row shows its verb text only when it is the recorded current step or
  the selected row; every other row is rail, glyph, short id and chips.
- Group headers show the splice name; the hash and template source come back
  with `d` and are in the inspector's `splice:` line.
- Receipt paths (`@path:line`) appear only at the detailed level, in `g` and in `x`.
- Runs entries are two lines (title; state when not open, project and the plan
  position up to its first `key=value` token); `d` restores the five-line entry
  with id, fields, plan counts and last checkpoint.

The calm wide frame (136×38, map focused):

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
  1 progress    2 gates !2 ○2    3 timeline    4 inbox 3    5 guards deny 1    6 stale    7 brief    8 resume
╭ runs (3) ──────────╮╭ plan ─────────────────────────────────────────────────╮╭ step 7/11 ────────────────────────────────────────────╮
│> Add --plan to kb …││step 7 of 11    5 done  1 skipped  1 stale             ││check  current                                         │
│  harbor  step 7/11 ││                                                       ││change#1/review/review#1/findings/findings#1/check     │
│  Review the lanter…││ ┬ change#1                                            ││                                                       │
│  lantern           ││ ✓ research    ●                                       ││owes: review-list [stale], signoff [open]              │
│  Retire the legacy…││ ! implement   ●!                                      ││                                                       │
│  closed  harbor  c…││ – benchmarks                                          ││gate review-list [stale] checklist 2/4 required covered│
│                    ││ ├─╮ review#1                                          ││  ■ ! □ ◆ (+1 optional)                                │
│                    ││ │ ├─╮ notes#1                                         ││  ! tests [stale] 1 reference by agent; cited evidence │
│                    ││ │ │ ✓ read    ●                                       ││  moved: gate:lint was satisfied, now stale            │
│                    ││ │ ├─╯                                                 ││  □ docs [open] The guide names every new flag         │
│                    ││ │ ✓ read-diff ●                                       ││  □ style [open] (optional) Names follow the module    │
│                    ││ │ ├─╮ findings#1                                      ││gate signoff [open] attested                           │
│                    ││ │ │ ✓ prepare ●                                       ││                                                       │
│                    ││ │ │ ▶ check   !○ owes: review-list [stale], signoff … ││read: instructions for signoff                         │
│                    ││ │ ├─╯                                                 ││next: writfold kb gate cover review-list tests         │
│                    ││ │ ○ findings  ··                                      ││supported REF… --by WHO                                │
│                    ││ ├─╯                                                   ││                                                       │
│                    ││ ○ review      ··                                      ││                                                       │
│                    ││ ○ deliver     ·                                       ││                                                       │
│                    ││ ○ reflect                                             ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││last plan event  2026-11-10T23:59:50Z done change#1/re…││                                                       │
│                    ││▶ next step to pick up, as recorded                    ││d receipts and covered items   g gates   x text        │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                j/k step   l inspect   d detail   ? help   q quit
```

(Blank pane rows are trimmed here.) The same map pane after `d`:

```text
│step 7 of 11    5 done  1 skipped  1 stale             │
│owes 2 of 2 gates                                      │
│                                                       │
│ ┬ change#1 @83c2cc9efde7 from .writfold/plans/change.…│
│ ✓ change#1/research      ●  recorded 2026-11-10T23:4… │
│ ! change#1/implement     ●! recorded 2026-11-10T23:5… │
│ – change#1/benchmarks       no hot path touched (by … │
│ ├─╮ review#1 @932e0ad862a8 from .writfold/plans/revie…│
│ …                                                     │
│last plan event: 2026-11-10T23:59:50Z done             │
│change#1/review/review#1/findings/findings#1/prepare   │
│last checkpoint: 2026-11-10T14:10:00Z Review started.  │
│▶ next step to pick up, as recorded                    │
```

At 80×24 the map has the frame to itself; `l` swaps it for the inspector, whose
footer hint becomes `h back`. Committed buffers:
[calm 136×38](../../crates/writfold-tui/tests/golden/t4-calm-136x38.txt),
[calm 80×24](../../crates/writfold-tui/tests/golden/t4-calm-80x24.txt),
[detailed 136×38](../../crates/writfold-tui/tests/golden/t4-detail-136x38.txt).

`x` switches to the earlier text list. There, a clean done step collapses to its
heading and completion time, and a later step to its heading; flagged, current
and skipped steps retain detail. `X` expands or collapses all text steps.
The text strip remains one cell per recorded step, never a completion bar.
Gates, Resume, the inspector and `--once` retain evidence positions.

The viewer adds no progress percentage, ETA, elapsed/relative time, spinner or
run-liveness claim. Authored text and verb states are not censored or rewritten.
Recorded times remain recorded times; `asked HH:MM:SSZ` is when the view last
asked a verb, not an activity timestamp for the run.

## Controls

| Key | Action |
|---|---|
| `j` / `k`, Up / Down | Select a run or map node; scroll text or a focused inspector |
| `[` / `]` | Previous/next run regardless of focus |
| `h` / `l`, Left / Right | Move focus left/right: runs → map → inspector |
| Enter / Esc | Cycle focus forward / step left; from the wide map, Esc restores the side stack |
| Tab / Shift-Tab | Next/previous view |
| PgDn / PgUp | Page map nodes, or scroll text by the pane's height less one |
| `0` | Show the full-frame runs list in a narrow layout; paging does not change selection |
| `.` | Select the recorded current step |
| `n` / `N` | Next/previous flagged step, wrapping |
| `d` | Calm / detailed frame: receipts, covered items, splice text, full runs entries and stamps; never written |
| `x` / `X` | In Progress: map/text toggle / expand or collapse the text list |
| `o` | Toggle `kb runs --open` |
| `P` | Toggle the selected project's `kb runs --project`; without a project, show a notice and change nothing |
| `r` | Drop cached answers and ask again |
| `T` | Cycle the eight themes in truecolor (the footer names the new theme for one key); otherwise explain how to enable it |
| `?` | Help: the marks, this frame's full stamps, keys and verbs; any key except `q` closes it |
| `q`, Ctrl-C | Quit and restore the terminal |

No key writes. `c` is unbound. Copy the verb's `confirm:` or `accept:` command to
another terminal when you intend a write. Local-only inboxes say they are local;
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
focusable. Text and evidence wrap with hanging indentation; maps and lists
truncate with `…`. The map reserves the recorded facts below the rail and drops
the caption first, then the oldest record line, when space is short.

Every layout has the same three bars, and every pane starts on the third row:

- **Header**: the run title, its status chip, a `current` chip on the current
  run, a project chip and `subject NAME`; the run id is right-aligned (`run N/M`
  in a narrow layout). When the row is short, subject then project drop.
- **Tabs**: eight views; the active one is a filled chip. A tab carries a badge
  only for a non-zero count from its own verb: Gates `!N` stale, `×N` failed,
  `○N` open, `?N` unknown; Inbox one number (drafts plus proposals; the Inbox
  view's first line has the breakdown); Guards `deny N`, `ask N`. Narrow
  layouts, or a row too short for all eight, show the active tab, the tabs with
  badges and `i/8`.
- **Footer**: a `read-only` chip, `kb STAMP`, `index STATE` (warning tone when
  not fresh) and `asked HH:MM:SSZ` on the left; at most three hints for the
  focused pane, then `? help   q quit`, on the right. Left and right are laid
  out, never overlaid: when they do not fit, asked, index, then kb drop whole.
  The detailed level adds the subject stamp when it fits. A notice (theme
  cycling, a refused filter) replaces the left group for one key.

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

`--once`, or a non-terminal stdout, prints one expanded, unwrapped plain frame:
Runs, Progress, Gates, Inbox, Guards, Timeline, then the footer. It contains no
view clock or terminal colour/style sequences, regardless of theme or depth.
`--view NAME` prints Runs, only that section, and the footer. `--view resume`
retains the gate-count heading of the earlier plain resume frame.
Progress gains only the verb's `splice:`/`under:` continuations, and Timeline its
plan event rows. No map rails or new graphical glyphs enter plain frames.

`watch --json` is not a second API. Ask the JSON verb listed beside the pane above.
Not built: write keys, text search over runs, cross-KB views, mouse interaction,
focusable side-stack panes, user-defined themes, or persistence of the theme choice.
