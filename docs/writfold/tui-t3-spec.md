# T3: the plan map and node inspector in `writfold watch` — specification

Status: design, not built. Written 14 November 2026 against `main` at `c7bb3d4` (v0.5.0-dev.5, T2 as built). Branch from current `main` and re-read every file named here before editing it.

Audience: an agent implementing T3 in its own worktree. Read first: `AGENTS.md`, `docs/writfold/mission.md`, [`tui-t2-spec.md`](tui-t2-spec.md) (its principles, section 1, and owner decisions, 0a, still hold unless this document says otherwise), `docs/writfold/kb-v1-design.md` §34 (T2 as built) and §21/§24 (run graph), `docs/guide/run-graph.md`, `docs/guide/watch.md`, `docs/guide/json.md`.

## 0. Summary

T2 shows run progress faithfully, but as the verbs' text in panes. T3 draws the run's plan as a **map**: the ordered spine of steps with spliced steps nested under the step they belong to, a glyph per step state, gate chips per step, and a marker on the recorded current step. Selecting a node opens an **inspector** that shows that step's gates, checklist items, evidence paths, why it is stale and the next command. Everything stays a projection of verb output (T2 principle 1), read-only (principle 3), and honest about progress (principle 2): the marker is the *recorded* position, never a claim that an agent is working.

The work is small on the KB side (two additive `more` labels on `plan status --gates`, plan events in `timeline <run>`) and mostly in `crates/writfold-tui`.

## 0a. Decisions (settled 14 November 2026; the owner may override any of them)

1. **Structure comes from the verb.** `plan status --gates` gains `splice` and `under` lines (3.1). The viewer never splits a step id on `/` or `#`.
2. **Plan events join `timeline <run>`** (3.2), so the map can show the last recorded plan event.
3. **Unicode marks**, with an ASCII set under `--color none` and `TERM=dumb` (5.2).
4. **The map is the default form of Progress**; `x` toggles to the T2 text list (which keeps its expand/collapse on `X`, see 7).
5. **While the map has focus in the wide layout, the inspector replaces the side stack**; Esc or `h` brings the stack back. Medium splits runs/map/inspector; narrow shows one at a time.
6. **No new dependencies.** ratatui 0.29's `Block`, `Paragraph`, `List`/`Table` and plain buffer drawing are enough. No `Canvas`, braille, `Gauge`, `BarChart` or `Sparkline`: curves add nothing to a spine, gauges scale to width and print percentages, and there is no honest series to plot.
7. **Gates and merge** as T2 0a.7: the implementer does not run `scripts/gate` or `writfold kb gate run`; commit per slice on the worktree branch with local fmt, clippy, tests and the xtask quality report clean (private `CARGO_TARGET_DIR` inside the worktree), write §35 and the docs, and report. The integrator merges, runs the gates from `main` and releases.
8. **Naming** as T2 0a.8: no document, comment or commit mentions anything outside Writfold's own terms.

## 1. What the graph is (and is not)

