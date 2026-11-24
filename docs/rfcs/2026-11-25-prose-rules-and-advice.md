---
rfc: "2026-11-25-prose-rules-and-advice"
title: "Prose rules for delivery drafts: rules as data, advice separate from blocking"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-25
updated: 2026-11-25
issue: https://github.com/writfold-dev/writfold/issues/127
milestone: "v0.6.0-dev.20"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Prose rules and advice

Issues: #127 (`prose.rules`) and #115 (advice kept on a pass). Owner
questions: #130. It replaces the draft diagnostics and Vale RFCs of PR #120,
which never reached `main`.

## Proof obligations

A new kernel, `writfold_core::prose`, decides what a list of rule matches
means. Each match is weighed `Advice` or `Blocking`.

- `blocking(found)` is how many matches block.
- `verdict(rules, found)` is `Unknown` when there are no rules, `Fail` when
  anything blocks, and `Pass` otherwise.
- `agrees(outcome, blocking)` is the reader's check that a delivery
  receipt's outcome fits its blocking findings. It applies to every delivery
  check, not only prose. It moves the reader's existing tested rule (a pass
  has no findings, a failure has some) into the kernel.

`proofs/Writfold/Prose.lean` proves:

- `blocking_spec` (via `blocking_loop_spec`): the count of blocking matches.
- `verdict_spec`, which `verdict_no_rules_unknown`, `verdict_pass_iff` and
  `verdict_fail_iff` restate:
  - with no rules the verdict is `Unknown`, so an empty rule set is never a
    pass;
  - it is `Pass` exactly when there are rules and nothing blocks;
  - it is `Fail` exactly when there are rules and something blocks.
- `advice_never_decides`: removing every piece of advice leaves the verdict
  unchanged, so advice can never decide an outcome.
- `verdict_perm`: reordering the matches leaves the verdict unchanged, so
  display order and the 20-finding display cap are not inputs.
- `agrees_spec`, `agrees_pass_iff_none`: a pass agrees exactly with zero
  blocking findings.
- `agrees_verdict`: a receipt written from the verdict always reads back
  consistent.

Mutants:
- `prose-no-rules-pass` breaks `verdict_spec`;
- `prose-advice-blocks` breaks `blocking_loop_spec`;
- `prose-blocking-ignored` breaks `verdict_spec`;
- `prose-agrees-pass-any` breaks `agrees_spec`.

**Tested, not proved:**
- reading the rule file and compiling patterns (the `regex` crate, as for
  the deny list);
- which lines are prose (code fences, indented code, inline code and URLs
  are skipped);
- matching;
- receipt encoding and rendering;
- the script-gate line grammar.

## Why built-in, and not Vale first

The assessment of 24 November 2026 checked each tool's status through
GitHub, crates.io and its release archives.

| Option | Form | Verdict |
|---|---|---|
| Vale 3.28.1 (MIT) | static Go binary, 29 MB; YAML styles tree; `vale sync` needs the network | The best external linter. An optional recipe later (#117). |
| Harper 3.3.0 (Apache-2.0) | Rust; `harper-core` has 28 required dependencies; its CLI calls itself experimental | Not a dependency. A script-gate recipe later. |
| textlint 16 (MIT) | Node and an npm tree | Rejected for the product path |
| LanguageTool 6.8 (LGPL-2.1) | JVM, normally a local server | Rejected: a companion process |
| proselint 0.16 (BSD-3) | Python, no custom rules as data | Rejected |
| write-good (MIT) | Node, dormant | Rejected |
| **built-in `prose.rules`** | inside the binary; `regex` is already linked | **This RFC** |

- **Delivery texts are short.** PR comments, Slack summaries and commit
  messages run 3 to 30 lines. What goes wrong in them is filler, hedges,
  placeholders and house terms: phrase and pattern rules.
- **The mechanism exists.** `references.denied` already reads
  `"name: pattern"` rules from the KB, compiles them and binds every rule
  into the obligation digest, so editing a rule stales a receipt.
- **What was missing is advice.** A delivery receipt saying `pass` with
  findings was refused on read, and a script gate threw its output away on
  exit 0.
- **Reproducibility is cheap here.** The rule file's bytes are in the
  digest. With Vale it would need the pinned executable, an isolated config,
  a styles-tree inventory (#116) and `retain` on candidate gates (#128).
- **The decision is provable.** With an external linter, Writfold's only
  decision is exit-status handling.

**What it does not do:** grammar, spelling, sentence rules (#129), style
packs, editor integration or a quality score. It reports matches of rules
you wrote; it does not judge tone or truth.

## Decision

1. **Rule file.** `.writfold/deliveries/prose-<name>.toml`, beside the deny
   lists:

   ```toml
   schema = "writfold-prose-rules/1"
   name = "plain"
   advise = ["filler: \\b(just|simply|basically)\\b"]
   block = ["placeholder: \\b(TODO|TBD)\\b"]
   ```

   Entries use the deny list's `"name: pattern"` form and the same compiler
   (ASCII, case-insensitive, size-limited). A repeated name or a bad
   pattern is reported at its span.
2. **Mode key.** A mode that lists `prose.rules` in `checks` names its rule
   set with `prose = "<name>"`. The key is required with the check and
   refused without it. The built-in modes do not list it, so they and every
   existing receipt are unchanged. A missing rule file is `ref.undefined`.
3. **Matching.** Rules are matched line by line on prose lines only. Lines
   inside code fences and indented code blocks are skipped, and inline code
   spans and URLs are blanked before matching.
4. **Verdict.** `prose::verdict(rules, weights)`. An empty rule set is
   `unknown` with a note.
5. **Digest.** The obligation digest covers the rule set's name and each
   rule as tier, name and pattern, so changing a pattern or moving a rule
   between advice and blocking stales earlier receipts.
6. **Receipt.** Blocking findings stay in `result.findings` and
   `findings_total`. Advice goes in new `result.advice` and
   `result.advice_total` fields, written only when there is advice, so a
   receipt without advice has today's bytes. The reader checks
   `prose::agrees` for every delivery check.
7. **Script gates (#115).** On exit 0 a candidate script gate's stdout
   findings are kept as advice instead of discarded. A finding line may name
   its rule as `<line>: [rule] text`. Exit codes keep their meaning: a
   nonzero exit is a failure, and a signal, timeout or start failure is
   unknown. Any external linter can therefore give advice through a short
   wrapper.
8. **Showing advice.** `deliver check`, `deliver status` and JSON show
   advice while the receipt is about the present bytes. Advice never
   appears in `next:` and never changes an exit status.

## Acceptance gates

| Gate | Evidence |
|---|---|
| proofs | `Writfold.Prose` builds; the 4 mutants calibrate; the axiom audit passes |
| unit | `crates/writfold-kb/tests/deliver_prose.rs` (advice-only ready and shown after a fresh read; blocking fails; empty set unknown; code skipped; a tier or pattern edit stales the receipt; a mode without the check is unchanged) and the script-gate advice cases |

## Decision log

- 2026-11-25: written from the prose assessment of 24 November 2026 and
  accepted by default under the standing acceptance (AGENTS.md). The owner
  questions are in #130, and the work proceeds on their defaults.
