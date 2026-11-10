#!/usr/bin/env python3
"""Write prompts/<task>-<arm>.txt. Arm B's tool note is docs/tuft/agent-note.md's
fenced text verbatim; <trial>, <commit> and <writfold> are filled per trial."""
import pathlib, re
root = pathlib.Path(__file__).resolve().parents[1]
note = re.search(r"```text\n(.*?)```", (root / "../../docs/tuft/agent-note.md").read_text(), re.S).group(1)
TASKS = {
 "P1": "Where is the function `claim_matches_spec` called? List every line under `crates/` that calls `claim_matches_spec(`. Do not list the line that defines it.",
 "P2": "Which files under `crates/` mention `serde_json::`? For each such file, list only its first line (lowest line number) that contains `serde_json::`. Exactly one line per file, for every such file.",
 "P3": "Under `crates/`, the function `decode_pin` is called many times, sometimes qualified (for example `dolt::decode_pin(`). List only the calls whose first argument is a `&changed(...)` expression. For each such call, give the line that contains `decode_pin(`: when a call spans several lines, that is the line where the call opens, not the line with `&changed(`. Do not list definitions or calls with any other first argument.",
 "P4": "Consider every line under `crates/` that contains `.to_owned()`. Order those lines by path (byte order, as `git ls-files` sorts paths) and then by line number. List only the last 6 lines in that order.",
 "P5": "List every `pub enum` declaration under `crates/` whose enum name ends in `Error` (an enum named exactly `Error` counts). Give the declaration line of each.",
 "P6": "Where is the constant `PIN_BYTES` defined? List the line that defines it.",
}
HEAD = "You are working in a Git repository checked out at <trial>/repo, at commit <commit>.\n\nTask: {task}\n\n"
ANSWER = ("Answer: write the answer to <trial>/answer.txt, one line per piece of evidence, in the form\n"
          "  <path>:<line>: <the full text of that source line, exactly as in the file>\n"
          "where <path> is relative to the repository root and <line> is 1-based. Include every line the task asks for and nothing else. Leading indentation may be omitted.\n\n"
          "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")
ARM = {
 "A": "- Use your ordinary tools: Bash (grep, rg, git grep, sed and similar) and Read. Do not modify the repository.\n",
 "B": ("- Search the repository with Tuft, as the note below says. Do not use grep, rg, git grep, find or other search commands on the repository; you may post-process Tuft's own output (for example the --out file) with ordinary shell tools. Read a file directly (Read, or sed on a line range) only when the Tuft output is not enough. Do not modify the repository.\n"
       + "".join("  " + l + "\n" for l in note.rstrip("\n").split("\n"))),
}
TAIL = "- Do not read anything else outside <trial> except where stated above.\n- When finished, reply with exactly one line: `DONE`.\n"
for t, task in TASKS.items():
    for a in "AB":
        (root / "prompts" / f"{t}-{a}.txt").write_text(HEAD.format(task=task) + ANSWER + ARM[a] + TAIL)
