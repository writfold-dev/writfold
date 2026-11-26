# The Claude Code adapter

> **Status:** built in v0.6.0-dev.25 ([#150](https://github.com/writfold-dev/writfold/issues/150)). It was tested with Claude Code 2.1.332: offline with a fake harness in the unit gate, and by hand once against the real CLI, on the [agent example](agents.md) and on the [KB-consistency fixture](research.md). Both transcripts are kept in `fixtures/harness/claude-code/`. It is not a part of the binary: Writfold starts no model, and nothing the binary decides depends on the adapter.

`plugins/writfold/adapter/claude_code.py` is the bridge between a package that [`agent prepare`](agents.md#prepare-record-what-the-worker-is-handed) wrote and one run of Claude Code. It needs `python3` (standard library only), the `claude` CLI and `writfold` on `PATH`. It has three commands:

```text
claude_code.py export PKG [--tools MAP.json] [--model MODEL] > agents.json
claude_code.py run PKG --report FILE [--tools MAP.json] [--model MODEL] [--harness CMD]
claude_code.py deliver PKG --mode MODE --report FILE --run RUN [--attest CHECK]
```

## Export: the agent definition

`export` prints a Claude Code agent definition (the JSON `claude --agents` takes) built from the package:
- the prompt is `instructions.md`: the profile's instructions, then each skill's;
- the tools are the profile's Writfold tools, each mapped to Claude Code tools;
- the model is the profile's `model` hint.

The definition names the closure digest and the adapter version. It holds no invocation id, no input and no time, so the same closure and adapter version give the same bytes.

The tool table maps `repo.read` and `kb.read` to `Read`, `Grep` and `Glob`, and `git.diff` to `Bash(git diff:*)`. `--tools MAP.json` adds to it or replaces entries:

```json
{"web.fetch": ["WebFetch"]}
```

`export` refuses (exit 3, nothing printed) rather than drop something silently:

| What | Refusal |
|---|---|
| a key of `invocation.json` the adapter does not know | `invocation.json has keys this adapter (0.1.0) does not know: …; they would be ignored` |
| a Writfold tool with no mapping | `tool <id> has no Claude Code mapping; name one with --tools MAP.json` |
| a `model` hint Claude Code does not take (`opus`, `sonnet`, `haiku`, `inherit` and `claude-…` ids are taken) | `model hint '<hint>' is not a Claude Code model; pass --model MODEL to state the substitution` |

With `--model`, the definition records the substitution (`Stated: model hint 'reviewer' substituted with 'haiku'`). A profile with no hint runs as `inherit`.

## Run: one harness call

`run` calls:

```text
claude -p --agents <definition> --agent <id> --tools <built-in tools> --allowedTools <mapped tools> --model <model> --no-session-persistence --output-format text <prompt>
```

- **The prompt** is the package's `input`, preceded by `packet.md` when the profile requires a packet.
- **The report** is the printed answer, written to `--report`.
- **A failed run:** when the harness exits non-zero, the adapter writes no report and exits 3.
- **Directory:** run it from the directory the worker should read, usually the KB root.
- **`--harness CMD`:** replaces `claude`. The tests use `fixtures/harness/fake-claude.sh`.

## Deliver: back into the run

`deliver` runs three steps:
1. `writfold kb deliver prepare --mode MODE --from REPORT --invocation <id>`;
2. with `--attest CHECK`, `writfold kb deliver attest … --check CHECK --by adapter:claude-code@0.1.0`;
3. `writfold kb deliver check`.

Cover a unit with the candidate as usual (`gate cover … delivery:<candidate>`). The [rehearsal](recipes/claude-code-rehearsal.sh) runs the whole loop. Pass it `claude` and set `ADAPTER_MODEL=haiku` to use the real CLI.

## What the adapter attests, and what it does not see

The only statement it makes is `context.submitted`: *the adapter sent the package of invocation X as the prompt to Claude Code 2.1.332*. It is recorded as attested, never as a passed check. It says nothing about what the model read or did.

The adapter does not see:
- **Which files the worker read, and which tools it called.** Claude Code's own transcript has them, and the adapter keeps no copy.
- **Tool calls the harness refused.**
- **The model the CLI resolved `inherit` or an alias to.**
- **Whether the worker followed its instructions.** `invocation.bound` judges records, and a report that is wrong and well formed still covers its unit.
- **Hooks or settings in the directory it runs in.** These can change what the worker may do. Hooks are a guard, not enforcement.

`--tools` makes each mapped built-in tool available (`Bash` for `Bash(git diff:*)`). `--allowedTools` approves only the mapped patterns. In print mode, the harness refuses a call that needs approval; the adapter cannot make it fail closed beyond that.

## What the hand runs showed

- **Agent example ([transcript](../../fixtures/harness/claude-code/agent-example.transcript.txt)):**
  - `export` refused the hint `reviewer` until `--model haiku` stated the substitution.
  - The report found that the change's claim "keeps the 5 ms default" does not hold, since the diff falls back to 50.
  - `invocation.bound` and `claims.cited-fresh` passed, and `report.read` stayed open for a person.
- **KB-consistency ([transcript](../../fixtures/harness/claude-code/kb-consistency.transcript.txt)):**
  - The worker read both records and reported that they disagree (5 ms against 50 ms).
  - The report covered its unit (1/1).
  - In the mode that asks for `context.submitted`, the adapter's statement made the report ready.
- **A gap found on the first try:** a unit's name is two record ids, and a worker cannot open a record from its id alone. The fixture now gives the worker each member's file in its input (the kept manifest has the paths), and the skill says so.
