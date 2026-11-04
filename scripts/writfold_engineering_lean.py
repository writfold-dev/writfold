#!/usr/bin/env python3
"""Clean Lean engineering-calculus build, axiom audit and executable examples.

This component cannot establish theorem fidelity, Rust refinement or complete WF31.
The outer gate runner binds the full checkout and admitted Compose image.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import traceback

sys.dont_write_bytecode = True
from writfold_conformance_process import (
    ROOT, Commands, canonical, manifest, observe_tools, record, retain_binary,
    sha, write_json,
)

AUTHORITY = ROOT / "handovers/revision-5/writfold"
ALLOWED_AXIOMS = {"propext", "Classical.choice", "Quot.sound"}
REQUIRED = {f"E{number:02}" for number in range(1, 13)}
SYMBOL = re.compile(r"Kctx\.(?:[A-Za-z0-9_]+\.)*[A-Za-z0-9_]+")
SCOPE = (
    "Clean Lean kernel acceptance, transitive registered-axiom audit and executable "
    "engineering examples only. Theorem fidelity is reviewed separately. Not a "
    "Rust/Lean refinement theorem, operational qualification or complete WF31."
)


def inputs() -> dict:
    formal = ROOT / "formal"
    lean = list((formal / "Kctx").rglob("*.lean"))
    lean.extend(formal / name for name in (
        "Kctx.lean", "lean-toolchain", "lakefile.toml", "lake-manifest.json",
        "theorems.json", "engineering-theorems.json",
    ))
    recipe = [Path(__file__), ROOT / "scripts/writfold_conformance_process.py",
              ROOT / "compose.yaml", ROOT / "toolchains.lock.json",
              ROOT / "qualification/toolchain-observation.json"]
    contracts = [AUTHORITY / "MANIFEST.sha256",
                 AUTHORITY / "registries/engineering-theorems.json"]
    contracts.extend((AUTHORITY / "plan").glob("1[7-9]-*.md"))
    contracts.extend((AUTHORITY / "plan").glob("2[0-3]-*.md"))
    return {"lean": manifest(lean), "recipe": manifest(recipe),
            "contracts": manifest(contracts)}


def registry() -> tuple[dict, dict, set[str]]:
    current = json.loads((ROOT / "formal/engineering-theorems.json").read_text())
    legacy = json.loads((ROOT / "formal/theorems.json").read_text())
    normative = json.loads((AUTHORITY / "registries/engineering-theorems.json").read_text())
    if {entry["id"] for entry in normative} != REQUIRED or len(normative) != 12:
        raise ValueError("Immutable E01-E12 inventory differs from the admitted contract")
    if (current.get("schema_version") != 1
            or current.get("profile") != "writfold-engineering/1"
            or set(current.get("theorems", {})) != REQUIRED):
        raise ValueError("Engineering registry must enumerate exactly E01-E12")
    if set(legacy.get("theorems", {})) != {f"T{number:02}" for number in range(1, 23)}:
        raise ValueError("Historical registry no longer enumerates T01-T22")
    names: set[str] = set()
    for source in (legacy, current):
        for identity, entry in source["theorems"].items():
            declarations = entry.get("theorem")
            if not isinstance(declarations, list) or not declarations:
                raise ValueError(f"Unmapped theorem obligation: {identity}")
            if source is current and any(not entry.get(key) for key in
                                         ("meaning", "assumptions", "executable", "actions")):
                raise ValueError(f"Incomplete engineering traceability: {identity}")
            names.update(declarations)
        names.update(source.get("nonvacuity", []))
    if not current.get("nonvacuity") or any(not isinstance(name, str)
                                           or SYMBOL.fullmatch(name) is None for name in names):
        raise ValueError("Invalid/missing registered nonvacuity declarations")
    cases = current.get("cases", [])
    if (not cases or len({case["id"] for case in cases}) != len(cases)
            or {(case["obligation"], case["polarity"]) for case in cases}
            != {(identity, polarity) for identity in REQUIRED
                for polarity in ("positive", "negative")}):
        raise ValueError("Each E obligation needs distinct executable positive/negative cases")
    return current, legacy, names


def prepare_build(sources: dict, cache: Path) -> Path:
    key = hashlib.sha256(canonical(sources).encode()).hexdigest()
    parent = cache / f"engineering-lean-{key}"
    parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="attempt-", dir=parent))
    for name, digest in sources["lean"]["files"].items():
        content = (ROOT / name).read_bytes()
        if hashlib.sha256(content).hexdigest() != digest:
            raise ValueError(f"Source changed while copying: {name}")
        target = work / Path(name).relative_to("formal")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
    if (work / "lean-toolchain").read_text().strip() != "leanprover/lean4:v4.21.0":
        raise ValueError("Engineering calculus changed the historical Lean toolchain")
    if json.loads((work / "lake-manifest.json").read_text()).get("packages") != []:
        raise ValueError("Unadmitted Lean dependency; offline calculus has no packages")
    return work


def audit(commands: Commands, work: Path, names: set[str], output: Path) -> dict:
    source = output / "axiom-audit.lean"
    source.write_text("import Kctx\nimport Kctx.Engineering\n"
                      + "".join(f"#check {name}\n#print axioms {name}\n"
                                for name in sorted(names)))
    log = commands.run("axiom-audit", ["lake", "env", "lean", str(source)],
                       cwd=work, timeout=600)
    text = log.read_text()
    entries = {}
    for match in re.finditer(r"'([^']+)'\s+depends on axioms:\s*\[([^\]]*)\]", text):
        entries[match[1]] = [value.strip() for value in match[2].split(",") if value.strip()]
    for match in re.finditer(r"'([^']+)'\s+does not depend on any axioms", text):
        entries[match[1]] = []
    if set(entries) != names:
        raise ValueError(f"Incomplete axiom audit: {sorted(names - entries.keys())}")
    bad = {name: sorted(set(values) - ALLOWED_AXIOMS) for name, values in entries.items()
           if set(values) - ALLOWED_AXIOMS}
    if bad:
        raise ValueError(f"Unapproved transitive axioms: {bad}")
    return {"declarations": len(entries), "axioms": entries,
            "statements_and_axioms": record(log), "audit_source": record(source)}


def examples(commands: Commands, work: Path, current: dict, output: Path) -> dict:
    binary = output / "engineeringref"
    artifact = retain_binary(work / ".lake/build/bin/engineeringref", binary)
    log = commands.run("engineering-reference", [str(binary)], cwd=work)
    rows = [json.loads(line) for line in log.read_text().splitlines()]
    expected = {case["id"]: case for case in current["cases"]}
    if len(rows) != len(expected) or {row["id"] for row in rows} != expected.keys():
        raise ValueError("Executable reference omitted, repeated or invented registered cases")
    for row in rows:
        case = expected[row["id"]]
        if (row.get("passed") is not True or row.get("obligation") != case["obligation"]
                or row.get("polarity") != case["polarity"]):
            raise ValueError(f"Failed/misbound calculus case: {row}")
    return {"binary": artifact, "results": record(log), "cases": rows,
            "positive": sum(row["polarity"] == "positive" for row in rows),
            "negative": sum(row["polarity"] == "negative" for row in rows)}


def integrity(commands: Commands, sources: dict, work: Path) -> None:
    if inputs() != sources:
        raise ValueError("Source, contract or qualification recipe changed during execution")
    for name, digest in sources["lean"]["files"].items():
        if sha(work / Path(name).relative_to("formal")) != digest:
            raise ValueError(f"Build altered a declared Lean input: {name}")
    for command in commands.records:
        for log in command["logs"]:
            if sha(Path(log["path"])) != log["sha256"]:
                raise ValueError(f"Command output changed: {log['path']}")
    for command in commands.records:
        for log in command["logs"]:
            if "declaration uses 'sorry'" in Path(log["path"]).read_text(errors="replace"):
                raise ValueError("Lean accepted an unfinished proof; component fails")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("/evidence"))
    parser.add_argument("--cache", type=Path, default=Path("/cache/target"))
    args = parser.parse_args()
    output, cache = args.output.resolve(), args.cache.resolve()
    if output.is_relative_to(ROOT) or cache.is_relative_to(ROOT):
        parser.error("Evidence and proof builds must stay outside the source checkout")
    output.mkdir(parents=True, exist_ok=True)
    # Exclusive attempt marker prevents accidental replacement of retained evidence.
    write_json(output / "engineering-lean-attempt.json", {"argv": sys.argv, "scope": SCOPE})
    commands = Commands(output)
    report = {"schema_version": 1, "result": "fail", "scope": SCOPE,
              "commands": commands.records, "skips": [], "errors": []}
    checks = []
    try:
        sources = inputs()
        report["sources"] = sources
        write_json(output / "engineering-lean-inputs.json", sources)
        report["tools"] = observe_tools(commands)
        current, legacy, names = registry()
        report["registry"] = current
        report["legacy_semantic_version"] = legacy["semantic_version"]
        work = prepare_build(sources, cache)
        report["build_directory"] = str(work)
        commands.run("clean-lean-build", ["lake", "build"], cwd=work, timeout=1800)
        commands.run("engineering-reference-build",
                     ["lake", "build", "+Kctx.Engineering", "engineeringref"],
                     cwd=work, timeout=1800)
        report["audit"] = audit(commands, work, names, output)
        report["examples"] = examples(commands, work, current, output)
        integrity(commands, sources, work)
        if sha(Path(report["examples"]["binary"]["path"])) != report["examples"]["binary"]["sha256"]:
            raise ValueError("Executed reference artifact changed")
        checks = [{"name": f"{identity}-kernel-audit-and-examples", "required": True,
                   "status": "pass"} for identity in sorted(REQUIRED)]
        report["result"] = "pass"
    except Exception as exc:
        report["errors"].append(f"{type(exc).__name__}: {exc}")
        (output / "engineering-lean.failure.log").write_text(traceback.format_exc())
        checks = [{"name": "engineering-lean-complete-component", "required": True,
                   "status": "fail"}]
    finally:
        write_json(output / "engineering-lean-report.json", report)
        write_json(output / "obligations.json", {"checks": checks, "skips": []})
    print(json.dumps({"result": report["result"], "scope": SCOPE,
                      "errors": report["errors"]}))
    return 0 if report["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
