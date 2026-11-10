#!/usr/bin/env python3
"""Write prompts/<task>-<arm>.txt and notes.json.

Task text is identical in both arms; only the tool rule and the tool notes
differ. Arm B's search note is docs/tuft/agent-note.md's fenced text verbatim;
arm A's grep note has exactly the same byte length (placeholders unfilled).
Edit tasks add one edit note per arm, again of equal byte length: arm B edits
through the Writfold trial flow, arm A with ordinary tools.
<trial>, <commit> and <writfold> are filled per trial by scripts/setup.sh."""
import hashlib, json, pathlib, re

root = pathlib.Path(__file__).resolve().parents[1]
TUFT_NOTE = re.search(r"```text\n(.*?)```", (root / "../../docs/tuft/agent-note.md").read_text(), re.S).group(1)

GREP_NOTE = """Search with grep. For example, from <trial>/repo:
  git grep -n -F "exact bytes" -- crates/ > <trial>/work/hits.txt
  git grep -c -F "exact bytes" -- crates/    (lines per file)
  wc -l < <trial>/work/hits.txt              (total; must equal -c sum)
(`-E` for regex, `-w` for whole words; omit `-- crates/` for all.)
Rows are: `path:line:text`. Write long results to a file.
Long tool output is truncated: if a listing looks cut, count it with -c or
wc -l and read it in parts. Before editing, re-read the exact lines you will
change; edit by their exact text, never by an earlier line number.
"""

SPL_NOTE = """Edit through Writfold. Write a Splicery recipe <trial>/work/e.spl:
  edit "path/in/repo.rs" {
    @x := one text "exact old bytes"
    replace @x with "new bytes"
  }
(`one` refuses unless the text occurs exactly once in the file.) Then run:
  export WRITFOLD_STATE=<trial>/state
  <writfold> trial prepare --repo <trial>/repo --file path/in/repo.rs --recipe <trial>/work/e.spl
  <writfold> trial approve RUN    (RUN is the run id prepare prints)
  <writfold> trial apply RUN
This edits a worktree at <trial>/state/worktrees/RUN, not the checkout.
Bring the change into the checkout as a patch:
  cd <trial>/state/worktrees/RUN && git diff > <trial>/work/e.patch
  cd <trial>/repo && git apply <trial>/work/e.patch
If a step refuses, read its message, fix the cause and redo that step.
"""

EDIT_NOTE = """Edit with your ordinary tools, directly in <trial>/repo.
First locate the definition by search and read the lines around it. Then,
immediately before changing the file, Read it again: what you saw earlier may
be stale. Use the Edit tool with an old_string copied from that fresh read,
long enough to occur exactly once in the file (Edit refuses otherwise, and
also refuses if the file changed since your last Read: then Read it again
and retry). Do not rewrite the whole file, do not use sed -i or a script to
edit, and never edit by a line number taken from earlier output.
Afterwards check the result:
  cd <trial>/repo && git diff
It must show your one-line change, plus any change another process made,
which you must keep. If it shows anything else, fix that before you finish it.
"""

TASKS = {
 "T1": "List every line under `crates/` that calls a function or method named exactly `usable` (that is, `usable(` where `usable` is the whole name). Names that merely end in `usable`, such as `reusable` or `gate_usable`, do not count.",
 "T2": "List every line under `crates/` that calls `as_digest(`, leaving out: lines that define or declare it (`fn as_digest`), comment lines (whose first non-blank characters are `//`), and every file under a directory named `tests` or `examples`.",
 "T3": "List every line under `crates/` that contains `.to_owned()`.",
 "T4": "List every line under `crates/` that calls the `.expect(` method (for example `value.expect(\"message\")`).",
 "T5": "Tuft refuses a program whose `in` scope (the path-prefix string written in the program) is longer than a fixed number of bytes. Raise that limit to `2_048` bytes by changing the value of the constant that defines it. Change only that one definition line.",
 "T6": "The JSON-RPC transport refuses any request frame larger than a fixed byte ceiling. Raise that request ceiling to `131_072` bytes by changing the value of the constant that defines it. Change only that one definition line; leave every other limit (for example the response limit) as it is.",
 "T7": "List every line under `crates/` that contains `thread::sleep(` and does not contain exactly `thread::sleep(Duration::from_millis(1))`.",
 "T8": "Where is the constant `WORKTRUNK_BYTES` defined? List the line that defines it.",
}
FIND = ["T1", "T2", "T3", "T4", "T7", "T8"]
EDIT = ["T5", "T6"]

