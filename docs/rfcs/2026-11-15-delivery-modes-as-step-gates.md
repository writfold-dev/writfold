---
rfc: "2026-11-15-delivery-modes-as-step-gates"
title: "Delivery modes as a step's gates"
status: accepted
implementation: complete
authors:
  - tknawara
created: 2026-11-15
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Delivery modes as a step's gates

> **Status:** built in v0.5.0-dev.10 ([as built](../writfold/kb-v1-design.md#42-delivery-modes-as-a-steps-gates-as-built), with its deviations). Written against `main` at `808b48b` (v0.5.0-dev.9, with v0.5.0-dev.10 planned) and read against [dev10-spec.md](2026-11-15-v050-dev10.md), whose changes to `gates.met`, the `gate status` notes and the command reader it composes with. The authority is [mission.md](../writfold/mission.md).

## 1. Need

A plan step such as "post the PR comment" can be advanced today with nothing checked. A step names repository, assessed, checklist and run-record gates; a delivery candidate's readiness is visible only in `deliver status` and to the optional delivery guard, and a candidate gate named on a step is refused. So the one step whose output leaves the repository is the one step the plan cannot hold open.

This change lets a step owe a delivery, written `deliver:<mode>`. The step cannot be advanced until a candidate of that mode, prepared in the run, is ready: every obligation of the mode met for the candidate's present bytes. Nothing else about delivery changes. Writfold still sends nothing; the done record says a candidate was ready, never that it was posted.

```toml
schema = "writfold-plan/1"
name = "change"
steps = [
  "implement: unit",
  "deliver: deliver:pr-comment",
]
```

## 2. Decisions

| # | Question | Decision | Reason |
|---|---|---|---|
| 1 | How a step names a delivery | A word `deliver:<mode>` in the step string, beside gate IDs and `use=`, in any position; at most once per mode per step. Schema stays `writfold-plan/1` | A gate ID is a slug and cannot hold `:`, so the word is unambiguous and no existing template changes meaning. A binary older than this release refuses such a template by its step text; it never misreads it |
| 2 | Which candidate decides an **open** (current) step | The newest candidate of that mode in the run that no done step is bound to (decision 4). Newest is by the manifest's new `seq`, then by ID; never by a clock | "Newest" is what `deliver` already teaches: repair in place or prepare a new one, and the new one is the one meant. `seq` follows gate receipts, so a wrong clock on one machine cannot change which candidate decides |
| 3 | Must the candidate be prepared after the step became current? | No. Any unbound candidate of the run may decide | Drafting the comment during `implement` is ordinary work, and refusing it would only force a second `prepare` of the same bytes. A step window would need the run-record baseline to count deliveries, which changes every plan event's bytes |
| 4 | Which candidate decides a **done** step | The candidate its done record names, and only that one, for ever. That candidate is *bound*: it never decides another step | Done is history. Without binding, a second step of the same mode would be met at once by the first step's candidate, and preparing a later candidate would turn an earlier done step stale |
| 5 | Explicit binding (`deliver prepare --step`) | Not built | Binding follows from the records (the done event); a flag adds a way to be wrong and nothing the fold cannot derive |
| 6 | What "passes" means | The deciding candidate is ready exactly as `deliver status` says `[ready]`: every obligation of its mode stands `satisfied`, `attested` or `waived` for its present bytes and the present mode, deny list and gate definitions | One meaning of ready across `deliver status`, the delivery guard and the plan |
| 7 | How the step's delivery gate is worded | One standing for the whole delivery, the weakest word that is true: an unmet obligation gives its own standing (the first unmet, in the mode's order: `open`, `stale`, `failed`, `unknown`); else `waived` if any obligation is waived; else `attested` if any is attested; else `satisfied` | A statement is never read as a passing check, here as everywhere. `satisfied` means every obligation passed a check on these bytes |
| 8 | May a waived check let the step advance? | Yes. The gate stands `waived`, the done record says `waived`, and `plan status` shows it | A waived gate advances a step today and a waived check makes a candidate ready today; a third rule for the same word would be a trap. The waiver is tracked text with who and why, bound to the candidate's bytes |
| 9 | Waiving the delivery as a whole | Not built. `gate waive deliver:<mode>` is refused (it is not a gate definition). Waive a check with `deliver waive`; close the step without a delivery with `plan skip --reason --by` | A skip is already the honest record of "this step was not done" |
| 10 | A mode with no obligation | Refused at `plan set`/`plan use`; if the mode file is emptied later the gate stands `open` with the reason | An empty set is not met ([dev10-spec 2.1](2026-11-15-v050-dev10.md#21-empty-is-not-met-item-4)) |
| 11 | No candidate, or only bound ones | The gate stands `open`; `next:` is the `deliver prepare` command | Open is "nothing recorded", as for a gate without a receipt |
| 12 | Staleness | Derived on every read. Candidate bytes changed, mode file changed, deny list or paragraph bound changed, a mode gate's definition, instructions or script changed: the affected obligations go `stale`, as they do today. New: a passed `claims.cited-fresh` receipt stands only while every citation token in the candidate is still fresh (section 5). A done step whose delivery no longer holds is `[done] !stale`; the current step never moves back | Like every other gate. The citation re-check closes the one case where a candidate read `[ready]` after its evidence moved |
| 13 | `gate status`, `gate lines`, the `gates.met` requirement | They do not list or count deliveries. `gate status` adds a note naming the plan's deliveries and their steps. `step.met` does judge them | A delivery is ordered by the plan: it becomes due at its step. `gates.met` is used on `git commit`; counting a PR comment there would refuse the commit the comment is about. A gate-wide answer is also not defined when two steps name one mode |
| 14 | A new guard requirement `deliver:<mode>` | Not built | `step.met` covers the plan; the delivery guard covers the posting command |
| 15 | Subject repository | A step is judged as today, with a subject repository, also when it names only deliveries | One judging path. A delivery-only step in a place with no subject passes `--repo DIR`. Listed under "not built" |
| 16 | A proved kernel | Unchanged. The delivery's standing is mapped into the existing `met: &[bool]` input through the proved `Standing::met` (section 7) | The advance decision does not need to know what kind of gate it is |

## 3. Template and `plan set`

`template::step` accepts a word `deliver:<mode>`. `<mode>` must be a mode name (`[a-z0-9.-]`, not `guard`, not `deny-*`, as `deliver prepare --mode`). The word is kept, as written, in the step's ordered gate list, so the refusal order of `plan advance` and the `met` list of the done record follow the template.

Refused when the template is parsed (exit 3, nothing written):

| Input | Refusal |
|---|---|
| `deliver:` , `deliver:Bad`, `deliver:guard`, `deliver:deny-external` | `plan template: step <id>: <word> does not name a delivery mode ([a-z0-9.-], not guard or deny-*)` |
| the same mode twice on one step | `plan template: step <id> names deliver:<mode> twice` |

Refused when the template is spliced (`plan set`, `plan use`, `use=` closure, required start and end templates), exit 3, nothing written, beside the existing check that every gate is defined:

| Case | Refusal |
|---|---|
| no such mode | `plan <t> names deliver:<mode>, which is not a delivery mode: add .writfold/deliveries/<mode>.toml to the KB root, or use a built-in mode: pr-comment, slack-summary` |
| the mode file does not load | `deliver:<mode>: <path>: <why>` (the mode loader's own reason) |
| the mode declares no check, gate or attestation | `plan <t> names deliver:<mode>, whose mode declares nothing to check; a delivery is not met by an empty set: add checks, gates or attest to .writfold/deliveries/<mode>.toml` |
| a candidate gate named directly (existing refusal, text extended) | `gate <g> judges delivery candidates; a plan step owes a delivery by its mode: deliver:<mode>` |

Two steps of one plan may name the same mode; each needs its own candidate (decision 4).

## 4. Judging

### 4.1 Selection

- Candidates of mode `M`: the run's candidates whose manifest reads as valid and names `M`, ordered by `(seq, id)`. A candidate with an invalid manifest belongs to no mode and decides nothing.
- **Bound** candidates: every candidate named by the `evidence` of a `deliver:M` entry in some done record of the run (`runs/<run>/deliveries/<id>/candidate.md` binds `<id>`).
- A **done** step's `deliver:M` is judged on the candidate its own done record names. If that directory is gone the gate is `open` (so the step is `!stale`).
- The **current** step's `deliver:M` is judged on the last unbound candidate of mode `M`. None: `open`.
- Later and skipped steps are not judged, as today.

After a merge of two clones, two done records may name one candidate; each step is judged on it and nothing is flagged.

### 4.2 `seq` on the manifest

`deliver prepare` writes `"seq": N` in `delivery.json`: one more than the highest `seq` among the run's candidates of that mode. A manifest without it counts as 0 and keeps ID order; equal values are ordered by ID. `writfold-delivery/1` is unchanged otherwise, and manifests already written keep their bytes.

### 4.3 Standing

The obligations are those of `deliver status` (mode checks, mode gates, `attest` names, for the candidate's audience), each judged by the existing kernel. The delivery's standing is decision 7. The done record's entry for the gate:

| Field | Value |
|---|---|
| `gate` | `deliver:<mode>` |
| `standing` | `satisfied`, `attested` or `waived` |
| `evidence` | `runs/<run>/deliveries/<id>/candidate.md` |
| `content` | the candidate's SHA-256 at that moment |
| `definition` | SHA-256 of `writfold-delivery-gate/1\n<mode>\n<audience>\n` followed by one `<obligation>\t<obligation digest>\n` line per obligation, in order |

`writfold-plan-event/1` is unchanged: these are the fields every `met` entry has.

## 5. Cited evidence that moved

Today a `claims.cited-fresh` receipt is bound to the candidate's bytes and nothing else, so a candidate keeps reading `[ready]` after a record it cites has changed. From this release, wherever a candidate's obligations are judged (`deliver status`, `resume`, the delivery guard, the plan), a `claims.cited-fresh` obligation whose receipt passed is checked again when the candidate holds at least one citation token: the tokens are run through `stale` (local, nothing fetched, the same call `deliver check` makes). If any is no longer fresh the obligation's checked state is `stale`, its line ends `; cited evidence moved: <n> of <m> citation tokens no longer fresh !stale`, and `next:` is `deliver check`. A waiver on that check still stands (it is bound to bytes and definition, not to the receipt). If the evidence returns, the receipt stands again. A candidate with no tokens costs nothing more.

Limits, stated in the guide: a mode gate of class `observed` is not re-run on read, and prose without a citation token is not seen.

**Compatibility, named.** A candidate whose citations moved after its check, which reads `[ready]` today, reads `[blocked]` after this change, and the delivery guard refuses it. The release note says so.

## 6. What each surface shows

### 6.1 `plan status` and `plan advance`

The gate is listed as `deliver:<mode> [<standing>]` wherever a gate is. The current step adds one `candidate:` line when a candidate decides, before `read:` and `next:`.

```text
$ writfold kb plan advance
step change#1/deliver [current] owes: deliver:pr-comment [open] @runs/run-…/plan/splice-change.1.json:1
  next: writfold kb deliver prepare --mode pr-comment --from FILE
# note: not advanced: change#1/deliver owes deliver:pr-comment [open] (first owed: deliver:pr-comment); nothing written
```

```text
step change#1/deliver [current] owes: deliver:pr-comment [failed] @runs/run-…/plan/splice-change.1.json:1
  candidate: 20261112T211045Z-ee94cc [blocked] owes references.denied [failed], tone.reviewed [open] @runs/run-…/deliveries/20261112T211045Z-ee94cc/candidate.md:1
  next: writfold kb deliver check 20261112T211045Z-ee94cc
```

```text
step change#1/deliver [done] deliver:pr-comment [waived] @runs/run-…/plan/done-change.1~deliver.json:1
step change#1/deliver [done] deliver:pr-comment [stale] @runs/run-…/plan/done-change.1~deliver.json:1 !stale
  why: no longer holds: deliver:pr-comment [stale] (done stays done; list the gate again on a later step if it must hold there)
```

`next:` for an owed delivery, from its first unmet obligation:

| State | `next:` |
|---|---|
| no deciding candidate | `writfold kb deliver prepare --mode <mode> --from FILE` |
| a check or script gate `open`, `stale`, `failed` or `unknown` | `writfold kb deliver check <id>` |
| an attested obligation `open` or `stale` | `writfold kb deliver attest <id> --check <name> --statement TEXT --by WHO` |
| the mode no longer loads, or declares nothing | no command; the `candidate:` line is replaced by `why: <reason>` |

A `read:` line is added for an owed attested mode gate with instructions, as for instruction gates. When every gate of the step is met the row reads `met: deliver:pr-comment [satisfied]` with `next: writfold kb plan advance`, as today.

`plan advance` refused: exit 3, nothing written, the row above and the existing note. Advanced: exit 0, the done record written with the entry of 4.3.

`plan status` exit code and its `stale=` count follow today's rule; a done delivery step that no longer holds counts in `stale=` like any other.

### 6.2 `plan status --gates`

Under a judged step, for each delivery gate, in template order:

```text
  gate: deliver:pr-comment [failed] delivery 1/3 checks met @runs/run-…/deliveries/…/candidate.md:1 !failed
  candidate: 20261112T211045Z-ee94cc [blocked] pr-comment audience=external destination=harbor#42 candidate=bb69f10c8fd9 bytes=136 seq=2 @runs/run-…/deliveries/…/candidate.md:1 !blocked
  item: paragraphs.bounded [satisfied] mechanical pass for bb69f10c8fd9 (at most 150 words per paragraph) @runs/run-…/checks/….json:1
  item: references.denied [failed] mechanical fail for bb69f10c8fd9: line 3 internal-host (column 50) @runs/run-…/checks/….json:1 !failed
  item: tone.reviewed [open] attested needs: writfold kb deliver attest … !open
```

With no deciding candidate: `gate: deliver:pr-comment [open] delivery no candidate` and no further lines. On a later step: `gate: deliver:pr-comment [later]`. The `item` text is the `check:` text of `deliver status`. `current_gates` and `current_owed` count delivery gates.

### 6.3 `gate status` and `gate lines`

Rows, counts and exit codes are unchanged: deliveries are not listed. When the run's plan names any delivery, `gate status` adds one note:

`the plan also owes deliver:pr-comment at change#1/deliver; a delivery is judged at its step: writfold kb plan status`

When the plan names deliveries and no gate, the dev10 note `no gate is owed: the run's plan names none (N defined)` reads instead `no gate is owed here: the run's plan names only deliveries (deliver:pr-comment), judged at their steps: writfold kb plan status`; `reason=no-gates` and exit 2 stay. `gate attest`, `gate waive`, `gate run` and `gate cover` given `deliver:<mode>` are refused (exit 3): `deliver:<mode> is a delivery, not a gate: use writfold kb deliver check|attest|waive`.

### 6.4 Guard requirements

- `step.met`: judges the current step's deliveries with its gates. Owed text: `step deliver owes deliver:pr-comment [open]; next: writfold kb deliver prepare --mode pr-comment --from FILE`.
- `gates.met`: deliveries are neither owed nor counted. With a plan that names deliveries and no gate, the owed set is empty and the dev10 rule refuses; the text is `no gate is owed here: the run's plan names only deliveries; gates.met is not met by an empty set; next: require step.met for the delivery step, name a gate in a plan step, or leave this repository out of the rule with repos`.
- `gate:<id>` and the delivery guard: unchanged, apart from section 5.

The guide gives the order for a posting step under the delivery guard: prepare, check, post (the guard passes the ready candidate's bytes), `plan advance`. A team that wants "it was posted" on record adds an attested repository gate to the same step (`"deliver: deliver:pr-comment posted"`); it must not be an `attest` name of the mode, because the guard requires the candidate ready before the post.

### 6.5 `deliver status`

A candidate that a step is judged on gains one line; candidates no step looks at are unchanged, so the existing goldens hold.

```text
  step: decides deliver:pr-comment at change#1/deliver [current]
  step: bound to change#1/deliver [done]
```

### 6.6 `kb check`, `resume`, hooks, `runs --plan`, `timeline`

- `kb check`: no new row. The `plan` row is `plan status`, so it carries a stale done delivery; the `gates` row is `gate status`, unchanged.
- `resume`: the plan line reads `plan change#1/deliver [current] step 2/2 owes: deliver:pr-comment [open]` with its `next:`; the delivery rows are as today plus 6.5.
- Session-start and compaction hooks: the plan position is cut from `plan status`, so it shows the current step with its `candidate:`, `read:` and `next:` lines and each `!stale` done step. No hook code changes; one new golden.
- `runs --plan` and `timeline`: unchanged (they fold records and judge nothing).

### 6.7 `writfold watch` (data only)

Watch reads `plan status --gates --json`. Per delivery gate, as `more` entries of the step row, in this order:

| `label` | `status` | `text` | `path`, `line` | `flags` |
|---|---|---|---|---|
| `gate` | the standing, or `later` | `deliver:<mode> [<standing>] delivery <met>/<total> checks met`, or `… delivery no candidate`, or `deliver:<mode> [later]` | the candidate payload, 1; absent with no candidate | the standing when not met |
| `candidate` | `ready` or `blocked` | `<id> [<status>] <mode> audience=<a> destination=<d> candidate=<digest12> bytes=<n> seq=<n>` | the candidate payload, 1 | `blocked`, `edited` |
| `item` | the obligation's standing | `<name> [<standing>] <class> <detail>` | the receipt or statement, 1; absent when none | the standing when not `satisfied`; `stale` when cited evidence moved |

A gate whose ID starts `deliver:` is a delivery; its `item` entries are its checks. The current view shows them as it shows checklist items; it ignores `candidate`.

## 7. JSON

Additive; `writfold_kb` stays 1.

- `plan status --gates`: the `candidate` label; `item` entries under a delivery gate; the gate text forms of 6.7.
- `plan status`, `plan advance`, `resume`, hook text: the `candidate` label on the current step.
- `deliver status` (and every verb that prints its rows): `status` on each `check` entry (the obligation's standing); the `step` label.
- `delivery.json`: `seq`.
- Done records: a `met` entry whose `gate` is `deliver:<mode>`.

`docs/guide/json.md` gains a row or sentence for each; `tests/json.rs` pins them. No existing text golden changes. New goldens are listed in section 10.

## 8. Refusals and exit codes

| Case | Exit | Written |
|---|---|---|
| Template word malformed; unknown mode; mode does not load; mode declares nothing; candidate gate named directly | 3 | nothing |
| `plan advance`, delivery owed | 3, with the step row | nothing |
| `plan advance`, delivery `satisfied`, `attested` or `waived` and every other gate met | 0 | the done record |
| `gate run|attest|waive|cover deliver:<mode>` | 3 | nothing |
| `plan status` with a done delivery step no longer holding | as today for `!stale` | nothing |
| `deliver status` with a candidate whose cited evidence moved | 2 (`blocked`) | nothing |

The advance re-check under the KB write lock is unchanged (the step is still current). A candidate edited between the judgement and the write is caught on the next read: the done record holds the digest judged, and the step reads `!stale`.

## 9. Kernel and proofs

**Proved kernels: no change, nothing to re-prove.** `writfold_core::plan` (wholly in the Charon `start_from` list; `proofs/Writfold/Plan.lean`) is not edited: `advance_check(nodes, at, met)` and `stale(node, met)` receive one more `bool` in `met`, computed by the proved `gate::Standing::met` from the delivery's standing. `gate::state` and `gate::standing` (`proofs/Writfold/Gate.lean`) judge each obligation exactly as `deliver status` already calls them; the citation re-check of section 5 passes `State::Stale` to `gate::standing`, an input the theorems already quantify over (`standing_spec` and `standing_satisfied_iff` hold for every state and every statement summary). `proofs/gate.sh` must pass with `Extracted/WritfoldCore.lean` byte-identical; the implementer confirms that with a diff.

**New pure module, `writfold_core::delivery`** (no IO, `alloc` only, **not** added to `start_from`):

```rust
/// The candidate that decides an open step: the last unbound one.
/// `bound[i]` is whether candidate `i` (oldest first) is named by a done record.
pub fn deciding(bound: &[bool]) -> Option<usize>;

/// One standing for a delivery from its obligations', in the mode's order.
/// Empty: `Checked(Open)`. Else the first unmet obligation's standing; else
/// `Waived` if any is waived; else `Attested` if any is attested; else
/// `Checked(Satisfied)`.
pub fn standing(obligations: &[gate::Standing]) -> gate::Standing;
```

Exhaustive tests: every `bound` of up to four; every obligation list of up to three over the seven standings. Properties asserted: `standing` is `Checked(Satisfied)` only when the list is non-empty and every element is; `standing(..).met()` exactly when the list is non-empty and every element is met; it is never `Checked(Satisfied)` when any element is `Attested` or `Waived`.

Nothing must go in `writfold_core::plan` (everything there is translated). If a proof is wanted later, it is: add `writfold_core::delivery` to `start_from`, write `proofs/Writfold/Delivery.lean` with those three properties and `deciding_last_unbound`, and add its mutants to `proofs/mutants.py`. Not recommended now: the input space is small and the exhaustive tests cover it.

No protocol changes: no new write path, lock or crash point, so no TLC model.

## 10. Tests, with the mutation each one catches

New fixture `fixtures/kb/deliver-step/`: template `change` (`"implement:"`, `"deliver: deliver:pr-comment"`), template `two` (two steps naming `deliver:pr-comment`), a mode `note` with one `attest` name, an empty mode `empty`, a deny list for `external`. New test file `crates/writfold-kb/tests/plan_deliver.rs` (and `plan_deliver_faults.rs` for refusals); no `#[ignore]`.

| Test | Removing this makes it fail |
|---|---|
| `step("d: deliver:pr-comment unit use=r")` parses; `deliver:`, `deliver:Bad`, `deliver:guard`, a mode twice are refused | the mode-name check; the duplicate check |
| `plan set` with `deliver:nope`: exit 3, the text of section 3, no file under `plan/` | the mode lookup in the splice check |
| The same through a `use=` include and through a required end template | the check over the closure; over required templates |
| `plan set` naming `deliver:empty`: exit 3 | the empty-mode refusal |
| No candidate: `plan advance` exit 3, golden `plan-advance-deliver-refused.txt`, nothing written | the delivery branch in the judge (the gate would read `undefined`) |
| Candidate prepared, not checked: gate `open`, `next:` is `deliver check <id>` | the standing from the first unmet obligation; the `next:` table |
| Candidate failing `references.denied`: gate `failed`, the `candidate:` line names both owed obligations | the `candidate:` line |
| Candidate ready by checks only: advance exits 0; the done record's entry has `standing: satisfied`, the payload path, the content digest | the done entry's fields |
| One check waived: advance exits 0, `standing: waived`, the row shows `[waived]` | `Waived` before `Satisfied` in `delivery::standing` |
| Mode `note` (one attested name) attested: `[attested]`, never `[satisfied]` | `Attested` before `Satisfied` |
| After advance, edit `candidate.md`: the step is `[done] … [stale] !stale`, `stale=1`, the current step unchanged | judging a done step at all; judging it on present bytes |
| After advance, change `paragraph_words` in the mode file: `!stale`; restore: holds again | the obligation digest in the judgement |
| After advance, prepare a newer, unchecked candidate of the mode: the done step still holds | judging a done step on its named candidate, not the newest |
| Template `two`: after the first step is done, the second is `open` although the first candidate is ready; a second candidate meets it | the bound exclusion in `deciding` |
| Two candidates of the mode, the newer blocked and the older ready: owed | "last" in `deciding` |
| Two candidates with `seq` 1 and 2 whose IDs sort the other way: the `seq` 2 one decides; a manifest without `seq` sorts first | ordering by `seq` before ID; `seq` defaulting to 0 |
| A candidate of another mode, ready: the step stays `open` | the mode filter |
| A candidate with an invalid manifest, alone: `open` | skipping invalid manifests |
| Candidate citing a record, checked, then the record edited: `deliver status` `[blocked]`, the check line says `cited evidence moved`, the step is owed; reverting the record restores both | the re-check of section 5 |
| The same with the check waived: still `[waived]`, still ready | passing statements to `gate::standing` on the re-check |
| The delivery guard on that candidate's bytes: denied while moved | the re-check in `readiness` |
| `gate status` on the `change` plan: no delivery row, the note of 6.3, exit as before | excluding deliveries from the owed gate set (the gate would count as undefined) |
| A `gates.met` rule on a plan naming only a delivery: refused with the text of 6.4; with one met gate beside it: passes while the delivery is open | the same exclusion in `require`; the text branch |
| A `step.met` rule with the delivery step current and open: denies with the `deliver prepare` command; passes once ready | deliveries in the step judgement |
| `gate waive deliver:pr-comment`: exit 3 | the refusal |
| `plan status --gates --json`, golden `plan-status-gates-deliver.json`: the `gate`, `candidate`, `item` entries with `status`; a later step's `[later]` | the detail projection; `status` on entries |
| `deliver status` after advance: the `step: bound to …` line; before: `step: decides …` | the `step` line |
| Session-start hook golden `hook-session-start-deliver.txt` | (pins the cut; no code) |
| `tests/json.rs` reads the new rows of `json.md` | a key added without a doc row |
| Existing goldens: `plan-*.txt`, `deliver-*.txt`, `hook-session-start-plan.txt`, `plan-status-gates.{txt,json}` byte-identical | (guards against over-reach) |
| Core: the exhaustive tests of section 9 | each branch of `standing`; `!bound` in `deciding` |

The one existing assertion that changes is `tests/deliver_modes.rs:283`, which keeps matching `judges delivery candidates`.

## 11. Docs

- `docs/guide/run-graph.md`: the status line (`delivery modes as a step's gates are built in <the next release>`); the "Planned" section becomes "A delivery as a step's gate" with decisions 2, 4, 7, 8, the worked output of 6.1 and the limit that a done record says ready, not posted; the sentence "A delivery-candidate gate cannot be a step's gate" is rewritten.
- `docs/guide/delivery-modes.md`: "On a plan step" (which candidate decides, binding, `seq`), "Cited evidence that moved" (section 5 and its limits), the posting order under the guard.
- `docs/guide/gates.md`: "Which gates a run owes" (deliveries are owed at their step, not listed by `gate status`).
- `docs/guide/hooks.md`: the requirement table (`step.met` judges deliveries; `gates.met` does not, with the reason).
- `docs/guide/json.md`: section 7. `docs/guide/watch.md`: one sentence on delivery checks under a gate, only once exercised. `docs/guide/README.md` and `validation.md`: status cells.
- `docs/writfold/kb-v1-design.md`: a new as-built section. `docs/writfold/mission.md`: the order-of-work item struck through with the link.
- Release note: the three behaviour changes (a step may owe a delivery; cited evidence that moved blocks a candidate; `delivery.json` gains `seq`). Website copy only for what was exercised.

## 12. Build plan

Module budgets: every new module at most 250 lines, functions at most 80, tests included; no `unwrap`/`expect`; no new crate. `plan/judge.rs`, `deliver/mod.rs` and `deliver/status.rs` are near their size, so the new code goes in new modules.

| # | Step | Days |
|---|---|---|
| 1 | `writfold_core::delivery` with its exhaustive tests; confirm the Lean extraction is byte-identical | 0.25 |
| 2 | `deliver`: `seq` on the manifest and `(seq, id)` order; new `deliver/step.rs` (candidates of a mode, bound set, the deciding candidate, the judged view with the aggregate standing and definition digest) | 0.5 |
| 3 | Section 5: `deliver/recheck.rs`; the session threaded into `Tally::add` and `readiness`; `status` on `check` entries | 0.5 |
| 4 | Template word and its refusals; the splice-time mode check over the closure and required templates; `Graph::gates()` without deliveries and `Graph::deliveries()` | 0.5 |
| 5 | New `plan/delivery.rs`: the judge branch for done and current steps, `judge::Gate` filled (its `cover` field renamed `command`), the `candidate:` line, `next:`; done-record entry | 0.75 |
| 6 | `plan status --gates` projection; the `deliver status` `step:` line | 0.5 |
| 7 | `gate status` note and refusals; `gates.met` text; rebase on the dev10 texts | 0.25 |
| 8 | Fixture, goldens, fault tests, `json.rs` rows | 0.75 |
| 9 | Docs and the as-built section | 0.5 |

About 4.5 days. Cut line after step 5 plus its tests and docs: steps 6 (the `step:` line only) and the `candidate` entry of `--gates` can follow in a later release without changing any decision. Release steps as usual: `scripts/gate quality` and `scripts/gate unit` from main, receipt paths in the commit message.

## 13. Not built

- Sending, or any record that a candidate was posted. An attested gate on the step is the way to state it.
- `deliver prepare --step`, a step window for candidates, withdrawing a candidate.
- Judging a delivery-only step without a subject repository.
- A `deliver:<mode>` guard requirement; deliveries in `gate status`, `gate lines` or `gates.met`.
- Re-running `observed` mode gates on read.
- A Lean proof of `writfold_core::delivery`.

## 14. Open question

1. **Section 5 changes `deliver status` and the delivery guard, not only the plan.** Recommendation: yes, build it for every reader. The alternative, re-checking only at a plan step, leaves `deliver status` saying `[ready]` where `plan status` says `[stale]` for the same candidate.

## Decisions (owner, 15 November 2026)

1. **Re-checking `claims.cited-fresh` on every read: yes.** A candidate whose cited evidence moved reads `[blocked]` in `deliver status`, the delivery guard and `plan status` alike; the release note names the change.
2. **Ships in the same release as the v0.5.0-dev.10 round**, built after that round merges, on top of it.

> Shipped in v0.6.0-dev.1 (16 November 2026), which also removed the `trial`, `serve` and `tuft` subcommands from the binary.
