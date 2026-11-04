#!/usr/bin/env python3
"""Structural validation of the proof-first plan. No production gate is passed."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def registry(name):
    return json.loads((ROOT / "registries" / (name + ".json")).read_text())


def main():
    meta = registry("bundle")
    assert meta["revision"] in (6, 7, 8, 9)
    stages = {s["id"]: s for s in registry("stages")}
    gates = {g["id"]: g for g in registry("gates")}
    proofs = {p["id"]: p for p in registry("proofs")}
    citation = registry("citation-theorems")
    expected = {f"CIT{i:02}" for i in range(1, 9)}
    assert len(citation) == meta["citation_theorem_count"] == len(expected)
    assert {c["id"] for c in citation} == expected
    assert all(c["status"] == "not_run" and not c["theorem_symbols"] and c["receipt"] is None
               for c in citation)
    assert "P03C" in stages["P05C"]["dependencies"] and "P05C" in stages["P07"]["dependencies"]
    assert set(gates["WF32"]["stages"]) == {"P03C", "P11"}
    assert set(gates["WF33"]["stages"]) == {"P05C", "P11"}
    assert {"V22", "V23"} <= set(stages["P05C"]["proofs"]) & set(stages["P11"]["proofs"])
    assert {f"X{i}" for i in range(149, 165)} <= set(stages["P11"]["scenarios"])
    profile = registry("proposal-proof-profiles")
    assert profile["default"] == "deny" and set(profile["common_lean"]) == expected
    assert profile["instance_certificate"]["required"] and profile["instance_certificate"]["checker"] == "Lean kernel"
    assert set(profile["common_gates"]) == {"WF32", "WF33"}
    assert set(profile["common_verus"]) == {"V22", "V23"}
    old_lean = json.loads((ROOT / "reference/lean-theorems.json").read_text())["theorems"]
    known_lean = set(old_lean) | {t["id"] for t in registry("engineering-theorems")}
    kinds = {p["kind"] for p in profile["profiles"]}
    assert kinds == {"knowledge-admission-or-correction", "citation-successor", "design-acceptance-or-binding",
                     "procedure-or-skill", "learning-promotion", "policy-schema-or-rule"}
    for rule in profile["profiles"]:
        assert rule["additional_lean"] and set(rule["additional_lean"]) <= known_lean
        assert set(rule["additional_verus"] + rule["protocol_models"]) <= proofs.keys()
        assert rule["status"] == "not_run" and rule["production_receipt"] is None
    assert meta["priority_entry"] == "PRIORITIES.md" and (ROOT / meta["priority_entry"]).is_file()
    assert "P03C" == meta["first_new_stage"]
    print("PASS: citation targets, fail-closed proposal profiles, acyclic gate prerequisites and priority entry.")
    print("Structural handover checks only. Lean proofs and production gates are pending.")


if __name__ == "__main__":
    main()
