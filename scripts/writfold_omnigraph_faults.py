#!/usr/bin/env python3
"""Source-bound native failure qualification, not WF24 or owner-service admission.

Run only in the admitted runtime: --work /evidence/<fresh-dir>
--profile /profile/profile.json. Retains failures and all original write intents.
"""
from __future__ import annotations

import argparse
import copy
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
import threading
import time

from writfold_omnigraph_companion import Process, GRACE, LIMIT, require, save, sha
from writfold_omnigraph_probe import validate_profile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/qualification/omnigraph-faults"
CONDITION = "Omnigraph-If-Graph-Commit"
CONDITIONAL = "queries/change/if-graph-commit"
SOURCE_COMMIT = "1a856292c569e214333ab16b1d1af4d0e2de4ac2"


def canonical(value):
    # JSON spelling distinguishes bool/int/float and missing/null fields.
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def same(left, right):
    return canonical(left) == canonical(right)


def free_port():
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        return reservation.getsockname()[1]


def interrupted(signum, frame):
    raise TimeoutError(f"Signal {signum}; original write outcome may be Unknown; never replay")


class Case:
    def __init__(self, work, name, definition):
        self.work, self.name, self.definition = work, name, definition
        work.mkdir(mode=0o700)
        home = work / "home"
        home.mkdir(mode=0o700)
        self.tokens = {actor: secrets.token_hex(32) for actor in ("act-reader", "act-writer")}
        token_file = work / "tokens.json"
        with open(token_file, "x", opener=lambda p, flags: os.open(p, flags, 0o600)) as stream:
            json.dump(self.tokens, stream)
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(home), "TMPDIR": str(work),
                    "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
                    "RUST_BACKTRACE": "0", "RUST_LOG": "info", "RAYON_NUM_THREADS": "2",
                    "TOKIO_WORKER_THREADS": "2", "OMP_NUM_THREADS": "2", "OPENBLAS_NUM_THREADS": "2",
                    "OMNIGRAPH_SERVER_BEARER_TOKENS_FILE": str(token_file)}
        self.receipt = {"name": name, "status": "fail", "checks": [], "commands": [],
                        "requests": [], "ownership": [], "start_ns": time.time_ns(),
                        "tokens_sha256": {k: hashlib.sha256(v.encode()).hexdigest()
                                          for k, v in self.tokens.items()}}
        self.server = None
        self.processes = []
        self.threads = []
        self.sockets = []
        self.connections = []
        self.guard = threading.Lock()
        self.server_generation = 0
        self.root = work / "cluster"
        self.initial = definition["initial_rows"]

    def check(self, name, condition):
        self.receipt["checks"].append({"name": name, "pass": bool(condition)})
        require(condition, name)

    def owned(self, name, argv):
        process = Process(self.work, name, argv, self.env, self.tokens.values(), self.receipt["commands"])
        self.processes.append(process)
        return process

    def settled(self, result):
        return (result.get("reaped") and result.get("group_absent") and result.get("logs_drained")
                and all(not v.get("error") and not v["truncated"] for v in result["logs"].values()))

    def cli(self, name, args):
        require(self.server is None, "No direct CLI while a native server owns this root")
        result = self.owned(name, ["/native/omnigraph", *args]).finish(120)
        self.check(name + ": owned CLI completed", result["exit"] == 0 and self.settled(result)
                   and not result["forced_kill"])
        return json.loads((self.work / (name + ".stdout")).read_text())

    def bootstrap(self):
        bundle = self.work / "bundle"
        bundle.mkdir()
        for path in sorted(FIXTURE.iterdir()):
            (bundle / path.name).write_bytes(path.read_bytes())
        self.root.mkdir()
        self.target = self.root.as_uri()
        self.cli("validate", ["cluster", "validate", "--config", str(bundle), "--json"])
        result = self.cli("apply", ["--target", self.target, "cluster", "apply", "--config", str(bundle), "--json"])
        self.check("fresh bootstrap converged", result["status"] == "complete" and result["result"]["converged"] is True)
        diagnostic = (self.work / "apply.stderr").read_text()
        roots = re.findall(r"^Cluster-root: (\S+)$", diagnostic, re.MULTILINE)
        locks = re.findall(r"^Admission lock: (\S+); retain until prior work is quiescent, "
                           r"then exact-ID force-unlock$", diagnostic, re.MULTILINE)
        self.check("bootstrap exact ownership identity", roots == [self.target] and len(locks) == 1)
        self.unlock("bootstrap-unlock", locks[0], "completed/reaped/drained exclusive local CLI")
        self.start_server()
        empty_pin = self.ready([])
        self.receipt["empty_pin"] = empty_pin
        status, seeded = self.request("seed-once", "load", {"data": (bundle / "seed.jsonl").read_text(),
                                                          "mode": "overwrite"}, write=True)
        self.check("seed acknowledged two typed cells", status == 200 and seeded.get("total_entities") == 2)
        self.seed_pin = seeded["commit"]["graph_commit_id"]
        self.state("complete seeded state", self.initial, self.seed_pin)
        self.history("seed complete history")

    def unlock(self, name, lock_id, basis):
        require(isinstance(lock_id, str) and bool(lock_id), "Missing exact observed lock ID")
        self.receipt["ownership"].append({"operation": name, "canonical_root": self.target,
                                          "lock_id": lock_id, "quiescence_basis": basis,
                                          "scope": "private local Linux root; no remote outstanding IO"})
        result = self.cli(name, ["--target", self.target, "cluster", "force-unlock", lock_id, "--json"])
        self.check(name + ": exact lock released", same(result, {"unlocked": True, "lock_id": lock_id}))

    def server_argv(self, port):
        return ["/native/omnigraph-server", "--cluster", self.target, "--bind", f"127.0.0.1:{port}",
                "--require-all-graphs", "--shutdown-grace-seconds", str(GRACE)]

    def start_server(self):
        require(self.server is None, "Cannot replace a live or uncertain owner")
        self.port = free_port()
        self.server_generation += 1
        self.server = self.owned(f"server-{self.server_generation}", self.server_argv(self.port))
        self.receipt["ownership"].append({"operation": "start-server", "pid": self.server.process.pid,
                                          "canonical_root": self.target, "port": self.port})

    def prepare(self, name, route, body, *, write=False, condition=None, method="POST"):
        request_id = secrets.token_hex(16)
        record = {"name": name, "request_id": request_id, "method": method,
                  "route": "/graphs/faults/" + route, "body": body,
                  "actor": "act-writer" if write else "act-reader", "conditional_headers": condition or [],
                  "contract": "0.14", "write": write, "attempts": 0, "outcome": "Unknown"}
        record["intent_sha256"] = hashlib.sha256(canonical(record).encode()).hexdigest()
        # Persist original identity BEFORE dispatch, even if the response never arrives.
        save(self.work / ("intent-" + request_id + ".json"), record)
        with self.guard:
            self.receipt["requests"].append(record)
        return record

    def headers(self, record):
        return [("Content-Type", "application/json"), ("Omnigraph-Http-Api", "0.14"),
                ("Authorization", "Bearer " + self.tokens[record["actor"]]),
                *[(CONDITION, value) for value in record["conditional_headers"]]]

    def wire(self, record, port):
        body = b"" if record["body"] is None else canonical(record["body"]).encode()
        headers = self.headers(record) + [("Host", f"127.0.0.1:{port}"),
                                          ("Content-Length", str(len(body))), ("Connection", "close")]
        return ((record["method"] + " " + record["route"] + " HTTP/1.1\r\n"
                 + "".join(k + ": " + v + "\r\n" for k, v in headers) + "\r\n").encode() + body)

    def send(self, record, port=None, seconds=15):
        require(record["attempts"] == 0, "Original request already dispatched; retries forbidden")
        record["attempts"] = 1
        record["start_ns"] = time.time_ns()
        record["socket_timeout_seconds"] = seconds
        connection = http.client.HTTPConnection("127.0.0.1", self.port if port is None else port, timeout=seconds)
        with self.guard:
            self.connections.append(connection)
        try:
            connection.putrequest(record["method"], record["route"])
            for key, value in self.headers(record):
                connection.putheader(key, value)
            raw_body = b"" if record["body"] is None else canonical(record["body"]).encode()
            connection.putheader("Content-Length", str(len(raw_body)))
            connection.putheader("Connection", "close")
            connection.endheaders(raw_body)
            response = connection.getresponse()
            record["status"] = response.status
            record["response_contract"] = response.getheader("Omnigraph-Http-Api")
            require(record["response_contract"] == "0.14", "Incompatible response; effects Unknown")
            raw = response.read(LIMIT + 1)
            require(len(raw) <= LIMIT, "Response too large; effects Unknown")
            for token in self.tokens.values():
                raw = raw.replace(token.encode(), b"[REDACTED]")
            (self.work / ("response-" + record["request_id"] + ".json")).write_bytes(raw)
            result = json.loads(raw)
            record["response"] = result
            record["outcome"] = "observed_response"
            return response.status, result
        except BaseException as error:
            record["failure_class"] = type(error).__name__
            raise
        finally:
            connection.close()
            record["end_ns"] = time.time_ns()

    def request(self, name, route, body=None, *, seconds=15, **kwargs):
        return self.send(self.prepare(name, route, body, **kwargs), seconds=seconds)

    def inventory(self, name, snapshot=None, *, seconds=15):
        body = {"params": {}, "expect_mutation": False}
        body.update({"snapshot": snapshot} if snapshot else {"branch": "main"})
        status, result = self.request(name, "queries/inventory", body, seconds=seconds)
        self.check(name + ": complete native projection", status == 200 and result.get("query_name") == "inventory"
                   and type(result.get("row_count")) is int and result["row_count"] == len(result.get("rows", []))
                   and isinstance(result.get("graph_commit_id"), str) and bool(result["graph_commit_id"])
                   and result.get("target", {}).get("snapshot" if snapshot else "branch") == (snapshot or "main"))
        if snapshot:
            self.check(name + ": exact snapshot identity", result["graph_commit_id"] == snapshot)
        return result["graph_commit_id"], result["rows"]

    def state(self, name, rows, pin, *, snapshot=False):
        actual_pin, actual = self.inventory(name, pin if snapshot else None)
        self.check(name + ": exact pin and complete typed rows", actual_pin == pin and same(actual, rows))

    def ready(self, rows):
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            require(self.server.process.poll() is None, "Native owner exited before readiness")
            try:
                pin, actual = self.inventory("authenticated-readiness", seconds=max(0.01, min(3, deadline - time.monotonic())))
                self.check("ready within deadline with complete rows", time.monotonic() <= deadline and same(actual, rows))
                return pin
            except (ConnectionRefusedError, ConnectionResetError, TimeoutError):
                time.sleep(0.1)  # Only read-only readiness is retried.
        raise TimeoutError("Native readiness deadline expired")

    def history(self, name):
        status, result = self.request(name, "commits?branch=main", method="GET")
        commits = result.get("commits", [])
        self.check(name + ": complete bounded native history", status == 200 and isinstance(commits, list)
                   and 0 < len(commits) <= self.definition["bounds"]["history_commits"]
                   and not result.get("next_page_token"))
        ids = [entry["graph_commit_id"] for entry in commits]
        self.check(name + ": unique native identities", len(ids) == len(set(ids)))
        return commits

    def change_intent(self, label, pin, key="a", route=CONDITIONAL, headers=None):
        value = label + ":" + secrets.token_hex(16)
        body = {"params": {"key": key, "value": value}, "branch": "main", "expect_mutation": True}
        return self.prepare(label, route, body, write=True, condition=[pin] if headers is None else headers)

    def changed_rows(self, original, record):
        rows = copy.deepcopy(original)
        for row in rows:
            if row["c.key"] == record["body"]["params"]["key"]:
                row["c.value"] = record["body"]["params"]["value"]
        return rows

    def acknowledged(self, record, response, old):
        status, result = response
        self.check(record["name"] + ": native mutation acknowledged", status == 200
                   and result.get("actor_id") == "act-writer" and result.get("affected_nodes") == 1)
        pin = result["commit"]["graph_commit_id"]
        self.check(record["name"] + ": new native identity", isinstance(pin, str) and bool(pin) and pin != old)
        record["native_commit_id"] = pin
        return pin

    def refused(self, record, response, status, code, actual=None):
        actual_status, result = response
        self.check(record["name"] + ": typed no-effect refusal", actual_status == status and result.get("code") == code
                   and result.get("disposition", {}).get("effects") == "none")
        if actual is not None:
            self.check(record["name"] + ": exact precondition identities",
                       same(result.get("precondition_failure"), {"expected": record["conditional_headers"][0], "actual": actual}))

    def second_owner(self, label):
        port = free_port()
        attempted = self.owned(label, self.server_argv(port))
        result = attempted.finish(15)
        diagnostic = (self.work / (label + ".stderr")).read_text()
        self.check(label + ": typed competing owner refused", result["exit"] != 0 and self.settled(result)
                   and not result["forced_kill"] and "state_lock_held" in diagnostic)
        self.check(label + ": original owner still alive", self.server.process.poll() is None)
        with socket.socket() as endpoint:
            endpoint.settimeout(1)
            self.check(label + ": refused endpoint closed", endpoint.connect_ex(("127.0.0.1", port)) == errno.ECONNREFUSED)

    def stale(self):
        intent = self.change_intent("advance-once", self.seed_pin)
        pin = self.acknowledged(intent, self.send(intent), self.seed_pin)
        rows = self.changed_rows(self.initial, intent)
        self.state("advanced complete rows", rows, pin)
        history = self.history("before stale rejection")
        for key in ("b", "missing-key"):
            refused = self.change_intent("stale-" + key, self.seed_pin, key)
            self.refused(refused, self.send(refused), 412, "precondition_failed", pin)
            self.state("stale unchanged head " + key, rows, pin)
            self.state("stale old pin " + key, self.initial, self.seed_pin, snapshot=True)
            self.check("stale unchanged complete history " + key, same(self.history("history after " + key), history))

    def malformed(self):
        history = self.history("before malformed requests")
        variants = [("missing", []), ("empty", [""]), ("wildcard", ["*"]),
                    ("quoted", ['"' + self.seed_pin + '"']), ("weak-etag", ['W/"' + self.seed_pin + '"']),
                    ("comma-list", [self.seed_pin + "," + self.seed_pin]),
                    ("duplicate", [self.seed_pin, self.seed_pin])]
        for label, headers in variants + [("ordinary-route", [self.seed_pin])]:
            record = self.change_intent(label, self.seed_pin,
                                        route="queries/change" if label == "ordinary-route" else CONDITIONAL,
                                        headers=headers)
            self.refused(record, self.send(record), 400, "bad_request")
            self.state(label + " unchanged complete head", self.initial, self.seed_pin)
            self.state(label + " unchanged old snapshot", self.initial, self.seed_pin, snapshot=True)
            self.check(label + " unchanged complete history", same(self.history(label + " history"), history))

    def race(self):
        before = self.history("before concurrent intents")
        records = [self.change_intent("race-" + key, self.seed_pin, key) for key in ("a", "b")]
        barrier = threading.Barrier(len(records) + 1, timeout=10)
        results, errors = {}, {}

        def run(index):
            try:
                barrier.wait()
                results[index] = self.send(records[index])
            except BaseException as error:
                errors[index] = type(error).__name__

        for index in range(len(records)):
            thread = threading.Thread(target=run, args=(index,), daemon=True)
            self.threads.append(thread)
            thread.start()
        barrier.wait()
        for thread in self.threads:
            thread.join(20)
        self.receipt["race"] = {"request_ids": [r["request_id"] for r in records], "errors": errors,
                                 "boundary": "two client threads released by one barrier; native scheduling uninstrumented"}
        self.check("all concurrent responses observed", not errors and len(results) == len(records)
                   and all(not thread.is_alive() for thread in self.threads))
        winners = [index for index, (status, _) in results.items() if status == 200]
        self.check("exactly one same-base winner", len(winners) == 1)
        winner = winners[0]
        pin = self.acknowledged(records[winner], results[winner], self.seed_pin)
        for index, record in enumerate(records):
            if index != winner:
                self.refused(record, results[index], 412, "precondition_failed", pin)
        rows = self.changed_rows(self.initial, records[winner])
        self.state("race exact complete winning state", rows, pin)
        self.state("race old snapshot unchanged", self.initial, self.seed_pin, snapshot=True)
        after = self.history("after concurrent intents")
        self.check("only winning native commit added", len(after) == len(before) + 1
                   and after[0]["graph_commit_id"] == pin and same(after[1:], before))

    def overlap(self):
        before = self.history("before second owner")
        self.second_owner("adversarial-second-owner")
        self.state("owner refusal preserves complete head", self.initial, self.seed_pin)
        self.check("owner refusal preserves complete history", same(self.history("after second owner"), before))

    def reconcile(self, record, before, expected_commit=None):
        expected = self.changed_rows(self.initial, record)
        deadline = time.monotonic() + 30
        pin, actual = self.seed_pin, self.initial
        while time.monotonic() < deadline:
            pin, actual = self.inventory("independent-original-intent-observation")
            if pin != self.seed_pin:
                break
            self.check("pending observation is coherent original state", same(actual, self.initial))
            time.sleep(1)  # Bounded read-only observation; retain complete logs, never repeat the write.
        record["resolution"] = {"original_request_id": record["request_id"], "native_commit_id": None,
                                 "observed_head": pin, "source": "independent complete native state and history",
                                 "status": "Unknown"}
        if pin == self.seed_pin and expected_commit is None:
            history = self.history("bounded disconnected-request observation")
            if same(history, before):
                self.state("original snapshot at observation cut", self.initial, self.seed_pin, snapshot=True)
                record["resolution"].update(
                    observation="head and history unchanged at this read cut; not proof of no effect",
                    observed_ns=time.time_ns())
                self.check("uncertain original request never replayed",
                           record["attempts"] == 1 and record["outcome"] == "Unknown")
                return None
            require(history, "Missing native history after disconnected request")
            pin, actual = self.inventory("late original-intent observation", history[0]["graph_commit_id"])
        self.check("original intent uniquely observed", pin != self.seed_pin and same(actual, expected))
        if expected_commit is not None:
            self.check("observed native commit matches retained acknowledgement", pin == expected_commit)
        record["resolution"]["native_commit_id"] = pin
        self.state("original old pin unchanged", self.initial, self.seed_pin, snapshot=True)
        self.state("resolved effect pinned exact rows", expected, pin, snapshot=True)
        after = self.history("original intent complete history")
        self.check("exactly one original effect commit", len(after) == len(before) + 1
                   and after[0]["graph_commit_id"] == pin and same(after[1:], before))
        record["resolution"]["status"] = "effect_observed"
        self.check("uncertain original request never replayed", record["attempts"] == 1 and record["outcome"] == "Unknown")
        return pin

    def lost_response(self):
        before = self.history("before response loss")
        record = self.change_intent("lost-response-original", self.seed_pin)
        listener = socket.socket()
        self.sockets.append(listener)
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        listener.settimeout(15)
        port = listener.getsockname()[1]
        wire = self.wire(record, port)
        proxy = {"forwarded_requests": 0,
                 "boundary": self.definition["boundaries"]["lost_response"]}
        self.receipt["fault_proxy"] = proxy

        def relay():
            connection = None
            try:
                client, _ = listener.accept()
                self.sockets.append(client)
                client.settimeout(15)
                received = bytearray()
                while len(received) < len(wire):
                    chunk = client.recv(min(8192, len(wire) - len(received)))
                    require(bool(chunk), "Proxy request disconnected before full body")
                    received.extend(chunk)
                require(bytes(received) == wire, "Proxy must forward exact original request bytes")
                upstream = socket.create_connection(("127.0.0.1", self.port), timeout=15)
                self.sockets.append(upstream)
                upstream.sendall(received)
                proxy["forwarded_requests"] += 1
                connection = http.client.HTTPResponse(upstream)
                connection.begin()
                raw = connection.read(LIMIT + 1)
                require(len(raw) <= LIMIT, "Withheld native response exceeded bound")
                for token in self.tokens.values():
                    raw = raw.replace(token.encode(), b"[REDACTED]")
                (self.work / "proxy-withheld-native-response.json").write_bytes(raw)
                proxy.update({"native_status": connection.status,
                              "native_contract": connection.getheader("Omnigraph-Http-Api"),
                              "native_response": json.loads(raw), "upstream_response_consumed_ns": time.time_ns()})
                client.shutdown(socket.SHUT_RDWR)
                client.close()  # Intentionally withhold EVERY real response byte.
            except BaseException as error:
                proxy["failure_class"] = type(error).__name__
            finally:
                if connection is not None:
                    connection.close()
                listener.close()

        thread = threading.Thread(target=relay, daemon=True)
        self.threads.append(thread)
        thread.start()
        client = socket.create_connection(("127.0.0.1", port), timeout=20)
        self.sockets.append(client)
        record["attempts"] = 1
        record["start_ns"] = time.time_ns()
        client.sendall(wire)
        try:
            received = client.recv(1)
            record["transport_observation"] = "EOF" if not received else "unexpected_response_bytes"
        except ConnectionResetError:
            record["transport_observation"] = "connection_reset"
        finally:
            client.close()
            record["end_ns"] = time.time_ns()
        thread.join(20)
        self.check("real response withheld by finished one-shot relay", not thread.is_alive()
                   and not proxy.get("failure_class") and proxy["forwarded_requests"] == 1
                   and proxy.get("native_status") == 200
                   and proxy.get("native_contract") == "0.14"
                   and record["transport_observation"] in ("EOF", "connection_reset"))
        self.reconcile(record, before, proxy["native_response"]["commit"]["graph_commit_id"])

    def disconnect(self):
        before = self.history("before disconnect")
        record = self.change_intent("disconnected-original", self.seed_pin)
        with socket.create_connection(("127.0.0.1", self.port), timeout=15) as client:
            record["attempts"] = 1
            record["start_ns"] = time.time_ns()
            client.sendall(self.wire(record, self.port))
            record["full_request_sent_ns"] = time.time_ns()
            client.shutdown(socket.SHUT_RDWR)
        record["end_ns"] = time.time_ns()
        record["transport_observation"] = "client closed before reading any response"
        self.receipt["disconnect_boundary"] = self.definition["boundaries"]["disconnect"]
        self.second_owner("adversarial-owner-after-disconnect")
        self.reconcile(record, before)
        self.check("disconnect retained original native owner", self.server.process.poll() is None)

    def stop(self, kill=False):
        if self.server is None:
            return
        process = self.server
        if kill:
            process.record["fault_boundary"] = "after acknowledged publication and complete history/pin observations"
            process.kill()
        result = process.finish(GRACE + 5, terminate=not kill)
        with socket.socket() as endpoint:
            endpoint.settimeout(1)
            closed = endpoint.connect_ex(("127.0.0.1", self.port)) == errno.ECONNREFUSED
        self.receipt["ownership"].append({"operation": "kill-reap" if kill else "graceful-stop",
                                          "pid": process.process.pid, "endpoint_refused": closed,
                                          "reaped": result["reaped"], "group_absent": result["group_absent"],
                                          "logs_drained": result["logs_drained"]})
        self.check("native owner quiescent " + str(self.server_generation), self.settled(result) and closed
                   and (result["exit"] == -signal.SIGKILL if kill else result["exit"] == 0 and not result["forced_kill"]))
        self.server = None

    def killed(self):
        known = {self.seed_pin: self.initial, self.receipt["empty_pin"]: []}
        rows, pin = self.initial, self.seed_pin
        for key in ("a", "b"):
            record = self.change_intent("pre-kill-acknowledged-" + key, pin, key)
            pin = self.acknowledged(record, self.send(record), pin)
            rows = self.changed_rows(rows, record)
            known[pin] = rows
            self.state("acknowledged before kill " + key, rows, pin)
        before = self.history("complete history before kill")
        observed = {}
        for entry in before:
            historical_pin = entry["graph_commit_id"]
            _, historical_rows = self.inventory("pre-kill retained snapshot", historical_pin)
            expected = known.get(historical_pin, [])
            self.check("complete pre-kill history canonical state", same(historical_rows, expected))
            observed[historical_pin] = historical_rows
        self.receipt["kill_boundary"] = self.definition["boundaries"]["kill"]
        self.stop(kill=True)
        status = self.cli("stopped-status", ["--target", self.target, "cluster", "status", "--json"])
        self.check("stopped exact canonical root", status.get("canonical_root") == self.target)
        lock = status.get("lock_id")
        self.check("killed local owner retains exact admission lock", isinstance(lock, str) and bool(lock))
        self.unlock("killed-owner-exact-unlock", lock,
                    "SIGKILL observed; original group absent; pipes drained; endpoint refused; local storage only")
        self.start_server()
        reopened = self.ready(rows)
        self.check("reopened exact head identity", reopened == pin)
        self.check("reopened complete native history unchanged", same(self.history("complete reopened history"), before))
        for historical_pin, historical_rows in observed.items():
            self.state("reopened retained native snapshot", historical_rows, historical_pin, snapshot=True)

    def cleanup(self):
        failures = []
        for connection in self.connections:
            try:
                if connection.sock is not None:
                    connection.sock.shutdown(socket.SHUT_RDWR)
                connection.close()
            except OSError:
                pass
        for owned_socket in self.sockets:
            try:
                owned_socket.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            owned_socket.close()
        for thread in self.threads:
            thread.join(2)
        if any(thread.is_alive() for thread in self.threads):
            failures.append("owned client/proxy thread did not quiesce")
        try:
            self.stop()
        except BaseException as error:
            failures.append(type(error).__name__ + ": " + str(error))
        for process in self.processes:
            if not process.record.get("reaped") or not process.record.get("group_absent"):
                process.finish(GRACE + 5, terminate=True)
            if not self.settled(process.record):
                failures.append("owned process not completely reaped/drained: " + process.record["name"])
        self.receipt["cleanup"] = {"pass": not failures, "failures": failures,
                                   "native_root_retained": True, "unknown_state_deleted": False,
                                   "threads_quiescent": all(not t.is_alive() for t in self.threads)}
        self.receipt["checks"].append({"name": "owned teardown complete", "pass": not failures})
        if failures:
            self.receipt["status"] = "fail"
        self.receipt["end_ns"] = time.time_ns()
        self.receipt["elapsed_seconds"] = (self.receipt["end_ns"] - self.receipt["start_ns"]) / 1e9
        encoded = json.dumps(self.receipt)
        for token in self.tokens.values():
            encoded = encoded.replace(token, "[REDACTED]")
        self.receipt = json.loads(encoded)
        save(self.work / "receipt.json", self.receipt)


