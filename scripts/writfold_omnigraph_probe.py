#!/usr/bin/env python3
"""Run source-pinned native foundation campaigns; never issue a WF24 gate pass."""
from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import time

import writfold_evidence as evidence

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "qualification/omnigraph-profile.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def extract(archive: Path, destination: Path, profile: dict) -> None:
    require(archive.is_file() and not archive.is_symlink(), "Archive must be a regular retained file")
    require(archive.stat().st_size == profile["archive_bytes"] and
            evidence.sha(archive) == profile["archive_sha256"], "Archive digest/size mismatch")
    destination.mkdir(mode=0o700)
    # Explicit allowlist: no links, directories, traversal, devices or duplicate names.
    with tarfile.open(archive, "r:gz") as package:
        members = package.getmembers()
        require(len(members) == len(profile["members"]) and
                {m.name for m in members} == set(profile["members"]), "Unexpected archive members")
        for member in members:
            require(member.isfile() and member.name == Path(member.name).name and
                    0 < member.size < 600_000_000, "Unsafe archive member")
            target = destination / member.name
            with package.extractfile(member) as source, target.open("xb") as output:
                shutil.copyfileobj(source, output)
            require(evidence.sha(target) == profile["members"][member.name],
                    f"Executable digest mismatch: {member.name}")
            target.chmod(0o555)


