"""Retarget only the frozen probes' executable identity literals to a new pin.

The immutable handover files stay untouched. Their behavior assertions are not
changed: an AST-constrained derivation replaces exactly one version and one ELF
SHA256 literal in each script, and retains both original and derived identities.
"""
from __future__ import annotations

import ast
from pathlib import Path
import shutil

from writfold_evidence import file_record, publish, sha


def retarget(source: bytes, replacements: dict[str, str]) -> bytes:
    text = source.decode("utf-8")
    tree = ast.parse(text)
    lines = source.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))
    changes = []
    counts = dict.fromkeys(replacements, 0)
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and node.value in replacements:
            counts[node.value] += 1
            changes.append((offsets[node.lineno - 1] + node.col_offset,
                            offsets[node.end_lineno - 1] + node.end_col_offset,
                            repr(replacements[node.value]).encode("ascii")))
            node.value = replacements[node.value]
    if any(count != 1 for count in counts.values()):
        raise ValueError("Frozen probe identity literal inventory changed")
    result = source
    for start, end, value in sorted(changes, reverse=True):
        result = result[:start] + value + result[end:]
    if ast.dump(ast.parse(result), include_attributes=False) != ast.dump(tree, include_attributes=False):
        raise ValueError("Probe derivation changed more than identity literals")
    return result


def derive(fixture: Path, destination: Path, historical: dict, candidate: dict) -> Path:
    if candidate["fixture_sha256"] != historical["fixture_sha256"]:
        raise ValueError("Candidate must preserve original immutable fixture inventory")
    destination.mkdir(mode=0o700)
    replacements = {historical["version"]: candidate["version"],
                    historical["members"]["omnigraph"]: candidate["members"]["omnigraph"]}
    records = {}
    for name, expected in historical["fixture_sha256"].items():
        original = fixture / name
        if original.is_symlink() or not original.is_file() or sha(original) != expected:
            raise ValueError(f"Immutable fixture changed: {name}")
        derived = destination / name
        if name in ("compare.py", "versioned_schema.py"):
            derived.write_bytes(retarget(original.read_bytes(), replacements))
        else:
            shutil.copyfile(original, derived)
            if sha(derived) != expected:
                raise ValueError(f"Copied fixture bytes changed: {name}")
        records[name] = {"original": file_record(original), "derived": file_record(derived)}
    publish(destination.parent / "fixture-derivation.json", {
        "schema": "writfold-native-probe-derivation/1",
        "scope": "Expected version and ELF identity only; all semantic assertions unchanged",
        "source_commit": candidate["source_commit"],
        "replacements": replacements,
        "files": records,
    })
    return destination
