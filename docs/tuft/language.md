# Tuft v0 language

Tuft is the retrieval and projection language of the Writfold family. Weft
checks what assumptions support, Splicery describes byte-exact edits, and Tuft
finds exact evidence, retains it as citations and shows it compactly under a
token budget. Milestone T0 of
[the agent-boundary plan](../writfold/agent-boundary-plan.md) defines v0.

**What v0 is:** exact text/regex search, or partial symbol lookup from a
user-generated ctags index, over pinned Git commits. One CLI invocation can
apply a program to several mapped repositories. Retention and projection are
deterministic at a commit; an explicit working-tree mode labels changed bytes
dirty rather than inventing committed evidence.

**What v0 is not:** it is not corpus search, semantic search, ranking or
grounding. `similar`, `rank` and `follow` parse but are refused by type until
the Omnigraph backend exists (P01O/WF24 is blocked). They are never emulated
with exact matching. A Tuft result is navigation and evidence selection. It is
not a claim that the bytes are true, and it grants no authority.

## Example

```tuft
@hits := find text "trial_blob(" in "crates/"
           at "a853c87ef689ad99fc6d0fb225b0374fdc91b9cc" limit 20;
@ev   := retain @hits around lines 0;        # each hit's whole line
@view := project @ev [path, line, binding, excerpt(max_chars: 72)]
           sort [path asc, line asc] limit 6;
emit @view budget_tokens 300;
```

`writfold tuft run examples/tuft/trial-blob.tuft --repo .` prints one
`path:line: …` row per hit and a trailer (bindings depend only on the realm,
commit, path and range, so they are reproducible):

```text
crates/writfold-app/src/trial/retain.rs:90:d7fa5172…262d:     let (blob, content) = trial_blob(&executable, &git_dir, &commit, pat…+29B
crates/writfold-runtime/src/git/trial.rs:145:ccd646fc…1865: pub fn trial_blob(
# coverage=complete files=540/540 hits=2 shown=2 omitted=0 cursor=none program=9558f72e5876
```

With `--verbose` it prints the full envelope instead:

```text
tuft/0 program=sha256:9558f72e5876a4cbb5aa94911901f939ddb93feb15415d10397cca8ff3abc700 commit=a853c87ef689ad99fc6d0fb225b0374fdc91b9cc
find @hits text "trial_blob(" in "crates/" limit=20: coverage=complete files=540/540 hits=2
retain @ev range=lines(around=0) bindings=2 schema=writfold-citation/1
view @view rows=2 merged=0 limited=0 from=0 shown=2 omitted=0 budget_tokens=300 used=81 estimate=ceil((bytes+1)/4) per compact row
cursor=none
columns path line binding excerpt
crates/writfold-app/src/trial/retain.rs 90 d7fa5172…262d 2819..2891:"    let (blob, content) = trial_blob(&executable, &git_dir, &commit, pat"…+29B
crates/writfold-runtime/src/git/trial.rs 145 ccd646fc…1865 5258..5277:"pub fn trial_blob(\n"
```

(Bindings are shortened here; the tool prints all 64 hex digits.) A Weft report
cites a row as `asset excerpt = citation "<binding>";`, and `writfold trial
prepare --report` accepts it, because Tuft retains through exactly the path
`writfold trial retain` uses.

## Lexical rules

These follow Splicery's conventions.

- Source is strict UTF-8. A byte-order mark, NUL or an invalid sequence is
  refused (`T001`).
- Outside tokens, space, tab, LF and CRLF are skipped. A lone CR is refused
  (`T002`). `#` starts a comment through the next line break.
- Keywords and field names are ASCII identifiers. A `HANDLE` is `@` followed
  immediately by `[A-Za-z_][A-Za-z0-9_]*`.
- `STRING` uses double quotes and decodes exactly `\"`, `\\`, `\n`, `\r` and
  `\t`. Any other escape, control character or physical newline is refused
  (`T003`). Literal Unicode becomes its exact UTF-8 bytes, without
  normalization.
- `RX` is `rx` followed immediately by a double quote. The body is raw and
  single-line, and it ends at the next quote. Backslashes pass through. To
  match a quote, use `\x22`.
- `NAT` is `0` or `[1-9][0-9]*`, parsed exactly as a 64-bit number. A leading
  zero or overflow is refused (`T004`). `:=` is one token.