def inside(campaign: str) -> None:
    work = Path("/evidence")
    profile = evidence.document(PROFILE)
    lock = evidence.document(ROOT / "toolchains.lock.json")
    observation_path = ROOT / lock["observation"]["path"]
    require(evidence.sha(observation_path) == lock["observation"]["sha256"], "Tool observation changed")
    tools = {item["name"]: item for item in evidence.document(observation_path)["tools"]}
    for name in ("python", "dolt"):
        require(evidence.sha(Path(tools[name]["executable"])) == tools[name]["sha256"],
                f"Admitted {name} executable changed")
    require(Path(sys.executable).resolve() == Path(tools["python"]["executable"]),
            "Unexpected Python interpreter")
    for name, digest in profile["members"].items():
        require(evidence.sha(Path("/native") / name) == digest, f"Package changed: {name}")
    home = work / "home"
    home.mkdir(mode=0o700)
    env = {"PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": str(home), "TMPDIR": "/tmp",
           "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
           "PYTHONDONTWRITEBYTECODE": "1", "RUST_BACKTRACE": "0", "RAYON_NUM_THREADS": "2",
           "TOKIO_WORKER_THREADS": "2", "OMP_NUM_THREADS": "2", "OPENBLAS_NUM_THREADS": "2",
           "GOMAXPROCS": "2", "DOLT_DISABLE_VERSION_CHECK": "1"}
    commands = []

    def run(name: str, argv: list[str], allowed: tuple[int, ...] = (0,), timeout: int = 120) -> str:
        record = {"name": name, "argv": argv, "start_ns": time.time_ns(), "exit": None}
        commands.append(record)
        try:
            with (work / f"{name}.stdout").open("xb") as stdout, (work / f"{name}.stderr").open("xb") as stderr:
                result = subprocess.run(argv, cwd=work, env=env, stdout=stdout, stderr=stderr, timeout=timeout)
            record["exit"] = result.returncode
            require(result.returncode in allowed, f"{name} exited {result.returncode}; retained logs")
            return (work / f"{name}.stdout").read_text()
        finally:
            record["end_ns"] = time.time_ns()
            (work / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")

    dependencies = {}
    for name in profile["members"]:
        text = run(name + "-ldd", ["/usr/bin/ldd", "/native/" + name])
        require("not found" not in text, f"Unresolved dependency: {name}")
        paths = sorted(set(re.findall(r"(?:=>\s+|^\s*)(/[^\s]+)", text, re.MULTILINE)))
        require(bool(paths), f"No dynamic dependency closure observed for {name}")
        dependencies[name] = [evidence.file_record(Path(path)) for path in paths]
    runtime = {"machine": platform.machine(), "platform": platform.platform(),
               "python": run("python-version", [sys.executable, "--version"]),
               "libc": run("libc-version", ["/usr/bin/ldd", "--version"]),
               "os_release": Path("/etc/os-release").read_text(),
               "distro_packages": evidence.file_record(Path("/opt/writfold-toolchain/distro-packages.tsv")),
               "dependency_observer": evidence.file_record(Path("/usr/bin/ldd")),
               "dependencies": dependencies, "environment": env,
               "executables": {name: evidence.file_record(Path("/native") / name) for name in profile["members"]}}
    version = json.loads(run("cli-version", ["/native/omnigraph", "version", "--json"]))
    require(version["version"] == profile["version"], "CLI version mismatch")
    runtime["cli_version"] = version
    run("server-help", ["/native/omnigraph-server", "--help"])
    run("server-version", ["/native/omnigraph-server", "--version"], allowed=(2,))
    runtime["server_reported_version"] = None
    runtime["server_version_observation"] = "--version refused with exit 2; retained stdout/stderr; version observation alone starts no lifecycle"
    evidence.publish(work / "runtime.json", runtime)
    fixture = ROOT / profile["fixture_root"]
    for name, expected in profile["fixture_sha256"].items():
        require(evidence.sha(fixture / name) == expected, f"Immutable fixture changed: {name}")
    # Independent scopes use separate fresh graph roots, never a second live writer.
    if campaign == "companion":
        run("companion", [sys.executable, str(ROOT / "scripts/writfold_omnigraph_companion.py"),
                          "--work", str(work / "companion")], timeout=660)
    else:
        run("compare", [sys.executable, str(fixture / "compare.py"), "--omnigraph", "/native/omnigraph",
                        "--dolt", tools["dolt"]["executable"], "--out", str(work / "compare")], timeout=1500)
        run("versioned-schema", [sys.executable, str(fixture / "versioned_schema.py"), "--omnigraph",
                                 "/native/omnigraph", "--out", str(work / "versioned-schema")], timeout=900)
        run("epochs-prepare", [sys.executable, str(ROOT / "scripts/writfold_omnigraph_epochs.py"),
                               "prepare", "--work", str(work / "epochs")], timeout=900)


def native_container(attempt: evidence.Attempt, env: dict, command: list[str] | None = None) -> None:
    name = f"writfold-omnigraph-{os.getpid()}-{time.monotonic_ns()}"
    argv = ["docker", "compose", "--env-file", "/dev/null", "--project-directory", str(ROOT),
            "-f", str(ROOT / "compose.omnigraph.yaml"), "run", "--rm", "--no-deps", "-T",
            "--name", name, "probe", *(command or [])]
    try:
        attempt.run(argv, env)
    except BaseException:
        # Reap the container, not only its client, before releasing owned capacity.
        attempt.run(["docker", "rm", "--force", name])
        raise


def independent_restore(attempt: evidence.Attempt, env: dict) -> dict:
    mount = attempt.directory / "restore-work"
    incoming = mount / "incoming"
    incoming.mkdir(mode=0o700, parents=True)
    for name in ("backup.tar", "backup.json"):
        source = attempt.directory / "work/epochs" / name
        require(source.is_file() and not source.is_symlink(), "Backup input must be regular")
        with source.open("rb") as original, (incoming / name).open("xb") as copied:
            shutil.copyfileobj(original, copied)
        require(evidence.sha(source) == evidence.sha(incoming / name), "Backup transfer changed bytes")
    isolated = dict(env, WRITFOLD_PROBE_WORK=str(mount))
    native_container(attempt, isolated, [
        "/usr/bin/python3", "/work/writfold/scripts/writfold_omnigraph_epochs.py", "restore",
        "--work", "/evidence/restore", "--backup", "/evidence/incoming/backup.tar",
        "--manifest", "/evidence/incoming/backup.json"])
    report = evidence.document(mount / "restore/receipt.json")
    require(report["status"] == "pass" and report["original_roots_unavailable"] is True
            and report["independent_native_restore"] is True, "Independent restore did not pass")
    return {"receipt": report, "mount": str(mount),
            "inputs": [evidence.file_record(incoming / name) for name in ("backup.tar", "backup.json")],
            "isolation": "Fresh container mounts only copied backup inputs, never original native roots"}


def execute_campaigns(attempt: evidence.Attempt, env: dict, work: Path, campaign: str, receipt: dict) -> None:
    command = None if campaign == "all" else [
        "/usr/bin/python3", "/work/writfold/scripts/writfold_omnigraph_probe.py",
        "--inside", "--campaign", campaign]
    native_container(attempt, env, command)
    receipt["runtime"] = evidence.document(work / "runtime.json")
    receipt["campaign_receipts"] = {}
    if campaign == "all":
        receipt["probe_receipts"] = {name: evidence.document(work / name / "receipt.json")
                                     for name in ("compare", "versioned-schema")}
        epochs = evidence.document(work / "epochs/receipt.json")
        receipt["campaign_receipts"]["epochs"] = epochs
        require(epochs["status"] == "pass", "Epoch preparation failed")
        receipt["independent_restore"] = independent_restore(attempt, env)
        receipt["epochs_restore"] = "scoped-smoke-pass"
        native_container(attempt, env, [
            "/usr/bin/python3", "/work/writfold/scripts/writfold_omnigraph_companion.py",
            "--work", "/evidence/companion"])
    companion = evidence.document(work / "companion/receipt.json")
    receipt["campaign_receipts"]["companion"] = companion
    require(companion["status"] == "pass", "Companion campaign failed")
    shutdown = companion["shutdown"]
    require(shutdown["reaped"] and shutdown["group_absent"] and shutdown["endpoint_refused"]
            and not shutdown["forced_kill"] and shutdown["exit"] == 0,
            "Companion ownership was not cleanly discharged")
    receipt["server_lifecycle"] = "scoped-smoke-pass"


def host(args: argparse.Namespace) -> int:
    attempt = evidence.Attempt(args.receipt, 2700)
    receipt = {"schema_version": 1, "scope": f"Supplemental P01O native campaign: {args.campaign}",
               "selected_campaign": args.campaign,
               "wf24": "not_run", "server_lifecycle": "not_run", "epochs_restore": "not_run",
               "status": "fail", "commands": attempt.commands, "started_ns": time.time_ns()}
    try:
        with (attempt.directory / "exclusive.lock").open("xb") as exclusive:
            fcntl.flock(exclusive, fcntl.LOCK_EX | fcntl.LOCK_NB)
            preflight = attempt.run([sys.executable, str(ROOT / "scripts/writfold_preflight.py")])
            receipt["preflight"] = evidence.document(preflight)
            receipt["source"] = evidence.snapshot()
            profile = evidence.document(PROFILE)
            receipt["profile"] = profile
            lock = evidence.document(ROOT / "toolchains.lock.json")
            require(lock.get("admitted") is True and lock.get("schema_version") == 1, "Tool image not admitted")
            image = lock["image"]["reference"]
            require(re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", image) is not None,
                    "Mutable image reference refused")
            observed = Path(lock["observation"]["path"])
            if not observed.is_absolute():
                observed = ROOT / observed
            require(evidence.sha(observed) == lock["observation"]["sha256"], "Tool observation digest mismatch")
            inspection = evidence.document(attempt.run(["docker", "image", "inspect", image]))
            require(len(inspection) == 1 and image in inspection[0].get("RepoDigests", []) and
                    inspection[0]["Id"] == lock["image"]["id"] and
                    inspection[0]["Architecture"] == "amd64" and inspection[0]["Os"] == "linux",
                    "Local image differs from immutable admission")
            receipt["tool_lock"] = lock
            archive = args.archive.expanduser().absolute()
            package = attempt.directory / "package"
            extract(archive, package, profile)
            receipt["archive"] = evidence.file_record(archive)
            fixture = ROOT / profile["fixture_root"]
            receipt["fixtures"] = {p.name: evidence.file_record(p) for p in sorted(fixture.iterdir()) if p.is_file()}
            require({name: item["sha256"] for name, item in receipt["fixtures"].items()} ==
                    profile["fixture_sha256"], "Immutable supplied fixture inventory changed")
            work = attempt.directory / "work"
            work.mkdir(mode=0o700)
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith(("COMPOSE_", "WRITFOLD_PROBE_", "WRITFOLD_TOOL_"))}
            env.update(WRITFOLD_TOOL_IMAGE=image, WRITFOLD_PROBE_WORK=str(work),
                       WRITFOLD_PROBE_PACKAGE=str(package), WRITFOLD_PROBE_UID=str(os.getuid()),
                       WRITFOLD_PROBE_GID=str(os.getgid()))
            execute_campaigns(attempt, env, work, args.campaign, receipt)
            require(evidence.snapshot()["tree_digest"] == receipt["source"]["tree_digest"],
                    "Checkout changed during attempt; refuse source-bound success")
            receipt["status"] = "pass"
    except (Exception, KeyboardInterrupt) as error:
        receipt["error"] = f"{type(error).__name__}: {error}"
    finally:
        receipt["ended_ns"] = time.time_ns()
        receipt["outputs"] = [evidence.file_record(p) for p in sorted(attempt.directory.rglob("*"))
                              if p.is_file() and not p.is_symlink()]
        evidence.publish(attempt.receipt, receipt)
    print(f"{receipt['status']}: {attempt.receipt}; WF24 remains not_run")
    return 0 if receipt["status"] == "pass" else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--archive", type=Path, default=Path.home() /
                        ".local/share/writfold/native/omnigraph/0.14.0/omnigraph-linux-x86_64.tar.gz")
    parser.add_argument("--campaign", choices=("all", "companion"), default="all",
                        help="Select a supplemental scope; no selection passes WF24")
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside:
        inside(args.campaign)
        return 0
    if args.receipt is None:
        parser.error("--receipt is required (fresh out-of-tree path)")
    return host(args)


if __name__ == "__main__":
    raise SystemExit(main())
