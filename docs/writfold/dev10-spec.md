# v0.5.0-dev.10 specification

> **Status:** built in v0.5.0-dev.10: steps 1 to 11 landed; the as-built record, with every deviation, is [kb-v1-design.md §40](kb-v1-design.md#40-v050-dev10-as-built). Written against `main` at `f2ced5b` (v0.5.0-dev.9). The authority is [mission.md](mission.md).

This release takes eight candidate needs, most of them about guards, and builds the ones that hold for any single user, team or CI job on any harness. Two observations frame it. A guard that blocks the wrong thing costs more than it saves, so the first work is making a rule match exactly the command it names and judge the repository that command acts on. And an empty answer must never read as a met one, so the one place where it still does is closed.

Each candidate's description of today's behaviour was checked against the code at `f2ced5b` and, where a run was cheap, against the release binary on a copy of `fixtures/kb/basic`. What was found is in [section 6](#6-what-was-checked); three of the findings are faults that this release fixes.

## 1. Triage

| # | Need | Verdict | Reason |
|---|---|---|---|
| 1 | Try a guard rule without enforcing it, and see what it would have refused | **Adopt, as `mode = "audit"`, below the cut line; decline a second "warn" mode** | A new or changed rule is where false refusals come from. One mode is enough: a warning shown to the agent is a nudge, and a rule that only nudges is not being tested as a rule that blocks |
| 2 | A `commands` rule matches the command it names and judges the repository that command acts on | **Already does in part; adopt the rest** | Quoted text and an `ssh` argument already never match, and a rule cannot match by working directory alone. But a here-document body matches (a false refusal), `git -C DIR commit` and common wrappers do not (a silent miss), and the repository judged is always the event's working directory |
| 3 | A way past a guard that an agent can use and that leaves a record | **Adopt an alternative: `gate waive` exists; make a waiver visible in the guard answers; `mode = "audit"` for a rule that misfires** | The waiver asked for is already there for every requirement that is a gate: a tracked file outside `.git/`, with who and why, bound to the bytes it excuses. A second, rule-level waiver that the refused agent files itself would turn every refusal into a waiver |
| 4 | Empty is not ready | **Already does in `gate status`; adopt one fix in the guard** | `gate status` answers `unknown`, exit 2, with no gates. A guard rule requiring `gates.met` passes when the run owes no gate at all: that is the fault |
| 5 | Find runs nobody has written to; close finished ones automatically | **Adopt `runs --idle`, below the cut line; decline automatic closing** | Idle runs can be listed from tracked file names, identically on every clone. Closing on a condition needs a scheduler Writfold does not have, and "every gate met" is not "the work is done" |
| 6 | What was written to the KB since a point in time | **Adopt an alternative, below the cut line: `kb delta --since REV` and named marks; decline `--since <timestamp>`** | Neither Git nor a record holds a per-file write time that survives a clone. A baseline (a revision, or a mark that remembers the dirty files) is exact and needs no clock |
| 7 | Lookups by what people call a thing | **Already does exact, alias, FTS5 text with BM25, trigram substring and typo tiers; adopt `normalized`, `prefix` and `heading` tiers, `matched_on`, `find --eval` and (below the cut line) candidates on a miss; defer semantic search, fusion, learned aliases and aliases on document records; decline a batch form** | The lexical layers exist in the one index file. What misses is the step between an exact ID and free text, and there is no way to measure a change. Semantic search waits for that measurement ([4.6](#46-lookup-layers-not-built-item-7)) |
| 8 | Accept an external conformance suite for guards | **Adopt: Writfold's own case format, seeded from this release's tests; an outside suite is converted into it** | The decision is defined here, so the format is too. Nothing depends on fixtures that do not exist yet |

## 2. Adopted items

### 2.1 Empty is not met (item 4)

**Today.** With a current run and no gate defined, `gate status` prints no row, `answer=unknown reason=no-gates`, exit 2; `gate lines` prints `0 of 0 satisfied`, exit 2; `kb check` shows `check gates [attention]`. A checklist gate with no cover row stands `open` (`0/2 required covered`), exit 2, and a rule requiring it refuses. With no current run, `gate status` is refused, exit 3. All of that is right and stays.

**The fault.** `guard::require::gates_met` collects the unmet gates and passes when the list is empty. When the run owes nothing (no gate defined for the repository, or a plan whose steps name no gate), the list is empty, so a rule with `require = ["gates.met"]` passes and `guard status` says `would-pass`, while `gate status` for the same run says `unknown`.

**Fix.**

- Kernel, `writfold_core::guard`: `gates_fact(owed: usize, unmet: usize, invalid: usize) -> Fact`, `Met` only when `owed > 0 && unmet == 0 && invalid == 0`, `Unmet` otherwise. `require::gates_met` calls it. Exhaustive test over `0..=2` for each argument.
- The owed text for the empty case, in place of a gate list: `no gate is owed here: none is defined for <repo>; gates.met is not met by an empty set; next: define .writfold/gates/<id>.toml, or leave this repository out of the rule with repos`. With a plan that names none: `no gate is owed here: the run's plan names none; …`.
- `gate status` with gates defined but a plan that owes none says today `no gates defined`, which is false. The note becomes `no gate is owed: the run's plan names none (N defined)`; `reason=no-gates` and exit 2 are unchanged.
- `gate lines` with nothing owed: the header ends `: no gate is owed; nothing was checked` instead of `0 of 0 satisfied`, since the line is meant for a commit message. Exit 2 unchanged.
- `step.met` keeps its documented meaning (met with no plan, or when every step is closed): it is a statement about a plan step, and the Stop rule built on it must stay quiet in a run without a plan. The guide says to add `run.current` or `gates.met` beside it when a missing plan should not pass.

**A waiver or attestation that makes itself stale.** Found while checking: when the subject repository is the KB's own and a gate's scope admits `runs/`, the statement or cover row just written is inside the scope, so it changes the content it is bound to and the gate reads `open` again, with `why: a waiver exists, but for other content or another definition`. `gate waive`, `gate attest` and `gate cover` add a note when the gate does not stand as written immediately after their own write: `# note: this gate's scope includes the run's own files, so the statement changed the content it is about; exclude runs/** from the scope`. `gates.md` says the same under Scope patterns.

**Compatibility, named.** A rule requiring `gates.met` in a repository with no gate, which passes today, refuses after this change. That is the intent; the release note says so and names `repos` as the way to leave such a repository out. Text goldens that change: any `guard status` or hook golden over a run that owes nothing, the `gate lines` zero-gate header, and the `gate status` note for a plan that owes none.

**Tests, with the mutation each one catches.**

| Test | Removing this makes it fail |
|---|---|
| A `gates.met` rule, a current run, no gate defined: the hook denies with the empty-set text; `guard status` is `would-deny`, exit 2 | `owed > 0` in `gates_fact` |
| The same with a plan whose steps name no gate | the plan branch |
| One gate defined and met: passes (regression) | (none; guards the fix against over-reach) |
| `gate lines` with nothing owed prints the new header and exits 2 | the header branch |
| A KB-as-subject gate with scope `**`: `gate waive` prints the note; with `!runs/**` it stands `waived` and prints none | the standing re-check after the write |

Docs: `hooks.md` (the requirement table), `gates.md`, design as-built.

### 2.2 How a command is read (item 2)

**Today, as run on the binary** (rule `commands = ["git commit", "git push"]`):

| Command line | Today | After |
|---|---|---|
| `cat README.md`, `grep -r "git commit" .`, `echo "git commit"` | pass | pass |
| `ssh host "git commit -m x"`, `ssh host 'cd r && git push'` | pass | pass |
| `git commit -m x`, `FOO=1 git commit`, `cd /tmp && git commit`, `true \|\| git push`, `echo $(git commit)` | refused | refused |
| `cat > notes.md <<EOF` … a body line `git commit -m x` … `EOF` | **refused** | pass |
| `git commit --help` | **refused** | pass |
| `git -C ~/other commit`, `git --no-pager commit`, `git -c user.name=x commit` | **pass** | refused |
| `/usr/bin/git commit`, `command git commit`, `time git commit`, `sudo git commit`, `env A=1 git commit`, `if git commit; then …`, `{ git commit; }`, `! git commit` | **pass** | refused |
| `bash -c "git commit"`, `xargs git commit`, ``x=`git commit` ``, `python -c '…'` | pass | pass (not followed; documented) |

A rule has never been able to match on the working directory alone: a `pre-tool-use` rule must name `commands` or `tools`, so `cat` and `grep` inside a guarded repository pass unless a rule lists the tool `Bash` itself. That stays, and gains a test.

The repository whose gates are judged is today the one enclosing the event's `cwd` (else the run's recorded worktree, else the KB's). `cd /other && git commit` and `git -C /other commit` are therefore judged against the wrong repository, in both directions: a refusal over gates the commit does not touch, or a pass over gates it does.

#### The reader

A pure module, `writfold_core::command` (no IO, `alloc` only). The existing reader in `writfold-kb/src/shell.rs` moves into it and is re-exported, so the delivery guard uses the same reader and its tests do not change.

```rust
pub enum Dir { At(String), Unresolved(Why) }          // an absolute path, by name
pub enum Why { Variable, Glob, OtherUser, NoHome, Previous, GitDir, TooMany }
pub struct Invocation { pub words: Vec<String>, pub dir: Dir }
pub fn invocations(line: &str, cwd: &str, home: Option<&str>) -> Vec<Invocation>
```

Rules, each with a table test:

1. **Segments.** As today: words split on blanks; quotes and escapes removed; `;`, `&`, `|`, newline, `(` and `)` end a simple command; `#` starts a comment. Text inside quotes is one word and is never a command. The reader now also records which separator ended each segment.
2. **Here-documents.** After `<<` or `<<-` and its delimiter word (quoted or not), the lines from the next newline up to a line equal to the delimiter (leading tabs dropped for `<<-`) are data, not commands. Several here-documents on one line are consumed in order. `<<<` is a word. An unterminated here-document consumes the rest of the line.
3. **Leading words that are skipped.** `NAME=value` assignments (as today); the shell words `if`, `then`, `else`, `elif`, `do`, `while`, `until`, `!`, `{` and `time`; the wrappers `command`, `exec`, `nohup` and `builtin` when the next word does not begin with `-`; `env` followed only by assignments; `sudo` when the next word does not begin with `-`. A wrapper used with options is left as it is and does not match.
4. **The command's name.** A first word containing `/` is compared by its last segment, so `/usr/bin/git` is `git`.
5. **`git`'s global options.** Between `git` and its subcommand: `-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace`, `--super-prefix` and `--attr-source` are dropped with their value (also in the `--name=value` form); any other word beginning with `-` is dropped alone. `words` is then `git`, the subcommand and its arguments, which is what a rule's `commands` entry is compared with, as a prefix, as today.
6. **`--help`.** An invocation whose only word after the matched prefix is `--help` does not match. `-h` and `--dry-run` are not exempt: their meaning differs by command.
7. **The directory.** It starts as the event's `cwd`. A segment that is exactly `cd DIR`, not in a pipeline and not in the background, sets it for the segments that follow in the same group; a `( … )` or `$( … )` group starts from the directory outside it and its `cd` ends with it. `cd` alone is the home directory. `git -C DIR` (repeatable, each relative to the last) and `--work-tree DIR` apply to that invocation only. A relative directory is joined by name, `.` and `..` resolved by name, as `paths` does.
8. **What is expanded.** A leading unquoted `~` or `~/`, and a word that begins `$HOME` or `${HOME}` followed by nothing or `/` and holds no other expansion, become the home directory (`HOME` in the hook's environment).
9. **What is never guessed.** The directory is `Unresolved` when a directory word holds any other expansion (`Variable`), an unquoted `*`, `?` or `[` (`Glob`), `~name` (`OtherUser`), a home directory that is needed and unknown (`NoHome`); after `cd -`, `pushd`, `popd` or `cd` with an option (`Previous`); with `--git-dir` and no `--work-tree`, or a `GIT_DIR` or `GIT_WORK_TREE` assignment (`GitDir`). Once a `cd` is unresolved, so is every later segment of its group. The reader's `Word` gains `tilde` and `glob` (set only outside quotes) so these are known, not inferred.

Not followed, and listed in the guide as such: the string given to `bash -c`, `sh -c` or `eval`; `xargs`, `find -exec`, `watch` and other programs that run their arguments; backquote substitution; aliases, functions, scripts and Makefile targets; the remote command of `ssh` (it acts on another machine). A rule is a guard: these are misses, never refusals.

#### What the hook does with it

- A `commands` rule matches an invocation when the rule's words are a prefix of `words` (rules 3 to 6 applied). The delivery guard keeps its own matching and gains only rule 2.
- For each matched invocation the rule is judged **at that invocation's directory**: `subject::locate` is given it in place of the event's `cwd`, so the repository is the one enclosing that directory, else the run's recorded worktree, else the KB's, exactly as today for the `cwd`. `repos` names that repository.
- **Which rules apply.** The KB root's rules apply to every invocation. A repository's own rules (the nearest `.writfold/guards/` at or above a directory) apply to the invocations whose directory is at or below it; for a `tools` rule that directory is the event's `cwd`, as today. With an unresolved directory only the KB root's rules apply.
- **An unresolved directory** on a matched invocation is not a pass and not a guess. `run.current` is judged as usual; every requirement that needs a repository (`gates.met`, `gate:<id>`, `step.met`) is `Fact::Unknown`, which takes the rule's action, with the text `cannot tell which repository this command acts on (git -C "$REPO": a variable); name the directory literally`. A rule with `repos` applies. The way out is in the text and costs one retry.
- Several matched invocations with different directories are judged separately and combined by `combine` (a denial wins). The index is opened once. At most four distinct directories are judged per call; a matched invocation beyond that is `Unknown` with `TooMany`.
- **A linked worktree is named after its repository.** Today a linked worktree with no `origin` and no map entry is named by its own directory (`api-wt`, not `api`), and a map entry for the main clone does not name its worktrees, so a `repos` pattern and the run's receipts do not carry across them. The name becomes the first of: the map entry whose Git directory is the worktree's common directory; `remote.origin.url` (unchanged); the main work tree's directory name. Named change: receipts filed from such a worktree under its directory name are then about another repository name and read `open` until the gate is run again. Nothing is relabelled.
- `guard status` is unchanged in shape: it judges each rule as if it had matched at the working directory.

**Time.** A call that matches no rule opens no file. It parses the command line and, per distinct directory, walks upward for `.writfold/guards/` by `stat`. Budget: within 0.5 ms of today's 1.4 ms on the reference fixture, re-measured over 300 calls and recorded in `hooks.md`. A matched call costs today's evaluation (about 15 ms on the fixture, plus what `gate status` costs) once per distinct directory, at most four. The hook's timeout stays 10 s and it still always exits 0.

**Tests.** Table tests in `writfold-core/src/command.rs` for each numbered rule; every row of the table above as a conformance case (2.3). Mutations:

| Test | Removing this makes it fail |
|---|---|
| A here-document body holding `git commit` passes; the command after the delimiter line is still refused | rule 2; ending the body at the delimiter |
| A generated table: any printable string placed inside single quotes, inside double quotes with `$` and `` ` `` escaped, or as a here-document body with an unused delimiter, after `echo` or `cat`, yields no invocation named `git` | quote and here-document handling (the property, enumerated over a fixed corpus and seed; no new crate) |
| `git -C DIR commit`, `git --no-pager commit`, `git -c a=b commit` are refused | rule 5 |
| `command`, `time`, `sudo`, `env A=1`, `if`, `{`, `!` forms are refused; `sudo -u x git commit` is not unwrapped | rule 3 and its option stop |
| `/usr/bin/git commit` is refused; `git commit-tree` passes | rule 4; whole-word comparison |
| `git commit --help` passes; `git commit -m --help` and `git push --dry-run` are refused | rule 6 and its "only word" limit |
| Two repositories, one with its gate met: `cd B && git commit` from A, `git -C B commit` from A, and `(cd B && true); git commit` from A each judge the right one | rule 7, `-C`, the group boundary |
| `git -C ~/b commit`, `git -C "$HOME/b" commit` resolve; `git -C '~/b' commit` is a directory named `~` | rule 8 and the `tilde` flag |
| `git -C "$R" commit`, `git -C b* commit`, `cd - && git commit`, `GIT_DIR=x git commit`: the action is taken with the unresolved text; the same with `action = "ask"` asks | rule 9; `Unknown` for repository requirements |
| A repository's own rule does not apply to `git -C ../other commit`; the other repository's rule does | rule discovery per directory |
| A linked worktree without `origin` answers to the main clone's name and to its map entry | the common-directory lookup |
| `cat`, `grep` and a lookup verb inside a guarded repository pass | (regression: no match by directory) |
| The delivery guard's existing tests, unchanged; one new case: a publishing command quoted in a here-document body is not matched | the shared reader |

Docs: `hooks.md` gains "How a command is read" (the numbered rules, the not-followed list, the unresolved text) and the re-measured cost; the `commands` row of the key table points at it; `delivery-modes.md` one sentence on here-documents; design as-built.

### 2.3 Guard conformance cases (item 8)

**Decision.** Writfold defines the case format, keeps a suite of its own, and runs every case through the real hook entry point in the unit gate. A suite written elsewhere is accepted by conversion into this format at import, never by a runtime adapter.

**Layout.**

```text
fixtures/guard-conformance/
  README.md                     the format, the import rules, the table of recorded differences
  <suite>/SUITE.json
  <suite>/cases/<case-id>/event.json
                          world.json
                          expected.json
                          transcript.jsonl      optional; kept for other implementations, never read
```

`SUITE.json` (`writfold-guard-suite/1`): `name`, `source` (`this repository`, or a URL), `revision` (the source commit, or null), `licence` (an SPDX identifier), `imported` (a date, or null), `converted_by` (how the cases were converted, or null), `cases` (the count, which must equal the number of case directories).

`event.json` is the hook's input exactly as a harness sends it (`hook_event_name`, `tool_name`, `tool_input`, `cwd`, `session_id`, `stop_hook_active`), with every path written under `{root}` or `{home}`.

`world.json` (`writfold-guard-world/1`) states what the files and the KB hold, and the runner builds it in a temporary directory:

```json
{
  "schema": "writfold-guard-world/1",
  "home": "{root}/home",
  "dirs": ["{root}/tmp"],
  "repos": [
    {"top": "{root}/src/api", "origin": "git@host.example:acme/api.git",
     "worktrees": ["{root}/src/api-review"], "rules": {}}
  ],
  "kb": {
    "rules": {"commit-needs-gates": "schema = \"writfold-guard-rule/1\"\n…"},
    "run": "open",
    "map": {"api": "{root}/src/api"},
    "gates": [{"id": "unit", "repo": "api", "standing": "open"}]
  }
}
```

`run` is `open`, `closed` or `none`; a gate's `standing` is `met`, `open` or `waived` (the runner defines an attested gate and files the statement through the ordinary verbs). Rule text is the rule file's bytes.

`expected.json` (`writfold-guard-expected/1`):

```json
{"decision": "deny", "decided_by": ["commit-needs-gates"], "reason_has": ["gate unit [open]"]}
```

`decision` is `pass`, `deny` or `ask` for `PreToolUse`, and `nudge` or `quiet` for `Stop`. `decided_by` (sorted rule IDs) and `reason_has` (substrings of the reason) are optional. A converted case whose source expects otherwise carries `"upstream": {"decision": "…"}` and `"differs": "<why Writfold decides this way>"`.

**Rules of the suite.**

- Reading is strict: an unknown key, a missing file, a path outside `{root}` and `{home}`, or a case the runner cannot build fails the test. There is no skipped case and no `#[ignore]`: a case Writfold cannot evaluate (one that needs a transcript, say) is not imported, and the README lists it by its source name with the reason.
- A difference from the source's expectation is recorded, not hidden and not relabelled: `expected.json` holds Writfold's decision, `upstream` the source's, `differs` the reason, and the README's table lists the case. A test fails when a case with `upstream` differing has no `differs`, or is missing from the table.
- Provenance: synthetic data only (no real host, user, path or command line taken from a session); a licence compatible with this repository's (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, CC0-1.0 or Unlicense), stated in `SUITE.json` and by the contributor in the commit; the source revision pinned. The converter is not part of the product.
- Runner: `crates/writfold-kb/tests/guard_conformance.rs`. One test builds each world with the existing test support, runs `writfold_kb::hook::run` with the event and a fixed clock, compares, and reports every failing case by ID; a second checks each suite's shape and count. Both run under `cargo test`, so `scripts/gate unit` counts them.

**The seed suite** (`writfold/`, `source: this repository`, MIT): every row of the tables in 2.2 and their tests, the empty-set cases of 2.1, and the scenarios of `tests/guard_rules.rs` and `tests/guard_paths.rs` that are decisions (a `repos` exclusion, a repository's own rule, an unreadable KB taking the action, `ask`, a `paths` exclusion, a Stop rule once per session and once per turn). About forty cases. The Rust tests stay; the cases are the language-neutral statement of the same decisions.

Docs: the README above; `validation.md` gains a paragraph; `assurance-map.md` one row.

### 2.4 Layered `find` (item 7)

**Need.** The KB helps only when the first lookup hits. Misses are mostly naming gaps: a phrase for an ID (`harbor api` for `harbor-api`), part of a name (`quay work` for `quay-worker`), a section of a project's notes, a decision inside a dossier.

**Today.** `find` already has tiers, merged in a fixed order with each record shown once and each hit labelled on its `match:` line:

| Tier | Over | Notes |
|---|---|---|
| `signature` | signature patterns | with `match` or `--signatures` |
| `exact` | record ID, ticket form (`HBR 101`, `hbr101`), typed prefix, alias, asset locator | assets carry `aliases`; signatures, investigations and projects answer to their bare names |
| `text` | SQLite FTS5 (`unicode61`, diacritics removed) over ID, title, summary and body, BM25 weighted 10, 5, 2, 1; every word | the `match:` line is the first line holding a query word, with its path and line, so it is citable |
| `substring` | an FTS5 `trigram` table over ID, title and body; the whole query as a substring | three characters or more |
| `typo` | trigram candidates re-ranked by edit distance (one edit, two for long queries) | ID and title only |
| `text-any` | BM25 again, any word | |

The index is one SQLite file (bundled SQLite 3.50.2 through `rusqlite` 0.37, FTS5 compiled in; the trigram tokenizer needs 3.34), rebuilt from Git, holding every file's body and each Markdown section's name and line range. So lexical search over bodies with a citable line is built, and no new table is needed for what follows. All tiers run on every query; one `find --json` call takes about 10 ms on the fixture KB with process start.

What misses or ranks badly today (run): `harbor api` and `harbor_api` list the asset fourth, behind three relationship records; `HAPI` reaches the alias `hapi` only as body text; `quay work` lists `quay-worker` second; a section heading is found only as body text of its file, with no heading named; a typo in a body word (`batchng`) finds nothing, and an empty answer offers no candidate.

**Changes.** Three tiers between `exact` and `text`, one new key, and candidates on an empty answer. Nothing is added to the index schema; every tier is computed at query time over `records`, `aliases` and `sections`.

- `norm(s)`: lower-case; every run of characters that are not letters or digits becomes one `-`; leading and trailing `-` removed. Its tokens are the parts between `-`. A pure function in `writfold_core::find`, with `prefix_match(query_tokens, name_tokens)`, both table-tested.
- **`normalized`**: `norm(query)` equals the `norm` of a record's ID, of the ID after its typed prefix, of the ID's last `/` segment, of its title, or of one of its aliases. The query's `norm` is at least three characters.
- **`prefix`**: two or more query tokens, or one of four characters or more, each a prefix of a distinct token of one of those same forms, in the same order, gaps allowed (`jmga web` matches `repo-jmga-web-app`; `quay work` matches `quay-worker`). Ordered by fewest unmatched name tokens, then ID.
- **`heading`**: `norm(query)` equals, or by the `prefix` rule matches, the `norm` of a section name. The row is the file's record (or `doc:<path>`), and its `match:` line is the heading with the section's first line, so the hit cites the section.
- Order: `signature`, `exact`, `normalized`, `prefix`, `heading`, `text`, `substring`, `typo`, `text-any`. Each record is still shown once, under its first tier. The text label on the `match:` line is the tier name followed by the form that matched (`normalized harbor-api`, `prefix repo-jmga-web-app`, `heading Staging sweep`).
- **`matched_on`** (JSON, additive): every `find` and `match` row gains the key `matched_on`, the tier name as above (`listed` for a `--kind` listing). It is absent on other verbs' rows, as `edge` is for `impact`. Today a reader has to split the `match` continuation's text (`typo d=1 api`), which the JSON guide advises against elsewhere. The values `exact`, `normalized` and `text` are spelt as a reader of other tools' output would expect; the remaining values are Writfold's own tiers, and a reader that knows only three ignores the rest. The `match` continuation stays, with its path and line.
- **Candidates on an empty answer.** When no tier finds anything, up to three nearest names are shown: trigram candidates over ID, title, alias and section name, ranked by edit distance to the closest form, beyond the `typo` tier's tolerance but within half the query's length. Each is a row with status `suggested`, `matched_on: "suggestion"` and the flag `!suggestion`. The answer stays `unknown reason=no-record`, exit 2, with the existing note: a suggestion is not a hit.
- **Budgets.** `find` stays within 5 ms of today's time on the fixture, and under 50 ms, process start included, on a generated KB of 3,000 records; both are measured in this step and recorded in `validation.md` with the index size for that KB. Nothing in the hooks calls `find`; the budget is for a caller's own hook.

**`find --eval`: measuring it.**

```text
writfold kb find --eval [FILE] [--json]          default FILE: .writfold/queries.jsonl
```

One JSON object per line: `{"query": "jmga web", "expect": ["repo-jmga-web-app"], "note": "…"}`; `expect` is one or more record IDs (or `doc:<path>`), any of which counts; unknown keys are refused, and a line that does not parse is refused with its line number (exit 3). The file is tracked, so a team's known phrases are reviewed like any other text and CI can run it.

- One row per query: kind `query`, status `hit@1`, `hit@5` or `miss`, the rank and tier of the first expected record, and, on a miss, what ranked first.
- Trailer counts: `queries`, `hit1`, `hit5`, `miss`, and one count per tier of first expected hits (`by-exact`, `by-normalized`, `by-prefix`, `by-heading`, `by-text`, …), which is recall per tier. An `expect` ID that is not a record is flagged `!unknown-id` and counts as a miss.
- Exit 0 only when every query has an expected record in the first five; otherwise 2. An empty or absent file is `answer=unknown reason=no-queries`, exit 2: an evaluation with no queries is not a pass.
- It runs the same code as `find`, with no budget cut, and writes nothing.

The fixture KB gains `.writfold/queries.jsonl` with the phrases above, and the unit gate runs it: a tier that stops finding its phrase fails a test.

**Tests, with the mutation each one catches.**

| Test | Removing this makes it fail |
|---|---|
| `harbor api`, `harbor_api` and `HAPI` put the asset first, `matched_on: normalized` | the `normalized` tier |
| `quay work` puts `quay-worker` first, `matched_on: prefix`; `work quay` does not match by prefix | the `prefix` tier; token order |
| `restart harbor` puts `runbook:ops/restart-harbor` first | the last-segment form |
| A phrase equal to a section name returns that file with the heading and its line | the `heading` tier |
| A one-token three-character query adds no `prefix` row | the length floor |
| An exact ID shows `exact` once, with no second row from a later tier | once-per-record merging |
| `batchng` returns `suggested` rows, `answer=unknown`, exit 2 | candidates; the exit staying 2 |
| `find --eval` over the fixture file: every query `hit@1`, `by-*` counts as listed; a query made to miss gives `miss`, exit 2; an empty file exits 2 | the evaluation's exit rule; the empty case |
| `json.rs`: `matched_on` is documented in `json.md` and absent on other verbs | (the existing key-set test) |

Text goldens: a `find` golden whose query matches by a new tier gains the row or moves it up, and its `match:` label changes. The implementer lists each in the commit. No golden of another verb changes.

Docs: the tier table in `find`'s help and module comment; `kb-layout.md` ("what a lookup matches", with the table above extended); `json.md` (`matched_on`, the `suggested` status, the `query` rows and counts); `validation.md` (`find --eval`, the measured times); the `writfold` skill's line on lookups (try the phrase first; a miss is `answer=unknown`, exit 2, and may name candidates).

### 2.5 `mode = "audit"` and the guard log (items 1 and 3; below the cut line)

**Need.** A new or changed rule should be watched before it blocks, and a rule that misfires should be demotable without deleting it. Today a rule either decides or does not exist.

#### The rule key

```toml
mode = "audit"      # or "enforce", the default; pre-tool-use rules only
```

- An audit rule is matched and judged exactly as an enforcing one. The harness is told nothing for it: stdout carries no decision from that rule, and one line goes to stderr, `writfold kb hook: guard rule <id> (audit) would deny: <reason>`. Its would-be decision is appended to the log.
- Any other value is refused; `mode` on a `stop` rule is refused (`mode applies to a pre-tool-use rule only`).
- Kernel: `enum Mode { Enforce, Audit }` and `effect(mode, decided) -> Decision` (`Pass` under `Audit`, `decided` otherwise), enumerated. `decide` is unchanged, so the log holds what enforcing would have decided, by construction.
- `kb edit apply` with a matched audit rule naming `writfold:edit` applies the edit and adds `# note: guard rule <id> (audit) would refuse this edit: …; applied and logged`.

There is no `"warn"`: context added on a passing call is a nudge, and whether the rule is right can only be read from what it would have blocked.

#### Never read as passing

- `guard status`: an audit rule keeps its judged status (`would-deny`, `would-ask`, `would-pass`) and carries the flag `!audit` and a continuation `mode: audit: a matched call is let through and logged`. It counts under a new `audit` count when it would deny or ask, not under `deny` or `ask`; `audit=` appears in the trailer only when not zero, so existing goldens do not change. Exit 2 when `audit` is not zero: an audit rule that would refuse is not a clean answer.
- **A waiver shown where the guard passes on it** (item 3). Today a rule that passes only because a gate is waived reads `would-pass` with no trace. `require` reports the waived gates it relied on; the `guard status` row carries `!waived` and a `waived:` continuation with the reason, who and when, pointing at the statement file; the hook writes `writfold kb hook: guard rule <id> passed on a waiver: gate <gate>` to stderr and logs it.
- `writfold watch` shows `guard status` rows with their flags; one buffer test pins `!audit` and `!waived`.

#### The log

`<KB Git directory>/writfold/guard-log.jsonl`: local, untracked, this machine only, mode 0600. It is a log of what one machine's hook saw, so it does not belong in the KB work tree: committed, it would conflict on every merge and publish command lines.

One line per entry, appended with one `write` on a file opened for append:

```json
{"schema":"writfold-guard-log/1","at":"2026-11-16T10:02:11Z","via":"hook","session":"3f9c…","rule":"commit-needs-gates","rule_path":".writfold/guards/commit-needs-gates.toml","rule_digest":"sha256:…","mode":"audit","decided":"deny","effect":"pass","tool":"Bash","command":"git commit -m …","dir":"/home/u/src/api","repo":"api","run":"run-…","owes":["gate unit [stale]; next: writfold kb gate run unit"],"waived":[]}
```

- Written for: every evaluation of a matched audit rule (passes included, so a clean log can be told from a rule that never matched); every decision of an enforcing rule that is not a pass; every pass that rested on a waiver. A call no rule matches writes nothing.
- `command` is the matched invocation's words after 2.2's normalisation (leading assignments dropped, expansions as written), cut to 200 characters. `rule_digest` is the SHA-256 of the rule file's bytes. `via` is `hook` or `edit-apply`.
- At 4 MiB the file is renamed to `guard-log.1.jsonl`, replacing the previous one. A line that does not parse is counted and skipped by the reader.
- A log that cannot be written changes no decision; the hook says so on stderr. With no KB Git directory there is no log.
- Time: one append on a matched call, measured and recorded in `hooks.md`; nothing on an unmatched call.

#### The report

```text
writfold kb guard log [--rule ID] [--since AGE] [--json]
```

```text
guard commit-needs-gates [audit] matched=14 would-deny=2 would-ask=0 pass=12 since 2026-11-10T09:00:00Z @.writfold/guards/commit-needs-gates.toml:1 !audit !would-deny
  would-deny: 2026-11-15T10:02:11Z git commit -m … in api; owes: gate unit [stale] (session 3f9c…)
  would-deny: 2026-11-14T16:40:02Z git push in api; owes: gate review [open] (session 81aa…)
  earlier: 9 entries are for an earlier version of the rule file and are not counted
guard source-edits-need-a-run [enforce] deny=5 ask=0 passed-on-waiver=1 since 2026-11-09T08:12:00Z @.writfold/guards/source-edits-need-a-run.toml:1
# note: from the local log <path>: this machine only, never committed; a guard, not enforcement
# coverage=… rules=2 entries=20 would-deny=2 would-ask=0 deny=5 ask=0 unparsed=0 …
```

- Counts are over entries whose `rule_digest` is the rule file's current digest, so editing a rule restarts its count; older entries are reported as `earlier` and never added in.
- The five newest would-deny or would-ask entries are shown per rule; `--rule ID` shows all, paged by the budget.
- When an audit rule has `matched` of at least 1 and no would-deny or would-ask for its current version, its row carries `next: remove mode = "audit" from <path> to enforce it`. Writfold never promotes a rule. An audit rule with `matched=0` carries `!unmatched` and no `next:`.
- Exit 2 when an audit rule would have denied or asked, or has no match for its current version; otherwise 0. A rule in the log with no file is listed as `[removed]`.
- JSON: the ordinary envelope; rows of kind `guard-log`, continuations labelled `would-deny`, `would-ask`, `earlier`, `next`; the counts above. `guard status` adds the flags `audit` and `waived`, the labels `mode` and `waived`, and the count `audit`.

**Tests.**

| Test | Removing this makes it fail |
|---|---|
| An audit rule with its gate open: stdout empty, the stderr line present, one log entry with `decided: deny`, `effect: pass` | `effect`; the append |
| The same rule without `mode` denies, and logs `effect: deny` | logging enforced decisions |
| An audit rule and an enforcing rule matching one call: the call is denied by the enforcing rule alone, and both are logged | combining over effects, not decisions |
| `guard status` on an audit rule that would deny: `would-deny !audit`, `audit=1`, `deny=0`, exit 2 | the separate count and the exit rule |
| A gate waived, rule passes: `!waived` and the `waived:` line in `guard status`; a log entry with `waived` | reporting the waiver from `require` |
| `guard log` after the rule file is edited: earlier entries are under `earlier`, counts restart, no `next:` | the digest comparison |
| An audit rule with twelve passes and no would-deny: `next:` present, exit 0; with none matched: `!unmatched`, exit 2 | the `matched >= 1` condition |
| `mode = "warn"`, and `mode` on a stop rule: the file is invalid | strict parsing |
| `edit apply` under an audit rule applies and notes; under the enforcing rule it refuses (regression) | the mode in `edit::guarded` |
| A log directory that cannot be written: the decision is unchanged | failing open |
| Rotation at the size limit; a torn last line is skipped and counted | the reader's tolerance |

Docs: `hooks.md` (the key table, "Trying a rule: audit mode", "The guard log", "Past a guard" below; "Not built" loses "receipts of guard decisions" and keeps "a warning without a decision" with the reason), `kb-layout.md` (the local-state list), `json.md`, `watch.md` one sentence, help text, design as-built.

**"Past a guard" (documentation, item 3).** One section in `hooks.md`, built from what exists:

| The refusal names | The way past, and the record it leaves |
|---|---|
| a gate that is `open`, `stale`, `failed` or `unknown` | meet it, or `writfold kb gate waive <id> --reason TEXT --by WHO`: a tracked file in `runs/<run>/statements/`, bound to the gate's definition and the bytes in its scope, so it lapses when either changes; shown as `[waived]` by `gate status`, `resume` and the session-start context, as a `waived by … : reason` line by `gate lines` (so it reaches the commit message), and, from this release, in `guard status` and the guard log |
| no current run, or a closed one | start a run; nothing to waive |
| an undefined gate, or no gate owed | define the gate, or narrow the rule with `repos` |
| "cannot tell which repository" | repeat the command with a literal directory |
| the KB cannot be read, or the rule matches what it should not | set `mode = "audit"` in the rule file: a one-line tracked change, visible in `git diff`, in `guard status` and in every trailer's `+dirty` count until it is committed or reverted, with the log recording what the rule would have refused meanwhile |

The existing recipe for keeping `gate waive` and `gate attest` for people (`permissions.ask`) is repeated there, with the matching line for edits to `.writfold/guards/**`.

### 2.6 `runs --idle` (item 5; below the cut line)

```text
writfold kb runs --idle AGE [--project P] [--plan] [--json]        AGE: 7d, 36h
```

- Lists runs that are not closed and whose last recorded write is older than `AGE`, oldest first.
- **Last recorded write** is the newest of: the manifest's `created`; the last checkpoint heading; the UTC stamp in the name of any file under the run's `gates/`, `statements/`, `covers/`, `observed/`, `retrievals/`, `edits/`, `plan/` and `deliveries/`. All of it is tracked text, so every clone gives the same answer; file modification times are not used. A pure function in `writfold_core` picks the newest stamp and ages it (`idle(now, stamps, age)`), with a table test.
- Each row is `runs`' row plus `idle: 9d since 2026-11-06T14:02:11Z (gate receipt)`; the trailer adds `idle=N`. With `--plan`, the recorded position shows which idle runs have a complete plan.
- Nothing matching: `answer=unknown reason=no-record`, exit 2, as `runs` answers today.
- The age is for display and filtering only; no state is decided by a clock. No liveness is claimed: a run with a recent write is not "active", only not idle.
- JSON: a `more` entry with `label` `idle`; `counts.idle`.

Tests: a run whose only recent write is a cover row is not idle (dropping `covers/` from the scan fails this); a closed run is never listed; the boundary second; `--project`; a run with no checkpoint uses `created`.

Docs: `feeding-back.md` gains "Closing runs": `status RUN --set closed`, and the loop that closes idle runs whose plan is complete (`runs --idle 14d --plan --json` piped through `jq` to `status … --set closed`), reviewed as a Git diff like any other write.

### 2.7 `kb delta` (item 6; below the cut line, alternative shape)

**Need.** A person's own end-of-turn hook, a reviewer or a CI job asks: was anything written to the KB since this point, and what kind of thing?

```text
writfold kb delta --mark NAME                   remember the KB as it is now
writfold kb delta --since-mark NAME [--json]    what differs from that
writfold kb delta --since REV [--json]          what differs from a commit of the KB repository
```

- `--since REV` takes anything `git rev-parse` resolves to a commit in the KB repository (`origin/main`, a tag, a hash) and echoes the full commit. The changed paths are the committed differences from it to `HEAD` plus the dirty and untracked files now, through the existing subject snapshot (`gate::subject`), so ignore rules apply and nothing is written.
- `--mark NAME` writes `<KB Git directory>/writfold/marks/<NAME>.json`: `HEAD` and each dirty or untracked path with the blob ID its bytes would have. `NAME` follows the session-ID rule drafts use. A mark is local and never committed; writing one removes marks older than 30 days. `--since-mark` reports a path when its content now differs from its content at the mark, so files already dirty at the mark are not counted unless they changed again. A mark whose commit is no longer in the clone is refused, exit 3.
- `writfold kb hook session-start` writes the mark named by the event's `session_id` when none exists (so a resume or compaction keeps the first). A Stop hook under the plugin then needs one line, `writfold kb delta --since-mark "$session_id" --json`; any other harness calls `--mark` at its own start. The session-start table in `hooks.md` changes from "writes nothing" to "the session's mark (local)", and the added cost (hashing the dirty files) is measured there.
- **Rows:** one per changed path. `kind` is the record's kind where the path is a record (`ticket`, `decision`, `investigation`, …, from the index, or from the layout for a deleted file), else the run artefact's (`checkpoint`, `gate-receipt`, `statement`, `cover`, `observation`, `retrieval`, `edit-receipt`, `plan-event`, `delivery`), else the configuration's (`gate`, `guard`, `plan`, `kind`, `check-receipt`, `proposal`), else `doc`. `status` is `added`, `modified` or `deleted`; `id` is the record ID or the path; `section` is `records`, `run`, `config` or `other`.
- **Trailer counts:** `changed`, `records`, `run`, `config`, `other`. A note names the base: `# note: compared with mark s-3f9c (HEAD 4be03f7897b6 and 3 dirty files, taken 2026-11-16T09:00:00Z)`.
- **Exit:** 0 when at least one path differs; 2 when none, with `# note: nothing in the KB differs from <base>`. Nothing written is never reported as success.
- It says what differs since a baseline, not who wrote it: commits pulled after the baseline are included, and the note says so when `HEAD` is not a descendant of, or has merged past, the baseline (`# note: includes N commits by others' merges`, counted from merge commits on the path).

Tests: a record, an observation and a cover row written after a mark are three rows in two sections (classification by layout); a file dirty at the mark and untouched since is absent, and present once edited again (comparing blob IDs, not paths); `--since HEAD` on a clean tree exits 2; `--since` an unknown revision is exit 3; an ignored file is never a row; the session-start hook writes the mark once and keeps it on `source: compact`; a kill during the mark write leaves no file or the whole file.

Docs: `hooks.md` (a recipe beside the hard stop: block the end of a turn when `delta --since-mark` exits 2, with `stop_hook_active` honoured), `feeding-back.md`, `json.md`, `kb-layout.md` (local state), help text. The guide also names what already answers the per-run form of the question: a `run-record` gate with `research.backfilled`.

## 3. Build plan

One implementer, one worktree, in this order. Each step ends with its tests and documentation; sizes are working days.

| Step | Work | Size |
|---|---|---|
| 1 | Documentation of what already holds: "Past a guard" (without the audit row), the lookup tiers, closing runs, the `step.met` note, the KB-as-subject scope note | 0.5 |
| 2 | 2.1: `gates_fact`, the empty-set texts, the `gate status` note, the `gate lines` header, the self-staling note | 0.5 |
| 3 | 2.2 kernel: `writfold_core::command` (the reader moved, here-documents, skipped words, `git` options, `--help`, directories) with its table tests | 1.25 |
| 4 | 2.2 wiring: matching on invocations, judging per directory, rule discovery per directory, unresolved directories, the worktree name, the delivery guard on the shared reader, re-measured cost | 1 |
| 5 | 2.3: the case format, the runner, the seed suite, the README | 0.75 |
| 6 | 2.4 tiers: `norm` and `prefix_match`, the `normalized`, `prefix` and `heading` tiers, `matched_on` | 0.75 |
| 7 | 2.4 measuring: `find --eval`, the fixture's `queries.jsonl` in the unit gate, the measured times | 0.5 |
| | **Cut line: steps 1–7 are v0.5.0-dev.10** | **about 5.25** |
| 8 | 2.4 candidates on an empty answer | 0.25 |
| 9 | 2.5: `mode = "audit"`, the waiver flags, the guard log, `guard log`, the audit row of "Past a guard" | 1.5 |
| 10 | 2.6: `runs --idle` | 0.5 |
| 11 | 2.7: `kb delta`, marks, the session-start mark | 1.25 |

Steps 8 to 11 are independent of each other; each ships in dev.10 when complete and gated, or moves to dev.11 whole. Step 5 depends on 2 to 4 and must not slip behind them: the cases are how the decisions of 2.1 and 2.2 are stated. Step 7 stays above the line with step 6: a ranking change without its measurement cannot be judged, and the evaluation file is what later decides whether semantic search is worth building. If step 4 runs long, the worktree name may move to dev.11; judging per directory and the unresolved text may not, since a reader that finds `git -C` commands without judging the right repository would add refusals about the wrong one.

Release steps as usual: version and guide status lines, `scripts/gate quality` and `scripts/gate unit` from main with receipt paths in the commit message, website copy only for what was exercised. The release note names the three behaviour changes: `gates.met` over an empty set, commands now matched through `git -C` and wrappers, and here-document bodies no longer matched.

## 4. Alternatives and reasons for not building

### 4.1 A "warn" mode (item 1)

Not built. A rule in `warn` would add text to a call it lets through. That tests the wording of a message, not the rule: what matters before promoting a rule is every call it would have blocked, with the command and the reason, and that is the log. A single non-enforcing mode also keeps the honest reading simple: a rule either decides or is being audited, and an audited rule is flagged wherever it is shown.

### 4.2 A rule-level waiver, `guard waive` (item 3)

Not built. Three reasons:

- **For every requirement that is a gate, the waiver exists.** `gate waive` writes a tracked file outside `.git/`, with who and why, and it is bound to the gate's definition and scope bytes, which is a tighter limit than a clock: it lapses the moment the thing it excuses changes. It is shown as `waived`, never `satisfied`, in `gate status`, `gate lines`, `resume` and now in the guard's own answers.
- **For what is not a gate, a waiver is the wrong tool.** No current run is fixed by starting one. A rule that matches what it should not, or a KB that cannot be read, is a fault in the rule or the machine; excusing one call leaves the fault for the next. Demoting the rule to `mode = "audit"` is the record that fits: one tracked line, visible in the diff, with the log showing what it would have refused.
- **A waiver the refused agent can file for itself, named in the refusal, makes the refusal optional.** Blocking works because it is not a suggestion. Writfold therefore does not put a waive command in a refusal's text; the guide documents the way past, and a team that wants waivers kept for people asks before `gate waive` runs.

A time limit on waivers (`gate waive --until`) is planned only if real waivers are seen to outlive their reason. It would put a clock into gate standing, which today depends on recorded order and content alone, and the standing kernels are among the proved ones.

### 4.3 "Skip" for a directory that cannot be resolved (item 2)

The candidate rule was that a path holding an unexpanded variable or a glob is skipped, never guessed. Never guessing is adopted. Skipping is not: in a guard, a skipped matched command is a pass, and `git -C "$REPO" push` would then be the way around every rule. The matched command takes the rule's action with a reason that names the fix (write the directory literally), which costs one retry and guesses nothing.

Not built either: following `bash -c`, `eval` and `xargs`. Each is a second parser with its own quoting, and each miss is a pass, which is the cheap direction for a guard to be wrong in. Excluding a clone by where it lives (a cache directory, say) is not built: a tracked rule cannot hold one machine's paths. Map that clone under its own name in the local repository map and exclude the name with `repos`.

### 4.4 Closing runs automatically (item 5)

Not built. Writfold records and never schedules: nothing runs between commands, so "close when complete" would have to be invoked, and then it is `status RUN --set closed`. "Every required item supported" is also not "done": gates say what was checked, the plan's position says what was reached, and only the person or the wrap-up step knows the work is finished. User-declared lifecycles remain not planned ([mission.md](mission.md)). What an open, finished run costs is noise in listings, and `runs --idle --plan` removes it: the idle runs whose plan is complete are the ones to close, in one reviewed loop.

### 4.5 `delta --since <timestamp>` (item 6)

Not built. A record's `created` and `updated` are dates; a run file's name carries a time but a hand-edited file carries none; file modification times are reset by a checkout and absent from a clone. Any answer by timestamp would be right on one machine and unknown on the next. A baseline is exact: a revision for CI and review, a mark for a session. The mark is written at session start by Writfold's own hook, so the end-of-turn question stays one stable query.

### 4.6 Lookup layers not built (item 7)

**Running a layer only when the one before finds too little.** Not adopted. Every tier runs and the rows are merged in tier order, each record once. The result is the list a cascade would give, with the later tiers' rows below it, at a measured cost of about 10 ms a call; and "too little" would be a threshold that changes which rows appear.

**A second trigram table, or typos in body words.** Not built. The `trigram` table over ID, title and body exists and serves `substring` and `typo`. Edit distance against every body word is a different cost class from distance against names; candidates on an empty answer cover the misspelt name, and `text` covers a correctly spelt body word.

**Semantic search: deferred.** The shape that fits, so that it can be built without changing anything adopted here:

- **Declared, not built in.** `.writfold/embed.toml` names a program found on `PATH`, as a checked kind names its checker, speaking newline-delimited JSON: `describe` (model name, a digest of the model, dimensions), then one request per chunk, answered with a vector. Writfold links no model and opens no connection.
- **Run only by an explicit verb** (`kb index embed`), never by `find`, a hook or any other reader, as checkers run only in `kb stale`.
- **Stored in the derived index**, in the `embeddings` table the v1 design already sketches: one row per record chunk (a section, by its line range), keyed by the SHA-256 of the chunk text and the model digest, so unchanged chunks are never embedded again and a changed model invalidates its rows. It is a cache, like the verdict cache: rebuildable from Git plus the program, never a source of truth, and absent on a clone that has not run the verb.
- **Searched by a scan in Rust** (cosine over the stored vectors). A few thousand chunks need no vector extension; `sqlite-vec` would be a new native dependency in a static binary, and Omnigraph stays frozen.
- **Opt-in per query** (`find --semantic`), because embedding the query starts the program and loads a model: seconds, not milliseconds, so it is never on a hook's path. Rows carry `matched_on: "semantic"`, a score, the chunk's line span and the flag `!semantic`. A semantic hit is a pointer to read, never evidence: it grounds no checklist item and no citation.

What blocks it: there is no measurement showing that the lexical tiers miss often enough to pay for a model at query time. `find --eval` is that measurement. The condition to start is a team's query file whose misses, after this release, are phrases that share no word with the record.

**Rank fusion: not built.** With lexical tiers only, a fixed tier order is deterministic and explains itself: the row says which tier found it. Reciprocal-rank fusion earns its place when two rankings of different kinds disagree, which is the semantic layer; it is decided then, by `find --eval` on the same query file.

**Aliases on tickets, decisions and other Markdown records: deferred.** It needs a new front-matter key, which changes the record file's shape for every other reader of the layout; the same kind of change was put off for `verified_basis`. The `normalized`, `prefix` and `heading` tiers answer the cases seen so far without touching a file. If phrases that share no token with the ID, title or a heading keep missing in a team's query file, the planned shape is an optional `aliases:` list read into the existing alias table.

**Aliases learned from misses: deferred.** It needs a local log of queries and a rule joining a miss to a later citation, and it would propose into a key that document records do not have yet. Until both exist, the miss is written down where it is useful at once: a line in `.writfold/queries.jsonl`, which fails `find --eval` until some tier or an alias finds it. If built, it files proposals only, through the existing queue, never a write.

**`find a b c` as a batch: not built.** `find` joins its arguments into one query (`find staging sweep` is the phrase), so a batch would need a new flag and an envelope holding several answers. One call takes about 10 ms; a loop in the caller is within any hook's budget and keeps one answer per envelope. `find --eval` is the many-queries form, for measuring.

**A lookup after a miss.** A miss exits 2 with `answer=unknown reason=no-record`, says that absence in the KB is not evidence of absence, and from this release may name candidates. Whether an agent returns to the KB after searching code is not something a lookup verb decides; a `run-record` gate (`research.backfilled`) on the research step is the blocking form.

## 5. Open questions

None that cannot be decided here. Two decisions change behaviour for existing users and are restated for a deliberate yes:

1. **`gates.met` over an empty set refuses** (2.1). Recommendation: yes. The alternative leaves a guard saying `would-pass` where `gate status` says `unknown`.
2. **A linked worktree takes its repository's name** (2.2), which makes receipts filed under the worktree's directory name read `open`. Recommendation: yes, in dev.10 if step 4 has room, otherwise dev.11 whole.

## 6. What was checked

Each description of today's behaviour was compared with the code at `f2ced5b`; the rows marked "run" were also run with the release binary on a copy of `fixtures/kb/basic`.

| Claim | Finding |
|---|---|
| A `commands` rule matches only a command at the start of a simple command, never quoted text or an `ssh` argument | Correct for quotes and `ssh` (run). Not for a here-document body: each body line is read as a command, so `cat > f <<EOF` with a line `git commit` is refused (run) |
| A rule can refuse every command because the working directory is inside a repository | It cannot: a rule must name `commands` or `tools`; `cat` and `grep` pass (run). Only a rule that lists the tool `Bash` matches every command |
| The repository comes from `git -C` before the working directory | No. `git -C DIR commit`, `git --no-pager commit` and `git -c k=v commit` match no `git commit` rule at all (run); the repository judged is always the one enclosing the event's `cwd`, else the run's recorded worktree, else the KB's, also after `cd DIR &&` |
| `~` and `$HOME` are expanded; a variable or glob is never guessed | Nothing is expanded today; the reader marks a word that holds `$` or a backquote and has no flag for `~` or a glob |
| `--help` runs are let through | No: `git commit --help` is refused (run) |
| Wrapped commands are matched | No: `sudo`, `env`, `command`, `time`, `if …`, `{ …`, `!` and a path to the binary all pass (run) |
| Linked worktrees are the same repository | Only through `remote.origin.url`. With no origin, the worktree is named by its directory, and the local map names only the clone whose Git directory it lists (run: `api` and `api-wt`; both `ledger` once an origin is set) |
| `gate status` tells "nothing recorded" from "ready" | Correct: no gates is `answer=unknown reason=no-gates`, exit 2; a checklist with no rows is `open`, `0/2 required covered`, exit 2; no current run is refused, exit 3 (run) |
| A guard requiring gates treats empty as not ready | No for `gates.met`: with no gate owed the rule passes and `guard status` says `would-pass` (run). Yes for `gate:<id>`: an undefined gate is owed |
| With a plan that names no gate, `gate status` says so | It says `no gates defined` although one is (run) |
| A waiver outside `.git/` exists | Yes: `gate waive` writes `runs/<run>/statements/…`, shown `[waived]` with reason, who and when; `gate lines` prints it; `gates.met` accepts it. `guard status` shows `would-pass` with no sign of the waiver (run) |
| A waiver always stands once written | Not when the subject is the KB's own repository and the scope admits `runs/`: the statement changes the scoped content and reads as for other content (run) |
| Runs can be listed by age, and closed | `runs` orders by the last checkpoint (else `created`) and has `--open`; there is no age filter. Closing is `status RUN --set closed`, with no condition |
| A query for what was written since a point exists | No. A `run-record` gate (`research.backfilled`) answers it for a plan step or a run; records carry dates only |
| `find` normalises spaces and hyphens | Ticket forms only (`HBR 101`, `hbr101`). `harbor api` and `harbor_api` reach `harbor-api` through the text tier, fourth; `HAPI` misses the alias `hapi` as exact (run) |
| `find` has aliases and a body-text fallback | Assets have `aliases`; signatures, investigations and projects answer to their names; tickets, decisions and runbooks have none. Body text is searched on every query, not as a fallback (run) |
| The text search is FTS5 with a trigram tokenizer, in the same file | Yes: `fts` (FTS5, `unicode61`) over ID, title, summary and body with BM25, and `tri` (FTS5, `trigram`, `detail = none`) over ID, title and body, in the one derived index. `rusqlite` 0.37 with `bundled` compiles SQLite 3.50.2 with FTS5. Aliases are a separate table and reach the text tier only where the file's body holds them |
| The index stores what a citable snippet needs | Bodies in full, and each section's name with its first and last line (`sections`). The `match:` line already carries a path and line. Headings are not a search column: `find` reaches one only as body text |
| A prefix of a name finds it | Not as such: `quay work` lists `quay-worker` second, through the any-word text tier; a misspelt body word (`batchng`) finds nothing and nothing is suggested (run) |
| A per-hit provenance field exists | The tier is the first word of the `match` continuation's text; there is no key for it |
| A vector table exists | No. The v1 design sketches an `embeddings` table and the mission allows one later, in the same index, never as a source of truth |
| `find a b c` is a batch | No: the arguments are joined into one query. One call takes about 10 ms (30 calls in 0.31 s) |
| Hook cost | An unmatched `Bash` event about 1.4 ms, a matched `git commit` about 15 ms, process start included (100 and 50 calls) |
| Guard decisions are pure Rust | `decide`, `combine` and the Stop rule are in `writfold-core`; matching and the reader are pure but live in `writfold-kb`; requirements read the KB. 2.2 moves the reader and the matching into the kernel crate |

## 7. Decisions (owner, 15 November 2026)

These override the sections above where they differ.

1. **Both behaviour changes in section 5: yes.** `gates.met` over an empty set refuses, and a linked worktree takes its repository's name. The release note names both, with `repos` as the way out for the first.
2. **An unresolvable directory takes the rule's action** with the "name it literally" text (4.3), rather than being skipped.
3. **Tuft, Weft and Splicery are frozen** (see `mission.md`, Component status): they keep building and their tests keep passing, with no new development. Nothing in this round extends them. The audit note on `edit apply` (2.5) is guard behaviour over an existing verb and stays. Step 1 also sets the status lines of `docs/tuft/language.md`, the Weft section of `gates.md` and the Splicery parts of `feeding-back.md` to say they are frozen, and the guide index says so once.
4. **Cut line:** steps 1–7 must land. Steps 8–11 are built in this round too and each ships in dev.10 when complete and gated, or moves to dev.11 whole.
