#!/usr/bin/env python3
"""Offline, source-bound nominal/quantity Verus producer; use the verus service."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import uuid

from writfold_verus_evidence import ADMISSION_CONTROLS, Evidence, EvidenceError, mutation_functions, require, self_controls

ROOT = Path(__file__).resolve().parent.parent
COMMAND = ["cargo", "verus", "verify", "-p", "writfold-protocol", "--locked", "--offline",
           "--message-format=json", "--", "--output-json", "--time-expanded"]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob("*")) if p.is_file()}


def readable(root):
    root.chmod(0o755)
    for path in root.rglob("*"):
        if path.is_dir():
            path.chmod(0o755)
        elif path.is_file():
            path.chmod(path.stat().st_mode | 0o444)


def copy_workspace(destination):
    destination.mkdir(mode=0o755)
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        shutil.copy2(ROOT / name, destination / name)
    shutil.copytree(ROOT / "crates", destination / "crates")
    (destination / "qualification").mkdir(mode=0o755)
    shutil.copy2(ROOT / "qualification/trusted-boundaries.json",
                 destination / "qualification/trusted-boundaries.json")
    if (ROOT / ".cargo").exists():
        shutil.copytree(ROOT / ".cargo", destination / ".cargo")
    readable(destination)


def run(command, cwd, directory, environment, timeout):
    directory.mkdir(parents=True, exist_ok=True, mode=0o755)
    receipt = {"command": command, "cwd": str(cwd), "environment": environment,
               "started_at": datetime.now(timezone.utc).isoformat(), "timeout_seconds": timeout}
    save(directory / "command.json", receipt)
    with (directory / "output.log").open("wb") as log:
        process = subprocess.Popen(command, cwd=cwd, env=environment, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            receipt["returncode"] = process.wait(timeout=timeout)
            receipt["timed_out"] = False
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            receipt["returncode"] = process.wait()
            receipt["timed_out"] = True
    receipt["finished_at"] = datetime.now(timezone.utc).isoformat()
    receipt["log_sha256"] = digest(directory / "output.log")
    save(directory / "command.json", receipt)
    require(not receipt["timed_out"], f"child timeout: {directory}")
    require(receipt["returncode"] >= 0, f"child signal/tool failure: {directory}")
    return receipt["returncode"], (directory / "output.log").read_text()


def environment(target, profile):
    # Do not inherit arbitrary compiler wrappers, flags, credentials, or selectors.
    names = ("PATH", "HOME", "RUSTUP_HOME", "CARGO_HOME", "RUSTUP_TOOLCHAIN", "VERUS_Z3_PATH",
             "LD_LIBRARY_PATH", "CARGO_BUILD_JOBS")
    result = {name: os.environ[name] for name in names if name in os.environ}
    result.update({"CARGO_TARGET_DIR": str(target), "CARGO_NET_OFFLINE": "true",
                   "CARGO_BUILD_TARGET": profile["verification_target"], "CARGO_TERM_COLOR": "never",
                   "LC_ALL": "C.UTF-8"})
    return result


def identities(directory, workspace, env, timeout):
    observations = {}
    for name, command in (("cargo", ["cargo", "--version"]),
                          ("rustc", ["rustc", "-vV"]),
                          ("verus", ["verus", "--version"]),
                          ("rustc-path", ["rustup", "which", "rustc"]),
                          ("cargo-path", ["rustup", "which", "cargo"]),
                          ("metadata", ["cargo", "metadata", "--locked", "--offline", "--format-version=1"])):
        status, text = run(command, workspace, directory / name, env, timeout)
        require(status == 0, f"identity command failed: {name}")
        observations[name] = text.strip()
    binaries = {}
    for name in ("cargo-verus", "verus", "rust_verify"):
        path = shutil.which(name, path=env.get("PATH"))
        require(path is not None, f"missing tool: {name}")
        binaries[name] = {"path": path, "sha256": digest(Path(path).resolve())}
    for name in ("rustc", "cargo"):
        path = Path(observations[name + "-path"])
        binaries[name] = {"path": str(path), "sha256": digest(path)}
    z3 = Path(env["VERUS_Z3_PATH"])
    binaries["z3"] = {"path": str(z3), "sha256": digest(z3)}
    # Distribution includes the bundled vstd and compiler arrangement, not just a launcher.
    verus_root = Path(binaries["verus"]["path"]).resolve().parent
    save(directory / "verus-distribution-hashes.json", inventory(verus_root))
    metadata = json.loads(observations.pop("metadata"))
    dependencies = {}
    for package in metadata["packages"]:
        package_root = Path(package["manifest_path"]).parent
        if not package_root.is_relative_to(workspace):
            dependencies[package["id"]] = {"root": str(package_root), "files": inventory(package_root)}
    save(directory / "dependency-source-hashes.json", dependencies)
    save(directory / "identities.json", {"commands": observations, "executables": binaries,
                                         "resolved_features": metadata["resolve"]})
    return dependencies


def target_environment(profile):
    cache = Path(os.environ.get("CARGO_TARGET_DIR", ""))
    require(cache.is_absolute() and cache.is_relative_to(Path("/cache/target")),
            "verification requires the admitted Docker-managed target cache")
    target = cache / "verus" / uuid.uuid4().hex
    target.mkdir(parents=True, mode=0o755)
    return environment(target, profile)


def inspect_workspace(directory, workspace, profile, timeout):
    env = target_environment(profile)
    before = inventory(workspace)
    status, text = run(["cargo", "metadata", "--locked", "--offline", "--all-features",
                        "--format-version=1"], workspace, directory / "metadata", env, timeout)
    require(status == 0, "source trust metadata failed")
    metadata = directory / "metadata.json"
    save(metadata, json.loads(text))
    report = directory / "source-audit.json"
    status, _ = run(["cargo", "run", "-p", "writfold-xtask", "--locked", "--offline", "--",
                     "quality", "--root", str(workspace), "--metadata", str(metadata),
                     "--report", str(report)], workspace, directory / "audit", env, timeout)
    require(status == 0, "source trust/syntax admission failed")
    audit = json.loads(report.read_text())
    require(audit.get("complete") is True and audit.get("success") is True,
            "incomplete source trust/syntax admission")
    available = {(item["source"], item["name"], tuple(item["lines"]))
                 for item in audit["declarations"]}
    for spec in profile["crates"].values():
        for name, function in spec["functions"].items():
            require(function["declarations"], "missing declared production source")
            for declaration in function["declarations"]:
                require((declaration["source"], name.split("::")[-1],
                         tuple(declaration["lines"])) in available,
                        f"stale syntax-derived production linkage: {name}")
    require(inventory(workspace) == before, "source audit changed candidate inputs")
    return audit


def verify_attempt(directory, workspace, profile, timeout, mutation=None):
    env = target_environment(profile)
    before = inventory(workspace)
    save(directory / "source-hashes.json", before)
    if mutation:
        save(directory / "mutation.json", mutation)
    status, text = run(COMMAND, workspace, directory / "verify", env, timeout)
    require(inventory(workspace) == before, "verifier changed input sources or lock")
    evidence = Evidence.parse(text)
    save(directory / "reports.json", evidence.records)
    admitted = (evidence.mutant(status, profile, mutation, workspace) if mutation
                else evidence.positive(status, profile))
    if not mutation:
        for crate in profile["crates"]:
            artifacts = [r for r in evidence.records if r.get("reason") == "compiler-artifact"
                         and r.get("target", {}).get("name") == crate]
            expected = workspace / "crates" / crate.replace("_", "-") / "src/lib.rs"
            require(Path(artifacts[0]["target"]["src_path"]).resolve() == expected.resolve(),
                    "compiler artifact does not name this attempt's source")
    save(directory / "admission.json", admitted)
    return evidence


def produce(args, run_root, summary):
    profile = json.loads((ROOT / "qualification/verus-profile.json").read_text())
    require(profile["schema_version"] == 4, "unsupported Verus profile schema")
    summary["required_mutations"] = [mutation["name"] for mutation in profile["mutations"]]
    summary.update({"scope": profile["scope"], "excluded_claims": profile["excluded_claims"],
                    "verification_target": profile["verification_target"],
                    "shipping_target": profile["shipping_target"]})
    inputs = run_root / "inputs"
    inputs.mkdir(mode=0o755)
    for name in ("qualification/verus-profile.json", "toolchains.lock.json",
                 "qualification/trusted-boundaries.json", "qualification/implementation-decisions.json",
                 "scripts/writfold_verus.py", "scripts/writfold_verus_evidence.py"):
        destination = inputs / name
        destination.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
        shutil.copy2(ROOT / name, destination)
    positive_dir = run_root / "positive"
    positive_dir.mkdir(mode=0o755)
    workspace = positive_dir / "source"
    copy_workspace(workspace)
    inspect_workspace(run_root / "source-admission", workspace, profile, args.timeout)
    summary["source_admission"] = "pass"
    dependencies = identities(run_root / "identity", workspace,
                              environment(run_root / "identity-target", profile), args.timeout)
    positive = verify_attempt(positive_dir, workspace, profile, args.timeout)
    summary["positive"] = "pass"
    baseline = inventory(workspace)
    for mutation in profile["mutations"]:
        attempt = run_root / mutation["name"]
        attempt.mkdir(mode=0o755)
        clone = attempt / "source"
        shutil.copytree(workspace, clone)
        readable(clone)
        names = mutation_functions(profile, mutation)
        spec = profile["crates"][mutation["crate"]]["functions"][names[0]]
        source = clone / spec["declarations"][0]["source"]
        text = source.read_text()
        require(text.count(mutation["before"]) == 1, "mutation must match exactly once")
        source.write_text(text.replace(mutation["before"], mutation["after"]))
        changed = inventory(clone)
        require({p for p in baseline if baseline[p] != changed.get(p)} == {spec["declarations"][0]["source"]}
                and baseline.keys() == changed.keys(), "mutant changed unrelated sources")
        relative = spec["declarations"][0]["source"]
        attribution = {**mutation, "source": relative, "before_sha256": baseline[relative],
                       "after_sha256": changed[relative]}
        try:
            negative = verify_attempt(attempt, clone, profile, args.timeout, attribution)
            controls = self_controls(profile, positive, negative, mutation, clone)
            save(attempt / "admission-self-controls.json", controls)
            summary["mutations"][mutation["name"]] = {"outcome": "pass", "controls": controls}
        except (EvidenceError, OSError, ValueError, KeyError) as error:
            summary["mutations"][mutation["name"]] = {"outcome": "fail", "error": str(error)}
    for dependency in dependencies.values():
        require(inventory(Path(dependency["root"])) == dependency["files"],
                "dependency source changed during verification")
    require(inventory(workspace) == baseline, "positive source changed during calibration")
    require(all(m["outcome"] == "pass" for m in summary["mutations"].values())
            and len(summary["mutations"]) == len(profile["mutations"]), "negative calibration failed")
    summary["outcome"] = "pass"


def checks_for(summary):
    checks = [
        {"name": "proof-trust-and-source-admission", "required": True,
         "status": "pass" if summary["source_admission"] == "pass" else "fail"},
        {"name": "positive-production-proof", "required": True,
         "status": "pass" if summary["positive"] == "pass" else "fail"},
        {"name": "source-and-dependency-stability", "required": True,
         "status": "pass" if summary["outcome"] == "pass" else "fail"},
    ]
    for name in summary.get("required_mutations", []):
        mutation = summary["mutations"].get(name, {})
        checks.append({"name": "mutation:" + name, "required": True,
                       "status": "pass" if mutation.get("outcome") == "pass" else "fail"})
        for control in ADMISSION_CONTROLS:
            outcomes = [row.get("outcome") for row in mutation.get("controls", [])
                        if row.get("name") == control]
            checks.append({"name": f"admission:{name}:{control}", "required": True,
                           "status": "pass" if outcomes == ["rejected"] else "fail"})
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=Path("/evidence"))
    parser.add_argument("--timeout", type=int, default=1800)
    args = parser.parse_args()
    require(args.timeout > 0, "timeout must be positive")
    os.umask(0o022)
    args.evidence.mkdir(parents=True, exist_ok=True, mode=0o755)
    run_root = args.evidence / ("verus-" + uuid.uuid4().hex)
    run_root.mkdir(mode=0o755)
    summary = {"schema_version": 1, "producer": "writfold_verus.py", "outcome": "fail",
               "run_directory": str(run_root), "positive": "not_run",
               "source_admission": "not_run", "mutations": {}}
    try:
        produce(args, run_root, summary)
    except (EvidenceError, OSError, ValueError, KeyError) as error:
        summary["error"] = str(error)
    finally:
        # Keep every failed attempt; exclude build outputs from the artifact hash walk.
        artifacts = {}
        for path in sorted(run_root.rglob("*")):
            relative = path.relative_to(run_root)
            if path.is_file() and "target" not in relative.parts and "identity-target" not in relative.parts:
                artifacts[str(relative)] = digest(path)
        save(run_root / "artifact-hashes.json", artifacts)
        summary["artifact_manifest_sha256"] = digest(run_root / "artifact-hashes.json")
        summary.update(checks=checks_for(summary), skips=[])
        save(run_root / "obligations.json", summary)
        temporary = args.evidence / (run_root.name + "-obligations.tmp")
        save(temporary, summary)
        temporary.replace(args.evidence / "obligations.json")
    print(json.dumps(summary, sort_keys=True))
    return 0 if summary["outcome"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
