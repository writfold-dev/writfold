# Diagnostics for the files a user writes

> **Status:** steps 1–7 built in v0.6.0-dev.2 (the model and renderer, the reader with spans, gate definitions, guard rules and their surfaces, plan templates and references, `gate status` and JSON, `kb lint`); the guide page is [errors](../guide/errors.md). Steps 8–12 are not built and move to the next release whole. Measurements are of the v0.6.0-dev.1 release binary (commit `da63b1f`) on a scratch KB, 16 November 2026.

Writfold reads a dozen kinds of file that a person or an agent writes by hand. When one is wrong, the answer today is one line, often without a line number, sometimes about the wrong field. This spec gives every such file the same treatment a compiler gives source code: the exact file, line and column, the source line with a caret under the fault, what is wrong in plain words, and help where a fix can be named ("did you mean `timeout_seconds`?"). All faults of a file are reported together.

## 1. What is read today, and by what

### 1.1 The reader

Every `.toml` file Writfold reads goes through one hand-written reader, `crates/writfold-kb/src/gate/toml.rs` (254 lines). It accepts a subset: top-level `key = value`, comments, basic and literal strings, integers, and arrays of strings that may span lines. It is the only TOML reader in the product; no TOML crate is a dependency, and none is to be added.

What it tracks today:

- **Position:** a `Cursor` over `Peekable<Chars>` with one counter, `line`. No byte offset, no column.
- **Result:** `BTreeMap<String, Value>` with `Value::{Str(String), Int(i64), List(Vec<String>)}`. Keys and values carry no position, and map order is alphabetical, so file order is lost.
- **Errors:** `Result<_, String>`, formatted `line N: why`, and it stops at the first.

So a syntax error has a line and nothing else, and every error found after reading (an unknown key, a wrong type, an unknown value) has no position at all: by then the position is gone. This is why five of the six measured cases have no usable location.

### 1.2 Inventory

"Checks" names the file that validates the parsed values. Every one of them returns at the first fault.

