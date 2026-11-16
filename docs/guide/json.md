# Reading answers as JSON

> **Status:** built in v0.5.0-dev.1 (the envelope itself has been emitted since v0.1; this page and the test that pins it are new). `matched_on`, the `find --eval`, `guard log`, `runs --idle` and `delta` answers built in v0.5.0-dev.10.

Every `writfold kb` verb that prints rows and a trailer takes `--json`. The answer is then one JSON object on one line of stdout, holding the same rows, notes and trailer as the text form. The exit code is the same in both forms.

JSON is for reading answers in scripts and tools. It is not a way to enforce anything: to stop a commit or a publication on a standing, use a gate in a plan step or a delivery mode ([gates](gates.md), [delivery-modes](delivery-modes.md)).

## The envelope

```text
$ writfold kb why "batch ledger" --json
{"notes":[],"rows":[{"flags":[],"id":"proposal/decision/2026-03-02-batch-ledger-writes","kind":"decision","line":5,"more":[…],"path":"proposals/decision/2026-03-02-batch-ledger-writes/proposal.md","section":null,"status":"accepted","text":"Batch ledger writes in the API"},…],"trailer":{"answer":"known","counts":{},"coverage":"complete","cursor":null,"index":"fresh","kb":"d5db758bda80","needs":null,"omitted":0,"omitted_by":{},"q":"206e21d1c60f","reason":null,"records_examined":28,"records_total":28,"shown":3,"unreadable":0},"verb":"why","writfold_kb":1}
```

| Key | Type | Meaning |
|---|---|---|
| `writfold_kb` | integer | the envelope version, `1` today |
| `verb` | string | the verb that answered (`why`, `stale` and so on) |
| `rows` | array | the rows of this page, in the order the text form prints them |
| `notes` | array of strings | the `# note:` lines of the text form |
| `trailer` | object | the trailer line, as fields |

## A row

| Key | Type | Meaning |
|---|---|---|
| `kind` | string | the row's kind: `project`, `ticket`, `decision`, `cite`, `gate`, `step`, `check` and so on |
| `id` | string | what the row is about: a record ID, a citation target, a gate name |
| `status` | string or null | the bracketed state in the text form (`[accepted]`, `[expired]`) |
| `text` | string | the row's text |
| `path`, `line` | string, integer, or null | the source the row ends in (`@path:line`) |
| `flags` | array of strings | the `!` flags, without the `!` |
| `more` | array | the indented lines under the row |
| `section` | string or null | the output section, used for `omitted_by` |

Each entry of `more` has `label` (`why`, `note`, `cite`, `next` and so on), `text`, `path`, `line` and `flags`, with the same meanings. It may also have `status` (a string): the continuation's state. This key is absent, not null, when the verb does not supply a state.

### Keys only some verbs add

`impact` rows carry two more keys, both absent (not null) on the rows of every other verb:

