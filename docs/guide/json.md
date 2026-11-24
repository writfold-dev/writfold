# Reading answers as JSON

> **Status:** built in v0.5.0-dev.1 (the envelope itself has been emitted since v0.1; this page and the test that pins it are new). `matched_on`, the `find --eval`, `guard log`, `runs --idle` and `delta` answers built in v0.6.0-dev.1. The policy keys (`accept`, `accepted`, `allowed`, `locked`, `summary`, `passed`, `attested`, `waived`, `policy`, `identity`) and `kb init`'s envelope built in v0.6.0-dev.3. The delivery keys (on `delivery` rows and `check` continuations, `posted` and `withdrawn` continuations, `deliver match`, `deliver payload`, `deliver modes` and `kb policy`'s `mode` and `route` rows) built in v0.6.0-dev.4. `find --each` built in v0.6.0-dev.6.

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
| `flags` | array of strings | the `!` flags, without the `!`. Some ask for a re-check at the source (`stale`, `moved…`, `changed`, `gone`, `expired`, `superseded…`); others only describe the row (`current`, `draft`, `recorded`, `store`, `suggested`) |
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

`find` and `match` rows carry one more key (built in v0.6.0-dev.1), absent on every other verb's rows:

| Key | Type | Meaning |
|---|---|---|
| `matched_on` | string | the tier that found the row: `signature`, `exact`, `normalized`, `prefix`, `heading`, `text`, `substring`, `typo` or `text-any`, and `listed` for a `--kind` listing; `suggestion` for a name offered when nothing matched (status `suggested`, flag `suggestion`), which is not a hit. `exact`, `normalized` and `text` are spelt as other tools' readers expect; a reader that knows only those ignores the rest. The `match` continuation stays, with its path and line |

Readers should use `matched_on`, not split the `match` continuation's text (`typo d=1 api`).

`guard status` (from v0.6.0-dev.1) adds the flags `audit` and `waived`, the continuation labels `mode` and `waived`, and the count `audit` (present only when not zero). `guard log` answers rows of kind `guard-log` (status `audit`, `enforce` or `removed`) with continuations labelled `would-deny`, `would-ask`, `deny`, `ask`, `earlier` and `next`, the flags `audit`, `would-deny`, `would-ask` and `unmatched`, and the counts `rules`, `entries`, `would-deny`, `would-ask`, `deny`, `ask` and `unparsed`.

`delta` (built in v0.6.0-dev.1) answers rows of kind `delta`, one per changed path: `id` the record ID or the path, `status` `added`, `modified` or `deleted`, `text` the path's kind, `path` the KB-relative path, `section` `records`, `run`, `config` or `other`; counts `changed`, `records`, `run`, `config` and `other`.

`runs --idle AGE` (built in v0.6.0-dev.1) adds an `idle` continuation to each run row (`9d since <time> (<what was written>)`) and the count `idle`.

`find --eval` (built in v0.6.0-dev.1) answers with one row of kind `query` per line of the query file: `id` is `q<line>`, `status` is `hit@1`, `hit@5` or `miss`, `text` names the first expected record found with its rank and tier, `path` and `line` point at the query's line, the flag `unknown-id` marks an `expect` ID that is not a record, and a miss has a `first` continuation naming what ranked first. The trailer counts `queries`, `hit1`, `hit5`, `miss` and, per tier of the first expected hit, `by-exact`, `by-normalized`, `by-prefix`, `by-heading`, `by-text` and so on. A missing or empty file answers `reason` `no-queries`.

The policy keys (built in v0.6.0-dev.3), each absent unless it applies: a gate row gains `accept` (the list as written, in the order satisfied, attested, waived) when the definition writes it and `accepted: false` when its standing is a met word `accept` leaves out; a checklist item (`more` entry) gains `allowed` (the outcomes that cover it) when the checklist restricts them and `accepted: false` when a current row's outcome is not allowed; an item's `status` may be `none-found`; a `plan status` step row gains `locked: true` when the run's pin locks it; the plan row of `plan status` and `resume` gains `summary`, `passed`, `attested` and `waived`. `plan status`'s trailer gains `policy` (`same`, `moved` or `unpinned`), `summary`, `passed`, `attested` and `waived`, and the count `repinned`; `kb policy`'s trailer gains `policy` and `identity`. `plan repin` (built in v0.6.0-dev.6) answers as `plan status` does under the verb `plan-repin`; each repin is a `more` entry of the plan row with the `label` `repin` (`text` such as `1 (note) by owner: adopted the stricter rubric`, `path` the pin file, `line` 1), and no key is added. `kb init` answers its own envelope: rows of kind `init`, `status` `wrote` or `kept`, and a trailer with `wrote`, `kept` and `dry_run`.

