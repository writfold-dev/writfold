# Rust development reference host

`writfold-ref` is the Rust development conformance executable in `writfold-wire`.
It implements semantic wire schema 2 and workflow schema 3, both retaining semantic
version `2.0.0`. It is not the production CLI, RPC server, or an authentication adapter.

Build and invoke through the repository's qualified build environment. The Cargo
binary target is `writfold-ref` in package `writfold-wire`. The existing harness can
use its executable path with `scripts/conformance_check.py --lean PATH`; the option
name is historical. That invocation compares Python against the chosen executable,
not three hosts automatically, and does not prove refinement.

## Transport

Standard input and output are JSONL. Blank lines are ignored. Every nonblank input
line produces one complete compact response, including malformed lines; input EOF
ends the process. Diagnostics are not mixed into standard output. The host bounds
each input line to 16 MiB, individual containers to 250,000 entries and the parsed
tree to 1,000,000 nodes, using the wire crate's supported nesting ceiling. Oversized
lines are drained without retaining the excess and receive `invalid_request`.
The semantic formula and witness codecs additionally enforce the baseline 256-node
recursive-depth limit. These are resource bounds, not finite-integer substitutions.

Integer JSON tokens retain arbitrary precision. The raw boundary also parses valid
fractional and exponent tokens, but semantic integer fields reject them, including
`1.0` and `1e0`; booleans are not integers either. Natural-number fields reject
negatives. Duplicate object keys, unpaired Unicode surrogates, unknown variants,
wrong versions and extra or missing fields are rejected.
Ordered arrays remain ordered, including plan assignments and context families.

For example:

```json
{"schema_version":2,"semantic_version":"2.0.0","id":"version-v2","op":"semantic_version"}
{"schema_version":2,"semantic_version":"2.0.0","id":"ttl-inclusive-boundary","op":"fresh","now":110,"observed_at":100,"ttl":10}
```

The complete corresponding responses are:

```json
{"id":"version-v2","ok":true,"result":"2.0.0"}
{"id":"ttl-inclusive-boundary","ok":true,"result":"fresh"}
```

These examples come from `tests/conformance/cases.jsonl`. The Rust reference tests
also consume the retained workflow-3 and malformed-boundary fixture files, comparing
complete response objects rather than incidental source text. Tests are added, not
claimed executed by this implementation change.

## Trusted model environments

Ordinary requests cannot carry approval flags or trusted-environment fields. The
development harness has a separate outer envelope, exactly as the retained Lean
executable and `scripts/conformance_check.py` use:

```json
{"request":{"schema_version":2,"semantic_version":"2.0.0","id":"version-v2","op":"semantic_version"},"trusted_environment":{"approvals":[],"selections":[],"retained":[]}}
```

Approvals wrap exact plans; selections wrap exact selection plans. Retained values
are KBs for schema 2 and portfolio states for schema 3. Duplicate retained snapshot
identities are rejected. The environment is decoded before evaluating the request;
malformed environments return the empty safe ID. Validly parsed ordinary request
errors preserve a string ID, otherwise the ID is empty.

The domain names `ModelApproval`, `ModelSelectionApproval`,
`ModelPortfolioApproval` and `ModelPortfolioSelectionApproval` deliberately identify
reference assumptions. Deserializing them in this development host does not verify
signatures or create production authorization capabilities. Apply, sequence, replay
and forward-selection operations receive these assumptions separately from their
ordinary request records. There is no ambient authority or cross-line mutable state:
stateful scenarios explicitly carry state and journals in subsequent requests, as
the existing harness does.

## Implemented operations

Schema 2: `semantic_version`, `fresh`, `validate`, `check`, `supported`, `assess`,
`usable`, `reuse`, `retrieve`, `admit`, `sequence`, `transition`, `conclude`,
`authored_transition`, `authored_candidate_valid`, `select`, `retrieve_selected`.

Schema 3: `semantic_version`, `workflow_version`, `portfolio_state_valid`,
`portfolio_valid`, `portfolio_admit`, `portfolio_query_allowed`, `portfolio_assess`,
`portfolio_selection_valid`, `portfolio_select`, `design_valid`, `design_transition`,
`design_bind`, `design_reopen`.

The reference dispatchers convert DTOs into concrete domain records and invoke the
Rust kernel; JSON values are confined to codecs and response rendering. This is
executable behavior, not a proof claim. Source-bound Verus qualification and the
full multi-host conformance gate remain separate evidence obligations.