| Key | Type | Meaning |
|---|---|---|
| `edge` | object | the edge that reached this row: `from`, `rel`, `to` and `origin` (`authored`, `field`, `cite`, `mention`), and `depth` (an integer, 1 for an edge at the walk's start). The row's `id` is `from` or `to`; the other end is the record the walk came from |
| `title` | string | the reached record's title; absent when the record has none (a code locator, a record labelled by its ID) |

The text form of `impact` is unchanged (`depth=N SRC REL DST (ORIGIN)` in `text`); readers should use `edge`, not split that text.

`find` and `match` rows carry one more key (built in v0.5.0-dev.10), absent on every other verb's rows:

| Key | Type | Meaning |
|---|---|---|
| `matched_on` | string | the tier that found the row: `signature`, `exact`, `normalized`, `prefix`, `heading`, `text`, `substring`, `typo` or `text-any`, and `listed` for a `--kind` listing; `suggestion` for a name offered when nothing matched (status `suggested`, flag `suggestion`), which is not a hit. `exact`, `normalized` and `text` are spelt as other tools' readers expect; a reader that knows only those ignores the rest. The `match` continuation stays, with its path and line |

Readers should use `matched_on`, not split the `match` continuation's text (`typo d=1 api`).

`guard status` (from v0.5.0-dev.10) adds the flags `audit` and `waived`, the continuation labels `mode` and `waived`, and the count `audit` (present only when not zero). `guard log` answers rows of kind `guard-log` (status `audit`, `enforce` or `removed`) with continuations labelled `would-deny`, `would-ask`, `deny`, `ask`, `earlier` and `next`, the flags `audit`, `would-deny`, `would-ask` and `unmatched`, and the counts `rules`, `entries`, `would-deny`, `would-ask`, `deny`, `ask` and `unparsed`.

`delta` (built in v0.5.0-dev.10) answers rows of kind `delta`, one per changed path: `id` the record ID or the path, `status` `added`, `modified` or `deleted`, `text` the path's kind, `path` the KB-relative path, `section` `records`, `run`, `config` or `other`; counts `changed`, `records`, `run`, `config` and `other`.

`runs --idle AGE` (built in v0.5.0-dev.10) adds an `idle` continuation to each run row (`9d since <time> (<what was written>)`) and the count `idle`.

`find --eval` (built in v0.5.0-dev.10) answers with one row of kind `query` per line of the query file: `id` is `q<line>`, `status` is `hit@1`, `hit@5` or `miss`, `text` names the first expected record found with its rank and tier, `path` and `line` point at the query's line, the flag `unknown-id` marks an `expect` ID that is not a record, and a miss has a `first` continuation naming what ranked first. The trailer counts `queries`, `hit1`, `hit5`, `miss` and, per tier of the first expected hit, `by-exact`, `by-normalized`, `by-prefix`, `by-heading`, `by-text` and so on. A missing or empty file answers `reason` `no-queries`.

`find --kind K[,K] [--project P]` with no text lists every record of those kinds in ID order, each once, with a `match` continuation whose `text` is `listed`; there is no fifty-row cap, the budget pages the answer. Rows keep the record's title in `text`. Without `--kind`, `find` still requires text; `match` and `--signatures` always do.

### Labels the views rely on

`plan status --gates` supplies `status` on its `gate` and `item` continuations, and `gate status` on its checklist `item` continuations. Their `text` starts with `<id> [<state>]`, where the ID has no space and the state has no `]`; readers should use `status`, not parse the brackets. Judged gates append their class (except undefined gates), and checklist gates append `<covered>/<required> required covered`. Evidence locations and unmet-gate flags accompany the gate; checklist items retain the same text, location and flags as `gate status`. Later gates say `[later]` without evidence or flags; skipped steps have no gate continuations.

With `--gates` and a subject repository, `counts` includes `current` (the 1-based step position), `current_gates` and `current_owed`. Once every step is closed, only `current=0` is added; a run with no plan gets none of them. Without a subject none of these counts is present. Without `--gates`, plan status keeps its ordinary output and query digest.

Detailed steps also carry a `splice` continuation, then `under` for non-root
steps, after any existing `why` group and before gates. `splice.text` is
`<template>#<ordinal> @<digest12> from <source>`; its `path` is the recorded splice
file and `line` is 1. Static includes retain their containing splice file.
`under.text` is the parent's exact row ID and has no path. Both have empty flags
and omit `status`. Group by splice text within its parent; follow `under` links
for nesting rather than splitting IDs. These labels appear only with `--gates`.

Checklist item text marks optional items immediately after the state:
`<id> [<state>] (optional) …`, whether covered or open. A later occurrence of
`(optional)` in authored prose is not that marker.

Run timelines additionally emit `kind: "plan"` rows for splice, done and skip
events accepted by the fold. Their `status` is `spliced`, `done` or `skipped`;
`text` starts `<recorded time> <status> <id>`, with `(by WHO)` and the reason on
a skip. A splice ID is its recorded template-use prefix; done/skip IDs are step
IDs. Each row points to its event file at line 1. Invalid events are omitted.
These rows share the existing date order, with equal-time plan events ordered
by event filename. Non-run timelines and the envelope/key sets are unchanged.

`runs --plan` adds a `plan` continuation and `counts.planned`; its text holds the
recorded position and done/skipped counts, never owed-gate counts. `timeline`
on a run adds `checkpoint` rows with timestamp IDs, preview text and source
positions, and `counts.checkpoints`. `guard status` adds `guard`/`guard-rule`
rows and the counts `rules`, `deny`, `ask`, `nudge`, `invalid`. Its context note
names the current run and working directory. These additions keep the envelope
at version 1; `watch` does not introduce another JSON envelope.

A citation verdict read from a check receipt (built in v0.5.0-dev.9) carries the flag `recorded` on its `cite` row and a `more` entry with `label` `check`, `path` the receipt and `line` 1; `trailer.counts.recorded` counts such verdicts when it is not zero, and `stale --record` adds `counts["recorded-now"]` and `counts.receipts`. A checklist `item` continuation grounded that way carries the flag `recorded`. `checker list` rows may carry a `more` entry with `label` `receipts`.

The command that would move a row forward is a `more` entry with `label` `next`, its `text` the command line (`writfold kb gate run lint`). A script that wants it reads that entry rather than parsing the text form.

## The trailer

| Key | Type | Meaning |
|---|---|---|
| `coverage` | string | `complete` or `partial` |
| `records_examined`, `records_total` | integer | `records=a/b` |
| `shown`, `omitted` | integer | rows on this page, and rows held back by the budget |
| `omitted_by` | object | rows held back per section |
| `answer`, `reason` | string, string or null | `known`, or `unknown` with its reason |
| `counts` | object | the verb's own counts (`checked`, `stale`, `unverifiable`, `open` and so on) |
| `unreadable` | integer | files that could not be read |
| `needs` | integer or null | the tokens the next row needs, when not even one row fits the budget |
| `kb` | string | the KB commit, with `+dirty:N` |
| `index` | string | `fresh`, `refreshed:N`, `rebuilt` or `behind:N` |
| `cursor` | string or null | the value for `--cursor` to get the next page |
| `q` | string | the query digest |

The meanings are those of the text trailer ([kb-layout](kb-layout.md#the-trailer)).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | complete, nothing flagged |
| 2 | partial, unknown, stale or owed; the JSON is still the whole answer |
| 3 | refused: nothing is printed on stdout, and one `writfold kb: refused: …` line goes to stderr |
| 1 | an error: as for 3, stderr only; or a verb that reports a fault in its rows (`index verify` finding a difference), which prints the JSON as usual |

`guard status` has a specific read policy: exit 2 when `deny`, `ask`, `nudge` or
`invalid` is non-zero, otherwise 0. With no guard files it reports
`answer=unknown reason=no-guards` and exits 0.

## Compatibility

`writfold_kb` increases only when a change breaks a reader of the current shape: a key removed or renamed, a type changed, or a meaning changed. Otherwise keys are only added. A reader should ignore keys it does not know and check `writfold_kb` before anything else.

The key sets above are pinned by a test (`crates/writfold-kb/tests/json.rs`), which also reads this page: a key added to the output without a row here fails it.

## Which commands use it

Every `writfold kb` verb, including the writes and the run verbs: `record`, `status --set`, `checkpoint`, `propose` and `proposals`, `observe`, `repos`, `checker list`, `cite make`, `gate run|status|cover|attest|waive|lines`, `guard status`, `plan`, `deliver`, `edit apply` and `edit recover`. A write answers with the rows of what it wrote and its notes.

Two outputs are not this envelope:

- The hook commands (`writfold kb hook <event>`) print the JSON Claude Code expects.
- `writfold tuft run --json` prints Tuft's own result object, described in the [Tuft language reference](../tuft/language.md). Inside a run, a search with `--cite` is also kept as a retrieval receipt that `kb resume` lists.