CASES = {
    "conditional-stale-head-no-effect": Case.stale,
    "conditional-header-never-downgrades": Case.malformed,
    "concurrent-condition-single-winner": Case.race,
    "second-writer-refused": Case.overlap,
    "lost-response-original-effect-observed": Case.lost_response,
    "client-disconnect-does-not-release-owner": Case.disconnect,
    "killed-owner-reopen-preserves-complete-history": Case.killed,
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    require(args.work.is_absolute(), "Work directory must be absolute")
    require(args.work.parent.resolve().is_relative_to(Path("/evidence")), "Work must be beneath /evidence")
    work = args.work
    work.mkdir(mode=0o700, exist_ok=False)
    receipt = {"schema": "writfold-native-faults/1", "scope": "bounded exact-source native failure qualification",
               "status": "blocked", "wf24": "not_run", "owner_service_admission": "not_run",
               "checks": [], "cases": [], "start_ns": time.time_ns()}
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM):
        signal.signal(signum, interrupted)
    try:
        definition = json.loads((FIXTURE / "definition.json").read_text())
        profile = validate_profile(args.profile, Path("/profile/source-build.json"))
        receipt.update({"profile_sha256": sha(args.profile), "source_commit": profile["source_commit"],
                        "source_build_receipt_sha256": sha(Path("/profile/source-build.json")),
                        "executables": {name: sha(Path("/native") / name) for name in ("omnigraph", "omnigraph-server")},
                        "fixtures": {p.name: sha(p) for p in sorted(FIXTURE.iterdir())},
                        "implementation_sha256": sha(Path(__file__)),
                        "owned_process_implementation_sha256": sha(ROOT / "scripts/writfold_omnigraph_companion.py"),
                        "profile_validation_implementation_sha256": sha(ROOT / "scripts/writfold_omnigraph_probe.py"),
                        "bounds": definition["bounds"], "boundaries": definition["boundaries"],
                        "unrun": definition["unrun"]})
        require(profile["source_commit"] == SOURCE_COMMIT, "Campaign requires the exact investigated source commit")
        require(all(profile["members"][name] == digest for name, digest in receipt["executables"].items()),
                "Native executable identity mismatch")
        required = profile["development_contract"]["required_failure_cases"]
        receipt["required_cases"] = required
        receipt["sources"] = [
            f"https://github.com/ModernRelay/omnigraph/blob/{SOURCE_COMMIT}/{path}"
            for path in (
                "docs/user/mutations/conditional.md",
                "docs/user/operations/server.md",
                "docs/user/operations/storage-failures.md",
                "openapi.json",
                "crates/omnigraph-server/tests/integration/stored_queries.rs",
                "crates/omnigraph-server/tests/boot_settings.rs",
                "crates/omnigraph-cli/src/commands/cluster/args.rs",
                "crates/omnigraph-cli/src/commands/cluster/mod.rs",
            )
        ]
        require(required == definition["required_cases"] and set(required) == set(CASES) and len(required) == len(CASES),
                "Required cases missing, duplicated or not implemented; no partial suite pass")
        receipt["checks"].append({"name": "exact source executable and complete case definitions", "pass": True})
        receipt["status"] = "fail"
        for name in required:
            case = Case(work / name, name, definition)
            signal.setitimer(signal.ITIMER_REAL, definition["bounds"]["case_seconds"])
            try:
                case.bootstrap()
                CASES[name](case)
                case.receipt["status"] = "pass"
            except BaseException as error:
                case.receipt["failure"] = {"class": type(error).__name__, "message": str(error),
                                           "write_retry": False}
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
                case.cleanup()
                receipt["cases"].append(case.receipt)
                save(work / "receipt.json", receipt)
            if not case.receipt["cleanup"]["pass"]:
                # A non-quiescent owner blocks more work, even on a different root.
                break
        present = {case["name"] for case in receipt["cases"]}
        for name in required:
            if name not in present:
                receipt["cases"].append({"name": name, "status": "blocked", "checks": [
                    {"name": "required case executed", "pass": False}],
                    "reason": "Prior ownership did not quiesce; no further native owner started"})
        passed = len(receipt["cases"]) == len(required) and all(case["status"] == "pass" and case["checks"]
                   and all(check["pass"] for check in case["checks"]) for case in receipt["cases"])
        receipt["checks"].append({"name": "every frozen required case passed", "pass": passed})
        receipt["status"] = "pass" if passed else "fail"
    except BaseException as error:
        receipt["failure"] = {"class": type(error).__name__, "message": str(error)}
        receipt["checks"].append({"name": "complete campaign prerequisites and execution", "pass": False})
        receipt["status"] = "blocked" if not receipt["cases"] else "fail"
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        present = {case["name"] for case in receipt["cases"]}
        for name in receipt.get("required_cases", list(CASES)):
            if name not in present:
                receipt["cases"].append({
                    "name": name, "status": "blocked",
                    "checks": [{"name": "required case executed", "pass": False}],
                    "reason": "Campaign prerequisite or ownership failure prevented execution",
                })
        receipt["end_ns"] = time.time_ns()
        receipt["elapsed_seconds"] = (receipt["end_ns"] - receipt["start_ns"]) / 1e9
        save(work / "receipt.json", receipt)
    return 0 if receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
