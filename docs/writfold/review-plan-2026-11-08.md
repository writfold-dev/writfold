# Independent review: accepted plan (8 November 2026)

An independent read-only review of Writfold, Weft, Splicery and the website
was run on 8 November 2026. This page records what was accepted, deferred or
left for the owner, and the resulting execution order. It is planning, not
evidence: no gate, ALPHA requirement or admission changes status because of it.

## Headline finding (accepted)

Components are being admitted faster than they are connected. The runtime
still serves only `initialize`/`shutdown`, every ALPHA requirement is
`not_run`, and no library component is on a wired path. From here, work is
ordered by the single-file vertical slice, and each step must extend a
callable path rather than add another isolated component.

## Accepted and done

| Item | Change | Repo / commit |
|---|---|---|
| Split identity contract blocking SG08 | Splicery's local `sg08-identity-vectors.json` marked superseded; `writfold-worktree/1` (`wfwt1`) declared normative in `protocol/worktree.md` | splicery `eae0bdf` |
| Pin drift (hand-typed hash) | Weft's vendored `writfold.agent/1` README archive hash corrected (`…6e53a9af…`) | weft `c8ecb68` |
| Lean↔Rust shared procedure cases | 39-case Lean-exported corpus with a Rust agreement test and a fixture freshness script; caught a self-route gap | writfold |
| Splicery example language | Website Splicery page edits a Rust file, run with the pinned binary | website |
| Signed edit approval (first slice) | Closed CBOR payload, distinct Ed25519 domain, strict policy-relative verification against current expected values; grants and revocations remain | writfold |
| Operator grants and revocation | Closed CBOR grant/revocation payloads with distinct Ed25519 domains and target uses; pure authority kernel (install, revoke, current grant, coverage) in exact agreement with Lean on 79 exported steps | writfold |
| Validated ledger kernel (first slice) | Pure history check and conditional append mirroring Lean E05; exact step-by-step agreement on 31 Lean-exported steps | writfold |
| Website overclaims and wording | "a future native adapter"; Splicery page uses *apply* for writing an edit and *install* only for software; guide date updated | website |

## Accepted: execution order

1. **Commit structural procedure admission.** Already implemented; its unit, Verus and quality receipts complete the scoped component.
2. **Lean↔Rust shared cases (S–M).** Export the engineering reference examples (procedure, catalog, registry, expected result) as JSON from the Lean executable. A Rust test then requires agreement: every Lean-false procedure case is refused, and every Rust-admitted case is Lean-true. Property tests cover edge order and routes. This turns "stricter than Lean" into checked divergence.
3. **Operator approval payload and signature domain (M).** Exact closed prepared-edit approval, grant and revocation payloads with distinct Ed25519 domains, strict verification and wrong-domain/transplant refusals. The parked draft (`interrupted-procedure-authority-20261108`) is the starting point; its missing `signing` module is rewritten, not recovered.
4. **Validated ledger (ALPHA03 core, M).** Rust history check and expected-head append mirroring Lean `historyCheck`/`AppendReady`/`append` (E05), with Lean-exported cases. This is the most important missing module and precedes further Omnigraph work.
5. **CLI vertical slice (L).** `prepare` → `approve` → `apply` on one file, reusing embedded Splicery replay, retained objects, staged install and the ledger, through `writfold-app` decisions that stdio methods share later.
6. **Weft report verdict (M, Weft).** Add `summary` (all required entailed, counts, failing IDs), per-check `required`, producer identity (version, binary SHA-256, profile), typed inputs bound to exact blob digests, a canonical self-digest, and a verdict header under the 64 KiB frame with a separate trace artifact. Writfold's ALPHA05 refusal then has a machine-readable input.
7. **Verus retargeting (M).** New Verus effort goes to the bodies the alpha executes: approval byte assembly, ledger prefix/head validation, expected-old CAS, reservation generation arithmetic and the staged-install state machine. Existing newtype coverage stays as admitted; whether to collapse it into a single macro obligation is an owner question.
8. **Smaller items (S).** A README "What works today" table (command → scope → receipt). Weft reuse parses prior reports under the transport node bound, so large valid reports are refused (fix in Weft). Splicery owns an `alpha-edit/1` profile matching Writfold's single-file replay. Each repository gets a machine-checked sibling pin lock. Commits keep code separate from evidence records.

## Deferred, needs a design decision

- **Per-gate declared input sets.** The whole-checkout binding is fragile: a documentation edit invalidates a running gate. Narrowing it changes the evidence-binding policy in plan 05, so it needs owner approval first. Until then, edit documentation only between gate runs.
- **Restructuring `status.json` into uniform component records.** A mechanical change, but every historical record must be preserved exactly. Do it after the vertical slice, together with the README table.

## Owner questions

1. May the experimental alpha use a filesystem/Git-CAS ledger with no Omnigraph companion, given that the only Omnigraph build is a development-only source candidate? The alpha profile currently names Omnigraph as its backend.
2. Is the operator approval signed over the exact Weft report bytes, or over a Writfold-canonical digest of them? Weft has no canonical form today.
3. Should Verus coverage of macro-generated newtypes collapse into a single macro obligation?
4. Who owns the dirty-worktree profile: Splicery `dirty-worktree-utf8/1` or Writfold's `metadata_policy`?
5. May gates declare narrower input sets, keeping whole-checkout binding for `baseline` and `release-required` only?
