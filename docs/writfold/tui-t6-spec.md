# `writfold watch` T6: a plain-language surface

Looked at: main at 808b48b (v0.5.0-dev.10 planning), `crates/writfold-tui/src`, and live
frames driven in tmux at 136×38 and 80×24 against the two capture worlds (the run world of
the homepage frame and the knowledge-base world): every run view, every KB view, the
inspector, `d`, `x` and `?`. Read-only; nothing in the repository was changed. The "before"
frames below are those captures with blank pane rows trimmed.

Owner signal: the idea is right (execution tracking and the knowledge base in one frame),
and the layout stays. What has to change is the wording. The default surface must read as
plain language to a person glancing at it; the terse machine text stays reachable, but not
on the surface. Emphasis on readability.

T4 removed lines. T6 changes what the remaining lines say. The frame keeps its panes, rail,
state glyphs, bars, keys and both modes.

## 0. What a person reads today

The homepage frame (136×38, run mode, the current step selected) is the calm level, and it
still carries 8 bracketed states (`[stale]`, `[open]`), 7 verb labels (`owes:`, `gate`,
`read:`, `next:`), a run id, a full step id, two timestamps, a KB stamp with a dirty count
and a command template with placeholders. The full views are denser: Gates has 11 bracketed
states, 14 `@path:line` positions and 19 `key=value` tokens in one pane; Timeline has 13
ISO timestamps and 14 positions; Inbox prints absolute paths into `.git`.

None of that is wrong. It is the verbs' own text, written for an agent that will act on it.
A person asks five things, in this order:

1. What is this run? (title, project, open or closed)
2. Where is it? (which step, of how many)
3. What is next? (the step to pick up)
4. What is in the way, and why? (which gates, in words)
5. What changed? (the last recorded event)

An agent or a power user asks for exact ids, commands and provenance. T6 gives the first
reader the surface and the second reader one key.

## 1. Readability principles

**R1. Two levels, one key.** The frame has a **plain** level (the default) and an **exact**
level (`d`). Exact is the pre-T6 detailed frame, byte for byte: the verbs' wording, ids,
flags, receipts and stamps. The pre-T6 calm level is retired; nothing else is removed from
the product.

**R2. Keep the nouns, drop the notation.** The surface uses the product's nouns (run, step,
gate, checklist, guard, draft, proposal, decision, checkpoint) and the names people chose
(gate names, step names, ticket keys, titles). It does not use notation: `[state]`
brackets, `!flag` words, `key=value`, `label:` prefixes, `@path:line`, hashes, ISO
timestamps, typed ids (`project:harbor`, `code:repo:path`), command templates.

**R3. Words carry states; glyphs only repeat them.** A state is always written as a word or
phrase. A glyph may sit beside the phrase (rail glyphs, the item box) or stand alone only
when the same pane says what it means in a caption (`▶`, the `!` mark of KB lists). Gate
chips (`●!`, `!○`, `··`) leave the map rows.

**R4. Say it only where it matters.** A map row is rail, glyph and name. It gets words only
when it is the recorded current step, the selected row, or flagged. Zero counts are not
printed. A footer fact appears only when it is not the normal case.

**R5. One phrase per state, everywhere.** `stale` is "out of date" in the map, the
inspector, Gates, the tab badge and KB lists. The table in §2.1 is the whole vocabulary,
and `?` prints it next to the exact words.

**R6. Sentences, top-down.** The inspector answers in reading order: what this is, where it
sits, what it needs, each need with its state, what is suggested next. One fact per line;
sections are separated by a blank line and headed by a plain word (`Why`, `Sources`,
`Linked from`).

**R7. Honesty before fluency.** A phrase never claims more than the state it stands for
(§5.2). When the viewer has no phrase for a word, it prints the word. When a verb's
sentence cannot be shown whole in plain form, it is withheld, never rewritten, and the pane
says so in one dim line.

**R8. The exact form is one key away and copyable.** `d` for the whole frame, `?` for the
vocabulary and this frame's stamps, `c` for the ids and commands on borderless rows.

## 2. Phrase tables

Each row: what the surface prints today, what the plain level prints, and where the exact
form lives afterwards. "Exact (`d`)" means the pre-T6 detailed frame, unchanged.

### 2.1 State vocabulary

The plain word is a lookup on the row's `status` or `flags`; the product's definition of
*met* (satisfied, attested or waived) is kept.

