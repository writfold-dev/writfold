# Behaviour changes by release

> **Status:** kept with each release from v0.6.0-dev.6 on; the entries for v0.6.0-dev.1 to dev.5 were gathered from their release notes on 17 November 2026. Each line names something that was built in the release it is listed under.

This page is for someone pinned to one release who wants to upgrade on purpose. It lists only what an existing setup can notice: a different answer, exit code, file or default for input that was already valid. New verbs and new optional keys are not listed unless they change what an old command does; the release notes have those. Within the v0.6.0 pre-releases the entries add up: upgrading from v0.5.0-dev.9 to the newest means reading the whole page.

A line marked **fix** changed an answer that was wrong; anything depending on the old answer was depending on the fault.

## v0.6.0-dev.27

- **fix: a run's unread files** ([#181](https://github.com/writfold-dev/writfold/issues/181)): `resume` and `resume --packet` on a run that holds files in no format Writfold reads (another tool's `observations/`, a stray file, an unknown file in a Writfold directory) print an `unread` row, count `unread=N`, and read `coverage=partial` and `packet=partial` (exit 2) where they read complete (exit 0). A run with only Writfold's own files is unchanged. `--json` gains the count and the row. A hook or script that treats exit 2 from `resume` as a failure will see it for runs another tool wrote.
- The laws of the calculus have a page, [docs/writfold/laws.md](../writfold/laws.md), generated from the Lean sources, and every kernel is now linked to a law, declared out, or on a backlog that only shrinks ([#206](https://github.com/writfold-dev/writfold/issues/206), [#207](https://github.com/writfold-dev/writfold/issues/207), [#208](https://github.com/writfold-dev/writfold/issues/208)). No answer changes.

## v0.6.0-dev.26

- **fix: a fork clone's citations** ([#179](https://github.com/writfold-dev/writfold/issues/179)): in a clone with several remotes, citations are judged at the remote whose URL names the cited repository, not at `origin`. Rows that read `gone`, `changed` or `moved` only because `origin` was a stale fork now read what the cited repository says. `kb stale` rows and `kb repos` name the judged ref by its branch (`(origin/main)`, `(upstream/main)`) where they said `(origin/HEAD)`.
- **fix:** a qualified `repo=org/name` whose clone has no remote naming `org/name` (a fork-only clone), or whose naming remotes disagree or have no default branch, reads `unverifiable (target-ambiguous: …)`. `kb repos` shows such a row `[ambiguous]`, counts it (`ambiguous=N`) and exits 2.
- **fix:** a citation whose cited commit is ahead of the judged commit reads `unverifiable (target-behind: …)` where it read `gone`, `changed` or `moved`.
- New: `ref=<ref>` after the path on a line of the local repository map names the commit to judge that clone at.
- `cite make` cites the commit `stale` will judge at and writes that remote's URL as `url=`; in an ambiguous clone it refuses (exit 3) with the reason ([#180](https://github.com/writfold-dev/writfold/issues/180)). `record decision --commit` checks the commit against the same target.
- The asset boundary ([#196](https://github.com/writfold-dev/writfold/issues/196), [RFC](../rfcs/2026-11-27-asset-boundary.md)): what a chosen target can support is decided by a provider-neutral proved kernel, `writfold_core::referent`. Answers are as listed above.
- The calculus in Lean ([#205](https://github.com/writfold-dev/writfold/issues/205)): `proofs/Calculus/` states the words of a judgment and the accounted universe with their laws, audited with the kernel proofs. No answer changes.
- The installer names Claude Code's configuration directory and where it came from; with `CLAUDE_CONFIG_DIR` set, the plugin step runs only with `--plugin` ([#185](https://github.com/writfold-dev/writfold/issues/185)).

## v0.6.0-dev.25

- A run's policy pin records, for each delivery mode with `agent = "<id>"` that its steps owe, that agent's closure digest (`agent:<id>`, [#161](https://github.com/writfold-dev/writfold/issues/161)). While the step owing the mode is open, an edited profile, skill, `model` hint or instructions file makes the run read `policy moved` (exit 2 from `plan status`), as an edited gate or mode already did; `plan repin` adopts it. A run pinned before has no `agent:` entries and is judged as before.

- The KB-consistency fixture's skill (`fixtures/kb/kb-consistency/`) reads each member's file from its input, and the rehearsal gives it the paths: a real worker cannot open a record from its id alone ([#150](https://github.com/writfold-dev/writfold/issues/150)). The fixture's closure digest changes.
- New, outside the binary: the receiver's pre-receive hook recipe ([receiving work](receiver.md), [#64](https://github.com/writfold-dev/writfold/issues/64)), and the Claude Code adapter `plugins/writfold/adapter/claude_code.py` ([guide](claude-code-adapter.md)).

## v0.6.0-dev.24

- **fix** A cover row that cites a withdrawn report (`delivery:<candidate>`) reads stale, and `gate cover` refuses to cite one (`is not grounded (withdrawn)`) ([#164](https://github.com/writfold-dev/writfold/issues/164)).
- **fix** `deliver prepare --invocation` records the run's current step in the candidate (`step`), and `invocation.bound` judges that step. Checking an unchanged report again after `plan advance` now passes instead of failing as `another-step`. A candidate written earlier has no `step` and is judged as before.
- **fix** A generated checklist has units only when its scope's kept manifest says `coverage` `complete`, has schema `writfold-manifest/1`, lists every member with an id, path and digest, names no member (and no seed) outside the scope gate's or the checklist's `scope`, and makes no two units of one name ([#165](https://github.com/writfold-dev/writfold/issues/165)). A manifest that read before and fails one of these now gives no units, with a `why:` naming it; `gate cover` refuses the same way. A scope gate that stands on a waiver gives none either.
- `kb lint` reports an `items_from` gate that is undefined (`ref.undefined`), not mechanical or not retaining the file (`ref.wrong-kind`), and warns when the scope gate's `scope` is narrower than `**` (`policy.narrow-scope`).
- A generated checklist's new cover rows record the digests of the seed and of their two members (`unit`), and stay current while those and the definition are unchanged ([#166](https://github.com/writfold-dev/writfold/issues/166)): editing one member stales only its units, and a member added leaves earlier units covered. Rows written before keep the earlier rule. `impact --manifest` writes `seed_path` and `seed_digest` when the seed is a record.
- `agent prepare` refuses `--out` inside the KB and `--for` a gate or item the checklist does not have now; both exit 3 and write nothing ([#167](https://github.com/writfold-dev/writfold/issues/167)). An invocation record of a profile with `needs` gains `needs`, the gate and the receipt or statement each stood on; a record without needs is byte-for-byte as before.
- The refusal for a report prepared for another unit now says the unit takes `a report prepared with agent prepare --for …` ([#168](https://github.com/writfold-dev/writfold/issues/168)).

## v0.6.0-dev.23

- **Research contracts** ([#153](https://github.com/writfold-dev/writfold/issues/153), [research](research.md)): `kb impact --manifest FILE`; checklist keys `items_from`, `units` and `empty`; `delivery:<candidate>` as a cover reference; `agent prepare --for GATE.ITEM`; `needs` on skills and agent profiles. A definition without the new keys keeps its digest, and `impact` without `--manifest` answers as before.
- With `--manifest`, `impact` does not walk edges declared in a run's own files, so its answer can differ from the same walk without the flag; the note says so.
- A cover row's file name writes each `/` of its item as `_`. Only generated units have one, so existing rows keep their names.
- An invocation record gains `for` when prepared with `--for`. A record without it keeps its bytes.

## v0.6.0-dev.22

- **Agent vocabulary** ([#143](https://github.com/writfold-dev/writfold/issues/143), [agents](agents.md)): new definition files `.writfold/tools/`, `.writfold/skills/` and `.writfold/agents/`, read by `kb lint`; new verbs `agent inspect` and `agent prepare`; a new run directory `runs/<run>/invocations/`. A KB without these directories lints and answers as before.
- Every definition file Writfold reads now accepts multi-line `"""` strings. Such a file was refused before (`syntax.subset`); a line-ending backslash and more than three closing quotes are still refused.
- `deliver prepare --invocation ID` writes `invocation` into `delivery.json`. A manifest without it keeps its bytes. A new built-in check, `invocation.bound`, with a new mode key `agent`. The check is never a default, so the built-in modes and existing receipts are unchanged.
- `resume` adds one row per invocation (the newest three), and the run row in `runs` and `resume` gains an `invocations:` line, when the run has any.
- `kb init --example agent` writes the example in [agents](agents.md).

## v0.6.0-dev.21

- **Vale for documents** ([#117](https://github.com/writfold-dev/writfold/issues/117)): a new built-in check, `prose.vale`, with new mode keys `vale` and `vale_block`, new profile files `.writfold/deliveries/vale-<name>.toml`, and a new KB directory `.writfold/vale/<name>/` for the Vale config and styles. It is never a default, so the built-in modes and existing receipts are unchanged. Mode names starting `vale-` are refused, as `deny-*` and `prose-*` are. `kb init --example delivery` also writes a `design-doc` mode, the `house` profile and an example style.
- **Behaviour change, `prose.rules` on documents** ([#135](https://github.com/writfold-dev/writfold/issues/135)): a leading YAML front-matter block (a first line `---` up to the next `---` or `...`) is no longer prose, so rules do not match it; line numbers still count from the top of the draft. The check's obligation digest gains a marker for this, so an earlier `prose.rules` receipt reads stale once: check the draft again. With more than 20 matches the note ends with a count per rule, such as `; by rule: 41 filler, 12 hedge`.
- `prose.vale` is now tested against the real Vale 3.28.1 as well as the stand-in ([#133](https://github.com/writfold-dev/writfold/issues/133)). The tools image every gate runs in carries that release, and the unit gate fails if it is missing. The test runs Vale's JSON report, exit statuses 0, 1 and 2, the example `design-doc` flow, findings equal to a native run on the same bytes, and a planted global config, `VALE_CONFIG_PATH` and `VALE_STYLES_PATH` that change nothing.
- A failed `prose.rules` finding is drawn with the help line "a blocking rule" instead of "a blocking prose rule". The JSON `column` of a finding is now read from a detail such as `column 14: warning: …` as well as `column 14`.

## v0.6.0-dev.20

- **Prose rules for delivery drafts** ([#127](https://github.com/writfold-dev/writfold/issues/127)): a new built-in check, `prose.rules`, with a new mode key `prose` and new rule files, `.writfold/deliveries/prose-<set>.toml`. It is never a default, so the built-in modes and existing receipts are unchanged. Mode names starting `prose-` are refused, as `deny-*` is. `kb init --example delivery` writes `prose-plain.toml` and lists the check in `release-note.toml`.
- **Advice on delivery receipts** ([#115](https://github.com/writfold-dev/writfold/issues/115)): `result.advice` and `result.advice_total`, written only when there is advice, so a receipt without advice keeps its bytes.
- **Behaviour change:** a candidate script gate's stdout on exit 0 is now kept as advice instead of being discarded. A finding line may name its rule as `<line>: [rule.name] text`. Outcomes and exit codes are unchanged.
- The rule that a delivery receipt's outcome agrees with its findings (a pass has none, a failure has some) is now the proved kernel `writfold_core::prose::agrees`. Answers are unchanged.

## v0.6.0-dev.19

- **Pre-alpha cleanup** ([#105](https://github.com/writfold-dev/writfold/issues/105)): readers for formats of the tools removed on 21 November 2026 are gone. Removed: `resume`'s retrieval-receipt and edit-receipt rows and their notes; grounding a checklist cover row on a retrieval receipt or edit receipt (such a reference now reads `unresolved`, so `gate cover … supported` refuses it); retrieval receipts counting towards `research.backfilled`; `retrieval` and `edit-receipt` in `kb delta`'s run kinds (those paths now read as `run-file`) and `retrievals/`, `edits/` in `runs --idle`; the kernel function `writfold_core::ground::of_citations` with its two theorems and mutant. A gate receipt without `seq` is now invalid instead of ordering as 0. Plan events no longer write or read `records.retrievals`, so the run-record digest changes and an earlier `research.backfilled` or `run.reflected` receipt reads stale once: run the gate again.
- **Retained gate evidence** ([#121](https://github.com/writfold-dev/writfold/issues/121)): a new optional gate key, `retain`. A definition without it keeps its digest and its receipts keep their bytes. With it, a missing declared file makes a receipt `unknown`, and a receipt whose retained files are later deleted or edited reads `[invalid]`. `runs/<run>/evidence/` is a new run directory (`kb delta` kind `gate-evidence`), and it is not indexed.
- New: `resume --packet`, the bounded handover ([#109](https://github.com/writfold-dev/writfold/issues/109)). `resume` without it is unchanged.
- New recipe, no change to existing answers: an optional Plyrune check as an ordinary command gate (`docs/guide/recipes/plyrune-check.sh`), rehearsed against a stand-in checker ([#106](https://github.com/writfold-dev/writfold/issues/106)).

## v0.6.0-dev.18

- **fix** `kb runs` and `kb runs --idle` read `coverage=partial` (exit 2) while a directory under `runs/` holds files but no `manifest.json`. The trailer counts them (`no_manifest=N`), a note names each directory, and `kb index status` lists each as unreadable ("no manifest.json"). Before, such a directory was silently missing from a list that read complete. Other verbs are unaffected: the directory's files are still indexed ([#98](https://github.com/writfold-dev/writfold/issues/98)).
- `kb runs` names, in a note, each `manifest.json` under a `runs/` directory that is not `runs/<id>/` at the KB root ("only runs/<id>/manifest.json is a run"), and `kb index status` lists it as `not-a-run`. Coverage and exit codes are unchanged (#98).
- A run whose state is neither `open` nor `closed` is flagged `vocab=unknown`, and `kb runs` counts it (`vocab_unknown=N`) with a note that `--open` leaves out only closed runs. `--open` and `--idle` still list such a run (#98).

## v0.6.0-dev.17

- **fix** A code citation whose cited commit is not in the local clone (a shallow or single-branch clone, an unfetched or rewritten commit) now reads `unverifiable` `history-not-local` instead of `changed` or `gone`. This applies to a token with no `blob=` and no `span=`, and to a path renamed or deleted since. A token without `rev=` that would need one reads `nothing-to-compare`, and the 64-revision rename cap reads `rename-unchecked`; both used to read `gone`. Git failures while following a rename other than an unknown commit read `git-error`. Exit codes are unchanged (both exit 2). The trailer moves those citations from `stale=` to `unverifiable=`, and a delivery's `claims.cited-fresh` check reads `unknown` instead of `fail` for them. `proposal accept` still refuses them. An authored `stale=` mark now shows on them. `kb repos` lists a shallow clone with `history: shallow` and a `shallow=` count ([#90](https://github.com/writfold-dev/writfold/issues/90)).
- `kb runs` no longer prints an empty `field: ""` line for a run with no project, place or base ([#97](https://github.com/writfold-dev/writfold/issues/97)).

## v0.6.0-dev.16

- No change to the binary's answers: this release only speeds up the release process ([#91](https://github.com/writfold-dev/writfold/issues/91)–[#95](https://github.com/writfold-dev/writfold/issues/95)). Pre-releases record proof claims at tauten's ship level, re-calibrating only the claims whose inputs changed. A drifted mutant site fails the quality gate in seconds. The unit gate runs test binaries four at a time. The Lean translation is split per kernel module (`proofs/Extracted/WritfoldCore/`), so a mutant rebuilds only the theorems it can reach. `scripts/release` cuts the release.

## v0.6.0-dev.15

- **Loops** ([#74](https://github.com/writfold-dev/writfold/issues/74)): new step words `loop=`, `max=`, `min=` and the verb `writfold kb plan again`; a template without them reads, splices and folds as before. An earlier binary refuses a template (or a run snapshot) that uses them (`value.form`). New diagnostic codes `value.no-exit` and the warning `policy.self-exit`. A guard rule may require `step.settled` (#79); a `step.met` nudge on a loop step now names `plan again` when a round is left. A `plan again` splice event records an optional `until` (each gate's standing and evidence when the round began; earlier binaries ignore it), shown as `began:` in `plan status`; `watch` folds earlier rounds into one line (#81).
- **The index is rebuilt once** (schema 7: an index on each reference's token). A refresh after a record is added, removed, renamed or duplicated now rewrites only the links of the files whose tokens resolve differently, not every link; `last-refresh` reads `keys=same|moved graph=delta affected=N` and never `graph=whole`; a rename on a 50,000-file KB refreshes in 3 s instead of 32 s, and an edit that touches no record ID or alias no longer recomputes the keys ([#65](https://github.com/writfold-dev/writfold/issues/65)). Answers are unchanged.
- **fix** An evidence directory of a run (receipts, statements, covers, delivery checks, observations) that cannot be listed in full now reads as one invalid file instead of as empty or complete, so `gate status`, `gate lines` and `gates.met` do not read clean over it; `handoff check` refuses (`incomplete`) while any of the run's receipts, statements or covers does not read ([#67](https://github.com/writfold-dev/writfold/issues/67)).
- **fix** A `.writfold/guards/` that exists but cannot be listed (not a directory, a dangling link, a permission error) is reported as `file.unlisted` by the hooks, like a rule file that does not parse, instead of reading as no rules; `kb policy` reads `coverage=partial` (exit 2) for an unlistable `plans/`, `guards/` or `deliveries/` ([#63](https://github.com/writfold-dev/writfold/issues/63)).
- Session start waits for the brief at most 20 s (`WRITFOLD_HOOK_DEADLINE_MS`); past that, on a KB whose index is not built yet, it injects `index not built yet: run writfold kb index rebuild, then writfold kb brief` with the behind-remote line and the inbox instead of nothing ([#69](https://github.com/writfold-dev/writfold/issues/69)).
- `find`'s substring tier no longer loads the matching bodies and stops reading at the limit, and its typo tier reads names in batches; the rows are unchanged ([#70](https://github.com/writfold-dev/writfold/issues/70)).
- **fix** `record` and `propose` refuse an option the record kind does not keep, with a usage error (exit 1) and nothing written or queued: `record investigation --title T --prs PR-42` used to succeed and drop `--prs`; `--project` is kept only by a decision. Valid calls, stored proposals and record formats are unchanged. `record <kind> --help` and `propose <kind> --help` show that kind's options only ([#85](https://github.com/writfold-dev/writfold/issues/85)).
- **Project asset catalogs** ([#84](https://github.com/writfold-dev/writfold/issues/84), [#86](https://github.com/writfold-dev/writfold/issues/86)): `.writfold/catalogs/<name>.toml`, `kb catalog diff` and `kb catalog check` are new. Nothing existing changes unless a catalog is declared; then `kb check` gains a `catalogs` row and `kb lint` reads the declarations.
- The release profile sets `overflow-checks = true` ([#62](https://github.com/writfold-dev/writfold/issues/62)): an arithmetic overflow outside the proved kernel now aborts instead of wrapping silently.
- **fix** When a source kind or store declaration cannot be read, `brief`, `context`, `why` and edge flags add a note naming it and read `coverage=partial` (exit 2), and session start prints `Citations: could not be read: …`, instead of showing committed verdicts as never checked ([#68](https://github.com/writfold-dev/writfold/issues/68)).

## v0.6.0-dev.14

- `impact`, `uses` and `context` read at most `WRITFOLD_KB_WORK` edges (default 100,000); one that stops there says so in a note and reads `coverage=partial` (exit 2) instead of loading a hub's every edge. `impact` adds `examined=N` to its trailer; `uses` adds a `links: N more from this source` line where it keeps three of more ([#60](https://github.com/writfold-dev/writfold/issues/60)). 

- **fix** A `.writfold/gates` directory (KB root or subject repository) that exists but cannot be listed is now `error[file.unlisted]`, counted in `invalid=`: `gate status`, `gate lines` and `gates.met` are no longer satisfied over the gates that happened to load, `kb policy` reads `coverage=partial` (exit 2) instead of `coverage=complete`, and session start no longer says "policy: none configured" for it. Before, it was skipped as if absent ([#56](https://github.com/writfold-dev/writfold/issues/56)). `kb lint` reports the same fault for any unreadable policy directory it reads. `kb policy` now also reports definitions that do not parse (exit 2) instead of dropping them, and `kb policy --paths` refuses a malformed definition with its diagnostics.
- **fix** `index verify` and `kb check` no longer replace the index when the KB, or the live index, moved while they compared it: the answer is then partial (exit 2) with the note `the KB or the index moved while it was compared: nothing was replaced …`. Before, the comparison could report a mismatch and replace a healthy index. A temporary index file left by a killed rebuild or verify is removed by the next one ([#58](https://github.com/writfold-dev/writfold/issues/58)). Memory use of `index rebuild` and `index verify` no longer grows with the KB.
- **The index is rebuilt once** (schema 6: an index on each edge's source file, and incoming edges indexed in key order). After that, a refresh rewrites only the links of the files whose references changed, none when no reference changed, and every link only when a record, alias or duplicate changed; `index status` shows the last refresh's work (`last-refresh: files=1 graph=delta edges-deleted=41 edges-written=42`). Answers are unchanged ([#59](https://github.com/writfold-dev/writfold/issues/59)). An earlier binary rebuilds the index again when it next reads.
- `writfold kb handoff check` and `.writfold/handoff.toml` are new; nothing existing changes ([#57](https://github.com/writfold-dev/writfold/issues/57)).

## v0.6.0-dev.13

- **`kb edit apply` and `kb edit recover` are removed** with Splicery ([#54](https://github.com/writfold-dev/writfold/issues/54)). Finish or abandon an interrupted edit with v0.6.0-dev.12 before upgrading. Edit receipts already in a run stay readable: `resume` lists them and a checklist row may still cite one. A guard rule naming `writfold:edit` in `tools` now matches nothing.
- **A gate of `class = "assessed"` is refused** (`value.unknown`, and its `weft`, `checks` and `weft_sha256` keys `key.unknown`), with Weft: a plan step owing such a gate cannot be met until its definition changes. `runs/*/assessed/` receipts are no longer read. `$WRITFOLD_WEFT` and a `weft =` line in the user config are ignored, and the config hint no longer suggests one.
- `brief`, `context`, `why`, edge flags and the session-start stale line show a usable committed check receipt's verdict, as `stale --cached` and the gates already did: a clone that never ran `stale` may now see `!gone !recorded` rows, `unverifiable` `check-expired` rows past `max_age`, a `check:` line under `brief`/`context` rows, and a session-start line `Citations: … (N read from committed check receipts) …`. Only kinds that declare `max_age` and have committed receipts are affected ([#35](https://github.com/writfold-dev/writfold/issues/35)).
- **The command line is parsed by clap** ([#55](https://github.com/writfold-dev/writfold/issues/55)). Every verb and flag accepted before is accepted with the same meaning, and the exit codes are unchanged: a usage error is still exit 1. What differs: `writfold kb <verb> --help` (and `writfold kb help <verb>`) prints that verb's help — its lines of the overview and its options — instead of the whole overview, which `writfold kb help`, `kb --help` and `--help` before a verb still print. Usage errors are worded by clap (`error: unexpected argument '--jsn' found`, with a `tip:` naming a similar verb or flag) and no longer repeat the overview; `--X does not apply to <verb>`, the range messages and `--at`'s refusal keep their wording. A flag given twice is refused as before, in clap's words. `--flag=value` and `--` (the rest are positional) are now accepted. A positional that begins with `-` (other than `-` itself) now needs `--` before it. `--color auto|always|never` is new on every `kb` verb (auto: colour on a terminal unless `NO_COLOR`); `writfold watch` keeps its own `--color`. `writfold --help` is clap's, `writfold` with no command prints it and exits 1, and a mistyped command is suggested (`writfold kbb` → `kb`).
- Nothing else that an existing setup can notice changes: the resume cursor moved into `writfold-kb` byte for byte, so issued cursors still resume.

## v0.6.0-dev.12

- A Markdown record's front matter may list `aliases`. Without the key nothing changes. With it, `find` answers those names (often ahead of the text tiers), and a reference token equal to an alias of only that record now links to it. `kb lint --records` may report the warning `alias.collides` for such an alias, counted in `warnings=` (exit 2 under `--deny-warnings`). An earlier binary ignores the key ([#41](https://github.com/writfold-dev/writfold/issues/41)).

- A store declaration may set `recorded_max_age`. Without it nothing changes. With it, `stale --record` also writes `.writfold/checks/store@<name>/` receipts, and on a machine without the store's clone its pinned citations read the recorded verdict (marked `recorded`) instead of `unverifiable: store-not-local`, so a gate needing that evidence can be met there. Writing the key changes `kb policy`'s `identity`. **An earlier binary refuses a declaration that writes it** (`key.unknown`) ([#4](https://github.com/writfold-dev/writfold/issues/4)).

- `gate attest`, `gate waive`, `deliver attest` and `deliver waive` write a `seq` field (one more than the highest among that obligation's statements there), and statements are ordered by (`seq`, file name), not by their recorded time. When two attestations or waivers are about the present, the one written later now stands even if the machine's clock had moved backwards between them. Statements written before carry no `seq`, read as 0 and keep their file-name order. **An earlier binary ignores `seq`** and orders by recorded time ([#15](https://github.com/writfold-dev/writfold/issues/15)).

## v0.6.0-dev.11

- A checklist gate may list `snapshot = ["item"]`. Without the key nothing changes, and the definition's digest is unchanged. **An earlier binary refuses a definition that writes it** (`key.unknown`) ([#3](https://github.com/writfold-dev/writfold/issues/3)).
- The session-start context may gain a `Named here` part after the brief: records that the working branch's ticket-shaped words, the current run's task or the repository's name resolve to exactly. It is the first part cut under the 9,000-character limit, so nothing else is cut sooner than before ([#26](https://github.com/writfold-dev/writfold/issues/26)).
- **The index is rebuilt once** (schema 4): the `text` and `text-any` tiers stem English words (FTS5 `porter`), so `migrating` now finds "migrations" and rows in those tiers may come in a different order. Their `match:` line is the body line holding the first word the tier matched, which may differ from before (before, the first line containing a query word as a substring, so `end` matched "appends"). An earlier binary rebuilds the index again when it next reads ([#21](https://github.com/writfold-dev/writfold/issues/21)).
- `find --each` answers a query that is a record's own ID (`LAN-20`, `harbor` for `project:harbor`; not an alias) with its `exact` row only, the first row it gave before, and the note `an exact ID: the other tiers were not searched`. A lone `find` without `--limit` is unchanged. `find --limit N` is new ([#20](https://github.com/writfold-dev/writfold/issues/20)).
- **fix** `match` (and `find --signatures`) shows at most 50 rows like any other `find`; before, every matching signature was added even past the limit ([#16](https://github.com/writfold-dev/writfold/issues/16)).
- `find --eval` adds one note per class named in the queries' `note` fields (text before the first `:`), and the JSON trailer key `classes`. A query file whose notes have no `:` reads as before ([#19](https://github.com/writfold-dev/writfold/issues/19)).
- The plugin adds a `PostToolUse` hook for `Bash` and `mcp__*` calls (`writfold kb hook post-tool-use`). Without `.writfold/deliveries/guard.toml` it prints nothing. With one, after a call whose bytes are a ready candidate's, it adds one line of context naming the `deliver posted` command for that candidate ([#13](https://github.com/writfold-dev/writfold/issues/13)).
- A delivery mode may list a fourth built-in check, `length.bounded`, with the new keys `max_chars` and `max_lines`. The built-in modes' defaults are unchanged, so no existing receipt moves; a mode file that does not use them reads as before ([#12](https://github.com/writfold-dev/writfold/issues/12)).
- `kb policy` accepts `--paths GATE`; without it nothing changes ([#10](https://github.com/writfold-dev/writfold/issues/10)).
- `kb lint` without `PATH` may now report the warning `policy.never-ages` (once per undeclared source kind that has `no-data` or `not-found` looks) and counts it in `warnings=`; under `--deny-warnings` that exits 2 ([#8](https://github.com/writfold-dev/writfold/issues/8)).
- `gate status --json`: a `looked:` line whose newest look kept a snapshot gains the key `snapshot` (`store`, `rev`, `path`, `verdict`, `basis`). Text output is unchanged ([#2](https://github.com/writfold-dev/writfold/issues/2)).
- `resume`'s `artifact` rows no longer repeat their verdict as a flag (`[fresh] … !fresh` is now `[fresh] …`); in JSON the row's `flags` no longer holds the verdict, which stays in `status`. An `unverifiable` row still carries its reason as a flag ([#7](https://github.com/writfold-dev/writfold/issues/7)).

## v0.6.0-dev.10

- **A KB without `.writfold/stores/` behaves exactly as before.** With a store declared, `context` and `uses` on a record end with `material` rows (the store directories whose `README.md` names the record in `subject:`), the count `material=` and, in JSON, the trailer key `stores` ([stores](stores.md#material-behind-a-record)). They read each store's clone (one or two Git calls per store, plus one tree read per new commit), which they never did before.
- `kb repos` adds a `material:` line (`material=N ambiguous=N unknown=N`) to each readable store's row, and a `claim:` line for up to five directories listed under no record.
- `observe` accepts `--snapshot FILE [--store NAME]` ([stores](stores.md#keeping-a-looks-bytes-snapshots)). Without it nothing changes: an observation without a `snapshot` field parses, grounds and prints as before. With it, the first call may write two files into the store's work tree (never into the KB) and is refused until they are committed.
- An observation with a `snapshot` field grounds only while its snapshot reads fresh as well. **An earlier binary ignores the field** and grounds such a look on `cite` alone, so it does not see a lost snapshot.
- The index file gains two cache tables on first use. The index is not rebuilt, and an earlier binary opens the same file.

## v0.6.0-dev.9

- **A look that found nothing now ages** by its `--source` kind's declared `default_ttl` ([gates](gates.md#observations-as-evidence-built-in-v050-dev3)). A `none-found` row citing a `no-data` or `not-found` look of a kind declared with a lifetime turns `stale` once that lifetime has passed (`… was no-data, now aged`); before, such a look never aged. Looks of undeclared kinds, or kinds with no `default_ttl`, are unchanged. A kind catalog that cannot be read leaves such a look `unreadable`.
- `gate status` and `gate cover` add `bound:` lines under an item that stands on a look recorded `--for` another item (`!other-item`) or on a reference that also backs other items (`!shared`), and the item line carries those flags. No standing or exit code changes.
- A `scope` pattern starting `repo:` (or `!repo:`) now names a store ([gates](gates.md#a-definition-every-key)) and must be `repo:<store>/<glob>`; before, it was a glob over the subject's own paths. A definition with another `repo:` pattern is now refused.
- New optional keys `own_evidence`, `evidence_from` (checklists) and `ages_as` (mechanical and observed gates) and the optional `searched` field of an observation change nothing until written. Writing a key changes that definition's digest, so its earlier receipts, rows and statements read `stale` once.

## v0.6.0-dev.8

- `checkpoint` accepts `--artifact <store>@<rev>:<path>`, repeatable. Without it nothing changes. With it, the checkpoint also writes `runs/<run>/artifacts/<time>-<hex>.json`, and is refused, writing nothing, when a pointer would not read fresh ([stores](stores.md#pointing-a-run-at-its-material)).
- `resume` adds an `artifact` row per pointer, after the delivery rows, for a run that has any.
- Every write flag could be given once; `--artifact` is the one that may be repeated.

## v0.6.0-dev.7

- **A KB without `.writfold/stores/` behaves exactly as before.** Everything below needs a store declaration.
- A citation whose `repo=` names a declared store is found through the store's rule ([stores](stores.md#where-the-clone-is-found)), not only through the local map: a clone found under a `$WRITFOLD_KB_REPOS` root or a path hint whose `origin` is another repository is no longer read (`unverifiable: store-origin-mismatch`). A clone named in the local map is still used, flagged when its origin differs.
- A citation into a store declared `pinned` (the default) is judged at its own revision: a path moved, edited or deleted in the store since reads `fresh`, and a cited commit no branch or tag reaches reads `gone`. A short or symbolic `rev=` into such a store reads `unverifiable`.
- `kb repos` has a row for each declared store, even one nothing cites yet, flagged `!store`; a `mismatch` counts as missing (exit 2 with `--missing`).
- `kb policy`: the trailer gains `stores=`, and `identity` changes for a KB that declares stores.
- A faulty store declaration refuses `stale`, `repos` and `policy`, as a faulty kind declaration refuses `stale`.

## v0.6.0-dev.6

- **The current run is resolved per session, then per work tree, then from the shared pointer.** Before, one pointer per KB clone (`<KB git dir>/writfold/current-run`) decided for every hook and every verb without `--run`. Now a session's own pointer decides first, then the pointer of the work tree the command runs (or acts) in, and the shared pointer only when neither is set ([hooks](hooks.md#the-current-run-per-session-and-per-work-tree)). For one session in one repository nothing changes. What can be noticed:
  - Starting a run in one repository no longer changes the current run of a session or shell in another repository whose own pointer is set. A second session's `checkpoint --new-run` no longer moves the first session's run.
  - A script that set the current run by writing the `current-run` file is no longer read by a session or work tree that has its own pointer. Use `writfold kb resume <run> --here`, or delete `<KB git dir>/writfold/current-run.d/`.
  - A session whose run was closed is told the run is closed, even when the shared pointer names an open run. Before, with one pointer, there was no second run to fall back to, so this is the same answer as before; the change is that it is not replaced by another session's run.
  - `writfold watch` and verbs run inside the KB's own work tree read the shared pointer as before.
- **`checkpoint` writes up to three pointer files** (`current-run.d/session-<id>`, `current-run.d/tree-<hex>`, `current-run`) where it wrote one. They are local and untracked.
- **The `SessionStart` hook writes to `CLAUDE_ENV_FILE`** when Claude Code names one: one `export WRITFOLD_SESSION=<session_id>` line. Verbs run later in that session's shell read it: `propose` then counts its ten per session against that session (before, a shell with no `--session` shared one bucket), and run writes go to the session's run.
- **A checkpoint draft's `run`** is the session's current run, not the clone's.
- **`guard status`** adds one note when a pointer narrower than the shared one decided.
- **`plan repin` is built.** A run that is `policy-moved` still reads so, and still exits 2 from `plan status`, until it is repinned; the note that says so now names the verb. A repinned run never reads plain `complete` (`complete-with-exceptions`). After a repin, the pin in force for a gate (and for the required file) is the newest one naming it; before, the required file was always compared with the first pin. A done step whose record names another digest than its gate's now is kept at its own digest (the new `steps` key of a repin pin); an earlier binary ignores the key and may still read such a run `moved`.
- `find a b c` is unchanged (one phrase); several queries are `find --each a b c`.

## v0.6.0-dev.5

- `deliver match` shows the same candidate on every run when several hold the same bytes: one with a post record about its present bytes, else the newest postable one, else the newest. Counts and exit codes are unchanged.

## v0.6.0-dev.4

- Assessed and delivery check receipts gain `seq` and are ordered by (`seq`, name), not by their recorded time. Older receipts are read as before.
- An older draft no longer meets a later step once a newer draft of that mode is bound.
- A guard route whose posted text cannot be read is denied, naming the fix; before, another ready draft could let it pass.
- `gate lines` ends with the plan's summary word, lists a skipped step's gates as `skipped`, and exits like the summary.
- **fix** A second clone or a linked worktree on the same machine is judged in its own checkout; before, a check could read `satisfied` from the first clone.
- **fix** A delivery mode that declares no checks is never ready.
- **fix** A `*` in a scope pattern matches a literal `*` in a file name.

## v0.6.0-dev.3

- `plan status` exits 2 while a run's policy has moved since it was pinned (`policy=moved`): a gate definition, an instruction file or the required template was edited after the run's plan was set.
- `plan status` gains a plan row and `policy=`, `summary=` and `repinned=` in the trailer; a plan with an attested, waived or skipped step reads `complete-with-exceptions`, not `complete`.
- Each plan write (`plan set`, `plan use`) adds a pin file under `runs/<run>/policy/`.
- Closing a run with `status RUN --set closed --note …` keeps the note and the run's open obligations; a closed run is never reported complete.
- A receipt from another run or another subject counts as absent, not as stale evidence for this one.

## v0.6.0-dev.2

- A faulty definition file (gate, guard rule, plan template, delivery mode, deny list, kind declaration, `queries.jsonl`) reports every fault it has, each with a stable code, on stderr; before, the first fault only, as one line. `--json` answers gain a `diagnostics` array.
- `kb check` gains a `check definitions` row.
- **fix** A gate with a wrong-typed `version` is reported as that, not as a bad `timeout_seconds`.

## v0.6.0-dev.1

- **`gates.met` over an empty set no longer passes.** A guard rule requiring it in a repository with no gate, or in a run whose plan names none, takes its action. Leave such a repository out with `repos`.
- A `commands` rule reads the command line as the shell splits it. A here-document body no longer matches (before: a false refusal); `git -C DIR commit`, `sudo`, `env`, `command`, `time` and a full path to `git` now match (before: a silent miss); `git commit --help` passes.
- Each matched command is judged in the repository it acts on (`cd X &&`, `git -C`), not the event's working directory. A directory that cannot be told takes the rule's action.
- A linked worktree answers to its repository's name in `repos`.
- A passed `claims.cited-fresh` delivery check stands only while its citations are fresh; it is re-checked on every read.
- The `trial`, `serve` and `tuft` subcommands are removed; `--version` prints only the version.
- The KB root is no longer read from an earlier configuration file: use `--kb`, `$WRITFOLD_KB` or the plugin's `kb_root`.
