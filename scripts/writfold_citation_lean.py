#!/usr/bin/env python3
"""Clean scoped citation calculus qualification, never full WF08 admission."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
import traceback

sys.dont_write_bytecode = True
from writfold_conformance_process import (
    ROOT, Commands, manifest, observe_tools, record, retain_binary, sha, write_json,
)
from writfold_engineering_lean import ALLOWED_AXIOMS, prepare_build
from writfold_citation_mutations import calibrate

REQUIRED = {f"CIT{number:02}" for number in (1, 2, 3, 4, 5, 6, 8)}
SYMBOL = re.compile(r"Kctx\.(?:[A-Za-z0-9_]+\.)*[A-Za-z0-9_]+")
SCOPE = "Scoped citation Lean kernel, transitive axioms, executable cases and semantic mutations; not Rust refinement or full WF08."


def inputs() -> dict:
    formal = ROOT / "formal"
    lean = list((formal / "Kctx").rglob("*.lean"))
    lean.extend(formal / name for name in (
        "Kctx.lean", "lean-toolchain", "lakefile.toml", "lake-manifest.json",
        "theorems.json", "engineering-theorems.json", "citation-theorems.json",
    ))
    recipe = [Path(__file__), ROOT / "scripts/writfold_engineering_lean.py",
              ROOT / "scripts/writfold_citation_mutations.py",
              ROOT / "scripts/writfold_conformance_process.py", ROOT / "compose.yaml",
              ROOT / "toolchains.lock.json", ROOT / "qualification/toolchain-observation.json"]
    contract = [ROOT / name for name in (
        "qualification/citation-profile.json", "qualification/revision-9-feature-adoption.json",
        "docs/writfold/revision-9-features.md", "handovers/revision-9/writfold/MANIFEST.sha256",
        "handovers/revision-9/writfold/plan/24-citation-calculus.md",
        "handovers/revision-9/writfold/plan/26-compact-citations-and-followups.md",
        "handovers/revision-9/writfold/examples/citations/duplicate-line.json",
    )]
    return {"lean": manifest(lean), "recipe": manifest(recipe), "contracts": manifest(contract)}


def registry() -> tuple[dict, set[str]]:
    current = json.loads((ROOT / "formal/citation-theorems.json").read_text())
    profile = json.loads((ROOT / "qualification/citation-profile.json").read_text())
    if (current.get("schema_version") != 1 or current.get("profile") != "writfold-citation/1"
            or set(current.get("theorems", {})) != REQUIRED
            or set(profile["required_lean"]) != REQUIRED or current.get("deferred") != ["CIT07"]):
        raise ValueError("Citation inventory must match the approved seven-target scope")
    names = set()
    for filename, expected in (("theorems.json", {f"T{n:02}" for n in range(1, 23)}),
                               ("engineering-theorems.json", {f"E{n:02}" for n in range(1, 13)}),
                               ("citation-theorems.json", REQUIRED)):
        entries = json.loads((ROOT / "formal" / filename).read_text())
        if set(entries["theorems"]) != expected:
            raise ValueError(f"Changed proof inventory: {filename}")
        for identity, row in entries["theorems"].items():
            if not isinstance(row.get("theorem"), list) or not row["theorem"]:
                raise ValueError(f"Unmapped proof target: {identity}")
            names.update(row["theorem"])
            if filename == "citation-theorems.json" and any(not row.get(key) for key in
                    ("meaning", "assumptions", "actions", "production_boundary")):
                raise ValueError(f"Incomplete citation meaning/assumptions: {identity}")
        names.update(entries.get("nonvacuity", []))
    if not current.get("nonvacuity") or any(SYMBOL.fullmatch(name) is None for name in names):
        raise ValueError("Invalid or absent registered proof declarations")
    cases = current.get("cases", [])
    if (not cases or len({case["id"] for case in cases}) != len(cases)
            or {(case["obligation"], case["polarity"]) for case in cases} !=
            {(identity, polarity) for identity in REQUIRED for polarity in ("positive", "negative")}):
        raise ValueError("Every citation target requires distinct positive and negative executable cases")
    return current, names


def audit(commands: Commands, work: Path, names: set[str], output: Path) -> dict:
    source = output / "citation-axiom-audit.lean"
    source.write_text("import Kctx\nimport Kctx.Engineering\nimport Kctx.Citation\n" +
                      "".join(f"#check {name}\n#print axioms {name}\n" for name in sorted(names)))
    log = commands.run("citation-axiom-audit", ["lake", "env", "lean", str(source)],
                       cwd=work, timeout=900)
    text = log.read_text()
    entries = {match[1]: [v.strip() for v in match[2].split(",") if v.strip()]
               for match in re.finditer(r"'([^']+)'\s+depends on axioms:\s*\[([^\]]*)\]", text)}
    for match in re.finditer(r"'([^']+)'\s+does not depend on any axioms", text):
        entries[match[1]] = []
    if set(entries) != names:
        raise ValueError(f"Incomplete audit: {sorted(names - entries.keys())}")
    rejected = {name: sorted(set(values) - ALLOWED_AXIOMS) for name, values in entries.items()
                if set(values) - ALLOWED_AXIOMS}
    if rejected:
        raise ValueError(f"Unapproved transitive axioms: {rejected}")
    return {"declarations": len(entries), "axioms": entries,
            "statements_and_axioms": record(log), "audit_source": record(source)}


def examples(commands: Commands, work: Path, current: dict, output: Path) -> dict:
    binary = output / "citationref"
    artifact = retain_binary(work / ".lake/build/bin/citationref", binary)
    log = commands.run("citation-reference", [str(binary)], cwd=work)
    rows = [json.loads(line) for line in log.read_text().splitlines()]
    expected = {case["id"]: case for case in current["cases"]}
    if len(rows) != len(expected) or {row["id"] for row in rows} != expected.keys():
        raise ValueError("Citation executable omitted, repeated or invented a registered case")
    for row in rows:
        case = expected[row["id"]]
        if (row.get("passed") is not True or row.get("obligation") != case["obligation"]
                or row.get("polarity") != case["polarity"]):
            raise ValueError(f"Failed or misbound citation case: {row}")
    return {"binary": artifact, "results": record(log), "cases": rows}


def integrity(sources: dict, work: Path, commands: Commands, expected_rejections: set[int]) -> None:
    if inputs() != sources:
        raise ValueError("Citation source, contract or recipe changed during qualification")
    for name, digest in sources["lean"]["files"].items():
        if sha(work / Path(name).relative_to("formal")) != digest:
            raise ValueError(f"Build changed declared input: {name}")
    for index, command in enumerate(commands.records):
        expected_exit = 1 if index in expected_rejections else 0
        if command["exit_code"] != expected_exit or command["timed_out"]:
            raise ValueError("Unaccounted command failure or timeout")
        for log in command["logs"]:
            path = Path(log["path"])
            if sha(path) != log["sha256"]:
                raise ValueError(f"Changed command output: {path}")
            if index not in expected_rejections and "declaration uses 'sorry'" in path.read_text(errors="replace"):
                raise ValueError(f"Unfinished accepted proof: {path}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("/evidence"))
    parser.add_argument("--cache", type=Path, default=Path("/cache/target"))
    args = parser.parse_args()
    output, cache = args.output.resolve(), args.cache.resolve()
    if output.is_relative_to(ROOT) or cache.is_relative_to(ROOT):
        parser.error("Evidence and proof builds must stay outside the source checkout")
    output.mkdir(parents=True, exist_ok=True)
    write_json(output / "citation-lean-attempt.json", {"argv": sys.argv, "scope": SCOPE})
    commands = Commands(output)
    report = {"schema_version": 1, "result": "fail", "scope": SCOPE,
              "commands": commands.records, "skips": [], "errors": []}
    checks = [{"name": "citation-lean-complete-component", "required": True, "status": "fail"}]
    try:
        sources = inputs()
        report["sources"] = sources
        write_json(output / "citation-lean-inputs.json", sources)
        report["tools"] = observe_tools(commands)
        current, names = registry()
        report["registry"] = current
        work = prepare_build(sources, cache)
        report["build_directory"] = str(work)
        commands.run("clean-legacy-build", ["lake", "build"], cwd=work, timeout=1800)
        commands.run("citation-reference-build",
                     ["lake", "build", "+Kctx.Engineering", "+Kctx.Citation", "citationref"],
                     cwd=work, timeout=1800)
        report["audit"] = audit(commands, work, names, output)
        report["examples"] = examples(commands, work, current, output)
        report["calibration"] = calibrate(commands, work, output)
        rejections = {row["proof_rejection_command"] for row in report["calibration"]["mutants"]}
        integrity(sources, work, commands, rejections)
        checks = [{"name": f"{identity}-kernel-audit-and-examples", "required": True, "status": "pass"}
                  for identity in sorted(REQUIRED)]
        checks.extend({"name": row["name"], "required": True, "status": row["result"]}
                      for row in report["calibration"]["mutants"])
        report["result"] = "pass"
    except Exception as exc:
        report["errors"].append(f"{type(exc).__name__}: {exc}")
        (output / "citation-lean.failure.log").write_text(traceback.format_exc())
    finally:
        write_json(output / "citation-lean-report.json", report)
        write_json(output / "obligations.json", {"checks": checks, "skips": []})
    print(json.dumps({"result": report["result"], "scope": SCOPE, "errors": report["errors"]}))
    return 0 if report["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
