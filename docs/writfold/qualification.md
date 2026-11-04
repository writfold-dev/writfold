# Local qualification runner

`./scripts/gate COMMAND --receipt /absolute/outside-source/attempt.json`
executes explicit local commands and publishes a version-2 receipt described by
`qualification/gate-receipt.schema.json`. This runner does not update
`qualification/status.json`, invent unavailable gate producers, or turn a
successful reference command into Rust product qualification.

## Commands and scope

| Command | Executed work | Meaning of a successful receipt |
| --- | --- | --- |
| `baseline` | Historical reconciliation and frozen inventory hashes, plus revision-5 manifest/registry identities, 25 stages, 32 gates, 37 V/M obligations and E01–E12 | Retained and active-contract identities reconciled; not complete WF00/WF21 or fresh historical test execution |
| `bootstrap --tool-image IMAGE@sha256:DIGEST` | Storage preflight, local immutable image inspection, fresh in-container executable/tool observation | Explicitly unadmitted tool observation only |
| `bootstrap` | Storage preflight, local Compose/BuildKit `tools` build, then inspection and observation of the admitted lock image | Tool build/observation only; not offline-closure qualification. A missing lock is a failure, never execution of the mutable bootstrap tag |
| `reference`, `native-smoke` | Compose `reference`, running the existing real Git/Dolt/Python CLI smoke | Historical Dolt reference-only scope; retains historical WF09 association, never qualifies Omnigraph WF09 |
| `unit` | Compose `unit`, offline locked workspace Cargo tests | Executed Rust test counts; ignored tests fail the attempt |
| `quality`, `verus`, `conformance` | Corresponding Compose services and existing backing Python producers | Only the explicitly reported producer obligations; absent implementation/report fails |
| `engineering-lean` | Clean isolated Lean build, transitive audit of legacy and engineering declarations, executable engineering examples | Scoped calculus component only; theorem fidelity remains separate, and WF31 also requires production verification and model bridges |
| `citation-lean` | Clean legacy/engineering/citation build and transitive axiom audit, native citation reference, concrete semantic-mutant controls and proof rejections | Scoped CIT01–CIT06/CIT08 component under WF08; never full WF08, WF32/WF33 activation, universal instance certificates or Rust refinement |
| `build` | Compose `build`, offline locked release musl workspace build with Cargo JSON messages | Compiled workspace artifacts, not an installed executable or release. A second explicit container command copies exactly the compiler-reported filenames into the attempt, including Cargo-reported cached artifacts |
| `release-required` | Independent validation of all WF00–WF31 entries, V01–V21/M01–M16, E01–E12, composition bridges and source-bound receipts | Complete required-gate aggregation only if every mandatory gate independently qualifies |

The new explicit commands are `storage-foundation` (WF24), `omnigraph` (WF09),
`legacy-import` (WF14), `views` (WF22), `obsidian` (WF23), and
`storage-profile` (WF03: versioned storage wire/signature/profile boundary).
They refuse until actual producers exist. The planned production `dolt` command
is removed; reference-native smoke remains available only with its historical scope.
The other normative names (`lean`, `model`, `tlaps`, `sources`, `publication`,
`workflows`, `transport`, `migration`, `evaluate`, `runtime`, `package`, `site`,
`scribes`, `portfolio`, `design`, `companion`) likewise refuse until implemented.
Revision 5 additionally recognizes `engineering-ledger`, `skills`, `effects`,
`cross-repo`, `learning`, `engineering-cuts` and `engineering-calculus`
(WF25–WF31 respectively); these full-gate producers remain unavailable.
They do not print an unrun success. The runner currently emits no `full-gate`
producer receipts. A scoped pass is deliberately insufficient for release.

`quality`, `verus`, `conformance`, `engineering-lean`, and `citation-lean`
producers must write `/evidence/obligations.json` containing:

```json
{
  "checks": [
    {"name": "an-actually-executed-obligation", "required": true, "status": "pass"}
  ],
  "skips": []
}
```

Each named check must describe actual work. Allowed check statuses are `pass`,
`fail`, and `skip`; every reported obligation is required. An empty report,
missing report, nonzero process result, timeout, or any skipped/failed obligation
fails the attempt. Additional producer reports and partial outputs are retained
and hashed even when the producer fails. Unit counts are extracted from Cargo's
actual test summaries; the native reference command must emit its existing
`DOLT_INTEGRATION_OK:` marker and exit successfully.

