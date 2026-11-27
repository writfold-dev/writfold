#!/usr/bin/env python3
"""Every translated function is linked to the calculus, declared out, or on
a backlog that only shrinks (#206, docs/rfcs/2026-11-27-calculus-in-lean.md).

The universe is every function of the translation (proofs/Extracted), loop
bodies and trait instances excluded. `proofs/laws.toml` registers:

  [[word]]  type, core, abstraction, onto   a Rust word type and its core word
  [[local]] type, why                       a type with no core word
  [[link]]  function, implements, theorem, laws
  [[out]]   function, why                   a function the calculus is not about

`proofs/unlinked.txt` is the backlog: functions (and, as `type:<name>`,
enums) not yet registered. Its first line, `# entries: N`, must equal the
number of entries, so growing it is a visible edit of the ceiling.

Refused: a function in none or in two of linked, out and backlog; a
backlog entry that is registered or no longer exists; a link whose theorem
is not a required claim, is not stated over its function or does not name
its `implements` constant; a law that is not a calculus theorem naming
that constant; a function whose result is a registered word declared out;
an enum neither a word, local nor on the backlog; a word whose abstraction
or `_onto` theorem is missing. It reads text files only: tested, not proved.

    python3 proofs/law_coverage.py [PROOFS_DIR]
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

DEF = re.compile(r"^def\s+(?P<name>[\w.']+)", re.M)
INDUCTIVE = re.compile(r"^inductive\s+(?P<name>[\w.']+)\s+where", re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)
CEILING = re.compile(r"^# entries: (\d+)\s*$")


def functions(extracted: Path) -> dict[str, str]:
    """Each translated function and the head of its result type."""
    out = {}
    for path in sorted((extracted / "WritfoldCore").glob("*.lean")):
        if path.name == "Types.lean":
            continue
        text = path.read_text()
        for match in DEF.finditer(text):
            name = match.group("name")
            if ".Insts." in name or "_loop" in name or name.endswith(".body"):
                continue
            signature = text[match.end():text.find(":=", match.end())]
            result = re.findall(r"Result\s+\(?([\w.]+)", signature)
            out[name] = result[-1] if result else ""
    return out


def enums(extracted: Path) -> set[str]:
    return {m.group("name") for m in INDUCTIVE.finditer((extracted / "WritfoldCore" / "Types.lean").read_text())}


def theorems(directory: Path) -> dict[str, str]:
    """Fully qualified theorem name to its statement (up to `:=`), from
    Lean sources, following nested `namespace`/`end`."""
    out = {}
    for path in sorted(directory.rglob("*.lean")):
        stack: list[str] = []
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if match := NAMESPACE.match(line):
                stack.append(match.group("name"))
            elif line.startswith("end ") and stack:
                stack.pop()
            elif line.startswith("theorem "):
                name = line.split()[1]
                text = "\n".join(lines[index:index + 40])
                statement = text[len("theorem ") + len(name):text.find(":=")]
                out[".".join([*stack, name])] = statement
    return out


def required(audit: Path) -> set[str]:
    text = audit.read_text()
    start = text.index("def claims : List Name := [")
    return set(re.findall(r"``([\w.']+)", text[start:text.index("\n]", start)]))


def backlog(path: Path) -> tuple[list[str], list[str]]:
    lines = path.read_text().splitlines()
    errors = []
    ceiling = CEILING.match(lines[0]) if lines else None
    entries = [line.strip() for line in lines if line.strip() and not line.startswith("#")]
    if ceiling is None:
        errors.append(f"{path.name}: the first line must be '# entries: N'")
    elif int(ceiling.group(1)) != len(entries):
        errors.append(f"{path.name}: '# entries: {ceiling.group(1)}' but {len(entries)} entries; "
                      "the backlog only shrinks: lower the ceiling with the line you remove")
    return entries, errors


def check(proofs: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    extracted = proofs / "Extracted"
    universe = functions(extracted)
    types = enums(extracted)
    register = tomllib.loads((proofs / "laws.toml").read_text())
    words = {w["type"]: w for w in register.get("word", [])}
    local = {t["type"] for t in register.get("local", [])}
    links = {l["function"]: l for l in register.get("link", [])}
    outs = {o["function"]: o for o in register.get("out", [])}
    pending, problems = backlog(proofs / "unlinked.txt")
    errors += problems
    pending_fns = {p for p in pending if not p.startswith("type:")}
    pending_types = {p[5:] for p in pending if p.startswith("type:")}
    claims = required(proofs / "Audit.lean")
    stated = theorems(proofs / "Writfold")
    laws = theorems(proofs / "Calculus")
    conform_defs = "\n".join(p.read_text() for p in (proofs / "Writfold" / "Conform").glob("*.lean"))

    for name in sorted(universe):
        places = [k for k, s in (("linked", links), ("out", outs), ("backlog", pending_fns)) if name in s]
        if not places:
            errors.append(f"{name}: translated, but neither linked, out nor on the backlog")
        elif len(places) > 1:
            errors.append(f"{name}: in more than one of {places}")
    for name in sorted((set(links) | set(outs) | pending_fns) - set(universe)):
        errors.append(f"{name}: registered or on the backlog, but not a translated function")
    for name, link in sorted(links.items()):
        theorem, implements = link.get("theorem", ""), link.get("implements", "")
        if theorem not in claims:
            errors.append(f"{name}: link theorem {theorem} is not a required claim")
        statement = stated.get(theorem)
        if statement is None:
            errors.append(f"{name}: link theorem {theorem} not found under proofs/Writfold")
        else:
            if not re.search(rf"(^|[\s(]){re.escape(name)}(\s|$)", statement):
                errors.append(f"{name}: link theorem {theorem} is not stated over {name}")
            if implements not in statement:
                errors.append(f"{name}: link theorem {theorem} does not name {implements}")
        short = implements.rsplit(".", 1)[-1]
        for law in link.get("laws", []):
            if law not in laws:
                errors.append(f"{name}: law {law} is not a theorem under proofs/Calculus")
            elif short not in laws[law]:
                errors.append(f"{name}: law {law} does not mention {implements}")
    for name in sorted(outs):
        if universe.get(name) in words:
            errors.append(f"{name}: returns the word {universe[name]}, so it may not be declared out")
    for name in sorted(types):
        if name not in words and name not in local and name not in pending_types:
            errors.append(f"type {name}: neither a [[word]], a [[local]] nor on the backlog")
    for name in sorted(pending_types & (set(words) | local)):
        errors.append(f"type {name}: registered, so it is no longer on the backlog")
    for name in sorted(pending_types - types):
        errors.append(f"type {name}: on the backlog, but not an enum of the translation")
    for name, word in sorted(words.items()):
        abstraction = word.get("abstraction", "")
        if not re.search(rf"^def {re.escape(abstraction)}\b", conform_defs, re.M):
            errors.append(f"word {name}: abstraction {abstraction} is not defined in proofs/Writfold/Conform")
        if f"Writfold.Core.Conform.{word.get('onto', '')}" not in stated:
            errors.append(f"word {name}: no onto theorem {word.get('onto')}")
    remainder = len(pending_fns & set(universe))
    line = (f"LAW_COVERAGE judged={len(links)} declared_out={len(outs)} "
            f"remainder={remainder} of {len(universe)}")
    return errors, line


def main() -> int:
    proofs = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
    errors, line = check(proofs)
    for error in errors:
        print(f"law_coverage: {error}")
    print(line)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
