#!/usr/bin/env python3
"""Supplemental, exclusive native epoch/retention/relocation probe; never WF24."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import stat
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/qualification/omnigraph-epochs"
PROFILE = ROOT / "qualification/omnigraph-profile.json"
SOURCE = "60bfd19db780fdff3a5178f959c1204143420114"
NOT_RUN = ["WF24", "signed Writfold migration", "completed Git CAS", "GC/cleanup",
           "powerloss/crash durability", "concurrent writer ownership", "uncertain writes",
           "production storage adapter", "full semantic codecs/workloads", "merge/changefeed"]


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def document(path):
    return json.loads(path.read_text())


def save(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n")


def canonical(rows):
    return sorted(rows, key=lambda row: json.dumps(row, sort_keys=True, ensure_ascii=False))


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        mode = path.lstat().st_mode
        require(stat.S_ISDIR(mode) or stat.S_ISREG(mode), f"Link/special file refused: {path}")
        result[path.relative_to(root).as_posix()] = ({"kind": "directory"} if stat.S_ISDIR(mode)
            else {"kind": "file", "bytes": path.stat().st_size, "sha256": sha(path)})
    return result


class Campaign:
    def __init__(self, work, mode):
        self.work = work
        work.mkdir(mode=0o700, parents=False, exist_ok=False)
        (work / "home").mkdir(mode=0o700)
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(work / "home"), "TMPDIR": "/tmp",
                    "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
                    "RUST_BACKTRACE": "0", "RAYON_NUM_THREADS": "2", "TOKIO_WORKER_THREADS": "2",
                    "OMP_NUM_THREADS": "2", "OPENBLAS_NUM_THREADS": "2"}
        self.receipt = {"schema_version": 1, "scope": "Supplemental native separate epochs and quiesced restore",
                        "mode": mode, "status": "fail", "started_ns": time.time_ns(), "commands": [],
                        "assertions": [], "not_run": NOT_RUN, "wf24": "not_run", "environment": self.env,
                        "expected_refusals": [], "authority": "Native commits confer no Writfold approval"}
        self.receipt["refusal_contract"] = {
            "existing_work": "Refused before any writes; existing evidence never overwritten",
            "unsafe_backup": "Refuse original roots present, wrong digest/inventory, links or special files",
            "cross_schema_branch": "Not exercised here; epochs never schema-applied",
            "native_expected_nonzero_commands": "None; every selected supported native operation must succeed"}
        self.receipt["upstream_contracts"] = [
            f"https://github.com/ModernRelay/omnigraph/blob/{SOURCE}/docs/user/{path}"
            for path in ("branching/index.md", "queries/index.md", "queries/traversal.md")]

    def check(self, name, ok):
        self.receipt["assertions"].append({"name": name, "pass": bool(ok)})
        require(ok, name)

    def run(self, label, argv, json_output=True):
        command = {"name": label, "argv": ["/native/omnigraph", *map(str, argv)],
                   "started_ns": time.time_ns(), "deadline_seconds": 120, "expected_exit": 0}
        self.receipt["commands"].append(command)
        try:
            with (self.work / f"{label}.stdout").open("xb") as out, (self.work / f"{label}.stderr").open("xb") as err:
                with subprocess.Popen(command["argv"], cwd=self.work, env=self.env, stdout=out,
                                      stderr=err, start_new_session=True) as proc:
                    try:
                        command["exit"] = proc.wait(timeout=120)
                    except BaseException:
                        try:
                            os.killpg(proc.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        proc.wait()
                        command["exit"] = proc.returncode
                        raise
            require(command["exit"] == 0, f"{label} failed; retained stdout/stderr")
            text = (self.work / f"{label}.stdout").read_text()
            return json.loads(text) if json_output else text
        except BaseException as error:
            command["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            command["ended_ns"] = time.time_ns()
            save(self.work / "receipt.json", self.receipt)

    def admit(self):
        profile = document(PROFILE)
        self.check("Exact admitted source and version", profile["source_commit"] == SOURCE and profile["version"] == "0.14.0")
        for name, digest in profile["members"].items():
            path = Path("/native") / name
            self.check(f"Exact regular native binary: {name}", path.is_file() and not path.is_symlink() and sha(path) == digest)
        self.check("CLI reports exact version", self.run("version", ["version", "--json"])["version"] == "0.14.0")
        fixture = ROOT / profile["fixture_root"]
        for name, digest in profile["fixture_sha256"].items():
            self.check(f"Immutable supplied fixture: {name}", sha(fixture / name) == digest)
        paths = [Path(__file__).resolve(), PROFILE, ROOT / "toolchains.lock.json"]
        paths += sorted(FIXTURE.iterdir()) + [fixture / name for name in profile["fixture_sha256"]]
        self.receipt["bindings"] = {p.relative_to(ROOT).as_posix(): sha(p) for p in paths}
        self.receipt["profile"] = profile
        return fixture

    def read_pin(self, label, payload, pin, branch=None):
        graph = payload / pin["root"]
        query = payload / pin["query"]
        state = {}
        for kind in ("nodes", "relationships"):
            selector = ["--branch", branch] if branch else ["--snapshot", pin["commit"]]
            result = self.run(f"{label}-{kind}", ["query", kind, "--query", query, *selector, "--target", graph, "--json"])
            self.check(f"{label}/{kind}: echoed graph commit", result["graph_commit_id"] == pin["commit"])
            self.check(f"{label}/{kind}: echoed selector", result["target"].get("branch" if branch else "snapshot") == (branch or pin["commit"]))
            columns = ["identity", "slug", "name"] if kind == "nodes" else ["identity", "type", "source", "destination"]
            if kind == "nodes" and pin["epoch"] == "epoch2":
                columns.append("description")
            self.check(f"{label}/{kind}: complete explicit columns", result["columns"] == columns)
            rows = result["rows"]
            self.check(f"{label}/{kind}: complete row count", len(rows) == result["row_count"])
            for row in rows:
                self.check(f"{label}/{kind}: row shape", set(row) <= set(columns) and
                           set(columns) - set(row) <= {"description"})
            state[kind] = canonical([{key: row.get(key) for key in columns} for row in rows])
        if "expected" in pin:
            self.check(f"{label}: complete native rows identical", state == pin["expected"])
        return state


def semantic(state):
    return {"nodes": canonical([{k: v for k, v in row.items() if k != "description"} for row in state["nodes"]]),
            "relationships": canonical([{k: v for k, v in row.items() if k != "identity"} for row in state["relationships"]])}


def expected_records(paths):
    records = [json.loads(line) for path in paths for line in path.read_text().splitlines() if line]
    return {"nodes": canonical([{"identity": r["data"]["slug"], **r["data"]} for r in records if "type" in r]),
            "relationships": canonical([{"type": r["edge"], "source": r["from"], "destination": r["to"]} for r in records if "edge" in r])}


def retain(campaign, payload, pin, label):
    root = payload / pin["root"]
    result = campaign.run(label + "-create", ["branch", "create", pin["branch"], "--from-commit", pin["commit"], "--target", root, "--json"])
    campaign.check(label + ": native retention root starts at exact pin", result["name"] == pin["branch"] and result["from_commit_id"] == pin["commit"])
    campaign.read_pin(label + "-read", payload, pin, branch=pin["branch"])


def prepare(campaign, fixture):
    payload = campaign.work / "payload"
    payload.mkdir()
    inputs = payload / "inputs"
    inputs.mkdir()
    for name in ("schema.pg", "schema-next.pg", "seed.jsonl", "queries.gq"):
        shutil.copyfile(fixture / name, inputs / name)
    for path in sorted(FIXTURE.iterdir()):
        shutil.copyfile(path, inputs / path.name)
    pins = []
    for epoch, schema in (("epoch1", "schema.pg"), ("epoch2", "schema-next.pg")):
        root = payload / f"{epoch}.omni"
        campaign.run(epoch + "-init", ["init", "--schema", inputs / schema, root], json_output=False)
        loaded = campaign.run(epoch + "-load", ["load", "--data", inputs / "seed.jsonl", "--mode", "overwrite", "--target", root, "--json"])
        pin = {"epoch": epoch, "root": root.name, "schema": "inputs/" + schema,
               "query": f"inputs/{epoch}.gq", "branch": "retained_seed", "commit": loaded["commit"]["graph_commit_id"]}
        pin.update(schema_sha256=sha(inputs / schema), query_sha256=sha(inputs / f"{epoch}.gq"))
        campaign.check(epoch + ": published native seed", loaded["commit"]["published"] is True)
        pin["expected"] = campaign.read_pin(epoch + "-seed", payload, pin)
        campaign.check(epoch + ": complete trusted semantic records", semantic(pin["expected"]) == expected_records([inputs / "seed.jsonl"]))
        retain(campaign, payload, pin, epoch + "-retention")
        pins.append(pin)
        if epoch == "epoch2":
            campaign.read_pin("old-after-new-epoch", payload, pins[0])
    campaign.check("New optional field is null, not silently populated", all(row["description"] is None for row in pins[1]["expected"]["nodes"]))
    advanced = campaign.run("epoch1-advance", ["load", "--data", inputs / "advance.jsonl", "--mode", "append", "--target", payload / pins[0]["root"], "--json"])
    head = {key: value for key, value in pins[0].items() if key != "expected"}
    head.update(commit=advanced["commit"]["graph_commit_id"], branch="retained_advanced")
    campaign.check("Old epoch head actually advanced", head["commit"] != pins[0]["commit"] and advanced["commit"]["published"] is True)
    head["expected"] = campaign.read_pin("advanced-head", payload, head, branch="main")
    campaign.check("Advanced head exact records", semantic(head["expected"]) == expected_records([inputs / "seed.jsonl", inputs / "advance.jsonl"]))
    retain(campaign, payload, head, "advanced-retention")
    pins.append(head)
    campaign.read_pin("old-after-head-advance", payload, pins[0])
    campaign.read_pin("old-retained-after-head-advance", payload, pins[0], branch=pins[0]["branch"])
    mapping = {"scope": "Trusted synthetic identity mapping, not an approved migration",
               "node_identity": "Service.slug (native @id)", "edge_semantics": "DependsOn typed directed endpoint multiset; no properties",
               "nullable_codec": "Missing Arrow JSON description normalized to null; all other fields required",
               "nodes": [{"old": row, "new": next(r for r in pins[1]["expected"]["nodes"] if r["identity"] == row["identity"])} for row in pins[0]["expected"]["nodes"]],
               "old_relationships": pins[0]["expected"]["relationships"], "new_relationships": pins[1]["expected"]["relationships"],
               "semantic_records": semantic(pins[0]["expected"]), "quarantined": [], "refused": []}
    closure = {"schema_version": 1, "bindings": campaign.receipt["bindings"], "pins": pins, "mapping": mapping,
               "original_roots": [str(payload / f"{epoch}.omni") for epoch in ("epoch1", "epoch2")],
               "native_source": SOURCE, "not_run": NOT_RUN}
    save(payload / "closure.json", closure)
    # Every synchronous CLI has exited. Archive entire native roots, not selected Lance files.
    before = inventory(payload)
    archive = campaign.work / "backup.tar"
    with tarfile.open(archive, "x", format=tarfile.PAX_FORMAT, dereference=True) as package:
        for name in before:
            package.add(payload / name, arcname=name, recursive=False)
    campaign.check("Quiesced payload unchanged while archived", inventory(payload) == before)
    manifest = {"schema_version": 1, "archive_sha256": sha(archive), "archive_bytes": archive.stat().st_size,
                "inventory": before, "original_roots": closure["original_roots"], "closure_sha256": sha(payload / "closure.json")}
    save(campaign.work / "backup.json", manifest)
    campaign.receipt.update(pins=pins, mapping=mapping, backup=manifest,
                            quiesced=True, backup_method="Complete regular-file native-root tar after all CLI exits")


def unpack(campaign, archive, manifest_path):
    for path in (archive, manifest_path):
        require(path.is_file() and not path.is_symlink(), "Backup input must be a regular file")
    manifest = document(manifest_path)
    campaign.check("Backup manifest version", manifest["schema_version"] == 1)
    campaign.check("Exact backup digest and size", sha(archive) == manifest["archive_sha256"] and archive.stat().st_size == manifest["archive_bytes"])
    originals = manifest["original_roots"]
    campaign.check("Both original roots declared", len(originals) == 2 and len(set(originals)) == 2)
    for original in originals:
        campaign.check("Original native root absent: " + original, Path(original).is_absolute() and not os.path.lexists(original))
    destination = campaign.work / "relocated"
    destination.mkdir(mode=0o700)
    with tarfile.open(archive, "r:") as package:
        members = package.getmembers()
        names = [m.name for m in members]
        campaign.check("No duplicate/missing/unexpected archive members", len(names) == len(set(names)) and set(names) == set(manifest["inventory"]))
        for member in members:
            path = PurePosixPath(member.name)
            require(bool(path.parts) and not path.is_absolute() and str(path) == member.name and all(p not in (".", "..") for p in path.parts), "Unsafe archive path")
            require(member.isdir() or member.isfile(), "Archive links/special files refused")
            expected = manifest["inventory"][member.name]
            require(expected["kind"] == ("directory" if member.isdir() else "file"), "Archive type mismatch")
            if member.isfile():
                require(member.size == expected["bytes"] and member.size >= 0, "Archive file size mismatch")
        for member in members:
            target = destination / member.name
            if member.isdir():
                target.mkdir(mode=0o700, parents=True, exist_ok=True)
            else:
                target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                with package.extractfile(member) as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output)
    campaign.check("Every extracted byte matches SHA256 inventory", inventory(destination) == manifest["inventory"])
    campaign.check("Frozen closure identity", sha(destination / "closure.json") == manifest["closure_sha256"])
    campaign.receipt["backup_inputs"] = {"archive": str(archive), "sha256": sha(archive), "manifest": str(manifest_path), "manifest_sha256": sha(manifest_path)}
    return destination, manifest


def restore(campaign, archive, manifest_path):
    payload, manifest = unpack(campaign, archive, manifest_path)
    closure = document(payload / "closure.json")
    campaign.check("Closure source/profile/fixtures identical", closure["schema_version"] == 1 and closure["native_source"] == SOURCE and closure["bindings"] == campaign.receipt["bindings"])
    campaign.check("Closure original roots match external manifest", closure["original_roots"] == manifest["original_roots"])
    supplied = ROOT / campaign.receipt["profile"]["fixture_root"]
    for source in [supplied / name for name in ("schema.pg", "schema-next.pg", "queries.gq", "seed.jsonl")] + sorted(FIXTURE.iterdir()):
        campaign.check("Restored frozen input: " + source.name, sha(payload / "inputs" / source.name) == sha(source))
    campaign.check("Exactly all three declared pins retained", len(closure["pins"]) == 3 and
                   [(p["epoch"], p["branch"]) for p in closure["pins"]] == [("epoch1", "retained_seed"), ("epoch2", "retained_seed"), ("epoch1", "retained_advanced")])
    for index, pin in enumerate(closure["pins"]):
        require(pin["root"] == pin["epoch"] + ".omni" and pin["query"] == "inputs/" + pin["epoch"] + ".gq", "Unsafe pin paths")
        schema = "schema.pg" if pin["epoch"] == "epoch1" else "schema-next.pg"
        campaign.check("Pin freezes exact schema/query bytes", pin["schema"] == "inputs/" + schema and
                       sha(payload / pin["schema"]) == pin["schema_sha256"] and sha(payload / pin["query"]) == pin["query_sha256"])
        campaign.check("Relocated native root differs from originals", str(payload / pin["root"]) not in closure["original_roots"])
        campaign.read_pin(f"restored-{index}-snapshot", payload, pin)
        campaign.read_pin(f"restored-{index}-retention", payload, pin, branch=pin["branch"])
    campaign.receipt.update(pins=closure["pins"], mapping=closure["mapping"], original_roots_unavailable=True,
                            independent_native_restore=True, relocated_roots=str(payload))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prepare", "restore"))
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--backup", type=Path)
    parser.add_argument("--manifest", type=Path)
    args = parser.parse_args()
    if args.mode == "restore" and (args.backup is None or args.manifest is None):
        parser.error("restore requires --backup and --manifest")
    if args.mode == "prepare" and (args.backup is not None or args.manifest is not None):
        parser.error("prepare does not accept backup inputs")
    work = args.work.absolute()
    require(work == work.resolve(), "Work path aliases or symlink ancestors refused")
    require(work.is_relative_to(Path("/evidence")) and work != Path("/evidence"), "Work must be a fresh child of /evidence")
    campaign = Campaign(work, args.mode)
    try:
        fixture = campaign.admit()
        if args.mode == "prepare":
            prepare(campaign, fixture)
        else:
            restore(campaign, args.backup.absolute(), args.manifest.absolute())
        campaign.receipt["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        campaign.receipt["error"] = f"{type(error).__name__}: {error}"
    finally:
        campaign.receipt["ended_ns"] = time.time_ns()
        campaign.receipt["outputs"] = {p.relative_to(work).as_posix(): {"sha256": sha(p), "bytes": p.stat().st_size}
                                       for p in sorted(work.iterdir()) if p.is_file() and p.name != "receipt.json"}
        save(work / "receipt.json", campaign.receipt)
    print(f"{campaign.receipt['status']}: {work / 'receipt.json'}; WF24 not_run")
    return 0 if campaign.receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