The citation producer follows
[`qualification/citation-profile.json`](../../qualification/citation-profile.json)
and the [scoped feature adoption](revision-9-features.md). Calibration compiles
each actual mutated decision body first, kernel-checks a concrete violating
witness, and requires rejection at an intended safety declaration. Syntax,
missing-import, timeout and unrelated failures do not count as mutant kills.
Expected negative exits remain in the raw logs and are explicitly bound by
command index; every other command must succeed. A successful accepted proof
cannot contain `sorry`. Models, proof sources and concrete counterexamples are
retained outside disposable build caches in the attempt's `calibration/` tree.
No mandatory full-gate producer is replaced by this component.

## Invocation and tool admission

Optional arguments are `--playbook PATH`, `--tool-image IMMUTABLE_REFERENCE`,
repeatable `--artifact PATH`, and `--timeout SECONDS` (positive, per command,
default 3600). The playbook defaults to
`~/.local/share/writfold/references/playbook-e7d991e`; its HEAD must equal
`e7d991e990e3dff9c5d70e4327c0e90298e2f67e` and its working tree, including
untracked files, must be clean. It is checked again before publishing a success.
The baseline reconciler additionally checks the adopted manifest. Never edit
the operator's live playbook to satisfy this check.

Examples, each using a new receipt name:

```sh
./scripts/gate baseline --receipt "$HOME/.local/share/writfold/qualification/baseline-003.json" --playbook "$HOME/.local/share/writfold/references/playbook-e7d991e"
./scripts/gate bootstrap --receipt "$HOME/.local/share/writfold/qualification/tools-003.json" --tool-image writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6
./scripts/gate native-smoke --receipt "$HOME/.local/share/writfold/qualification/native-003.json" --tool-image writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6
./scripts/gate unit --receipt "$HOME/.local/share/writfold/qualification/unit-003.json"
./scripts/gate build --receipt "$HOME/.local/share/writfold/qualification/build-003.json"
./scripts/gate release-required --receipt "$HOME/.local/share/writfold/qualification/release-003.json" --artifact /absolute/retained/candidate-archive.tar.gz
```

These are invocation examples, not records that these gates passed. `--artifact`
is required for release aggregation and must identify retained files outside the
checkout. Additional candidate artifacts may be supplied repeatedly. Their
identities are checked before and after execution.

The runner consumes these fields of the parent's `toolchains.lock.json`:

- `schema_version`: integer `1`.
- `admitted`: boolean `true`.
- `image.reference`: immutable `name@sha256:<64 lowercase hex>`.
- `image.id`: the exact `sha256:...` ID returned by local Docker image inspection.
- `observation.path`: a checkout-relative path such as
  `qualification/toolchain-observation.json`, or an absolute retained path.
- `observation.sha256`: SHA256 of that exact observation JSON file.

The product lock also retains its separately admitted artifact, archive,
executable, provenance and license metadata; this runner does not synthesize
those facts. Docker must already have the reference locally, and its RepoDigests
and ID must match. A fresh `scripts/writfold_tools.py` observation must exactly
match the locked observation JSON. The observer itself, Compose recipes, and
lock bytes are source-bound. No image pull or mutable-tag fallback occurs.

`--tool-image` is accepted only for `bootstrap`, `reference`, and `native-smoke`.
It still requires local digest verification and fresh tool observation, but
labels `tool_admission` as `explicit-unadmitted`. It cannot qualify release and
does not write a tool lock. `bootstrap` without this override builds locally,
but a missing/unadmitted lock still fails observation admission afterward.
Build output is never automatically promoted to an admitted product toolchain.

## Evidence lifecycle

The receipt's sibling `attempt.json.attempt/` directory is exclusively created;
an existing receipt, dangling receipt symlink, or attempt directory is never
overwritten or resumed. Failed attempts remain available. Choose a new path for
a replacement and preserve the previous attempt in the parent's status history.
The complete receipt is atomically hard-linked into place without replacement.
Logs and user-owned output files are made read-only, and the completed attempt
directory is sealed read-only. Container-owned outputs are retained with their
ownership and verified hashes; hashes detect subsequent modification rather
than claiming protection against an administrator changing filesystem bytes.

