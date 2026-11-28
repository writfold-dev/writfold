<!-- tauten agent profile v0. Paste into your project's AGENTS.md / CLAUDE.md. -->
## Strictness profile (tauten)

This workspace uses the tauten profile. `cargo tauten check` must pass before
you report a task as done. Run it with `--format json` to get one diagnostic
per line; each has a rule `id`, the `file`/`line`, and a `fix`.
`cargo tauten explain <ID>` prints the full rule.

**The architecture on one page.** Each line says what enforces it, and how
strongly: *proved* (a Lean theorem over the deciding code; the facts it is
given are read by tested, unproved code), *compiler-enforced* (rustc rejects
the code), *lint* (`cargo tauten check` reports it; an exception needs a
reason), *tested*, or *not checked*. Links are to
[tauten](https://github.com/tauten-dev/tauten/blob/main/) sources.

1. **Decisions are total kernel functions with claims.** Put a decision in a
   `kernel` crate as a plain function that returns for every input.
   - The crate is `#![no_std]`, so it cannot do IO: *compiler-enforced*,
     required by TAU-K003 (decided by the proved
     [`layout::k003`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/layout.rs)).
   - No panics, unchecked arithmetic, indexing, floats or std paths:
     TAU-P001–P004, TAU-N001–N003, TAU-K001–K002, *lint*.
   - A claim in `proofs/claims.toml` (a Lean theorem over the function's
     translation, plus mutants that must break it) is recorded `proved` by
     `cargo tauten proofs` only when it builds, passes the axiom audit and
     every mutant fails it: *proved* (gate admission is
     [`control::admit`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/control.rs)).
     A `total` theorem lets `check` treat TAU-N001/P003 findings in that
     function as subsumed: *proved*
     ([`subsume::subsumes`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/subsume.rs)).
   - That a decision has a claim at all: *not checked*. Add one.
2. **Protocols are machines.** Write a protocol as
   [`tauten::machine::Machine`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten/src/machine.rs):
   `step(state, event) -> (state, command)`, with a driver that carries out
   the commands.
   - `step` takes no capability: *compiler-enforced* (the trait signature);
     in kernel and core crates it cannot reach ambient IO either (TAU-A001,
     *lint*).
   - A recorded tape replays exactly (`machine::replay`): *tested*.
   - An invariant kept by every step holds in every reachable state
     (`Tauten.Machine.inv_of_step`, shipped as `proofs/Proofs/TautenMachine.lean`):
     *proved* once; your protocol's invariant is proved only when you add
     that claim.
3. **The world is reached only through capabilities named in the effect
   row.** Core code takes capabilities
   ([`tauten::cap`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten/src/cap.rs):
   `Clock`, `Env`, `Dir`, `ReadDir`, `#[derive(Cap)]` bundles) as
   parameters; only the edge creates them.
   - Ambient filesystem, environment, clock, network, stdio and randomness
     in kernel and core: TAU-A001, *lint*. `Authority::acquire` outside the
     edge: TAU-A002, *lint*.
   - `Authority` and `SystemClock` have private constructors, and a
     `#[derive(Cap)]` field must be a capability: *compiler-enforced*. Test
     fakes (`ManualClock`, `Env::from_pairs`) can be built anywhere; they
     reach nothing real.
   - `#[effects(..)]` lists exactly what the signature grants (TAU-F001):
     *lint*, decided by the proved
     [`effects::verdict`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/effects.rs);
     parameter types are classified by name, and an unclassified type only
     warns. With `effects = "required"` in a crate's
     `[package.metadata.tauten]`, a missing or wrong row is an error.
4. **A check travels as evidence or a binding.**
   - [`tauten::origin::Evidence`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten/src/origin.rs)
     (`#[evidence(subject = T)]`) borrows the state it checked: changing
     the state while it lives, forging it outside its module, or cloning it
     does not compile: *compiler-enforced* (until `redeem()`; redeem it in
     the call that acts).
   - A recorded result (a proof outcome, a generated file) is reused only
     while every input it was bound to is byte-identical: *proved*
     ([`binding::fresh`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/binding.rs)).
5. **A result is shown with the badge its controls admit.** `proved` only
   for a fresh Lean theorem over the extracted code with no assumption;
   otherwise `proved-under`, `model-proved`, `bounded`, `tested`,
   `reviewed`, `pending` or `not-established`: *proved*
   ([`badge::badge`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/badge.rs),
   [`control::admit`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/control.rs)).
   A timeout or a missing tool never counts as a pass. Say only what the
   badge says.
6. **Registries are never weakened.** A recorded claim may not lose a
   theorem or mutant or gain an assumption without an `[[amend]]` entry
   with a reason (TAU-R002), and the lint baseline may only shrink
   (TAU-R001): *proved*
   ([`registry::conserves`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/registry.rs),
   [`ratchet`](https://github.com/tauten-dev/tauten/blob/main/crates/tauten-check-kernel/src/ratchet.rs)),
   relative to the committed `proofs/claims.json` and baseline.

**Tiers.** Every crate declares `[package.metadata.tauten] tier`:
- `kernel`: pure decisions. `no_std`, no IO, no floats, no `HashMap`, no
  shared mutability, no lint exceptions at all.
- `core`: library logic. No direct filesystem, environment, process, clock,
  randomness, network, stdout/stderr or global state. These arrive as
  parameters (a capability, a `&mut impl Write`, a `Clock` trait).
- `edge`: `main`, startup, adapters. The only place that touches the world
  or creates capabilities. Keep it thin.

Dependencies point inward only: edge → core → kernel.

**Protocols are step machines.** Write a protocol (a publish, a lease, a
retry) as `tauten::machine::Machine`: a pure `step(state, event) ->
(state, command)` with an empty effect row, and let an edge or core driver
carry out the commands. The empty effect row is what makes a recorded tape
replay exactly (`tauten::machine::replay`) and what lets the protocol be
proved once by induction (`Tauten.Machine.inv_of_step` in the proofs).

**When a rule fires, fix the cause.**
- Do not add `#[allow]`. If an exception is truly correct, use
  `#[expect(lint, reason = "why this is safe here")]` on the smallest item,
  and say so in your report. Some rules (stubs, `unsafe` outside the edge,
  anything in a kernel crate) admit no exceptions.
- Do not move code to the edge tier, change a crate's tier, or edit
  `[lints]`/`clippy.toml` to make a finding go away.
- Replace `unwrap`/`expect` with `?` and a typed error enum.
- Replace `a + b` with `checked_add` (or `saturating_*` when that is the
  specified behaviour), `v[i]` with `.get(i)`, `as` with `try_from`.
- List every enum variant instead of `_ =>`.
- Never leave `todo!()`, `unimplemented!()`, `dbg!()` or "for now" comments.

**Tests.** Tests may panic, index, do arithmetic, use temp files and print.
They may not silently drop a `Result`, use stubs, or use `#[allow]`. Put
helpers used only by tests inside `#[cfg(test)]` modules. Never delete,
weaken or `#[ignore]` a test to make a task pass; report the conflict instead.

**Proved kernel code.** A kernel function with a proof must stay inside
what the pinned translation toolchain accepts (lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd).
`cargo tauten proofs extract` decides; these notes are advice:
- No trait objects (`dyn Trait`, `&dyn Fn`): use an enum or a generic.
- No names Lean reserves (`given`, `set`, `at`, `fun`, `have`, `show`, ...) for parameters, locals or fields; the extract stage builds the translation, so Lean reports any such name.
- Do not name a parameter like its own module (`fn width(span: Span)` in `mod span`): in Lean it shadows the module's types.
- No floating point: use integers or fixed point.
- No slices of structs that hold borrows: use owned data (`Vec<u8>`, indices).
- At most one loop per function when it returns early from inside a loop: split the loops into helpers.
- No `match` on `xs.get(i)` directly in a loop body: use `if let Some(x) = xs.get(i)` or a helper.
- No iterator chains (`iter().map(..).sum()`): they translate only as axioms, which the proof audit refuses. Write a `while` loop over `get`.
Known to translate:
- `while` loops over `get`, `matches!` / `match` on enums, owned `Vec<u8>`, `saturating_*`.
- Integer `as` casts translate (the profile still prefers `try_from` for narrowing).
- A derived `PartialEq` used with `==` translates.
