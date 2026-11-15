# `writfold watch` T4: calm defaults and progressive disclosure

Reviewed: main at 22acc4b (v0.5.0-dev.6), `crates/writfold-tui/src`, the captured frames
the v0.5.0-dev.6 homepage capture (136x38) and the how-it-works capture (160x40), and live runs in tmux at 80x24,
100x30, 136x38 and 160x40 against the fixture KB. Read-only review; nothing in the repository
was changed. Line numbers below refer to the 136x38 v0.5.0-dev.6 capture (the homepage frame).

Owner signal: the map is loved; the frame carries too many cryptic tokens at once. The plan
keeps the rail, glyphs and chips exactly as they are and removes text around them. Nothing is
deleted from the product: every fact moves one keypress away (`d`, `?`, or a view key).

## 1. Critique: what is on screen by default

Counted on `site-home.txt`: 38 rows, about 2,700 non-blank characters (borders excluded), 10 ISO timestamps or
their fragments, 5 wrapped `@path` receipts, 5 `@hash` tokens, the run id in full or in part 10 times.

| Pane | Text today (line) | Verdict |
|---|---|---|
| Header | `run-20261101T090000Z-a1b2c3 [open] !current  Add --plan to kb runs  project=harbor  subject=kb` (1): id first, title in the middle, `key=value` tokens | Keep, reorder: title first, state as chips, id last and muted |
| Tab bar | `[1 progress] 2 gates … 8 resume` (2) | Keep; active tab becomes a chip; counts attach to the tab they belong to |
| Summary | `gates: open=2 stale=2 of 6 \| inbox: drafts=1 proposals=2 \| guards: deny=1 ask=0 of 2` (3), Medium class only | Drop the row; non-zero counts become tab badges; full counts stay the first line of views 2, 4, 5 |
| Runs | 5 lines per run (5–18): `[open] run-2026110` (all three ids clip to the same prefix, so the id line distinguishes nothing at 22 columns), title, `project=`, `plan:`, `last:` | Default 2 lines: title; project and plan position. Id, checkpoint and full plan counts behind `d` |
| Map title | `progress: map run-20261101T090000Z-a1b2c3` (4): third copy of the run id | Becomes `plan`; the id lives once, in the header |
| Map prelude | `plan 5 done, 1 skipped of 11 steps; current step 7/11; owes 2 of 2 gates stale=1` (5–6): two wrapped lines, all bold red because one count is stale | One line, neutral; only the stale count takes the error tone. `owes 2 of 2 gates` is redundant with the `▶` row's chips; behind `d` |
| Group headers | `change#1 @83c2cc9efde7 from .writfold/plans/change.t` (7, 11, 12, 16): hash and source on every header, hard-cut at the border with no `…` | Default: name only. Hash and source behind `d` and already in the inspector's `splice:` line with its receipt |
| Rail rows, done | `recorded 2026-11-10T23:4…` (8, 9, 13, 15, 17): a clipped timestamp that cannot be read anyway; `✓` already says done | Drop from default; the inspector has `done: recorded …` with the receipt |
| Rail rows, later | `gates: review-list, sign…` (20, 22, 23), `no gates` (24) | Drop from default; chips (`··`, `·`, none) carry it; names in the inspector |
| Rail row, skipped | `no hot path touched (by …` (10) | Drop from default; `–` says skipped; reason in the inspector |
| Rail row, `▶` | `owes: review-list [stale…` (18) | Keep: the one row whose text is the point |
| Rail ids | `change#1/research`, `findings#1/prepare`: group name repeated on every row under its own header | Default: the part after the group name (`research`, `prepare`); full id in the inspector |
| Records | `recorded position …`, `last plan event …`, `last checkpoint …` (28–32, 5 rows) | Position is redundant with the prelude and `▶`: move its definition to `?`. Keep one clipped `last plan event` line; checkpoint behind `d` |
| Legend | two entries wrapped to 4 rows (33–36), orphans `unknown` and `not a liveness claim)` | Move to `?`. Keep a one-line caption for `▶` only |
| Inspector title | `step 7/11 (Esc: back)` | `step 7/11`; the back hint moves to the footer when the inspector has focus |
| Inspector | 33 rows, 10 of them the five `@runs/…json:1` receipts, each wrapped mid-token (8–9, 14–15, 18–19, 23–24, 28–29); `splice:` + `under:` (6–10) restate the id; `!stale`/`!open` suffixes repeat the bracketed state; `2/4 required covered` appears twice (13, 16); run-level `last …` records appended to a node view | Default: id, owes, gates with the segment strip, only items still open or stale, `read`/`next`. Receipts, covered items, splice, under behind `d` |
| Footer | `kb=…+dirty:19 index=fresh  subject=kb@0ea3a7 asked 00:21:18Z theme=clay read-only  jk node  l inspect  . current  n flag  ? q` (38) | Keep read-only, kb stamp, index, asked. Subject stamp and theme move to `?` ("This frame"). Keys: three contextual hints plus `? help  q quit` |

