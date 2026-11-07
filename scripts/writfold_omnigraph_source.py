#!/usr/bin/env python3
"""Build an isolated source candidate, not an Omnigraph/WF24 admission.

Online image provisioning and locked fetch precede a network-disabled build.
Each invocation reserves a new external receipt and retains failed attempt logs.
The archive metadata is deterministic; compiler output reproducibility is not claimed.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import tarfile
import time
import uuid

import writfold_evidence as evidence
import writfold_preflight

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("omnigraph", "omnigraph-server", "omnigraph-azure-admission")
PACKAGES = ("omnigraph-cli", "omnigraph-server", "omnigraph-azure-admission")


def snapshot(source: Path, expected: str) -> dict:
    def git(*args: str) -> bytes:
        return subprocess.run(["git", "--no-replace-objects", "-C", str(source), *args],
                              check=True, capture_output=True, timeout=60).stdout
    if git("rev-parse", "--show-toplevel").decode().strip() != str(source):
        raise ValueError("--source must be the checkout root")
    if git("rev-parse", "HEAD").decode().strip() != expected:
        raise ValueError("Source HEAD differs from expected full commit")
    if git("status", "--porcelain=v1", "--untracked-files=all"):
        raise ValueError("Source must be clean, including untracked files")
    # Compare every actual blob to the committed tree, bypassing index stat caches,
    # assume-unchanged/skip-worktree flags and checkout filters.
    tree = {}
    for entry in git("ls-tree", "-rz", "HEAD").split(b"\0"):
        if not entry:
            continue
        metadata, name = entry.split(b"\t", 1)
        mode, kind, oid = metadata.decode().split()
        if kind != "blob" or mode not in ("100644", "100755", "120000"):
            raise ValueError("Source submodules/special files are not supported")
        tree[name.decode()] = (mode, oid)
    files = {}
    for directory, dirs, names in os.walk(source, followlinks=False):
        if Path(directory) == source:
            dirs[:] = [name for name in dirs if name != ".git"]
        for name in dirs:
            if (Path(directory) / name).is_symlink():
                raise ValueError("Source contains a directory symlink")
        for name in names:
            path = Path(directory) / name
            relative = path.relative_to(source).as_posix()
            info = path.lstat()
            if relative not in tree:
                raise ValueError(f"Nontracked or nonregular source: {relative}")
            mode, oid = tree[relative]
            algorithm = "sha1" if len(oid) == 40 else "sha256"
            blob = hashlib.new(algorithm)
            digest = hashlib.sha256()
            if mode == "120000":
                if not stat.S_ISLNK(info.st_mode):
                    raise ValueError(f"Committed source link is not a symlink: {relative}")
                target = os.readlink(path)
                resolved = path.resolve(strict=True)
                if (Path(target).is_absolute() or not resolved.is_relative_to(source)
                        or not resolved.is_file() or resolved.relative_to(source).as_posix() not in tree):
                    raise ValueError(f"Source symlink leaves committed regular-file closure: {relative}")
                raw = os.fsencode(target)
                blob.update(f"blob {len(raw)}\0".encode())
                blob.update(raw)
                digest.update(raw)
            else:
                if not stat.S_ISREG(info.st_mode):
                    raise ValueError(f"Committed source file is not regular: {relative}")
                blob.update(f"blob {info.st_size}\0".encode())
                with path.open("rb") as stream:
                    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                        blob.update(chunk)
                        digest.update(chunk)
                if bool(info.st_mode & 0o111) != (mode == "100755"):
                    raise ValueError(f"Source mode differs from committed mode: {relative}")
            if blob.hexdigest() != oid:
                raise ValueError(f"Source differs from committed bytes: {relative}")
            files[relative] = {"sha256": digest.hexdigest(), "bytes": info.st_size, "git_mode": mode}
    if set(files) != set(tree):
        raise ValueError("Missing committed source files")
    return {"source_commit": expected, "files": files, "manifest_sha256": evidence.digest(files),
            "cargo_lock_sha256": files["Cargo.lock"]["sha256"]}


def observe(argv: list[str], *, required: bool = True) -> dict:
    result = subprocess.run(argv, capture_output=True, text=True, timeout=120, check=False)
    record = {"argv": argv, "exit_code": result.returncode,
              "stdout": result.stdout, "stderr": result.stderr}
    print(json.dumps(record), flush=True)
    if required and result.returncode:
        raise ValueError(f"Observation failed: {argv}")
    return record


def inside(bootstrap: bool) -> None:
    out = Path("/evidence")
    if bootstrap:
        tools = {}
        for name, argv in {
            "rustc": ["rustc", "--version", "--verbose"],
            "cargo": ["cargo", "--version", "--verbose"],
            "rustup": ["rustup", "show"], "protoc": ["protoc", "--version"],
            "clang": ["clang", "--version"], "cmake": ["cmake", "--version"],
            "cc": ["cc", "--version"], "ld": ["ld", "--version"],
        }.items():
            tools[name] = observe(argv)
            resolved = observe(["which", argv[0]])["stdout"].strip()
            tools[name]["executable"] = evidence.file_record(Path(resolved).resolve(strict=True))
        for name in ("rustc", "cargo"):
            path = Path(observe(["rustup", "which", name])["stdout"].strip())
            tools[name]["executable"] = evidence.file_record(path)
        packages = Path("/opt/writfold-omnigraph/distro-packages.tsv")
        evidence.publish(out / "build-tools.json", {"tools": tools,
                         "packages": packages.read_text(), "packages_sha256": evidence.sha(packages)})
        subprocess.run(["cargo", "fetch", "--locked"], check=True)
        return
    command = ["cargo", "build", "--release", "--locked", "--offline"]
    for package in PACKAGES:
        command.extend(["-p", package])
    subprocess.run(command, check=True)
    target = Path(os.environ["CARGO_TARGET_DIR"]) / "release"
    version_observation = observe([str(target / "omnigraph"), "version", "--json"])
    cli_version = json.loads(version_observation["stdout"])
    if not isinstance(cli_version.get("version"), str):
        raise ValueError("CLI version command did not report a version")
    observations, members = {}, {}
    for name in NAMES:
        binary = target / name
        if binary.is_symlink() or not binary.is_file():
            raise ValueError(f"Expected regular binary: {binary}")
        members[name] = evidence.sha(binary)
        version = observe([str(binary), "--version"], required=False)
        help_output = observe([str(binary), "--help"])
        elf = observe(["readelf", "-h", "-l", "-d", str(binary)])
        ldd = observe(["ldd", str(binary)])
        if "not found" in ldd["stdout"] + ldd["stderr"]:
            raise ValueError(f"Unresolved runtime dependency: {name}")
        dependencies = {}
        for line in ldd["stdout"].splitlines():
            match = re.search(r"(?:=>\s+)?(/\S+)\s+\(", line)
            if match:
                path = Path(match.group(1)).resolve(strict=True)
                dependencies[str(path)] = evidence.file_record(path)
        observations[name] = {"version": version, "help": help_output, "elf": elf,
                              "ldd": ldd, "dependencies": dependencies}
    archive = out / "omnigraph-linux-x86_64.tar.gz"
    with archive.open("xb") as raw, gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as tar:
            for name in sorted(NAMES):
                binary = target / name
                info = tarfile.TarInfo(name)
                info.size = binary.stat().st_size
                info.mode = 0o755
                info.mtime = info.uid = info.gid = 0
                with binary.open("rb") as stream:
                    tar.addfile(info, stream)
    archive.chmod(0o444)
    evidence.publish(out / "producer.json", {"version": cli_version["version"],
                     "cli_version": cli_version, "version_observation": version_observation,
                     "members": members, "observations": observations,
                     "archive": evidence.file_record(archive)})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--expect-source")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--timeout", type=int, default=14400)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--inside", action="store_true")
    mode.add_argument("--bootstrap", action="store_true")
    args = parser.parse_args()
    if args.inside or args.bootstrap:
        inside(args.bootstrap)
        return 0
    if not args.source or not args.receipt or not re.fullmatch(r"[0-9a-f]{40}", args.expect_source or ""):
        parser.error("--source, --expect-source <full SHA>, and --receipt are required")
    source = args.source.resolve(strict=True)
    receipt_path = args.receipt.absolute().parent.resolve() / args.receipt.name
    if receipt_path.is_relative_to(source):
        parser.error("Receipt must be outside both source checkouts")
    attempt = evidence.Attempt(receipt_path, args.timeout)
    receipt = {"schema_version": 1, "scope": "source-candidate; not admission or WF24",
               "status": "fail", "source_commit": args.expect_source,
               "source": str(source), "commands": attempt.commands, "logs": attempt.logs,
               "start_ns": time.time_ns(), "reproducibility": "invocation only; online provisioning, not byte reproducibility"}
    previous = signal.signal(signal.SIGTERM, lambda signum, frame: (_ for _ in ()).throw(KeyboardInterrupt()))
    before = None
    try:
        # The unchanged operator storage/resource admission precedes all Docker build work.
        receipt["preflight"] = writfold_preflight.check()
        profile = evidence.document(ROOT / "qualification/omnigraph-profile.json")
        receipt["phases"] = {
            "provision": {"status": "not_run", "network": "online"},
            "fetch": {"status": "not_run", "network": "online", "locked": True},
            "compile": {"status": "not_run", "network": "none", "locked": True, "offline": True},
        }
        before = snapshot(source, args.expect_source)
        evidence.publish(attempt.directory / "source-before.json", before)
        receipt["source_before"] = evidence.file_record(attempt.directory / "source-before.json")
        receipt["definitions"] = {name: evidence.sha(ROOT / name) for name in
                                  ("scripts/writfold_omnigraph_source.py", "compose.omnigraph-build.yaml",
                                   "docker/Dockerfile.omnigraph-build", "qualification/omnigraph-profile.json",
                                   "scripts/writfold_evidence.py", "scripts/writfold_preflight.py")}
        lock = evidence.document(ROOT / "toolchains.lock.json")
        if lock.get("schema_version") != 1 or lock.get("admitted") is not True:
            raise ValueError("Tool lock is not admitted")
        base = lock["image"]["reference"]
        if not re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", base):
            raise ValueError("Tool base must be digest pinned")
        inspection = evidence.document(attempt.run(["docker", "image", "inspect", base]))
        if len(inspection) != 1 or inspection[0]["Id"] != lock["image"]["id"] or base not in inspection[0].get("RepoDigests", []):
            raise ValueError("Base image differs from admitted identity")
        observed = Path(lock["observation"]["path"])
        if not observed.is_absolute():
            observed = ROOT / observed
        if evidence.sha(observed) != lock["observation"]["sha256"]:
            raise ValueError("Admitted base observation changed")
        receipt["base_image"] = inspection[0]
        run_id = uuid.uuid4().hex
        # BuildKit cannot resolve this locally imported image by a registry
        # manifest digest. Give its verified image ID a content-named local tag;
        # check the binding and inherited layers, never fall back to another base.
        local_base = "writfold-omnigraph-base:" + lock["image"]["id"].replace(":", "-")
        attempt.run(["docker", "image", "tag", base, local_base])
        receipt["build_base_alias"] = local_base
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(("COMPOSE_", "WRITFOLD_OMNI_", "WRITFOLD_TOOL_"))}
        env.update(WRITFOLD_TOOL_IMAGE=local_base, WRITFOLD_OMNI_BUILD_IMAGE=f"writfold-omnigraph-source:{run_id}",
                   WRITFOLD_OMNI_SOURCE=str(source), WRITFOLD_OMNI_EVIDENCE=str(attempt.directory),
                   WRITFOLD_OMNI_RUN=run_id, DOCKER_BUILDKIT="1")
        compose = ["docker", "compose", "--project-name", f"writfold-omni-{run_id}",
                   "-f", str(ROOT / "compose.omnigraph-build.yaml")]
        attempt.run(compose + ["build", "provision"], env)
        receipt["phases"]["provision"]["status"] = "pass"
        built = evidence.document(attempt.run(["docker", "image", "inspect", env["WRITFOLD_OMNI_BUILD_IMAGE"]]))
        if len(built) != 1:
            raise ValueError("Missing unique derived image")
        alias = evidence.document(attempt.run(["docker", "image", "inspect", local_base]))
        layers = inspection[0]["RootFS"]["Layers"]
        if (len(alias) != 1 or alias[0]["Id"] != lock["image"]["id"]
                or built[0]["RootFS"]["Layers"][:len(layers)] != layers):
            raise ValueError("Derived image does not retain the admitted base")
        receipt["build_image"] = built[0]
        env["WRITFOLD_OMNI_BUILD_IMAGE"] = built[0]["Id"]
        for service in ("fetch", "compile"):
            name = f"writfold-omni-{run_id}-{service}"
            try:
                attempt.run(compose + ["run", "--rm", "--no-deps", "--name", name, service], env)
                receipt["phases"][service]["status"] = "pass"
            except BaseException:
                receipt["phases"][service]["status"] = "fail"
                old_timeout = attempt.timeout
                attempt.timeout = 30
                try:
                    attempt.run(["docker", "rm", "--force", name])
                finally:
                    attempt.timeout = old_timeout
                raise
        producer = evidence.document(attempt.directory / "producer.json")
        archive = attempt.directory / "omnigraph-linux-x86_64.tar.gz"
        if evidence.sha(archive) != producer["archive"]["sha256"]:
            raise ValueError("Producer archive digest mismatch")
        reported_commit = producer["cli_version"].get("commit")
        if reported_commit is not None and reported_commit != args.expect_source:
            raise ValueError("CLI reports a different source commit")
        with tarfile.open(archive, "r:gz") as tar:
            entries = tar.getmembers()
            if len(entries) != 3 or {entry.name for entry in entries} != set(NAMES):
                raise ValueError("Archive membership mismatch")
            for entry in entries:
                if not entry.isfile():
                    raise ValueError("Archive must contain only regular binaries")
                stream = tar.extractfile(entry)
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
                if digest != producer["members"][entry.name]:
                    raise ValueError("Archive member digest mismatch")
        receipt.update(archive=evidence.file_record(archive), members=producer["members"],
                       version=producer["version"], producer=evidence.file_record(attempt.directory / "producer.json"),
                       build_tools=evidence.file_record(attempt.directory / "build-tools.json"))
        for name, digest in receipt["definitions"].items():
            if evidence.sha(ROOT / name) != digest:
                raise ValueError(f"Build definition changed during invocation: {name}")
        receipt["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        receipt["error"] = f"{type(error).__name__}: {error}"
    finally:
        if before is not None:
            try:
                after = snapshot(source, args.expect_source)
                evidence.publish(attempt.directory / "source-after.json", after)
                receipt["source_after"] = evidence.file_record(attempt.directory / "source-after.json")
                if after != before:
                    raise ValueError("Source manifest changed during build")
            except (Exception, KeyboardInterrupt) as error:
                receipt.update(status="fail", source_after_error=str(error))
        receipt["end_ns"] = time.time_ns()
        evidence.publish(attempt.receipt, receipt)
        signal.signal(signal.SIGTERM, previous)
    if receipt["status"] != "pass":
        print(f"Source build failed; evidence: {attempt.receipt}", file=sys.stderr)
        return 1
    profile.update(scope="source-candidate; isolated development unblocker, not release or WF24 admission",
                   version=receipt["version"], source_commit=args.expect_source,
                   archive_url=Path(receipt["archive"]["path"]).as_uri(),
                   archive_sha256=receipt["archive"]["sha256"], archive_bytes=receipt["archive"]["bytes"],
                   members=receipt["members"], member_provenance="Built from pinned source; observed hashes in source build receipt",
                   server_reported_version=None,
                   server_version_note="Actual --version and --help results retained in producer.json; no inferred server-reported version",
                   dependency_scope="ldd-resolved ELF bytes observed inside recorded derived build image; not exhaustive runtime loading",
                   source_build_receipt={"path": str(attempt.receipt), "sha256": evidence.sha(attempt.receipt)})
    evidence.publish(attempt.directory / "profile.json", profile)
    print(json.dumps({"receipt": str(attempt.receipt), "profile": str(attempt.directory / "profile.json"),
                      "archive": receipt["archive"]["path"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
