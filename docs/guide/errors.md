# When a file you wrote is wrong

> **Status:** built in v0.6.0-dev.2: diagnostics for every file a person writes (gate definitions, guard rules, plan templates and `required.toml`, delivery modes, deny lists and the delivery guard, kind declarations, `.writfold/queries.jsonl`, the rows of `gate cover --from`, the sections `status --set concluded` requires, records with `kb lint --records`, and the user configuration file), with every fault of a file reported together; `kb lint`; the `check definitions` row of `kb check`; the `diagnostics` key of `gate status`, `guard status` and `kb lint` in JSON; the hooks' compact lines; `writfold watch` in plain words. The codes `value.class`, `value.unmeetable` and the warning `policy.waivable` built in v0.6.0-dev.3.

Writfold reads files that a person or an agent writes by hand: gate definitions, guard rules, plan templates and the required steps, delivery modes and deny lists, kind declarations, query and cover-row files, and the records themselves. When one is wrong, Writfold says where, what and, when it can, how to fix it, the way a compiler does for source code. Every fault of a file is reported at once, in file order.

## Reading a diagnostic

```text
$ writfold kb gate run unit
error[key.unknown]: unknown key `timout_seconds` in a gate definition
 --> .writfold/gates/unit.toml:6:1
  |
6 | timout_seconds = 600
  | ^^^^^^^^^^^^^^ not a gate key
  |
  = help: did you mean `timeout_seconds`?

writfold kb: refused: .writfold/gates/unit.toml: 1 error
```

- The first line is the severity (`error` or `warning`), the [code](#codes) in brackets, and what is wrong.
- `-->` gives the file, line and column (both 1-based, counting characters). A fault about the file as a whole, such as a missing key, gives the file alone and no source line.
- The source line is the one that was read, with carets under the fault and a few words beside them.
- `= help:` is one concrete fix, present only when Writfold can name one: a close name from the set allowed at that position ("did you mean"), or the line to add. `= note:` gives context: where an array opened, or the set a value must come from.

Two faults in one file are both reported, the earlier line first, and a fault about the file as a whole comes last:

```text
$ writfold kb gate run unit
error[key.type]: `version` must be an integer, and this is a string
 --> .writfold/gates/unit.toml:3:11
  |
3 | version = "one"
  |           ^^^^^ expected an integer from 1 to 4294967295
  |
  = help: write a number without quotes: version = 1

error[key.missing]: a mechanical gate needs `timeout_seconds`
 --> .writfold/gates/unit.toml
  |
  = help: add a line: timeout_seconds = 600   (seconds, 1 to 86400)

2 errors in 1 file

writfold kb: refused: .writfold/gates/unit.toml: 2 errors
```

A name is suggested only from the set allowed at that position (the keys of that kind of file, a gate class, the gate IDs that are defined), only when it is close (equal once case and `_`, `-`, `.` are ignored; one to three edits depending on length; or the one name the written word starts), and never for a word of one or two characters. When three or more names are equally close, none is suggested.

A fault that stops the reader (an unclosed string, a value these files do not use) is reported and reading goes on at the next line, so the keys after it are still checked. An array whose `]` is missing is reported once, where the `]` belongs:

```text
$ writfold kb gate run unit
error[syntax.unclosed-array]: this array is never closed
 --> .writfold/gates/unit.toml:5:24
  |
5 | argv = ["cargo", "test"
  |                        ^ expected `,` or `]` here
  |
  = help: add `]` at the end of line 5
  = note: the array opens at 5:8, and line 6 starts the next key, `timeout_seconds`

writfold kb: refused: .writfold/gates/unit.toml: 1 error
```

No key is reported missing in a file that had such a fault: the skipped line may hold it.

## Where they appear

