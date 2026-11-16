# When a file you wrote is wrong

> **Status:** built in v0.6.0-dev.2: diagnostics for gate definitions, guard rules, plan templates and `required.toml`, with every fault of a file reported together; `kb lint`; the `check definitions` row of `kb check`; the `diagnostics` key of `gate status`, `guard status` and `kb lint` in JSON; the hooks' compact lines. Planned: the same treatment for delivery modes, deny lists, `guard.toml` and kind declarations, for `.writfold/queries.jsonl` and `gate cover --from`, in `writfold watch`, for the sections `status --set concluded` requires, for records (`kb lint --records`), and for the user configuration file.

Writfold reads files that a person or an agent writes by hand: gate definitions, guard rules, plan templates and the required steps. When one is wrong, Writfold says where, what and, when it can, how to fix it, the way a compiler does for source code. Every fault of a file is reported at once, in file order.

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
| a verb that needs the file (`gate run`, `gate attest`, `gate waive`, `gate cover`, `plan set`, `plan use`) | exit 3; on stderr each fault drawn as above, then the one line `writfold kb: refused: <file>: N errors` last, as before; stdout stays empty |
| `gate status`, `guard status` | the `[invalid]` row stays, its text the first fault, its source that fault's line, then `help:` and `next: writfold kb lint <path>`; stderr draws up to three faults; `--json` carries them all as `diagnostics` ([json](json.md#diagnostics)) |
| `kb check` | a `check definitions` row carrying `kb lint`'s trailer, with `next: writfold kb lint` |
| `kb lint` | every definition file, below |
| the hooks | compact lines only ([hooks](hooks.md#definitions-with-errors)) |

Colour is used when the stream is a terminal, `NO_COLOR` is unset and `TERM` is not `dumb`; without it the text is the same, byte for byte, without the escape sequences. A source line is file content: every control character in it but tab is drawn as `\u{…}`, so a file cannot colour, clear or retitle a terminal.

## `kb lint`

```text
writfold kb lint [PATH…] [--repo DIR] [--deny-warnings] [--json]
```

`kb lint` reads every gate definition, guard rule, plan template and `required.toml` under the KB root's `.writfold/` and, when a repository is given with `--repo` or holds the working directory, the repository's `.writfold/`. Then it checks the references between them: the gates, delivery modes and templates (`use=`) a plan step names, the templates `required.toml` names, a guard rule's `gate:<id>`, the same name in both roots, and `use=` cycles. With `PATH`, only those files are reported (their references are still resolved against the rest).

It reads files and nothing else: it runs no command, starts no checker, spawns no Git, opens no index and writes nothing, so it works before the first run and is fast enough for a pre-commit hook or CI.

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
| `ref.undefined` | a name that should be defined elsewhere and is not: a gate, a template, a mode, a checklist item |
| `ref.wrong-kind` | defined, but not usable here (a candidate gate in a plan step) |
| `ref.cycle` | templates that include each other, or nesting past the bound |
| `file.name` | `id` or `name` does not equal the file name |
| `file.twice` | the same name defined in the KB root and the repository |
| `file.unreadable` | not UTF-8, or the file cannot be read |
| `section.missing` | a Markdown section a verb requires is absent or empty |
| `section.form` | a line in a required section lacks its form |

`syntax.json`, `value.pattern`, `section.missing` and `section.form` belong to files whose diagnostics are planned (above); they are listed so the set is complete.