## 2. Alignment and defects found

Terminal output is cell-exact: every pane row in all three captured frames is exactly the
terminal width, and no East Asian wide character is used. Real defects:

1. **Footer overlay cuts the left text mid-token.** `chrome::map_hints` paints the right block
   over the stamps: `subject=kb@0ea3a7 asked` (line 38), `subject=writfold@ asked` (136 live),
   `kb=0ea3a743bfef+dir asked` (100x30). The subject stamp is never fully readable in map mode.
2. **Group headers hard-cut without `…`** (`change.t`, `plans/review`, `plans/fi`) while node
   rows use `rail::clip`. `rail::build` pushes the header span unclipped.
3. **Rail bars change colour per row.** `rail::node_line` styles the whole prefix, depth bars
   included, with the row's state tone; headers and closers use the dim tone. The same vertical
   line is green, red, yellow and grey down one column (clearly visible in the website shot).
   Bars should always be dim; only the state glyph takes the tone.
4. **Inspector hint clipped at 100x30**: `full receipts: g   all steps as text:` loses its key.
5. **Whole prelude is error-toned** for one stale count; it reads as an alarm banner.
6. **Stale help text**: `frame::help` says `Theme: … else hone`; the default is clay.
7. **Ambiguous-width glyphs**: every mark except `✓` (`● ○ ◆ ■ □ ▶ × – … ·` and box drawing)
   is East Asian Ambiguous. ratatui counts one cell; a terminal configured to render ambiguous
   characters wide will drift. Not fixable in the frame and not seen in a default setup;
   the guide should say that `--color none` or `TERM=dumb` selects the ASCII marks.

Website screenshot (`site-watch.png`): measured on the capture, text columns do line up (ids at
columns 3/5/7, chips in one column, borders straight); that capture resolved the stack to a
font that has every glyph. What looks broken there is (a) item 3 above, a product behaviour;
(b) gaps in the rail because `.term-frame code` uses `line-height: 1.2`, taller than the
box-drawing glyphs; (c) the stack `ui-monospace, "SFMono-Regular", Consolas, monospace` pins no
font, so on platforms whose first match lacks `✓ ▶ ◆ ■ □` or box drawing the browser substitutes
per glyph with a different advance and columns will drift. (c) is inferred from the CSS, not
reproduced. So: browser-side except item 3. Site fix (separate repository, after this slice):
self-host one monospace face with full coverage for `.term-frame`, set `line-height` to the
face's box height (about 1.0–1.15), and recapture the calm frame.

## 3. Design

### 3.1 Disclosure scheme: one new key

