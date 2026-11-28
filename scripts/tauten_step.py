"""The `tauten-check` quality step (#173, docs/rfcs/2026-11-27-tauten-adoption.md).

`cargo tauten check`, from the version pinned in `toolchains.lock.json` (#174),
passes when no error-level finding lies outside `tauten-baseline.json` and no
baseline slot is stale (TAU-R001). A proof record older than the kernel is
named before tauten runs: tauten reports one only as ordinary TAU-N001 and
TAU-P003 errors in the kernel (tauten-dev/tauten#100). No step here writes
`tauten-baseline.json` or `proofs/claims.json`.

Three calibrations run on copies of the tracked tree: a planted unchecked `+`
in `writfold-sources` fails and names TAU-N001 and the file; a comment edit
under `crates/writfold-core/src/` fails with the stale-record message; the
copy as committed passes.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

STALE = ("tauten-check: proofs/claims.json was recorded at {commit}; crates/writfold-core "
         "or proofs changed since. Run scripts/release claims and commit the record.")
INPUTS = ["crates/writfold-core", "proofs", ":!proofs/claims.json"]
PLANTED = "crates/writfold-sources/src/lib.rs"
KERNEL_COMMENT = "crates/writfold-core/src/lib.rs"


def pinned(root: Path) -> str | None:
    """The cargo-tauten version `toolchains.lock.json` pins."""
    lock = json.loads((root / "toolchains.lock.json").read_text())
    for tool in lock.get("artifacts", []):
        if tool.get("name") == "cargo-tauten":
            return tool.get("version")
    return None


def stale_record(tree: Path, git_dir: Path) -> str | None:
    """The stale-record message when the kernel or proofs in `tree` differ
    from the commit `proofs/claims.json` was recorded at; None when fresh."""
    try:
        commit = json.loads((tree / "proofs/claims.json").read_text()).get("commit")
    except (OSError, json.JSONDecodeError) as error:
        return f"tauten-check: proofs/claims.json cannot be read: {error}"
    if not commit:
        return "tauten-check: proofs/claims.json names no commit. Run scripts/release claims."
    argv = ["git", f"--git-dir={git_dir}", f"--work-tree={tree}", "diff", "--quiet", commit, "--", *INPUTS]
    code = subprocess.run(argv, cwd=tree, capture_output=True, check=False).returncode
    if code == 0:
        return None
    if code == 1:
        return STALE.format(commit=commit)
    return f"tauten-check: cannot compare the tree with {commit}, the record's commit (git exit {code})"


def check(tree: Path, git_dir: Path, env: dict, timeout: int) -> tuple[int, str]:
    """The step on `tree`: exit status and what it printed."""
    message = stale_record(tree, git_dir)
    if message:
        return 1, message + "\n"
    env = dict(env, CARGO_NET_OFFLINE="true")
    try:
        done = subprocess.run(["cargo", "tauten", "check", "--manifest-path", str(tree / "Cargo.toml"),
                               "--format", "json"], cwd=tree, env=env, capture_output=True,
                              text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return 125, f"tauten-check: {error}\n"
    return done.returncode, done.stdout + done.stderr


def copy(root: Path, into: Path) -> Path:
    """A copy of the tracked files of `root` (no target, no .git)."""
    listed = subprocess.run(["git", "ls-files", "-z"], cwd=root, capture_output=True, check=True).stdout
    for name in filter(None, listed.decode().split("\0")):
        source = root / name
        if source.is_file() and not source.is_symlink():
            (into / name).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, into / name)
    return into


def append(path: Path, text: str) -> None:
    with path.open("a") as out:
        out.write(text)


def calibrations(root: Path, env: dict, timeout: int) -> list[tuple[str, bool, str]]:
    """Each calibration's name, whether it behaved as stated, and why not."""
    git_dir = root / ".git"
    results = []
    with tempfile.TemporaryDirectory(prefix="tauten-calibration-") as scratch:
        tree = copy(root, Path(scratch) / "tree")
        code, out = check(tree, git_dir, env, timeout)
        results.append(("committed-passes", code == 0, out[-400:]))
        append(tree / KERNEL_COMMENT, "// a comment edit stales the proof record\n")
        code, out = check(tree, git_dir, env, timeout)
        results.append(("stale-record-named", code != 0 and "recorded at" in out and "scripts/release claims" in out, out[-400:]))
        shutil.copy2(root / KERNEL_COMMENT, tree / KERNEL_COMMENT)
        append(tree / PLANTED, "\n#[doc(hidden)]\npub fn tauten_calibration(a: u32, b: u32) -> u32 {\n    a + b\n}\n")
        code, out = check(tree, git_dir, env, timeout)
        results.append(("unchecked-add-named", code != 0 and "TAU-N001" in out and "writfold-sources" in out, out[-400:]))
    return results


def version(env: dict) -> str:
    try:
        return subprocess.run(["cargo", "tauten", "--version"], env=env, capture_output=True,
                              text=True, check=False).stdout.strip()
    except OSError as error:
        return f"unavailable: {error}"


def run(producer) -> None:
    """Record the step and its calibrations on `producer` (writfold_quality.Producer)."""
    root, env = producer.root, producer.env
    found, want = version(env), pinned(root)
    if found != f"cargo-tauten {want}":
        producer.record("tauten-check", False, reason=f"tauten-check: {found!r} is not the pinned cargo-tauten {want}")
        return
    code, out = check(root, root / ".git", env, producer.timeout)
    (producer.evidence / "tauten-check.stdout").write_text(out)
    producer.record("tauten-check", code == 0, exit_status=code, tool=found)
    for name, held, why in calibrations(root, env, producer.timeout):
        producer.record(f"tauten-calibration-{name}", held, **({} if held else {"output": why}))


if __name__ == "__main__":
    import sys
    here = Path(__file__).resolve().parents[1]
    code, out = check(here, here / ".git", dict(os.environ), 900)
    sys.stdout.write(out if code else "tauten-check: ok\n")
    if "--calibrate" in sys.argv:
        for name, held, why in calibrations(here, dict(os.environ), 900):
            print(f"calibration {name}: {'ok' if held else 'FAIL ' + why}")
    raise SystemExit(code)
