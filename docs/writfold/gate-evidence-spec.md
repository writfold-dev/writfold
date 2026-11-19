# Gate evidence: binding and pins

> **Status:** specification, 18 November 2026, against `main` at `c2de2a7` (v0.6.0-dev.8). Stages 1 (kernels and proofs), 2 (report-only notes), 3 (`own_evidence`, `evidence_from`) and 4 (negative looks age; `--searched`) are built; stages 5 and 6 are not. The authority is [mission.md](mission.md). The rule is AGENTS.md's "Lean proofs first": every decision below is a pure kernel in `crates/writfold-core`, proved in Lean, before any verb calls it.

## 0. Why

Outside feedback (a field report from the owner's earlier prototype, checked against the code at `c2de2a7`) found two separate problems.

| Question | Answer at `c2de2a7` |
|---|---|
| Does a run owe the right set of gates? | Yes. A run owes its plan's gates, and `required.toml` start and end templates are spliced into every plan ([run-graph](../guide/run-graph.md)). Nothing forces an agent through them, but the plan is not complete until each one is done or skipped. Only a guard rule blocks anything. |
| Once owed, is the evidence for an item about that item? | No. `observe::grounds(status, cites_fresh)` takes no item (`crates/writfold-core/src/observe.rs:82`). One `ok` look grounds any item it is cited for, and `--for` only affects display. [policy.md](../guide/policy.md) documents relevance as the coverer's judgement. |
| Does a negative look age? | No. A `no-data` or `not-found` look embeds no citation, `looks::moved` compares only its status, and the recorded `revision` is never read. |
| Does observed outside state age? | Observations do, through their kind's lifetime in the catalog (`.writfold/sources/KIND.toml`). Gate receipts do not, and an undeclared kind without `ttl=` never ages. |
| Can a receipt stale when another repository moves? | Citations already cross repositories through attached stores. Receipts and rows are about one subject repository. |

Neither binding nor pins alone would have caught the miss that prompted the feedback: four ADRs in another repository stayed the same while the code changed. That takes an executed sweep, run as a mechanical gate, whose receipt stales when either repository moves. This spec gives a team the means to require one (stages 3 and 6). It ships no such gate (owner decision).

## 1. The model

A cover row stands on (item, evidence, present). Two concepts cover the gaps.

**Binding.** An item may declare what answers it. Each cited reference relates to the item in one of four ways:

| `Bound` | Meaning |
|---|---|
| `Named` | the item names it: a receipt or standing of a gate in the item's `evidence_from` entry |
| `This` | it names this item: a look recorded `--for` this item |
| `Other` | it names another item: a look recorded `--for` another item |
| `Unbound` | neither: a citation token, a retrieval, a look without `--for`, a gate the item does not name |

| `Need` | Declared as | Answered by |
|---|---|---|
| `Any` | no key (default, today's behaviour) | anything |
| `Own` | the item in `own_evidence = [...]` | `This`, `Named` |
| `Named` | the item in `evidence_from = ["item: gate …"]` only | `Named` |

An item is *answered* when some cited reference is both grounded and answers it. A `supported` or `none-found` row that is not answered stands `stale`, as a row whose evidence moved does. Binding only narrows the standing. Only `Named` is checked from both sides. `--for` is the recorder's own word, and it is enough to catch mistakes ([gates](../guide/gates.md): mistakes, not adversaries). Under `Own`, a look must name the item explicitly; a look without `--for` does not count (owner default).

**Pins.** A piece of evidence carries zero or more pins, each `Held`, `Moved`, `Aged` or `Unreadable`. It holds while every pin holds. No pins means it holds, which is today's behaviour. A pin that cannot be read never holds. There are three sources of pins:

- **What a negative look searched.** An optional `searched` citation token on a `no-data` or `not-found` observation, judged like any citation. If what it names moved, the look no longer reads as finding nothing.
- **Time, from the kind catalog.** This is the owner's decision: things that expire by time expire through the catalog and are reported `stale`. A negative look ages by its `--source` kind's lifetime. An observed-mode gate may name a kind (`ages_as = "<kind>"`), and its receipts age by that kind's lifetime. The arithmetic is `verdict::lifetime` and `verdict::expired`, the same rule citations already age by. `recorded` is compared for expiry only, never for ordering.
- **Another repository's scope**, through an attached store (`repo:<store>/<glob>` scope entries). A store that is missing or unresolved is `Unreadable`, so the gate is open.

## 2. Theorems (stage 1, built)

`proofs/Writfold/Bind.lean` and `proofs/Writfold/Pin.lean`, over the translation of `crates/writfold-core/src/{bind,pin}.rs`. All are total-correctness triples.

| Theorem | Statement |
|---|---|
| `answers_spec` | `answers` is the table above |
| `answers_any` | under `Any` every reference answers |
| `other_never_answers` | `need ≠ Any → answers need Other = false` |
| `unbound_never_answers` | `need ≠ Any → answers need Unbound = false` |
| `named_need_iff` | under `Named`, answers ↔ `Named` |
| `answered_iff` | answered ↔ ∃ cited reference, grounded ∧ answers |
| `answeredB_skip`, `answered_unrelated` | **monotonicity:** a reference that does not ground or does not answer never changes whether the item is answered, wherever it is cited |
| `item_under_spec` | `item_under r a` = `checklist.item r`, with supported or none-found turned stale when unanswered |
| `item_under_answered` | **refinement:** answered, `item_under` is `checklist.item` exactly |
| `item_under_supported_iff`, `item_under_none_found_iff` | supported (none-found) ↔ `checklist.item` says so ∧ answered |
| `item_under_narrows` | whatever `item_under` covers under a policy, `checklist.item` covered already |
| `item_under_unanswered` | unanswered: supported or none-found → stale; every other standing is unchanged |
| `bound_supported_iff` | **relevance soundness:** supported ↔ the newest row is current, supported, cites ≥ 1 reference, all grounded ∧ some cited reference is grounded and answers the item |
| `holds_spec`, `holds_nil`, `unheld_never_holds` | holds ↔ every pin `Held`; no pins hold; one moved, aged or unreadable pin is enough to stop it holding |
| `timed_spec`, `timed_aged_iff`, `timed_untimed`, `timed_never_moved` | a token or catalog lifetime ages ↔ `now − seen > ttl` over ℤ; immutable or no lifetime never ages; time only ages |
| `nothing_found_spec`, `nothing_found_refines` | a look still found nothing ↔ (`no-data` ∨ `not-found`) ∧ every pin holds; with no pins this is today's status test |
| `current_under_spec`, `current_under_about`, `current_under_refines` | current ↔ about the present bytes ∧ every pin holds; pins only take standing away; no pins = about |

There are eight mutants, one per decision, each calibrated against the theorems it must break ([aeneas.md](aeneas.md#the-claims-and-their-mutants)).

The existing kernels are untouched: `observe::grounds`, `ground::of_*` (they also serve the frozen assessed gate), `checklist::item` and `gate::about`. The new kernels sit on top of them, and each one has a refinement theorem showing that the default reproduces today's behaviour.

## 3. Kernels

```rust
// crates/writfold-core/src/bind.rs
pub enum Bound { Named, This, Other, Unbound }
pub enum Need { Any, Own, Named }
pub struct Ref { pub grounded: bool, pub bound: Bound }
pub fn answers(need: Need, bound: Bound) -> bool;
pub fn answered(need: Need, refs: &[Ref]) -> bool;           // via answered_from
pub fn item_under(newest: Option<checklist::Row>, answered: bool) -> checklist::Item;

// crates/writfold-core/src/pin.rs
pub enum Pin { Held, Moved, Aged, Unreadable }
pub fn holds(pins: &[Pin]) -> bool;                          // via holds_from
pub fn timed(life: verdict::Lifetime<i64>, seen: i64, now: i64) -> Pin;
pub fn nothing_found(status: observe::Status, pins: &[Pin]) -> bool;
pub fn current_under(about: bool, pins: &[Pin]) -> bool;
```

**Tested, not proved:** the TOML keys `own_evidence`, `evidence_from` and `ages_as` and their diagnostics; parsing the `for` and `searched` fields; classifying a reference into a `Bound`; resolving stores and computing other repositories' scope digests (`stale/repos.rs`); clock reads and turning a duration into seconds; rendering `why:` lines and notes about shared references.

**TLC:** no change. Each decision is made per read, and `cover` and `observe` remain single create-new writes. `RunGraph.tla` treats a gate as met when a passing receipt is about the present bytes, and pins only narrow "about".

## 4. Stages

Each stage ships on its own. Stages 2 to 6 call only the kernels proved in stage 1. A stage that needs a new decision adds a kernel and its proofs first.

1. **Kernels and proofs.** Built: `bind.rs`, `pin.rs`, `Bind.lean`, `Pin.lean`, the audit claims and eight mutants.
2. **Report only.** Built (`gate/checklist/bound.rs`; [gates](../guide/gates.md#which-item-the-evidence-was-for-built-in-v060-dev9)). `gate status` and `gate cover` add notes: `!other-item` when a cited look's `for` names another item, and a count when one reference grounds several items. No standing changes. No refusal on a count: a count is shown, never decided on.
3. **Per-item evidence.** Built as two checklist keys in the list style of `not_applicable`: `own_evidence = [items]` (`Need::Own`) and `evidence_from = ["item: gate …"]` (`Need::Named`, or `Own` when the item is in both); `gate/checklist/evidence.rs`, classification in `bound.rs`. The keys join the definition digest only when written, as `not_applicable` and `none_found` do: a checklist without them keeps its digest and its rows; adding one makes that checklist's rows stale. `gate cover` refuses a `supported` or `none-found` row that `item_under` would not stand, and the read path reports it `stale` with the reason.
4. **Negative looks age.** Built (`gate/pins.rs`, the `negative` arm of `gate/ground.rs`; standings `aged`, `searched-moved`, `unreadable`). Not built: the `kb lint` warning for negative looks of undeclared kinds. A look's `--source` kind lifetime applies to `no-data` and `not-found` through `timed`. The optional `searched` token on `writfold-observation/1` is an additive field, so an older binary ignores it and reads the row as standing. It is judged like any citation, resolved through the store map, and never fetched. `kb lint` warns about a negative look of an undeclared kind, which never ages.
5. **Receipt lifetime.** `ages_as = "<kind>"` on an observed-mode gate. The receipt's time pin comes from `timed` with that kind's catalog lifetime, and `current_under` decides. An aged receipt is `stale`.
6. **`repo:<store>/<glob>` scope entries.** The caller folds each store's scope digest into the content digest the receipt already records. An unresolved store is `Unreadable`.

## 5. Not doing

- Judging whether evidence is semantically relevant or whether the conclusion follows. [policy.md](../guide/policy.md) still says this.
- A threshold for shared references that refuses anything.
- Pattern matching over notes ("mentions a grep").
- Independent-origins counting.
- Executing or checking what an observation says was run. A team that wants execution uses a mechanical gate and names it in the item's `evidence_from`.
- Fetching, cloning or authenticated approval.
- Receipts about more than one subject, or any change to the proved `gate::about`.
- An example docs-sync sweep gate (owner, 18 November 2026).
- **Deferred: owed by diff (`applies_when`).** The owner is undecided. It would change which gates are owed, so it needs a `RunGraph.tla` change and a mutant in which a gate that is not owed reads as met. A wrong path list would silently stop a gate from being owed. Until then, a template choice or a reasoned `not-applicable` row covers it.

## 6. Owner decisions (18 November 2026)

| Question | Decision |
|---|---|
| Ship an example docs-sync gate? | No. |
| Proofs before wiring? | Yes: stage 1 is kernels and proofs only. |
| Expire things by time? | Yes, through the kind catalog, reported `stale`. |
| Owed by diff (B5)? | Undecided; deferred. |
| Does `Own` require an explicit `--for`? | Default taken: yes. |
| Add an optional `searched` field to `writfold-observation/1`? | Default taken: yes, additive. To revisit before stage 4. |
| Negative looks of undeclared kinds? | Default taken: keep them (compatible), and `kb lint` warns. |
