#!/usr/bin/env python3
"""Exact-source native relocation adversaries; candidate smoke only, never WF24."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import time

from writfold_omnigraph_companion import Process
from writfold_omnigraph_epochs import (Campaign, FIXTURE,
                                       expected_records, inventory, require, save, semantic, sha)
from writfold_omnigraph_probe import SOURCE_CANDIDATE, validate_profile


class Relocation(Campaign):
    def run(self, label, argv, json_output=True, refusal=False):
        process = Process(self.work, label, ["/native/omnigraph", *map(str, argv)],
                          self.env, [], self.receipt["commands"])
        try:
            result = process.finish(120)
            self.check(label + ": bounded owned process discharged",
                       result["reaped"] and result["group_absent"] and result["logs_drained"]
                       and not result["forced_kill"] and not result.get("interrupted_unknown_outcome")
                       and all(not item.get("error") and not item["truncated"]
                               for item in result["logs"].values()))
            self.check(label + ": expected exit", result["exit"] > 0 if refusal else result["exit"] == 0)
            text = (self.work / f"{label}.stdout").read_text()
            if refusal:
                return text + (self.work / f"{label}.stderr").read_text()
            return json.loads(text) if json_output else text
        finally:
            save(self.work / "receipt.json", self.receipt)

    def clone(self, source, destination, label):
        before = inventory(source)
        shutil.copytree(source, destination)
        self.check(label + ": complete regular-file closure", inventory(destination) == before)
        self.check(label + ": source quiescent", inventory(source) == before)
        save(self.work / f"{label}-inventory.json", before)
        self.receipt.setdefault("copies", []).append({"source": str(source), "destination": str(destination),
            "inventory_sha256": sha(self.work / f"{label}-inventory.json")})

    def read_all(self, label, root, pins):
        before = inventory(root)
        for index, pin in enumerate(pins):
            selected = dict(pin, root=root.name)
            self.read_pin(f"{label}-{index}-snapshot", root.parent, selected)
            if pin.get("branch"):
                self.read_pin(f"{label}-{index}-branch", root.parent, selected, branch=pin["branch"])
        self.check(label + ": reads preserve full native manifest/object closure", inventory(root) == before)


def exercise(campaign, fixture):
    work = campaign.work
    inputs = work / "inputs"
    inputs.mkdir()
    for name in ("schema.pg", "seed.jsonl"):
        shutil.copyfile(fixture / name, inputs / name)
    shutil.copyfile(FIXTURE / "epoch1.gq", inputs / "epoch1.gq")
    shutil.copyfile(FIXTURE / "advance.jsonl", inputs / "advance.jsonl")
    # Deterministic extra writes distinguish branch history and the divergent source.
    for name in ("later", "divergent"):
        (inputs / f"{name}.jsonl").write_text(json.dumps({"type": "Service", "data": {
            "slug": f"relocation-{name}", "name": f"Relocation {name}"}}) + "\n")
    original = work / "original.omni"
    campaign.run("init", ["init", "--schema", inputs / "schema.pg", original], False)

    def load(label, root, data, branch="main", mode="append"):
        result = campaign.run(label, ["load", "--data", data, "--mode", mode,
                                      "--branch", branch, "--target", root, "--json"])
        campaign.check(label + ": native commit published", result["commit"]["published"] is True)
        return result["commit"]["graph_commit_id"]

    seed = load("seed", original, inputs / "seed.jsonl", mode="overwrite")

    def pin(label, commit, branch, files):
        value = {"epoch": "epoch1", "root": original.name, "query": "inputs/epoch1.gq",
                 "commit": commit, "branch": branch}
        value["expected"] = campaign.read_pin(label, work, value, branch=branch)
        campaign.check(label + ": complete trusted semantic rows",
                       semantic(value["expected"]) == expected_records(files))
        return value

    def branch(label, name, commit):
        result = campaign.run(label, ["branch", "create", name, "--from-commit", commit,
                                      "--target", original, "--json"])
        campaign.check(label + ": exact native fork", result["name"] == name and result["from_commit_id"] == commit)

    seed_files = [inputs / "seed.jsonl"]
    main = pin("seed-rows", seed, "main", seed_files)
    branch("retain-seed", "retained", seed)
    retained = dict(main, branch="retained")
    # First reproduce only the provided minimal seed + retained-branch case.
    minimal = work / "minimal.omni"
    campaign.clone(original, minimal, "minimal-copy")
    parked = work / "minimal-original-unavailable.omni"
    original.rename(parked)
    campaign.check("Minimal original pathname absent", not os.path.lexists(original))
    campaign.read_all("minimal-relocated", minimal, [main, retained])
    parked.rename(original)

    branch("create-written", "written", seed)
    written_commit = load("write-branch", original, inputs / "advance.jsonl", "written")
    campaign.check("Written branch differs from main", written_commit != seed)
    written_files = seed_files + [inputs / "advance.jsonl"]
    historical = pin("written-history", written_commit, "written", written_files)
    branch("child-of-written", "child", written_commit)
    child = dict(historical, branch="child")
    later_commit = load("advance-written", original, inputs / "later.jsonl", "written")
    campaign.check("Branch-only historical snapshot no longer head", later_commit not in (seed, written_commit))
    written = pin("written-head", later_commit, "written", written_files + [inputs / "later.jsonl"])
    historical = dict(historical, branch=None)
    pins = [main, retained, child, historical, written]
    campaign.read_all("baseline", original, pins)
    closure = {"source_commit": campaign.receipt["profile"]["source_commit"],
               "profile_sha256": sha(campaign.profile_path), "pins": pins,
               "inputs": inventory(inputs), "native": inventory(original)}
    save(work / "closure.json", closure)
    campaign.receipt["closure_sha256"] = sha(work / "closure.json")
    campaign.receipt["pins"] = pins
    backup, relocated, missing = [work / name for name in ("backup.omni", "relocated.omni", "missing.omni")]
    for label, target in (("backup", backup), ("relocated", relocated), ("missing", missing)):
        campaign.clone(original, target, label)

    # Remove copied inherited manifest objects only; retain the removed bytes as evidence.
    inherited = sorted((missing / "__manifest/data").glob("*.lance"))
    campaign.check("Copied inherited manifest data objects exist", bool(inherited))
    quarantine = work / "missing-objects"
    quarantine.mkdir()
    removed = []
    for path in inherited:
        relative = path.relative_to(missing)
        campaign.check("Original still owns exact missing-copy object", sha(original / relative) == sha(path))
        removed.append({"path": relative.as_posix(), "sha256": sha(path), "bytes": path.stat().st_size})
        path.rename(quarantine / path.name)
    campaign.receipt["removed_copy_objects"] = removed
    before_original, before_missing = inventory(original), inventory(missing)
    diagnostic = campaign.run("missing-copy-refusal", ["query", "nodes", "--query", inputs / "epoch1.gq",
                              "--branch", "retained", "--target", missing, "--json"], refusal=True)
    campaign.check("Missing copy refuses at copied path without original fallback",
                   str(missing) in diagnostic and str(original) not in diagnostic)
    campaign.check("Refusal changes neither original nor damaged copy",
                   inventory(original) == before_original and inventory(missing) == before_missing)
    campaign.read_all("original-after-refusal", original, pins)

    divergent = load("diverge-original", original, inputs / "divergent.jsonl")
    campaign.check("Original head truly diverges", divergent != seed)
    divergent_pin = dict(main, commit=divergent)
    divergent_pin.pop("expected")
    state = campaign.read_pin("divergent-original-rows", work, divergent_pin, branch="main")
    campaign.check("Original has exact divergent rows", semantic(state) == expected_records(seed_files + [inputs / "divergent.jsonl"]))
    # Remove original manifest data too: a copy cannot succeed by consulting old bytes.
    original_quarantine = work / "divergent-original-objects"
    original_quarantine.mkdir()
    manifest_objects = sorted(path for path in original.rglob("*.lance")
                              if "__manifest" in path.parts and path.parent.name == "data")
    campaign.check("Divergent original manifest objects exist", bool(manifest_objects))
    for index, path in enumerate(manifest_objects):
        path.rename(original_quarantine / f"{index:04d}-{path.name}")
    campaign.read_all("divergent-original-copy", relocated, pins)
    original.rename(work / "retained-divergent-original.omni")
    campaign.check("Original pathname absent for relocation", not os.path.lexists(original))
    campaign.read_all("absent-original-copy", relocated, pins)

    campaign.clone(backup, original, "same-path-restore")
    campaign.read_all("same-path-restored", original, pins)
    campaign.check("Frozen backup closure unchanged", inventory(backup) == closure["native"])
    campaign.check("Frozen inputs unchanged", inventory(inputs) == closure["inputs"])
    campaign.receipt["scenarios"] = ["minimal-retained-branch", "child-of-written-branch",
        "branch-only-historical-snapshot-with-unwritten-sibling", "divergent-original",
        "original-path-absent", "same-path-restore", "missing-copy-object-no-original-fallback"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    args = parser.parse_args()
    work = args.work.absolute()
    require(work == work.resolve() and work.is_relative_to(Path("/evidence"))
            and work != Path("/evidence"), "Fresh non-aliased child of /evidence required")
    campaign = Relocation(work, "relocation", args.profile)
    campaign.receipt["scope"] = "Exact-source candidate native relocation smoke only"
    campaign.receipt["refusal_contract"]["native_expected_nonzero_commands"] = "missing-copy-refusal only"
    try:
        profile = validate_profile(args.profile, Path("/profile/source-build.json"))
        campaign.check("Only exact relocation candidate allowed", profile["source_commit"] == SOURCE_CANDIDATE)
        fixture = campaign.admit()
        exercise(campaign, fixture)
        campaign.receipt["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        campaign.receipt["error"] = f"{type(error).__name__}: {error}"
    finally:
        campaign.receipt["ended_ns"] = time.time_ns()
        campaign.receipt["outputs"] = inventory(work)
        campaign.receipt["outputs"].pop("receipt.json", None)
        save(work / "receipt.json", campaign.receipt)
    print(f"{campaign.receipt['status']}: {work / 'receipt.json'}; candidate smoke only; WF24 not_run")
    return 0 if campaign.receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
