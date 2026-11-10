#!/usr/bin/env python3
"""Write prompts/<task>-<arm>.txt and notes.json for R2.

Adapted from the reliability pilot's prompts.py. Task text is identical in
both arms; only the tool rule and the tool notes differ, and each pair of
notes has equal byte length (asserted).

- Search notes: the pilot's notes with the example scope `crates/` changed to
  this corpus's package directory `httpx/` (in both arms; the Tuft note is
  otherwise docs/tuft/agent-note.md verbatim). Arm A's note gets three filler
  bytes to stay equal in length.
- Edit notes: arm B's now says to copy the printed `next:` line (which carries
  an absolute --state, base commit 044d475) instead of exporting a variable,
  that a recipe edits one file (one prepare per file), and that the worktree
  is made from the checkout's HEAD commit. Arm A's is the pilot's, with "your
  one-line change" made "your change", padded to equal length.
<trial>, <commit> and <writfold> are filled per trial by scripts/setup.sh."""
import hashlib, json, pathlib, re, sys

here = pathlib.Path(__file__).resolve().parent
root = here.parent
sys.path.insert(0, str(here))
from tasks import TASKS  # noqa: E402

TUFT_NOTE = re.search(r"```text\n(.*?)```", (root / "../../docs/tuft/agent-note.md").read_text(), re.S).group(1)
assert TUFT_NOTE.count('in "crates/"') == 1
TUFT_NOTE = TUFT_NOTE.replace('in "crates/"', 'in "httpx/"')

GREP_NOTE = """Search with grep. For example, from <trial>/repo:
  git grep -n -F "exact bytes" -- httpx/ > <trial>/work/hits.txt
  git grep -c -F "exact bytes" -- httpx/     (lines per file)
  wc -l < <trial>/work/hits.txt              (total; must equal -c sum)
(`-E` for regex, `-w` for whole words; omit `-- httpx/` for all.)
Rows are: `path:line:text`. Write long results to a file.
Long tool output is truncated: if a listing looks cut, count it with -c or
wc -l and read it in parts. Before editing, re-read the exact lines you will
change; edit by their exact text, never by an earlier line number.
"""

SPL_NOTE = """Edit through Writfold. A recipe edits one file. Write <trial>/work/e.spl:
  edit "path/in/repo.py" {
    @x := one text "exact old bytes"
    replace @x with "new bytes"
  }
(`one` refuses unless the text occurs exactly once in the file.) Then run:
  <writfold> trial prepare --repo <trial>/repo --file path/in/repo.py --recipe <trial>/work/e.spl --state <trial>/state
Each step prints a `next:` line (approve, then apply): run it as printed,
with `writfold` replaced by <writfold>. It carries --state.
This edits a worktree at <trial>/state/worktrees/RUN made from the HEAD
commit, not the checkout. Bring the change into the checkout as a patch:
  cd <trial>/state/worktrees/RUN && git diff > <trial>/work/e.patch
  cd <trial>/repo && git apply <trial>/work/e.patch
Several files: one recipe and one prepare per file.
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
It must show your change, plus any change another process made,
which you must keep. If it shows anything else, fix that before you finish.
"""


def pad(text, target):
    """Pad arm A's note with spaces before its final newline (asserted >= 0)."""
    n = target - len(text.encode())
    assert n >= 0, (n, text[:40])
    return text[:-1] + " " * n + "\n"


GREP_NOTE = pad(GREP_NOTE, len(TUFT_NOTE.encode()))
EDIT_NOTE = pad(EDIT_NOTE, len(SPL_NOTE.encode()))

HEAD = "You are working in a Git repository checked out at <trial>/repo, at commit <commit>.\n\nTask: {task}\n\n"
ROWS = ("one line per piece of evidence, in the form\n"
        "  {prefix}<path>:<line>: <the full text of that source line, exactly as in the file>\n"
        "where <path> is relative to the repository root and <line> is 1-based. Leading indentation may be omitted.")
ANSWER = ("Answer: write the answer to <trial>/answer.txt, " + ROWS.format(prefix="") +
          " Include every line the task asks for and nothing else. If no line qualifies, write the single line `NONE`.\n\n"
          "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")
EDIT_HEAD = ("Make the change in the checkout <trial>/repo; its final state is graded with `git diff`. "
             "Another process may also be editing files in this checkout while you work; keep any change it makes.\n\n"
             "Rules:\n- Put any scratch files in <trial>/work, never in the repository.\n")
SESSION_HEAD = ("Write the step results to <trial>/answer.txt, " + ROWS.format(prefix="S<n> ") +
                " Make the step-4 change in the checkout <trial>/repo; its final state is graded with `git diff`. "
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
TAIL_SESSION = ("- Do not read anything else outside <trial> except where stated above.\n"
                "- When finished, reply with exactly one line: `DONE`, `INCOMPLETE: <reason>` if you could not complete every step, or `REFUSED: <reason>` if you made no change.\n")


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def build(t, a):
    kind, task = TASKS[t]["kind"], TASKS[t]["text"]
    if kind == "find":
        return HEAD.format(task=task) + ANSWER + SEARCH[a].format(mod=MOD_FIND) + TAIL_FIND
    if kind == "edit":
        return HEAD.format(task=task) + EDIT_HEAD + SEARCH[a].format(mod=MOD_EDIT[a]) + EDITS[a] + TAIL_EDIT
    return HEAD.format(task=task) + SESSION_HEAD + SEARCH[a].format(mod=MOD_EDIT[a]) + EDITS[a] + TAIL_SESSION


def main():
    assert len(GREP_NOTE.encode()) == len(TUFT_NOTE.encode())
    assert len(EDIT_NOTE.encode()) == len(SPL_NOTE.encode())
    out = root / "prompts"
    out.mkdir(exist_ok=True)
    for t in TASKS:
        for a in "AB":
            (out / f"{t}-{a}.txt").write_text(build(t, a))
    notes = {"search": {"A_grep_note": {"bytes": len(GREP_NOTE.encode()), "sha256": sha(GREP_NOTE)},
                        "B_tuft_note": {"bytes": len(TUFT_NOTE.encode()), "sha256": sha(TUFT_NOTE),
                                        "source": "docs/tuft/agent-note.md fenced text, with `crates/` -> `httpx/`"}},
             "edit": {"A_edit_note": {"bytes": len(EDIT_NOTE.encode()), "sha256": sha(EDIT_NOTE)},
                      "B_writfold_edit_note": {"bytes": len(SPL_NOTE.encode()), "sha256": sha(SPL_NOTE)}},
             "texts": {"A_grep_note": GREP_NOTE, "B_tuft_note": TUFT_NOTE, "A_edit_note": EDIT_NOTE,
                       "B_writfold_edit_note": SPL_NOTE}}
    (root / "notes.json").write_text(json.dumps(notes, indent=1) + "\n")
    print(json.dumps({k: v for k, v in notes.items() if k != "texts"}, indent=1))


if __name__ == "__main__":
    main()