Each subprocess records exact argv, cwd, deadline, wall-clock start/end,
monotonic duration, exit and timeout disposition. Stdout and stderr are separate
hashed files. Timeouts kill and reap the host process group; failed/interrupted
Compose runs also force-remove their named container before returning. Storage
preflight failures produce failed receipts and raw logs instead of disappearing.

Source identity enumerates Git-tracked and nonignored untracked source files,
including deletions, content SHA256, byte lengths and file modes. It does not
walk build caches. Symlinks, submodule directories and special files require
separate admission and are refused. `qualification/status.json` is deliberately
excluded from the tested content scope because it is a mutable evidence index;
release aggregation separately binds its exact SHA256. The Git commit and scoped
dirty flag (with that same status-index exclusion) are also recorded. Source
identity is compared before and after execution;
a mismatch prevents success. Stop concurrent source editing during qualification.

The receipt records Cargo/tool-lock hashes (null when genuinely unavailable),
the candidate musl target and default Cargo feature selection, scoped fixture
and gate-definition digests, playbook pin, obligation counts, candidate artifact
identities and all retained report/log hashes. The command argv distinguishes
host-target unit/proof work from the musl build. `evidence_mode: executed` means
these commands executed in this attempt, not that Cargo rebuilt every cached
artifact or historical reference tests were rerun. Cargo's JSON artifact
freshness flags remain in the build log.

## Fail-closed release consumption

Every WF00–WF31 entry must be present exactly once, required, passed and linked
to at least one receipt. Receipt paths in `qualification/status.json` are retained
external paths, not in-tree reports. Every selected receipt must use the current
closed schema, name the expected gate, have `scope: full-gate`, admitted tools,
zero exit, no skips or failed obligations, positive complete obligation counts,
and actual command/log/artifact evidence. Every command must have succeeded
without timeout. Each receipt must bind all supplied candidate artifact hashes.
E01–E12 additionally require exact symbol agreement with the audited engineering
registry, its explicit full-obligation pass, and applicable `engineering-lean-only`
receipts. Component success while fidelity remains open cannot qualify an E row.
These receipts do not replace the separate full-gate WF31 receipt.

The engineering index distinguishes completed mathematical discharge from
source-current receipt admission. Its external receipt paths are locators, never
inferred successes: an absent, failed or mismatched artifact is refused even when
the mathematical status is `pass`. Finalize source-bound registry/documentation
changes before qualification and keep that checkout frozen during execution.
Record the final receipt identity and admission outcome in the separately hashed
mutable `qualification/status.json`, rather than rewriting source-bound metadata
after its own receipt was produced.

V obligations require declared Verus-profile symbols and full WF05 receipts;
M obligations require qualified `Module!Theorem` declarations and full WF06/WF07
receipts. Composition rows require full WF07/WF31 receipts. Bare status labels
cannot qualify these rows. Full producers remain responsible for validating
their complete named obligations, not merely finding declaration text.

The consumer independently compares exact source scope/commit/dirty state,
Cargo/tool locks, target/features, fixtures/assumption registries, gate definition
and playbook pin. It verifies retained artifact and raw-log SHA256/lengths and
refuses missing or altered files, stale source, unknown schema, scoped smoke
passes, and missing required evidence. The status index must remain unchanged
while aggregation reads it. There is no waiver switch. Reused receipts are
currently refused conservatively: no undocumented source-closure equivalence or
documentation-only reuse is inferred. A successful unit command cannot renew a
stale persistence or proof receipt.

The active contract is `handovers/revision-5/writfold/`. Gate definitions bind
its complete bundled bytes plus the current adoption record; fixture identity
binds its registries, references, examples and qualification fixtures explicitly.
Aggregation checks that the candidate carries those current identities, so
old receipts cannot qualify the new contract even if manually relabelled.
The old `handover/`, `handovers/revision-4/writfold/`, historical manifests and
frozen inventories remain unchanged. `baseline` reconciles those historical
inputs separately from the active contract; it does not pass a product gate.
Earlier P00/P01/P02 completions and revision-4 ledger snapshots remain retained;
active stage statuses separately describe revision-5 obligations.

The original runner implementation handoff did not execute integration checks.
Later source-bound attempts are indexed separately in
[`qualification/revision-4-progress.json`](../../qualification/revision-4-progress.json).
Run qualification only after concurrent source edits finish; neither this
documentation nor a checkpoint commit renews earlier receipts.
