---
rfc: "2026-11-27-calculus-in-lean"
title: "The calculus in Lean: a formal core, every kernel shown to implement it, and a law-coverage check"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/204
milestone: "v0.6.0-dev.26"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: The calculus in Lean

Foundation for the proofs. It states
[the calculus](2026-11-25-agent-vocabulary/calculus.md), the three laws of
[referents, remainders and sites](2026-11-27-referents-remainders-sites.md)
(R, U, S) and law A of [the asset boundary](2026-11-27-asset-boundary.md)
as Lean definitions and theorems, and ties each translated kernel to them.
It changes no product behaviour.

## Summary

The owner asked for the calculus to be put on a formal footing so that
later work builds on something checked. Today the proofs are per kernel:
732 theorems in 40 files, each about one translated Rust module. The
calculus that says what those kernels have in common is prose. A law such
as "unknown never admits" is proved again in each kernel that remembers
it, and a new kernel can break it without any proof failing.

This RFC adds three things under `proofs/`:

1. **A core**, `proofs/Calculus/`: about 700 lines of plain Lean with no
   import from the translation. It defines the calculus's objects (the
   three-valued word, decisions, an accounted universe, evidence classes
   and policy, the deciding record, referents and their resolution, sites
   and postures) and proves its laws once.
2. **A conformance layer**: for each translated kernel function, a *link
   theorem*, a Hoare triple over the shipped Rust whose postcondition is a
   core operation. A law of the core then holds of the kernel by one
   `rw`. Link theorems are derived from the spec theorems that exist; no
   existing proof is rewritten.
3. **A law-coverage check** in the proofs gate: every translated function
   is linked, declared out with a reason, or on a backlog that only
   shrinks. A function that returns a judgment may not be declared out.
   The check prints its own remainder, which is law U applied to the
   proofs.

What does not change: the proofs stay about the shipped Rust
(the owner decision of 9 November 2026). The core is a specification that
kernels are proved against, not a second implementation. Parsers, IO and
rendering stay tested. Protocol properties stay in TLA+.

A prototype of the whole core and of seven links compiles against
today's translation (below, and under
[`2026-11-27-calculus-in-lean/prototype/`](2026-11-27-calculus-in-lean/prototype)).

## Motivation: the audit

Read at `0a0bca86`. Counts are from `proofs/Writfold/*.lean`,
`proofs/Audit.lean`, `proofs/mutants.py` and the built environment.

| | Count |
|---|---|
| Theorems in `proofs/Writfold/` | 732 in 40 files |
| Required claims in `Audit.lean` | 621 |
| Kernel mutants | 354 |
| tauten claims | 195, with 140 required theorems still unclaimed |
| Translated functions, loops excluded | 247 (116 return `bool`) |
| Enums in the translated types | 85 |
| Lean definitions or theorems about the calculus as such | 0 |

### Which laws are theorems

Every theorem `calculus.md` marks **shipped** exists under the name it
gives. All but one are required claims.