- **One ordered spine, no branching.** `run-graph.md`: "There is no branching and no general graph." No parallel lanes, no gate-to-gate edges, no subagents.
- **Nesting is the only structure.** A splice (`plan use`, or a template's `use=` include) puts children immediately before the step they sit under; that step stays open until after them. Depth is at most `writfold_core::plan::MAX_DEPTH`. Root-level groups, in order: required-start templates, the plan's template, required-end templates.
- **Order can be disturbed.** After a merge of two clones, `!out-of-order`, `!two-plans` and `!repeated` can put a done step after open ones. The map draws each node from its own state, never "everything above the marker is done".
- Therefore the map is a **bracket rail** (design A below), not a flowchart. A boxed flowchart (3–4 rows per node, side boxes that read as parallel branches) and a horizontal track (does not fit 15+ steps; a time axis implies durations) were considered and rejected.

## 2. "Where the agent is": the recorded position

Writfold records no presence, and a run can be walked by several sessions. The map therefore says **recorded position**, never "agent".

Shown:
- `▶` on the row the verb marks `[current]`, with the legend "recorded current step, not a liveness claim".
- `step N/M` from the trailer counts (`current`, `steps`).
- **last plan event**: the newest plan row of `timeline <run>` (3.2), its recorded time verbatim and its node id.
- **last checkpoint**: the selected run's `runs` row `last:` line, verbatim.
- Receipt and event paths, in the inspector.
- The footer's `asked HH:MM:SSZ`, unchanged (a fact about the view).

Never shown: a spinner, blink, pulse or animated edge; the words `running`, `working`, `active`, `idle`, `stalled`, `live`, `in progress` about a run or step; elapsed, relative or "since" times or step durations; a track or bar filled up to the marker; a later gate drawn as open (it is `·`, not judged); a session or agent name beside the marker; a marker moved back onto a stale done step.

## 3. Verb changes in `writfold-kb` (additive; envelope unchanged)

### 3.1 `plan status --gates`: `splice` and `under`

Only with `--gates` (the detailed form T2 added). Without `--gates` output and query digest are unchanged.

- Every step row gains one `more` line, label `splice`: text `<template>#<n> @<digest12> from <source>` (the same words the plan note uses today), `path` the splice record (`runs/<run>/plan/splice-….json`), `line` 1, no flags, no `status`. Steps of one splice share identical text; the viewer groups on string equality.
- Every step that is not at the root gains one `more` line, label `under`: text the parent step's `id` exactly as that parent's own row prints it, no path.
- Place both immediately after the row's existing first continuation group and before `gate` lines, in a fixed order (`splice`, then `under`), so text output stays stable.
- The values come from `fold::Step` (`template`, `digest`, `source`, `splice`, `root`) and the parent the fold already knows; expose what is needed through `plan/detail.rs` and `plan/show.rs`. No new parsing.
- Tests: the `plan_detail` goldens (text and JSON) change and are reviewed by hand; a new nested fixture (plan with a `plan use` splice at depth 2 and a `use=` include) asserts that every `under` names an existing row id and that the rows sharing a `splice` text are exactly the steps of that splice. `docs/guide/json.md` documents both labels.

### 3.2 `timeline <run>`: plan events

The run's plan events join the run's own entries (T2 3.4) as rows of kind `plan`:

- One row per splice, done and skip event under `runs/<run>/plan/` that folds into the plan (an `!invalid` event is not listed). `status` is `spliced`, `done` or `skipped`; text `<recorded time> <status> <node id>` plus `(by WHO)` and the reason for a skip; `path` the event file, line 1.
- Ordered with the other rows by the verb's existing date order; ties keep a stable order (event file name).
- Non-run timeline targets are unchanged. The `timeline_run` golden changes and is reviewed by hand.
- The Timeline view and the T2 compact timeline pane show these rows like any other.

### 3.3 Not changed

The JSON envelope, `crates/writfold-kb/tests/json.rs`'s row key set and every other verb. If a `more` label set is pinned anywhere, add `splice` and `under` there as optional labels.

## 4. Layout and panes

T2's breakpoints are unchanged (wide 140×32, medium 90×16, narrow 60×12). Progress with a plan renders the map; without a plan it renders exactly T2's no-plan content (note plus Gates), with no edges, because there is no order.

Focus order: **runs → map → inspector**. `l`/Right/Enter move right, `h`/Left/Esc move left.

- **Wide:** runs | map | side stack. When focus is on the map or inspector, the side-stack rectangle shows the inspector of the selected node; returning focus to runs restores gates/inbox/guards/timeline.
- **Medium:** runs (24 columns) | map (≥32) | inspector (rest), from the moment Progress shows a map; the T2 one-line gate/inbox/guard summary stays above.
- **Narrow:** the map fills the main pane; `l`/Enter opens the inspector full-pane; `h`/Esc returns. `0` still shows runs.
- Below the rail, three fixed lines: `recorded position: step N/M (…)`, `last plan event: …`, `last checkpoint: …`; then the glyph legend (two lines). Omit lines that do not fit, legend first.

### 4.1 Mockup, 160×40 (wide), map focused, hone

Schematic content (a nested fixture: a `review` skill spliced under `change#2/review`). The selected row is reverse video (here step 2/9). The inspector shows two examples separated by a rule: the selected step, then what it looks like when a checklist gate is selected.

```text
run-20261114T070737Z-53e8a7 [open] !current  Add --plan to kb runs  project=harbor  subject=kb
[1 progress] 2 gates 3 timeline 4 inbox 5 guards 6 stale 7 brief 8 resume
┌ runs (3) ────────────────────────────┐┌ progress: map run-20261114T070737Z-53e8a7 (focus) ───────────────────┐┌ step 2/9 (Esc: gates/inbox/guards/timeline) ─┐
│> [open] run-20261114T070737Z-53e8a7 !││plan 3 done, 1 skipped of 9 steps; current step 5/9; owes 1 of 1 gates││step 2/9 · change#2/implement [done] !stale   │
│    Add --plan to kb runs             ││ [x] [!] [-] [x] [>] [ ] [ ] [ ] [ ]                                  ││spliced by change#2 @83c2cc9efde7             │
│    project=harbor repo=kb branch=main││                                                                      ││                                              │
│    plan: step 5/9 done=3 skipped=1 cu││ ┬ research#1 @5d01c2e88a17 from .writfold/plans/research.toml        ││why: no longer holds: lint [stale] (done stays│
│    last: 2026-11-14T07:07:44Z Review ││ ✓ research#1/research                  ●    done 2026-11-14T07:07:40Z││done; list the gate again on a later step if  │
│  [open] run-20261114T070737Z-856743  ││ ┼ change#2 @83c2cc9efde7 from .writfold/plans/change.toml            ││it must hold there)                           │
│    Review the lantern guide          ││ ! change#2/implement                   ●!   done 2026-11-14T07:07:44Z││                                              │
│    project=lantern repo=kb branch=mai││ – change#2/benchmarks                       skipped by owner 2026-11-││gates (2)                                 ●!  │
│    last: 2026-11-14T07:07:37Z Read th││ ├─╮ review#1 @94d8a4f70bc4 under change#2/review                     ││  unit [satisfied] mechanical                 │
│  [open] run-20261101T090000Z-a1b2c3  ││ │ ✓ …/review#1/read-diff                    done 2026-11-14T07:07:44Z││    @runs/…/gates/unit-20261114T070744Z-b74539│
│    Batch ledger writes rollout       ││ │ ▶ …/review#1/findings                ○    current: owes: findings [││    .json:1                                   │
│    project=harbor                    ││ ├─╯                                                                  ││  lint [stale] mechanical !stale              │
│    last: 2026-11-03T16:00:00Z Batchin││ ○ change#2/review                      ··   later: gates: review-list││    @runs/…/gates/lint-20261114T070744Z-ad7026│
│                                      ││ ○ change#2/deliver                     ·    later: gates: unit       ││    .json:1                                   │
│                                      ││ ┼ reflect#3 @77aa10c4e2b9 from .writfold/plans/reflect.toml          ││                                              │
│                                      ││ ○ reflect#3/reflect                    ··   later: gates: reflected, ││done: recorded 2026-11-14T07:07:44Z           │
│                                      ││ ┴                                                                    ││  @runs/…/plan/done-change.2~implement.json:1 │
│                                      ││                                                                      ││                                              │
│                                      ││recorded position: step 5/9 (first step neither done nor skipped)     ││── when a checklist gate is selected ─────────│
│                                      ││last plan event:   2026-11-14T07:07:44Z done …/review#1/read-diff     ││review-list [stale] checklist                 │
│                                      ││last checkpoint:   2026-11-14T07:07:44Z Review started.               ││  2/4 required covered   ■ ! □ ■   (+1 optiona│
│                                      ││                                                                      ││  item errors [supported] 1 reference by agent│
│                                      ││● satisfied ◆ attested/waived ○ open ! stale × failed ? unknown       ││  item tests [stale] 1 reference by agent;    │
│                                      ││· later, not judged      ▶ recorded current step, not a liveness      ││    cited evidence moved: gate:lint was       │
│                                      ││                           claim                                      ││    satisfied, now stale !stale               │
│                                      ││                                                                      ││  item docs [open] The guide names every new  │
│                                      ││                                                                      ││    flag                                      │
│                                      ││                                                                      ││  item perf [not-applicable] no hot path      │
│                                      ││                                                                      ││  item style [open] (optional) Names follow   │
│                                      ││                                                                      ││next: writfold kb gate cover review-list tests│
│                                      ││                                                                      ││  supported REF… --by WHO                     │
│                                      ││                                                                      ││                                              │
│                                      ││                                                                      ││full receipts: g   all steps as text: x       │
│                                      ││                                                                      ││                                              │
│                                      ││                                                                      ││                                              │
└──────────────────────────────────────┘└──────────────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────┘
kb=50272d6dff79 index=fresh  subject=kb@50272d6dff79+dirty:0  asked 07:47:29Z  theme=hone  read-only; no daemon  jk node  l inspect  . current  n flag  ? help
```

### 4.2 Mockup, 100×30 (medium)

```text
run-20261114T070737Z-53e8a7 [open] !current  Add --plan to kb runs
[1 progress] 2 gates 3 timeline 4 inbox 5 guards 6 stale 7 brief 8 resume
gates: open=2 stale=2 of 6 | inbox: drafts=0 proposals=1 | guards: deny=1 ask=0 of 2
┌ runs (3) ────────────┐┌ map ─────────────────────────┐┌ step 5/9 ────────────────────────────────┐
│> [open] run-20261114T││3 done, 1 skipped of 9; step 5││step 5/9 · …/review#1/findings [current]  │
│    Add --plan to kb r││ ┬ research#1                 ││under change#2/review; review#1 @94d8a4f70│
│    project=harbor rep││ ✓ research#1/research    ●   ││                                          │
│    plan: step 5/9 don││ ┼ change#2                   ││owes: findings [open]                     │
│    last: 2026-11-14T0││ ! change#2/implement     ●!  ││gates (1)  ○                              │
│  [open] run-20261114T││ – change#2/benchmarks        ││  findings [open] attested !open          │
│    Review the lantern││ ├─╮ review#1                 ││read: instructions for findings           │
│    project=lantern re││ │ ✓ …iew#1/read-diff         ││  @.writfold/gates/findings.md:1          │
│    last: 2026-11-14T0││ │ ▶ …view#1/findings     ○   ││next: writfold kb gate attest findings    │
│  [open] run-20261101T││ ├─╯                          ││  --statement TEXT --by WHO               │
│    Batch ledger write││ ○ change#2/review        ··  ││                                          │
│    project=harbor    ││ ○ change#2/deliver       ·   ││last plan event: 2026-11-14T07:07:44Z     │
│    last: 2026-11-03T1││ ┼ reflect#3                  ││  done …/review#1/read-diff               │
│                      ││ ○ reflect#3/reflect      ··  ││last checkpoint: 2026-11-14T07:07:44Z     │
│                      ││ ┴                            ││  Review started.                         │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
│                      ││                              ││                                          │
└──────────────────────┘└──────────────────────────────┘└──────────────────────────────────────────┘
kb=50272d6dff79 index=fresh  subject=kb@50272d6dff79+dirty:0  hone  read-only  ? q
```

## 5. The map

### 5.1 Rows

One row per step, in the verb's row order; group headers between them:

- **Group header** at the first step of each splice group: `┬` for the first root group, `┼` for later root groups, `├─╮` for a nested group, and the header text is the `splice` line's text (or `under <parent>` for nested groups where width is short). The nested group's last step is followed by `├─╯`; the plan ends with `┴`.
- **Depth** is shown by `│ ` per nesting level, from the `under` chain.
- **State glyph** (column after the rail): `✓` done, `!` done but flagged stale, `–` skipped, `▶` current, `○` later, `×` a flag the verb marks invalid. Disorder flags (`!out-of-order`, `!two-plans`, `!repeated`) show the step's own glyph plus the flag word at the row end.
- **Id**: the row `id`, truncated from the left with `…` when narrow (presentation only).
- **Gate chips**: one glyph per gate of the step, in the verb's gate order, from each `gate` line's `status`: `●` satisfied, `◆` attested or waived, `○` open, `!` stale, `×` failed, `?` unknown, `·` later (not judged). Never a count bar.
- **Rail text** (right of the chips): the row's `done:` line, skip text or `owes:`/`later:` text verbatim, truncated.

### 5.2 Glyph sets and colour

- Truecolor and ANSI use the Unicode set above and T2's theme roles: done/satisfied Good, attested/waived the distinct attestation role, stale/failed/invalid Bad, current/owed Warn, later and `·` Dim.
- Under `--color none` or `TERM=dumb`, use ASCII: `[x] [!] [-] [>] [ ] [#]` for states and `o * . ! x ?` for chips, with `|`, `+`, `\` and `/` for the rail. Shapes alone must distinguish states (no-colour test).
- Checklist **segments** in the inspector: one cell per required item in the verb's item order (`■` supported, `◆` not-applicable/attested, `!` stale, `□` open), then `(+N optional)`. Like the T2 strip, the cell count never scales with width.

### 5.3 Selection and scrolling

- Selection is a step id. It defaults to the verb's current row, is kept by id across re-asks and refreshes, and falls back to the current row (then the first row) when the id disappears.
- `j`/`k` move it; the map scrolls to keep it visible with two rows of context. `.` jumps to the recorded current step; `n`/`N` to the next/previous row with any flag.
- The map is not a scrolled `Paragraph` with a `u16` offset: build it from a row model (`rail.rs`, 9) that knows each node's line index, and render only the visible window.

## 6. The inspector

For the selected step, in order, each line verbatim from the verb (`plan status --gates` row and its `more` lines):

1. `step N/M · <id> [<status>] <flags>`, then the `splice` and `under` lines.
2. `why:` (if any), `owes:` (current step).
3. `gates (K)` with the chips, then each `gate` line with its evidence path wrapped with a hanging indent.
4. For a checklist gate: the `<covered>/<required> required covered` count with segments (5.2), then each `item` line.
5. `read:` and `next:` lines.
6. `done:` / skip line with its event path.
7. A fixed hint line: `full receipts: g   all steps as text: x`.

The inspector does not join `gate status` rows (their `more` lines carry no gate id to join on); `g` opens the Gates view for full receipts. `j`/`k` scroll the inspector when it has focus.

## 7. Keys

All T2 keys keep their meaning except `x` (below). New bindings, all currently unbound:

| Key | Action |
|---|---|
| `h` / Left | Move focus left (inspector → map → runs) |
| `l` / Right | Move focus right (runs → map → inspector) |
| Enter / Esc | As T2 (Enter cycles focus forward; Esc steps back) |
| `j` / `k` | In the map: move the node selection; in the inspector: scroll |
| `.` | Select the recorded current step |
| `n` / `N` | Next / previous flagged step |
| `x` | In Progress: toggle map and T2 text list; inside the list it keeps T2's expand/collapse via `X` |

`c` stays unbound. No key writes. Add Left/Right to `Key` and to `term.rs`'s mapping. Update the help overlay.

## 8. `--once` and `--view`

No box drawing or glyphs. `--once` prints T2's expanded plain frame; Progress gains only the verb's new `splice:` and `under:` lines (3.1) and Timeline the plan rows (3.2). The `--once` goldens change by exactly those lines and are reviewed by hand.

## 9. Modules, budgets, tests

### 9.1 `crates/writfold-tui`

New, each ≤250 lines, functions ≤80, no `unwrap`/`expect`:

- `graph.rs` (pure): plan rows → `Vec<Node { id, depth, group, state, chips, rail_text, flags }>` plus group boundaries, using only `id`, `status`, `flags`, `splice`, `under`, `gate` `status`.
- `rail.rs`: nodes + width + glyph set → lines, and each node's line index; the visible window for a selection.
- `inspect.rs`: selected node → lines (reuse `progress.rs` line builders where they fit).
- `cells.rs`: glyph sets (Unicode/ASCII) and tones for states, chips and segments.
- `nav.rs`: split from `keys.rs` (112 lines today) for focus and selection movement.

Changed: `app.rs` (`Focus::Inspector`, selected id, `Key::Left`/`Right`), `term.rs`, `keys.rs`, `layout.rs` (focused variants), `view.rs`, `frame.rs` (help), `chrome.rs` (footer hints `jk node  l inspect  . current  n flag`).

### 9.2 Fix first (T2 defects visible in the 160×40 frame)

- Wrapped continuation lines in the main pane restart at column 0 (`--by WHO`, `stale=1` in the T2 frame). Give wrapped lines a hanging indent equal to their first line's indent. This applies to the T2 list and the inspector.

### 9.3 Fixtures

Extend `crates/writfold-tui/tests/support/` with: **linear** (the T2 six-step fixture), **nested** (a `plan use` splice at depth 2 plus a `use=` include), **out-of-order** (two clones merged so a done step follows an open one), **no subject**, and **long** (40 steps, to exercise scrolling). Record them through the KB library as T2's fixture does; never hand-write plan event files except where a test exists to prove the fold flags them.

### 9.4 Tests

- `TestBackend` buffer goldens of the map focused at 160×40, 100×30 and 72×20 for the nested and out-of-order fixtures; plus one 160×40 golden with the inspector on a checklist gate. Reviewed by hand.
- `graph.rs` unit tests: nesting from `under`; groups from `splice`; chips from gate `status`; the out-of-order fixture draws each node from its own state.
- Selection: default current; kept by id across `r` and a refresh that adds a step; falls back when the id disappears; `.`, `n`, `N`; the selected node stays visible while scrolling the long fixture.
- No-colour: ASCII set; every state glyph distinct.
- The T2 read-only key tour, the no-checker test and the forbidden-language assertion are extended to the new keys, labels and legend.
- Mutations, each of which must fail a test (record them in §35 as T2 did):
  1. a later gate drawn as open instead of `·`;
  2. the marker placed on a stale done step instead of the verb's current row;
  3. nodes above the marker drawn done regardless of their state (out-of-order fixture);
  4. `under` ignored, flattening the nesting;
  5. segment cells scaled with width (same cell count asserted at two widths);
  6. selection lost on re-ask.

## 10. Documentation to update

- `docs/guide/watch.md`: the map, glyph legend and ASCII set, inspector, focus model, new keys, the "recorded position" wording.
- `docs/guide/json.md`: `splice`, `under`, timeline `plan` rows.
- `docs/guide/run-graph.md`: one line pointing to the map.
- `docs/writfold/kb-v1-design.md` §35 "T3 as built", from what was built, with deviations, mutations and Not done.
- `docs/writfold/mission.md`: the roadmap line for T3.
- README `watch` paragraph if its key list changes.
- The website is the integrator's job; do not edit it.

## 11. Order of work (commit per slice)

1. The hanging-indent fix (9.2), with a test.
2. `plan status --gates` `splice`/`under` (3.1) with the nested fixture and goldens.
3. `timeline <run>` plan rows (3.2) and goldens.
4. `graph.rs`, `cells.rs` and their unit tests.
5. `rail.rs`, the map in Progress, selection and scrolling, keys and focus (`nav.rs`), `x` toggle.
6. `inspect.rs` and the focused layouts (wide/medium/narrow); buffer goldens.
7. Documentation and §35. Then report; the integrator runs the gates (0a.7).

## 12. Acceptance checklist

- [ ] On a planned run, Progress shows the map: one row per step in verb order, nesting equal to the `under` chain, group headers equal to the `splice` text, state glyphs and gate chips equal to the verb's statuses.
- [ ] `▶` is on the verb's `[current]` row only; on the out-of-order fixture no node's glyph depends on its position relative to the marker.
- [ ] The recorded-position block shows `step N/M`, the newest timeline plan event and the last checkpoint, each verbatim; the forbidden-language assertion passes across every frame and size.
- [ ] The inspector shows the selected step's lines verbatim, with segments for checklist gates that do not scale with width.
- [ ] `h`/`l`/Left/Right/Enter/Esc move focus as section 4; `j`/`k`, `.`, `n`, `N`, `x` work; `c` does nothing; the read-only and no-checker tests pass.
- [ ] 160×40, 100×30, 72×20 and a narrow size render as section 4 says; the buffer goldens are committed and reviewed.
- [ ] `--color none` and `TERM=dumb` use the ASCII set and every state is distinguishable without colour.
- [ ] `--once` differs from T2 only by the verb's new lines; no glyphs or escape sequences.
- [ ] `plan status` without `--gates` and every non-run `timeline` are byte-identical to before; `json.md` documents the additions.
- [ ] No new crate in `Cargo.lock`; the xtask quality report has no new error or warning; modules ≤250 lines, functions ≤80; no `unwrap`/`expect`.
- [ ] Each of the six mutations in 9.4 fails a test.
- [ ] The static musl binary builds offline; its `watch` was driven through a pseudo-terminal at 160×40 and 100×30 in truecolor and with `NO_COLOR=1`.
- [ ] §35 and the guides are written from what was built.
- [ ] (Integrator) `scripts/gate quality` and `scripts/gate unit` pass from `main`.

## 13. Not in T3

- Any write key; a mouse; a text search; cross-KB views.
- A flowchart, a horizontal timeline track, a time-scaled lane, gauges, bar charts, sparklines or braille drawing.
- Durations, relative times, liveness, or names of sessions/agents.
- Joining `gate status` rows into the inspector.
- Changing `--once` beyond the verb's new lines; `watch --json`.
