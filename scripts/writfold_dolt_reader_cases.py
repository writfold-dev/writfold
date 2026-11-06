#!/usr/bin/env python3
"""Supplemental DINT01 native fixture owner; all product reads use the Rust ELF."""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import sys
import time

sys.dont_write_bytecode = True
from writfold_dolt_fixture import admit_binary, native_hash, require, text_literal
from writfold_evidence import document, file_record, publish, sha
from writfold_stdio_process import Pipes

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "qualification/dolt-integration-profile.json"
READ_PROFILE = ROOT / "qualification/dolt-reader-profile.json"
FIXTURE = ROOT / "fixtures/qualification/dolt-reader.json"
IDENTITY_CASES = {
    "realm": ("other-realm", "pin.realm"),
    "kb": ("other knowledge base", "pin.kb"),
    "database": ("other-database", "pin.database"),
    "epoch": ("epoch-2", "pin.epoch"),
    "schema_digest": ("0" * 64, "schema_mismatch"),
    "query_digest": ("0" * 64, "query_mismatch"),
    "producer": ("0" * 64, "executable_mismatch"),
    "storage_profile": ("unsupported-profile", "pin.profile"),
}
CHECKS = (
    "candidate-identity", "compiled-contracts", "seed-after-head", "head", "repeat-seed", "empty",
    *("wrong-" + key for key in IDENTITY_CASES),
    "missing-commit", "injection-commit", "stdout-limit", "row-limit", "zero-timeout", "cancelled",
    "moved-root", "moved-root-restored", "missing-root", "stderr-limit", "missing-root-restored",
    "dangling-edge", "missing-metadata", "busy-lock", "released-lock", "missing-lock",
    "metadata-producer", "metadata-query",
    "missing-lock-restored", "symlink-database", "symlink-metadata", "symlink-metadata-restored",
    "native-server-marker", "native-server-marker-restored", "home-config", "home-config-restored",
    "native-state-unchanged",
)