| Law | State | Where |
|---|---|---|
| 3.1 binding | proved, per kernel | `Gate`, `Current`, `Invoke`, `Pin` |
| 3.2 one record decides | proved, per kernel | `Gate`, `Checklist`, `Delivery`, `Order`, `Handoff`, `Recorded` |
| 3.3 a class is never promoted | proved, per kernel: seven theorems in five files | `Gate`, `Checklist`, `Delivery`, `Ground`, `Handoff` |
| 3.4 policy narrows | `met_narrows` and `accept_only_narrows` proved. **The meet law itself, `narrow_met_iff`, is prose**: the calculus says the first kernel to combine two policies owes it | `Policy`, `Delivery` |
| 3.5 unknown and empty | proved, per kernel: 17 named theorems in ten files | `Guard`, `Gate`, `Invoke`, `Inventory`, `Policy`, `Delivery`, `Handoff`, `Packet`, `Walk`, `Units` |
| 3.6 tool coverage | proved for one grant source. **The intersection rule is prose** | `Compose` |
| 3.7 order | splices proved. **The plan fold and instruction order are tested** (#17 was declined) | `Plan` |
| 3.8 sites; judge again | each site's refusal on unknown is proved. **"Judged again" is prose** and a TLC invariant. `stale_never_current` is proved and is not a required claim, so it could be deleted without the audit failing | `Guard`, `Current`, `Plan`, `Handoff`, `Invoke` |
| 3.9 generated universe | proved | `Checklist`, `Bind`, `Units`, `Walk`, `Ground` |
| 3.10 specialization | **prose**: a definition, no theorem, no consumer | — |
| 3.11 law R, referent | **prose**. Instances proved one at a time: `subject::pick`, `pointer`, `store::claim`, `absence_never_evidence` | `Subject`, `Pointer`, `Store`, `Verdict` |
| 3.12 law U, remainder | **prose**. Instances proved one at a time; the `account` kernel is specified and not built (#181) | `Inventory`, `Runs`, `Walk`, `Packet`, `Units`, `Catalog` |
| 3.13 law S, site | **prose**. No kernel yet (#184, #186) | — |
| Law A, `judge` | **prose**. Kernel specified and not built (#196) | — |
| The five layers; the trust tiers (recorded, guarded, received) | **prose** | — |

No law is false of a shipped kernel as far as this audit found. The
field-feedback review reached the same conclusion from the other side.

### Where kernels say the same thing differently

- **90 theorems in 27 files** have `never`, `unknown`, `empty`,
  `incomplete`, `truncated`, `partial`, `unread`, `absent`, `ambiguous`,
  `refines` or `narrows` in their name. Each is a law instance proved by
  unfolding one kernel.
- **20 of the 85 enums** carry their own word for "could not be told":
  `Unknown` (`guard::Fact`, `gate::Outcome`, `gate::State`,
  `invoke::ForItem`, `delivery::Body`, `investigation::Hypothesis`,
  `store::Claim`), `Unreadable` (`inventory::Root`, `pin::Pin`,
  `units::Absent`), `Unverifiable` (`verdict::Verdict`, `verdict::Said`),
  `Unsearched`, `Capped`, `Unresolved`, `Partial`, `Incomplete`,
  `Undefined`, `Ambiguous`. `guard::Fact` and `gate::Outcome` are the same
  three words under two sets of names.
- **Three enums carry the evidence class**: `gate::Standing`,
  `policy::How`, `grounding::Grounding`.
- **Four kernels define `complete`** (`walk`, `packet`, `runs`,
  `inventory`), each with its own "never complete" lemma. Two define
  `all_met`, two `first_unmet`, two `first_owed`, three `met`.
- **28 theorem names occur in more than one file**: `met_spec` and
  `standing_spec` three times each, `complete_iff`, `state_spec`,
  `all_met_spec`, `item_spec`, `standing_satisfied_iff` twice each.
- **Three refinement theorems are one statement**: `history_only_refines`
  (#90), `fetch_only_refines` (#182, specified) and `judge_refines`
  (#196, specified) each say that knowing more changes a verdict only
  where it was `unverifiable`.

### What that costs

The focused RFCs of 27 November each list their own "never passes"
theorems for their own new enums: `bare_never_passes`,
`empty_never_passes`, `unread_never_complete`, `absence_needs_whole`,
`unreported_never_quiet`, `ambiguous_never_judged`,
`unfetched_never_decides`, `empty_never_matches`. That is the per-kernel
habit continuing. Nothing checks that the list is complete for a kernel,
and nothing fails when a kernel arrives without one.

## Proof obligations

- **Referent and resolver:** none; this RFC decides nothing at run time.
  **Universe:** the translated functions (247 today), each linked,
  declared out or on the backlog, with the remainder printed by the gate.
  **Site:** the proofs gate.
- **Kernel:** none in `crates/writfold-core`. The new code is Lean.

### Core theorems (`proofs/Calculus/`)

Names marked † are not in the prototype yet. Every other one compiles.

**Words** (`Word.lean`; 3.5, L3, L4)

- `unknown_never_admits`, `admits_iff`: only `yes` admits.
- `le_refl`, `le_trans`, `le_antisymm`; `refines_only_unknown`: in the
  knowledge order, knowing more changes an answer only where it was
  `unknown`.
- `and_comm`, `and_assoc`, `and_idem`, `and_yes`, `and_mono`,
  `and_eq_yes`, `and_eq_no`.
- `all_eq_yes`: all hold exactly when each holds. `all_eq_no`: a refusal
  has a witness. `all_append`; `all_mono` †.
- `unknown_mem_never_admits`; `more_never_admits`: adding an obligation
  never makes an unmet set met.
- `join_comm`, `join_assoc`, `join_idem`, `pass_join`, `join_eq_pass`:
  a denial beats a question, which beats a pass.
- `admit_pass_iff`; `unknown_never_passes`: law 3.5 at an admission site.

**Accounting** (`Account.lean`; law U, 3.9)

- `remainder_append`, `whole_append`, `whole_iff_no_remainder`,
  `judged_never_counts`, `declared_out_never_counts`, `remainder_counts`,
  `whole_counts`.
- `universal_yes_iff`: a universal word is `yes` exactly when the
  universe is whole, every judged member is `yes`, and there is a member
  or the empty universe was allowed.
- `remainder_never_whole`, `empty_never_yes`, `universal_sound`,
  `universal_no_of_witness`.
- `search_yes_iff`; `absence_needs_whole`: a partial universe never
  proves that something is absent.

**Evidence** (`Evidence.lean`; 3.2 to 3.4)

- `class_never_promoted`, `satisfied_only_by_current_pass`.
- `met_inf`: `met s (a ⊓ b) = (met s a && met s b)`. This is
  `narrow_met_iff`. `met_narrows`, `reported_never_met`,
  `unknown_never_met`, `inf_comm`, `inf_assoc`, `inf_idem`, `inf_top`.
- `deciding_none_iff`, `uncounted_never_decides`; `deciding_mem` †,
  `deciding_is_newest` †, `older_never_outvotes` †.

**Sites** (`Site.lean`; law S, 3.8)

- `judged_unchanged`, `closed_pass_iff`, `posture_only_narrows`,
  `stricter_never_loosens`, `stricter_comm`, `stricter_assoc`,
  `stricter_open_left`; `open_is_declared` †.

**Referents** (`Referent.lean`; laws R and A)

- `resolve_nil`; `resolve_one_mem`: what was chosen was offered at the
  deciding level. `resolve_one_agree`: every candidate at that level
  agreed. `resolve_order_free`: two listings with the same members never
  resolve to different referents. `unresolved_unknown`.
- `settle_only_withholds`, `unresolved_never_judged`,
  `change_needs_refutable`, `settle_identity`, `unclaimed_survives`,
  `settle_refines`, `judge_needs`.

**Small definitions with their order laws, when a kernel first needs
them** (3.6, 3.10): `covers_inter_iff` †
(`R ⊆ A ∩ B ↔ R ⊆ A ∧ R ⊆ B`) with #159; the specialization preorder †
with its first consumer.

### Link theorems (`proofs/Writfold/Conform/`)

One per linked function, named `<function>_implements`, in the kernel's
namespace. The first seven compile in the prototype:

| Function | Implements | Laws it then has |
|---|---|---|
| `guard::decide` | `Calculus.admit` | `admit_pass_iff`, `unknown_never_passes` |
| `guard::combine` | `Decision.join` | `join_comm`, `join_assoc`, `pass_join` |
| `walk::complete_within`, `packet::complete`, `runs::complete` | `Calculus.whole` over counts | `whole_counts`, `remainder_never_whole` |
| `policy::met` | `Calculus.met` | `met_inf` (so `narrow_met_iff`), `met_narrows` |
| `pointer::open` | `Calculus.resolve` | `resolve_one_mem`, `resolve_one_agree` |

The rest are listed by step in the issues (#209 to #212): about 150
functions.

### Mutants

- **Core mutants** change a definition under `proofs/Calculus/` and must
  fail exactly the law theorems they name. They need no translation step.
  About 25 in all; each step's issue lists its own (for example
  `admits-unknown`, `whole-ignores-remainder`, `resolve-first-wins`,
  `settle-keeps-change-behind`, `undecided-passes-closed`). A law
  theorem that no core mutant names fails `mutants.py`.
- **Kernel mutants** are unchanged. A link derived from a spec theorem
  sits in a file that imports the spec's file, so a mutant that breaks
  the spec stops the build there and attributes no failure to the link.
  The 354 calibrate as before.
- **A new kernel written against the core** states its table theorem as
  the link, so its mutants name the link directly. That is how a mutant
  attaches to a law from now on.

### Tested and unproved

- `proofs/law_coverage.py` and the reader of `proofs/laws.toml`: one test
  per refusal (#206).
- Everything that is tested today stays tested: parsers, file and Git IO,
  digests, the plan fold, rendering, and how a caller turns what it read
  into the facts a kernel is given. The core does not reach them. See
  "What is not formalized".

No protocol changes. No TLA+ model.

## Design

### The core

Plain Lean. No file under `proofs/Calculus/` imports `Extracted`,
`Aeneas`, `Mathlib` or `Writfold`; a gate step refuses any other import.
It builds in two seconds. The definitions below are copied from the
prototype.

```lean
namespace Calculus

/-- What a judgment says: it holds, it does not, or it could not be told. -/
inductive Tri where
  | yes | no | unknown

/-- Only `yes` admits. -/
def Tri.admits : Tri → Bool
  | yes => true
  | _ => false

/-- The knowledge order: `unknown` is below both answers. -/
def Tri.leb : Tri → Tri → Bool
  | unknown, _ => true
  | yes, yes => true
  | no, no => true
  | _, _ => false

/-- Both must hold: a refusal wins, then a doubt. -/
def Tri.and : Tri → Tri → Tri
  | no, _ => no
  | _, no => no
  | unknown, _ => unknown
  | _, unknown => unknown
  | yes, yes => yes

def Tri.all (ts : List Tri) : Tri := ts.foldr Tri.and yes

/-- What a site answers. -/
inductive Decision where
  | pass | ask | deny

/-- L4: an operation passes only when every requirement holds. -/
def admit (action : Action) (reqs : List Tri) : Decision :=
  if (Tri.all reqs).admits then .pass else action.decision
```

```lean
/-- Every member of a universe is exactly one of these. -/
inductive Member where
  | judged | declaredOut | remainder

def remainder (ms : List Member) : Nat := ms.countP (· = .remainder)
def whole (ms : List Member) : Bool := remainder ms == 0

/-- A word that claims all of something. A judged member that fails is a
witness, whole or not. Short of that the word needs the whole universe,
and a member to speak of unless the empty universe was allowed. -/
def universal (allowEmpty : Bool) (ms : List (Member × Tri)) : Tri :=
  if .no ∈ judgedOf ms then .no
  else if !whole (ms.map (·.1)) then .unknown
  else if (judgedOf ms).isEmpty && !allowEmpty then .unknown
  else Tri.all (judgedOf ms)

/-- Law U as a property of any word said of an accounted universe: only
the `yes` row is constrained. -/
structure Universal (allowEmpty : Bool) (ms : List (Member × Tri)) (word : Tri) : Prop where
  accounted : word = .yes → whole (ms.map (·.1)) = true
  each : word = .yes → ∀ t ∈ judgedOf ms, t = .yes
  inhabited : word = .yes → allowEmpty = true ∨ judgedOf ms ≠ []

/-- The mirror: a word that claims some member exists. A witness is
enough in any universe; saying there is none needs all of it. -/
def search (isWhole found : Bool) : Tri :=
  if found then .yes else if isWhole then .no else .unknown
```

```lean
/-- How a fact came to be recorded. `reported` is what a harness said
happened (law S); no policy takes it. -/
inductive Class where
  | checked | attested | waived | reported

/-- A policy: the standings an obligation takes. There is no field for
`reported`, so no policy can take it. -/
structure Accept where
  satisfied : Bool
  attested : Bool
  waived : Bool

def met (s : Standing) (a : Accept) : Bool := …
def Accept.inf (a b : Accept) : Accept := …

/-- 3.2: of the records that count (bound, intact, of this subject), the
one with the greatest `seq` decides; a later one wins a tie. -/
def deciding (seq : α → Nat) (counts : α → Bool) (rs : List α) : Option α := …
```

```lean
/-- How far the site got with one event. -/
inductive Basis where
  | noRules | noMatch | judged (d : Decision) | undecided

/-- What an undecided event gets. `open` is a declared exemption from
"unknown never admits"; nothing else is. -/
inductive Posture where
  | open | ask | deny

def Site.decide (b : Basis) (p : Posture) : Decision :=
  match b with
  | .noRules | .noMatch => .pass
  | .judged d => d
  | .undecided => p.undecided
```

```lean
/-- The outcome of choosing a referent. -/
inductive Resolution (ρ : Type) where
  | one (r : ρ) | ambiguous | none

/-- One candidate the caller found: how strongly it names the asset, and
what it offers to judge against (`none`: it names the asset and offers
nothing). -/
structure Candidate (ρ : Type) where
  level : Nat
  referent : Option ρ

/-- The strongest level present decides, alone. Its candidates must all
offer the same referent. No fall-back to a weaker level, and no
first-listed winner. -/
def resolve [DecidableEq ρ] (cs : List (Candidate ρ)) : Resolution ρ := …

/-- A vocabulary of verdicts: the word for "could not be told", and which
words claim change or absence. -/
structure Words (W : Type) where
  unknown : W
  claimsChange : W → Bool
  unknown_claims_nothing : claimsChange unknown = false

/-- What a referent can support. -/
def settle (ws : Words W) (resolved refutable : Bool) (w : W) : W :=
  if !resolved then ws.unknown
  else if ws.claimsChange w && !refutable then ws.unknown
  else w

/-- Law A: a verdict on a cited asset is judge(resolved, relation, fact). -/
def judge (ws : Words W) (observe : Fact → W) (resolved refutable : Bool) (fact : Fact) : W :=
  settle ws resolved refutable (observe fact)
```

**Room for the asset model.** The core has no word of Git. The type of
referents `ρ`, the type of facts `Fact`, the function `observe` and the
vocabulary `W` are parameters. A provider, in the asset boundary's
sense, is whatever supplies `judge`'s three inputs. `judge` has no
argument through which a provider could supply a verdict, which is law
A's "never a verdict" by construction. `Relation::Behind` is the one
relation that is not `refutable`; a provider without history answers
`Unknown`, which is refutable, so `judge` is `observe` over a resolved
referent, as that RFC says. `resolve` is the shape of a resolver:
`target::choose`'s map `ref=` and a declaration (#197) are its top
levels, the inferred `Url`, `Path`, `Base` and `Sole` the levels below.
A resolver whose table has a row `resolve` lacks proves the two
properties (`resolve_one_mem`, `resolve_one_agree`) of its own table
instead.

### The conformance layer

For a translated function `k` and a core operation `op`, the link is

```lean
theorem k_implements (x : …) : k x ⦃ r => abs_out r = op (abs_in x) ⦄
```

where `abs_in` and `abs_out` are abstraction functions from the
**dictionary**, `proofs/Writfold/Conform/Words.lean`: one function per
Rust word type onto a core type.

```lean
def factTri : guard.Fact → Calculus.Tri
  | .Met => .yes
  | .Unmet => .no
  | .Unknown => .unknown

theorem decide_implements (a : guard.Action) (fs : Slice guard.Fact) :
    guard.decide true a fs ⦃ d =>
      decisionOf d = Calculus.admit (actionOf a) (fs.val.map factTri) ⦄ := by
  apply WP.spec_mono (Guard.decide_spec true a fs)
  …

/-- Law 3.5, transferred, not re-proved. -/
theorem unknown_never_passes (a : guard.Action) (fs : Slice guard.Fact)
    (h : guard.Fact.Unknown ∈ fs.val) :
    guard.decide true a fs ⦃ d => decisionOf d ≠ .pass ⦄ := by
  apply WP.spec_mono (decide_implements a fs)
  intro d hd; rw [hd]
  exact Calculus.unknown_never_passes _ (List.mem_map.mpr ⟨_, h, rfl⟩)
```

The shipped statement of `unknown_never_passes` is `d ≠ .Pass` over the
Rust enum. It follows from this one by `cases d`, so its name and
statement stay.

Three rules keep a link honest.

1. **Equality by default.** The postcondition is an equation with the
   core operation. A kernel whose table differs from the core's off the
   admitting row (it says `fail` where the core says `unknown`) proves the
   core's predicate form (`Universal`, or the two `resolve` properties)
   instead, and the register records which.
2. **Abstractions are onto.** Each dictionary function has a theorem
   that it reaches every word of its core type, or a named subset. A
   constant abstraction, which would make any link true, is not onto.
3. **The dictionary is small and read by a person.** It is one file. The
   gate prints it as a table (`Met ↦ yes`, `Unmet ↦ no`, …). It joins the
   trust base: a word wrongly mapped to `yes` would weaken a law without
   failing a proof.

Links live in `proofs/Writfold/Conform/<Kernel>.lean`, in the kernel's
namespace (`Writfold.Core.Guard.decide_implements`), so `Audit.lean`,
`Total.lean` and the mutant attribution see them with no new rule. A
kernel written after step 2 puts its link in its own theorem file.

Where a kernel is given counts and the core speaks of lists, the link
goes through `Member.counts`:

```lean
theorem complete_implements (kept required unreadable : Usize) :
    packet.complete kept required unreadable ⦃ b =>
      b = Calculus.whole (Calculus.Member.counts kept.val 0
        ((required.val - kept.val) + unreadable.val)) ⦄
```

That one line says what `packet::complete`'s remainder is: the required
rows it omitted and the inputs it could not read. #181 adds `unread` to
the same sum.

### The law-coverage check

A register, `proofs/laws.toml`, written by hand and checked:

```toml
[[word]]
type = "writfold_core.guard.Fact"
core = "Calculus.Tri"
abs = "Writfold.Core.Words.factTri"
onto = "Writfold.Core.Words.factTri_onto"

[[link]]
function = "writfold_core.guard.decide"
implements = "Calculus.admit"
theorem = "Writfold.Core.Guard.decide_implements"
laws = ["Calculus.admit_pass_iff", "Calculus.unknown_never_passes"]

[[out]]
function = "writfold_core.bytes.same"
why = "byte equality; no word of the calculus"
```

`proofs/Laws.lean` runs after the build, as `Total.lean` does. It lists
the universe from the environment (every definition under `writfold_core`
that is not a loop: 247 today) and, for each link, the function its
theorem is a triple over and the `Calculus` constants in its
postcondition. `proofs/law_coverage.py` then refuses:

1. a translated function that is not exactly one of linked, out, or on
   `proofs/unlinked.txt`;
2. a backlog that grew;
3. a link whose theorem is missing, is not a required claim, is not a
   triple over its function, or does not mention its `implements`
   constant; a law that is not a theorem under `Calculus` about that
   constant;
4. a function whose result type is a dictionary word and that is listed
   out. Such a function returns a judgment, and a judgment has a law;
5. an enum of the translated types that is neither a `[[word]]` nor a
   `[[local]]` with a reason;
6. a dictionary function without its `onto` theorem.

It prints `LAW_COVERAGE judged=7 declared_out=0 remainder=240 of 247` at
step 2 and `remainder=0` at step 9, when the backlog file is deleted and
any remainder fails.

The universe is the translated functions, not `start_from`: the drift
check already ties the two, and the environment cannot be out of date.

This is the mechanism behind the owner's concern. A new kernel adds a
translated function. If it returns `guard::Fact`, `gate::Outcome`,
`guard::Decision`, a verdict or any other dictionary word, it must be
linked, and the link names the core operation and so the laws. If it
returns a new enum, the enum must enter the dictionary or be declared
local with a reason a reviewer reads. If it returns `bool`, it is linked
or declared out by name; the claims register of #190 lists every
universal word a verb prints, and step 7 requires each such row to name a
link.

### Claims, tauten and the gate

- `proofs/lakefile.toml`: a `Calculus` library, built by default.
- `proofs/Audit.lean`: audits the `Calculus` namespace too; law theorems
  and link theorems are required claims.
- `proofs/mutants.py`: copies `Calculus/` into each scratch project;
  runs core mutants; refuses a law no mutant names.
- `proofs/gate.sh`: a step after the axiom audit,
  `lake env lean Laws.lean | python3 proofs/law_coverage.py`; a step
  refusing foreign imports under `proofs/Calculus/`.
- `proofs/claims.toml`: each link theorem joins its function's
  `[[claim]]`, so the tauten proof record covers it with no schema
  change. Any edit under `proofs/` stales `proofs/claims.json` until
  `cargo tauten proofs` is rerun, as today; each step does that.
- `proofs/laws.toml` stays outside tauten's format. Whether tauten's
  claim schema should carry a `laws` list is an owner question (#214).
- The split translation (#95) is unaffected: a link file imports one
  kernel's theorem file, so a mutant rebuilds one more small file per
  kernel it reaches.
- The RFC template gains one line under Proof obligations (step 2): the
  core operation each kernel function implements and the laws it is
  subject to, or why none applies.

### What the field-feedback kernels look like against the core

Each of these is specified in its own RFC. None changes behaviour here;
what changes is the right-hand side of its table theorem.

| Issue | Kernel | Implements | Its listed theorems become |
|---|---|---|---|
| #181 | `account::remainder`, `whole` | `Calculus.remainder`, `whole` | `remainder_spec` is the link; the other four are the core's |
| #181 | `packet::complete` with `unread` | `whole` over counts | `unread_never_complete`, `complete_implies_whole` from `whole_counts` |
| #189 | `claims::accounted` | the predicate `Universal` (a bare block is `fail`) | `bare_never_passes`, `empty_never_passes`, `remainder_is_bare` from `universal_sound`'s fields |
| #188 | `delivery::parts_match` | `(universal false …).admits` | `empty_never_matches`, `unread_part_never_matches` |
| #187 | `delivery::outbound` | a fold of `Decision.join`; `Unclassified` is a `Posture` | `unclassified_pass_iff`, `local_never_refuses` from `join_eq_pass` |
| #184 | `site::decide`, `stricter` | `Site.decide`, `Posture.stricter` | `closed_pass_iff`, `judged_unchanged`, `posture_only_narrows` and the three `stricter` laws are the core's |
| #186 | `saw_fact` | `Calculus.search` | `saw_met_iff`, `absence_needs_whole`, `stale_never_meets`, `unreported_never_quiet` |
| #196 | `referent::settle`, `judge` | `Calculus.settle`, `judge` | `settle_only_withholds`, `judge_fresh_needs`, `judge_refines` |
| #179, #197 | `target::choose`, `choose_in` | `Calculus.resolve`, or its two properties | `other_never_chosen`, `disagreement_is_ambiguous`, `pin_decides`, `declared_only_url` |
| #182 | `verdict::observed` with `Unfetched` | monotone into the knowledge order (#212) | `fetch_only_refines`, `unfetched_never_decides` |

Kernel-specific theorems (`open_is_today`, `default_unchanged`,
`one_remote_unchanged`, `no_min_unchanged`, `tag_is_not_a_citation`,
`block_bounded`) stay in their RFCs: they are about compatibility or one
kernel's table, not about a law.

### What is not formalized, and why

| Not in the core | Why | What covers it |
|---|---|---|
| Parsers and readers (TOML, tokens, command lines, event bindings, URL normalisation) | their correctness is agreement with a format, not a law; a Lean model of a grammar would be a second implementation with nothing to refine | tables, property tests, golden files |
| File and Git IO, digests | not pure; "a digest identifies content" is a stated premise | kill tests, `index verify` |
| Turning what was read into facts (which file is a `Remainder`, which remote is `Url`) | the abstraction boundary: a link's input abstraction starts at the facts | the foreign-member test and the convention-literal check of #190 |
| Time: "judged again" (3.8), the invocation rules (§4), merges | properties of interleavings, with no pure function to state them of | `spec/*.tla` under TLC |
| The plan fold and splice placement (3.7) | #17 was declined; no new theorem may rest on it | the comparison test against the proved `splice` |
| Glob matching, `find`'s tiers, the command reader's tables, lifetimes, the checker handshake, the decision-state table | proved, each with its own spec; no law of the calculus is about them | their existing theorems; declared out in the register with a reason |
| That a model read anything, that a statement is true | not determinable (`assurance.md`) | nothing |
| Budgets, delegation, tool schemas | deferred by the calculus (#159, #146) | — |

## Compatibility and reversibility

No product code, file format, verb or output changes. The steps add
files under `proofs/` and lines to `Audit.lean`, `mutants.py`,
`gate.sh`, `claims.toml` and the docs. Existing theorem statements keep
their names. Where a step replaces a per-kernel proof by a transferred
one, the statement is unchanged, so the audit and the mutants see no
difference.

Reverting is deleting `proofs/Calculus/`, `proofs/Writfold/Conform/`,
the register and one gate step. Nothing else imports them until a new
kernel states its table theorem as a link; such a kernel would need its
theorem restated.

## Alternatives

- **Keep proving laws kernel by kernel.** It works and found no false
  kernel. It is what let seven RFCs of one day each invent their own
  "never passes" list, and nothing fails when a list is short.
- **A core with no refinement: Lean definitions as documentation.** It
  would give the prose a type checker and the kernels nothing. The
  archived calculus of the edit flow (E01 to E12) was of this kind, tied
  to kernels by exported cases. It was archived with the flow.
- **Write the kernels in Lean and extract Rust.** It reverses the
  decision of 9 November: production would no longer run the code the
  theorems are about.
- **Type classes in Rust: one `Word` trait all kernels implement.**
  Aeneas translates trait use poorly, the kernels are monomorphic on
  purpose, and it is product code churn. The dictionary gets the same
  identification on the Lean side for free.
- **Rewrite every spec theorem against the core.** About 600 statements
  would change and every mutant would need recalibrating. Deriving links
  from the existing specs costs about a dozen lines per function and
  touches nothing.
- **Mathlib's lattice classes for the orders.** The core's orders are
  three and four elements; `cases <;> rfl` proves each law. Importing
  Mathlib would make the core depend on the heaviest pin in the image.

## Evidence and tests

A prototype, compiled on 27 November 2026 in the `wf-lean` container
(Lean 4.35.0, the pinned Aeneas backend, `proofs/Extracted` and the
built `proofs/Writfold` of `0a0bca86`):

| File | Lines | Theorems | Build |
|---|---|---|---|
| `Calculus/Word.lean` | 163 | 25 | |
| `Calculus/Account.lean` | 131 | 14 | |
| `Calculus/Evidence.lean` | 120 | 12 | |
| `Calculus/Site.lean` | 82 | 7 | |
| `Calculus/Referent.lean` | 178 | 12 | the five: 2 s |
| `Conform/Sample.lean` | 162 | 12 (7 links) | 3 s |
| `Conform/Cover.lean` | 49 | the universe and link listing | |

Every theorem uses only Lean's three standard axioms (checked with
`collectAxioms` over both namespaces). The links are over `guard::decide`,
`guard::combine`, `walk::complete_within`, `packet::complete`,
`runs::complete`, `policy::met` and `pointer::open`, each derived from
the kernel's existing spec theorem. `narrow_met_iff`, which the calculus
lists as owed, falls out of `policy_met_implements` and `met_inf`.
`Cover.lean` lists the 247 functions and finds the seven links by
shape. It also matched `guard_unknown_never_passes`, a transferred law
and not a link, which is why the check reads an explicit register and
uses the shape only to verify each entry.

The prototype is committed beside this RFC, under
`2026-11-27-calculus-in-lean/prototype/`, so that step 1 starts from
files that compile. No gate builds it. It is deleted when step 4 lands.

Each step passes the proofs gate (drift, `lake build`, axiom audit,
mutants, and from step 2 law coverage) and re-records the tauten proof
record. No step changes a file under `crates/`, so the quality and unit
receipts stay valid.

## Rollout

Steps, each gated on its own. Sizes are agent working days.

| Step | Issue | What | Size | Needs |
|---|---|---|---|---|
| 1 | #205 | the core: words and accounting; the library, the audit, core mutants | 1 | — |
| 2 | #206 | the dictionary, the first links, the register, the coverage check | 2 to 3 | #205 |
| 3 | #207 | the core: evidence and sites; `narrow_met_iff` | 1 | #205 |
| 4 | #208 | the core: referents and `judge`; link `pointer`, `subject`, `store::claim`, and `referent`, `target` when on `main` | 1 | #206 |
| 5 | #209 | link the admission kernels (about 55 functions) | 2 to 3 | #206 |
| 6 | #210 | link the evidence kernels (about 70) | 3 to 4 | #206, #207 |
| 7 | #211 | link every universal word (about 20) | 1 | #206 |
| 8 | #212 | link the verdict kernels (about 20); one monotonicity theorem | 2 | #206, #208 |
| 9 | #213 | backlog to zero, the check refuses a remainder, the laws table | 1 | 5 to 8 |

Steps 1 and 2 pay off first: after about four days the two laws the
feedback turned on are theorems, seven kernel functions are tied to them, and no
new kernel can arrive unlinked. Steps 5 to 8 are independent of each
other.

**Which calculus steps come before which field-feedback issues.**

| Field-feedback issue | Must follow | Why |
|---|---|---|
| #181 run directory accounting | #205 | `account` is the core's `remainder` and `whole` in Rust; its spec is the first link |
| #189 claims in text; #187, #188 outbound | #205 (and #207 for #187's policy) | their words are `universal` and `Decision.join` |
| #184 posture; #186 ledger | #205, #207 | `site::decide` is `Site.decide`; `saw_fact` is `search`; a ledger row is `reported` |
| **#196 `referent`, #179 citation target** | **nothing** | priority work that unblocks a team. If #208's core file is on `main` first they state links; otherwise #208 links them |
| #180, #197, #182 | nothing | linked by #208 and #212 after they land |
| #198, #202 | #208 | later resolvers and the directory provider are written against `resolve` and `judge` |
| #183 `guard check`, #185, #191, #192 | nothing | no decision logic |
| #190 audit | nothing; #211 joins its register | |

Estimated size in full: about 3,500 to 4,500 lines of Lean added to
27,960 (the core 700 to 900, the dictionary about 300, links about
2,500 to 3,000, the check about 250 of Lean and Python); about 90 core
theorems, 150 links, 25 core mutants. Fourteen to seventeen working
days end to end.

## Unresolved questions

None blocks step 1. Owner questions, each with the default taken, are in
#214:

1. A hand-written Lean model returns, as a specification and not a copy
   of the code. *Default: proceed; one dated sentence in `aeneas.md`.*
2. The holds in the table above. *Default: as listed.*
3. The dictionary and the out list join the trust base. *Default: the
   owner reviews them once per step, after the fact.*
4. One word type in Rust. *Default: no.*
5. Proofs gate time. *Default: accept; record it per step.*
6. Laws about time stay in TLA+. *Default: yes.*
7. A `laws` key in tauten's claim schema. *Default: ask upstream after
   step 2; not filed yet.*
8. One sentence in `AGENTS.md` at step 9. *Default: yes.*
9. The committed prototype. *Default: keep until step 4, then delete.*

## Decision log

- 2026-11-27: drafted after the owner asked for the calculus to be put
  on a formal foundation, from an audit of `proofs/` at `0a0bca86` and a
  prototype compiled the same day. Accepted under the owner's standing
  acceptance (21 November 2026); not yet reviewed by the owner. Aligned
  with the asset boundary RFC of the same day: its law A is the core's
  `judge`, and no step here holds #196 or #179.
- 2026-11-27 (as built, v0.6.0-dev.27): #206, #207 and #208 landed
  together. The register adds two rules the RFC left open: the backlog
  also holds enums (`type:<name>`), and its first line `# entries: N` must
  equal its length, so growing it is a visible edit. The check reads the
  translation and the Lean sources as text instead of the built
  environment (`proofs/law_coverage.py`, tested). The generated page
  `docs/writfold/laws.md` (owner, 27 November: a home for the laws a
  reviewer can read and flag) lists every law with its statement, links
  and mutants; the quality gate refuses a stale page (the proofs image
  holds no `docs/`). `standingOf_onto` is onto every standing but
  `reported`, which no gate records. `guard_unknown_never_passes` is a new
  theorem in `Conform`, since `Guard.lean` cannot import the links that
  import it. `target::choose` stays on the backlog: its `Head` row (a
  clone with no fetched remote) and its pin row map onto `resolve` only
  with an extra candidate per row; the link lands with #212 and
  `choose_in` (#197).
- 2026-11-28 (as built, #212): the knowledge order of a vocabulary is a
  new core file, `Calculus/Knowledge.lean`: `Words.refines` (the same
  word, or the unknown word) and `Words.told`, with `refines_disagree`
  (two fuller facts that disagree leave the lesser one unknown) and
  `settle_monotone`. The order on facts is `Core.Verdict.Knows` (history,
  a span search, fetched bytes, a completed follow); `observed_monotone`
  is the link of `verdict::observed`, and `history_only_refines`,
  `fetch_only_refines`, `unfetched_never_decides`,
  `absence_never_evidence` and the `k = false` half of
  `unsearched_never_fresh` are proved through it. `judge_refines` is
  `Calculus.settle_refines` through `settleOf_settle`, and
  `judge_monotone` composes both. A mark is the unknown word of a marked
  citation (`markWords`), so `with_mark` and `decide` are monotone too.
  Not monotone, and so not in `Knows`: a known cited blob with an
  unsearched span reads `changed`, where a search could read `fresh`
  (`unsearched_never_fresh`, k = true). `store::resolve` is `resolve`
  once a clone with another origin is not a candidate;
  `store::{artifact, snapshot}` admit as `Tri.all` of their facts.
- 2026-11-28 (as built, v0.6.0-dev.29, steps 5, 6, 7 and 9: #209, #210,
  #211, #213). Backlog closed: `LAW_COVERAGE judged=107 declared_out=156
  remainder=0 of 263`; `proofs/unlinked.txt` deleted and `law_coverage.py`
  refuses any function neither linked nor out, and an out without a reason.
  Step 5: admission kernels linked to `admit`, `Tri.all` and `join`; those
  that return a `Result` or a refusal (`plan.advance_check`, `advance`,
  `skip_check`, `round.{advance_check, again_check}`,
  `invoke.prepare{,_check}`, `repin.repin_check`, `compose.{closure,
  covers}`, `handoff.{strict, checked}`) prove the predicate form ("Ok iff
  admit … = pass"); `plan.round.exit` covers only its met row. Existing
  per-kernel laws were kept in place (moving them out of the kernel files
  would cycle imports); new corollaries `plan_advance_undefined_never_passes`
  and `delivery_empty_never_ready` go through the link. Step 6: the core is
  coarser than `gate.State` (open, stale and failed are all `no`) and than a
  grounding (no waived or unknown word: its onto is restricted); `order.newest`
  and `verdict::recorded::{newest, better, best_recorded}` are declared out,
  since their orders are not the single number `Calculus.deciding` orders
  by, and `policy.close` is out (the core has no weakest-class operation);
  `done_only_when_met` is restated beside the original as
  `Conform.done_only_when_met`. Step 7: `units.universe` is proved in one
  direction (an accepted universe is whole) and `inventory.unreadable` by
  whether its count is above zero; transferred laws are new corollaries with
  their own names, originals untouched. Step 9: the laws table of
  `docs/guide/assurance.md` is generated by `laws_page.py` and the quality
  gate refuses it stale.
