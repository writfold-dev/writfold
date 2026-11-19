#!/usr/bin/env python3
"""Write proofs/claims.toml for `cargo tauten proofs` (docs/writfold/tauten-adoption.md).

    docker run --rm --network none writfold-proofs:tauten \\
        sh -c 'cd proofs && lake build >&2 && lake env lean Total.lean' > /tmp/total.txt
    python3 proofs/tauten_claims.py /tmp/total.txt

Reads the `TOTAL`/`SPEC` lines of proofs/Total.lean (which theorem is a Hoare
triple about which extracted function) and the mutants of proofs/mutants.py,
and writes one `[[claim]]` per Rust function of writfold-core that some mutant
breaks a theorem of:

  theorems  every triple about the function (or its loops), the theorems the
            mutants inside the function break, and its total theorem
  total     a `TOTAL` theorem about the function itself (tauten's shape:
            `∀ args, f args ⦃ _ ⦄`, no hypotheses), if there is one
  subsumes  TAU-N001 and TAU-P003, only with `total`
  mutants   the mutants inside the function, and those whose named theorems
            are about it

A function with no such mutant is not claimed (tauten needs a killed mutant
per claim). This only drafts the file; the gate decides every outcome.
Functions are located by a brace scan of the Rust source, which is enough for
writfold-core's plain items; a wrong name only means nothing is subsumed.
"""
from pathlib import Path
import importlib.util
import re
import sys

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "crates" / "writfold-core" / "src"
LINE = re.compile(r"\b(TOTAL|SPEC) (Writfold\.Core\.\S+) (writfold_core\.\S+)")
FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+(\w+)")
IMPL = re.compile(r"^\s*impl(?:<[^>]*>)?\s+(\w+)")
MOD = re.compile(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*\{")


def functions(path: Path) -> list[tuple[str, int, int]]:
    """(path like `Type::name`, first line, last line) of every fn body."""
    found, stack, pending = [], [], None
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        for kind, pattern in (("fn", FN), ("impl", IMPL), ("mod", MOD)):
            if match := pattern.match(line):
                pending = (kind, match.group(1), number)
                break
        code = re.sub(r"//.*", "", line)
        code = re.sub(r"'.'|\"(\\.|[^\"])*\"", "", code)
        for char in code:
            if char == "{":
                stack.append(pending)
                pending = None
            elif char == "}":
                item = stack.pop()
                if item and item[0] == "fn":
                    prefix = [i[1] for i in stack if i]
                    found.append(("::".join(prefix + [item[1]]), item[2], number))
        if pending and code.rstrip().endswith(";"):
            pending = None
    return found


def enclosing(path: Path, line: int) -> str:
    spans = [s for s in functions(path) if s[1] <= line <= s[2]]
    return min(spans, key=lambda s: s[2] - s[1])[0]


def lean_name(file: str, function: str) -> str:
    module = file.removesuffix(".rs").removesuffix("/mod").replace("/", ".")
    return f"writfold_core.{module}.{function.replace('::', '.')}"


def base(lean: str) -> str:
    return re.sub(r"_loop\d*$", "", lean)


def main() -> None:
    triples: dict[str, list[tuple[str, str]]] = {}
    for kind, theorem, function in LINE.findall(Path(sys.argv[1]).read_text()):
        # a theorem about a loop is about its function, but total only for the loop
        kind = kind if function == base(function) else "SPEC"
        triples.setdefault(base(function), []).append((kind, theorem))
    spec = importlib.util.spec_from_file_location("mutants", ROOT / "proofs" / "mutants.py")
    mutants = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mutants)

    located: dict[tuple[str, str], list] = {}
    for mutant in mutants.MUTANTS:
        text = (SRC / mutant.file).read_text()
        line = text[: text.index(mutant.before)].count("\n") + 1
        located.setdefault((mutant.file, enclosing(SRC / mutant.file, line)), []).append(mutant)

    claims = []
    for file, function in sorted({key for key in located}
                                 | {(str(p.relative_to(SRC)), f) for p in SRC.rglob("*.rs")
                                    for f, _, _ in functions(p)
                                    if lean_name(str(p.relative_to(SRC)), f) in triples}):
        lean = lean_name(file, function)
        theorems = {t for _, t in triples.get(lean, [])}
        for mutant in located.get((file, function), []):
            theorems |= set(mutant.fails)
        names = [m.name for m in mutants.MUTANTS
                 if m in located.get((file, function), []) or set(m.fails) & theorems]
        if not names:
            continue
        totals = sorted(t for k, t in triples.get(lean, []) if k == "TOTAL")
        preferred = [t for t in totals if t.endswith(("_spec", "_total"))]
        total = (preferred or totals or [None])[0]
        claims.append((file, function, lean, sorted(theorems), total, names))

    out = ["# Proof claims for `cargo tauten proofs` (docs/writfold/tauten-adoption.md).",
           "# Drafted by proofs/tauten_claims.py from proofs/Total.lean and",
           "# proofs/mutants.py; the gate (proofs/tauten-gate.sh) decides each outcome",
           "# and `cargo tauten proofs` records it in proofs/claims.json.",
           "version = 1", "", "[gate]",
           'command = ["sh", "proofs/tauten-gate.sh"]',
           'crates = ["crates/writfold-core"]']
    for file, function, lean, theorems, total, names in claims:
        out += ["", "[[claim]]", 'crate = "writfold-core"',
                f'file = "crates/writfold-core/src/{file}"', f'function = "{function}"',
                f'lean = "{lean}"', "theorems = ["] + [f'  "{t}",' for t in theorems] + ["]"]
        if total:
            out += [f'total = "{total}"', 'subsumes = ["TAU-N001", "TAU-P003"]']
        out += ["mutants = ["] + [f'  "{n}",' for n in names] + ["]"]
    (ROOT / "proofs" / "claims.toml").write_text("\n".join(out) + "\n")
    print(f"{len(claims)} claims, {sum(1 for c in claims if c[4])} with a total theorem",
          file=sys.stderr)


if __name__ == "__main__":
    main()
