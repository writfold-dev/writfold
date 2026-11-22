#!/usr/bin/env python3
"""Split Aeneas's single-file translation into one Lean module per kernel
module (#95, docs/rfcs/2026-11-22-split-translation.md).

    python3 proofs/split.py DIR        # DIR/WritfoldCore.lean, as Aeneas wrote it
    python3 proofs/split.py --self-test
    python3 proofs/split.py --imports proofs/Extracted proofs/Writfold/*.lean
                                       # narrow `import Extracted.WritfoldCore` to the modules named

Aeneas writes every declaration of the kernel into one file, so a mutant
anywhere re-elaborates every theorem. This rewrites DIR/WritfoldCore.lean
into DIR/WritfoldCore/<Module>.lean, one per top-level kernel module (a
group of modules if they import each other), each importing only the
modules its declarations name, and leaves DIR/WritfoldCore.lean importing
them all so existing imports keep working.

Obligation 1 of the RFC is checked on every run, before anything is
written: every declaration is kept byte for byte, each module keeps its
declarations in Aeneas's order, and no module needs one written after it.
Lean's elaboration in the gate's `lake build` is the final check that each
declaration only names what its imports provide. A parser of Aeneas output:
tested here, not proved.
"""
from __future__ import annotations

from pathlib import Path
import re
import sys

DECL = re.compile(r"^/-- \[writfold_core::(\w+)", re.M)
NAME = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:def|inductive|structure|abbrev|theorem|opaque|axiom)\s+([\w.]+)", re.M)
WORD = re.compile(r"[A-Za-z_][\w.]*")
FOOTER = "end writfold_core\n"
# Every `@[discriminant]` enum that names only other such enums goes to one
# module, `Types`, in Aeneas's order.
# The attribute names its instance after the type's last component
# (`gate.State` and `decision.State` both give `instDiscriminantStateIsize`);
# Lean picks a fresh name for a repeat only when it sees the first, so keeping
# them together keeps the names of the single file. Mutants change function
# bodies, so `Types` is not rebuilt by them.
TYPES = "types"
IMPORT = "public import Aeneas\n"


class Refused(Exception):
    pass


def parse(text: str) -> tuple[str, list[tuple[str, str]]]:
    """The header and each declaration as (module, text), in file order."""
    starts = [m.start() for m in DECL.finditer(text)]
    if not starts or not text.endswith("\n" + FOOTER) or text.count(IMPORT) != 1:
        raise Refused("not a single-file Aeneas translation of writfold_core")
    end = len(text) - len(FOOTER)
    decls = [text[a:b] for a, b in zip(starts, starts[1:] + [end])]
    if any(DECL.match(d).group(1) == TYPES for d in decls):
        raise Refused(f"a kernel module is named `{TYPES}`, which the split reserves")
    return text[:starts[0]], [(DECL.match(d).group(1), d) for d in decls]


def references(decls: list[tuple[str, str]]) -> list[set[int]]:
    """For each declaration, the other declarations it names."""
    owner = {}
    for i, (_, text) in enumerate(decls):
        for name in NAME.findall(text):
            owner[name] = i
    out = []
    for i, (_, text) in enumerate(decls):
        found = set()
        for word in set(WORD.findall(text.split("-/", 1)[1])):
            parts = word.split(".")
            for k in range(len(parts), 0, -1):
                j = owner.get(".".join(parts[:k]))
                if j is not None:
                    if j != i:
                        found.add(j)
                    break
        out.append(found)
    return out