| Command | What you see |
|---|---|
| a verb that needs the file (`gate run`, `gate attest`, `gate waive`, `gate cover` and `gate cover --from`, `plan set`, `plan use`, `deliver prepare` and `deliver check`, `stale`, `checker list`, `find --eval`, `status --set concluded`) | exit 3; on stderr each fault drawn as above, then the one line `writfold kb: refused: <file>: N errors` last, as before; stdout stays empty |
| `gate status`, `guard status` | the `[invalid]` row stays, its text the first fault, its source that fault's line, then `help:` and `next: writfold kb lint <path>`; stderr draws up to three faults; `--json` carries them all as `diagnostics` ([json](json.md#diagnostics)) |
| `kb check` | a `check definitions` row carrying `kb lint`'s trailer, with `next: writfold kb lint` |
| `kb lint` | every definition file, below |
| the hooks | compact lines only ([hooks](hooks.md#definitions-with-errors)) |
| `writfold watch` | plain words, below |
| the user configuration (`$XDG_CONFIG_HOME/writfold/config.toml`) | a line that is not `key = "value"` is ignored and named once per process on stderr as a warning, never in a hook |

Colour is used when the stream is a terminal, `NO_COLOR` is unset and `TERM` is not `dumb`; without it the text is the same, byte for byte, without the escape sequences. A source line is file content: every control character in it but tab is drawn as `\u{…}`, so a file cannot colour, clear or retitle a terminal.

## `kb lint`

```text
writfold kb lint [PATH…] [--repo DIR] [--deny-warnings] [--json]
```

`kb lint` reads every gate definition, guard rule, plan template and `required.toml` under the KB root's `.writfold/` (with the KB root's delivery files, kind declarations and `.writfold/queries.jsonl`, whose `expect` IDs need the index and stay with `find --eval`) and, when a repository is given with `--repo` or holds the working directory, the repository's `.writfold/`. Then it checks the references between them: the gates, delivery modes and templates (`use=`) a plan step names, the templates `required.toml` names, a guard rule's `gate:<id>`, the same name in both roots, and `use=` cycles. With `PATH`, only those files are reported (their references are still resolved against the rest).

It reads files and nothing else: it runs no command, starts no checker, spawns no Git, opens no index and writes nothing, so it works before the first run and is fast enough for a pre-commit hook or CI. Without `PATH` it also reads the runs' observation files, for one warning (since v0.6.0-dev.11): `policy.never-ages`, once per source kind with no declaration in `.writfold/sources/` that has `no-data` or `not-found` looks, which never age ([gates](gates.md#observations-as-evidence-built-in-v050-dev3)).

Over a KB with a gate whose class and one key are misspelt, a guard rule with an unknown requirement and action, a template naming an undefined gate and template, and a `required.toml` naming an undefined template:

```text
$ writfold kb lint
error[value.unknown]: unknown class "mechancal"
 --> .writfold/gates/lint.toml:4:9
  |
4 | class = "mechancal"
  |         ^^^^^^^^^^^ not a gate class
  |
  = help: did you mean "mechanical"?

error[key.unknown]: unknown key `timout_seconds` in a gate definition
 --> .writfold/gates/lint.toml:6:1
  |
6 | timout_seconds = 600
  | ^^^^^^^^^^^^^^ not a gate key
  |
  = help: did you mean `timeout_seconds`?

error[value.unknown]: unknown requirement "gates.mett"
 --> .writfold/guards/commit.toml:5:12
  |
5 | require = ["gates.mett", "gate:unitt"]
  |            ^^^^^^^^^^^^ not a requirement
  |
  = help: did you mean "gates.met"?
  = note: a rule may require run.current, gates.met, step.met or gate:<id>

error[value.unknown]: unknown action "warn"
 --> .writfold/guards/commit.toml:6:10
  |
6 | action = "warn"
  |          ^^^^^^ not an action
  |
  = note: one of deny or ask

error[ref.undefined]: step `implement` owes gate `unitt`, which is not defined
 --> .writfold/plans/main.toml:4:15
  |
4 |   "implement: unitt",
  |               ^^^^^ no .writfold/gates/unitt.toml in the KB root or the repository
  |
  = help: did you mean `unit`?

error[ref.undefined]: step `review` names template `reveiw`, which is not defined
 --> .writfold/plans/main.toml:5:17
  |
5 |   "review: lint use=reveiw",
  |                 ^^^^^^^^^^ no .writfold/plans/reveiw.toml in the KB root or the repository

error[ref.undefined]: the required steps names template `mian`, which is not defined
 --> .writfold/plans/required.toml:2:10
  |
2 | start = ["mian"]
  |          ^^^^^^ no .writfold/plans/mian.toml in the KB root or the repository
  |
  = help: did you mean `main`?

7 errors in 4 files

# coverage=complete files=5 clean=1 invalid=4 errors=7 warnings=0
```

A reference that may resolve elsewhere is a warning: a KB-root template or rule naming a gate that the KB root does not define, when no repository is in view (the gate may live in the repository a run works in). With a repository in view it is an error. A reference to a file that exists but has errors is not reported again; the file's own faults are.

| Exit | Meaning |
|---|---|
| 0 | no errors (warnings allowed) |
| 2 | at least one error; or, with `--deny-warnings`, at least one warning |
| 1 | a usage or IO error (an unknown option, a `PATH` that is not a definition file) |

`kb lint` never exits 3. `--json` gives the envelope with one row per file (`definition <path> [clean|invalid]`), the trailer's counts as fields, and `diagnostics`.

## Other files you write

Delivery modes, deny lists and the delivery guard, and kind declarations are read the same way:

```text
$ writfold kb plan set ship
error[value.unknown]: paragraphs.bonded is not a built-in check; list a gate under gates
 --> .writfold/deliveries/release-note.toml:3:11
  |
3 | checks = ["paragraphs.bonded"]
  |           ^^^^^^^^^^^^^^^^^^^ not a built-in check
  |
  = help: did you mean "paragraphs.bounded"?

error[ref.undefined]: unknown gate `reveiw`: define .writfold/gates/reveiw.toml in the KB root
 --> .writfold/deliveries/release-note.toml:4:10
  |
4 | gates = ["reveiw"]
  |          ^^^^^^^^ no such candidate gate

2 errors in 1 file

writfold kb: refused: .writfold/deliveries/release-note.toml: 2 errors
```

```text
$ writfold kb deliver prepare --mode pr-comment --audience external --from draft.md
error[value.pattern]: rule hosts: the pattern is not a regular expression
 --> .writfold/deliveries/deny-external.toml:3:18
  |
3 | rules = ['hosts: (internal|corp\.example']
  |                  ^^^^^^^^^^^^^^^^^^^^^^^ does not compile
  |
  = note: unclosed group

writfold kb: refused: .writfold/deliveries/deny-external.toml: 1 error
```

The JSON-lines files (`.writfold/queries.jsonl` for `find --eval`, the rows of `gate cover --from`) are read one object per line; a line that is not JSON is reported at the parser's own column, and a key or value at its place in the line:

```text
$ writfold kb gate cover review --from rows.jsonl --by smoke
error[value.unknown]: outcome "suported" is not supported, unsupported or not-applicable
 --> --from rows.jsonl:1:28
  |
1 | {"item":"errors","outcome":"suported","refs":["gate:unit"]}
  |                            ^^^^^^^^^^ not an outcome
  |
  = help: did you mean "supported"?

writfold kb: refused: --from rows.jsonl: 1 error
```

`status <investigation> --set concluded` names each missing or empty section, at the heading that may be meant when one is close, and each hypothesis still open or without an outcome at its line:

```text
$ writfold kb status 2026/2026-06-01-flush-stalls-again --set concluded
error[section.missing]: concluding needs a ## Checked against section
 --> investigations/2026/2026-06-01-flush-stalls-again/README.md
  |
  = help: add a section: ## Checked against
  = note: an investigation is concluded with ## Hypotheses, ## Checked against and ## Scope and
          limits

error[section.missing]: concluding needs a ## Scope and limits section
 --> investigations/2026/2026-06-01-flush-stalls-again/README.md
  |
  = help: add a section: ## Scope and limits
  = note: an investigation is concluded with ## Hypotheses, ## Checked against and ## Scope and
          limits

2 errors in 1 file

writfold kb: refused: investigations/2026/2026-06-01-flush-stalls-again/README.md: 2 errors
```

## Records: `kb lint --records`

`kb lint --records [PATH…]` reads every record of the KB instead of the definition files: a JSON record that is not JSON (at the parser's line and column), a front matter fence that is missing or never closed, a key given twice, a required key absent (`id` of a decision, `ticket` of a ticket), a citation token that does not read, and a token missing a field its kind declares (`required` in [the kind declaration](kb-layout.md)), with the written field it may mean. Exit codes are those of `kb lint`.

```text
$ writfold kb lint --records
error[syntax.json]: this record is not JSON: expected `:`
 --> inventory/assets/broken.json:2:10
  |
2 |   "name" "X"}
  |          ^ here

error[key.twice]: `status` is given twice
 --> ktlo/2026/hbr-9-twice.md:4:1
  |
4 | status: done
  | ^^^^^^ given again
  |
  = note: first given on line 3

2 errors in 2 files

# coverage=complete files=32 clean=30 invalid=2 errors=2 warnings=0
```

## In `writfold watch`

The Gates and Guards views show a definition that does not parse in plain words: the invalid mark, the file's name and the first fault with its line; the counts say how many definitions have errors; a block below says what it means, where, the source line with its caret, the fault and its fix, how many more, and the code, dim, for searching. A plan step owing a gate whose definition stopped parsing shows that gate as not met, with the reason under it. Captured in a 150-column terminal (`NO_COLOR`, so the invalid mark is `#`):

```text
1 gate · 1 with nothing recorded yet · 1 invalid · 1 not met
· 1 definition has errors

o unit   nothing recorded yet · checked by a command
# lint.toml   line 4: unknown class "mechancal"

This gate definition cannot be read, so the gate cannot be
run or met.

.writfold/gates/lint.toml, line 4, column 9

  4 | class = "mechancal"
    |         ^^^^^^^^^^^

Unknown class "mechancal". Did you mean "mechanical"?
1 more error in this file. · value.unknown
```

## Codes

A code is `<class>.<what>`. Codes are never renamed or reused; the same code means the same thing in every kind of file.

| Code | Meaning |
|---|---|
| `syntax.unclosed-array` | an array's `]` is missing |
| `syntax.unclosed-string` | a string's closing quote is missing on its line |
| `syntax.expected` | something else was expected here (`=`, a value, the end of the line) |
| `syntax.escape` | an unknown or malformed escape in a string |
| `syntax.subset` | valid TOML these files do not use: a table, a dotted or quoted key, a boolean, a float, a date, a multi-line string, an inline table, a non-string in an array |
| `syntax.json` | a line or file is not JSON |
| `key.unknown` | a key this kind of file does not have |
| `key.twice` | a key given twice |
| `key.missing` | a required key is absent |
| `key.type` | the value has the wrong type |
| `key.range` | a number outside its range, an empty array where one is needed, too many items |
| `key.not-here` | a key that another class or event takes (`argv` on an attested gate, `action` on a stop rule) |
| `value.unknown` | a value outside a closed set (`class`, `event`, `action`, a requirement) |
| `value.form` | a string that does not have its form (`id: gate …`, a relative glob, a slug) |
| `value.twice` | the same item listed twice |
| `value.pattern` | a regular expression that does not compile |
| `value.class` | a standing the gate's class cannot reach in `accept` (`a mechanical gate is never attested`) |
| `value.unmeetable` | an `accept` list without the class's own standing: the gate could never be met by its own evidence |
| `value.conflict` | two settings that cannot stand together: a mode gate named in a mode's `waivable` (its own `accept` decides), a guard route with `field` and `body`/`file`, a route with none of them, a route for a command or tool also under `commands` or `tools` |
| `value.no-exit` | a plan step loops (`loop=`) with no gate and `min=1`: nothing of its own ends it (from v0.6.0-dev.15) |
| `ref.undefined` | a name that should be defined elsewhere and is not: a gate, a template, a mode, a checklist item |
| `ref.wrong-kind` | defined, but not usable here (a candidate gate in a plan step; an `items_from` gate that is not mechanical or does not retain the file) |
| `ref.cycle` | templates that include each other, or nesting past the bound |
| `file.name` | `id` or `name` does not equal the file name |
| `file.twice` | the same name defined in the KB root and the repository |
| `file.unreadable` | not UTF-8, or the file cannot be read |
| `file.unlisted` | a policy directory exists but could not be listed (in whole or part-way): the answers that depend on it are partial (from v0.6.0-dev.14) |
| `section.missing` | a Markdown section a verb requires is absent or empty |
| `section.form` | a line in a required section lacks its form |
| `policy.waivable` | a warning: a checklist restricts `not_applicable` while its gate still accepts a waiver |
| `policy.never-ages` | a warning: `no-data` or `not-found` looks of a source kind with no declaration in `.writfold/sources/`, which never age (built in v0.6.0-dev.11) |
| `policy.self-exit` | warning: a loop step (`loop=`) ends only on gates its walker can meet with a statement (attested or checklist class, or accepting a waiver) (from v0.6.0-dev.15) |
| `policy.narrow-scope` | warning: a generated checklist's scope gate (`items_from`) has a `scope` narrower than `**`, so a related record added outside it leaves the scope current (from v0.6.0-dev.24) |
| `alias.collides` | a warning from `kb lint --records`: every word of an alias in a Markdown record's front matter is in another record's ID, title or alias, so phrases using it may find that record (built in v0.6.0-dev.12) |