| Key | Effect |
|---|---|
| `d` | Toggle detail for the whole frame: **calm** (default) and **detailed** (today's text). Not written anywhere; a fresh start is calm |
| `?` | Help overlay, now also the home of the legend ("Marks"), the full stamps ("This frame") and the long key list |
| existing | `l`/Enter inspector, `g` gates with full receipts, `x` plan as text, `.`, `n`/`N`: unchanged |

Rules for the calm level, in one sentence each:

- **Text follows `▶` and the cursor.** A rail row shows its verb text only when it is the
  recorded current step or the selected row. Every other row is rail, glyph, short id, chips.
  Flags the glyph cannot say (`!out-of-order`, `!two-plans`, `!repeated`, `!invalid`) stay on
  every row at both levels.
- **Names once.** A group header shows the splice name; rows under it show the id after that
  name (extend `rail::shown_id`: strip the leading `name/` when present, otherwise show the id
  unchanged; still presentation only, ids stay opaque).
- **Receipts on demand.** `@path:line` lines appear only in the detailed level, `g` and `x`.
- **Attention first.** The calm inspector lists checklist items whose `status` is neither
  `supported`, `not-applicable` nor `attested`; the segment strip still shows all of them.
- **No row is reformatted.** Calm removes lines and clips at token boundaries; it never
  rewrites a verb's wording. Tab badges are the verbs' own counts.

Folding completed groups is deliberately **not** in this slice: selection is a node id, so a
folded group needs a second cursor kind, and with text gone the 11-step fixture's rail is 19
rows. Revisit when a real plan exceeds the pane.

### 3.2 Header bar and tab bar

Row 1, left to right: run title (text, bold) · status chip · `current` chip (only on the
current run) · project chip · `subject NAME` (muted) · right-aligned run id (muted). When the
row is too narrow, drop from the right end of the left group: subject, project; the id
gives way to `run N/M` in the narrow class.

Row 2: eight tabs, three spaces apart. The active tab is a filled chip. A tab carries badges
only for non-zero counts from its own verb: gates `!N` stale, `×N` failed, `○N` open,
`?N` unknown; inbox `N` (drafts plus proposals; the breakdown is the first line of view 4. If
the owner prefers no derived number, print `1+2`); guards `deny N`, `ask N`. The Medium
summary row is removed, which gives the panes one more row. Narrow class: the active tab, then
only tabs that carry a badge, then `i/8` right-aligned.

| Segment | Truecolor (clay) | 16-colour | No colour / dumb |
|---|---|---|---|
| Bar background | `surface` | none | none |
| Title | `text`, bold | bold | plain |
| Status chip ` open ` | `secondary` on `raised` | `[open]` default fg | `[open]` |
| ` current ` | `accent` on `raised`, bold | `[current]` cyan bold | `[current]` |
| Project chip | `secondary` on `raised` | `[harbor]` | `[harbor]` |
| `subject kb`, run id | `muted` | dark grey | plain |
| Active tab ` 1 progress ` | `on_accent` on `accent`, bold (the only filled accent in the frame) | `[1 progress]` bold | `[1 progress]` |
| Inactive tab | digit `accent`, name `secondary` | digit bold, name default | plain |
| Badges | `error` for `!`/`×`/`deny`, `warning` for `○`/`?`/`ask`, `secondary` for inbox | red / yellow / default | plain; ASCII marks `!2 o2 x1` |

A chip is always its text plus one cell each side: a space with a background in truecolor, a
bracket otherwise. Widths are identical at every depth, so one layout serves all three.

### 3.3 Footer (modeline)

Left: ` read-only ` chip (`muted` on `raised`; `[read-only]` degraded) · `kb STAMP` · `index
STATE` (`warning` tone when not `fresh`) · `asked HH:MM:SSZ`, all `muted`. Right: at most three
contextual hints, then `? help   q quit`; key in `accent` bold (existing `Tone::Key`), label
`muted`. Left and right are laid out, not overlaid: when they do not fit, drop from the left
group's end (asked, index, kb), never cut a token.

| Focus | Hints |
|---|---|
| Runs | `j/k run   l plan` |
| Map | `j/k step   l inspect   d detail` |
| Inspector | `j/k scroll   h back   d detail` |
| Other views | `j/k scroll   Tab view` |

Moved to `?`: `. current`, `n flag`, `theme=`, the subject stamp, "no daemon". A notice (theme
cycling, filter refusal) still replaces the left group for one keypress, as today.

### 3.4 Default frame, 136x38, clay, map focused, `▶` selected

Shown as truecolor text: chips are padded with spaces and distinguished by background.

```
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
│                    ││ │ │ ✓ read    ●                                       ││    moved: gate:lint was satisfied, now stale          │
│                    ││ │ ├─╯                                                 ││  □ docs [open] The guide names every new flag         │
│                    ││ │ ✓ read-diff ●                                       ││  □ style [open] (optional) Names follow the module    │
│                    ││ │ ├─╮ findings#1                                      ││gate signoff [open] attested                           │
│                    ││ │ │ ✓ prepare ●                                       ││                                                       │
│                    ││ │ │ ▶ check   !○ owes: review-list [stale], signoff […││read: instructions for signoff                         │
│                    ││ │ ├─╯                                                 ││next: writfold kb gate cover review-list errors        │
│                    ││ │ ○ findings  ··                                      ││  supported REF… --by WHO                              │
│                    ││ ├─╯                                                   ││                                                       │
│                    ││ ○ review      ··                                      ││                                                       │
│                    ││ ○ deliver     ·                                       ││                                                       │
│                    ││ ○ reflect                                             ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││last plan event  2026-11-10T23:59:50Z done change#1/re…││                                                       │
│                    ││▶ next step to pick up, as recorded                    ││d receipts and covered items   g gates   x text        │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                j/k step   l inspect   d detail   ? help   q quit
```

Notes: row 16 is the selected row (selection surface across the pane). The `current` word in
the inspector's first line is a chip. Rail bars are dim on every row. The map keeps a blank row
under the summary only when the whole rail still fits. About 1,100 non-blank characters against
2,700 today; one timestamp in the panes instead of ten; no `@path`, no `@hash`.

Degraded bars at the same size (`--color none`; `--color ansi` adds colour only):

```
 Add --plan to kb runs  [open] [current] [harbor]  subject kb                                               run-20261101T090000Z-a1b2c3
 [1 progress]   2 gates !2 o2    3 timeline    4 inbox 3    5 guards deny 1    6 stale    7 brief    8 resume
 [read-only]  kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                j/k step   l inspect   d detail   ? help   q quit
```

### 3.5 Default frame, 80x24 (narrow), map

```
 Add --plan to kb runs   open   current                                 run 1/3
  1 progress    2 gates !2 ○2    4 inbox 3    5 guards deny 1               1/8
╭ plan ────────────────────────────────────────────────────────────────────────╮
│step 7 of 11    5 done  1 skipped  1 stale                                    │
│ ┬ change#1                                                                   │
│ ✓ research    ●                                                              │
│ ! implement   ●!                                                             │
│ – benchmarks                                                                 │
│ ├─╮ review#1                                                                 │
│ │ ├─╮ notes#1                                                                │
│ │ │ ✓ read    ●                                                              │
│ │ ├─╯                                                                        │
│ │ ✓ read-diff ●                                                              │
│ │ ├─╮ findings#1                                                             │
│ │ │ ✓ prepare ●                                                              │
│ │ │ ▶ check   !○ owes: review-list [stale], signoff [open]                   │
│ │ ├─╯                                                                        │
│ │ ○ findings  ··                                                             │
│ ├─╯                                                                          │
│ ○ review      ··                                                             │
│last plan event  2026-11-10T23:59:50Z done change#1/review/review#1/findings/…│
│▶ next step to pick up, as recorded                                           │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   l inspect   d detail   ? help   q quit
```

The rail window scrolls as today (two rows of context). `l` swaps the pane for the inspector,
whose footer hint becomes `h back`. At 100x30 the three-pane Medium layout holds: the map's
inner width is 37, enough for the deepest row plus chips; the `▶` text clips with `…`.
At 160x40 the wide layout is unchanged apart from the bars and the calm rows; the side stack
keeps its current compact content.

### 3.6 Detailed level (`d`)

The panes return to today's content, with the fixes from section 2. Map pane at 136x38:

```
│step 7 of 11    5 done  1 skipped  1 stale             │
│owes 2 of 2 gates                                      │
│ ┬ change#1 @83c2cc9efde7 from .writfold/plans/change.…│
│ ✓ change#1/research      ●  recorded 2026-11-10T23:4… │
│ ! change#1/implement     ●! recorded 2026-11-10T23:5… │
│ – change#1/benchmarks       no hot path touched (by … │
│ ├─╮ review#1 @932e0ad862a8 from .writfold/plans/revie…│
│   (rows as in v0.5.0-dev.6)                           │
│last plan event: 2026-11-10T23:59:50Z done             │
│change#1/review/review#1/findings/findings#1/prepare   │
│last checkpoint: 2026-11-10T14:10:00Z Review started.  │
│▶ next step to pick up, as recorded                    │
```

Runs entries return to five lines; the inspector returns to `splice:`, `under:`, every item and
every `@path:line`, and its last row reads `d less   g gates   x text`. The footer adds
`subject kb@0ea3a7…` when it fits whole. The legend does not come back; it is in `?`.

### 3.7 `?` overlay additions (top of the existing help)

```
Marks   steps  ✓ done  ! done, now stale  – skipped  ▶ next step to pick up  ○ later  × invalid
        gates  ● satisfied  ◆ attested/waived  ○ open  ! stale  × failed  ? unknown  · later, not judged
        items  ■ supported  ◆ not applicable/attested  □ open  ! stale
        ▶ is the recorded position: the first step neither done nor skipped. It is not a liveness claim.
This frame   kb 0ea3a743bfef+dirty:19   index fresh   subject kb@0ea3a743bfef+dirty:28
             asked 00:21:18Z   theme clay   colour truecolor   read-only; no daemon
```

followed by the existing key and verb lists (with `d` added and the theme default corrected).

## 4. Implementation plan (one session)

Budgets: `rail.rs` is 260 lines and `frame.rs` 246, so new code goes into new modules; keep
every function under 80 lines; no `unwrap`/`expect`; no new crates.

| # | Work | Files |
|---|---|---|
| 1 | `Detail { Calm, Full }` on `App` (default `Calm`); `d` toggles it, resets `inspect_scroll`, keeps selection | `app.rs`, `keys.rs` |
| 2 | Bars as segments: new `bars.rs` with `Segment { text, role }`, `Role { Title, Chip, ChipAccent, TabActive, Tab, Badge(Tone), Muted, Key }`, pure builders `title(app, width)`, `tabs(app, width, narrow)`, `footer(app, width)` that fit left/right groups by dropping whole segments. `style.rs` gains `chip(look, role)` (backgrounds only at truecolor; uses the unused `on_accent`) and `bars.rs` wraps chip text in spaces or brackets by depth. `chrome.rs` renders segments and loses `map_hints` and the `mark` overlay. `frame::header/tabs` stay for `--once` only | new `bars.rs`, `chrome.rs`, `style.rs`, `side.rs` (badge counts beside `summary`), `guards.rs` |
| 3 | Remove the Medium summary row: `top = 2` for every class, `Areas.summary` deleted | `layout.rs`, `chrome.rs`, `tests/layout.rs` |
| 4 | Rail: dim bars with a separate glyph span; headers clipped with `…`; `Shape` gains `detail`, `current`/`selected` text rule; calm header shows the splice name only; `shown_id` strips `name/` at calm. Move `left`, `clip`, `shown_id`, `id_column` to new `rail_text.rs` to get back under budget | `rail.rs`, new `rail_text.rs`, `map_view.rs` (passes selection and detail) |
| 5 | Map pane: title `plan`; `progress::summary(answer, detail)` returns spans (neutral, stale count toned) used by the map only, `progress::facts` unchanged for text mode and `--once`; calm bottom block = clipped `last plan event` + `▶` caption; detailed = records without the position line; legend removed from the pane; blank spacer only when the rail fits | `map_view.rs`, `progress.rs` |
| 6 | Runs list: `lines::run_item(row, detail)`; calm = title line + `[status-if-not-open]  project  plan words before the first key=value token`, clipped with `…`; both callers (`view::runs`, `list_view`) | `lines.rs`, `view.rs`, `list_view.rs`, `frame.rs` (`--once` passes `Full`) |
| 7 | Inspector: `inspect::content(node, glyphs, look, detail)`; calm = short id + status chip, full id (dim), owes, gate lines without the duplicate `!flag` suffix, strip, attention items with their segment glyph, `read`/`next`, hint `d receipts and covered items   g gates   x text`; no `evidence`, no `splice`/`under`, no run-level records. Title drops `(Esc: back)` | `inspect.rs`, `inspect_view.rs` |
| 8 | Help: `Marks` and `This frame` blocks (legend strings move here from `map_view::legend`), `d` in the key list, `else clay` | `frame.rs` or new `help.rs` if `frame.rs` passes 250 |
| 9 | Docs: `docs/guide/watch.md` (panes, "Recorded progress", controls table gains `d`, layout section loses the summary row, new "Calm and detailed" section with the two frames); `docs/writfold/kb-v1-design.md` new **§36 T4 as built** (36.1 disclosure rules, 36.2 bars and degraded forms, 36.3 what moved where, a table mapping each removed default fact to its key, 36.4 tests and mutation checks, 36.5 defects fixed, 36.6 not done: group folding, persisted detail level, site font) | docs |
| 10 | Re-bless goldens with `WRITFOLD_KB_BLESS=1`, review each diff by eye; drive the binary in tmux at the four sizes, truecolor, `--color ansi`, `--color none`, `TERM=dumb`; run `scripts/gate quality` and `scripts/gate unit` from main | `tests/golden/*` |

`--once` and piped output are not touched: they stay the full plain frame, so scripts and the
three `*-once*.txt` goldens do not change (only `frame-*` and `t3-*` buffers do).

### Tests

New file `tests/calm.rs` (default-level guards; each fails if its check is removed):

- `default_map_rows_carry_no_timestamps_hashes_or_paths`: fresh `App`, nested fixture, 136x38
  and 160x40; inside the map rectangle no `recorded 20`, no ` @` followed by 12 hex, no
  `.writfold/plans/`; inside the inspector no line starting `@`. Mutation: setting the default
  to `Full`, or dropping the detail test in `node_line`, fails it.
- `text_follows_current_and_cursor`: exactly the `▶` row has verb text; after `j`, the newly
  selected row gains its text and the `▶` row keeps it.
- `d_restores_every_default_omission`: after `d`, each string the first test forbids is
  present, plus `last checkpoint:`, `splice:`, `under:`; a second `d` returns to a buffer equal
  to the first. Mutation: unbinding `d` fails it.
- `nothing_is_lost`: for every step row, every `more` text and every non-empty `path` appears
  either in the detailed inspector for that node (scrolled through) or in the `g`/`x` content.
- `calm_run_line_is_a_prefix_of_the_verb_plan_text` and `tab_badges_equal_verb_counts`
  (gates, inbox, guards; zero counts print no badge).
- `rail_bars_never_take_a_state_colour` in `tests/map_colors.rs`: truecolor buffer, every
  `│ ├ ╮ ╯` cell in the rail has the dim foreground. Mutation: restoring the single prefix span.
- `bars_never_cut_a_token`: for widths 60..=200, the footer and header rows split on runs of
  spaces yield only whole segments from `bars::footer`/`bars::title`; the right group is
  always complete.
- `chips_degrade_to_brackets`: at `Depth::Ansi` and `Depth::None` no bar cell has a
  background and the row contains `[open]`, `[1 progress]`, `[read-only]`; at truecolor the
  active tab cells are `accent`/`on_accent` and no bracket is printed. Extend
  `look.rs::degraded_depths_have_no_background_or_unwanted_colours` to cover the bars.
- `legend_lives_in_help`: the map buffer has no `satisfied`; the help buffer has `Marks`,
  `not a liveness claim` and the full subject stamp.

Changed: `tests/layout.rs` (no summary rectangle; Medium main starts at row 2; counts asserted
on the tab row), `tests/watch.rs:77` (summary text moves to the gates view / badges),
`tests/map_frames.rs:122` and `tests/map_recorded.rs` (`recorded position:` asserted in help;
event and checkpoint asserted at the detailed level; hint string), `tests/theme_select.rs`
(`theme=` moves to the help overlay and the cycle notice), `src/inspect_tests.rs` (hint string,
both levels), `src/layout.rs` unit tests, `tests/progress.rs:32`. The `honest()` word filter
stays and runs on both levels; the new strings (`as recorded`, `next step to pick up`) pass it.
Add goldens `t4-calm-136x38.txt`, `t4-calm-80x24.txt`, `t4-detail-136x38.txt`.

### Order and size

1 → 4 → 5 → 7 → 6 (panes; frame already calmer after each) → 2 → 3 (bars) → 8 → tests → 10 → 9.
Roughly 450 lines of source changed or added, 250 of tests, 12 goldens re-blessed and 3 added.
If the session runs short, items 2–3 (bars) are separable from 4–7 (panes): ship the panes and
`d` first, since they answer the owner's complaint directly.

## 5. Decisions (15 November 2026)

1. The inbox badge is one number (drafts plus proposals); view 4's first line keeps the breakdown.
2. The calm inspector hides covered checklist items; the segment strip still shows all of them, and `d` lists every item with its receipt.
3. The `▶` caption stays on screen by default as one dim row.
4. Bars (items 2–3) are in scope for this slice, not optional: the owner asked for the header, tab bar and modeline to be polished with chips.