def classify(decls: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Move to `Types` every discriminant enum that names only other such
    enums; one that carries another module's type stays in its module."""
    refs = references(decls)
    chosen = {i for i, (_, text) in enumerate(decls) if DISCRIMINANT.search(text)}
    while True:
        keep = {i for i in chosen if refs[i] <= chosen}
        if keep == chosen:
            break
        chosen = keep
    return [(TYPES if i in chosen else module, text) for i, (module, text) in enumerate(decls)]


def dependencies(decls: list[tuple[str, str]]) -> dict[str, set[str]]:
    """Module to the other modules its declarations name."""
    needs: dict[str, set[str]] = {module: set() for module, _ in decls}
    for (module, _), refs in zip(decls, references(decls)):
        needs[module] |= {decls[j][0] for j in refs} - {module}
    return needs


def groups(needs: dict[str, set[str]], order: list[str]) -> list[list[str]]:
    """Strongly connected groups of modules, dependencies first; ties keep file order."""
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    stack: list[str] = []
    found: list[list[str]] = []

    def visit(v: str) -> None:
        index[v] = low[v] = len(index)
        stack.append(v)
        for w in sorted(needs[v], key=order.index):
            if w not in index:
                visit(w)
                low[v] = min(low[v], low[w])
            elif w in stack:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            group = []
            while True:
                w = stack.pop()
                group.append(w)
                if w == v:
                    break
            found.append(sorted(group, key=order.index))

    for v in order:
        if v not in index:
            visit(v)
    return found


def camel(group: list[str]) -> str:
    return "".join(part.capitalize() for module in group for part in module.split("_"))


DISCRIMINANT = re.compile(r"^@\[discriminant \w+\]\s*\ninductive ([\w.]+)", re.M)


def shared_names(decls: list[tuple[str, str]], name: dict[str, str], order: list[str]) -> dict[str, set[str]]:
    """Extra imports so that generated names stay unique: a module that
    reuses a discriminant's last component imports the previous module that
    used it (an edge to an earlier module, so no cycle). With every
    discriminant in `Types` this adds nothing; it is the safety net."""
    users: dict[str, list[str]] = {}
    for module, text in decls:
        for full in DISCRIMINANT.findall(text):
            group = name[module]
            seen = users.setdefault(full.rsplit(".", 1)[-1], [])
            if group not in seen:
                seen.append(group)
    extra: dict[str, set[str]] = {group: set() for group in order}
    for groups_ in users.values():
        ordered = sorted(groups_, key=order.index)
        for earlier, later in zip(ordered, ordered[1:]):
            extra[later].add(earlier)
    return extra


def split(text: str) -> dict[str, str]:
    """Relative path to contents of every file the split writes."""
    header, decls = parse(text)
    decls = classify(decls)
    order = list(dict.fromkeys(module for module, _ in decls))
    needs = dependencies(decls)
    found = groups(needs, order)
    name = {module: camel(group) for group in found for module in group}
    shared = shared_names(decls, name, [camel(g) for g in found])
    files = {}
    for group in found:
        deps = sorted(({name[d] for m in group for d in needs[m]} | shared[camel(group)]) - {camel(group)},
                      key=lambda n: [camel(g) for g in found].index(n))
        imports = IMPORT + "".join(f"public import Extracted.WritfoldCore.{d}\n" for d in deps)
        body = "".join(d for module, d in decls if module in group)
        files[f"WritfoldCore/{camel(group)}.lean"] = header.replace(IMPORT, imports) + body + FOOTER
    umbrella = ("-- THIS FILE WAS AUTOMATICALLY GENERATED BY proofs/split.py FROM AENEAS'S OUTPUT\n"
                "-- [writfold_core]: every module of the translation\nmodule\n"
                + "".join(f"public import Extracted.WritfoldCore.{camel(g)}\n" for g in found))
    files["WritfoldCore.lean"] = umbrella
    check(text, files, [camel(g) for g in found], name)
    return files


def check(text: str, files: dict[str, str], order: list[str], name: dict[str, str]) -> None:
    """Obligation 1: the same declarations, byte for byte, in a valid order."""
    header, decls = parse(text)
    decls = classify(decls)
    group_of = {d: name[m] for m, d in decls}
    seen: list[tuple[str, str]] = []
    for position, group in enumerate(order):
        own_header, own = parse(files[f"WritfoldCore/{group}.lean"])
        imported = re.findall(r"^public import Extracted\.WritfoldCore\.(\w+)$", own_header, re.M)
        if set(imported) & set(order[position:]):
            raise Refused(f"{group} imports itself or a module written after it")
        if re.sub(r"public import Extracted\.WritfoldCore\.\w+\n", "", own_header) != header:
            raise Refused(f"{group} has a header other than Aeneas's")
        if any(group_of.get(d) != group for _, d in own):
            raise Refused(f"{group} holds another module's declaration")
        if [d for _, d in own] != [d for m, d in decls if name[m] == group]:
            raise Refused(f"{group} does not keep its declarations, in Aeneas's order")
        seen.extend(own)
    if sorted(d for _, d in seen) != sorted(d for _, d in decls):
        raise Refused("the split does not keep every declaration byte for byte")


def write(directory: Path) -> None:
    source = directory / "WritfoldCore.lean"
    files = split(source.read_text())
    # Aeneas's file stays, unbuilt (lakefile roots), as tauten's input: tauten
    # takes a claim's reach per extracted file (tauten#79), and this one file
    # holds every callee. Remove it once tauten takes the reach across files.
    (directory / "Aeneas").mkdir(exist_ok=False)
    source.rename(directory / "Aeneas" / "WritfoldCore.lean")
    (directory / "WritfoldCore").mkdir(exist_ok=False)
    for path, content in files.items():
        (directory / path).write_text(content)
    print(f"split: {len(files) - 1} modules")


SAMPLE = """-- THIS FILE WAS AUTOMATICALLY GENERATED BY AENEAS
module
public import Aeneas
@[expose] public section
namespace writfold_core

/-- [writfold_core::b::B]
    Visibility: public -/
inductive b.B where
| X : b.B

/-- [writfold_core::a::f]
    Visibility: public -/
def a.f (x : b.B) : Result Bool := ok true

/-- [writfold_core::b::g]
    Visibility: public -/
@[reducible] def b.g (x : b.B) : Result Bool := a.f x

/-- [writfold_core::c::h]
    Visibility: public -/
def c.h : Result Bool := ok false

end writfold_core
"""


def self_test() -> None:
    files = split(SAMPLE)
    assert sorted(files) == ["WritfoldCore.lean", "WritfoldCore/BA.lean", "WritfoldCore/C.lean"], sorted(files)
    ab = files["WritfoldCore/BA.lean"]
    assert ab.index("inductive b.B") < ab.index("def a.f") < ab.index("def b.g"), ab
    assert "public import Extracted" not in files["WritfoldCore/C.lean"]
    acyclic = SAMPLE.replace("a.f x", "ok true")
    files = split(acyclic)
    assert sorted(files) == ["WritfoldCore.lean", "WritfoldCore/A.lean",
                             "WritfoldCore/B.lean", "WritfoldCore/C.lean"], sorted(files)
    assert "public import Extracted.WritfoldCore.B\n" in files["WritfoldCore/A.lean"]
    umbrella = files["WritfoldCore.lean"]
    assert umbrella.index(".B\n") < umbrella.index(".A\n") < umbrella.index(".C\n"), umbrella
    for bad in (SAMPLE.replace(FOOTER, ""), SAMPLE.replace(IMPORT, IMPORT * 2), "module\n"):
        try:
            split(bad)
        except Refused:
            continue
        raise AssertionError("a malformed translation was accepted")
    twins = SAMPLE.replace("a.f x", "ok true").replace(
        "/-- [writfold_core::c::h]", "/-- [writfold_core::c::B]\n    Visibility: public -/\n"
        "@[discriminant isize]\ninductive c.B where\n| Y : c.B\n\n/-- [writfold_core::c::h]").replace(
        "inductive b.B where", "@[discriminant isize]\ninductive b.B where")
    files = split(twins)
    types = files["WritfoldCore/Types.lean"]
    assert types.index("inductive b.B") < types.index("inductive c.B"), types
    assert all("@[discriminant" not in v for k, v in files.items() if k != "WritfoldCore/Types.lean")
    assert "public import Extracted.WritfoldCore.Types\n" in files["WritfoldCore/B.lean"]
    try:
        split(SAMPLE.replace("writfold_core::c::h", "writfold_core::types::h"))
        raise AssertionError("a kernel module named types was accepted")
    except Refused:
        pass
    owner = {"b.B": "BA", "a.f": "BA", "c.h": "C"}
    assert named("theorem t : writfold_core.c.h = ok false ∧ a.f x = y", owner) == ["BA", "C"]
    assert named("theorem t : b.B.X = b.B.X", owner) == ["BA"]
    print("split: self-test ok")


def named(text: str, owner: dict[str, str]) -> list[str]:
    """The split modules whose declarations a theorem file names, sorted."""
    found = set()
    for word in set(WORD.findall(text)):
        parts = word.removeprefix("writfold_core.").split(".")
        for k in range(len(parts), 0, -1):
            if ".".join(parts[:k]) in owner:
                found.add(owner[".".join(parts[:k])])
                break
    return sorted(found)


def narrow(extracted: Path, theorems: list[Path]) -> None:
    """Give each theorem file the split modules it names, in place of any
    `import Extracted.WritfoldCore…` it had. A file that names none and
    imported the translation imports Aeneas, which came with it."""
    owner = {}
    for module in sorted((extracted / "WritfoldCore").glob("*.lean")):
        for name in NAME.findall(module.read_text()):
            owner[name] = module.stem
    pattern = re.compile(r"^import Extracted\.WritfoldCore(?:\.\w+)?\n", re.M)
    for path in theorems:
        text = path.read_text()
        modules = named(text, owner)
        had = pattern.search(text)
        if not modules and not had:
            continue
        lines = "".join(f"import Extracted.WritfoldCore.{m}\n" for m in modules)
        if had:
            lines = lines or "import Aeneas\n"
            text = text[:had.start()] + lines + pattern.sub("", text[had.start():])
        else:
            first = re.search(r"^import ", text, re.M)
            at = first.start() if first else 0
            text = text[:at] + lines + text[at:]
        path.write_text(text)
        print(f"split: {path.name} imports {len(modules)} module(s)")


def main(argv: list[str]) -> int:
    try:
        if argv == ["--self-test"]:
            self_test()
        elif argv[:1] == ["--imports"] and len(argv) > 2:
            narrow(Path(argv[1]), [Path(a) for a in argv[2:]])
        elif len(argv) == 1:
            write(Path(argv[0]))
        else:
            print(__doc__, file=sys.stderr)
            return 2
    except Refused as refusal:
        print(f"split: FAIL: {refusal}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