| Field value | Where | Plain phrase | Long form (inspector, Gates) | Exact form lives in |
|---|---|---|---|---|
| step `done` | map glyph `✓`, inspector chip | `done` | `Done on 2026-11-10 23:50.` | `d`, `x`, `?` Words |
| step `done` + `stale` | map `!`, `!stale` | `done; lint is now out of date` | `It stays done, but 1 of its gates is now out of date.` | `d` (`why:` line), `x` |
| step `skipped` | map `–` | `skipped` | `Skipped: (by owner) no hot path touched` when the reason passes §5.1, else `Skipped.` | `d`, `x` |
| step `current` | map `▶`, chip `current` | `next up` | `Needs 2 gates before it can be marked done.` | `d`, `x` |
| step `later` | map `○` | (no words) | `Later in the plan. Its gates are not judged yet.` | `d`, `x` |
| flag `out-of-order` | row end `!out-of-order` | `done out of order` | `Done while an earlier step was still open.` | `d` |
| flag `two-plans` | `!two-plans` | `two plans on this run` | same | `d` |
| flag `repeated` | `!repeated` | `spliced twice` | `The same template was spliced twice on this chain of steps.` | `d` |
| flag `template-moved` | `!template-moved`, count `moved` | `template changed` | `Its template file changed after the run spliced it.` | `d` |
| flag `invalid` | `!invalid`, glyph `×` | `invalid record` | same | `d` |
| gate `satisfied` | chip `●`, `[satisfied]` | `passed` | same | `d`, `g` at exact |
| gate `attested` | chip `◆`, `[attested]` | `signed off` | `signed off by a person; not a passing check` | `d` |
| gate `waived` | chip `◆`, `[waived]` | `waived` | `waived by a person; the check itself did not pass` | `d` |
| gate `open` | chip `○`, `[open]`, `!open` | `nothing recorded yet` | same | `d` |
| gate `stale` | chip `!`, `[stale]`, `!stale` | `out of date` | `out of date: the recorded result is about an earlier version` | `d` |
| gate `failed` | chip `×`, `[failed]` | `failed` | same | `d` |
| gate `unknown` | chip `?`, `[unknown]` | `no verdict` | `no verdict: the last run did not finish, or the files moved under it` | `d` |
| gate `later` | chip `·`, `[later]` | `not judged yet` | same | `d` |
| gate class `mechanical` | `mechanical` | `checked by a command` | same | `d` |
| gate class `checklist N/M required covered` | same text, strip `■ ! □ ◆ (+1 optional)` | `checklist · N of M required items covered` | same | `d` (strip and count) |
| gate class `attested` | `attested`, `attested: statements only, no command` | `decided by a person's statement; no command is run` | same | `d` |
| item `supported` | `■`, `[supported]` | `covered` | same | `d` |
| item `not-applicable` | `◆`, `[not-applicable]` | `marked not applicable` | same | `d` |
| item `attested` | `◆`, `[attested]` | `signed off` | same | `d` |
| item `stale` | `!`, `[stale]` | `out of date · the evidence it cites has changed` | same | `d` |
| item `open` | `□`, `[open]` | `not covered` | same | `d` |
| item marker `(optional)` | `(optional)`, `(+1 optional)` | `optional ·` before the state | same | `d` |
| guard `would-deny` | `[would-deny]`, `!deny`, badge `deny 1` | `would block` | same | `d` |
| guard `would-ask` | `[would-ask]`, `!ask`, badge `ask 1` | `would ask first` | same | `d` |
| guard `would-nudge` | `[would-nudge]`, `!nudge` | `would remind` | same | `d` |
| guard `would-pass` | `[would-pass]` | `would allow` | same | `d` |
| guard condition `gates.met` | `requires gates.met` | `needs every gate of the run met` | same | `d` |
| guard condition `step.met` | `requires step.met` | `needs the current step's gates met` | same | `d` |
| draft `open` + `draft` | `[open]`, `!draft` | `draft` | `confirming it records a checkpoint` | `d`, `c` |
| proposal `pending` | `[pending]`, `!proposal` | `pending` | same | `d`, `c` |
| flag `local` | `!local` | `kept only on this machine` | same | `d` |
| flag `tracked` | `!tracked` | `in the work tree` | same | `d` |
| run `open` / `closed` | chips | `open` / `closed` (unchanged) | | |
| run flag `current` | chip `current`, `!current` | chip `current run` | | `d` |
| cite flag `changed` | mark `!`, word `changed` | `changed` | `The cited content has changed since it was cited.` | `d` |
| cite flag `gone` | `!`, `gone` | `gone` | `The cited file or lines are no longer there.` | `d` |
| cite flag `moved→X` | `!`, `[moved]`, `moved→4-6` | `moved` | `The cited lines are now at 4-6.` (X whole) | `d` |
| cite flag `expired` | `!`, `expired` | `expired` | `It has not been seen again within its allowed age.` | `d` |
| asset flag `past-ttl` | `!`, word `active` | `overdue` | `Its last verification is older than its allowed age.` | `d` |
| flag `regression` | `!`, word `open` | `came back` | `Opened after the problem it concerns was marked fixed.` | `d` |
| flag `unverifiable` | mark `?` | `can't be checked here` | same | `d` |
| flag `unchecked` | mark `?` | `not checked yet` | same | `d` |
| flag `recorded` | word `recorded` | (no words) | `from a recorded check` | `d` |
| record status (`accepted`, `draft`, `in-progress`, …) | dim word | the same word (it is the record's own vocabulary) | | |

Relation words in KB lists (the row points at the record above it / is pointed at by it):

| `rel` | Row is the edge's `from` | Row is the edge's `to` |
|---|---|---|
| `concerns` | `about it` | `what it is about` |
| `depends-on` | `depends on it` | `what it depends on` |
| `references` | `refers to it` | `what it refers to` |
| `cites` | `cites it` | `what it cites` |
| `superseded-by` | `replaced by it` | `what replaced it` |
| `dossier-of` | `its dossier` | `what it is the dossier of` |
| `promoted-to` | `promoted to it` | `what it was promoted to` |
| `project` | `in project` | `its project` |
| any other word | the word, unchanged | the word, unchanged |

The origin in brackets (`(field)`, `(cite)`, `(…`) moves to `d`.

### 2.2 Surface strings

| Where | Today | Plain | Exact form lives in |
|---|---|---|---|
| Header, right | `run-20261101T090000Z-a1b2c3` | `run 1 of 3` | `d` (header), `?` This frame, `c` |
| Header | `subject kb` | nothing when a subject repository is found; `no repository: gates not judged` or `repository unreadable` (warning tone) otherwise | `d`, `?` This frame |
| Header chip | `current` | `current run` | |
| Tab badge, Gates | `!2 ○2` (also `×N`, `?N`) | `4 not met` (gates that are not satisfied, attested or waived; bad tone when any is stale, failed or invalid) | first line of Gates; `d` |
| Tab badge, Inbox | `3` | `3 pending` (`3` when the row is short) | first line of Inbox |
| Tab badge, Guards | `deny 1`, `ask 1` | `1 would block`, `1 would ask` (`1` when the row is short) | first line of Guards; `d` |
| Pane title | `step 7/11`, `decision 3/5`, `cite 6/9` | `step 7 of 11`, `decision 3 of 5`, `citation 6 of 9` | |
| Pane title | `gates run-20261101T090000Z-a1b2c3` (also timeline, inbox, resume, progress) | `gates` | `d` |
| Pane title | `guards (current run run-2026…)` | `guards · the current run` (`· not the selected run` when they differ) | `d` |
| Plan head | `1 stale` | `1 out of date` | `d` |
| Plan head | `gates not judged` | `gates not judged (no repository)` | `d` |
| Group header | `change#1` (exact: `change#1 @83c2cc9efde7 from .writfold/plans/change.toml`) | `change`; `change · 2nd time` from the second splice of a template on | `d`, inspector at exact (`splice:`) |
| Map row, current | `▶ check   !○ owes: review-list [stale], signoff …` | `▶ check     needs review-list and signoff` | `d` |
| Map row, done and stale | `! implement   ●!` | `! implement     done; lint is now out of date` | `d` |
| Map row, selected done | `✓ research    ●  recorded 2026-11-10T23:47:00Z` | `✓ research      done 2026-11-10 23:47` | `d` |
| Map row, other | `✓ read    ●`, `○ review   ··` | `✓ read`, `○ review` | `d` (chips) |
| Map row end | ` !out-of-order` | `done out of order` | `d` |
| Map facts | `last plan event  2026-11-10T23:59:50Z done change#1/re…` | `last recorded   2026-11-10 23:59 · prepare done` | `d`, Timeline at exact |
| Map caption | `▶ next step to pick up, as recorded` | `▶ the next step to pick up, as recorded` | |
| Inspector head | `check  current` + `change#1/review/review#1/findings/findings#1/check` | `check   next up` + `step 7 of 11 · in findings` | `d`, `c` |
| Inspector | `owes: review-list [stale], signoff [open]` | `Needs 2 gates before it can be marked done.` | `d` |
| Inspector | `gate review-list [stale] checklist 2/4 required covered` + `■ ! □ ◆ (+1 optional)` | `review-list   out of date` + `checklist · 2 of 4 required items covered` | `d` |
| Inspector | `! tests [stale] 1 reference by agent; cited evidence moved: gate:lint was satisfied, now stale` | `! tests   out of date · the evidence it cites has changed` | `d`, `g` at exact |
| Inspector | `□ docs [open] The guide names every new flag` | `□ docs    not covered · The guide names every new flag` | `d` |
| Inspector | `□ style [open] (optional) Names follow the module` | `□ style   optional · not covered · Names follow the module` | `d` |
| Inspector | `gate signoff [open] attested` | `signoff   nothing recorded yet` + `decided by a person's statement; no command is run` | `d` |
| Inspector | `read: instructions for signoff` | `Read first   instructions for signoff` | `c` (the file), `d` |
| Inspector | `next: writfold kb gate cover review-list tests supported REF… --by WHO` | `Suggested    give evidence for tests in review-list` | `c` (the command, whole), `d` |
| Inspector hint | `d receipts and covered items   g gates   x text` | `d exact wording   c commands   g gates` | |
| Gates, first line | `gates: gates=6 satisfied=2 open=2 stale=2 failed=0 unknown=0 attested=0 waived=0 invalid=0` | `6 gates · 2 passed · 2 out of date · 2 with nothing recorded yet · 4 not met` (non-zero counts only) | `d` |
| Gates row | `gate lint [stale] mechanical pass exit=0 at kb@0ea3a743bfef scope=a61da1b21345 paths=1 @runs/…/lint-….json:1 !stale` + `why: changed in scope: harbor/lint.txt` + `next: writfold kb gate run lint` | `! lint   out of date · changed in scope: harbor/lint.txt` | `d`; `c` for the command |
| Gates note | `# note: receipts of run run-… for kb; state is derived on every read, never stored` | `Standings are worked out from the run's recorded results each time this view asks; nothing is run.` | `d` |
| Inbox, first lines | `drafts=1 proposals=2` + `local; not in the KB until confirmed or accepted` | `3 pending · 1 draft · 2 proposals` + `Nothing here is in the KB until someone confirms or accepts it.` | `d` |
| Inbox row | `draft s-progress [open] Review started; … @/home/…/.git/writfold/drafts/s-progress.json !draft` + `confirm: writfold kb checkpoint --from-draft s-progress` | `draft      Review started; lint went stale after the rename.` + `confirming it records a checkpoint` | `d`; `c` |
| Inbox row | `proposal 2c6ba260f2cde3fa [pending] ticket HBR-210: … @/home/…json !proposal !local` + `field: created=… session=… run=…` + `accept: writfold kb proposals accept 2c6ba260f2cde3fa` | `proposal   ticket HBR-210: Lint gate scope misses the command module` + `pending · kept only on this machine` | `d`; `c` |
| Guards, first line | `rules=2 deny=1 ask=0 nudge=1 invalid=0` | `2 rules · 1 would block · 1 would remind` | `d` |
| Guards row | `guard commit-needs-gates [would-deny] pre-tool-use git commit requires gates.met @.writfold/guards/….toml:1 !deny` + `owes: gate lint [stale]; next: …` | `commit-needs-gates   would block    needs every gate of the run met` | `d`; `c` |
| Guards note | `# note: judged for the current run run-… in /home/… as if each rule had matched; a guard, not enforcement; nothing was run` | `Judged for the current run as if each rule applied right now. A guard advises; it does not enforce, and nothing was run.` | `d` |
| Timeline row | `plan change#1/implement [done] 2026-11-10T23:50:00Z done change#1/implement @runs/…/plan/done-change.1~implement.json:1` | `2026-11-10 23:50   step done      implement` | `d` |
| Timeline row | `checkpoint 2026-11-10T14:10:00Z entry 3 of 3: Review started. @runs/…/checkpoint.md:10` | `2026-11-10 14:10   checkpoint     entry 3 of 3: Review started.` | `d` |
| Timeline row | `plan change#1/review/review#1 [spliced] 2026-11-10T23:59:10Z spliced change#1/review/review#1 @…` | `2026-11-10 23:59   plan added     review` | `d` |
| Runs entry | `harbor  step 7/11`, `closed  harbor  complete` | unchanged (already plain) | `d` (five-line entry) |
| Footer | `kb 0ea3a743bfef+dirty:19` | `KB has 19 uncommitted changes`; nothing when the KB is clean | `d` (footer), `?` This frame |
| Footer | `index fresh` | nothing when fresh; `index STATE` in the warning tone otherwise | `d`, `?` |
| Footer | `asked 03:58:49Z` | nothing | `d` (footer), `?` This frame |
| KB header, right | `project:harbor` | `project 1 of 3` | `d`, `c` |
| KB tab badge | `5 stale !9` (and `?N`) | `5 stale 9 moved` (and `N unchecked`) | first line of stale at exact |
| KB list mark | `!` / `?` column, no legend | same column, with the pane caption `! cites a source that has moved` (`? has a source not checked`) when any row carries it | `?` Marks |
| KB why caption | `decisions that name harbor as their project` | `harbor's decisions` | |
| KB tracker word | `concerns`, `promoted-to` | the record's status word (`in-progress`, `done`); the relation moves to the inspector | `d`, inspector |
| KB tracker headers | `┬ project:harbor`, `┼ harbor-api` | the record's title when an answer on screen has it (`Harbor status`, `Harbor billing API`), else the id | `d` |
| KB trailer | `19 reached   not followed: 1 disputed, 22 inferred` | `19 records reached · not followed: 1 disputed link, 22 inferred links` | `d` |
| KB impact head | `in  what cites or links to project:harbor` | `what points at Harbor status` (`what Harbor status points at`, `everything linked to Harbor status`) | `d` |
| KB stale row | `· code:demo-service:src/batch.rs:2-4  !  moved` | `· src/batch.rs:2-4                        moved` | `d`; inspector names the repository |
| KB stale row | `● harbor-api  !  active` | `● harbor-api                              overdue` | `d` |
| KB inspector | `proposal/decision/2026-03-02-batch-ledger-writes` (id line) | `decision · decided by ops-review on 2026-03-09` | `d`, `c` |
| KB inspector | `why  Group ledger writes…` (label per line) | `Why` heading, then the lines | |
| KB inspector | `cites 2  ● !` + `! code:demo-service:src/batch.rs:2-4 [moved]` + `moved→4-6` | `Sources · 1 of 2 has moved` + `! src/batch.rs:2-4 in demo-service` + `moved to 4-6` | `d` |
| KB inspector | `linked from` + `– One writer process for the ledger  superseded-by (…` | `Linked from` + `– One writer process for the ledger   replaced by it` | `d` |
| KB inspector | `runs` | `Runs that name it` | |
| KB inspector | `status=in-progress`, `promoted_to=harbor` | `promoted to   harbor` (key, spaces for underscores; value whole; `status` omitted when the chip already shows it) | `d` |
| KB inspector | `…/decision/2026-03-02-batch-ledger-writes/proposal.md:5` | nothing | `d`, `c` |
| KB inspector hint | `d sources and fresh cites   f run   a impact` | `d exact wording and file   f run   a impact` | |
| KB status view | the `brief` answer as verb text | one row per record: glyph, title, `kind · status`, then its readable continuation lines | `d` |
| Any pane | `# note: …` | the note whole when it passes §5.1; otherwise `N notes in exact wording (d)` | `d` |
| Any pane | `refused (exit 3): …` | unchanged: a refusal is shown whole at both levels | |
| Help | Marks, This frame, keys, verbs | adds **Words**: every plain phrase beside its exact word, printed from the §2.1 tables | |

Times: a recorded stamp `2026-11-10T23:59:50Z` prints as `2026-11-10 23:59`, a date stays a
date. This is a change of spelling only (no time zone conversion, no relative time, no
clock); Timeline's caption says `Times are UTC, as recorded.`

## 3. Frames

Blank pane rows are trimmed in every frame. The selected row is drawn reversed, which
plain text cannot show.

### 3.1 Run mode, progress map and inspector, 136×38 (the homepage frame)

Before:

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
  1 progress    2 gates !2 ○2    3 timeline    4 inbox 3    5 guards deny 1    6 stale    7 brief    8 resume                      m kb
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
│                    ││last plan event  2026-11-10T23:59:50Z done change#1/re…││                                                       │
│                    ││▶ next step to pick up, as recorded                    ││d receipts and covered items   g gates   x text        │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                j/k step   l inspect   d detail   ? help   q quit
```

After:

```text
 Add --plan to kb runs   open   current run   harbor                                                                          run 1 of 3
  1 progress    2 gates 4 not met    3 timeline    4 inbox 3 pending    5 guards 1 would block    6 stale    7 brief    8 resume    m kb
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
│                    ││ ┴                                                     ││d exact wording   c commands   g gates                 │
│                    ││                                                       ││                                                       │
│                    ││last recorded   2026-11-10 23:59 · prepare done        ││                                                       │
│                    ││▶ the next step to pick up, as recorded                ││                                                       │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                            j/k step   l inspect   d detail   ? help   q quit
```

Reading the after frame top-down answers the five questions: the run and its project
(header), step 7 of 11 (plan head), `check` is next (the marker and its caption), it needs
two gates and why each is not met (inspector), and the last recorded event (below the
rail). No row carries a bracket, a stamp with seconds, a hash or a command.

### 3.2 Run mode, 80×24

Map, before and after:

```text
 Add --plan to kb runs   open   current   harbor   subject kb           run 1/3
  1 progress    2 gates !2 ○2    4 inbox 3    5 guards deny 1       m kb    1/8
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

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  1 progress   2 gates 4 not met   4 inbox 3   5 guards 1             m kb   1/8
╭ plan ────────────────────────────────────────────────────────────────────────╮
│step 7 of 11    5 done  1 skipped  1 out of date                              │
│ ┬ change                                                                     │
│ ✓ research                                                                   │
│ ! implement     done; lint is now out of date                                │
│ – benchmarks                                                                 │
│ ├─╮ review                                                                   │
│ │ ├─╮ notes                                                                  │
│ │ │ ✓ read                                                                   │
│ │ ├─╯                                                                        │
│ │ ✓ read-diff                                                                │
│ │ ├─╮ findings                                                               │
│ │ │ ✓ prepare                                                                │
│ │ │ ▶ check     needs review-list and signoff                                │
│ │ ├─╯                                                                        │
│ │ ○ findings                                                                 │
│ ├─╯                                                                          │
│ ○ review                                                                     │
│last recorded   2026-11-10 23:59 · prepare done                               │
│▶ the next step to pick up, as recorded                                       │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                               l inspect   d detail   ? help   q quit
```

Inspector (`l`), before and after:

```text
 Add --plan to kb runs   open   current   harbor   subject kb           run 1/3
  1 progress    2 gates !2 ○2    4 inbox 3    5 guards deny 1       m kb    1/8
╭ step 7/11 ───────────────────────────────────────────────────────────────────╮
│check  current                                                                │
│change#1/review/review#1/findings/findings#1/check                            │
│owes: review-list [stale], signoff [open]                                     │
│gate review-list [stale] checklist 2/4 required covered                       │
│  ■ ! □ ◆ (+1 optional)                                                       │
│  ! tests [stale] 1 reference by agent; cited evidence moved: gate:lint was   │
│  satisfied, now stale                                                        │
│  □ docs [open] The guide names every new flag                                │
│  □ style [open] (optional) Names follow the module                           │
│gate signoff [open] attested                                                  │
│read: instructions for signoff                                                │
│next: writfold kb gate cover review-list tests supported REF… --by WHO        │
│d receipts and covered items   g gates   x text                               │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19      h back   d detail   ? help   q quit
```

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  1 progress   2 gates 4 not met   4 inbox 3   5 guards 1             m kb   1/8
╭ step 7 of 11 ────────────────────────────────────────────────────────────────╮
│check   next up                                                               │
│step 7 of 11 · in findings                                                    │
│                                                                              │
│Needs 2 gates before it can be marked done.                                   │
│                                                                              │
│review-list   out of date                                                     │
│  checklist · 2 of 4 required items covered                                   │
│  ! tests   out of date · the evidence it cites has changed                   │
│  □ docs    not covered · The guide names every new flag                      │
│  □ style   optional · not covered · Names follow the module                  │
│                                                                              │
│signoff   nothing recorded yet                                                │
│  decided by a person's statement; no command is run                          │
│                                                                              │
│Read first   instructions for signoff                                         │
│Suggested    give evidence for tests in review-list                           │
│                                                                              │
│d exact wording   c commands   g gates                                        │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                  h back   d detail   ? help   q quit
```

### 3.3 Gates

Before (136×38):

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
 1 progress    2 gates  !2 ○2    3 timeline    4 inbox 3    5 guards deny 1    6 stale    7 brief    8 resume                      m kb
╭ runs (3) ──────────────────╮╭ gates run-20261101T090000Z-a1b2c3 ─────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││gates: gates=6 satisfied=2 open=2 stale=2 failed=0 unknown=0 attested=0 waived=0 invalid=0              │
│  harbor  step 7/11         ││gate bench [open] mechanical no receipt in this run for kb @.writfold/gates/bench.toml:1 !open          │
│  Review the lantern guide  ││  next: writfold kb gate run bench                                                                      │
│  lantern                   ││gate lint [stale] mechanical pass exit=0 at kb@0ea3a743bfef scope=a61da1b21345 paths=1                  │
│  Retire the legacy flush p…││@runs/run-20261101T090000Z-a1b2c3/gates/lint-20261110T234900Z-2ad843.json:1 !stale                      │
│  closed  harbor  complete  ││  why: changed in scope: harbor/lint.txt                                                                │
│                            ││  next: writfold kb gate run lint                                                                       │
│                            ││gate notes [satisfied] mechanical pass exit=0 at kb@0ea3a743bfef scope=1075e30643f2 paths=10            │
│                            ││@runs/run-20261101T090000Z-a1b2c3/gates/notes-20261110T234600Z-0444e6.json:1                            │
│                            ││gate signoff [open] attested: statements only, no command, for kb @.writfold/gates/signoff.toml:1 !open │
│                            ││  read: instructions for signoff @.writfold/gates/signoff.md:1                                          │
│                            ││  next: writfold kb gate attest signoff --statement TEXT --by WHO                                       │
│                            ││gate unit [satisfied] mechanical pass exit=0 at kb@0ea3a743bfef scope=b7e787e397de paths=1              │
│                            ││@runs/run-20261101T090000Z-a1b2c3/gates/unit-20261110T234800Z-a1fb19.json:1                             │
│                            ││gate review-list [stale] checklist 2/4 required covered at kb@0ea3a743bfef scope=b7e787e397de           │
│                            ││@.writfold/gates/review-list.toml:1 !stale                                                              │
│                            ││  item: errors [supported] 1 reference by agent                                                         │
│                            ││  @runs/run-20261101T090000Z-a1b2c3/covers/review-list.errors-20261110T235200Z-1c1c15.json:1            │
│                            ││  item: tests [stale] 1 reference by agent; cited evidence moved: gate:lint was satisfied, now stale    │
│                            ││  @runs/run-20261101T090000Z-a1b2c3/covers/review-list.tests-20261110T235300Z-fd2f8a.json:1 !stale      │
│                            ││  item: docs [open] The guide names every new flag                                                      │
│                            ││  item: perf [not-applicable] no hot path touched by agent                                              │
│                            ││  @runs/run-20261101T090000Z-a1b2c3/covers/review-list.perf-20261110T235400Z-b95939.json:1              │
│                            ││  item: style [open] (optional) Names follow the module                                                 │
│                            ││  next: writfold kb gate cover review-list tests supported REF… --by WHO                                │
│                            ││# note: receipts of run run-20261101T090000Z-a1b2c3 for kb; state is derived on every read, never stored│
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                              j/k run   l gates   ? help   q quit
```

After (136×38):

```text
 Add --plan to kb runs   open   current run   harbor                                                                          run 1 of 3
  1 progress    2 gates 4 not met    3 timeline    4 inbox 3 pending    5 guards 1 would block    6 stale    7 brief    8 resume    m kb
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
│                            ││                                                                                                        │
│                            ││Standings are worked out from the run's recorded results each time this view asks; nothing is run.      │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                           j/k scroll   Tab view   d detail   ? help   q quit
```

Before and after (80×24):

```text
 Add --plan to kb runs   open   current   harbor   subject kb           run 1/3
  2 gates  !2 ○2    4 inbox 3    5 guards deny 1                    m kb    2/8
╭ gates run-20261101T090000Z-a1b2c3 ───────────────────────────────────────────╮
│gates: gates=6 satisfied=2 open=2 stale=2 failed=0 unknown=0 attested=0       │
│waived=0 invalid=0                                                            │
│gate bench [open] mechanical no receipt in this run for kb                    │
│@.writfold/gates/bench.toml:1 !open                                           │
│  next: writfold kb gate run bench                                            │
│gate lint [stale] mechanical pass exit=0 at kb@0ea3a743bfef scope=a61da1b21345│
│paths=1                                                                       │
│@runs/run-20261101T090000Z-a1b2c3/gates/lint-20261110T234900Z-2ad843.json:1   │
│!stale                                                                        │
│  why: changed in scope: harbor/lint.txt                                      │
│  next: writfold kb gate run lint                                             │
│gate notes [satisfied] mechanical pass exit=0 at kb@0ea3a743bfef              │
│scope=1075e30643f2 paths=10                                                   │
│@runs/run-20261101T090000Z-a1b2c3/gates/notes-20261110T234600Z-0444e6.json:1  │
│gate signoff [open] attested: statements only, no command, for kb             │
│@.writfold/gates/signoff.toml:1 !open                                         │
│  read: instructions for signoff @.writfold/gates/signoff.md:1                │
│  next: writfold kb gate attest signoff --statement TEXT --by WHO             │
│gate unit [satisfied] mechanical pass exit=0 at kb@0ea3a743bfef               │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19               Tab view   ? help   q quit
```

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  2 gates 4 not met   4 inbox 3   5 guards 1                          m kb   2/8
╭ gates ───────────────────────────────────────────────────────────────────────╮
│6 gates · 2 passed · 2 out of date · 2 with nothing recorded yet · 4 not met  │
│                                                                              │
│○ bench         nothing recorded yet · checked by a command                   │
│! lint          out of date · changed in scope: harbor/lint.txt               │
│● notes         passed                                                        │
│○ signoff       nothing recorded yet · decided by a person's statement; no    │
│                command is run                                                │
│● unit          passed                                                        │
│! review-list   out of date · checklist · 2 of 4 required items covered       │
│    ■ errors    covered · 1 reference by agent                                │
│    ! tests     out of date · the evidence it cites has changed               │
│    □ docs      not covered · The guide names every new flag                  │
│    ◆ perf      marked not applicable · no hot path touched by agent          │
│    □ style     optional · not covered · Names follow the module              │
│                                                                              │
│Standings are worked out from the run's recorded results each time this view  │
│asks; nothing is run.                                                         │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                Tab view   d detail   ? help   q quit
```

Rows keep the verb's order. A covered item's account (`1 reference by agent`) is shown
because it reads as a sentence; the stale item's account names `gate:lint`, so the state
phrase stands in for it and `d` has the sentence.

### 3.4 Inbox

Before (136×38):

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
 1 progress    2 gates !2 ○2    3 timeline    4 inbox  3    5 guards deny 1    6 stale    7 brief    8 resume                      m kb
╭ runs (3) ──────────────────╮╭ inbox run-20261101T090000Z-a1b2c3 ─────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││drafts=1 proposals=2                                                                                    │
│  harbor  step 7/11         ││local; not in the KB until confirmed or accepted                                                        │
│  Review the lantern guide  ││draft s-progress [open] Review started; lint went stale after the rename.                               │
│  lantern                   ││@/home/dev/workspace/writfold/target/capture-world--/kb/.git/writfold/drafts/s-progress.json !draft     │
│  Retire the legacy flush p…││  confirm: writfold kb checkpoint --from-draft s-progress                                               │
│  closed  harbor  complete  ││proposal 2c6ba260f2cde3fa [pending] ticket HBR-210: Lint gate scope misses the command module           │
│                            ││@/home/dev/workspace/writfold/target/capture-world--/kb/.git/writfold/proposals/2c6ba260f2cde3fa.json   │
│                            ││!proposal !local                                                                                        │
│                            ││  field: created=2026-11-10T23:59:00Z session=s-progress run=run-20261101T090000Z-a1b2c3                │
│                            ││  accept: writfold kb proposals accept 2c6ba260f2cde3fa                                                 │
│                            ││proposal 2c9a051ffe807243 [pending] link mentions run-20261101T090000Z-a1b2c3 →                         │
│                            ││proposal/decision/2026-03-02-batch-ledger-writes                                                        │
│                            ││@/home/dev/workspace/writfold/target/capture-world--/kb/.git/writfold/proposals/2c9a051ffe807243.json   │
│                            ││!proposal !local                                                                                        │
│                            ││  field: created=2026-11-10T23:59:00Z session=s-progress                                                │
│                            ││  accept: writfold kb proposals accept 2c9a051ffe807243                                                 │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                              j/k run   l inbox   ? help   q quit
```

After (136×38, then 80×24):

```text
 Add --plan to kb runs   open   current run   harbor                                                                          run 1 of 3
  1 progress    2 gates 4 not met    3 timeline    4 inbox 3 pending    5 guards 1 would block    6 stale    7 brief    8 resume    m kb
╭ runs (3) ──────────────────╮╭ inbox ─────────────────────────────────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││3 pending · 1 draft · 2 proposals                                                                       │
│  harbor  step 7/11         ││Nothing here is in the KB until someone confirms or accepts it.                                         │
│  Review the lantern guide  ││                                                                                                        │
│  lantern                   ││draft      Review started; lint went stale after the rename.                                            │
│  Retire the legacy flush p…││           confirming it records a checkpoint                                                           │
│  closed  harbor  complete  ││proposal   ticket HBR-210: Lint gate scope misses the command module                                    │
│                            ││           pending · kept only on this machine                                                          │
│                            ││proposal   pending · kept only on this machine                                                          │
│                            ││           its text names records by id; d shows it                                                     │
│                            ││                                                                                                        │
│                            ││c lists the confirm and accept commands; watch never runs them.                                         │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                           j/k scroll   Tab view   d detail   ? help   q quit
```

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  2 gates 4 not met   4 inbox 3   5 guards 1                          m kb   4/8
╭ inbox ───────────────────────────────────────────────────────────────────────╮
│3 pending · 1 draft · 2 proposals                                             │
│Nothing here is in the KB until someone confirms or accepts it.               │
│                                                                              │
│draft      Review started; lint went stale after the rename.                  │
│           confirming it records a checkpoint                                 │
│proposal   ticket HBR-210: Lint gate scope misses the command module          │
│           pending · kept only on this machine                                │
│proposal   pending · kept only on this machine                                │
│           its text names records by id; d shows it                           │
│                                                                              │
│c lists the confirm and accept commands; watch never runs them.               │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                Tab view   d detail   ? help   q quit
```

The third row is the honest worst case: the proposal's text is built from two record ids,
so it is withheld and the row says where it is.

### 3.5 Guards

Before (136×38):

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
 1 progress    2 gates !2 ○2    3 timeline    4 inbox 3    5 guards  deny 1    6 stale    7 brief    8 resume                      m kb
╭ runs (3) ──────────────────╮╭ guards (current run run-20261101T090000Z-a1b2c3) ──────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││rules=2 deny=1 ask=0 nudge=1 invalid=0                                                                  │
│  harbor  step 7/11         ││guard commit-needs-gates [would-deny] pre-tool-use git commit requires gates.met                        │
│  Review the lantern guide  ││@.writfold/guards/commit-needs-gates.toml:1 !deny                                                       │
│  lantern                   ││  owes: gate lint [stale]; next: writfold kb gate run lint; gate bench [open]; next: writfold kb gate   │
│  Retire the legacy flush p…││  run bench; gate review-list [stale]; next: writfold kb gate cover review-list tests supported REF…    │
│  closed  harbor  complete  ││  --by WHO; gate signoff [open]; next: writfold kb gate attest signoff --statement TEXT --by WHO        │
│                            ││guard stop-reflect [would-nudge] stop requires step.met @.writfold/guards/stop-reflect.toml:1 !nudge    │
│                            ││  owes: step check owes review-list [stale], signoff [open]; next: writfold kb gate cover review-list   │
│                            ││  tests supported REF… --by WHO                                                                         │
│                            ││# note: judged for the current run run-20261101T090000Z-a1b2c3 in                                       │
│                            ││/home/dev/workspace/writfold/target/capture-world--/kb as if each rule had matched; a guard, not        │
│                            ││enforcement; nothing was run                                                                            │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                             j/k run   l guards   ? help   q quit
```

After (136×38, then 80×24):

```text
 Add --plan to kb runs   open   current run   harbor                                                                          run 1 of 3
  1 progress    2 gates 4 not met    3 timeline    4 inbox 3 pending    5 guards 1 would block    6 stale    7 brief    8 resume    m kb
╭ runs (3) ──────────────────╮╭ guards · the current run ──────────────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││2 rules · 1 would block · 1 would remind                                                                │
│  harbor  step 7/11         ││                                                                                                        │
│  Review the lantern guide  ││commit-needs-gates   would block    needs every gate of the run met                                     │
│  lantern                   ││stop-reflect         would remind   needs the current step's gates met                                  │
│  Retire the legacy flush p…││                                                                                                        │
│  closed  harbor  complete  ││Judged for the current run as if each rule applied right now. A guard advises; it does not enforce,     │
│                            ││and nothing was run.                                                                                    │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                           j/k scroll   Tab view   d detail   ? help   q quit
```

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  2 gates 4 not met   4 inbox 3   5 guards 1                          m kb   5/8
╭ guards · the current run ────────────────────────────────────────────────────╮
│2 rules · 1 would block · 1 would remind                                      │
│                                                                              │
│commit-needs-gates   would block    needs every gate of the run met           │
│stop-reflect         would remind   needs the current step's gates met        │
│                                                                              │
│Judged for the current run as if each rule applied right now. A guard advises;│
│it does not enforce, and nothing was run.                                     │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                Tab view   d detail   ? help   q quit
```

### 3.6 Timeline

Before (136×38, first rows):

```text
 Add --plan to kb runs   open   current   harbor   subject kb                                               run-20261101T090000Z-a1b2c3
 1 progress    2 gates !2 ○2    3 timeline    4 inbox 3    5 guards deny 1    6 stale    7 brief    8 resume                       m kb
╭ runs (3) ──────────────────╮╭ timeline run-20261101T090000Z-a1b2c3 ──────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││decision proposal/decision/2026-03-02-batch-ledger-writes [accepted] 2026-03-02 created Batch ledger    │
│  harbor  step 7/11         ││writes in the API @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:6                      │
│  Review the lantern guide  ││decision proposal/decision/2026-03-02-batch-ledger-writes [accepted] 2026-03-09 updated Batch ledger    │
│  lantern                   ││writes in the API @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:7                      │
│  Retire the legacy flush p…││run run-20261101T090000Z-a1b2c3 [open] 2026-11-01T09:00:00Z created Add --plan to kb runs               │
│  closed  harbor  complete  ││@runs/run-20261101T090000Z-a1b2c3/manifest.json:1                                                       │
│                            ││checkpoint 2026-11-10T09:00:00Z entry 1 of 3: Started; read the runs verb.                              │
│                            ││@runs/run-20261101T090000Z-a1b2c3/checkpoint.md:2                                                       │
│                            ││checkpoint 2026-11-10T12:45:00Z entry 2 of 3: Implemented --plan; see                                   │
│                            ││proposal/decision/2026-03-02-batch-ledger-writes. @runs/run-20261101T090000Z-a1b2c3/checkpoint.md:6     │
│                            ││checkpoint 2026-11-10T14:10:00Z entry 3 of 3: Review started.                                           │
│                            ││@runs/run-20261101T090000Z-a1b2c3/checkpoint.md:10                                                      │
│                            ││plan change#1 [spliced] 2026-11-10T23:45:00Z spliced change#1                                           │
│                            ││@runs/run-20261101T090000Z-a1b2c3/plan/splice-change.1.json:1                                           │
│ … (19 more rows of the same shape)
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 0ea3a743bfef+dirty:19   index fresh   asked 00:21:18Z                           j/k run   l timeline   ? help   q quit
```

After (136×38, then 80×24):

```text
 Add --plan to kb runs   open   current run   harbor                                                                          run 1 of 3
  1 progress    2 gates 4 not met    3 timeline    4 inbox 3 pending    5 guards 1 would block    6 stale    7 brief    8 resume    m kb
╭ runs (3) ──────────────────╮╭ timeline ──────────────────────────────────────────────────────────────────────────────────────────────╮
│> Add --plan to kb runs     ││2026-03-02         decision       created Batch ledger writes in the API                                │
│  harbor  step 7/11         ││2026-03-09         decision       updated Batch ledger writes in the API                                │
│  Review the lantern guide  ││2026-11-01 09:00   run            created Add --plan to kb runs                                         │
│  lantern                   ││2026-11-10 09:00   checkpoint     entry 1 of 3: Started; read the runs verb.                            │
│  Retire the legacy flush p…││2026-11-10 12:45   checkpoint     entry 2 of 3: Implemented --plan; see                                 │
│  closed  harbor  complete  ││                                  proposal/decision/2026-03-02-batch-ledger-writes.                     │
│                            ││2026-11-10 14:10   checkpoint     entry 3 of 3: Review started.                                         │
│                            ││2026-11-10 23:45   plan added     change                                                                │
│                            ││2026-11-10 23:47   step done      research                                                              │
│                            ││2026-11-10 23:50   step done      implement                                                             │
│                            ││2026-11-10 23:51   step skipped   benchmarks · (by owner) no hot path touched                           │
│                            ││2026-11-10 23:59   plan added     review                                                                │
│                            ││2026-11-10 23:59   step done      read                                                                  │
│                            ││2026-11-10 23:59   step done      read-diff                                                             │
│                            ││2026-11-10 23:59   plan added     findings                                                              │
│                            ││2026-11-10 23:59   step done      prepare                                                               │
│                            ││                                                                                                        │
│                            ││Times are UTC, as recorded.                                                                             │
╰────────────────────────────╯╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  read-only   KB has 19 uncommitted changes                                           j/k scroll   Tab view   d detail   ? help   q quit
```

```text
 Add --plan to kb runs   open   current run   harbor                  run 1 of 3
  2 gates 4 not met   3 timeline   4 inbox 3   5 guards 1             m kb   3/8
╭ timeline ────────────────────────────────────────────────────────────────────╮
│2026-03-02         decision       created Batch ledger writes in the API      │
│2026-03-09         decision       updated Batch ledger writes in the API      │
│2026-11-01 09:00   run            created Add --plan to kb runs               │
│2026-11-10 09:00   checkpoint     entry 1 of 3: Started; read the runs verb.  │
│2026-11-10 12:45   checkpoint     entry 2 of 3: Implemented --plan; see       │
│                                  proposal/decision/2026-03-02-batch-ledger-wr│
│                                  ites.                                       │
│2026-11-10 14:10   checkpoint     entry 3 of 3: Review started.               │
│2026-11-10 23:45   plan added     change                                      │
│2026-11-10 23:47   step done      research                                    │
│2026-11-10 23:50   step done      implement                                   │
│2026-11-10 23:51   step skipped   benchmarks · (by owner) no hot path touched │
│2026-11-10 23:59   plan added     review                                      │
│2026-11-10 23:59   step done      read                                        │
│2026-11-10 23:59   step done      read-diff                                   │
│2026-11-10 23:59   plan added     findings                                    │
│2026-11-10 23:59   step done      prepare                                     │
│                                                                              │
│Times are UTC, as recorded.                                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                Tab view   d detail   ? help   q quit
```

A step is named as the map names it when the plan answer has that id; a splice is named by
the template of the group whose splice file is the row's `path`. Otherwise the id is
printed, clipped from the left. Authored checkpoint text is never altered, so the record id
in the second checkpoint stays.

### 3.7 KB mode, why and its inspector

Before (136×38):

```text
 Harbor status   build   knowledge base                                                                                  project:harbor
  1 why    2 status    3 tracker    4 impact    5 stale !9                                                                       m runs
╭ projects (2) ──────╮╭ why (5) ──────────────────────────────────────────────╮╭ decision 3/5 ─────────────────────────────────────────╮
│> Harbor status     ││ ┬ decisions                                           ││Batch ledger writes in the API   accepted              │
│  build  harbor     ││ – One writer process for the ledger    superseded     ││proposal/decision/2026-03-02-batch-ledger-writes       │
│  Lantern console   ││ – Drop client retries on flush         rejected       ││                                                       │
│  lantern           ││ ● Batch ledger writes in the API    !  accepted       ││decided by ops-review on 2026-03-09                    │
│  all records       ││ ○ Shard the ledger by tenant           draft          ││                                                       │
│  by kind           ││ ○ Cap replay batches at 500 rows    !  draft          ││why  Group ledger writes into batches of up to 500 rows│
│                    ││ ┴                                                     ││     per transaction.                                  │
│                    ││                                                       ││why  Batching keeps the flush under the store's fsync  │
│                    ││                                                       ││     budget.                                           │
│                    ││                                                       ││cites 2  ● !                                           │
│                    ││                                                       ││  ! code:demo-service:src/batch.rs:2-4 [moved]         │
│                    ││                                                       ││    moved→4-6                                          │
│                    ││                                                       ││see also                                               │
│                    ││                                                       ││  ● Flush stalls on the ledger store  concluded        │
│                    ││                                                       ││  ● Ledger flush stalls on slow disk  done             │
│                    ││                                                       ││linked from                                            │
│                    ││                                                       ││  – One writer process for the ledger  superseded-by (…│
│                    ││                                                       ││runs                                                   │
│                    ││                                                       ││  Add --plan to kb runs                                │
│                    ││                                                       ││…/decision/2026-03-02-batch-ledger-writes/proposal.md:5│
│                    ││decisions that name harbor as their project            ││d sources and fresh cites   f run   a impact           │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780   index fresh   asked 00:21:18Z                       j/k record   l inspect   a impact   ? help   q quit
```

After (136×38):

```text
 Harbor status   build   knowledge base                                                                                   project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                   m runs
╭ projects (2) ──────╮╭ why (5) ──────────────────────────────────────────────╮╭ decision 3 of 5 ──────────────────────────────────────╮
│> Harbor status     ││ ┬ decisions                                           ││Batch ledger writes in the API   accepted              │
│  build  harbor     ││ – One writer process for the ledger    superseded     ││decision · decided by ops-review on 2026-03-09         │
│  Lantern console   ││ – Drop client retries on flush         rejected       ││                                                       │
│  lantern           ││ ● Batch ledger writes in the API    !  accepted       ││Why                                                    │
│  all records       ││ ○ Shard the ledger by tenant           draft          ││  Group ledger writes into batches of up to 500 rows   │
│  by kind           ││ ○ Cap replay batches at 500 rows    !  draft          ││  per transaction.                                     │
│                    ││ ┴                                                     ││  Batching keeps the flush under the store's fsync     │
│                    ││                                                       ││  budget.                                              │
│                    ││harbor's decisions   ! cites a source that has moved   ││                                                       │
│                    ││                                                       ││Sources · 1 of 2 has moved                             │
│                    ││                                                       ││  ! src/batch.rs:2-4 in demo-service                   │
│                    ││                                                       ││    moved to 4-6                                       │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││See also                                               │
│                    ││                                                       ││  ● Flush stalls on the ledger store   concluded       │
│                    ││                                                       ││  ● Ledger flush stalls on slow disk   done            │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││Linked from                                            │
│                    ││                                                       ││  – One writer process for the ledger   replaced by it │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││Runs that name it                                      │
│                    ││                                                       ││  Add --plan to kb runs                                │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││d exact wording and file   f run   a impact            │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                          j/k record   l inspect   a impact   ? help   q quit
```

Before and after (80×24, list), then the inspector after (`l`):

```text
 Harbor status   build   knowledge base                             project 1/3
  1 why    5 stale !9                                             m runs    1/5
╭ why (5) ─────────────────────────────────────────────────────────────────────╮
│ ┬ decisions                                                                  │
│ – One writer process for the ledger    superseded                            │
│ – Drop client retries on flush         rejected                              │
│ ● Batch ledger writes in the API    !  accepted                              │
│ ○ Shard the ledger by tenant           draft                                 │
│ ○ Cap replay batches at 500 rows    !  draft                                 │
│ ┴                                                                            │
│decisions that name harbor as their project                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780            l inspect   a impact   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  1 why   5 stale 9 moved                                           m runs   1/5
╭ why (5) ─────────────────────────────────────────────────────────────────────╮
│ ┬ decisions                                                                  │
│ – One writer process for the ledger    superseded                            │
│ – Drop client retries on flush         rejected                              │
│ ● Batch ledger writes in the API    !  accepted                              │
│ ○ Shard the ledger by tenant           draft                                 │
│ ○ Cap replay batches at 500 rows    !  draft                                 │
│ ┴                                                                            │
│                                                                              │
│harbor's decisions   ! cites a source that has moved                          │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                               l inspect   a impact   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  1 why   5 stale 9 moved                                           m runs   1/5
╭ decision 3 of 5 ─────────────────────────────────────────────────────────────╮
│Batch ledger writes in the API   accepted                                     │
│decision · decided by ops-review on 2026-03-09                                │
│Why                                                                           │
│  Group ledger writes into batches of up to 500 rows per transaction.         │
│  Batching keeps the flush under the store's fsync budget.                    │
│Sources · 1 of 2 has moved                                                    │
│  ! src/batch.rs:2-4 in demo-service                                          │
│    moved to 4-6                                                              │
│See also                                                                      │
│  ● Flush stalls on the ledger store   concluded                              │
│  ● Ledger flush stalls on slow disk   done                                   │
│Linked from                                                                   │
│  – One writer process for the ledger   replaced by it                        │
│Runs that name it                                                             │
│  Add --plan to kb runs                                                       │
│d exact wording and file   f run   a impact                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                  h back   d detail   ? help   q quit
```

(At 80×24 the inspector drops the blank lines between sections before it scrolls.)

### 3.8 KB mode, status

Before (136×38, first rows):

```text
 Harbor status   build   knowledge base                                                                                  project:harbor
 1 why    2 status    3 tracker    4 impact    5 stale !9                                                                        m runs
╭ projects (2) ──────╮╭ status harbor ────────────────────────────────────────╮╭ project ──────────────────────────────────────────────╮
│> Harbor status     ││project project:harbor [build] Harbor status           ││Harbor status   build                                  │
│  build  harbor     ││@harbor/STATUS.md:2                                    ││project:harbor                                         │
│  Lantern console   ││  field: next=month-end replay                         ││                                                       │
│  lantern           ││  field: owner=team-quay                               ││next=month-end replay                                  │
│  all records       ││run run-20261104T080000Z-d4e5f6 [open] Retire the      ││owner=team-quay                                        │
│  by kind           ││legacy flush path (checkpoint 2026-11-04T09:00:00Z)    ││phase=build                                            │
│                    ││@runs/run-20261104T080000Z-d4e5f6/checkpoint.md:2      ││                                                       │
│                    ││  note: Legacy path removed; nothing left to do.       ││linked from                                            │
│                    ││  @runs/run-20261104T080000Z-d4e5f6/checkpoint.md:4    ││  ○ Slow ledger flush at month end  promoted-to (field)│
│                    ││ticket HBR-102 [in-progress] Replay too slow           ││  ● Harbor billing API  project (field)                │
│                    ││@ktlo/2026/hbr-102-replay-too-slow.md:5                ││  ● Harbor ledger store  project (field)               │
│                    ││decision                                               ││  – One writer process for the ledger  project (field) │
│                    ││proposal/decision/2026-11-11-cap-replay-batches-at-500-││  – Drop client retries on flush  project (field)      │
│                    ││rows [draft] Cap replay batches at 500 rows            ││  ● Batch ledger writes in the API  project (field)    │
│                    ││@proposals/decision/2026-11-11-cap-replay-batches-at-50││  ○ Shard the ledger by tenant  project (field)        │
│                    ││0-rows/proposal.md:6                                   ││  ○ Cap replay batches at 500 rows  project (field)    │
│ … (17 more rows of the same shape)
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780   index fresh   asked 00:21:18Z                                   j/k scroll   Tab view   ? help   q quit
```

After (136×38, then 80×24):

```text
 Harbor status   build   knowledge base                                                                                   project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                   m runs
╭ projects (2) ──────╮╭ status ───────────────────────────────────────────────╮╭ project ──────────────────────────────────────────────╮
│> Harbor status     ││Harbor status   build                                  ││Harbor status   build                                  │
│  build  harbor     ││  next    month-end replay                             ││project                                                │
│  Lantern console   ││  owner   team-quay                                    ││                                                       │
│  lantern           ││                                                       ││next    month-end replay                               │
│  all records       ││○ Retire the legacy flush path   run · open            ││owner   team-quay                                      │
│  by kind           ││    Legacy path removed; nothing left to do.           ││phase   build                                          │
│                    ││○ Replay too slow   ticket · in-progress               ││                                                       │
│                    ││○ Cap replay batches at 500 rows   decision · draft    ││Linked from                                            │
│                    ││○ Slow ledger flush at month end   ticket · in-progress││  ○ Slow ledger flush at month end   promoted to it    │
│                    ││○ Flush stalls return   investigation · open           ││  ● Harbor billing API   in project                    │
│                    ││○ Shard the ledger by tenant   decision · draft        ││  ● Harbor ledger store   in project                   │
│                    ││● Batch ledger writes in the API   decision · accepted ││  – One writer process for the ledger   in project     │
│                    ││    Group ledger writes into batches of up to 500 rows ││  – Drop client retries on flush   in project          │
│                    ││    per transaction.                                   ││  ● Batch ledger writes in the API   in project        │
│                    ││· 2026-11-05 progress fixture   commit                 ││  ○ Shard the ledger by tenant   in project            │
│                    ││                                                       ││  ○ Cap replay batches at 500 rows   in project        │
│                    ││1 note in exact wording (d)                            ││  · Quay batch worker   in project                     │
│                    ││                                                       ││  ○ Add --plan to kb runs   in project                 │
│                    ││                                                       ││  ○ Retire the legacy flush path   in project          │
│                    ││                                                       ││  ● Tide latency dashboard   in project                │
│                    ││                                                       ││                                                       │
│                    ││                                                       ││d exact wording and file   f run   a impact            │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                           j/k scroll   Tab view   d detail   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  2 status   5 stale 9 moved                                        m runs   2/5
╭ status ──────────────────────────────────────────────────────────────────────╮
│Harbor status   build                                                         │
│  next    month-end replay                                                    │
│  owner   team-quay                                                           │
│                                                                              │
│○ Retire the legacy flush path   run · open                                   │
│    Legacy path removed; nothing left to do.                                  │
│○ Replay too slow   ticket · in-progress                                      │
│○ Cap replay batches at 500 rows   decision · draft                           │
│○ Slow ledger flush at month end   ticket · in-progress                       │
│○ Flush stalls return   investigation · open                                  │
│○ Shard the ledger by tenant   decision · draft                               │
│● Batch ledger writes in the API   decision · accepted                        │
│    Group ledger writes into batches of up to 500 rows per transaction.       │
│· 2026-11-05 progress fixture   commit                                        │
│                                                                              │
│1 note in exact wording (d)                                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                Tab view   d detail   ? help   q quit
```

A run's row takes its title from the runs list by id; the verb's note names a command, so
it is counted, not printed.

### 3.9 KB mode, tracker

Before (136×38):

```text
 Harbor status   build   knowledge base                                                                                  project:harbor
 1 why    2 status    3 tracker    4 impact    5 stale !9                                                                        m runs
╭ projects (2) ──────╮╭ tracker (6) ──────────────────────────────────────────╮╭ ticket 1/6 ───────────────────────────────────────────╮
│> Harbor status     ││ ┬ project:harbor                                      ││Slow ledger flush at month end   in-progress           │
│  build  harbor     ││ ○ Slow ledger flush at month end      promoted-to     ││HBR-101                                                │
│  Lantern console   ││ ┼ harbor-api                                          ││                                                       │
│  lantern           ││ ○ Replay too slow                     concerns        ││status=in-progress                                     │
│  all records       ││ ┼ harbor-db                                           ││prs=[acme/harbor-api#42]                               │
│  by kind           ││ ● Ledger flush stalls on slow disk    concerns        ││created=2026-10-28                                     │
│                    ││ ● Flush stalls on the ledger store    concerns        ││updated=2026-11-03                                     │
│                    ││ ○ Flush stalls return              !  concerns        ││promoted_to=harbor                                     │
│                    ││ ● Fsync stall on the ledger store     concerns        ││                                                       │
│                    ││ ┴                                                     ││linked from                                            │
│                    ││                                                       ││  · Harbor status  cites (cite)                        │
│                    ││                                                       ││@ktlo/2026/hbr-101-slow-ledger-flush.md:5              │
│                    ││19 reached   not followed: 1 disputed, 22 inferred     ││d sources and fresh cites   f run   a impact           │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780   index fresh   asked 00:21:18Z                       j/k record   l inspect   a impact   ? help   q quit
```

After (136×38, then 80×24):

```text
 Harbor status   build   knowledge base                                                                                   project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                   m runs
╭ projects (2) ──────╮╭ tracker (6) ──────────────────────────────────────────╮╭ ticket 1 of 6 ────────────────────────────────────────╮
│> Harbor status     ││ ┬ Harbor status                                       ││Slow ledger flush at month end   in-progress           │
│  build  harbor     ││ ○ Slow ledger flush at month end      in-progress     ││ticket HBR-101                                         │
│  Lantern console   ││ ┼ Harbor billing API                                  ││                                                       │
│  lantern           ││ ○ Replay too slow                     in-progress     ││prs           [acme/harbor-api#42]                     │
│  all records       ││ ┼ Harbor ledger store                                 ││created       2026-10-28                               │
│  by kind           ││ ● Ledger flush stalls on slow disk    done            ││updated       2026-11-03                               │
│                    ││ ● Flush stalls on the ledger store    concluded       ││promoted to   harbor                                   │
│                    ││ ○ Flush stalls return              !  open            ││                                                       │
│                    ││ ● Fsync stall on the ledger store     fixed           ││Linked from                                            │
│                    ││ ┴                                                     ││  · Harbor status   cites it                           │
│                    ││                                                       ││                                                       │
│                    ││tickets, investigations and signatures within two links││d exact wording and file   f run   a impact            │
│                    ││19 records reached · not followed: 1 disputed link,    ││                                                       │
│                    ││22 inferred links                                      ││                                                       │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                          j/k record   l inspect   a impact   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  3 tracker   5 stale 9 moved                                       m runs   3/5
╭ tracker (6) ─────────────────────────────────────────────────────────────────╮
│ ┬ Harbor status                                                              │
│ ○ Slow ledger flush at month end      in-progress                            │
│ ┼ Harbor billing API                                                         │
│ ○ Replay too slow                     in-progress                            │
│ ┼ Harbor ledger store                                                        │
│ ● Ledger flush stalls on slow disk    done                                   │
│ ● Flush stalls on the ledger store    concluded                              │
│ ○ Flush stalls return              !  open                                   │
│ ● Fsync stall on the ledger store     fixed                                  │
│ ┴                                                                            │
│                                                                              │
│tickets, investigations and signatures within two links                       │
│19 records reached · not followed: 1 disputed link, 22 inferred links         │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                               l inspect   a impact   ? help   q quit
```

### 3.10 KB mode, impact

Before (136×38):

```text
 Harbor status   build   knowledge base                                                                                  project:harbor
 1 why    2 status    3 tracker    4 impact    5 stale !9                                                                        m runs
╭ projects (2) ──────╮╭ impact ───────────────────────────────────────────────╮╭ ticket 1/19 ──────────────────────────────────────────╮
│> Harbor status     ││in  what cites or links to project:harbor              ││Slow ledger flush at month end   in-progress           │
│  build  harbor     ││                                                       ││HBR-101                                                │
│  Lantern console   ││ ┬ project:harbor                                      ││                                                       │
│  lantern           ││ ○ Slow ledger flush at month end        promoted-to   ││status=in-progress                                     │
│  all records       ││ ● Harbor billing API                 !  project       ││prs=[acme/harbor-api#42]                               │
│  by kind           ││ │ ○ Replay too slow                     concerns      ││created=2026-10-28                                     │
│                    ││ │ · Harbor billing API               !  dossier-of    ││updated=2026-11-03                                     │
│                    ││ │ · Restart the Harbor API              concerns      ││promoted_to=harbor                                     │
│                    ││ ● Harbor ledger store                   project       ││                                                       │
│                    ││ │ ● Ledger flush stalls on slow disk    concerns      ││linked from                                            │
│                    ││ │ ● Flush stalls on the ledger store    concerns      ││  · Harbor status  cites (cite)                        │
│                    ││ │ ○ Flush stalls return              !  concerns      ││@ktlo/2026/hbr-101-slow-ledger-flush.md:5              │
│                    ││ │ ● Fsync stall on the ledger store     concerns      ││                                                       │
│                    ││ – One writer process for the ledger  !  project       ││                                                       │
│                    ││ – Drop client retries on flush       !  project       ││                                                       │
│                    ││ ● Batch ledger writes in the API     !  project       ││                                                       │
│                    ││ ○ Shard the ledger by tenant            project       ││                                                       │
│                    ││ ○ Cap replay batches at 500 rows     !  project       ││                                                       │
│                    ││ · Quay batch worker                     project       ││                                                       │
│                    ││ ○ Add --plan to kb runs                 project       ││                                                       │
│                    ││ ○ Retire the legacy flush path          project       ││                                                       │
│                    ││ ● Tide latency dashboard                project       ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││19 reached   not followed: 1 disputed, 22 inferred     ││d sources and fresh cites   f run   a impact           │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780   index fresh   asked 00:21:18Z                          j/k scroll   h back   d detail   ? help   q quit
```

After (136×38, then 80×24):

```text
 Harbor status   build   knowledge base                                                                                   project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                   m runs
╭ projects (2) ──────╮╭ impact ───────────────────────────────────────────────╮╭ ticket ───────────────────────────────────────────────╮
│> Harbor status     ││what points at Harbor status                           ││Slow ledger flush at month end   in-progress           │
│  build  harbor     ││                                                       ││ticket HBR-101                                         │
│  Lantern console   ││ ┬ Harbor status                                       ││                                                       │
│  lantern           ││ ○ Slow ledger flush at month end        promoted to it││prs           [acme/harbor-api#42]                     │
│  all records       ││ ● Harbor billing API                 !  in project    ││created       2026-10-28                               │
│  by kind           ││ │ ○ Replay too slow                     about it      ││updated       2026-11-03                               │
│                    ││ │ · Harbor billing API               !  its dossier   ││promoted to   harbor                                   │
│                    ││ │ · Restart the Harbor API              about it      ││                                                       │
│                    ││ ● Harbor ledger store                   in project    ││Linked from                                            │
│                    ││ │ ● Ledger flush stalls on slow disk    about it      ││  · Harbor status   cites it                           │
│                    ││ │ ● Flush stalls on the ledger store    about it      ││                                                       │
│                    ││ │ ○ Flush stalls return              !  about it      ││d exact wording and file   f run   a impact            │
│                    ││ │ ● Fsync stall on the ledger store     about it      ││                                                       │
│                    ││ – One writer process for the ledger  !  in project    ││                                                       │
│                    ││ – Drop client retries on flush       !  in project    ││                                                       │
│                    ││ ● Batch ledger writes in the API     !  in project    ││                                                       │
│                    ││ ○ Shard the ledger by tenant            in project    ││                                                       │
│                    ││ ○ Cap replay batches at 500 rows     !  in project    ││                                                       │
│                    ││ · Quay batch worker                     in project    ││                                                       │
│                    ││ ○ Add --plan to kb runs                 in project    ││                                                       │
│                    ││ ○ Retire the legacy flush path          in project    ││                                                       │
│                    ││ ● Tide latency dashboard                in project    ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││19 records reached · not followed: 1 disputed link,    ││                                                       │
│                    ││22 inferred links   ! cites a source that has moved    ││                                                       │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                       j/k record   l inspect   v direction   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  4 impact   5 stale 9 moved                                        m runs   4/5
╭ impact ──────────────────────────────────────────────────────────────────────╮
│what points at Harbor status                                                  │
│ ┬ Harbor status                                                              │
│ ○ Slow ledger flush at month end        promoted to it                       │
│ ● Harbor billing API                 !  in project                           │
│ │ ○ Replay too slow                     about it                             │
│ │ · Harbor billing API               !  its dossier                          │
│ │ · Restart the Harbor API              about it                             │
│ ● Harbor ledger store                   in project                           │
│ │ ● Ledger flush stalls on slow disk    about it                             │
│ │ ● Flush stalls on the ledger store    about it                             │
│ │ ○ Flush stalls return              !  about it                             │
│ │ ● Fsync stall on the ledger store     about it                             │
│ – One writer process for the ledger  !  in project                           │
│ – Drop client retries on flush       !  in project                           │
│ ● Batch ledger writes in the API     !  in project                           │
│ ○ Shard the ledger by tenant            in project                           │
│ ○ Cap replay batches at 500 rows     !  in project                           │
│ · Quay batch worker                     in project                           │
│19 records reached · not followed: 1 disputed link, 22 inferred links         │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                            l inspect   v direction   ? help   q quit
```

### 3.11 KB mode, stale and its inspector

Before (136×38):

```text
 Harbor status   build   knowledge base                                                                                  project:harbor
 1 why    2 status    3 tracker    4 impact    5 stale  !9                                                                       m runs
╭ projects (2) ──────╮╭ stale (9) ────────────────────────────────────────────╮╭ cite 6/9 ─────────────────────────────────────────────╮
│> Harbor status     ││ ┬ harbor/STATUS.md                                    ││code:demo-service:src/batch.rs:2-4   moved             │
│  build  harbor     ││ · …ttps://example.invalid/wiki/harbor !  expired      ││                                                       │
│  Lantern console   ││ · …/2026/hbr-101-slow-ledger-flush.md !  changed      ││at d0da8b6883f1 (HEAD)                                 │
│  lantern           ││ ┼ harbor/evidence/flush-latency.md                    ││moved→4-6                                              │
│  all records       ││ · code:demo-service:src/legacy.rs:1-2 !  gone         ││                                                       │
│  by kind           ││ · code:demo-service:src/db.rs:1-3     !  moved        ││cited by                                               │
│                    ││ ┼ inventory/dossiers/harbor-api.md                    ││  ● Batch ledger writes in the API  cites (cite)       │
│                    ││ · code:demo-service:src/retry.rs:2-3  !  changed      ││  ○ Cap replay batches at 500 rows  cites (cite)       │
│                    ││ ┼ proposals/decision/2026-03-02-batch-ledger-writes/p…││                                                       │
│                    ││ · code:demo-service:src/batch.rs:2-4  !  moved        ││referrers of code:demo-service:src/batch.rs lines 2-4; │
│                    ││ ┼ proposals/decision/2026-11-11-cap-replay-batches-at…││  reliance that no record cites or mentions is not     │
│                    ││ · code:demo-service:src/batch.rs:2-4  !  moved        ││  traced                                               │
│                    ││ ┼ inventory/assets/harbor-api.json                    ││…decision/2026-03-02-batch-ledger-writes/proposal.md:31│
│                    ││ ● harbor-api                          !  active       ││                                                       │
│                    ││ ┼ investigations/2026/2026-06-01-flush-stalls-again/R…││                                                       │
│                    ││ ○ …2026/2026-06-01-flush-stalls-again !  open         ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││                                                       ││d sources and fresh cites   f run   a impact           │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780   index fresh   asked 00:21:18Z                       j/k record   l inspect   a impact   ? help   q quit
```

After (136×38):

```text
 Harbor status   build   knowledge base                                                                                   project 1 of 3
  1 why    2 status    3 tracker    4 impact    5 stale 9 moved                                                                   m runs
╭ projects (2) ──────╮╭ stale (9) ────────────────────────────────────────────╮╭ citation 6 of 9 ──────────────────────────────────────╮
│> Harbor status     ││ ┬ harbor/STATUS.md                                    ││src/batch.rs:2-4 in demo-service   moved               │
│  build  harbor     ││ · https://example.invalid/wiki/harbor     expired     ││a citation of code                                     │
│  Lantern console   ││ · ktlo/2026/hbr-101-slow-ledger-flush.md  changed     ││                                                       │
│  lantern           ││ ┼ harbor/evidence/flush-latency.md                    ││The cited lines are now at 4-6.                        │
│  all records       ││ · src/legacy.rs:1-2                       gone        ││                                                       │
│  by kind           ││ · src/db.rs:1-3                           moved       ││Cited by                                               │
│                    ││ ┼ inventory/dossiers/harbor-api.md                    ││  ● Batch ledger writes in the API                     │
│                    ││ · src/retry.rs:2-3                        changed     ││  ○ Cap replay batches at 500 rows                     │
│                    ││ ┼ proposals/decision/2026-03-02-batch-ledger-writes/p…││                                                       │
│                    ││ · src/batch.rs:2-4                        moved       ││1 note in exact wording (d)                            │
│                    ││ ┼ proposals/decision/2026-11-11-cap-replay-batches-at…││                                                       │
│                    ││ · src/batch.rs:2-4                        moved       ││d exact wording and file   f run   a impact            │
│                    ││ ┼ inventory/assets/harbor-api.json                    ││                                                       │
│                    ││ ● harbor-api                              overdue     ││                                                       │
│                    ││ ┼ investigations/2026/2026-06-01-flush-stalls-again/R…││                                                       │
│                    ││ ○ …2026/2026-06-01-flush-stalls-again     came back   ││                                                       │
│                    ││ ┴                                                     ││                                                       │
│                    ││                                                       ││                                                       │
│                    ││9 need a look · 11 checked · from the last recorded    ││                                                       │
│                    ││check; nothing was re-run                              ││                                                       │
╰────────────────────╯╰───────────────────────────────────────────────────────╯╰───────────────────────────────────────────────────────╯
  read-only                                                                          j/k record   l inspect   a impact   ? help   q quit
```

Before and after (80×24, list):

```text
 Harbor status   build   knowledge base                             project 1/3
  5 stale  !9                                                     m runs    5/5
╭ stale (9) ───────────────────────────────────────────────────────────────────╮
│ ┬ harbor/STATUS.md                                                           │
│ · web:https://example.invalid/wiki/harbor       !  expired                   │
│ · record:ktlo/2026/hbr-101-slow-ledger-flush.md !  changed                   │
│ ┼ harbor/evidence/flush-latency.md                                           │
│ · code:demo-service:src/legacy.rs:1-2           !  gone                      │
│ · code:demo-service:src/db.rs:1-3               !  moved                     │
│ ┼ inventory/dossiers/harbor-api.md                                           │
│ · code:demo-service:src/retry.rs:2-3            !  changed                   │
│ ┼ proposals/decision/2026-03-02-batch-ledger-writes/proposal.md              │
│ · code:demo-service:src/batch.rs:2-4            !  moved                     │
│ ┼ proposals/decision/2026-11-11-cap-replay-batches-at-500-rows/proposal.md   │
│ · code:demo-service:src/batch.rs:2-4            !  moved                     │
│ ┼ inventory/assets/harbor-api.json                                           │
│ ● harbor-api                                    !  active                    │
│ ┼ investigations/2026/2026-06-01-flush-stalls-again/README.md                │
│ ○ inv:2026/2026-06-01-flush-stalls-again        !  open                      │
│ ┴                                                                            │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780            l inspect   a impact   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  5 stale 9 moved                                                   m runs   5/5
╭ stale (9) ───────────────────────────────────────────────────────────────────╮
│ ┬ harbor/STATUS.md                                                           │
│ · https://example.invalid/wiki/harbor     expired                            │
│ · ktlo/2026/hbr-101-slow-ledger-flush.md  changed                            │
│ ┼ harbor/evidence/flush-latency.md                                           │
│ · src/legacy.rs:1-2                       gone                               │
│ · src/db.rs:1-3                           moved                              │
│ ┼ inventory/dossiers/harbor-api.md                                           │
│ · src/retry.rs:2-3                        changed                            │
│ ┼ proposals/decision/2026-03-02-batch-ledger-writes/proposal.md              │
│ · src/batch.rs:2-4                        moved                              │
│ ┼ proposals/decision/2026-11-11-cap-replay-batches-at-500-rows/proposal.md   │
│ · src/batch.rs:2-4                        moved                              │
│ ┼ inventory/assets/harbor-api.json                                           │
│ ● harbor-api                              overdue                            │
│ ┼ investigations/2026/2026-06-01-flush-stalls-again/README.md                │
│ ○ …2026/2026-06-01-flush-stalls-again     came back                          │
│ ┴                                                                            │
│9 need a look · 11 checked · from the last recorded check; nothing was re-run │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                               l inspect   a impact   ? help   q quit
```

Inspector before and after (80×24):

```text
 Harbor status   build   knowledge base                             project 1/3
  5 stale  !9                                                     m runs    5/5
╭ cite 6/9 ────────────────────────────────────────────────────────────────────╮
│code:demo-service:src/batch.rs:2-4   moved                                    │
│at d0da8b6883f1 (HEAD)                                                        │
│moved→4-6                                                                     │
│cited by                                                                      │
│  ● Batch ledger writes in the API  cites (cite)                              │
│  ○ Cap replay batches at 500 rows  cites (cite)                              │
│referrers of code:demo-service:src/batch.rs lines 2-4; reliance that no record│
│  cites or mentions is not traced                                             │
│@proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:31             │
│d sources and fresh cites   f run   a impact                                  │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only   kb 441cbd386780               h back   d detail   ? help   q quit
```

```text
 Harbor status   build   knowledge base                           project 1 of 3
  5 stale 9 moved                                                   m runs   5/5
╭ citation 6 of 9 ─────────────────────────────────────────────────────────────╮
│src/batch.rs:2-4 in demo-service   moved                                      │
│a citation of code                                                            │
│                                                                              │
│The cited lines are now at 4-6.                                               │
│                                                                              │
│Cited by                                                                      │
│  ● Batch ledger writes in the API                                            │
│  ○ Cap replay batches at 500 rows                                            │
│                                                                              │
│1 note in exact wording (d)                                                   │
│                                                                              │
│d exact wording and file   f run   a impact                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
  read-only                                  h back   d detail   ? help   q quit
```

Every row of this view is there because something moved, so the mark column goes and the
dim word becomes the reason (today it is the record's status, which reads
`harbor-api ! active`).

### 3.12 Other panes

- **Wide side stack (140 columns and up).** Gates: the count line, then `! lint   out of
  date` per gate. Inbox: the count line, then one line per entry. Guards: the count line
  and one line per rule. Timeline: the same three columns, clipped.
- **Run views 6 stale, 7 brief, 8 resume.** The generic row form of §5.3: glyph where the
  kind has one, title or name, `kind · state`, readable continuations, withheld count.
- **`x` (text list)** is unchanged at both levels. It is the plan as the verb prints it,
  and it is reached by a key, not shown by default.
- **Help (`?`)** gains the Words section; Marks, This frame, keys and verbs are unchanged.

## 4. Reaching the exact text

| Want | Key | What appears |
|---|---|---|
| The verbs' wording for the whole frame | `d` | The exact level: the pre-T6 detailed frame. `d` again returns. Never written; a fresh start is plain |
| What a plain phrase stands for | `?` | Words: `out of date = stale`, `passed = satisfied`, `signed off = attested`, … and Marks |
| This frame's stamps | `?` | This frame: run id, `kb STAMP`, `index`, `subject`, `asked HH:MM:SSZ`, theme, colour (as today) |
| Ids and commands to copy | `c` | The command rows (below) |
| The plan as verb text | `x` | The text list (as today) |
| Full gate receipts | `g`, then `d` | Gates at the exact level |

**The command rows (`c`).** `c` replaces the bottom rows of the frame with borderless,
full-width lines, so a terminal selection copies clean text:

```text
 commands for step check · copy into a shell; watch never runs them · UPPERCASE words are placeholders
 # run   run-20261101T090000Z-a1b2c3
 # step  change#1/review/review#1/findings/findings#1/check
 # read  .writfold/gates/signoff.md
 writfold kb gate cover review-list tests supported REF… --by WHO
 any key closes
```

- Run mode, Progress: the selected step's `read` and `next` continuations. Gates, Inbox,
  Guards, Resume: every `next`, `read`, `confirm` and `accept` of the view, in row order,
  each under a `# name` comment.
- KB mode: `# record ID`, `# file PATH:LINE`, then the verbs the inspector asked
  (`writfold kb status ID`, `writfold kb why ID`, `writfold kb timeline ID`).
- A command is printed whole, exactly as the verb gave it. A line longer than the frame
  breaks at a space and ends in ` \`, so the copy still runs. Ids and paths are `#`
  comment lines, harmless if pasted.
- At most ten rows; `j`/`k` scroll; any other key closes. `watch` runs nothing, and the
  first row says so. `c` was unbound to keep any hint of "confirm" away from a key; the
  rows show commands and never accept input, and help lists `c` as `commands to copy`.

**`--once` and pipes.** Unchanged, byte for byte. `--once`, `--view NAME --once` and a
non-terminal stdout print the same expanded plain frame as today, and
`--once --view kb:NAME` the same sectioned text. That output is read by scripts and by
agents, it is already the exact form, and it has no `d` key to recover anything a phrase
left out. The JSON verbs are untouched; `watch` still has no JSON of its own. The existing
goldens `watch-once.txt`, `progress-once.txt`, `progress-once-view-gates.txt` and
`t5-once-kb-why.txt` must not change.

## 5. Code plan

### 5.1 The phrasing layer

New modules in `crates/writfold-tui/src`, all pure: they import `serde_json::Value`,
`crate::source::{Reply, field, flags}`, `crate::lines::Tone` and `crate::cells`, and
nothing from `ratatui`, `App` or the terminal. Rendering modules turn their output into
spans.

```rust
// say.rs
pub enum Piece {
    Said(String),    // the viewer's own words, from the tables in say_words
    Quoted(String),  // a verb's or an author's text, passed through whole
    Mark(Cell),      // a state glyph
}
pub struct Phrase { pub indent: u16, pub tone: Tone, pub pieces: Vec<Piece> }
pub struct Told { pub lines: Vec<Phrase>, pub withheld: usize }

pub fn readable(text: &str) -> bool;            // true when no token is notation
pub fn quote(text: &str) -> Option<Piece>;      // Some(Quoted) only when readable
```

`readable` is a hand-written token scan (no regex at run time; `regex` stays a
dev-dependency). It refuses a text when any whitespace-separated token: is `[word]`; starts
with `@`, `--` or `!` followed by a letter; contains `=` between word characters; contains
a backtick or `→`; has eight or more consecutive digits or twelve or more consecutive hex
digits; is an ISO stamp; is a lowercase word immediately followed by `:` and more text
(`gate:lint`, `code:repo:…`); contains `_` between letters; or is a path of three or more
segments. The rule is **whole or nothing**: a text is quoted unchanged or not shown.

Authored text is exempt and always quoted whole: run, record and proposal titles
(`text` of `run`, record and `find` rows, `title` of impact rows), checkpoint text,
checklist requirements (the remainder of an `open` item), `why` and `decided` lines of a
decision, draft text, field values. The existing rule stands: authored text is never
censored or rewritten.

| Module | Holds | Size |
|---|---|---|
| `say.rs` | `Piece`, `Phrase`, `Told`, `readable`, `quote`, joining with ` · ` | ≤160 |
| `say_words.rs` | the §2.1 tables as `const` slices: `STEP`, `STANDING`, `CLASS`, `ITEM`, `GUARD`, `CONDITION`, `MOVED`, `RELATION`, `STEP_FLAG`; `fn standing(&str) -> Option<&'static Word>`; `Word { short, long }`; `glossary()` for help | ≤230 |
| `say_shape.rs` | the anchored shapes below | ≤200 |
| `say_step.rs` | step head, where line, needs line, map row words, plan head | ≤220 |
| `say_gate.rs` | gate and item lines for the inspector and Gates; gate counts line; the `not met` badge | ≤220 |
| `say_row.rs` | the generic row form (Timeline, Inbox, Guards, Stale, Brief, Resume, KB status) and the per-view captions | ≤240 |
| `say_kb.rs` | KB inspector sections, relation phrases by edge end, stale reasons, trailer | ≤230 |
| `say_next.rs` | the suggested-next phrase from a command's argv | ≤120 |
| `commands.rs` | the `c` rows: collecting, wrapping with ` \`, drawing without borders | ≤200 |

**Shapes (`say_shape.rs`).** A shape takes a prefix or suffix off a verb's `text` only when
that affix is made of the row's own structured fields, so the remainder is known to be the
rest. Each returns `Option`; `None` means the text goes to `quote` whole.

| Shape | Rule | Source of the guarantee |
|---|---|---|
| `named(value)` | `text` starts with `ID [STATUS]` where `STATUS` equals the `status` field and `ID` has no space; yields `(ID, rest)` | `docs/guide/json.md`, "Labels the views rely on" |
| `optional(rest)` | `rest` starts with `(optional)` followed by a space or the end | same |
| `class(rest)` | first word is `mechanical`, `checklist` or `attested` (a trailing `:` allowed); for `checklist`, the next word is `N/M` and the two after are `required covered` | same; today's `inspect::checklist` already reads it |
| `template(splice)` | `NAME#ORDINAL @DIGEST12 from SOURCE`; yields `(NAME, ORDINAL)` | same |
| `when(text)` | a leading `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DD` token, checked character by character; yields the respelt stamp and the rest | lexical only |
| `event(row)` | timeline `plan` rows: after `when`, the rest starts with `STATUS ID` (the row's own fields); yields the remainder | the row's own fields |
| `pair(text)` | a `field` continuation `key=value`: split at the first `=`, key matches `[a-z_]+` | `field` label |
| `requires(text)` | guard text ends with ` requires CONDITION` | `docs/guide/hooks.md` conditions |
| `locator(id)` | `code:REPO:REST` → `REST in REPO`; `web:URL` → `URL`; `record:PATH` → `PATH`; any other id whole | citation token schemes |
| `stamp(kb)` | `HASH+dirty:N` → `N`; a bare hash → clean | the `kb` trailer |
| `argv(text)` | split on spaces; must start `writfold kb` | the CLI's own grammar |

**Suggested next (`say_next.rs`).** A lookup on the command's leading argv; the gate, item
and draft names are the argv words themselves.

| argv prefix | Phrase |
|---|---|
| `writfold kb gate run G` | `run the G gate` |
| `writfold kb gate cover G ITEM` | `give evidence for ITEM in G` |
| `writfold kb gate attest G` | `record a sign-off for G` |
| `writfold kb checkpoint --from-draft D` | `confirming it records a checkpoint` |
| `writfold kb proposals accept ID` | `accepting it adds it to the KB` |
| `writfold kb stale …` | `re-check the cited sources` |
| anything else | `a command is suggested (c)` |

### 5.2 Honesty rules

Each is enforced by a test in §5.6.

- **H1. States come from fields.** A `Said` state phrase is a lookup on `status`, `flags`
  or a trailer count. No phrase is derived from bracketed or free text.
- **H2. Only `satisfied` says "passed".** `attested` is "signed off", `waived` is "waived",
  and their long forms say that no check passed. "Met" and "not met" keep the product's
  meaning: met is satisfied, attested or waived.
- **H3. No liveness.** `current` is "next up"; `▶` is "the next step to pick up, as
  recorded". No `Said` piece contains running, working, in progress, underway, busy,
  live, active or waiting. (`in-progress` on a ticket is the record's own status word and
  is `Quoted`.)
- **H4. Stale is "out of date".** Never failed, broken or wrong.
- **H5. Guards are conditional.** Always "would …"; never blocked, denied or enforced.
  The caption says a guard advises and nothing was run.
- **H6. Later and unknown stay unjudged.** `later` is "not judged yet"; `unknown` is "no
  verdict".
- **H7. Counts are the verbs' counts.** The only sum is `not met`, and only when the
  `gates`, `satisfied`, `attested` and `waived` counts are all present; otherwise the tab
  has no badge. "Needs N gates" is the trailer's `current_owed`.
- **H8. No guessing.** A status, flag, relation, class or command with no table entry is
  printed as itself (`Quoted`).
- **H9. Whole or nothing.** A verb's sentence is quoted unchanged or withheld; a pane that
  withholds a row's text or a note with no phrase in its place ends with `N lines in exact
  wording (d)` (`N notes …` for notes).
- **H10. Names are looked up, not carved.** A step is named as the map names it (its
  group and `under` links); a run by the runs list; a tracker header by a row on screen.
  When the lookup fails the id is printed. Ids are still never split on `/`.
- **H11. Time is respelt, not computed.**

### 5.3 Modules to change

| Module | Change |
|---|---|
| `app.rs`, `keys.rs`, `kb_keys.rs` | `Detail::{Calm, Full}` → `Detail::{Plain, Exact}` (rename only); `c` toggles `app.commands`; any key closes it |
| `inspect.rs`, `inspect_calm.rs` → `inspect_plain.rs` | the plain inspector builds from `say_step` and `say_gate`; the exact path (`inspect.rs`) is untouched |
| `rail.rs`, `rail_text.rs`, `graph.rs` | at plain: no chip cells, group header from `template`, row words from `say_step::row` for current, selected and flagged rows; exact path untouched |
| `progress.rs`, `map_view.rs` | plan head and the two fact lines from `say_step`; `x` content untouched |
| `bar_content.rs`, `kb_bars.rs`, `bars.rs` | header right, subject, badges (long and short form, shortened before a tab drops), footer left; at exact the current bars |
| `side.rs`, `guards.rs`, `list_view.rs`, `view.rs`, `frame.rs` | text views and the side stack take `say_row`/`say_gate` at plain and today's lines at exact; `frame::once` and `once_kb` keep calling today's line builders only |
| `kb_entry.rs`, `kb_rail.rs`, `kb_tree.rs`, `kb_view.rs` | tracker word, stale reason and locator, header titles, caption with the mark legend, trailer |
| `kb_sections.rs`, `kb_inspect.rs` | section headings and lines from `say_kb`; exact path untouched |
| `help.rs`, `kb_help.rs` | Words section from `say_words::glossary()`; `c` in the key list |
| `wrap.rs` | a `Phrase` wraps with a hanging indent under its second column (reuse the existing hanging wrap) |

The exact level calls the pre-T6 functions with the pre-T6 arguments. No exact-level code
path is edited beyond the enum rename; that is what makes the byte-identical golden a real
check.

### 5.4 Goldens

- **Must not change:** `t4-detail-136x38.txt`, `t5-why-detail-136x38.txt` (the exact
  level) and the four plain-output goldens named in §4.
- **Re-blessed:** every interactive buffer rendered at the default level:
  `t4-calm-136x38.txt`, `t4-calm-80x24.txt`, `frame-100x30.txt`, `frame-160x40.txt`, the
  `t3-*` buffers that render at the default level, `t5-why-136x38.txt`,
  `t5-why-80x24.txt`, `t5-impact-136x38.txt`, `t5-tracker-100x30.txt`. Before re-blessing,
  copy each to `tests/golden/pre-t6/` under its current name, as `pre-t5/` does.
- **New:** `t6-inspector-80x24.txt`, `t6-gates-136x38.txt`, `t6-gates-80x24.txt`,
  `t6-inbox-136x38.txt`, `t6-guards-136x38.txt`, `t6-timeline-136x38.txt`,
  `t6-kb-status-136x38.txt`, `t6-kb-stale-136x38.txt`, `t6-kb-stale-inspector-80x24.txt`,
  `t6-commands-136x38.txt`.
- A re-bless is reviewed as a diff against `pre-t6/`: every removed token must appear in
  the matching exact buffer or in the `c` rows. `tests/plain_exact.rs` automates the part
  that can be: for each fixture, every gate name, step id, receipt path and command in the
  verb answers appears in the exact-level buffers or the command rows.

### 5.5 Budgets

Modules ≤250 lines and functions ≤80 with tests included (the sizes in §5.1 leave room for
the in-module unit tests of the smaller files; table tests for `say_words` live in
`tests/words.rs`). No `unwrap` or `expect`. No new crates: `regex` is used in tests only,
as today. `scripts/gate quality` and `scripts/gate unit` from main before pushing.

### 5.6 Tests, each with its mutation

| Test (file) | Asserts | Fails when |
|---|---|---|
| `every_standing_has_a_phrase` (`tests/words.rs`) | each status the kernels can produce (`open`, `satisfied`, `failed`, `unknown`, `stale`, `waived`, `attested`, `later`; item and guard sets) has a table entry | the `stale` row is removed from `STANDING` |
| `only_satisfied_says_passed` | among all `Word`s, `passed` appears only for `satisfied`; `attested` and `waived` contain none of pass, satisf, verified, checked | `attested` is reworded to "passed" |
| `no_liveness_words` | no table phrase or caption matches `\b(running|working|in progress|underway|busy|live|active|waiting)\b` | `current` becomes "running" or the map row says "waiting on" |
| `guards_stay_conditional` | every `GUARD` phrase starts with `would` | `would-deny` becomes "blocked" |
| `unknown_words_are_quoted` | status `weird`, flag `odd`, relation `rivals` come back as `Quoted` with the same text | a default phrase is added |
| `named_requires_the_rows_own_status` (`say_shape` unit) | `named` returns `None` for `{status: "stale", text: "unit [satisfied] mechanical"}` and for an id with a space | the status comparison is dropped |
| `readable_refuses_notation` (`say` unit) | the stale item's account, a gate's receipt text, a `field` line, a typed id, a command are refused; `changed in scope: harbor/lint.txt`, `1 reference by agent`, `entry 3 of 3: Review started.` pass | any one token rule is deleted |
| `readable_agrees_with_the_sweep` (`tests/plain_sweep.rs`) | for every `text` and note in every fixture answer, `readable(t)` implies no terse regex matches `t` (two independent implementations) | the scan and the list drift apart |
| `said_pieces_carry_no_notation` | over every fixture (nested, out-of-order, checklist, no-subject, the KB world), every run view and KB view, selection on every row: no `Said` piece matches the terse list | a phrase is written with a bracket, `=`, `!flag` or stamp |
| `plain_surface_has_no_terse_tokens` | the rendered plain buffers at 136×38, 100×30 and 80×24 match none of the terse list (below); the fixtures' authored text contains none | the `quote` call in `say_row` is replaced by the raw text; the footer prints the KB stamp; a pane title regains the run id |
| `plain_map_has_no_chips` | no cell of a plain map row is a chip glyph outside the state column | chips are drawn at plain |
| `withholding_is_announced` | a pane whose answer has an unreadable note or row text ends with the `in exact wording (d)` line, with the right count | the counter is not threaded through |
| `not_met_follows_the_product` | a fixture with one attested and one waived gate shows `not met` = gates − satisfied − attested − waived; with a count missing, no badge | the badge sums `open` and `stale` only |
| `exact_level_is_unchanged` | the two exact goldens are byte-identical after `d` | any exact path is edited |
| `once_is_unchanged` | the four plain-output goldens | a phrase leaks into `frame::once` |
| `d_restores_every_fact` (`tests/plain_exact.rs`) | §5.4, last bullet | a fact is dropped from both levels |
| `command_rows_are_whole_and_borderless` | each command of the answer appears verbatim once its ` \` breaks are joined; no row contains `│`; a wrapped row ends in ` \` | the rows are drawn inside a block, or a command is clipped with `…` |
| `stamps_are_respelt_only` | `when` maps `2026-11-10T23:59:50Z` to `2026-11-10 23:59` and returns `None` for anything else | a parse-and-format path is introduced |
| `help_words_match_the_tables` | every `Word` appears in the Words section with its exact word | help is hand-written |

The terse list (one `const` in `tests/support`): ISO stamp
`\d{4}-\d\d-\d\dT\d\d:\d\d`, clock stamp `\d\d:\d\d:\d\dZ`, position `(^|\s)@\S`, hash
`\b[0-9a-f]{12,}\b`, `key=value` `\b[a-z_]+=\S`, bracket state `\[[a-z-]+\]`, flag word
`(^|\s)![a-z]`, run id `run-\d{8}T`, placeholders `REF…|--by\b|--statement\b|\bWHO\b`,
label prefixes `(^|\s)(owes|next|read|confirm|accept|splice|under|field): `, `# note:`,
typed ids `\b(project|inv|sig|code|web|record|gate|seen):\S`, `\+dirty:`, `writfold kb`,
`→`, `would-[a-z]+`, and the chip pairs `[●◆○!×?·]{2,}`. The same sweep runs with
`Glyphs::Ascii`.

## 6. Build order

One implementer; each step leaves the gates green.

1. **Rename and scaffold.** `Detail::{Plain, Exact}`; `say.rs` with `Piece`, `Phrase`,
   `readable`, `quote` and their unit tests. Copy the goldens to `pre-t6/`.
2. **Vocabulary.** `say_words.rs`, `tests/words.rs` (H2–H6, H8), the Words section in
   help.
3. **Shapes.** `say_shape.rs` and `say_next.rs` with unit tests, including the refusals.
4. **The homepage frame.** `say_step.rs`, `say_gate.rs`; plain inspector, map rows, group
   headers, plan head, fact lines. Re-bless `t4-calm-*`, `frame-*`, `t3-*`; add
   `t6-inspector-80x24`. Check `exact_level_is_unchanged`.
5. **Bars.** Header, badges with their short forms, footer; narrow rows.
6. **Run text views.** `say_row.rs`; Gates, Inbox, Guards, Timeline, the side stack, and
   the generic form for Stale, Brief and Resume; withheld counts. New `t6-*` goldens.
7. **KB mode.** `say_kb.rs`; lists, captions, trailer, inspector sections, status view,
   stale reasons. Re-bless `t5-*` (not the detail and once goldens); new `t6-kb-*`.
8. **Command rows.** `commands.rs`, `c`, help line, `t6-commands-136x38`.
9. **Sweep and documents.** `tests/plain_sweep.rs` and `tests/plain_exact.rs` over every
   fixture and size; `docs/guide/watch.md` (the Calm and detailed section becomes Plain and
   exact, new frames, the `c` row in Controls, the Words table); a T6 as-built section in
   `docs/writfold/kb-v1-design.md`; recapture the website frames from the same worlds.

**Cut line.** Steps 1–9 are the release. Below the line, in this order, each independent:

- Bespoke layouts for run views 6–8 beyond the generic row form.
- `change · 2nd time` for repeated splices (until then the header shows `change#2`).
- `--level plain|exact` to start at a level (not persisted).
- Long forms for the disorder flags after a pass over real merged runs.

If step 8 has to slip, the inspector hint reads `d exact wording   g gates` and the plain
lines that mention `c` end in `(d)` instead; nothing else depends on it.

## 7. Open question

**Should the level be remembered?** `d` is never written today, so a fresh start is always
plain. Someone who works at the exact level presses `d` once per session. Recommendation:
keep it unwritten for this release (the default is the point of T6, and `watch` writes
nothing by rule), and add `--level exact` below the cut line if daily use asks for it. A
configuration key would be the first setting `watch` reads besides the theme and should
wait for that evidence.

## Decisions (owner, 15 November 2026)

1. **The level is not remembered.** A fresh start is plain. A `--level plain|exact` flag is built if time allows; it is below the cut line.
2. The plain phrases for the disorder flags and `step.met` are confirmed against the guides by the implementer; where a guide is ambiguous, the phrase says less, not more.
3. Ships in the same release as the v0.5.0-dev.10 round, with the website frames re-captured from the plain level.