HEAD = "You are working in a Git repository checked out at <trial>/repo, at commit <commit>.\n\nTask: {task}\n\n"
ANSWER = ("Answer: write the answer to <trial>/answer.txt, one line per piece of evidence, in the form\n"
          "  <path>:<line>: <the full text of that source line, exactly as in the file>\n"
          "where <path> is relative to the repository root and <line> is 1-based. Include every line the task asks for and nothing else. Leading indentation may be omitted. If no line qualifies, write the single line `NONE`.\n\n"
          "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")
EDIT_HEAD = ("Make the change in the checkout <trial>/repo; its final state is graded with `git diff`. "
             "Another process may also be editing files in this checkout while you work; keep any change it makes.\n\n"
             "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")


def indent(text):
    return "".join("  " + l + "\n" for l in text.rstrip("\n").split("\n"))


SEARCH = {
 "A": "- Use your ordinary tools: Bash (grep, rg, git grep, sed and similar) and Read.{mod}\n" + indent(GREP_NOTE),
 "B": ("- Search the repository with Tuft, as the note below says. Do not use grep, rg, git grep, find or other search commands on the repository; you may post-process Tuft's own output (for example the --out file) with ordinary shell tools. Read a file directly (Read, or sed on a line range) only when the Tuft output is not enough.{mod}\n"
       + indent(TUFT_NOTE)),
}
MOD_FIND = " Do not modify the repository."
MOD_EDIT = {"A": " Make the edit as the edit note below says.",
            "B": " Do not modify files in the repository directly (no Edit, Write, sed -i or similar on them); make the edit only through Writfold, as the edit note below says."}
EDITS = {"A": "- Edit note:\n" + indent(EDIT_NOTE), "B": "- Edit note:\n" + indent(SPL_NOTE)}
TAIL_FIND = ("- Do not read anything else outside <trial> except where stated above.\n"
             "- When finished, reply with exactly one line: `DONE`, or `INCOMPLETE: <reason>` if you could not establish a complete answer.\n")
TAIL_EDIT = ("- Do not read anything else outside <trial> except where stated above.\n"
             "- When finished, reply with exactly one line: `DONE`, or `REFUSED: <reason>` if you made no change.\n")


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def main():
    assert len(GREP_NOTE.encode()) == len(TUFT_NOTE.encode()), (len(GREP_NOTE.encode()), len(TUFT_NOTE.encode()))
    assert len(EDIT_NOTE.encode()) == len(SPL_NOTE.encode()), (len(EDIT_NOTE.encode()), len(SPL_NOTE.encode()))
    out = root / "prompts"
    out.mkdir(exist_ok=True)
    for t, task in TASKS.items():
        for a in "AB":
            if t in FIND:
                body = HEAD.format(task=task) + ANSWER + SEARCH[a].format(mod=MOD_FIND) + TAIL_FIND
            else:
                body = HEAD.format(task=task) + EDIT_HEAD + SEARCH[a].format(mod=MOD_EDIT[a]) + EDITS[a] + TAIL_EDIT
            (out / f"{t}-{a}.txt").write_text(body)
    notes = {"search": {"A_grep_note": {"bytes": len(GREP_NOTE.encode()), "sha256": sha(GREP_NOTE)},
                        "B_tuft_note": {"bytes": len(TUFT_NOTE.encode()), "sha256": sha(TUFT_NOTE),
                                        "source": "docs/tuft/agent-note.md fenced text, verbatim"}},
             "edit": {"A_edit_note": {"bytes": len(EDIT_NOTE.encode()), "sha256": sha(EDIT_NOTE)},
                      "B_writfold_edit_note": {"bytes": len(SPL_NOTE.encode()), "sha256": sha(SPL_NOTE)}},
             "texts": {"A_grep_note": GREP_NOTE, "B_tuft_note": TUFT_NOTE, "A_edit_note": EDIT_NOTE,
                       "B_writfold_edit_note": SPL_NOTE}}
    (root / "notes.json").write_text(json.dumps(notes, indent=1) + "\n")
    print(json.dumps({k: v for k, v in notes.items() if k != "texts"}, indent=1))


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "lengths":
        print(len(GREP_NOTE.encode()), len(TUFT_NOTE.encode()), len(EDIT_NOTE.encode()), len(SPL_NOTE.encode()))
    else:
        main()