## Grammar (EBNF)

```ebnf
program   = { binding } emit ;
binding   = HANDLE ":=" operation ";" ;
operation = find | retain | project | rank | follow ;

find      = "find" matcher [ "in" STRING ] "at" STRING [ "limit" NAT ] ;
matcher   = "text" STRING | RX | "def" STRING | "refs" STRING | "similar" STRING ;
retain    = "retain" HANDLE [ "around" "lines" NAT ] ;
project   = "project" HANDLE "[" field { "," field } "]"
            [ "sort" "[" key { "," key } "]" ] [ "limit" NAT ] ;
field     = "path" | "line" | "range" | "binding"
          | "excerpt" "(" "max_chars" ":" NAT ")" ;
key       = ( "path" | "line" | "range" ) [ "asc" | "desc" ] ;
emit      = "emit" HANDLE [ "budget_tokens" NAT ] ";" ;

(* parsed only to be refused by type, T301 *)
rank      = "rank" HANDLE "by" STRING ;
follow    = "follow" HANDLE "via" STRING ;
```

`emit` must be the last statement, and there is exactly one. Bindings are
resolved in source order, a name is never rebound (`T202`), and a binding
nothing uses is an error (`T204`). So a program is one pipeline:
`find → [retain] → project → emit`.

## Kinds and checking

| Operation | Operand | Result |
|---|---|---|
| `find` | — | hits |
| `retain` | hits | retained evidence |
| `project` | hits or retained evidence | view |
| `emit` | view | output |

`binding` and `excerpt` need retained evidence (`T203`). Fields and sort keys
may not repeat (`T210`).

## `find`

- **`at`** in a saved FILE, `tuft check`, the library's `compile`, and
  `--pinned` mode must be a full 40- or 64-digit lowercase commit ID (`T206`,
  with the `git rev-parse '<value>^{commit}'` fix). `grep` and one-shot
  `run -` programs accept `HEAD`, branches, tags and abbreviations. The CLI
  resolves each repository's ref once through Git, substitutes the full ID
  into the actual program, then runs the strict compiler. The displayed
  trailer echoes `at="HEAD"→<full ID>`; JSON carries `at`.
  Receipts and citation tokens use only full commits; the program digest is
  of the substituted source, not the ref-bearing input. Unknown refs, tree
  objects and full IDs that do not name the commit itself refuse.
- **`in`** without wildcards remains a byte prefix of repository paths.
  Write `"src/"`, since `"src"` also matches `srcfoo`. With wildcards it is
  a whole-path glob: `*` and `?` match bytes other than `/`, `**` crosses `/`,
  and `**/` also matches zero directories. There are no character classes or
  shell expansion. Quote CLI globs. Empty or absent means the whole tree.
  Leading `/` and `.`/`..` segments are refused (`T211`).
- **`text "..."`** matches exact bytes, 1..=4096 bytes. It reports every
  occurrence, including overlapping starts.
- **`rx"..."`** uses the dialect `rust-regex-1.11.1/bytes-ascii-multiline-v0`:
  Rust `regex` 1.11.1 over bytes with Unicode mode off. So `\w \d \s \b` are
  ASCII and `.` is any byte except LF. `^`/`$` match at lines. Matches are
  leftmost-first and non-overlapping. Inline flags (`(?i)`), Unicode classes
  (`\p`) and backreferences are refused (`T208`). The automaton is bounded at
  1 MiB, so matching runs in linear time. A zero-width match refuses the run
  rather than inventing an empty range.
- **`limit`** (default 100, 1..=10000) bounds match positions. In the IR it is
  the revision-9 `SearchBudget` (positions = candidates = limit).
- **Coverage** is the revision-9 `Coverage`, measured by scanning. It counts
  files fully searched over files in scope at the pinned commit.
  `coverage=complete` means every file in scope was searched and every match
  is reported. Then zero hits means none exist at that commit. If more matches
  exist than `limit`, the search stops and the envelope says
  `coverage=partial files=E/T stopped=hit-limit`. It is never a silent partial.
- **Files** are every Git blob in the tree: regular, executable and symlink
  entries. Submodule gitlinks have no bytes in this repository, so they are
  not files. A non-UTF-8 path refuses the run.

### Symbols from an existing index

