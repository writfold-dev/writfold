#!/usr/bin/env python3
"""Bounded synthetic authenticated lifecycle smoke, never a full WF24 receipt."""
from __future__ import annotations

import argparse
import errno
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import secrets
import signal
import socket
import subprocess
import threading
import time

from writfold_omnigraph_probe import validate_profile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/qualification/omnigraph-companion"
LIMIT = 1024 * 1024
GRACE = 5


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


class Process:
    """Own one process group, drain both pipes even after retained logs fill."""

    def __init__(self, work, name, argv, env, tokens, records):
        self.tokens = [token.encode() for token in tokens]
        self.record = {"name": name, "argv": argv, "logs": {}, "reaped": False,
                       "forced_kill": False, "start_ns": time.time_ns()}
        records.append(self.record)
        self.process = subprocess.Popen(argv, cwd=work, env=env, stdin=subprocess.DEVNULL,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                        start_new_session=True)
        self.record["pid"] = self.process.pid
        self.threads = []
        for label, stream in (("stdout", self.process.stdout), ("stderr", self.process.stderr)):
            thread = threading.Thread(target=self.drain, args=(work / f"{name}.{label}", stream),
                                      daemon=True)
            thread.start()
            self.threads.append(thread)

    def drain(self, path, stream):
        stats = {"bytes_seen": 0, "bytes_retained": 0, "truncated": False}
        self.record["logs"][path.name] = stats
        pending = b""
        try:
            with path.open("xb") as output:
                while True:
                    chunk = stream.read1(8192)
                    stats["bytes_seen"] += len(chunk)
                    pending += chunk
                    for token in self.tokens:
                        pending = pending.replace(token, b"[REDACTED]")
                    # Keep a full token length across arbitrary pipe chunk boundaries.
                    data, pending = (pending[:-64], pending[-64:]) if chunk else (pending, b"")
                    retained = data[:max(0, LIMIT - stats["bytes_retained"])]
                    output.write(retained)
                    stats["bytes_retained"] += len(retained)
                    stats["truncated"] |= len(retained) != len(data)
                    if not chunk:
                        break
        except BaseException as error:
            stats["error"] = type(error).__name__
        finally:
            stream.close()

    def kill(self):
        try:
            os.killpg(self.process.pid, signal.SIGKILL)
            self.record["forced_kill"] = True
        except ProcessLookupError:
            pass

    def finish(self, timeout, terminate=False):
        if terminate and self.process.poll() is None:
            try:
                os.killpg(self.process.pid, signal.SIGTERM)
                self.record["sigterm_ns"] = time.time_ns()
            except ProcessLookupError:
                pass
        try:
            self.record["exit"] = self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.record["timeout_unknown_outcome"] = True
            self.kill()
            self.record["exit"] = self.process.wait(timeout=5)
        except BaseException as error:
            self.record["interrupted_unknown_outcome"] = type(error).__name__
            self.kill()
            self.record["exit"] = self.process.wait(timeout=5)
        self.record["reaped"] = self.process.poll() is not None
        for thread in self.threads:
            thread.join(timeout=2)
        if any(thread.is_alive() for thread in self.threads):
            self.kill()
            for thread in self.threads:
                thread.join(timeout=2)
        self.record["logs_drained"] = all(not thread.is_alive() for thread in self.threads)
        try:
            os.killpg(self.process.pid, 0)
            self.record["group_absent"] = False
        except ProcessLookupError:
            self.record["group_absent"] = True
        self.record["end_ns"] = time.time_ns()
        return self.record


