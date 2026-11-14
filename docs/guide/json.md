# Reading answers as JSON

> **Status:** built in v0.5.0-dev.1 (the envelope itself has been emitted since v0.1; this page and the test that pins it are new).

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

### Labels the views rely on

`plan status --gates` supplies `status` on its `gate` and `item` continuations. Their `text` starts with `<id> [<state>]`, where the ID has no space and the state has no `]`; readers should use `status`, not parse the brackets. Judged gates append their class (except undefined gates), and checklist gates append `<covered>/<required> required covered`. Evidence locations and unmet-gate flags accompany the gate; checklist items retain the same text, location and flags as `gate status`. Later gates say `[later]` without evidence or flags; skipped steps have no gate continuations.

With `--gates` and a subject repository, `counts` includes `current` (the 1-based step position), `current_gates` and `current_owed`. Once every step is closed, only `current=0` is added. Without a subject none of these counts is present. Without `--gates`, plan status keeps its ordinary output and query digest.

`runs --plan` adds a `plan` continuation and `counts.planned`; its text holds the
recorded position and done/skipped counts, never owed-gate counts. `timeline`
on a run adds `checkpoint` rows with timestamp IDs, preview text and source
positions, and `counts.checkpoints`. `guard status` adds `guard`/`guard-rule`
rows and the counts `rules`, `deny`, `ask`, `nudge`, `invalid`. Its context note
names the current run and working directory. These additions keep the envelope
at version 1; `watch` does not introduce another JSON envelope.

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
