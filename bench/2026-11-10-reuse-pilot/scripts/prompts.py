#!/usr/bin/env python3
"""Write prompts/S1-<task>.txt (session 1, shared) and prompts/<task>-<arm>.txt
(session 2). Session 1 searches with Tuft (docs/tuft/agent-note.md's fenced
text verbatim) and both retains the evidence and writes plain-text notes. In
session 2 the task text is identical in both arms; only the tool rule and the
handed-over material differ: arm A gets the notes, arm B the retained state."""
import pathlib, re
root = pathlib.Path(__file__).resolve().parents[1]
note = re.search(r"```text\n(.*?)```", (root / "../../docs/tuft/agent-note.md").read_text(), re.S).group(1)
EVIDENCE_NOTE = """Earlier evidence, retained as exact bytes at the pinned commit:
  <writfold> trial evidence list --state <trial>/state [--path PREFIX] [--limit N] [--out NEW_FILE]
Rows are `binding12 path@commit7:line: text` (line = first retained line); the
last line `# count=M shown=S omitted=O` means O rows are not shown.
  <writfold> trial evidence show BINDING --state <trial>/state [--context-lines N --repo <trial>/repo]
prints the exact retained bytes; context lines, numbered `L| text`, come from
the pinned commit in separately marked sections.
"""
S1 = {
 "R1": "Find every line under `crates/` that calls `claim_matches_spec(` (not the line that defines it).",
 "R2": "Find every `pub enum` declaration under `crates/` whose enum name ends in `Error` (an enum named exactly `Error` counts).",
 "R3": "Find the calls of `decode_pin` under `crates/` whose first argument is a `&changed(...)` expression. Some calls span several lines; for each call the evidence is the line that contains `decode_pin(`.",
 "R4": "For each file under `crates/` that mentions `serde_json::`, find its first line (lowest line number) containing `serde_json::`.",
 "R5": "Find every `#[test]` attribute line under `crates/writfold-app/src/trial/`.",
 "R6": "Find every line under `crates/` that mentions `MAX_RESPONSE_BYTES`.",
}
S2 = {
 "R1": "Of the calls found in the earlier session, list those in files under `crates/writfold-kernel/src/checker/`.",
 "R2": "For each enum found in the earlier session that is declared under `crates/writfold-runtime/`, give the line immediately after its declaration line.",
 "R3": "For each call found in the earlier session, give the line that contains its `&changed(` argument: the `decode_pin(` line itself when the argument is on that line, otherwise the later line where `&changed(` appears.",
 "R4": "Of the first lines found in the earlier session, list those whose text contains `serde_json::Value`.",
 "R5": "For each `#[test]` attribute found in the earlier session in `retain_tests.rs` or `recovery_tests.rs`, give the line immediately after it.",
 "R6": "Of the lines found in the earlier session, list those in non-test files: leave out files under any `tests/` directory and files whose name ends in `_tests.rs`.",
}
HEAD = "You are working in a Git repository checked out at <trial>/repo, at commit <commit>.\n\n"
S1_BODY = ("Task: gather evidence for a later agent session, which will not be able to see this conversation.\n{task}\n\n"
           "Do both of these:\n"
           "1. Retain every piece of evidence with Tuft (`retain @h around lines 0`, so each evidence line is its own retained binding in <trial>/state).\n"
           "2. Write plain-text notes to <trial>/notes.txt for that later session: the evidence as lines of the form\n"
           "  <path>:<line>: <the full text of that source line, exactly as in the file>\n"
           "plus anything else you think it will need.\n\n"
           "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n"
           "- Search the repository with Tuft, as the note below says. Do not use grep, rg, git grep, find or other search commands on the repository; you may post-process Tuft's own output with ordinary shell tools. Read a file directly (Read, or sed on a line range) only when the Tuft output is not enough. Do not modify the repository.\n")
S2_BODY = ("An earlier agent session gathered evidence for this task:\n{s1}\n\n"
           "Follow-up task: {task}\n\n")
ANSWER = ("Answer: write the answer to <trial>/answer.txt, one line per piece of evidence, in the form\n"
          "  <path>:<line>: <the full text of that source line, exactly as in the file>\n"
          "where <path> is relative to the repository root and <line> is 1-based. Include every line the task asks for and nothing else. Leading indentation may be omitted.\n\n"
          "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")
ARM = {
 "A": "- The earlier session's plain-text notes are in <trial>/notes.txt. Use them and your ordinary tools: Bash (grep, rg, git grep, sed and similar) and Read. Do not modify the repository.\n",
 "B": ("- The earlier session's evidence is retained in <trial>/state; read it as the note below says. Do not use grep, rg, git grep, find or other search commands on the repository; you may post-process the evidence commands' own output with ordinary shell tools. Read a repository file directly (Read, or sed on a line range) only when the evidence output is not enough. Do not modify the repository.\n"
       + "".join("  " + l + "\n" for l in EVIDENCE_NOTE.rstrip("\n").split("\n"))),
}
NOTE = "".join("  " + l + "\n" for l in note.rstrip("\n").split("\n"))
TAIL = "- Do not read anything else outside <trial> except where stated above.\n- When finished, reply with exactly one line: `DONE`.\n"
out = root / "prompts"
out.mkdir(exist_ok=True)
for t in S1:
    (out / f"S1-{t}.txt").write_text(HEAD + S1_BODY.format(task=S1[t]) + NOTE + TAIL)
    for a in "AB":
        (out / f"{t}-{a}.txt").write_text(HEAD + S2_BODY.format(s1=S1[t], task=S2[t]) + ANSWER + ARM[a] + TAIL)
