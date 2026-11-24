---
rfc: "2026-11-24-optional-vale-delivery-checks"
title: "Optional Vale checks for delivery modes"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/117"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-24-delivery-check-diagnostics"
  - "2026-11-24-candidate-check-inputs"
---

# RFC: Optional Vale checks for delivery modes

## Summary

Provide an opt-in Vale recipe through existing candidate command gates.
Keep Vale's native configuration and style packages usable in ordinary
editors and CI. A small adapter supplies generic diagnostics; Writfold owns
receipts, freshness, gate policy and readiness.

Begin with curated advisory rules for PR comments, short Slack messages and
design documents. Keep absolute density thresholds and semantic writing
scores deferred. This is the integration slice of
[#119](https://github.com/writfold-dev/writfold/issues/119); rule evaluation is
[#118](https://github.com/writfold-dev/writfold/issues/118).

## Motivation

The owner wants quality control for agent-written delivery text. Writfold
already supports candidate command gates, but currently discards findings
from successful scripts and does not automatically bind transitive linter
resources. The companion RFCs address those generic limitations.

Vale supplies configurable styles, markup scopes and machine-readable
diagnostics. A wrapper plus native rules is smaller than building another
prose parser, rule engine and editor ecosystem. This is an architectural
choice, not a measured claim that Vale improves the owner's documents.

## Proof obligations

No new prose-quality kernel is proposed. Classification/advisory behavior
belongs to [the diagnostics RFC](2026-11-24-delivery-check-diagnostics.md); applicability and
resource completeness belong to [the input RFC](2026-11-24-candidate-check-inputs.md). Prove
their laws before wiring this recipe. Existing delivery readiness and gate
admission remain the final decision.

Any additional decision introduced while implementing must be put in the
appropriate pure kernel, with named theorems and mutants, before wiring.
The adapter supplies backend facts; a successful rule run is not a proof of
truth, relevance or effective communication.

Tested and unproved: invocation, installed executable/resource verification,
native config resolution, coverage detection, backend exit decoding, JSON
normalization, diagnostic coordinates and format handling. Fixtures must show
that an unconfigured integration has no tool dependency and that a configured
but unavailable checker never supplies positive evidence.

## Behaviour

Users select an explicit native Vale config for each delivery policy. A
proposed example layout is:

| File | Purpose |
|---|---|
| `.writfold/prose/pr-comment.ini` | PR-comment Vale configuration |
| `.writfold/prose/slack.ini` | Short-message configuration |
| `.writfold/prose/design-doc.ini` | Design-document configuration |
| `.writfold/prose/styles/` | Shared rules, vocabulary and other declared resources |
| `.writfold/gates/prose-style.toml` | Candidate command-gate registration |
| `.writfold/deliveries/pr-comment.toml` | Mode selecting its configured gate |

An existing root `.vale.ini` can be selected instead. Do not translate Vale
rules into another Writfold language or maintain a second hidden copy.
Distinct mode configs reference shared rule files. Reuse existing example
scaffolding where useful; the listed files are illustrative, not shipped.

The ordinary prepare/check/status flow invokes the configured gate on the
rendered payload. A clean completed check reports pass. Advisory suggestions
and warnings remain visible without blocking; a deliberately configured
blocking violation fails. Missing tooling, invalid configuration, incomplete
coverage or failed execution remains non-passing even during the advisory
period. Built-in modes change only when a user opts in.

Reports name the profile, tool/resource identity, applicability and findings.
They do not display a numerical overall writing-quality score. Existing
length/reference checks remain in Writfold; avoid duplicate reports.

## Design

### Optional packaging and offline execution

Maintain the first adapter and recipe with Writfold's examples. Use an
external Vale executable selected by explicit setup. Pin a tested release
and verify actual executable/environment identity; pin or vendor actual
style/resource contents as well. The user's existing package manager may
supply the tool. Do not embed Go into the Rust binary, add a required companion
service or implement a package manager.

Setup and updates are explicit, separate operations. The check and every
status/hook read must perform no package sync, model download or network
request. A fresh offline checkout with the declared resources and installed
pinned tool should exercise the same checks. Missing resources produce an
actionable diagnostic, not installation or a silent skip.

Vale's CLI offers `--config`, `--output=JSON` and `--no-global`. These are
useful building blocks, not sufficient proof of isolation by themselves:
verify effective resource paths and any global style/dictionary fallback.
Bind imported config, vocabulary, scripts, grammars or language assets when
used. A version string or a package URL alone is not the resource bytes.

### Exact payload, mode and coverage

Read the candidate payload supplied by Writfold. Declare the mode's format and
logical path explicitly so a configuration section matches the content being
checked, even though the stored path is under `runs/`. The adapter may use
Vale stdin with `--path` or a captured file layout; preserve the payload bytes.

Do not assume Slack formatting is CommonMark. The initial Slack profile can
use a tested plain-text or explicit formatting policy. Include mentions, links,
code blocks and quoted material in fixtures. Source and payload positions
must not be confused when Writfold strips citation tokens.

Validate target existence, matching configuration, installed styles and active
rules. No applicable rules or unsupported format is incomplete coverage for a
configured prose gate. A valid complete run with no findings is a pass;
absence of findings alone does not establish coverage. Inspect effective
configuration with supported Vale facilities and fixture the adapter's coverage
claim. If the pinned version cannot establish the required coverage, surface
that limitation instead of fabricating a positive check.

Inline suppressions need an explicit profile policy and visible suppression
information. Treat them as declared exclusions, not Writfold waivers or proof
that the excluded text was checked. Detect any suppression mechanism needed by
the admitted profile; blanket suppression cannot silently claim full coverage.

### Normalization and retention

Translate Vale rule ID, severity, message, line/span and explanation link to
the generic report. Normalize backend exit conventions using the pinned,
tested contract. Backend errors and malformed output are not editorial
violations. Keep report threshold and blocking policy distinct so advice is
not discarded merely because only selected rules block.

The companion input-binding implementation supplies immutable captured
resources and current applicability. Keep the full bounded report in the
existing receipt/artifact mechanism. No report import authority or competing
receipt ledger is added. The check does not rewrite the document or send it.

### Rule scope and ownership

Start with a small native package: terminology, accepted technical vocabulary,
unfinished placeholders, selected reviewed word substitutions and useful
mode-specific conventions. Reuse suitable upstream rules with their licenses
and notices. Give each custom rule a triggering example and a counterexample.

Do not enable several complete guides by default. Passive voice, necessary
hedging and useful repetition are not universal defects. Vale sequence/script
extensions can support justified custom logic before considering a fork.

Keep a shared style package independently publishable if several projects
reuse it. A separate runtime product is deferred until useful checks require
inputs/semantics beyond Vale and demonstrate value independently. Plyrune
remains optional under #106; no new integration is required for this recipe.

## Compatibility and reversibility

Pre-alpha policy (#105/#107) allows clean current formats and rejection of
unsupported evidence without compatibility shims. Removing the optional mode
gate restores ordinary Writfold use; preserve historical receipts as history.
A machine without a configured Vale gate needs no Vale installation.

The concrete supported Vale version and resource hashes are selected and
recorded during implementation, after fixtures pass. This RFC does not invent
a tested pin or claim parity across untested releases.

## Alternatives

| Option | Evaluation |
|---|---|
| Vale | Recommended for native configurable style rules and local diagnostics |
| textlint | Strong alternative for substantial JavaScript/TypeScript AST rules; adds Node/npm |
| Proselint | Useful editorial comparison; adopt only if it adds validated findings |
| LanguageTool | Possible grammar supplement; local Java operation is an additional runtime |
| Custom engine | Deferred while existing engines support the required rules |
| Vale fork or separate product now | Extra maintenance before a demonstrated limitation |

This is capability/source evaluation. Compare actual usefulness before claiming
one tool produces better writing. Semantic evidence/brief review can remain a
separate later checker; neither readable prose nor a high lexical density
establishes factual correctness.

## Evidence and tests

Primary references consulted on 24 November 2026:
[CLI](https://docs.vale.sh/topics/cli),
[packages](https://docs.vale.sh/keys/packages),
[resource layout](https://docs.vale.sh/keys/stylespath),
[sequence rules](https://docs.vale.sh/checks/sequence),
[script rules](https://docs.vale.sh/checks/script),
[textlint rules](https://textlint.org/docs/rule/),
[Proselint](https://github.com/amperser/proselint),
[LanguageTool local server](https://dev.languagetool.org/http-server.html).

Exercise clean/advisory/blocking results; missing binary/styles; engine version
mismatch; invalid JSON; zero active rules; ignored target paths; suppressions;
Unicode positions; code/quotes/identifiers; Markdown and Slack input; changed
config/resources; stale payload; and offline setup parity. Compare native Vale
findings with normalized findings for identical inputs.

Use public/synthetic fixtures. Any later real-draft evaluation is local and
owner-authorized; publish only approved redacted examples or aggregate results.
No workplace drafts, private paths or playbook material belong here. Existing
sent/linted drafts and constructed padding are not independent quality labels.

## Rollout

All slices are in Later:

1. Curate initial rules and positive/negative fixtures; record exact tool and
   resource candidates without claiming a tested release yet.
2. Land the generic diagnostics and input-binding decisions with their proofs.
3. Ship the opt-in recipe and exercise the complete offline delivery flow.
4. Plan and, when taken up, run a bounded per-rule review under #118. The term
   SQ-SOAK is a rollout label, not an existing Writfold feature.
5. Promote individual rules only under predeclared criteria based on useful
   findings, false alarms, missed defects and editing effort. Keep noisy rules
   advisory or remove them; every policy change invalidates prior applicability.

The planning request does not start new benchmark runs. Split held-out examples
by document/thread family and retain disagreements and negative results.
No arbitrary universal density threshold or unvalidated score is introduced.

## Unresolved questions

- Tested Vale release, adapter language and minimal supported format set?
- Which initial rules justify inclusion, and what is each mode's language policy?
- Wire-level representation of coverage and suppressions?
- Per-rule promotion criteria and the bounded evaluation scope?
- Separate versioned style package now or only after demonstrated reuse?

## Decision log

- 2026-11-24: Drafted at the owner's request for issues/RFCs. The architecture
  recommendation is optional Vale plus native rules and generic gate support.
  No implementation, package installation, corpus trial or benefit claim is
  made by this document.
