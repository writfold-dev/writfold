# Trial profile: prepare, approve, apply

`writfold-trial-worktree/1` is the first connected path through Writfold's edit
flow. It is a **cooperating single-user trial**, chosen by the owner on
8 November 2026 so people can try the flow on ordinary repositories. It is not
the hardened alpha worktree path (pinned Worktrunk, loose-object repositories
only), and it does not change that path's requirements.

## Try it

```sh
cargo build --release -p writfold
export WRITFOLD_STATE=/tmp/writfold-trial      # any new directory
target/release/writfold trial init
target/release/writfold trial retain --repo /path/to/your/repo \
    --commit <full-sha> --file src/client.rs --bytes 0..120   # optional evidence
target/release/writfold trial prepare --repo /path/to/your/repo \
    --file src/client.rs --recipe add-retry.spl
target/release/writfold trial approve <run> [--require <weft-check>]...
target/release/writfold trial apply <run>
target/release/writfold trial show <run>
target/release/writfold trial recover <run>   # only after an interrupted apply
git -C "$WRITFOLD_STATE/worktrees/<run>" diff
```

Recipes are [Splicery](https://github.com/writfold-dev/splicery) recipes; see its
`examples/try/` for one that works on a small Rust file. The pinned Splicery
(`b14c67a`) accepts `whole @line` (a line with its terminator) and
`from @a to @b` (both captures and everything between); replay refusals carry
its bounded selector detail (match count and sites) in the error text.
Splicery's own `--diff`, stdin-recipe and `--out` CLI options are not part of
this path: Writfold replays recipe files through the library, not the CLI.

## One command

`writfold trial run` goes from evidence to a prepared edit and stops at
approval:

```sh
writfold trial run --repo DIR --file PATH --recipe EDIT.spl \
    [--tuft QUERY.tuft] [--weft REVIEW.weft --weft-bin PATH] [--state DIR]
```

It runs, in order, exactly what the separate commands run:

1. `tuft`: the Tuft program, as `writfold tuft run` does; `retain` keeps its
   hits as `writfold-citation/1` bindings in this state.
2. `weft` (with `--weft`): the review source, after template substitution
   (below), is retained and written to `$WRITFOLD_STATE/weft/<sha256>.weft`;
   the binary named by `--weft-bin` runs `prove --json -- <that file>`, and its
   `weft.report/1` is written next to it.
3. `prepare`: exactly `trial prepare --report` with that report (no report
   without `--weft`).

The first failure stops the run and names its step (`step weft refused: ...`).
A refused `prepare` leaves no worktree, as it always does. `approve`, `apply`
and `recover` stay separate commands.

**Citation template.** Inside the Weft source given to `trial run` only,
`citation "tuft:@VIEW#N"` stands for the N-th row (from 1) of the Tuft view
`@VIEW`, in projected order before the emit budget pages it. Writfold replaces
the string with that row's exact binding digest, so Weft reads ordinary
`asset x = citation "<digest>";` source; the substituted source is what Weft
reads and what is retained. **This is a Writfold-side template, not Weft
syntax.** Every `"tuft:` in the source must be such a template directly after
`citation` and must resolve to a retained row; a wrong view name, a row out of
range, a view that does not `retain`, or a template anywhere else (a comment
included) refuses before Weft runs.

**Weft binary.** Writfold does not bundle Weft. `--weft-bin` is required with
`--weft`, is canonicalized and run by path (never looked up on `PATH`), and its
sha256, read just before it runs, is printed. Weft's exit status is not read
as a verdict: exits 0–2 hand the output to the same report check `prepare`
uses, which refuses anything but a complete local report; another status, a
signal or 30 s without finishing refuses.

Example (a three-line repository, the real Weft binary):

```text
$ cat q.tuft
@h := find text "open(url)" at "2fc202c137e58f5fbfe6d20ac522595e3f16721b";
@e := retain @h around lines 0;
@v := project @e [path, line, binding];
emit @v;
$ grep citation review.weft
    asset call_site = citation "tuft:@v#1";
$ writfold trial run --repo repo --file src/client.rs --recipe edit.spl \
    --tuft q.tuft --weft review.weft --weft-bin "$WEFT"
tuft @v retained=1 coverage=complete files=1/1 hits=1 shown=1 omitted=0 cursor=none program=fcf220f04fdd
  "tuft:@v#1" = af572aad857cd76cf36983ea58e32ab55136afe87e6077ce24e867ee20c28880 (src/client.rs:2)
weft bin sha256:345ea488… source sha256:d618877d… (results are hypothetical)
  check retry_safe: entailed
  check bounded: undetermined
prepared run run.1794273538.214907
  file src/client.rs at 2fc202c137e58f5fbfe6d20ac522595e3f16721b
  prepared edit wfwt1:prepared-edit:sha256:bac75e9a…
next: writfold trial approve run.1794273538.214907 --require retry_safe
$ writfold trial approve run.1794273538.214907 --require bounded
writfold: approval refused: required check "bounded" is not entailed
```

The suggested `--require` lists the checks Weft reported `entailed`; choose
the ones the edit actually needs.

With a stub Weft on the latency-bench repositories
([`bench/2026-11-10-trial-run-latency`](../../bench/2026-11-10-trial-run-latency/)),
`trial run` takes a median of 188 ms (small) and 319 ms (100,000 files),
within 10 ms of its parts run one by one. Tests
(`crates/writfold-app/src/trial/run/`) cover template substitution against the
retained binding, unresolved templates refused before Weft runs, each step's
named refusal leaving no worktree, and a refuted check making a later
`approve --require` refuse.

## What each step does

| Step | Effect | Ledger record |
|---|---|---|
| `init` | Creates the private state directory: an Ed25519 operator key, a bootstrap policy trusting it, the Git executable's observed digest, the ledger repository, the object store | — |
| `prepare` | Adds a detached, **sparse** worktree of your repository's **HEAD commit** inside the state directory (only the edited file is checked out; everything else is skip-worktree, so `git status`/`diff`/commits behave normally and cost does not grow with repository size), replays the recipe through the pinned Splicery kernel, builds and validates the exact `writfold-worktree/1` prepared edit, retains source, recipe, candidate and report. Refuses a report citing evidence (`asset` bindings) that `retain` did not keep, or whose retained bytes no longer match their digest. Refuses a candidate identical to its source, and a `--report` that is not a complete `weft.report/1` claiming no grounding or effect authority | `Prepared` |
| `retain` | Reads the exact bytes of a file at a **full pinned commit id** (refs, abbreviations and `HEAD` are refused), keeps the selected byte range in the state directory and binds it as a `writfold-citation/1` descriptor. Prints the binding digest; a Weft report cites it as `asset x = citation "<digest>";`. Equal text at two positions is two different bindings | — |
| `evidence list` | Lists retained bindings as compact rows `binding12 path@commit7:line: text` (`--path PREFIX`, `--commit SHA`, `--limit N`, `--out NEW_FILE`), then a `# count=M shown=S omitted=O` trailer like Tuft's; entries whose descriptor or bytes fail verification are counted as `refused=R` and not shown | — |
| `evidence show` | Prints one binding's **exact retained bytes**, verified first against its descriptor (re-fingerprint and selection digest; tampered evidence refuses). `BINDING` is the full digest, a unique 8+ hex prefix or a label. `--context-lines N --repo DIR` reads the pinned blob, checks its whole-file digest and the bytes at the retained range, and prints context in separately marked `L| text` sections, never inside the retained bytes. Writes nothing | — |
| `approve` | With `--require CHECK` (repeatable), first reads the retained Weft report and refuses unless every named check is present and `entailed` (the proved `required_entailed`). Then signs an operator grant (Write on this run's worktree, one hour) and a prepared-edit approval binding the prepared-edit digest, report, procedure, resource generation and validity | `Approved` |
| `apply` | Verifies both signatures strictly under the policy, every bound value and the validity window, and asks the kernel for a current grant covering Write; records a durable claim; checks the worktree file still holds the retained source bytes; installs the candidate through the staged installer | `Claimed`, then `Applied`, or `Abandoned` if the original was never replaced |
| `recover` | For a claim with no outcome (a crash or error during install): observes the file without rewriting it. The candidate's exact bytes mean it was installed; the source's exact bytes mean it was not; anything else stays Unknown and refuses | `Applied` (reconciled) or `Abandoned` |
| `show` | Replays the run's ledger history through the kernel and prints it with five independent badges: evidence, assessment, approval, claim and effect. There is no aggregate status | — |

Every append goes through the [durable local ledger](semantic-core.md#durable-local-ledger):
the run's history is replayed through the kernel and the edit-flow validator
(exactly `Prepared` → `Approved` → `Claimed` → `Applied` or `Abandoned`, each
carrying its material, with the claim and `Applied` naming the prepared
candidate and an abandonment saying why) before the head moves.

## Reusing evidence in a later session

`retain` and Tuft's `retain` store each binding under
`<state>/evidence/<digest>.{json,bytes}` (plus the start line in
`<digest>.line`, a display hint `show` recomputes from the blob when given
`--repo`). A later session reads them back without searching again:

```sh
writfold trial retain --repo R --commit <sha> --file src/a.rs --bytes 0..40 --label entry
writfold trial evidence list --path src/
# 29aa4846bd82 src/a.rs@81071d8:1: fn connect(url: &str) -> Result<Conn> {
# count=1 shown=1 omitted=0
writfold trial evidence show entry --context-lines 2 --repo R
writfold trial evidence labels
```

A **label** (`retain --label NAME`) is a mutable, local, per-state name kept in
`<state>/labels/`, apart from the immutable evidence. Setting it again moves it.
As the [shared-client rule for scoped aliases](shared-client-references.md)
requires, it is an interaction convenience only: it is never written into a
descriptor or report, and `show` prints the full digest it resolved to. The
same holds for the 12-hex abbreviation in `list` rows. Cite the full digest.
Tuft's `retain` does not take a label yet.

`evidence list/show` are CLI-only: `writfold.trial/1` advertises exactly three
methods through the pinned `contracts/agent-stdio/1/trial-methods.json`, so new
serve methods need a contract revision rather than an in-place addition.
Latency with 1,001 retained bindings on the 100,000-file repository:
[`bench/2026-11-10-evidence-latency`](../../bench/2026-11-10-evidence-latency/README.md).

## Through serve

An agent can run `show`, `retain` and `prepare` over `writfold serve`
(JSON-RPC 2.0 over bounded stdio, profile `writfold.agent/1`) by negotiating the
`writfold.trial/1` capability. Each method calls exactly the function the CLI
calls (`crates/writfold-app/src/trial/{show,retain,prepare}.rs`), so the same
input gets the same decision and the same message.

```sh
writfold serve --state "$WRITFOLD_STATE"     # or set WRITFOLD_STATE and run: writfold serve
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2","capabilities":["writfold.trial/1"]}}
{"jsonrpc":"2.0","id":2,"method":"trial/retain","params":{"repo":"/abs/repo","commit":"<full-sha>","file":"src/client.rs","start":0,"end":120}}
{"jsonrpc":"2.0","id":3,"method":"trial/prepare","params":{"repo":"/abs/repo","file":"src/client.rs","recipe":"/abs/add-retry.spl","report":"/abs/report.json"}}
{"jsonrpc":"2.0","id":4,"method":"trial/show","params":{"run":"run.1794262902.0f9dfb"}}
{"jsonrpc":"2.0","id":5,"method":"shutdown"}
```

The state directory is fixed when the server starts (`--state DIR`, else
`WRITFOLD_STATE`). It is never a request parameter. Without one, `initialize`
requesting `writfold.trial/1` refuses as an unsupported capability. The
capability is admitted only when requested; a session that did not request it
advertises exactly the control methods and `notifications`, as before.

| Method | Parameters (closed; unknown fields refuse) | Result `payload` |
|---|---|---|
| `trial/show` | `run` | `run`, `text`: exactly what `writfold trial show` prints |
| `trial/retain` | `repo` (absolute), `commit` (full id), `file` (relative), `start`, `end` (u64, bytes `[start, end)`) | `binding`, `file`, `commit`, `start`, `end` |
| `trial/prepare` | `repo`, `recipe` (absolute), `file` (relative), optional `report` (absolute) | `run`, `file`, `base`, `prepared`, `splices` (`start`, `end`, `inserted`) |

Results use the shutdown-shaped envelope (`exit_code` 0, `complete`,
`generation`, `session`, `payload`). `prepare` does not report the worktree
path, because the CLI does not print it either.

**Approve, apply and recover stay CLI-only.** There is no `trial/approve`,
`trial/apply` or `trial/recover` method; each is Method not found (-32601) in
every session state. The operator key decides an approval, and an agent session
must not approve its own edit. Apply and recover also stay with the operator's
CLI for now: apply installs into the worktree under the operator's grant, and
recovery decides an Unknown outcome that a person may need to resolve. Moving
either behind `serve` would need its own owner decision and tests.

Refusals are checked in this order, with the existing typed faults:

| Condition | Code | Category | Meaning |
|---|---|---|---|
| `trial/approve`, `trial/apply`, `trial/recover`, any unregistered name | -32601 | malformed | Method not found |
| Trial method before `initialize` (or after `shutdown`) | -32002 | conflict | Initialization required (or shut down) |
| Session did not negotiate `writfold.trial/1` | -32000 | malformed | Unsupported requested capability |
| Sent as a notification | — | — | Not run; nothing is emitted, since it could not report its outcome |
| Unknown, missing or mistyped parameter; relative host path | -32602 | malformed | Invalid params |
| The trial function refused | -32000 | by error type | `data.message` is exactly the text the CLI prints after `writfold: ` |

Trial refusal categories come from the error's type, never its text:
`conflict`/E104 for a ledger refusal or moved head, `not-found`/E101 and
`permission`/E106 for those host errors, `incomplete`/E105 for other Git, IO or
ledger failures and for a result over the 65,536-byte frame budget, and
`malformed`/E102 for the application's own input refusals (bad report, recipe,
path, range or commit). The outcome is `rejected` for `trial/show` and for a
refusal decided before anything was written (the CLI-side marker
`BeforeEffects`, displayed exactly as its cause). A `retain` or `prepare` that
fails after it may have written reports `unknown` with `exit_code` null:
observe with `trial/show` before retrying. An oversized result is refused, never
truncated; for `prepare` the refusal names the run.

Measured through one `serve` session per repository (release host build, WSL2,
10 calls each; [`bench/2026-11-09-serve-latency`](../../bench/2026-11-09-serve-latency/results.json),
`bench/latency/serve.py`): on the 100,000-file repository `trial/retain` p95
27 ms, `trial/prepare` p95 166 ms, `trial/show` p95 53 ms; on the small one
41, 184 and 53 ms. All are within the 1 s p95 target (250 ms for `show`).

The published contract (`crates/writfold-wire/contracts/agent-stdio/1/`) carries
the trial method schemas (`trial-methods.json`), result and refusal schemas, and
state-free negotiation vectors. A fresh WF13 `stdio-control` receipt binds this
contract and the executable Weft's `control` gate now pins (`91a1924f…`); its
vectors exercise negotiation and the CLI-only refusals, while the trial methods
themselves are covered by tests, not a gate receipt.

## What it does not do

- It never writes to your checkout. Uncommitted changes are not included: the
  worktree starts at HEAD. Bring the change back with `git diff` or by
  committing in the worktree.
- It adds worktree metadata to your repository (`git worktree list` shows it).
  A refused `prepare` removes the worktree it created.
- It assumes nobody races it on the worktree and that the repository is not
  hostile; it is not a sandbox. The operator key lives in the state directory.
- A Weft report is optional. Without `--report`, an empty report is retained
  and bound, and `--require` refuses. With one, Weft's results are recorded as
  Weft states them, local and **hypothetical**, never as grounded facts. The
  required check names are enforced at `approve` but not yet carried inside the
  signed approval.
- Recovery's decision, the required-check gate and the edit-flow validator
  are proved over the shipped Rust (`docs/writfold/aeneas.md`); the
  interleavings of apply, recover and an outside writer are model-checked
  (below); the rest of the flow is tested.
- Authority is checked once, immediately before `Claimed`. A claim made under
  a valid approval may finish its install after the approval expires (or a
  grant is later revoked): the claim is the commitment point and is never
  re-dispatched, so that install is the single dispatch the approval
  admitted. Nothing after the claim re-checks authority, and `recover` needs
  none because it never writes the file.
- A claim is never re-dispatched: after an interrupted `apply`, `apply`
  refuses and `recover` decides from the file's exact bytes. If the file holds
  neither the source nor the candidate, the outcome stays Unknown for a person
  to resolve. A leftover staging file from an interrupted install is not
  cleaned up yet.
- The procedure identity is a fixed digest of the profile name, not an
  admitted procedure. Grants are not stored durably beyond their retained
  envelopes, and there is no revocation command yet.

## Evidence

Tests cover the full flow on a real repository, every out-of-order step, a file
changed after approval, an approval under a different policy, an ambiguous
recipe and escaping paths (both leave no worktree), and each authorization
check alone: an untrusted grant signer, an approval for a different prepared
edit, and a grant that does not cover the worktree. Removing the grant
verification, the approval verification, the current-grant check, the file
guard or the validator's material rule each fails a test. Recovery is tested
for all three observations (installed, not installed, neither), plus a claim
that `apply` must not re-dispatch; removing either recovery comparison, the
claim's candidate binding or the abandonment-reason rule fails a test. The
interleavings are model-checked in `spec/TrialEditStream.tla` (TLC, in the
tool image): two apply invocations, recover and an outside writer, a crash at
every step boundary, the ledger's expected-old-head CAS and a clock. At most
one outcome is recorded; `Applied` implies the candidate was installed and
`Abandoned` that it never was; nothing is dispatched twice after `Claimed`;
install only ever replaces the exact source; `Unknown` only on bytes that are
neither; recover never writes the file; every claim was made under valid
authority. Removing the preimage guard, or letting `apply` resume a
three-record run, each yields a counterexample. The model assumes the outside
writer never writes the source's or candidate's exact bytes (a probe shows
recovery would then reconcile wrongly) and never races install's re-check and
rename. Two
checks are defensive only: the explicit "claimed without an outcome" message in
`apply` (the history-shape match already refuses) and the outcome rule that the
third record is the claim (the ledger re-validates the prefix, so nothing else
can be third).