class Campaign:
    def __init__(self, work: Path):
        self.work = work
        self.database = work / "database"
        self.home = work / "reader-home"
        self.binary = work / "dolt_read"
        self.profile = document(PROFILE)
        self.fixture = document(FIXTURE)
        self.dolt = Path(self.profile["candidate"]["executable"])
        self.pipes = Pipes(work / "commands")
        self.env = dict(PATH="/usr/local/bin:/usr/bin:/bin", HOME=str(work / "writer-home"),
                        DOLT_ROOT_PATH=str(work / "writer-root"), DOLT_DISABLE_EVENT_FLUSH="1",
                        DOLT_DISABLE_VERSION_CHECK="1", LANG="C.UTF-8", LC_ALL="C.UTF-8",
                        TZ="UTC", GOMAXPROCS="2")
        self.report = dict(schema_version=1, scope="DINT01 actual typed Rust exact reader",
                           status="fail", checks=[dict(name=n, required=True, status="fail") for n in CHECKS],
                           commands=self.pipes.records, errors=[], expected_refusals=[], pins={},
                           release_gates_passed=[], agent_e2e="not_run", writer="not_run",
                           omnigraph_wf24="blocked_unchanged", started_ns=time.time_ns(),
                           limits=["Synthetic projection, not knowledge authority or citation lineage",
                                   "Sequential stopped native database; no concurrent-writer or hostile same-user guarantee",
                                   "No independent Rust backup/restore, power-loss, SQL-server or agent E2E claim",
                                   "No performance or token-benefit claim; native internal caches may change"])

    def passed(self, name):
        matches = [c for c in self.report["checks"] if c["name"] == name]
        require(len(matches) == 1 and matches[0]["status"] == "fail", "Undeclared/repeated check: " + name)
        matches[0]["status"] = "pass"

    def command(self, name, argv, *, cwd=None, reader=False):
        env = dict(self.env)
        if reader:
            env.update(HOME=str(self.home), DOLT_ROOT_PATH=str(self.home))
        return self.pipes.run(name, list(map(str, argv)), [("eof", None)],
                              cwd=cwd or self.database, env=env, timeout=120, limit=16 * 1024 * 1024)

    def native(self, name, args):
        out, _, code = self.command(name, [self.dolt, *args])
        require(code == 0, "Unexpected native failure: " + name)
        return out

    def sql(self, name, sql):
        return self.native(name, ["sql", "--disable-auto-gc", "--result-format", "json", "--query", sql])

    def rows(self, name, sql):
        self.sql(name, sql)
        value = document(Path(self.pipes.records[-1]["artifacts"]["stdout"]["path"]))
        require(value == {} or (isinstance(value, dict) and set(value) == {"rows"}
                and isinstance(value["rows"], list)), "Unexpected native rows")
        return value.get("rows", [])

    def head(self, name):
        rows = self.rows(name, "SELECT HASHOF('HEAD') AS native_commit")
        require(len(rows) == 1 and set(rows[0]) == {"native_commit"}, "Invalid HEAD scalar")
        return native_hash(rows[0]["native_commit"])

    def state(self, name):
        head = self.head(name + "-head")
        require(self.rows(name + "-status", "SELECT * FROM dolt_status") == [], "Native working set changed")
        return head

    def replace(self, name, graph):
        statements = ["DELETE FROM wf_edges", "DELETE FROM wf_nodes"]
        for table, columns in (("nodes", ("id", "kind", "label", "home_path")),
                               ("edges", ("id", "source", "target", "kind"))):
            for row in graph[table]:
                statements.append("INSERT INTO wf_" + table + " (" + ",".join(columns) + ") VALUES ("
                                  + ",".join(text_literal(row[c]) for c in columns) + ")")
        self.sql(name, ";".join(statements))

    def commit(self, name):
        self.native(name + "-add", ["add", "-A"])
        self.native(name + "-commit", ["commit", "--author", "WritfoldFixture <fixture@example.invalid>",
                                     "--date", "2026-11-05T00:00:00Z", "-m", "DINT01 " + name])
        commit = self.head(name + "-hash")
        self.native(name + "-retain", ["branch", "wf-retained/" + commit, commit])
        pin = dict(self.base, commit=commit)
        self.report["pins"][name] = pin
        return pin

    def read(self, name, pin, graph=None, refusal=None, options=(), db=None, state_check=True):
        before = self.state(name + "-before") if state_check else None
        path = self.work / (name + ".pin.json")
        publish(path, pin)
        out, err, code = self.command(name, [self.binary, self.dolt, db or self.database,
                                             self.home, path, *options], reader=True)
        if refusal is None:
            require(code == 0 and not err, "Rust read failed: " + name)
            actual = document(Path(self.pipes.records[-1]["artifacts"]["stdout"]["path"]))
            expected = dict(pin=pin, nodes=graph["nodes"], edges=graph["edges"],
                            node_count=len(graph["nodes"]), edge_count=len(graph["edges"]))
            require(actual == expected, "Full typed projection/pin mismatch: " + name)
        else:
            self.check_refusal(name, out, code, refusal)
        if state_check:
            require(self.state(name + "-after") == before, "HEAD changed across read: " + name)
        self.passed(name)

    def check_refusal(self, name, out, code, expected):
        require(code == 1 and not out, "Rust refusal emitted success data or had an unexpected exit: " + name)
        diagnostic = document(Path(self.pipes.records[-1]["artifacts"]["stderr"]["path"]))
        require(isinstance(diagnostic, dict) and set(diagnostic) == {"kind", "message"}
                and diagnostic["kind"] in expected and isinstance(diagnostic["message"], str),
                "Wrong typed Rust refusal: " + name)
        self.pipes.records[-1].update(expected_refusal=True, refusal_kind=name,
                                     diagnostic_kind=diagnostic["kind"], expected_kinds=list(expected))
        self.report["expected_refusals"].append(dict(name=name, **diagnostic, exit_code=code))

    def execute(self):
        require(os.getuid() != 0, "Native reader campaign must be unprivileged")
        for name in ("commands", "database", "reader-home", "writer-home", "writer-root"):
            (self.work / name).mkdir(mode=0o700)
        candidate = self.profile["candidate"]
        lock = document(ROOT / candidate["tool_lock"])
        observation_path = ROOT / candidate["observation"]
        observation = document(observation_path)
        locked = [x for x in lock["artifacts"] if x["name"] == "dolt"]
        observed = [x for x in observation["tools"] if x["name"] == "dolt"]
        require(lock["admitted"] is True and len(locked) == len(observed) == 1
                and sha(observation_path) == lock["observation"]["sha256"]
                and candidate["sha256"] == locked[0]["executable_sha256"] == observed[0]["sha256"]
                and candidate["archive_sha256"] == locked[0]["archive_sha256"]
                and candidate["version"] == locked[0]["version"] == "2.6.1"
                and str(self.dolt) == observed[0]["executable"] == "/usr/local/bin/dolt",
                "Native profile/tool admission mismatch")
        reader_profile = document(READ_PROFILE)
        require(reader_profile["milestone"] == "DINT01"
                and reader_profile["profile"] == self.fixture["identity"]["storage_profile"] == "dolt-corpus-read-v1"
                and reader_profile["native_candidate"]["version"] == candidate["version"]
                and reader_profile["native_candidate"]["executable_sha256"] == candidate["sha256"],
                "Typed reader profile differs from native admission or fixture")
        self.report["candidate"] = admit_binary(self.dolt, candidate["sha256"])
        self.report["consumer"] = file_record(self.binary)
        self.report["inputs"] = [file_record(p) for p in (PROFILE, READ_PROFILE, FIXTURE, ROOT / "toolchains.lock.json", observation_path)]
        require(self.native("version", ["version"]).decode().splitlines()[0] == "dolt version 2.6.1", "Wrong Dolt version")
        self.passed("candidate-identity")
        contracts = {}
        for name in ("schema", "query-contract"):
            out, err, code = self.command(name, [self.binary, "--" + name], reader=True)
            require(code == 0 and not err and out, "Compiled contract unavailable")
            path = self.work / (name + ".txt")
            path.write_bytes(out)
            contracts[name] = hashlib.sha256(out).hexdigest()
        self.report["contracts"] = contracts
        self.passed("compiled-contracts")
        self.base = dict(namespace="writfold-dolt-read", schema=1, kind="pin", **self.fixture["identity"],
                         schema_digest=contracts["schema"], query_digest=contracts["query-contract"],
                         producer=candidate["sha256"])
        self.native("init", ["init", "--name", "WritfoldFixture", "--email", "fixture@example.invalid"])
        # Fixture author settings are local to the owned DB, never reader HOME.
        for key, value in (("user.name", "WritfoldFixture"), ("user.email", "fixture@example.invalid")):
            self.native("local-" + key, ["config", "--local", "--add", key, value])
        active = self.rows("active-branch", "SELECT ACTIVE_BRANCH() AS name")
        require(len(active) == 1, "Missing active branch")
        if active[0]["name"] != "main":
            self.native("rename-main", ["branch", "-m", "main"])
        self.sql("schema-create", (self.work / "schema.txt").read_text())
        cols = ("realm", "kb", "database_id", "epoch", "schema_digest", "query_digest", "storage_profile", "producer")
        values = [self.base["database" if c == "database_id" else c] for c in cols]
        self.sql("metadata", "INSERT INTO wf_reader_meta (id," + ",".join(cols) + ") VALUES (1,"
                 + ",".join(map(text_literal, values)) + ")")
        self.replace("seed-data", self.fixture["seed"])
        seed = self.commit("seed")
        self.replace("head-data", self.fixture["head"])
        head = self.commit("head")
        require(seed["commit"] != head["commit"], "Head did not advance")
        coordination = self.database / "coordination"
        coordination.mkdir(mode=0o700)
        lockfile = coordination / "database.lock"
        lockfile.touch(mode=0o600, exist_ok=False)
        initial_head = self.state("initial")
        for name, pin, graph in (("seed-after-head", seed, self.fixture["seed"]),
                                 ("head", head, self.fixture["head"]), ("repeat-seed", seed, self.fixture["seed"])):
            self.read(name, pin, graph)
        for key, (value, diagnostic) in IDENTITY_CASES.items():
            self.read("wrong-" + key, dict(seed, **{key: value}), refusal=(diagnostic,))
        self.read("missing-commit", dict(seed, commit="0" * 32), refusal=("native_command",))
        self.read("injection-commit", dict(seed, commit="'; DROP TABLE wf_nodes; --"),
                  refusal=("wire.identity",))
        for name, options, diagnostic in (
            ("stdout-limit", ("--stdout-bytes", "32"), "stdout_limit"),
            ("row-limit", ("--rows", "1"), "row_limit"),
            ("zero-timeout", ("--timeout-ms", "0"), "deadline"),
            ("cancelled", ("--cancelled",), "cancelled")):
            self.read(name, seed, refusal=(diagnostic,), options=options)
        root = "wf-retained/" + seed["commit"]
        self.native("move-root", ["branch", "-f", root, head["commit"]])
        try:
            self.read("moved-root", seed, refusal=("retention_root",))
        finally:
            self.native("restore-moved-root", ["branch", "-f", root, seed["commit"]])
        self.read("moved-root-restored", seed, self.fixture["seed"])
        self.native("remove-root", ["branch", "-D", root])
        try:
            self.read("missing-root", seed, refusal=("native_command",))
            self.read("stderr-limit", seed, refusal=("stderr_limit",), options=("--stderr-bytes", "0"))
        finally:
            self.native("restore-missing-root", ["branch", root, seed["commit"]])
        self.read("missing-root-restored", seed, self.fixture["seed"])
        for name in ("empty", "dangling-edge", "missing-metadata", "metadata-producer", "metadata-query"):
            self.native(name + "-branch", ["checkout", "-b", "fixture-" + name, head["commit"]])
            if name == "empty":
                graph = dict(nodes=[], edges=[])
                self.replace("empty-data", graph)
            elif name == "dangling-edge":
                self.sql("dangling-data", "UPDATE wf_edges SET target='missing:node' WHERE id='edge:a'")
            elif name in ("metadata-producer", "metadata-query"):
                column = "producer" if name == "metadata-producer" else "query_digest"
                self.sql(name + "-data", "UPDATE wf_reader_meta SET " + column + "='" + "0" * 64 + "'")
            else:
                self.sql("remove-metadata", "DELETE FROM wf_reader_meta")
            pin = self.commit(name)
            self.native(name + "-return-main", ["checkout", "main"])
            if name == "empty":
                self.read(name, pin, graph)
            else:
                diagnostics = {
                    "dangling-edge": ("dangling_edge",),
                    "missing-metadata": ("scalar_rows", "wire.shape"),
                    "metadata-producer": ("pin.producer",),
                    "metadata-query": ("pin.epoch",),
                }[name]
                self.read(name, pin, refusal=diagnostics)
        with lockfile.open("rb") as held:
            fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.read("busy-lock", seed, refusal=("fs.lock_busy",))
        self.read("released-lock", seed, self.fixture["seed"])
        aside = coordination / "database.lock.saved"
        lockfile.rename(aside)
        try:
            self.read("missing-lock", seed, refusal=("fs.missing",))
            require(not lockfile.exists(), "Reader recreated absent lock")
        finally:
            aside.rename(lockfile)
        self.read("missing-lock-restored", seed, self.fixture["seed"])
        link = self.work / "database-link"
        link.symlink_to(self.database, target_is_directory=True)
        self.read("symlink-database", seed, refusal=("fs.symlink", "fs.not_directory"), db=link)
        metadata = self.database / ".dolt"
        saved = self.database / ".dolt-saved"
        metadata.rename(saved)
        metadata.symlink_to(saved, target_is_directory=True)
        try:
            self.read("symlink-metadata", seed, refusal=("fs.symlink", "fs.not_directory"),
                      state_check=False)
        finally:
            metadata.unlink()
            saved.rename(metadata)
        self.read("symlink-metadata-restored", seed, self.fixture["seed"])
        for name, path in (("native-server-marker", metadata / "sql-server.info"),
                           ("home-config", self.home / ".dolt/config_global.json")):
            prior = path.read_bytes() if path.exists() else None
            contents = b"{}" if name == "native-server-marker" else b'{"user.name":"forbidden reader setting"}'
            path.write_bytes(contents)
            try:
                # Native state queries must not auto-connect to this synthetic marker.
                if name == "native-server-marker":
                    self.marker_read(name, seed)
                else:
                    self.read(name, seed, refusal=("unsupported_profile",))
                require(path.read_bytes() == contents, "Reader mutated refusal marker")
            finally:
                if prior is None:
                    path.unlink()
                else:
                    path.write_bytes(prior)
            self.read(name + "-restored", seed, self.fixture["seed"])
        require(self.state("final") == initial_head, "Native HEAD changed")
        self.passed("native-state-unchanged")

    def marker_read(self, name, pin):
        path = self.work / (name + ".pin.json")
        publish(path, pin)
        out, err, code = self.command(name, [self.binary, self.dolt, self.database, self.home, path], reader=True)
        self.check_refusal(name, out, code, ("unsupported_profile",))
        self.passed(name)

    def run(self):
        try:
            self.execute()
            require(all(c["status"] == "pass" for c in self.report["checks"]), "Unexercised checks")
            self.report["status"] = "pass"
        except (Exception, KeyboardInterrupt) as error:
            self.report["errors"].append(f"{type(error).__name__}: {error}")
        finally:
            self.report["ended_ns"] = time.time_ns()
            publish(self.work / "report.json", self.report)
        return 0 if self.report["status"] == "pass" else 1


def interrupted(signum, frame):
    raise KeyboardInterrupt(f"signal {signum}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupted)
    raise SystemExit(Campaign(args.work).run())
