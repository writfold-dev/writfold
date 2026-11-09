# Tuft v0 language

Tuft is the retrieval and projection language of the Writfold family. Weft
checks what assumptions support, Splicery describes byte-exact edits, and Tuft
finds exact evidence, retains it as citations and shows it compactly under a
token budget. Milestone T0 of
[the agent-boundary plan](../writfold/agent-boundary-plan.md) defines v0.

**What v0 is:** exact search over **one pinned Git commit**, retention of the
matched bytes as `writfold-citation/1` bindings through the trial retain path,
and a deterministic, source-mapped projection with explicit omissions and a
resume cursor.

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

`writfold tuft run examples/tuft/trial-blob.tuft --repo .` prints (bindings
depend only on the realm, commit, path and range, so they are reproducible):

```text
tuft/0 program=sha256:62539951f4af1345b03a3f5cd37885128b1c10bc4cf5c231893365328b9763db commit=a853c87ef689ad99fc6d0fb225b0374fdc91b9cc
find @hits text "trial_blob(" in "crates/" limit=20: coverage=complete files=540/540 hits=2
retain @ev range=lines(around=0) bindings=2 schema=writfold-citation/1
view @view rows=2 merged=0 limited=0 from=0 shown=2 omitted=0 budget_tokens=300 used=96 estimate=ceil((bytes+1)/4) per text line
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
matcher   = "text" STRING | RX | "similar" STRING ;
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

- **`at`** must be a full 40- or 64-digit lowercase commit id. Refs, `HEAD`
  and abbreviations are refused at check time (`T206`). At run time the id must
  name a commit in the repository and resolve to exactly itself. Otherwise the
  run refuses with the storage boundary's `Refusal::Pin`, never an empty
  result.
- **`in`** is a byte prefix of repository paths. Write `"src/"` for a
  directory, since `"src"` also matches `srcfoo`. Empty or absent means the
  whole tree. A leading `/` and `.`/`..` segments are refused (`T211`).
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
| `path` | Repository path. It prints bare unless it holds whitespace, quotes, backslashes or controls, which force a JSON-quoted form. |
| `line` | 1-based line of the row's first byte (for evidence, the first retained line) |
| `range` | Half-open file byte range `LO..HI` (for evidence, the retained range) |
| `binding` | 64-hex `writfold-citation/1` binding digest |
| `excerpt(max_chars: N)` | A prefix of the retained bytes, `LO..HI:"text"` |

`N` is 1..=4096. The excerpt shows at most `N` Unicode scalars of the longest
valid UTF-8 prefix of the retained bytes. `LO..HI` is the exact file byte range
shown. A truncated excerpt ends with `…+NB`, the retained bytes not shown.
Text is quoted as a JSON string literal.

Rows sort by the declared keys (`asc` by default), then by `(path, range)`
ascending, so the order never depends on a backend. Identical rows, with the
same path, range and binding, merge (`merged=` counts them). `limit` (1..=100000)
drops rows after sorting. They are counted as `limited=`. The cursor does not
resume them, because they are the program's own choice.

## `emit`

Output is a mandatory envelope followed by the rows, one per line.

```text
tuft/0 program=sha256:<program digest> commit=<commit>
find @<name> <text|rx> "<pattern>" in "<scope>" limit=<n>: coverage=<complete|partial> files=<E>/<T> hits=<n>[ stopped=hit-limit]
retain @<name> range=<match|lines(around=N)> bindings=<distinct> schema=writfold-citation/1
view @<name> rows=<n> merged=<n> limited=<n> from=<i> shown=<n> omitted=<n> budget_tokens=<b> used=<u> estimate=ceil((bytes+1)/4) per text line
[notice next row needs <t> tokens; raise budget_tokens]
cursor=<TAG.OFFSET|none>
columns <field> ...
<row>
```

- **Budget.** `budget_tokens` (default 1000, 1..=1000000) bounds the `columns`
  line plus the rows. Each line costs `ceil((bytes + 1) / 4)` estimated tokens,
  counting its LF. This is a conservative byte estimate, not a tokenizer. The
  envelope is always printed and sits outside the budget. Rows stop at a row
  boundary. `omitted` counts the rows not shown. If not even one row fits,
  the envelope says so and shows zero rows.
- **Cursor.** `TAG` is the first 16 hex digits of SHA-256 over the program
  digest, commit, budget and every rendered row. `--cursor TAG.OFFSET` reruns
  the same program and resumes at `OFFSET`. A cursor from a different program,
  commit, budget or result refuses. It is a continuation, not a capability.
- **Determinism.** The same program bytes and commit give byte-identical
  output in both formats.
- **`--json`** emits the same envelope and rows as one JSON object, with
  fields `tuft`, `program`, `commit`, `find`, `retain`, `view`, `budget`,
  `cursor` and `rows`. The budget is computed over the text rendering, so text
  and JSON pages show the same rows and share cursors.

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
The lexer and parser stop at the first error. The checker reports every error,
in source order. The golden cases are in
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
writfold tuft run FILE --repo DIR [--state DIR] [--cursor C] [--json]
writfold tuft check FILE
```

`--state` defaults to the trial state (`$WRITFOLD_STATE`, else
`~/.local/share/writfold/trial`), which holds the admitted Git executable and
the retained evidence. Create it once with `writfold trial init`. `--repo`
must be an ordinary repository (`DIR/.git` a directory). A whole-tree
`find` loads every blob in scope once, in parallel, bounded at 1 GiB. A larger
scope refuses and asks to be narrowed.

## Revision-9 vocabulary

v0 uses the domain's `Coverage` and `SearchBudget` directly.
`CandidateHandle` and `ScopeCut` exist only as planned names, with no code
yet. The domain's `NoticeCursor` is a borrowed, session-scoped value over a
`NoticeQuery` with an `AliasScope`. A CLI run has no session, so Tuft's
serialized `TAG.OFFSET` continuation is bound to program, commit, budget and
result instead. When a session layer (S3) exists, the cursor should move onto
`NoticeCursor`.
