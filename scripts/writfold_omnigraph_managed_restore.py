#!/usr/bin/env python3
"""Fixed-canonical-URI native cluster backup/restore, not managed-root relocation."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import signal
import stat
import tarfile
import time

from writfold_omnigraph_companion import require, save, sha
from writfold_omnigraph_probe import validate_profile
import writfold_omnigraph_workload as workload

BASE = Path("/evidence/workload")
MAX_MANIFEST = 64 * 1024 * 1024
MAX_MEMBERS = 200_000
MAX_ARCHIVE = 3 * 1024 * 1024 * 1024


def identity(path):
    observed = path.stat()
    return {"device": observed.st_dev, "inode": observed.st_ino}


def inventory(root):
    root_info = root.lstat()
    require(stat.S_ISDIR(root_info.st_mode) and root_info.st_mode & 0o7000 == 0,
            "Managed native root must be an ordinary directory, not a link")
    result = {}
    for path in root.rglob("*"):
        require(len(result) < MAX_MEMBERS, "Managed closure exceeds member bound")
        info = path.lstat()
        name = path.relative_to(root).as_posix()
        require(info.st_mode & 0o7000 == 0, "Unsupported native special mode")
        if stat.S_ISDIR(info.st_mode):
            result[name] = {"kind": "directory", "mode": stat.S_IMODE(info.st_mode)}
        elif stat.S_ISREG(info.st_mode):
            require(info.st_nlink == 1, "Managed backup profile refuses hardlinked native files")
            result[name] = {"kind": "file", "mode": stat.S_IMODE(info.st_mode),
                            "bytes": info.st_size, "sha256": sha(path)}
        else:
            raise ValueError("Managed closure contains a symlink or special file")
    require(result and any(item["kind"] == "file" for item in result.values()), "Missing native closure")
    return dict(sorted(result.items()))


def archive_root(root, destination, entries):
    with tarfile.open(destination, "x", format=tarfile.PAX_FORMAT) as archive:
        for name, entry in entries.items():
            info = tarfile.TarInfo(name)
            info.mode, info.mtime, info.uid, info.gid = entry["mode"], 0, 0, 0
            if entry["kind"] == "directory":
                info.type = tarfile.DIRTYPE
                archive.addfile(info)
            else:
                info.size = entry["bytes"]
                with (root / name).open("rb") as source:
                    archive.addfile(info, source)
    require(destination.stat().st_size <= MAX_ARCHIVE, "Managed archive exceeds declared bound")


def prepare(args, profile, receipt):
    source = args.workload
    require(source == BASE / "receipt.json", "Only the declared workload root is admitted")
    prior = json.loads(source.read_text())
    counts = profile["development_contract"]["workloads"]["asset_counts"]
    require(prior["status"] == "pass" and prior["completed_scales"] == counts,
            "Managed backup requires the complete successful workload")
    require(prior["profile_sha256"] == sha(args.profile)
            and prior["generator_sha256"] == sha(Path(workload.__file__))
            and all(check["pass"] for check in prior["checks"]), "Workload provenance changed")
    require(prior["commands"] and all(command["reaped"] and command["group_absent"]
            and command["logs_drained"] and not command["forced_kill"] for command in prior["commands"]),
            "Original native owners are not qualified quiescent")
    restore_inputs = prior["restore_inputs"]
    require([item["assets"] for item in restore_inputs] == counts, "Missing managed restore scale")
    manifest = {"schema": "writfold-managed-native-backup/1", "profile_sha256": sha(args.profile),
                "source_commit": profile["source_commit"], "workload_receipt_sha256": sha(source),
                "generator_sha256": prior["generator_sha256"], "fixtures": prior["fixtures"],
                "source_mount_identity": identity(Path("/evidence")), "roots": [],
                "scope": "Complete quiesced native roots at fixed canonical URIs; no location adoption"}
    for item in restore_inputs:
        count = item["assets"]
        root = BASE / str(count) / "cluster"
        require(item["quiescent"] is True and item["canonical_root"] == root.as_uri()
                and item["root_identity"] == identity(root),
                "Managed root does not have its exact quiescent physical binding")
        require(set(item["epochs"]) == {"1", "2"}, "Both immutable epoch graphs required")
        for epoch, record in item["epochs"].items():
            require(record["graph"] == f"workload-e{epoch}" and record["original_pin"]
                    and record["advanced_pin"] and record["retained_branch"]["name"]
                    and record["retained_branch"]["from_commit"] == record["original_pin"],
                    "Missing exact native pin/root")
            require(sha(Path(record["seed_file"])) == record["seed_sha256"], "Retained seed changed")
        require(sha(Path(item["mapping_file"])) == item["mapping_sha256"], "Retained mapping changed")
        before = inventory(root)
        observed = identity(root)
        archive = args.work / f"managed-{count}.tar"
        archive_root(root, archive, before)
        require(inventory(root) == before and identity(root) == observed, "Native root changed during backup")
        manifest["roots"].append({"assets": count, "canonical_root": root.as_uri(),
            "source_root_identity": observed, "entries": before, "archive": archive.name,
            "archive_bytes": archive.stat().st_size, "archive_sha256": sha(archive),
            "root_mode": stat.S_IMODE(root.stat().st_mode), "restore_input": item})
        receipt["checks"].append({"name": f"{count}: complete quiescent native backup", "pass": True})
    target = args.work / "manifest.json"
    save(target, manifest)
    require(target.stat().st_size <= MAX_MANIFEST, "Managed backup manifest exceeds bound")
    receipt.update(manifest={"path": str(target), "sha256": sha(target), "bytes": target.stat().st_size},
                   archives=[{key: item[key] for key in ("archive", "archive_bytes", "archive_sha256")}
                             for item in manifest["roots"]])


def safe_name(name):
    path = PurePosixPath(name)
    require(name and not path.is_absolute() and all(part not in ("", ".", "..") for part in name.split("/")),
            "Unsafe native archive path")
    return path


def unpack(archive, root, record):
    require(archive.is_file() and not archive.is_symlink()
            and archive.stat().st_size == record["archive_bytes"] <= MAX_ARCHIVE
            and sha(archive) == record["archive_sha256"], "Native backup archive mismatch")
    entries = record["entries"]
    require(0 < len(entries) <= MAX_MEMBERS, "Invalid native member count")
    require(sum(entry.get("bytes", 0) for entry in entries.values()) <= MAX_ARCHIVE,
            "Restored native bytes exceed the declared bound")
    root.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    root.mkdir(mode=0o700, exist_ok=False)
    with tarfile.open(archive, "r:") as source:
        members = []
        for member in source:
            require(len(members) < len(entries), "Native archive exceeds its declared member bound")
            members.append(member)
        require(len(members) == len(entries) and {m.name for m in members} == set(entries),
                "Native closure has duplicate/missing/unexpected members")
        for member in members:
            safe_name(member.name)
            require(member.sparse is None, "Sparse native archive members are not admitted")
            expected = entries[member.name]
            require((member.isdir() and expected["kind"] == "directory")
                    or (member.isfile() and expected["kind"] == "file"), "Native member kind mismatch")
            require(member.mode == expected["mode"] and member.mode & ~0o777 == 0, "Native member mode mismatch")
        for member in sorted(members, key=lambda m: (len(PurePosixPath(m.name).parts), m.name)):
            target = root / member.name
            if member.isdir():
                target.mkdir(mode=0o700, exist_ok=False)
            else:
                expected = entries[member.name]
                require(member.size == expected["bytes"], "Native member length mismatch")
                content = source.extractfile(member)
                require(content is not None, "Native member data missing")
                digest, size = hashlib.sha256(), 0
                with content, target.open("xb") as destination:
                    for chunk in iter(lambda: content.read(1024 * 1024), b""):
                        destination.write(chunk)
                        digest.update(chunk)
                        size += len(chunk)
                require(size == expected["bytes"] and digest.hexdigest() == expected["sha256"],
                        "Restored native member bytes differ")
                target.chmod(expected["mode"])
        for member in reversed(sorted(members, key=lambda m: len(PurePosixPath(m.name).parts))):
            if member.isdir():
                (root / member.name).chmod(member.mode)
    root.chmod(record["root_mode"])
    require(inventory(root) == entries, "Restored native closure differs before open")
    require(identity(root) != record["source_root_identity"], "Restore aliases the original physical root")


def exact_inventory(campaign, epoch, pin, advanced):
    start = time.monotonic()
    campaign.stage_deadline = start + campaign.slo["complete_inventory_seconds"]
    try:
        for query in ("inventory", "edges"):
            after, destination = "", ""
            for offset in range(0, campaign.count + 1, campaign.cfg["page_rows"]):
                expected = [workload.node(i, epoch, campaign.cfg["seed"]) if query == "inventory"
                            else workload.edge(i, campaign.count)
                            for i in range(offset, min(offset + campaign.cfg["page_rows"], campaign.count))]
                if advanced and query == "inventory" and offset == 0:
                    expected[0]["name"] = "advanced"
                params = {"after": after}
                if query == "edges":
                    params["destination"] = destination
                rows, _ = campaign.query(epoch, query, pin, params, expected)
                if rows:
                    after = rows[-1]["slug" if query == "inventory" else "source"]
                    if query == "edges":
                        destination = rows[-1]["destination"]
        campaign.check(f"{campaign.count}/e{epoch}/restored-{'advanced' if advanced else 'original'}-inventory",
                       time.monotonic() - start <= campaign.slo["complete_inventory_seconds"])
    finally:
        campaign.stage_deadline = None


def restore(args, profile, receipt):
    manifest_path = args.manifest
    require(manifest_path.is_file() and not manifest_path.is_symlink()
            and manifest_path.stat().st_size <= MAX_MANIFEST
            and sha(manifest_path) == args.manifest_sha256, "Managed manifest identity mismatch")
    manifest = json.loads(manifest_path.read_text())
    require(manifest["schema"] == "writfold-managed-native-backup/1"
            and manifest["profile_sha256"] == sha(args.profile)
            and manifest["source_commit"] == profile["source_commit"]
            and manifest["generator_sha256"] == sha(Path(workload.__file__)), "Managed restore profile/source mismatch")
    require(manifest["source_mount_identity"] != identity(Path("/evidence")), "Restore reuses the original physical mount")
    require(manifest["fixtures"] == {p.name: sha(p) for p in sorted(workload.FIXTURE.iterdir()) if p.is_file()},
            "Restored query/schema interpretation changed")
    counts = profile["development_contract"]["workloads"]["asset_counts"]
    require([record["assets"] for record in manifest["roots"]] == counts, "Incomplete managed restore inventory")
    receipt["source_mount_identity"] = manifest["source_mount_identity"]
    receipt["restore_mount_identity"] = identity(Path("/evidence"))
    receipt["root_observations"] = []
    for record in manifest["roots"]:
        count = record["assets"]
        root = BASE / str(count) / "cluster"
        require(record["canonical_root"] == root.as_uri(), "Managed canonical location may not change")
        name = record["archive"]
        require(Path(name).name == name and name == f"managed-{count}.tar", "Unexpected native archive binding")
        unpack(manifest_path.parent / name, root, record)
        receipt["root_observations"].append({"assets": count, "canonical_root": root.as_uri(),
            "source": record["source_root_identity"], "restored": identity(root), "byte_inventory_verified": True})
        campaign = workload.Campaign(root.parent, args.profile, count, receipt)
        try:
            token_file = root.parent / "restore-tokens.json"
            with open(token_file, "x", opener=lambda p, flags: os.open(p, flags, 0o600)) as output:
                json.dump({k: v for k, v in campaign.tokens.items() if k != "wrong"}, output)
            campaign.env["OMNIGRAPH_SERVER_BEARER_TOKENS_FILE"] = str(token_file)
            campaign.start()  # URI boot: no apply, init, migration, rebind or source bundle.
            for epoch in (1, 2):
                binding = record["restore_input"]["epochs"][str(epoch)]
                exact_inventory(campaign, epoch, binding["original_pin"], False)
                exact_inventory(campaign, epoch, binding["advanced_pin"], True)
                campaign.retained_read(epoch, binding["original_pin"], binding["retained_branch"]["name"])
                changed = workload.node(0, epoch, campaign.cfg["seed"])
                changed["name"] = "advanced"
                current, _ = campaign.request_native(epoch, "point", {
                    "params": {"key": workload.key(0)}, "branch": "main", "expect_mutation": False})
                campaign.check(f"{count}/e{epoch}/restored-current-head",
                    current["graph_commit_id"] == binding["advanced_pin"]
                    and workload.exact(workload.rows(current, epoch, "point"), [changed]))
            campaign.stop()
            after = inventory(root)
            added = sorted(set(after) - set(record["entries"]))
            removed = sorted(set(record["entries"]) - set(after))
            modified = {name: {"before": record["entries"][name], "after": after[name]}
                        for name in sorted(set(after) & set(record["entries"]))
                        if after[name] != record["entries"][name]}
            receipt["root_observations"][-1]["after_serving_delta"] = {
                "added": added, "removed": removed, "modified": modified}
            campaign.check(f"{count}/native-closure-unchanged-after-serving",
                           not added and not removed and not modified)
            receipt["checks"].append({"name": f"{count}: exact managed history and retained branches restored", "pass": True})
        finally:
            try:
                campaign.stop()
            finally:
                campaign.monitor_stop.set()
                campaign.monitor.join(timeout=2)
                require(not campaign.monitor.is_alive(), "Managed observer did not quiesce")
    receipt.update(independent_physical_restore=True, canonical_uris_preserved=True,
                   managed_root_relocation=False, source_bundle_used_for_boot=False)


def interrupted(signum, frame):
    raise TimeoutError(f"Managed restore interrupted by signal {signum}; native outcome is not replayed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prepare", "restore"))
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--workload", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--manifest-sha256")
    args = parser.parse_args()
    require(args.work.is_absolute() and args.work.parent.resolve().is_relative_to(Path("/evidence")), "Fresh work must be beneath /evidence")
    args.work.mkdir(mode=0o700, exist_ok=False)
    receipt = {"schema": "writfold-managed-native-restore/1", "scope": __doc__, "mode": args.mode,
               "status": "fail", "wf24": "not_run", "checks": [], "commands": [], "requests": [],
               "start_ns": time.time_ns(), "not_run": ["managed-root adoption at a new location", "owner-service recovery", "power-loss safety"]}
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM):
        signal.signal(sig, interrupted)
    try:
        profile = validate_profile(args.profile, Path("/profile/source-build.json"))
        bounds = profile["development_contract"]["managed_restore"]
        require(bounds["manifest_bytes"] == MAX_MANIFEST
                and bounds["maximum_members_per_root"] == MAX_MEMBERS
                and bounds["archive_bytes_per_root"] == MAX_ARCHIVE,
                "Managed restore bounds differ from the frozen profile")
        require(all(sha(Path("/native") / name) == digest for name, digest in profile["members"].items()), "Native executable mismatch")
        receipt.update(profile_sha256=sha(args.profile), source_commit=profile["source_commit"],
                       implementation_sha256=sha(Path(__file__)), generator_sha256=sha(Path(workload.__file__)),
                       bounds={"manifest_bytes": MAX_MANIFEST, "members": MAX_MEMBERS, "archive_bytes": MAX_ARCHIVE})
        if args.mode == "prepare":
            require(args.workload is not None, "Workload receipt required")
            prepare(args, profile, receipt)
        else:
            require(args.manifest is not None and args.manifest_sha256 is not None, "Exact backup manifest required")
            restore(args, profile, receipt)
        require(receipt["checks"] and all(check["pass"] for check in receipt["checks"]), "Managed checks incomplete")
        receipt["status"] = "pass"
    except BaseException as error:
        receipt["failure"] = {"class": type(error).__name__, "message": str(error)}
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        receipt["end_ns"] = time.time_ns()
        save(args.work / "receipt.json", receipt)
    return 0 if receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
