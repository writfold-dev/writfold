#!/usr/bin/env python3
"""DINT00 exclusive native Dolt fixture; not a Rust adapter or product workflow."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import time

from writfold_evidence import document, file_record, publish, sha, source_path
from writfold_stdio_process import Pipes

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "qualification/dolt-integration-profile.json"
MAX_BACKUP_BYTES = 128 * 1024 * 1024
COLUMNS = ("id", "text_value", "payload_hex", "mathematical_integer", "optional_value", "revision")
BRANCH_STATES = {"retained_seed": "seed", "side_history": "side", "main": "main"}
SELECT_ROWS = ("SELECT id,text_value,HEX(payload) AS payload_hex,mathematical_integer,"
               "optional_value,revision FROM exact_records")
SCHEMA = """CREATE TABLE exact_records (
    id VARCHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin PRIMARY KEY,
    text_value LONGTEXT CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
    payload LONGBLOB NOT NULL,
    mathematical_integer LONGTEXT CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
    optional_value LONGTEXT CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL,
    revision BIGINT NOT NULL
);"""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def inventory(root: Path) -> dict:
    """Inventory the complete stopped backup, refusing links and unsafe entries."""
    root = Path(root).absolute()
    require(root.resolve() == root and stat.S_ISDIR(root.lstat().st_mode),
            f"Unsafe backup root: {root}")
    result = {}
    total = 0
    pending = [root]
    while pending:
        directory = pending.pop()
        for path in sorted(directory.iterdir()):
            metadata = path.lstat()
            require(path.resolve().is_relative_to(root), f"Escaping backup path: {path}")
            if stat.S_ISDIR(metadata.st_mode):
                pending.append(path)
            else:
                require(stat.S_ISREG(metadata.st_mode), f"Link/special backup entry: {path}")
                total += metadata.st_size
                require(total <= MAX_BACKUP_BYTES, "Backup exceeds 128 MiB budget")
                result[path.relative_to(root).as_posix()] = {
                    "sha256": sha(path), "bytes": metadata.st_size}
                after = path.lstat()
                fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
                require(all(getattr(after, field) == getattr(metadata, field) for field in fields),
                        f"Backup changed during inventory: {path}")
    require(bool(result), "Empty backup inventory")
    return dict(sorted(result.items()))


def verify_backup(root: Path, expected: dict) -> None:
    """Require an exact inventory; the host uses the same admission for tampering."""
    require(isinstance(expected, dict) and bool(expected), "Invalid expected backup inventory")
    total = 0
    for name, entry in expected.items():
        require(isinstance(name, str), "Invalid backup inventory path")
        relative = PurePosixPath(name)
        require(name == relative.as_posix() and not relative.is_absolute()
                and name not in ("", ".") and ".." not in relative.parts,
                "Invalid backup inventory path")
        require(isinstance(entry, dict) and set(entry) == {"sha256", "bytes"}
                and isinstance(entry["sha256"], str)
                and re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]) is not None
                and type(entry["bytes"]) is int and entry["bytes"] >= 0,
                "Invalid backup inventory record")
        total += entry["bytes"]
    require(total <= MAX_BACKUP_BYTES, "Backup exceeds 128 MiB budget")
    require(inventory(root) == expected, "Backup inventory mismatch")


class CandidateRefusal(ValueError):
    """Executable digest was refused before spawning a native process."""


def admit_binary(path: Path, expected: str) -> dict:
    require(path.is_absolute() and stat.S_ISREG(path.lstat().st_mode),
            "Candidate must be an absolute regular executable")
    identity = file_record(path)
    if identity["sha256"] != expected:
        raise CandidateRefusal("Candidate digest mismatch")
    return identity


def native_hash(value: object) -> str:
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-v]{32}", value) is not None,
            "Invalid native Dolt commit hash")
    return value


def text_literal(value: str | None) -> str:
    if value is None:
        return "NULL"
    require(isinstance(value, str), "Expected exact UTF-8 text")
    return "CONVERT(X'" + value.encode("utf-8").hex() + "' USING utf8mb4)"


def fixture_rows(rows: object) -> None:
    require(isinstance(rows, list) and bool(rows), "Expected nonempty fixture rows")
    ids = []
    for row in rows:
        require(isinstance(row, dict) and set(row) == set(COLUMNS), "Invalid fixture columns")
        for column in ("id", "text_value", "mathematical_integer"):
            require(isinstance(row[column], str), f"Invalid fixture text: {column}")
        require(row["optional_value"] is None or isinstance(row["optional_value"], str),
                "Invalid optional fixture text")
        require(isinstance(row["payload_hex"], str)
                and re.fullmatch(r"(?:[0-9A-F]{2})*", row["payload_hex"]) is not None,
                "Invalid fixture binary encoding")
        require(re.fullmatch(r"-?(?:0|[1-9][0-9]*)", row["mathematical_integer"]) is not None,
                "Invalid exact decimal-text integer")
        require(type(row["revision"]) is int and 0 <= row["revision"] < 2**63,
                "Invalid fixture revision")
        ids.append(row["id"].encode("utf-8"))
    require(ids == sorted(set(ids)), "Fixture IDs must be unique and binary ordered")


class Campaign:
    def __init__(self, work: Path, mode: str, profile: dict):
        self.work, self.mode, self.profile = work, mode, profile
        self.pipes = Pipes(work / "commands")
        self.env = {
            "PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": str(work / "home"),
            "DOLT_ROOT_PATH": str(work / "dolt-root"), "DOLT_DISABLE_EVENT_FLUSH": "1",
            "DOLT_DISABLE_VERSION_CHECK": "1", "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8",
            "TZ": "UTC", "GOMAXPROCS": "2",
        }
        self.report = {
            "schema_version": 1, "profile": profile["profile"], "mode": mode, "status": "fail",
            "scope": profile["scope"], "started_ns": time.time_ns(), "environment": self.env,
            "checks": [{"name": name, "required": True, "status": "fail"}
                       for name in profile[f"required_{mode}_checks"]],
            "commands": self.pipes.records, "errors": [], "artifacts": {},
            "expected_refusals": [], "release_gates_passed": [],
            "omnigraph_wf24": "blocked_unchanged", "rust_adapter": "not_run", "agent_e2e": "not_run",
            "not_run": profile["excluded_claims"],
            "unknown": ["power-loss durability", "concurrent writers", "SQL-server lifecycle"],
        }
        self.database = work / "prepared/database" if mode == "prepare" else work / "restored"
        self.executable = Path(profile["candidate"]["executable"])
        self.expected = {}

    def passed(self, name: str) -> None:
        checks = [entry for entry in self.report["checks"] if entry["name"] == name]
        require(len(checks) == 1, f"Undeclared or duplicate required check: {name}")
        checks[0]["status"] = "pass"

    def run(self, name: str, arguments: list[str], *, cwd: Path | None = None,
            missing_history: bool = False) -> bytes:
        out, err, code = self.pipes.run(
            name, [str(self.executable), *arguments], [("eof", None)],
            cwd=cwd or self.database, env=self.env, timeout=120, limit=2 * 1024 * 1024)
        command = self.pipes.records[-1]
        if missing_history:
            # The pinned SQL AS OF boundary reports a missing target commit.
            # Require that refusal, not a syntax error or an arbitrary exit.
            diagnostic = err.decode("utf-8", errors="strict").rstrip()
            require(code == 1 and not out and diagnostic.endswith(": target commit not found"),
                    "Missing-history query did not produce the intended native commit refusal")
            command["expected_refusal"] = True
            command["refusal_kind"] = "missing-history"
            self.report["expected_refusals"].append({"command": name, "exit_code": code,
                                                    "diagnostic": "target commit not found"})
        else:
            require(code == 0, f"Native command failed: {name} (exit {code}); raw logs retained")
        return out

    def query(self, name: str, statement: str) -> list[dict]:
        self.run(name, ["sql", "--result-format", "json", "--query", statement])
        # The strict document reader also refuses duplicate keys and non-JSON constants.
        result = document(Path(self.pipes.records[-1]["artifacts"]["stdout"]["path"]))
        require(isinstance(result, dict) and set(result) == {"rows"}
                and isinstance(result["rows"], list)
                and all(isinstance(row, dict) for row in result["rows"]),
                f"Unexpected native SQL result shape: {name}")
        return result["rows"]

    def mutate(self, name: str, statement: str) -> None:
        self.run(name, ["sql", "--query", statement])

    def hashof(self, name: str, reference: str) -> str:
        require(reference in (*BRANCH_STATES, "HEAD")
                or re.fullmatch(r"[0-9a-v]{32}", reference) is not None,
                "Unsafe native reference")
        rows = self.query(name, f"SELECT HASHOF('{reference}') AS commit_hash;")
        require(len(rows) == 1 and set(rows[0]) == {"commit_hash"}, "Invalid native hash result")
        return native_hash(rows[0]["commit_hash"])

    def assert_rows(self, name: str, expected: list[dict], reference: str | None = None) -> None:
        suffix = ""
        if reference is not None:
            require(reference in BRANCH_STATES or re.fullmatch(r"[0-9a-v]{32}", reference) is not None,
                    "Unsafe pinned SQL reference")
            suffix = f" AS OF '{reference}'"
        rows = self.query(name, SELECT_ROWS + suffix + " ORDER BY BINARY id;")
        nulls = self.query(name + "-nulls",
                           "SELECT id,optional_value IS NULL AS optional_is_null FROM exact_records"
                           + suffix + " ORDER BY BINARY id;")
        require(nulls == [{"id": row["id"], "optional_is_null": int(row["optional_value"] is None)}
                          for row in expected], f"Native null/empty distinction mismatch: {name}")
        require(len(rows) == len(nulls), f"Native null probe row count mismatch: {name}")
        # The pinned CLI JSON writer omits SQL NULL keys. Decode only the nullable
        # column, independently confirmed by the native IS NULL query above.
        for row, null in zip(rows, nulls):
            require(row.get("id") == null["id"], f"Native null probe row identity mismatch: {name}")
            if null["optional_is_null"] == 1 and "optional_value" not in row:
                row["optional_value"] = None
        require(rows == expected, f"Exact ordered row mismatch: {name}")
        require(all(set(row) == set(COLUMNS) and type(row["revision"]) is int for row in rows),
                f"Exact row types/columns mismatch: {name}")

    def replace(self, name: str, rows: list[dict]) -> None:
        fixture_rows(rows)
        values = []
        for row in rows:
            values.append("(" + ",".join([
                text_literal(row["id"]), text_literal(row["text_value"]),
                "X'" + row["payload_hex"] + "'", text_literal(row["mathematical_integer"]),
                text_literal(row["optional_value"]), str(row["revision"]),
            ]) + ")")
        self.mutate(name, "DELETE FROM exact_records; INSERT INTO exact_records "
                    "(id,text_value,payload,mathematical_integer,optional_value,revision) VALUES "
                    + ",".join(values) + ";")

    def commit(self, name: str) -> str:
        self.run(name + "-add", ["add", "-A"])
        self.run(name + "-commit", ["commit", "--author", "WritfoldFixture <fixture@example.invalid>",
                                    "--date", "2026-11-05T00:00:00Z", "-m", "DINT00 " + name])
        return self.hashof(name + "-commit-hash", "HEAD")

    def admit(self) -> None:
        for name in ("commands", "home", "dolt-root"):
            (self.work / name).mkdir(mode=0o700)
        candidate = self.profile["candidate"]
        lock_path = source_path(candidate["tool_lock"])
        observation_path = source_path(candidate["observation"])
        fixture_path = source_path(self.profile["fixture"])
        lock, observation, fixture = document(lock_path), document(observation_path), document(fixture_path)
        require(lock["admitted"] is True and lock["observation"]["path"] == candidate["observation"]
                and sha(observation_path) == lock["observation"]["sha256"], "Tool observation identity mismatch")
        locked = [entry for entry in lock["artifacts"] if entry["name"] == "dolt"]
        observed = [entry for entry in observation["tools"] if entry["name"] == "dolt"]
        require(len(locked) == len(observed) == 1, "Ambiguous candidate tool identity")
        require(candidate["name"] == "dolt" and candidate["version"] == locked[0]["version"] == "2.6.1"
                and candidate["sha256"] == locked[0]["executable_sha256"] == observed[0]["sha256"]
                and candidate["archive_sha256"] == locked[0]["archive_sha256"]
                and str(self.executable) == observed[0]["executable"] == "/usr/local/bin/dolt"
                and observed[0]["version_output"].splitlines()[0] == "dolt version 2.6.1",
                "Profile/tool-lock/observation candidate mismatch")
        self.report["candidate"] = admit_binary(self.executable, candidate["sha256"])
        self.report["bindings"] = {str(path.relative_to(ROOT)): file_record(path) for path in (
            PROFILE, fixture_path, lock_path, observation_path, Path(__file__).resolve(),
            ROOT / "scripts/writfold_evidence.py", ROOT / "scripts/writfold_stdio_process.py",
            ROOT / "scripts/writfold_conformance_process.py")}
        version = self.run("candidate-version", ["version"], cwd=self.work).decode("utf-8")
        require(bool(version.splitlines()) and version.splitlines()[0] == "dolt version 2.6.1",
                "Native candidate version mismatch")
        self.passed("candidate-identity")
        require(fixture["schema_version"] == 1 and fixture["table"] == "exact_records", "Invalid fixture schema")
        self.expected = {name: fixture[name] for name in ("seed", "side", "main")}
        for rows in self.expected.values():
            fixture_rows(rows)
        if self.mode == "prepare":
            wrong = ("0" if candidate["sha256"][0] != "0" else "1") + candidate["sha256"][1:]
            try:
                admit_binary(self.executable, wrong)
            except CandidateRefusal as error:
                require(not self.database.exists(), "Wrong-candidate control ran after initialization")
                self.report["expected_refusals"].append({"kind": "wrong-candidate", "expected_sha256": wrong,
                                                        "error": str(error), "native_spawned": False})
            else:
                raise ValueError("Wrong candidate digest was admitted")
            self.passed("wrong-candidate-refusal")

    def assert_history(self, commits: dict, branches: dict, prefix: str) -> None:
        require(set(commits) == {"seed", "side", "main"} and set(branches) == set(BRANCH_STATES),
                "Invalid pin/branch manifest")
        require(len(set(commits.values())) == 3, "Seed, side and main commits must be distinct")
        for state, commit in commits.items():
            native_hash(commit)
            require(self.hashof(f"{prefix}-{state}-hash", commit) == commit, "Pinned hash identity mismatch")
            self.assert_rows(f"{prefix}-{state}-rows", self.expected[state], commit)
        for branch, state in BRANCH_STATES.items():
            require(native_hash(branches[branch]) == commits[state], "Branch pin/state mismatch")
            require(self.hashof(f"{prefix}-{branch}-root", branch) == branches[branch], "Native branch root mismatch")
            self.assert_rows(f"{prefix}-{branch}-rows", self.expected[state], branch)
        require(self.hashof(prefix + "-head", "HEAD") == commits["main"], "Current HEAD is not pinned main")
        self.assert_rows(prefix + "-working", self.expected["main"])

    def prepare(self) -> None:
        (self.work / "prepared").mkdir(mode=0o700)
        self.database.mkdir(mode=0o700)
        backup = self.work / "prepared/backup"
        backup.mkdir(mode=0o700)
        self.run("initialize", ["init", "--name", "WritfoldFixture", "--email", "fixture@example.invalid"])
        self.run("local-author-name", ["config", "--local", "--add", "user.name", "WritfoldFixture"])
        self.run("local-author-email", ["config", "--local", "--add", "user.email", "fixture@example.invalid"])
        branch = self.query("initial-branch", "SELECT ACTIVE_BRANCH() AS branch_name;")
        require(len(branch) == 1 and set(branch[0]) == {"branch_name"}
                and isinstance(branch[0]["branch_name"], str), "Invalid initial native branch")
        if branch[0]["branch_name"] != "main":
            self.run("name-main", ["branch", "-m", "main"])
        self.mutate("create-exact-schema", SCHEMA)
        self.replace("seed-rows", self.expected["seed"])
        self.assert_rows("seed-fidelity", self.expected["seed"])
        commits = {"seed": self.commit("seed")}
        self.run("retain-seed", ["branch", "retained_seed"])
        self.run("checkout-side", ["checkout", "-b", "side_history"])
        self.replace("side-rows", self.expected["side"])
        self.assert_rows("side-fidelity", self.expected["side"])
        commits["side"] = self.commit("side")
        self.run("checkout-main", ["checkout", "main"])
        pending = [dict(row, revision=1) if row["id"] == "a" else dict(row) for row in self.expected["main"]]
        require(sum(row["id"] == "a" and row["revision"] == 2 for row in self.expected["main"]) == 1,
                "Conditional-row fixture requires one revision-two lowercase a")
        self.replace("pending-main-rows", pending)
        self.assert_rows("pre-rollback", pending)
        self.mutate("transaction-rollback", "START TRANSACTION; UPDATE exact_records SET revision=99 "
                    "WHERE id='a' AND revision=1; ROLLBACK;")
        self.assert_rows("after-rollback", pending)
        self.passed("transaction-rollback")
        self.mutate("stale-conditional-row", "UPDATE exact_records SET revision=99 WHERE id='a' AND revision=0;")
        self.assert_rows("after-stale-row", pending)
        self.mutate("matching-conditional-row", "UPDATE exact_records SET revision=2 WHERE id='a' AND revision=1;")
        self.assert_rows("after-matching-row", self.expected["main"])
        self.passed("conditional-row-controls")
        self.passed("exact-row-fidelity")
        commits["main"] = self.commit("main")
        branches = {branch: commits[state] for branch, state in BRANCH_STATES.items()}
        self.assert_history(commits, branches, "prepared-history")
        self.passed("pinned-history-and-branches")
        missing = native_hash("0" * 32)
        require(missing not in commits.values(), "Missing-history control accidentally exists")
        self.run("missing-history", ["sql", "--result-format", "json", "--query",
                                    SELECT_ROWS + f" AS OF '{missing}' ORDER BY BINARY id;"],
                 missing_history=True)
        self.assert_history(commits, branches, "after-missing-history")
        self.passed("missing-history-refusal")
        self.run("native-backup", ["backup", "sync-url", backup.as_uri()])
        backup_inventory = inventory(backup)
        pins = {"schema_version": 1, "profile": self.profile["profile"],
                "profile_sha256": sha(PROFILE), "fixture_sha256": sha(source_path(self.profile["fixture"])),
                "original_database": str(self.database), "commits": commits, "branches": branches,
                "backup_inventory": backup_inventory, "expected": self.expected}
        publish(self.work / "pins.json", pins)
        self.report["pins"] = file_record(self.work / "pins.json")
        self.report["artifacts"]["backup_inventory"] = backup_inventory
        self.passed("native-backup-inventory")

    def restore(self) -> None:
        incoming = self.work / "incoming"
        require(stat.S_ISDIR(incoming.lstat().st_mode) and not incoming.is_symlink()
                and {path.name for path in incoming.iterdir()} == {"backup", "pins.json"},
                "Restore accepts only incoming backup and pin manifest")
        pin_path = incoming / "pins.json"
        require(stat.S_ISREG(pin_path.lstat().st_mode) and pin_path.stat().st_size <= 1024 * 1024,
                "Invalid incoming pin manifest")
        pins = document(pin_path)
        require(pins["schema_version"] == 1 and pins["profile"] == self.profile["profile"]
                and pins["profile_sha256"] == sha(PROFILE)
                and pins["fixture_sha256"] == sha(source_path(self.profile["fixture"]))
                and pins["expected"] == self.expected, "Pin profile/fixture identity mismatch")
        require(pins["original_database"] == "/evidence/prepared/database", "Unexpected original database path")
        require(not os.path.lexists(pins["original_database"])
                and not os.path.lexists(self.work / "prepared")
                and not os.path.lexists(self.database), "Original roots present or restore destination reused")
        self.passed("original-roots-unavailable")
        verify_backup(incoming / "backup", pins["backup_inventory"])
        self.passed("backup-input-integrity")
        self.report["pins"] = file_record(pin_path)
        self.report["artifacts"]["backup_inventory"] = pins["backup_inventory"]
        # Restore constructs native metadata and requires an author. Configure
        # only this disposable DOLT_ROOT_PATH, never the operator's global state.
        self.run("restore-author-name", ["config", "--global", "--add", "user.name", "WritfoldFixture"],
                 cwd=self.work)
        self.run("restore-author-email", ["config", "--global", "--add", "user.email", "fixture@example.invalid"],
                 cwd=self.work)
        self.run("native-restore", ["backup", "restore", (incoming / "backup").as_uri(), "restored"], cwd=self.work)
        self.assert_history(pins["commits"], pins["branches"], "restored-history")
        require(not os.path.lexists(pins["original_database"]), "Original database appeared during restore")
        self.passed("restored-pins-and-branches")

    def execute(self) -> int:
        try:
            self.admit()
            if self.mode == "prepare":
                self.prepare()
            else:
                self.restore()
            require(all(check["status"] == "pass" for check in self.report["checks"]),
                    "Required native checks remain unexercised")
            self.report["status"] = "pass"
        except BaseException as error:
            self.report["errors"].append(f"{type(error).__name__}: {error}")
        finally:
            self.report["ended_ns"] = time.time_ns()
            self.report["artifacts"]["command_metadata"] = [
                file_record(path) for path in sorted(self.pipes.directory.glob("*.command.json"))]
            publish(self.work / "report.json", self.report)
        print(json.dumps({"mode": self.mode, "status": self.report["status"],
                          "report": str(self.work / "report.json")}, sort_keys=True))
        return 0 if self.report["status"] == "pass" else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prepare", "restore"))
    parser.add_argument("--work", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        work = arguments.work
        require(work == Path("/evidence") and work.resolve() == work, "Work must be the private /evidence mount")
        metadata = work.lstat()
        require(stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.getuid()
                and stat.S_IMODE(metadata.st_mode) & 0o077 == 0, "Work must be an owned private directory")
        allowed = set() if arguments.mode == "prepare" else {"incoming"}
        require({path.name for path in work.iterdir()} == allowed, "Work is not fresh; existing evidence is never reused")
        os.umask(0o077)
        return Campaign(work, arguments.mode, document(PROFILE)).execute()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"DINT00 fixture refused: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
