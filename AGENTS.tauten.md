<!-- tauten agent profile v0. Paste into your project's AGENTS.md / CLAUDE.md. -->
## Strictness profile (tauten)

This workspace uses the tauten profile. `cargo tauten check` must pass before
you report a task as done. Run it with `--format json` to get one diagnostic
per line; each has a rule `id`, the `file`/`line`, and a `fix`.
`cargo tauten explain <ID>` prints the full rule.

**Tiers.** Every crate declares `[package.metadata.tauten] tier`:
- `kernel`: pure decisions. `no_std`, no IO, no floats, no `HashMap`, no
  shared mutability, no lint exceptions at all.
- `core`: library logic. No direct filesystem, environment, process, clock,
  randomness, network, stdout/stderr or global state. These arrive as
  parameters (a capability, a `&mut impl Write`, a `Clock` trait).
- `edge`: `main`, startup, adapters. The only place that touches the world
  or creates capabilities. Keep it thin.

Dependencies point inward only: edge → core → kernel.

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