`find def "NAME"` and `find refs "NAME"` require `--index FILE`. Tuft imports
universal-ctags JSON lines; it does not execute ctags. Each tag needs `_type`
`tag`, `name`, a repository-relative `path`, and a positive numeric `line`.
An optional numeric `end` is inclusive. Pseudo-tag (`ptag`) metadata is
ignored. Absent roles, or the role `def`, identify definitions; non-definition
roles identify references. Only references the selected ctags language emits
can be found. A missing result is never proof that no caller exists.

`FILE.meta.json` must contain `{"commit":"<full ID>","sha256":"<64 hex>"}`.
The digest covers the exact index bytes, and the commit must equal the
resolved query commit. A mismatch refuses with a rebuild instruction.
Metadata is the index producer's declaration, not a proof of its completeness.
Generate it from a clean detached worktree, not edited source:

```sh
pin=$(git rev-parse 'HEAD^{commit}')
index=/absolute/path/tags.jsonl
git worktree add --detach /tmp/tuft-index "$pin"
(cd /tmp/tuft-index && ctags --output-format=json --fields=+nr --extras=+r -R -f "$index" .)
printf '{"commit":"%s","sha256":"%s"}\n' "$pin" "$(sha256sum "$index" | cut -d' ' -f1)" > "$index.meta.json"
git worktree remove /tmp/tuft-index
```

Symbol coverage is always `partial`, including zero results, and records
`index_commit` and `index_sha256` in the coverage line/object. Reported spans
are checked against pinned file bytes. Symbol queries cannot be combined
with `--working-tree`: the index describes committed bytes only.
SCIP, tree-sitter and new dependencies are not included.

## `retain`

`retain @hits` retains each hit's exact match bytes. `around lines N` retains
the match's whole lines plus `N` whole lines before and after. The range ends
after the last line's LF, or at end of file, and CR is ordinary content. Each
range becomes one `writfold-citation/1` binding, made by
`trial::retain::retain_read`, the same code behind `writfold trial retain`. The
retained bytes go into the trial state's object store, with `evidence/<binding>.bytes`
and `.json`. Before retaining, the adapter checks that the blob read for
retention equals the bytes the search read.

## `project`, `sort`, `limit`

The fields form a closed set:

| Field | Value |
|---|---|
| `path` | Repository path (escaping: see [`emit`](#emit)) |
| `line` | 1-based line of the row's first byte (for evidence, the first retained line) |
| `range` | Half-open file byte range `LO..HI` (for evidence, the retained range) |
| `binding` | 64-hex `writfold-citation/1` binding digest |
| `excerpt(max_chars: N)` | A prefix of the retained bytes |

`N` is 1..=4096. The excerpt shows at most `N` Unicode scalars of the longest
valid UTF-8 prefix of the retained bytes. `LO..HI` is the exact file byte range
shown. A truncated excerpt ends with `…+NB`, the retained bytes not shown.

Rows sort by the declared keys (`asc` by default), then by `(path, range)`
ascending, so the order never depends on a backend. Identical rows, with the
same path, range and binding, merge (`merged=` counts them). `limit` (1..=100000)
drops rows after sorting. They are counted as `limited=`. The cursor does not
resume them, because they are the program's own choice.

## `emit`

Three renderings page the same rows under the same budget and share cursors.

### Compact view (default)

One row per line, then one trailer line:

```text
<path>:<line>: <text>
# coverage=<complete|partial> files=<E>/<T> hits=<n>[ stopped=hit-limit] shown=<n> omitted=<n>[ limited=<n>][ needs=<t>] cursor=<v2.P.C.OFFSET.K|none> program=<12 hex>
```

- **Rows.** The projection's fields other than `excerpt` print in projection
  order joined by `:` (`range` as `LO..HI`, `binding` as 64 hex digits).
  Then, if projected, come `: ` and the excerpt text. So
  `[path, line, excerpt(…)]` prints `src/lib.rs:12: let a = 1;`, as
  `grep -n` would. `binding` and byte ranges appear only when the projection
  names them.
- **Escaping.** Only what must be escaped is:
  - a path prints raw unless it is empty, starts with `"`, or holds `:` or a
    control character;
  - excerpt text prints raw unless it holds a control character other than
    tab, starts with `"` or holds `…`;
  - after `retain … around lines`, an untruncated excerpt is whole lines,
    and its one final LF is the row end, so it is not printed. If such an
    excerpt has no final LF (the end of a file), it is escaped. After
    `retain @h` (match bytes) nothing is dropped.

  An escaped value prints as a JSON string literal of the exact shown bytes
  (`\" \\ \n \r \t`, `\u00XX`). A raw value never starts with `"`, so the
  shown bytes are always recoverable. A truncated excerpt ends with `…+NB`,
  the retained bytes not shown.
- **Trailer.** Always the last line, and never without coverage or cursor:
  it is the completeness signal. `coverage=complete` with
  `omitted=0 cursor=none` means every match in scope is shown.
  - `stopped=hit-limit`: `find`'s `limit` cut the search;
  - `omitted=N`: rows that did not fit the budget; `--cursor` resumes them;
  - `limited=N` (only when nonzero): rows dropped by the projection's own
    `limit`;
  - `needs=T` (only when no row fits): the next row's cost;
  - `program`: the first 12 hex digits of the program digest.
- **`--out FILE`** writes the shown rows (each ending in LF, no trailer) to
  `FILE` and prints only the trailer. `FILE` must not exist: it is created,
  never overwritten. With `--verbose` or `--json` it writes that document
  instead.

### Verbose envelope (`--verbose`)

```text
tuft/0 program=sha256:<program digest> commit=<commit>
find @<name> <text|rx> "<pattern>" in "<scope>" limit=<n>: coverage=<complete|partial> files=<E>/<T> hits=<n>[ stopped=hit-limit]
retain @<name> range=<match|lines(around=N)> bindings=<distinct> schema=writfold-citation/1
view @<name> rows=<n> merged=<n> limited=<n> from=<i> shown=<n> omitted=<n> budget_tokens=<b> used=<u> estimate=ceil((bytes+1)/4) per compact row
[notice next row needs <t> tokens; raise budget_tokens]
cursor=<v2.P.C.OFFSET.K|none>
columns <field> ...
<row>
```

Verbose rows print every projected field, separated by one space. A path
prints bare unless it holds whitespace, quotes, backslashes or controls,
which force a JSON-quoted form. An excerpt prints as `LO..HI:"text"`: its
exact shown file range and its text as a JSON string literal.

### Budget, cursor and determinism

- **Budget.** `budget_tokens` (default 1000, 1..=1000000) bounds the compact
  rows, whatever the rendering. Each row costs `ceil((bytes + 1) / 4)`
  estimated tokens, counting its LF. This is a conservative byte estimate,
  not a tokenizer. The trailer and envelope are always printed and sit
  outside the budget. Rows stop at a row boundary. `omitted` counts the rows
  not shown. If not even one row fits, the trailer or envelope says so and
  shows zero rows.
- **Cursor.** A resume cursor is `v2.P.C.OFFSET.K`: the cursor version,
  the first 8 hex digits of the program digest (`P`) and of the commit
  (`C`), the first row to show (`OFFSET`, decimal) and an 8-hex check (`K`),
  the first 8 hex digits of SHA-256 over
  `tuft-cursor/2\n<program digest>\n<commit>\n<offset>\n<view digest>`.
  The view digest covers every compact row, so the cursor binds program,
  commit, offset and result; index identity and the working-tree byte snapshot
  also bind it in those modes, even without excerpts. The budget is part of
  the program. Only the canonical text parses. `--cursor` reruns the same program and resumes at
  `OFFSET`, in any rendering. Refusals are typed, checked in this order:
  `Malformed`, `UnsupportedVersion` (including the earlier unversioned
  `TAG.OFFSET` form), `OtherCommit` (a stale cursor; a new pin is also a new
  program, and the commit is what is reported), `ForeignProgram`, `Tampered`
  (the check fails: the offset was edited or the result changed) and
  `OutOfRange`. It is a continuation, not a capability: the check is not a
  secret, it only turns a wrong cursor into a refusal instead of a wrong
  page.
- **Determinism.** The same program bytes and commit give byte-identical
  output in every rendering; symbol lookup also requires the same index,
  and working-tree mode the same byte snapshot. Ref labels describe the
  invocation and are not included in the pinned program digest.
- **`--json`** emits the verbose envelope and rows as one JSON object, with
  fields `tuft`, `program`, `commit`, `find`, `retain`, `view`, `budget`,
  `cursor` and `rows`.

An agent's tool note for Tuft is in [agent-note.md](agent-note.md).

### JSON contract

`run --json` and `grep --json` emit one object, not the KB envelope. Unknown
additive keys should be ignored; incompatible meaning/type changes require
a new `tuft` version. The contract test reads this page and pins the key sets.

| Key | Type and meaning |
|---|---|
| `tuft` | String, currently `tuft/0` |
| `program` | `sha256:` plus the digest of the exact pinned program |
| `commit` | Full commit ID |
| `find` | Object: `binding`, `matcher`, `pattern`, `scope` strings; `limit`, `hits` integers; `coverage` object; `stopped` null or `hit-limit` |
| `coverage` | `state` (`complete`, `partial`, `unavailable`); measured coverage also has `examined_files`, `total_files`. Symbols add `index_commit`, `index_sha256` and are always partial |
| `retain` | Null, or object with `binding` string, `around_lines` integer/null, `bindings` integer, `schema` string |
| `view` | `binding` string, `columns` string array, and integer `rows`, `merged`, `limited`, `from`, `shown`, `omitted` |
| `budget` | Integer `tokens`, `used`; string `estimate` |
| `cursor` | Continuation string or null |
| `rows` | Ordered array; each row contains only projected fields |
| `at` | Only when resolving a ref: object with `requested` input and full `commit` |
| `cites` | With `--cite`: one token string per shown row, including an empty array for no rows |
| `working_tree` | Only in that mode: integer `dirty_files`, `skipped_files` |

Row fields are `path` (string), `line` (one-based integer), `range` (two
integers, half-open byte offsets), `binding` (string/null), and `excerpt`
(object/null). An excerpt holds `range`, `text`, `truncated` (boolean), and
`hidden_bytes` (integer). Dirty bindings start `dirty:sha256:` and are not
`writfold-citation/1` bindings.

With several repositories, the top level instead has `tuft`, `repositories`
and `overall`. Each entry of `repositories` is `{"repo":"<map name>",
"result":<single-repository object>}`, sorted by name then local path.
`overall` has `coverage`, `repositories` (count), `examined_files`,
`total_files`, `hits`, `shown`, `omitted`. Overall coverage is complete only
if every repository is complete. Limits and token budgets apply per repository.

`check FILE --json` emits `tuft`, `ok` (boolean), `file`, `program`
(digest/null), and `diagnostics` (array). Each diagnostic has `code`,
`message`, `span` (`lo`, `hi` half-open source byte offsets), `line` and
`column` (one-based byte coordinates). Invalid programs still produce this
object on stdout at exit 1; filesystem/argument failures use stderr instead.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Successful query or check, including no matches, partial search coverage, or omitted rows; inspect the coverage, limits and cursor |
| 1 | Invalid arguments or program, a refusal, or an IO error; normally stderr only, except invalid `check --json` programs as above |

Tuft does not use grep's “1 means no match” convention or KB's exit 2/3.
Notes and receipt paths go to stderr; `--print-program` also sends the exact
pinned program executed for each repository there.


## Lowering and backends

The pure crate [`crates/tuft`](../../crates/tuft) holds the lexer, parser,
typed AST, checker, IR, match location, projection and rendering. It does no
IO. The adapter [`crates/writfold-app/src/tuft`](../../crates/writfold-app/src/tuft)
lowers `find` onto the B0 storage boundary ([storage-backend.md](../writfold/storage-backend.md)).
It uses `GitTreeBackend`, a read-only adapter whose pin is the commit id. Its
records are the blobs in scope: key = path, kind = `blob`, content = exact
bytes.

- `find text` becomes `lookup(Content, Substring, needle)`, one matching file
  per page, so each page's measured coverage gives the exact number of files
  searched when the limit stops the search. Positions come from `read_exact`
  bound to the digest the lookup reported.
- `find rx` has no B0 match mode. It enumerates every record in scope with
  `lookup(Kind, Exact, "blob")` and reads each one exactly.
- Fuzzy, semantic and ranked lookups and writes refuse at the boundary with
  `Unsupported`/`Unavailable`. A Dolt or Omnigraph backend can serve `find`
  later behind the same trait.

The B0 conformance suite's fixed dataset needs per-record kinds
(`document`, `service`), which a Git tree does not have. So the Git adapter is
tested against the same rules (pinned history, digest binding, missing pins,
paging, cursor binding and typed refusals) in
`crates/writfold-app/src/tuft/backend_tests.rs`. It does not claim a
`conformance::run` pass.

## Diagnostics

`file:line:column: error[Tnnn]: message`, with a 1-based line and byte column.
Recoverable lexical and parse errors are collected in source order, recovering
at statement boundaries instead of stopping at the first malformed statement.
Unrecoverable encoding errors stop decoding. Once syntax is valid, the checker
reports all semantic errors. T203 names the missing `retain @h` step when an
excerpt or binding is projected from hits. The golden cases are in
[`crates/tuft/tests/cases`](../../crates/tuft/tests/cases).

| Code | Meaning |
|---|---|
| T001 | Encoding: invalid UTF-8, BOM or NUL |
| T002 | Unexpected character or lone CR |
| T003 | Malformed string or `rx` literal |
| T004 | Malformed or out-of-range number |
| T101 | Syntax |
| T201 | Unknown binding |
| T202 | Binding defined twice |
| T203 | Operand of the wrong kind |
| T204 | Unused binding |
| T206 | Pin is not a full commit id |
| T207 | Option out of range |
| T208 | Regex outside the dialect |
| T209 | Empty or oversized text needle |
| T210 | Repeated field or sort key |
| T211 | Malformed scope |
| T301 | Deferred operator (`similar`, `rank`, `follow`), mapped to `Refusal::Unsupported(Semantic \| Ranking \| Follow)` |

## Command line

```text
writfold tuft run FILE|- [--repo DIR|NAME[@COMMIT] ...] [--pinned]
writfold tuft grep PATTERN [--in PATH-GLOB] [--at REF] [--regex] [--limit N]
                         [--repo DIR|NAME[@REF] ...] [--pinned]
writfold tuft check FILE [--json]
# run/grep: --state DIR --cursor C --verbose|--json --out NEW_FILE
#           --cite --kb DIR --run ID --working-tree --index FILE --print-program
```

`grep` lowers to ordinary `find → retain around lines 0 → project → emit`:
path, line and a 300-character excerpt, sorted by path/line, with 4,000
budget tokens. Its default hit limit is 100 and default `--at` is `HEAD`.
`--regex` selects the same bounded regex dialect as `find rx`.
`--print-program` shows the exact pinned lowering on stderr after execution.
`--at` is accepted only by `grep` and `run -`; edit a saved program's pin
instead. `--pinned` refuses any ref, including the default `HEAD`.

`--repo` defaults to `.`. Repeat it to run the same query over multiple
repositories; `NAME@REF` supplies each repository's revision. FILE input and
`--pinned` require full IDs in those selectors too. Names and paths use
exactly the KB map/search-root resolution used by `kb repos` and `kb stale`.
Duplicate repositories refuse. Text output groups each repository's rows and
coverage and ends with overall coverage. Each repository gets its own pinned
receipt. Resume one repository with its own `--cursor`, not the whole group.
There is no `repos [...]` grammar addition.

A run needs a state: the Git executable it runs, and a directory where
`retain` keeps its bindings (`evidence/<binding>.json`, `.bytes` and
`.line`). It is chosen in this order:

1. `--state DIR`, else `$WRITFOLD_STATE`: a trial state made by
   `writfold trial init`, exactly as before. An uninitialized directory is
   refused with its absolute path and how it was chosen.
2. Inside a KB under Git (found as `writfold kb` finds it, `$WRITFOLD_KB`
   included), or with `--kb DIR` (which also records a receipt, so it needs
   an open current run; [below](#kb-adapter)): Tuft's own state, `<KB git dir>/writfold/tuft-state/`. It is
   created on first use, owner-only, with only `evidence/`, and reused after
   that. Git is the executable the `kb` verbs use (`$WRITFOLD_GIT`, else
   `git` on `PATH`). No signing key, policy, ledger or worktree is made, so
   `writfold trial` commands cannot use this state, and a Weft report for
   `trial prepare` must cite evidence retained in a trial state.
3. `~/.local/share/writfold/trial`, when `writfold trial init` made it.
4. Otherwise the run is refused, naming both ways forward: run inside a KB
   or pass `--kb DIR`, or pass `--state DIR` (or set `WRITFOLD_STATE`).

Bindings keep the trial realm in either state, so the same committed bytes
bind to the same digest. `--repo` accepts ordinary repositories, linked
worktrees and submodule checkouts where `.git` is a file. Gitlinks are not
recursed into automatically: select the submodule itself as a repository.
A whole-tree `find` is bounded at 1 GiB; narrow a larger scope.

`--working-tree` is a read-only source mode over tracked and nonignored
untracked files. It never stages, commits, or changes source files. State and
optional receipts still use their normal locations. Missing, unsafe (including
symlink traversal), unreadable or oversized files are skipped and force partial
coverage; `source=working-tree dirty=N skipped=N` states the mode and counts.
The bytes searched are snapshotted and used for excerpts and tokens; files
identical to the requested commit remain pinned. Changed/untracked bytes have
`dirty:sha256:` evidence bindings, with no trial evidence descriptor, and code
tokens with `dirty=<file SHA256>` and `blob=dirty:sha256:<file SHA256>`.
Their `rev` names the base commit, not a claim those bytes occur there.
`kb stale` reports these uncommitted tokens unverifiable, even offline.

Enclosing-function/block windows are deliberately not added: language-agnostic
brace and indentation scans cannot distinguish strings, comments, data and
continuations exactly. Use the existing explicit `around lines N`.

## KB adapter

The output adapter (owner decision, 12 November 2026) shares the vocabulary of
[`writfold kb`](../writfold/kb-v1-design.md#15-tuft-adapter-as-built).
Existing pinned text queries keep their output; expansion metadata appears
only when the relevant mode is selected.

- **`--cite`** prints, under each shown row, one continuation line
  `  cite: ⟦cite kind=code repo=… rev=<pinned commit> path=… L=a-b blob=… span=… seen=… url=…⟧`.
  It is the token `writfold kb cite make` prints for the same lines at the
  same commit, made by the same code, so pasted into a KB record it checks
  `fresh` under `writfold kb stale` until the lines move (`!moved→a-b`) or
  change. `L` covers the lines of the row's range: the retained lines after
  `retain … around lines`, else the lines the match touches. `repo=` is the
  name the KB's repository map gives `--repo`, else its directory name (a
  note on standard error says when `stale` could not find it from that
  name). In the verbose envelope the line follows each row; `--json` adds a
  `cites` array beside `rows`, one token per row. The cite lines sit outside
  the budget, like the trailer, and the trailer stays the last line.
- **Retrieval receipts.** When the KB (found as `writfold kb` finds it) has
  an open current run, or with `--kb DIR` or `--run ID`, the run also
  records one receipt: a new file
  `runs/<id>/retrievals/<UTC time>-<6 hex>.md` holding the tool, program
  digest, repository, commit, row counts, the compact trailer and one token
  per distinct cited span, written through the KB's write protocol (lock,
  re-check, create-new publish). `writfold kb resume` shows the newest three.
  The receipt's path goes to standard error; standard output is unchanged.
  Without `--kb`/`--run`, a problem recording (a busy lock, say) is a note
  and the run still succeeds; with them it fails the command. A closed
  current run takes no receipt.

## Revision-9 vocabulary

v0 uses the domain's `Coverage` and `SearchBudget` directly.
`CandidateHandle` and `ScopeCut` exist only as planned names, with no code
yet. The resume cursor is deliberately not the domain's `NoticeCursor`:

- `NoticeCursor` borrows a live `NoticeQuery` (`&'a NoticeQuery<'a>` plus an
  offset). Its Verus spec is stated over that in-memory query view, it has no
  serialized form, and no wire encoding exists for it. It is a position
  inside one process.
- Tuft's cursor crosses processes as text (`--cursor`, a later CLI run or
  an agent's next tool call), so it must carry its own binding: program
  digest, commit, offset and a digest of the result.
- `NoticeCursor` positions a notice stream (Run Watch); Tuft's cursor
  positions the rows of one pinned, deterministic view. Moving onto it would
  mean either a parallel serialized `NoticeQuery` or a borrowed value that
  cannot be resumed after the process exits.

So Tuft's cursor is its own nominal type, `tuft::cursor::Cursor`, with the
canonical versioned encoding above. If a session layer later serves Tuft
views as notice streams, that layer can map a resumed `Cursor` onto a
`NoticeCursor` over its own query; the text form stays the handoff.