`resume --packet` (built in v0.6.0-dev.19) answers under the verb `resume` with the arguments `[<run>, "packet"]`. Its rows are `resume`'s, in a new order, and its trailer gains `required`, `required_omitted` and `optional_omitted` (counts) and `packet` (`complete` or `partial`). A required row that did not fit sets `answer: unknown` with `reason: budget`.

Delivery check receipts (built in v0.6.0-dev.20) may carry `result.advice`, a list of `{line, rule, detail}`, and `result.advice_total`. Both are written only when there is advice. In `deliver status`, a check row's `more` entry gains `advice` (the same list) when its receipt has some. Advice never changes `outcome`, `findings` or an exit status.

`find --each Q…` (built in v0.6.0-dev.6) answers several queries in one envelope, in the order given. Each query opens with a row of kind `query` (`id` its position from 1, `status` `hit` or `none`, `text` the query, and two more keys: `query`, the same text, and `hits`, the number of rows shown for it), followed by the first five rows a lone `find` gives for it (for a record's own ID, from v0.6.0-dev.11, only the `exact` row: the same first row), each carrying the `query` key beside `matched_on`. A query with more rows than five is flagged `more` (ask it alone for the rest). A query that matched nothing has `hits: 0` and any suggested names under it, flagged `suggested`. `trailer.counts` holds `queries` and `miss`; any miss exits 2.

```console
$ writfold kb find --each "harbor api" "zzqx" --json
{"notes":["1 of 2 queries matched nothing; rows flagged !suggested are near names, not hits"],"rows":[{"flags":["more"],"hits":5,"id":"1","kind":"query","line":null,"more":[],"path":null,"query":"harbor api","section":null,"status":"hit","text":"harbor api"},{"flags":["past-ttl"],"id":"harbor-api","kind":"asset","line":10,"matched_on":"normalized","more":[…],"path":"inventory/assets/harbor-api.json","query":"harbor api","section":null,"status":"active","text":"Harbor billing API"},…,{"flags":[],"hits":0,"id":"2","kind":"query","line":null,"more":[],"path":null,"query":"zzqx","section":null,"status":"none","text":"zzqx"}],"trailer":{"answer":"known","counts":{"miss":1,"queries":2},"coverage":"complete",…,"shown":7,…},"verb":"find","writfold_kb":1}
```

`context` and `uses` (built in v0.6.0-dev.10) end, when a store is declared, with rows of kind `material` (section `material`, flag `store-claim`, `status`, `path` and `line` null, `text` the README's first heading or the directory), each with five more keys: `store`, `commit` (the full commit read), `dir`, `subject` (the element of `subject:` that listed it) and `claim` (always `store`). Its continuations are `at` (a code locator of the `README.md`) and `via` (`subject: <id>`). The trailer then gains `stores`, one object per declared store: `name`, `state` (`read`, `over-limit`, `missing`, `mismatch`, `partial-clone`, `no-commit` or `git-error`), `commit` (null when none was read) and `via` (`map`, `search-root`, `hint`, or empty); `counts.material` is the number of rows. With no store declared none of this is present ([stores](stores.md#material-behind-a-record)).

An observation recorded with `--snapshot` (built in v0.6.0-dev.10) carries a `snapshot` string beside `cite` in its file; a `looked` continuation whose newest look kept a snapshot has the flag `snapshot`.

`find --kind K[,K] [--project P]` with no text lists every record of those kinds in ID order, each once, with a `match` continuation whose `text` is `listed`; there is no fifty-row cap, the budget pages the answer. Rows keep the record's title in `text`. Without `--kind`, `find` still requires text; `match` and `--signatures` always do.

### Diagnostics

`gate status`, `guard status` and `kb lint` (built in v0.6.0-dev.2) add one top-level key, `diagnostics`: an array, empty when every definition file they read parses, absent on every other verb. Each entry is one fault in a file a person wrote ([errors](errors.md)):

```json
{"severity":"error","code":"key.unknown","file":".writfold/gates/unit.toml",
 "line":6,"col":1,"len":14,
 "message":"unknown key `timout_seconds` in a gate definition",
 "label":"not a gate key","help":"did you mean `timeout_seconds`?","notes":[]}
```

| Key | Type | Meaning |
|---|---|---|
| `severity` | string | `error` or `warning` |
| `code` | string | the stable code, such as `key.unknown` ([the codes](errors.md#codes)) |
| `file` | string | the file as rows show it; a repository's file is `<repo>:.writfold/…` |
| `line`, `col`, `len` | integers, or all three null | 1-based line and column, and the length, in characters; null for a fault about the file as a whole (a missing key) |
| `message` | string | what is wrong, in one sentence |
| `label` | string | the few words shown beside the caret; may be empty |
| `help` | string or null | one concrete fix, when Writfold can name it |
| `notes` | array of strings | context: the set a value must come from, where an array opened |

The source line is not included: a reader has the file. The row of a definition that does not parse keeps its shape; its `line` is the first fault's line.

### Labels the views rely on

`plan status --gates` supplies `status` on its `gate` and `item` continuations, and `gate status` on its checklist `item` continuations. Their `text` starts with `<id> [<state>]`, where the ID has no space and the state has no `]`; readers should use `status`, not parse the brackets. Judged gates append their class (except undefined gates), and checklist gates append `<covered>/<required> required covered`. Evidence locations and unmet-gate flags accompany the gate; checklist items retain the same text, location and flags as `gate status`. Later gates say `[later]` without evidence or flags; skipped steps have no gate continuations.

With `--gates` and a subject repository, `counts` includes `current` (the 1-based step position), `current_gates` and `current_owed`. Once every step is closed, only `current=0` is added; a run with no plan gets none of them. Without a subject none of these counts is present. Without `--gates`, plan status keeps its ordinary output and query digest.

A delivery a step owes (built in v0.6.0-dev.1; see [run-graph](run-graph.md#a-delivery-as-a-steps-gate)) is a gate whose ID starts `deliver:` (`deliver:pr-comment`). Under `plan status --gates`, in template order, it has a `gate` continuation whose `text` is `deliver:<mode> [<standing>] delivery <met>/<total> checks met` (or `delivery no candidate`, or `deliver:<mode> [later]` on a later step) with `path` the deciding candidate's payload; then a `candidate` continuation (`status` `ready` or `blocked`; `text` `<id> [<status>] <mode> audience=<a> destination=<d> candidate=<digest12> bytes=<n> seq=<n>`; flags `blocked`, `edited`); then one `item` continuation per obligation of the mode, whose `text` is the `check:` text of `deliver status` and whose `status` is the obligation's standing (flag `stale` when cited evidence moved). `current_gates` and `current_owed` count deliveries. Without `--gates`, `plan status`, `plan advance`, `resume` and the session-start text carry one `candidate` continuation on the current step (`<id> [<status>] owes <check> [<standing>], …`, `path` the payload), or a `why` continuation when no mode can meet the delivery.

`deliver status`, and every verb that prints its rows, supplies `status` (the obligation's standing) on each `check` continuation, and a `step` continuation under a candidate a plan step is judged on: `decides deliver:<mode> at <step> [current]` or `bound to <step> [done]`. A candidate's manifest (`delivery.json`) gains `seq`, its place among the run's candidates of its mode (absent reads as 0). A done record names a delivery as a `met` entry whose `gate` is `deliver:<mode>`, `evidence` the payload path and `content` its digest.

The delivery keys (built in v0.6.0-dev.4), all additive; `writfold_kb` stays 1. Readers should use them and stop splitting `text`:

| Where | Key | Type | Meaning |
|---|---|---|---|
| `delivery` row | `mode`, `audience`, `destination` | string (`destination` may be null) | from the manifest |
| | `digest`, `payload_digest` | string (`payload_digest` may be null) | the candidate's present digest; the digest of its payload under the mode's rendering |
| | `bytes`, `seq` | integer | the candidate's size; its place among the run's candidates of its mode |
| | `ready` | boolean | the candidate is ready |
| | `posts` | integer | how many valid post records it has |
| | `next` | string, absent when nothing is owed | the `next:` line's text |
| | `payload` | string | `deliver payload --json` only: the payload's text |
| `check` continuation | `check`, `class` | string | the obligation's name; `mechanical`, `observed` or `attested` |
| | `outcome` | string, absent with no receipt | `pass`, `fail` or `unknown`, from the deciding receipt |
| | `findings` | array, absent with no receipt | `{line, column, rule, detail}` per finding of the receipt (as many as it keeps); `column` absent when the finding has none |
| `posted` continuation | `status` | string | `attested` (a record about the present bytes) or `stale` (the candidate was edited after it); `path` the record |
| `withdrawn` continuation | | | `text` `by <who>: <reason>`, `path` the withdrawal |
| `match` continuation | | | `deliver match` only: `these bytes are this candidate's payload (<digest>)` |

The row status may be `withdrawn`. New flags: `withdrawn` (row), `posted` (row, a post record about the present bytes exists), `not-accepted` (a `check` continuation whose statement the policy does not take, with a `why` continuation after it) and `unready-when-recorded` (a `posted` continuation). Under a candidate, `step` may read `decides no step: <id>, a newer <mode> candidate, is bound to <step> [done]`. On a plan step, `posted:<mode>` is a gate like `deliver:<mode>` (its `candidate` line may be followed by a `posted` continuation), a met delivery with no `posted:` word adds a `note` continuation, and a done record names it as a `met` entry whose `gate` is `posted:<mode>`, `standing` `attested`, `evidence` the post record and `content` the candidate's digest.

`deliver status`'s trailer gains the count `withdrawn`. `deliver match` answers one `delivery` row or none (with `match` and `posted` continuations), the counts `matched` (candidates holding these bytes) and `postable`, and, when the bytes do not match, `reason` in the trailer with the guard's reason. `deliver payload --json` (or `--to FILE`) answers one `delivery` row with `payload` and `payload_digest`. `deliver modes` answers rows of kind `mode` (`id` the mode, `section` `modes`, `path` the mode file, or none for a built-in mode; status `invalid` when the file does not load) with `audience` (string, or null when `--audience` decides), `rendering`, `paragraph_words` and `waivable` (the obligations the mode takes a waiver of), continuations `check`, `read`, `why`, `deny` and `route`, and the count `modes`. `kb policy` adds the same `mode` rows and rows of kind `route` (`id` the command or tool, `text` `mode=<mode> body=… file=…` or `mode=<mode> field=…`).

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
| 3 | refused: nothing is printed on stdout, and one `writfold kb: refused: …` line goes to stderr, last (a refusal over a file a person wrote draws each fault above it, [errors](errors.md)) |
| 1 | an error: as for 3, stderr only; or a verb that reports a fault in its rows (`index verify` finding a difference), which prints the JSON as usual |

`guard status` has a specific read policy: exit 2 when `deny`, `ask`, `nudge` or
`invalid` is non-zero, otherwise 0. With no guard files it reports
`answer=unknown reason=no-guards` and exits 0.

## Compatibility

`writfold_kb` increases only when a change breaks a reader of the current shape: a key removed or renamed, a type changed, or a meaning changed. Otherwise keys are only added. A reader should ignore keys it does not know and check `writfold_kb` before anything else.

The key sets above are pinned by a test (`crates/writfold-kb/tests/json.rs`), which also reads this page: a key added to the output without a row here fails it.

## Which commands use it

Every `writfold kb` verb, including the writes and the run verbs: `record`, `status --set`, `checkpoint`, `propose` and `proposals`, `observe`, `repos`, `checker list`, `cite make`, `gate run|status|cover|attest|waive|lines`, `guard status`, `plan` and `deliver`. A write answers with the rows of what it wrote and its notes.

Two outputs are not this envelope:

- The hook commands (`writfold kb hook <event>`) print the JSON Claude Code expects.

## Loops in `plan status` (from v0.6.0-dev.15)

A loop step's row has `"loop": {"template", "min", "max", "rounds", "exit"}`, where `exit` is `"met"`, `"again"` or `"spent"` when the step's gates were judged (the current or a done step) and `null` otherwise. A step inside a loop round has `"round"` (its round's number) and `"superseded"` (`true` for an earlier round). The first step of a round made by `plan again` has `"until"`: each of the loop step's gates as it stood when the round began, `{"gate", "standing", "evidence"}` (`evidence` only when one stood). The trailer counts `loops` and `spent` when the plan has a loop; `!over-max` on a loop step makes the answer exit 2.