| File | Read by | Checks | Error today |
|---|---|---|---|
| `.writfold/gates/<id>.toml` (KB root or repository) | `gate/toml.rs` | `gate/def.rs`, `gate/def_class.rs`, `gate/assessed/spec.rs`, `gate/checklist/spec.rs` | `gate definition: <why>`; the path is added by `gate/discover.rs` |
| `.writfold/guards/<id>.toml` (KB root or repository) | same | `guard/rules.rs` | `(path, why)`; shown at `:1` with the ID `""` |
| `.writfold/plans/<name>.toml` | same | `plan/template.rs` (and the step form `id: gate … use=<t> deliver:<mode>` in `template::step`, `deliver/word.rs`) | `plan template: <why>`, no path |
| the gates, modes and templates a plan names | — | `plan/splice.rs`, `template::closure` | `plan <t> names gate <g>, which is not defined: …`, no line |
| `.writfold/plans/required.toml` | same | `plan/required.rs` | `<path>: <why>` |
| `.writfold/deliveries/<mode>.toml` | same | `deliver/config.rs::read`, `deliver/mode.rs` | `<path>: <why>` |
| `.writfold/deliveries/deny-<audience>.toml` | same | `deliver/config.rs::deny` (each rule `name: pattern`, a regular expression) | `<path>: <why>`; a bad pattern prints the regex library's text |
| `.writfold/deliveries/guard.toml` | same | `deliver/config.rs` | `<path>: <why>` |
| `.writfold/sources/<kind>.toml` (kind declarations, including a checked kind's `checker`, `fields`, `max_age`) | same | `stale/kinds.rs` | `<path>: <why>` |
| `.writfold/queries.jsonl` (`find --eval`) | `serde_json`, one line at a time | `verbs/eval.rs` | `queries line N: <why>`, no column |
| the rows file of `gate cover --from FILE` | `serde_json`, one line at a time | `gate/checklist/batch.rs` | `--from line N: <why>` |
| Markdown records: front matter | `writfold-sources/src/documents.rs` (`parse_fields`; it already keeps the byte range of every field value) | `parse/document.rs` | the record is counted `unreadable`; the reason has no line |
| Markdown sections a verb requires (`## Hypotheses`, `## Checked against`, `## Scope and limits` for `status --set concluded`) | `parse/document.rs` | `write/conclude.rs` | `cannot conclude <id>: it needs …`, no file or line |
| JSON records (`inventory/assets/*.json` and others) | `serde_json` | `parse/json.rs` | `invalid JSON: <serde text>` (line and column are inside the text only) |
| citation tokens in Markdown | `parse/cite.rs` | `stale/` | the token is `unverifiable`, at `path:line`, no column |
| `$XDG_CONFIG_HOME/writfold/config.toml` (`weft`, `theme`) | a line scan in `workspace::user_config` | none | a malformed line is silently ignored |
| `fixtures/guard-conformance/<suite>/cases/<case>/{event,world,expected}.json` | `serde_json`, in the test harness (`tests/guard_conformance/`), never by the binary | the harness | a failed test |

Not in scope, because Writfold or another program writes them and a digest or schema check already refuses an edit: run manifests, receipts, statements, cover rows, check receipts, proposals (`.writfold/proposals/`, written by `propose`), delivery manifests. There is no checker configuration file in a KB: a checked kind names its checker, and the checker brings its own configuration. There is no plugin configuration that Writfold parses. A Weft source named by an assessed gate is read by the pinned Weft binary, whose own report is passed through.

### 1.3 What the reader must change

Spans have to come from Writfold's own reader. The change is contained:

1. **Byte offsets.** The cursor walks `&str` by byte index (`pos`), not `Peekable<Chars>`. Lines and columns are not counted while reading; a `LineIndex` (the byte offset of each line start, built once) turns an offset into `(line, col)` when a diagnostic is made.
2. **Spans on everything.** `Span { start: u32, end: u32 }` in bytes. The reader returns a `Doc`:

   ```rust
   pub struct Doc { pub entries: Vec<Entry> }                 // file order
   pub struct Entry { pub key: Spanned<String>, pub value: Slot }
   pub enum Slot { Value(Spanned<Value>), Broken }            // Broken: a syntax error was reported here
   pub enum Value { Str(Text), Int(i64), List(Vec<Spanned<Text>>) }
   pub struct Text { pub text: String, pub plain: bool }      // plain: no escape was decoded
   ```

   `Doc::get(key)` replaces `map.get(key)`, so the checking code keeps its shape. A span inside a string (the gate ID `unitt` inside `"implement: unitt"`) is the string's start plus one plus the offset in the decoded text, valid when `plain`; for a string with escapes the whole string is the span.
3. **Recovery.** A syntax error is recorded and reading continues at the next line (§4). The entry it broke is `Slot::Broken`, which silences every later check on that key.
4. **Home.** The reader moves to `crates/writfold-kb/src/defs/` (it has not been gate-only for some time): `defs/toml.rs` (entries and recovery), `defs/lex.rs` (strings, escapes, integers), `defs/fields.rs` (typed access that reports, §6). `gate::toml` stays as a re-export so `tests/guide_toml.rs` and the guide's fenced blocks keep their test.

Digests do not move: a gate's digest is computed from its checked fields, a template's from its bytes.

## 2. One model

Pure, in `crates/writfold-core/src/diag/` (no IO, no clock, no environment):

```rust
pub struct Diagnostic {
    pub severity: Severity,        // Error | Warning
    pub code: &'static str,        // stable; §2.1
    pub file: String,              // as rows show it: `.writfold/gates/unit.toml`, `<repo>:.writfold/…`
    pub at: Option<At>,            // None: about the file as a whole (a missing key)
    pub message: String,           // one sentence, the fault
    pub label: String,             // a few words printed beside the caret; may be empty
    pub help: Option<String>,      // one concrete fix
    pub notes: Vec<String>,        // context: the closed set, where an array opened
}
pub struct At { pub line: u32, pub col: u32, pub len: u32 }   // 1-based; col and len in Unicode scalar values
pub struct Report { pub sources: Vec<Source>, pub diagnostics: Vec<Diagnostic> }
pub struct Source { pub file: String, pub text: String }      // the bytes that were parsed
```

A `Report` carries the text it was made from. The renderer never re-reads a file, so the snippet always shows the bytes that were judged, and rendering costs a hook no IO.

**Wording rules.** The message says what is wrong, not what the reader was doing. Names from the file are in backticks or quotes exactly as written. No "refused:" prefix inside a message, no stacked prefixes (`refused: … refused: gate definition: …` today). The label never repeats the message. Help is present only when Writfold can name the fix.

### 2.1 Codes

A code is `<class>.<what>`, lower case, the same for every kind of file; the file says which kind. Codes are never renamed or reused; the table below is read by a test (§8), so a code without a row, or a row without a code, fails.

| Code | Meaning |
|---|---|
| `syntax.unclosed-array` | an array's `]` is missing |
| `syntax.unclosed-string` | a string's closing quote is missing on its line |
| `syntax.expected` | something else was expected here (`=`, a value, the end of the line) |
| `syntax.escape` | an unknown or malformed escape in a string |
| `syntax.subset` | valid TOML that these files do not use: a table, a dotted or quoted key, a boolean, a float, a date, a multi-line string, an inline table, a non-string in an array |
| `syntax.json` | a line or file is not JSON |
| `key.unknown` | a key this kind of file does not have |
| `key.twice` | a key given twice |
| `key.missing` | a required key is absent |
| `key.type` | the value has the wrong type |
| `key.range` | a number outside its range, an empty array where one is needed, too many items |
| `key.not-here` | a key that another class or event takes (`argv` on an attested gate, `action` on a stop rule) |
| `value.unknown` | a value outside a closed set (`class`, `event`, `action`, a requirement, a built-in check) |
| `value.form` | a string that does not have its form (`id: gate …`, `name: pattern`, `<N>d`, a relative glob, a slug) |
| `value.twice` | the same item listed twice |
| `value.pattern` | a regular expression that does not compile |
| `ref.undefined` | a name that should be defined elsewhere and is not: a gate, a template, a mode, a checklist item |
| `ref.wrong-kind` | defined, but not usable here (a repository gate in a delivery mode, a candidate gate in a plan step) |
| `ref.cycle` | templates that include each other, or nesting past the bound |
| `file.name` | `id`, `name`, `kind`, `mode` or `audience` does not equal the file name |
| `file.twice` | the same ID defined in the KB root and the repository |
| `file.unreadable` | not UTF-8, or the file cannot be read |
| `section.missing` | a Markdown section a verb requires is absent or empty |
| `section.form` | a line in a required section lacks its form (a hypothesis without an outcome) |

### 2.2 The renderer

`diag::render(report, style) -> String`, pure. `Style { colour: bool, width: usize, cell: fn(char) -> usize }` is decided by the caller.

```text
error[key.unknown]: unknown key `timout_seconds` in a gate definition
 --> .writfold/gates/unit.toml:6:1
  |
6 | timout_seconds = 600
  | ^^^^^^^^^^^^^^ not a gate key
  |
  = help: did you mean `timeout_seconds`?
```

- **Header:** `error[code]: message` or `warning[code]: message`.
- **Location:** ` --> file:line:col`; `file` alone when `at` is `None`, and then no snippet.
- **Snippet:** an empty gutter line, the source line, the caret line with the label. The gutter is as wide as the largest line number shown. One line of source only; a fact about another line goes in a note.
- **Caret:** `len` carets, at least one, clamped to the end of the line. A position one past the last character (an unclosed array) is drawn there.
- **Help and notes:** `= help: …`, then each `= note: …`.
- **Between diagnostics:** one blank line. **After the last:** `N errors, M warnings in K files`, and `… and N more` when capped (§4).
- **Width.** `width` is the terminal's when known, else 100, never under 40. A source line that does not fit is windowed around the span with `…` at the cut ends. The message, help and notes wrap at word boundaries under their own indent. `cell` gives each character's display width, so a wide character before the span does not shift the caret; the application passes the width function already linked for the terminal view, tests pass `|_| 1`. A tab is drawn as four spaces in both the source and caret lines.
- **Untrusted bytes.** The source line is file content. Every control character other than tab, including ESC, is drawn as `\u{…}`, so a file cannot colour, clear or retitle a terminal through a diagnostic.
- **Colour.** On when the stream is a terminal, `NO_COLOR` is unset and `TERM` is not `dumb`; the application decides and passes `colour`. `error` bold red, `warning` bold yellow, the gutter and arrow blue, the caret and label in the severity's colour, `help` green. Plain output is byte-for-byte the coloured output with the escape sequences removed (a test).
- **One line.** `diag::line(d)` gives the compact form used where a snippet does not fit (rows, hooks): `file:line:col: message; help`.

### 2.3 JSON

The envelope stays `writfold_kb: 1`. One key is added, `diagnostics`, an array, present on the verbs of §5 (empty when there is nothing to report) and absent elsewhere:

```json
{"severity":"error","code":"key.unknown","file":".writfold/gates/unit.toml",
 "line":6,"col":1,"len":14,
 "message":"unknown key `timout_seconds` in a gate definition",
 "label":"not a gate key","help":"did you mean `timeout_seconds`?","notes":[]}
```

`line`, `col` and `len` are null together for a file-level diagnostic; `help` is null when there is none. The source line is not included: a reader has the file. Rows are unchanged in shape; an invalid row's `line` becomes the first diagnostic's line where it was `1` or null. `docs/guide/json.md` gains the key and this object, which its pinning test requires.

## 3. "Did you mean"

One function, `diag::suggest(written, candidates) -> Vec<&str>` (at most two), pure. The candidates are the closed set **at that position**, nothing wider:

| Position | Candidates |
|---|---|
| a key | the keys of that kind of file that are allowed for its class or event when that is readable, minus keys already in the file |
| `class`, `event`, `action`, `repeat`, `mode`, `subject`, `freshness`, `audience` | the enum's values |
| a guard `require` entry | `run.current`, `gates.met`, `step.met`; after `gate:`, the defined gate IDs |
| a run-record gate's `require` | the built-in requirement names |
| a plan step's gate word | the defined gate IDs; after `deliver:`, the mode names; after `use=`, the template names |
| `required.toml` `start` / `end` | the template names |
| a mode's `checks` / `gates` | the built-in checks / the candidate gates |
| a checklist's `optional` | the gate's own item IDs |
| a required Markdown heading | the headings the verb requires, against the headings the record has |
| a JSONL key | the keys of that row |

The distance is optimal string alignment (a transposition counts once), the function `index/search.rs` already has; it moves to `writfold-core` and is re-exported. Both sides are compared lower-cased.

A candidate is offered when one of these holds:

1. **Same after normalising:** equal once case and `_`, `-`, `.` are removed (`Timeout-Seconds`, `timeoutSeconds`).
2. **Close:** distance `d ≥ 1` with `d ≤ 1` when the shorter of the two has 3 or 4 characters, `d ≤ 2` for 5 to 8, `d ≤ 3` for 9 or more. Nothing is offered for a word of one or two characters.
3. **A prefix:** the written word has at least 3 characters and is a prefix of exactly one candidate (`timeout` → `timeout_seconds`, `desc` → `description`).

Then: only the candidates at the smallest distance are kept; one is printed as ``did you mean `x`?``, two as ``did you mean `x` or `y`?``; three or more tie and nothing is suggested. When nothing is suggested and the set has at most eight members, a note lists it (`= note: a gate's classes are mechanical, observed, attested, assessed, run-record, checklist`). A larger set (gate IDs in a big KB) is not listed.

These bounds keep `id` from suggesting `items`, `tools` from suggesting `tests`, and a deliberately different name from being "corrected".

Some help is not a spelling fix and comes from the fault itself:

- a quoted number where an integer is needed: `write it without quotes: timeout_seconds = 600`;
- a string where an array is needed: `write an array: scope = ["src/**"]`;
- `[gate]`: `remove this line: keys are written at the top level`;
- `true`: `these files have no booleans; this key takes "…"`;
- a step without a colon whose words are all slugs: `did you mean "implement: unit"?`;
- a missing key: the line to add, with a typical value.

## 4. All the faults, in order, capped

**Within a file.** Reading recovers at a line boundary:

- a fault in a value (a bad escape, an unclosed string, a type the subset lacks): report, mark the entry `Broken`, continue at the next line;
- inside an array, a line that starts `bare_key =` where `,` or `]` was expected: the array is unclosed. Report it once, at the end of the last item, close the array there, and read the line as the next entry. This is the unclosed-array case: the keys after it are still read and checked;
- a line that is not `key = value` at all: report, skip the line;
- a table header: report, skip the line; the keys under it are read as top-level keys, which is what the author most likely meant.

Checking then runs every check, not the first that fails. Each check reads through `defs/fields.rs`, which pushes onto a sink and returns `None`, so a later check that needs the value does not run and no fault is reported twice.

**No cascades.** A `Broken` entry gets no further diagnostic. When the reader recovered from any syntax fault, `key.missing` is not reported for that file (a skipped line may hold the key). A key reported `key.unknown` with a suggestion does not also cause `key.missing` for the suggested key. One diagnostic per code and span.

**Across files.** A verb that reads a directory reads all of it. References between files (`ref.*`) are checked only against files that parsed; a reference to a file that exists but is invalid gets one note, ``= note: `.writfold/gates/unit.toml` exists but has errors``, not a second error.

**Order.** KB root files, then repository files; by path; within a file by line, then column; file-level diagnostics last; errors before warnings at the same position.

**Caps.** 20 per file and 100 per invocation in the full rendering; 3 on a status verb's stderr; 5 lines in a hook's context. Whatever is cut is counted: `… and 14 more in this file`, and the trailer's `diagnostics=` and `invalid=` counts are always the full numbers.

## 5. Where they appear

The rule: **stdout keeps its contract; the full rendering goes to stderr; `--json` carries the array.**

| Verb | Today | With this spec |
|---|---|---|
| `gate run`, `gate cover`, `gate attest`, `gate waive`, `plan set`, `plan use`, `deliver prepare/check`, `checker list`, `stale`, `cite make`, `find --eval`, `status --set concluded` (any verb that cannot proceed without the file) | exit 3, one `writfold kb: refused: …` line on stderr | exit 3; stderr holds the full rendering, then the same single `writfold kb: refused: <file>: N errors` line last, so a script that reads the last line still works. Stdout stays empty |
| `gate status` | a `gate-definition <path> [invalid] … !invalid` row, exit 2 | the row stays; its text is the first diagnostic's message, its source is `@path:line`; a `help:` line follows when there is one, then `next: writfold kb lint <path>`. Stderr gets the full rendering (cap 3). `--json`: `diagnostics` |
| `guard status` | a `guard-rule "" [invalid] … @path:1 !invalid` row, exit 2 | as for `gate status`; the row's ID is the file's stem |
| `plan status`, `plan show`, `deliver status` | invalid events and filings as `[invalid]` rows | unchanged; these rows are about machine-written records. A template or mode that no longer parses is reported as above |
| `kb check` | `check guards [attention] 1 guard rule files do not parse and are skipped: …` | a new row `check definitions` carrying `kb lint`'s trailer, with `next: writfold kb lint`; `check guards` stays and shortens to the count |
| `kb lint` | — | the whole-KB validation, below |

`KbError` gains one variant, `Definition(Box<Report>)`, exit code 3. Its `Display` is the compact line of its first diagnostic plus `(+N more)`, so every place that formats an error into a row or a note keeps working; the application's top level renders the full report for this variant.

### 5.1 `kb lint`

A new verb, not a mode of `kb check`. `kb check` needs an index, Git and a run, and answers "is my evidence current"; validating the files a person wrote needs none of those, must work before the first run exists and must be fast enough for a pre-commit hook and CI. `kb check` includes it as one row, so one command still covers everything.

```text
writfold kb lint [PATH…] [--repo DIR] [--deny-warnings] [--json]
```

- **Reads:** every file of §1.2's TOML rows under the KB root's `.writfold/` and, when a repository is given or encloses the working directory, its `.writfold/`; `.writfold/queries.jsonl`; then the references between them. With `PATH`, only those files (their references are still resolved against the rest). It reads files; it runs no command, starts no checker, spawns no Git, opens no index and writes nothing.
- **Cross-file checks:** a plan step's gates, modes and included templates; `required.toml`'s templates; a mode's gates; a guard rule's `gate:<id>`; a checklist's `optional`; the same ID in both roots; `use=` cycles.
- **Severity.** A fault that makes a verb refuse the file is an error. A reference that may resolve elsewhere is a warning: a KB-root rule or template naming a gate that is not in the KB root, when no repository is in view (the gate may live in the repository a run works in). With a repository in view it is an error. An `expect` ID in `queries.jsonl` is not checked here (it needs the index; `find --eval` flags it).
- **Text output:** the full rendering on stdout (it is the answer), then the trailer: `# coverage=complete files=14 clean=12 invalid=2 errors=3 warnings=1 …`. Rows are not used; `--json` gives the envelope with `rows` (one `definition <path> [clean|invalid]` row per file) and `diagnostics`.
- **Exit:** 0 no errors; 2 any error, or any warning under `--deny-warnings`; 1 a usage or IO error. Never 3. This matches `guard status` and `kb check`, and any non-zero fails a CI step.

Records are a second scope, `kb lint --records [PATH…]`: front matter (missing or unclosed fence, a duplicate key, a missing required key, at the key's line), JSON records (the parser's own line and column), malformed citation tokens with a column and a field-name suggestion from the kind's declared `fields`. It is below the cut line (§9).

### 5.2 Hooks

A hook always exits 0 and never pays for a snippet. Today `pre-tool-use` writes `writfold kb hook: guard rule <path> skipped: <why>` to stderr on every call and prints nothing on stdout, so the agent is never told, and a broken rule simply stops guarding. That it is skipped rather than treated as a denial stays as it is (a typo must not block all work); the silence does not.

- **`session-start`** adds one block to its `additionalContext` when any definition that applies here does not parse, compact lines only, at most five, then a count:

  ```text
  Definitions with errors (skipped until fixed; `writfold kb lint` shows each):
  .writfold/guards/commit.toml:5:12: unknown requirement "gates.mett"; did you mean "gates.met"? This rule is not guarding.
  .writfold/gates/unit.toml:6:1: unknown key `timout_seconds`; did you mean `timeout_seconds`? This gate cannot be run or met.
  ```

  Each line ends with the consequence in plain words. The block is absent when everything parses.
- **`stop`** adds the same block to its nudge once per session, only for guard rules (the event it would otherwise have judged).
- **`pre-tool-use`** keeps the stderr line, now in the compact form with line, column and help. It adds nothing to stdout: a pre-tool-use decision is `allow`, `deny` or `ask`, and a broken rule is none of them.
- **Cost.** The hook parses the same files it parses today, once; spans add no IO and no allocation beyond the `Doc`. The latency example (`crates/writfold-kb/examples/kb_latency`) gains a case with five broken rule files; the hook's measured time must not move beyond its noise.

### 5.3 `writfold watch`

The watch reads the same JSON replies, so it shows what `gate status` and `guard status` carry in `diagnostics`. No code names and no brackets in the main text: plain words and the location.

- **In the list:** the invalid mark that exists (`×`, or `[#]` in plain mode), the file's name, and `line 6: unknown key timout_seconds`. The count bars say `1 definition has errors` in the bad tone.
- **In the inspector,** for the selected item:

  ```text
  This gate definition cannot be read, so the gate cannot be run or met.

  .writfold/gates/unit.toml, line 6, column 1

    6 | timout_seconds = 600
      | ^^^^^^^^^^^^^^

  Unknown key timout_seconds. Did you mean timeout_seconds?
  1 more error in this file.                              key.unknown
  ```

  The snippet is drawn by the same renderer with the pane's width; the code sits dim at the right for someone who wants to search for it. For a guard rule the first sentence is `This guard rule cannot be read, so it is not guarding.`
- The watch shows the next node, not liveness; an invalid definition changes neither. A plan step that owes an unreadable gate shows that gate as not met, with the line above as its reason.

## 6. The `version = "one"` case

**Measured.** A mechanical gate with `version = "one"` and no `timeout_seconds` answers `timeout_seconds must be an integer from 1 to 86400`. With `timeout_seconds = 600` present, the same file answers `version must be an integer from 1 to 4294967295`. So the message blames a key that is not in the file and hides the one that is wrong.

**Cause.** Two defects in `gate/def.rs`, each sufficient to mislead:

1. **Absent is reported as wrong.** `int()` ends in a catch-all:

   ```rust
   match map.get(key) {
       Some(Value::Int(n)) if (low..=high).contains(n) => Ok(*n),
       _ => Err(bad(format!("{key} must be an integer from {low} to {high}"))),
   }
   ```

   The `_` arm covers three different faults: the key is absent, the value is not an integer, the integer is out of range. An absent `timeout_seconds` therefore reads as a type error about `timeout_seconds`. `list()` has the same arm: a mechanical gate without `argv` answers `argv must be a non-empty array of strings`, and one without `scope` the same for `scope` (both measured).
2. **The first check to fail wins, and the order is not the file's.** `parse` checks `schema`, `id`, `class`, then `def_class::command` (which reads `timeout_seconds`), `scope`, the assessed and checklist keys, `description`, and only then `version`. Any fault in the command keys is reported instead of a fault in `version`, however early `version` is in the file.

**Fix.** Both follow from §1.3 and §4, and neither is special to gates:

- `defs/fields.rs` has no catch-all. `int(key, range)` has three arms with three codes: absent → `key.missing` (file-level, with the line to add); not an integer → `key.type` at the value's span; out of range → `key.range` at the value's span. `list`, `text` and `need` likewise. Optional and required are separate functions, so a required key can never be read through a path that treats absence as a default.
- every check runs and reports; the result is sorted by position. `version` on line 3 is reported before a missing `timeout_seconds`, and both are reported.

The same `_ =>` shape exists in `plan/template.rs` (`steps`), `gate/checklist/spec.rs` (`items`), `gate/assessed/spec.rs` (`checks`) and the JSONL readers (`expect`); they are replaced in the same step.

**The test that would have caught it.** A table test, `tests/defs_table.rs`, over every kind of file. Each kind supplies a valid exemplar per class (the guide's fenced blocks, already parsed by `tests/guide_toml.rs`, are the exemplars) and its key table. For every key present in an exemplar:

- **wrong type:** replace the value with each other type (`"x"`, `7`, `["x"]`). Assert there is a diagnostic `key.type` whose message names that key and whose `line` is that key's line, and that **no diagnostic names a key that is not in the file** except with code `key.missing`;
- **removed:** delete a required key's line. Assert exactly one `key.missing` naming it, and no `key.type` or `key.range` at all;
- **misspelt:** drop one character of the key. Assert `key.unknown` at that line with help naming the real key, and no `key.missing` for it;
- **two at once:** a wrong `version` and a removed `timeout_seconds` give both diagnostics, `version` first.

A property test over generated files (valid exemplars with random byte edits) holds the general invariants: reading never panics and always terminates; every span lies inside the file on a character boundary; `line` and `col` point at the span's first character; a key named in backticks occurs in the file unless the code is `key.missing`; diagnostics are sorted and free of duplicates.

## 7. Compatibility

- **States stay.** `[invalid]` and `!invalid` on rows, the `invalid=` trailer counts, exit 2 from `gate status` and `guard status`, exit 3 from a refusal, and the last stderr line `writfold kb: refused: …` are unchanged. An invalid definition is still skipped by the hook and still never counts as met. Nothing invalid becomes valid and nothing valid becomes invalid: the set of accepted files is exactly today's (the exemplar and refusal lists of `gate/toml.rs`, `gate/def_tests.rs`, `tests/guide_toml.rs`, `tests/kinds.rs` and `tests/deliver_modes.rs` pass unchanged as accept/refuse verdicts).
- **JSON is additive:** the `diagnostics` key; an invalid row's `line`. `writfold_kb` stays 1.
- **Text that changes,** all of it message wording inside an existing shape:
  - the text and `@path:line` of `gate-definition … [invalid]` rows; the doubled `kb definition: refused: gate definition:` prefix goes;
  - the ID (`""` → the stem), text and line of `guard-rule … [invalid]` rows;
  - the text of `check guards`; the new `check definitions` row and `checks=` count in `kb check`;
  - the stderr of every refusal listed in §5 (more lines before the same last line);
  - the `pre-tool-use` stderr line; the `session-start` context when something is invalid.
- **Tests that pin the old wording** and are updated with the change, by file: `crates/writfold-kb/src/gate/toml.rs`, `gate/def_tests.rs`, `gate/checklist/spec.rs`, `gate/checklist/batch.rs`, `guard/rules.rs`, `plan/template.rs`, `plan/required.rs`, `plan/splice.rs`, `stale/kinds.rs`, `deliver/config.rs`, `deliver/mode.rs`, `check.rs`; `crates/writfold-kb/tests/kinds.rs`, `tests/deliver_modes.rs`, `tests/guard_conformance/world.rs`, `tests/guide_toml.rs` (the reader's new return type), `tests/json.rs` (the new key); `crates/writfold-tui/src/kb_tree.rs`, `say_kb.rs` where they quote a reason. Any guard-conformance `expected.json` that holds the old stderr line is regenerated and reviewed case by case; a decision must not change.
- **Guide examples** that print an old message are replaced with measured output of the new binary, never hand-written.

## 8. Tests, budgets, documentation

**Tests.** Besides §6's table and property tests:

- **Renderer goldens** (`writfold-core`): each of the six cases of §10 at width 100; a 300-character line windowed at width 60; a tab before the span; a wide character before the span; an ESC byte in the source line; a span at the end of the line; a file-level diagnostic; coloured output equals plain output plus escapes.
- **Suggestion table:** pairs that must suggest (`timout_seconds`, `timeout`, `Timeout-Seconds`, `gates.mett`, `unitt`, `mechancal`) and pairs that must not (`id`/`items`, `tools`/`tests`, `x`, a three-way tie).
- **Recovery:** the unclosed array reports once and the keys after it are still checked; a broken value does not produce `key.missing`; two unrelated faults give two diagnostics.
- **Codes:** every code the source can emit has a row in §2.1 and in the guide page, and the reverse.
- **End to end** (`crates/writfold/tests/`): each verb of §5 on a scratch KB with one broken file, asserting the exit code, the last stderr line, the row and the `diagnostics` entry; `kb lint` exit codes 0, 2 and 2 under `--deny-warnings`; the hook exits 0 and its context holds the compact line.
- **Mutations**, each of which must fail a named test: restore the catch-all arm in `int`; return at the first diagnostic; drop the `Broken` silencing; report `col` in bytes; widen a suggestion threshold by one; let an ESC byte through the renderer; make `kb lint` exit 0 on an error; drop the `session-start` block; sort by code instead of position.

**Budgets.** No new crate (the display-width function is the one the terminal view already links; `writfold-core` takes it as a parameter and gains no dependency). Modules at most 250 lines and functions at most 80, tests included, so: `diag/{mod,render,snippet,suggest,lines}.rs` in `writfold-core`; `defs/{toml,lex,fields,jsonl,lint,lint_refs}.rs` in `writfold-kb`. `gate/def.rs` (351 lines today) shrinks under the budget as its accessors move to `defs/fields.rs`. No `unwrap`, no `expect`, no `#[ignore]`. The renderer and `suggest` are pure and small; no proof kernel is warranted.

**Documentation.** One new guide page, `docs/guide/errors.md`: how to read a diagnostic, the code table, `kb lint` and its exit codes, what a hook and the watch show. Each existing guide keeps one short measured example at the place its file is described (`gates.md`, `hooks.md`, `run-graph.md`, `delivery-modes.md`, `checkers.md`) and links to the page. `json.md` gains the `diagnostics` key, `validation.md` the `kb lint` section and the `check definitions` row, `kb-layout.md` a link from its `.writfold/` table. The page is labelled planned until the step that builds each part lands.

## 9. Build order

One implementer, in this order; each step leaves the tree building and its tests passing.

1. **Model and renderer** in `writfold-core/src/diag/`: `Diagnostic`, `At`, `Report`, `LineIndex`, `render`, `line`, `suggest` (moving `distance`). Renderer goldens and the suggestion table.
2. **The reader with spans** in `writfold-kb/src/defs/`: byte cursor, `Doc`, recovery, `Slot::Broken`; `gate::toml` re-export. The accept/refuse lists pass unchanged; the property test.
3. **`fields.rs` and gate definitions:** the sink, the three-arm accessors, every gate check converted, `KbError::Definition`, the application's top-level rendering. §6's table test for gates. This fixes the `version` case.
4. **Guard rules, and their surfaces:** `guard status` rows, `check guards`, the hook's compact line and the `session-start` block.
5. **Plan templates, `required.toml`, and references:** step-word spans, `ref.undefined` for gates, modes and templates with suggestions; `plan set` and `plan use`.
6. **`gate status` and JSON:** the row, the stderr rendering, the `diagnostics` key, `json.md`.
7. **`kb lint`** for definitions, the `check definitions` row, exit codes, `errors.md`.

   **— cut line: the six measured cases, every `.toml` a user writes that guards or gates work, the whole-KB check and the hook are done. —**

8. **Delivery modes, deny lists, `guard.toml`, kind declarations** through the same `fields.rs` (the table test extends to them by adding their exemplars).
9. **JSONL:** `queries.jsonl` and `gate cover --from`, with the parser's column and key spans found by a small scan of the raw line.
10. **`writfold watch`:** the list reason and the inspector block.
11. **Sections and records:** `status --set concluded` with heading suggestions; `kb lint --records` for front matter, JSON records and citation tokens.
12. **Leftovers:** `config.toml` in the user's configuration directory (a warning on stderr for a line that is not `key = "value"`, once per process, never in a hook); the guard-conformance harness reporting `path:line:col` for a case file that is not JSON.

## 10. The six measured cases

Before is the release binary's output (the row or stderr line; trailers omitted). After is the full rendering.

**1. An unclosed array in a gate.**

```text
gate-definition .writfold/gates/unit.toml [invalid] kb definition: refused: gate definition: line 6: expected , or ] in an array !invalid
```

```text
error[syntax.unclosed-array]: this array is never closed
 --> .writfold/gates/unit.toml:5:24
  |
5 | argv = ["cargo", "test"
  |                        ^ expected `,` or `]` here
  |
  = help: add `]` at the end of line 5
  = note: the array opens at 5:8, and line 6 starts the next key, `timeout_seconds`
```

**2. A misspelt key.**

```text
gate-definition .writfold/gates/unit.toml [invalid] kb definition: refused: gate definition: unknown key timout_seconds !invalid
```

```text
error[key.unknown]: unknown key `timout_seconds` in a gate definition
 --> .writfold/gates/unit.toml:6:1
  |
6 | timout_seconds = 600
  | ^^^^^^^^^^^^^^ not a gate key
  |
  = help: did you mean `timeout_seconds`?
```

**3. `version = "one"`** (the file has no `timeout_seconds`).

```text
gate-definition .writfold/gates/unit.toml [invalid] kb definition: refused: gate definition: timeout_seconds must be an integer from 1 to 86400 !invalid
```

```text
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
```

**4. A guard rule with `require = ["gates.mett"]`.**

```text
guard-rule "" [invalid] require "gates.mett" is not one of run.current, gates.met, gate:<id>, step.met @.writfold/guards/commit.toml:1 !invalid
```

```text
error[value.unknown]: unknown requirement "gates.mett"
 --> .writfold/guards/commit.toml:5:12
  |
5 | require = ["gates.mett"]
  |            ^^^^^^^^^^^^ not a requirement
  |
  = help: did you mean "gates.met"?
  = note: a rule may require run.current, gates.met, step.met or gate:<id>
```

and the row: `guard-rule commit [invalid] unknown requirement "gates.mett" @.writfold/guards/commit.toml:5 !invalid`, with `help: did you mean "gates.met"?` under it.

**5. A plan step naming an undefined gate.**

```text
writfold kb: refused: plan main names gate unitt, which is not defined: add .writfold/gates/unitt.toml to the KB root or the repository
```

```text
error[ref.undefined]: step `implement` owes gate `unitt`, which is not defined
 --> .writfold/plans/main.toml:4:15
  |
4 |   "implement: unitt",
  |               ^^^^^ no .writfold/gates/unitt.toml in the KB root or the repository
  |
  = help: did you mean `unit`?

writfold kb: refused: .writfold/plans/main.toml: 1 error
```

**6. A plan step written `"implement unit"`.**

```text
writfold kb: refused: plan template: step "implement unit" must be written "id: gate gate …"
```

```text
error[value.form]: a step is written "id: gate gate …"
 --> .writfold/plans/main.toml:4:3
  |
4 |   "implement unit",
  |   ^^^^^^^^^^^^^^^^ no `:` after the step's ID
  |
  = help: did you mean "implement: unit"?

writfold kb: refused: .writfold/plans/main.toml: 1 error
```

## Decisions (owner, 16 November 2026)

1. Adopted as written. Steps 1–7 must land; steps 8–12 are built in the same round and each ships when complete, or moves to the next release whole.
2. Ships as v0.6.0-dev.2.
