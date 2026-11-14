# Watching a run

> **Status:** built (T3). Implementation, tests and deviations: [design §35](../writfold/kb-v1-design.md#35-t3-as-built); the earlier text view is recorded in [§34](../writfold/kb-v1-design.md#34-t2-as-built).

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
ASCII chips are `o` satisfied/open, `*` attested/waived, `!`, `x`, `?`, and `.`;
the inspector retains the exact state words. ASCII rails use `|`, `+`, `\`, `/`.
`--color none`, automatic `NO_COLOR`, and `TERM=dumb` select ASCII map shapes;
`TERM=dumb` keeps those shapes even when colour is explicitly enabled.

The marker means **recorded current step, not a liveness claim**. The block below
the rail shows `step N/M` from the trailer, the newest plan event from Timeline,
and the run list's last checkpoint. Missing subject counts stay missing; the
verb's no-subject note is shown instead of inventing a position count.
Done stays done when its evidence becomes stale; the marker never moves back.

Selection starts at the recorded current step, survives refresh by ID, and falls
back to the current step or first row if its ID disappears. The rail keeps it
visible with two rows of context where space permits. IDs truncate from the
left; group headings and trailing rail text clip to the available width.

The inspector quotes the selected step's splice, parent, reasons, gates, items,
read/next commands and event evidence. Paths wrap with a hanging indent.
Checklist segments are one per required item, in source order: `■` supported,
`◆` not-applicable/attested, `!` stale, `□` open, then `(+N optional)`.
Their count never scales with width. The inspector also wraps the last plan
event and checkpoint; `g` opens full gate receipts. No gate-status rows are
joined into it.

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
| `x` / `X` | In Progress: map/text toggle / expand or collapse the text list |
| `o` | Toggle `kb runs --open` |
| `P` | Toggle the selected project's `kb runs --project`; without a project, show a notice and change nothing |
| `r` | Drop cached answers and ask again |
| `T` | Cycle the six themes in truecolor; otherwise explain how to enable it |
| `?` | Help; any key except `q` closes it |
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
| Medium | 90×16, when not wide | With a map: runs 24 columns, map 32, inspector in the remainder; summary above |
| Narrow | 60×12, when not medium | Map or inspector alone; `l`/Enter opens the inspector, `h`/Esc returns; `0` reveals runs |
| Too small | Otherwise | `writfold watch needs 60x12 …`; controls still work |

Without a plan, or in another view, the earlier layouts remain. The wide stack
contains Gates/Inbox/Timeline, plus Guards from height 38; its panes are not
focusable. Text and evidence wrap with hanging indentation; maps and lists
truncate. The map reserves recorded facts below the rail and omits legends first
when space is insufficient. The footer clips metadata to preserve the ask time,
theme and map hints; narrow layouts retain the KB pin and quit/help hints.
[§35](../writfold/kb-v1-design.md#35-t3-as-built) links the committed map frames.

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

Themes cycle in this order: **hone**, **slate**, **carbide**, **vellum**, **paper**,
**paper-dark**. Vellum and paper are light; the others are dark. The selected name
comes from `--theme`, then `WRITFOLD_THEME`, then the existing user configuration,
then hone. The configuration is `$XDG_CONFIG_HOME/writfold/config.toml`, otherwise
`~/.config/writfold/config.toml`; its quoted line is:

```text
theme = "paper"
```

An unknown highest-precedence name keeps hone and warns with the source and all
six choices. A lower-precedence name that was overridden produces no warning.
The interactive warning occupies the footer until the next key; plain output
sends it to stderr. Cycling never writes the configuration.

| Colour mode | Behaviour |
|---|---|
| `auto` | `NO_COLOR` set to any value, or `TERM=dumb`: none; otherwise `COLORTERM=truecolor` or `24bit`: truecolor; otherwise ANSI |
| `truecolor` | Force palette foregrounds and surfaces |
| `ansi` | Force named terminal colours, with no painted background |
| `none` | No foreground/background colours; selected entries reverse and key headings retain emphasis |

Explicit modes override environment detection. There is no 256-colour palette
quantisation. Contrast tests cover every palette's inks, surface ladder, accent
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