class Probe:
    def __init__(self, work, profile):
        self.work = work
        self.profile_path = profile
        self.profile = json.loads(profile.read_text())
        upstream = f"https://github.com/ModernRelay/omnigraph/blob/{self.profile['source_commit']}/"
        self.server = None
        self.port = None
        self.tokens = {actor: secrets.token_hex(32) for actor in ("act-reader", "act-writer", "wrong")}
        self.expected = json.loads((FIXTURE / "expected.json").read_text())
        self.receipt = {"scope": "P01O supplemental authenticated loopback companion smoke",
                        "status": "fail", "wf24": "not_run", "commands": [], "requests": [],
                        "checks": [], "ownership": "fresh exclusive stopped CLI bootstrap",
                        "bounds": {"log_bytes_per_stream": LIMIT, "response_bytes": LIMIT,
                                   "readiness_seconds": 45, "request_seconds": 15,
                                   "cli_seconds": 120, "shutdown_grace_seconds": GRACE},
                        "sources": [upstream + path for path in (
                            "docs/user/clusters/index.md", "docs/user/clusters/config.md",
                            "docs/user/operations/server.md", "docs/user/operations/policy.md",
                            "docs/user/cli/targets.md", "openapi.json",
                            "crates/omnigraph-cli/src/commands/cluster/mod.rs",
                            "crates/omnigraph-server/tests/integration/stored_queries.rs")]}
        home = work / "home"
        home.mkdir(mode=0o700)
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(home), "TMPDIR": str(work),
                    "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
                    "RUST_BACKTRACE": "0", "RUST_LOG": "info", "RAYON_NUM_THREADS": "2",
                    "TOKIO_WORKER_THREADS": "2", "OMP_NUM_THREADS": "2", "OPENBLAS_NUM_THREADS": "2"}

    def check(self, name, condition):
        self.receipt["checks"].append({"name": name, "pass": bool(condition)})
        require(condition, name)

    def cli(self, name, args):
        require(self.server is None, "Direct CLI forbidden after server ownership starts")
        process = Process(self.work, name, ["/native/omnigraph", *args], self.env, [],
                          self.receipt["commands"])
        result = process.finish(120)
        require(result["exit"] == 0 and not result["forced_kill"] and result["group_absent"]
                and result["logs_drained"] and all(not log.get("error") and not log["truncated"]
                                                  for log in result["logs"].values()),
                f"{name} failed, lost logs or timed out; retained original attempt, no retry")
        return process

    def bootstrap(self):
        profile = validate_profile(self.profile_path, Path("/profile/source-build.json"))
        identities = {name: sha(Path("/native") / name) for name in ("omnigraph", "omnigraph-server")}
        self.check("native executable identities", all(profile["members"][k] == v for k, v in identities.items()))
        self.receipt["executables"] = identities
        self.receipt["source_commit"] = profile["source_commit"]
        self.receipt["profile_sha256"] = sha(self.profile_path)
        self.cli("version", ["version", "--json"])
        self.check("Exact CLI version", json.loads((self.work / "version.stdout").read_text())["version"] == profile["version"])
        bundle = self.work / "bundle"
        bundle.mkdir()
        original = ROOT / profile["fixture_root"]
        for name in ("schema.pg", "queries.gq", "seed.jsonl"):
            require(sha(original / name) == profile["fixture_sha256"][name], "Immutable fixture mismatch")
            (bundle / name).write_bytes((original / name).read_bytes())
        for name in ("cluster.yaml", "graph.policy.yaml", "rename.gq", "expected.json"):
            (bundle / name).write_bytes((FIXTURE / name).read_bytes())
        self.receipt["fixtures"] = {path.name: sha(path) for path in sorted(bundle.iterdir())}
        self.native_root = self.work / "cluster"
        self.native_root.mkdir()
        target = self.native_root.as_uri()
        self.cli("validate", ["cluster", "validate", "--config", str(bundle)])
        self.cli("apply", ["--target", target, "cluster", "apply", "--config", str(bundle), "--json"])
        applied = json.loads((self.work / "apply.stdout").read_text())
        require(applied["status"] == "complete" and applied["result"]["converged"] is True,
                "Fresh bootstrap did not complete; ownership remains uncertain")
        diagnostic = (self.work / "apply.stderr").read_text()
        roots = re.findall(r"^Cluster-root: (\S+)$", diagnostic, re.MULTILINE)
        locks = re.findall(r"^Admission lock: (\S+); retain until prior work is quiescent, "
                           r"then exact-ID force-unlock$", diagnostic, re.MULTILINE)
        require(len(locks) == 1 and roots == [target], "Expected exact pinned CLI root/lock diagnostics")
        lock, actual_root = locks[0], roots[0]
        self.receipt["handoff"] = {"lock_id": lock, "canonical_root": actual_root,
                                   "prior_owner_reaped": True, "accepted_cli_io_settled": True}
        self.cli("force-unlock", ["--target", target, "cluster", "force-unlock", lock, "--json"])
        self.check("exact stopped-owner handoff", json.loads((self.work / "force-unlock.stdout").read_text())
                   == {"unlocked": True, "lock_id": lock})
        token_file = self.work / "tokens.json"
        with open(token_file, "x", opener=lambda p, flags: os.open(p, flags, 0o600)) as stream:
            json.dump({actor: token for actor, token in self.tokens.items() if actor != "wrong"}, stream)
        self.receipt["tokens"] = {actor: hashlib.sha256(token.encode()).hexdigest()
                                  for actor, token in self.tokens.items()}
        self.env["OMNIGRAPH_SERVER_BEARER_TOKENS_FILE"] = str(token_file)
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            self.port = reservation.getsockname()[1]
        argv = ["/native/omnigraph-server", "--cluster", target, "--bind", f"127.0.0.1:{self.port}",
                "--require-all-graphs", "--shutdown-grace-seconds", str(GRACE)]
        self.server = Process(self.work, "server", argv, self.env, self.tokens.values(), self.receipt["commands"])
        self.receipt["ownership"] = "server-only; no further direct graph or cluster CLI access"

    def request(self, name, route, body, actor="act-reader", contract="0.14", seconds=15):
        record = {"name": name, "route": route, "body": body, "actor": actor, "contract": contract,
                  "start_ns": time.time_ns(), "outcome": "unknown"}
        self.receipt["requests"].append(record)
        headers = {"Content-Type": "application/json", "Omnigraph-Http-Api": contract}
        if actor is not None:
            headers["Authorization"] = "Bearer " + self.tokens[actor]
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=seconds)
        signal.setitimer(signal.ITIMER_REAL, seconds)
        try:
            connection.request("POST", "/graphs/companion/" + route, json.dumps(body), headers)
            response = connection.getresponse()
            record["status"] = response.status
            record["response_contract"] = response.getheader("Omnigraph-Http-Api")
            record["challenge"] = response.getheader("WWW-Authenticate")
            require(record["response_contract"] == "0.14", "Response contract mismatch; effects unknown")
            raw = response.read(LIMIT + 1)
            require(len(raw) <= LIMIT, "Response exceeded bound; effects unknown")
            for token in self.tokens.values():
                raw = raw.replace(token.encode(), b"[REDACTED]")
            (self.work / f"http-{len(self.receipt['requests']):03d}.json").write_bytes(raw)
            result = json.loads(raw)
            record["response"] = result
            record["outcome"] = "observed"
            return response.status, result
        except BaseException as error:
            record["failure_class"] = type(error).__name__
            raise
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            connection.close()
            record["end_ns"] = time.time_ns()

    def ready(self):
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            require(self.server.process.poll() is None, "Server exited before authenticated readiness")
            try:
                status, body = self.request("readiness", "queries/all_nodes", {"params": {}, "expect_mutation": False},
                                            seconds=min(3, deadline - time.monotonic()))
                self.check("authenticated named-query readiness", status == 200 and body["rows"] == []
                           and body["row_count"] == 0 and body["query_name"] == "all_nodes")
                return
            except (ConnectionRefusedError, ConnectionResetError, TimeoutError):
                time.sleep(0.1)  # Read-only readiness only; writes never retry.
        raise TimeoutError("Authenticated query readiness deadline expired")

    def pinned(self, label, name, pin, expected):
        params = {"key": "a"} if name == "two" else {}
        status, result = self.request(label, "queries/" + name,
                                      {"params": params, "snapshot": pin, "expect_mutation": False})
        self.check(label, status == 200 and result["rows"] == expected and result["row_count"] == len(expected)
                   and result["query_name"] == name and result["graph_commit_id"] == pin
                   and result["target"].get("snapshot") == pin)

    def exercise(self):
        self.ready()
        for actor, label in ((None, "no-token refusal"), ("wrong", "wrong-token refusal")):
            status, result = self.request(label, "queries/all_nodes", {"params": {}}, actor=actor)
            challenge = self.receipt["requests"][-1]["challenge"] or ""
            self.check(label, status == 401 and result["code"] == "unauthorized" and "Bearer" in challenge
                       and (actor is None or 'error="invalid_token"' in challenge))
        status, result = self.request("unsupported contract", "queries/all_nodes", {}, contract="99.0")
        self.check("unsupported contract refusal", status == 400 and result["code"] == "api_contract_mismatch")
        status, seed = self.request("seed once", "load", {"data": (self.work / "bundle/seed.jsonl").read_text(),
                                                        "mode": "overwrite"}, actor="act-writer")
        self.check("server seed cardinality", status == 200 and seed["total_entities"] == self.expected["seed_entities"])
        old = seed["commit"]["graph_commit_id"]
        require(isinstance(old, str) and bool(old), "Missing exact seed pin")
        self.receipt["old_pin"] = old
        self.pinned("old exact rows before mutation", "all_nodes", old, self.expected["all_nodes"])
        self.pinned("old exact traversal before mutation", "two", old, self.expected["two"])
        request = {"params": self.expected["rename_params"], "branch": "main", "expect_mutation": True}
        status, result = self.request("reader mutation refusal", "queries/rename", request)
        self.check("read-only credential cannot mutate", status == 403 and result["code"] == "forbidden")
        status, unchanged = self.request("refusal preserves current head", "queries/all_nodes",
                                         {"params": {}, "branch": "main", "expect_mutation": False})
        self.check("refused mutation preserves current pin and complete state",
                   status == 200 and unchanged["graph_commit_id"] == old
                   and unchanged["target"].get("branch") == "main"
                   and unchanged["rows"] == self.expected["all_nodes"])
        status, mutation = self.request("rename once", "queries/rename", request, actor="act-writer")
        self.check("named server mutation", status == 200 and mutation["actor_id"] == "act-writer")
        new = mutation["commit"]["graph_commit_id"]
        self.check("distinct new native pin", isinstance(new, str) and bool(new) and new != old)
        self.receipt["new_pin"] = new
        self.pinned("new exact rows", "all_nodes", new, self.expected["renamed"])
        self.pinned("new exact traversal", "two", new, self.expected["two"])
        self.pinned("old exact rows survive", "all_nodes", old, self.expected["all_nodes"])
        self.pinned("old exact traversal survives", "two", old, self.expected["two"])

    def teardown(self):
        if self.server is None:
            self.receipt["shutdown"] = {"server_started": False}
            return
        result = self.server.finish(GRACE + 5, terminate=True)
        self.receipt["shutdown"] = result.copy()
        with socket.socket() as endpoint:
            endpoint.settimeout(1)
            closed = endpoint.connect_ex(("127.0.0.1", self.port)) == errno.ECONNREFUSED
        self.receipt["shutdown"]["endpoint_refused"] = closed
        self.check("server exited and reaped", result["reaped"] and result["exit"] == 0
                   and result["group_absent"] and not result["forced_kill"])
        self.check("bounded logs drained", result["logs_drained"] and all(not log.get("error")
                   and not log["truncated"] for log in result["logs"].values()))
        self.check("loopback endpoint no longer accepts", closed)


def interrupted(signum, frame):
    raise TimeoutError(f"Probe interrupted by signal {signum}; no uncertain write retry")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--profile", type=Path, default=ROOT / "qualification/omnigraph-profile.json")
    args = parser.parse_args()
    os.umask(0o077)
    work = args.work.absolute()
    require(work.parent.resolve().is_relative_to(Path("/evidence")), "Work must be beneath /evidence")
    work.mkdir(mode=0o700, exist_ok=False)
    probe = Probe(work, args.profile)
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM):
        signal.signal(number, interrupted)
    try:
        probe.bootstrap()
        probe.exercise()
        probe.receipt["status"] = "pass"
    except BaseException as error:
        probe.receipt["failure"] = {"class": type(error).__name__, "message": str(error)}
    finally:
        try:
            probe.teardown()
        except BaseException as error:
            probe.receipt["status"] = "fail"
            probe.receipt["teardown_failure"] = {"class": type(error).__name__, "message": str(error)}
        # Defense in depth: exceptions and server-owned headers never publish raw bearers.
        encoded = json.dumps(probe.receipt)
        for token in probe.tokens.values():
            encoded = encoded.replace(token, "[REDACTED]")
        save(work / "receipt.json", json.loads(encoded))
    return 0 if probe.receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
