#!/usr/bin/env python3
"""Bounded native workload qualification; never WF24 or product admission."""
from __future__ import annotations

import argparse
import errno
import http.client
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import threading
import time

from writfold_omnigraph_companion import Process, Probe, GRACE, LIMIT, require, save, sha
from writfold_omnigraph_probe import validate_profile

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures/qualification/omnigraph-workload"
SOURCE = "1a856292c569e214333ab16b1d1af4d0e2de4ac2"
QUERIES = ("inventory", "edges", "point", "discovery")


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def exact(actual, expected):
    # Python equality alone incorrectly accepts True == 1 == 1.0.
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(exact(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(exact(a, b) for a, b in zip(actual, expected))
    return actual == expected


def key(i):
    return f"asset-{i:06d}"


def node(i, epoch, seed):
    # Candidate docs/user/mutations/index.md: a single String key is its native ID.
    value = {"native_id": key(i), "slug": key(i), "name": f"Asset λ {i}", "ordinal": i,
             "active": (i + seed) % 2 == 0, "score": ((i + seed) % 97) + 0.25,
             "note": None if i % 3 != 2 else f"note-{i}"}
    if epoch == 2:
        value["epoch"] = 2
    return value


def edge(i, count):
    # Candidate docs/user/schema/index.md: keyed edge identity is the compact
    # JSON tuple in catalog order, @src then @dst.
    source, destination = key(i), key((i + 1) % count)
    return {"native_id": encoded([source, destination]),
            "native_source": source, "native_destination": destination,
            "source": source, "destination": destination, "ordinal": i}


def load_node(row):
    data = dict(row)
    del data["native_id"]  # Projection metadata is never a user property.
    # Exercise omitted and explicit-null inputs; Arrow JSON may omit a projected null cell.
    if data["ordinal"] % 3 == 0:
        del data["note"]
    return {"type": "Asset", "data": data}


def load_edge(row):
    return {"edge": "DependsOn", "from": row["native_source"], "to": row["native_destination"],
            "data": {"ordinal": row["ordinal"]}}


def rows(result, epoch, query):
    if query in ("inventory", "point"):
        columns = ["native_id", "slug", "name", "ordinal", "active", "score", "note"]
        if epoch == 2:
            columns.append("epoch")
        nullable = {"note"}
    elif query == "edges":
        columns = ["native_id", "native_source", "native_destination", "source", "destination", "ordinal"]
        nullable = set()
    else:
        require(query == "discovery", "Unknown exact projection")
        columns, nullable = ["native_id", "slug"], set()
    require(result.get("columns") == columns and type(result.get("rows")) is list
            and type(result.get("row_count")) is int
            and len(result["rows"]) == result["row_count"], "Incomplete explicit projection")
    expected = set(columns)
    normalized = []
    for row in result["rows"]:
        require(type(row) is dict and set(row) <= expected and expected - set(row) <= nullable,
                "Missing nonnullable or extra native projected field")
        # Same admitted Arrow-JSON rule as the epoch probe: only declared nullable
        # columns may be absent. Raw responses stay retained without alteration.
        normalized.append({column: row.get(column) for column in columns})
    return normalized


class Campaign(Probe):
    def __init__(self, work, profile, count, shared):
        super().__init__(work, profile)
        self.count = count
        self.receipt = shared
        self.cfg = self.profile["development_contract"]["workloads"]
        self.env["RUST_LOG"] = self.cfg["native_log_level"]
        self.receipt.setdefault("native_environments", []).append({
            "assets": count, "environment": dict(self.env),
            "credential_binding": "fresh private synthetic token file; secret values never recorded"})
        self.slo = self.cfg["slo"]
        self.native_root = work / "cluster"
        self.serial = 0
        self.query_sequence = {}
        self.monitor_stop = threading.Event()
        self.measurements = []
        self.receipt.setdefault("resource_observations", []).append({"assets": count, "processes": self.measurements})
        self.monitor = threading.Thread(target=self.observe, daemon=True)
        self.monitor.start()
        self.processes = []
        self.stage_deadline = None

    def observe(self):
        while not self.monitor_stop.wait(0.02):
            for process, observation in tuple(getattr(self, "processes", ())):
                try:
                    text = Path(f"/proc/{process.process.pid}/status").read_text()
                    for field in ("VmRSS", "VmHWM"):
                        match = re.search(rf"^{field}:\s+(\d+) kB$", text, re.M)
                        if match:
                            observation[field + "_bytes"] = max(observation.get(field + "_bytes", 0), int(match[1]) * 1024)
                    observation["samples"] += 1
                except FileNotFoundError:
                    pass
                except OSError as error:
                    observation["observation_error"] = type(error).__name__

    def launch(self, name, argv):
        self.serial += 1
        process = Process(self.work, f"{self.count}-{self.serial}-{name}", argv, self.env,
                          self.tokens.values(), self.receipt["commands"])
        observation = {"pid": process.process.pid, "name": process.record["name"], "samples": 0}
        self.measurements.append(observation)
        self.processes.append((process, observation))
        return process

    def cli(self, name, args):
        require(self.server is None, "CLI cannot open a live server root")
        process = self.launch(name, ["/native/omnigraph", *args])
        result = process.finish(120)
        self.check(f"{self.count}/{name}/quiescence", result["exit"] == 0 and result["group_absent"]
                   and result["logs_drained"] and not result["forced_kill"]
                   and all(not x.get("error") and not x["truncated"] for x in result["logs"].values()))
        return self.work / (process.record["name"] + ".stdout"), self.work / (process.record["name"] + ".stderr")

    def setup(self):
        bundle = self.work / "bundle"
        shutil.copytree(FIXTURE, bundle)
        self.native_root.mkdir()
        self.cli("validate", ["cluster", "validate", "--config", str(bundle)])
        out, err = self.cli("apply", ["--target", self.native_root.as_uri(), "cluster", "apply", "--config", str(bundle), "--json"])
        result = json.loads(out.read_text())
        self.check(f"{self.count}/bootstrap", result["status"] == "complete" and result["result"]["converged"] is True)
        locks = re.findall(r"^Admission lock: (\S+); retain until prior work is quiescent, then exact-ID force-unlock$", err.read_text(), re.M)
        roots = re.findall(r"^Cluster-root: (\S+)$", err.read_text(), re.M)
        require(len(locks) == 1 and roots == [self.native_root.as_uri()], "Exact stopped CLI lock/root missing")
        out, _ = self.cli("handoff", ["--target", self.native_root.as_uri(), "cluster", "force-unlock", locks[0], "--json"])
        self.check(f"{self.count}/exact-id-handoff", json.loads(out.read_text()) == {"unlocked": True, "lock_id": locks[0]})
        token_file = self.work / "tokens.json"
        with open(token_file, "x", opener=lambda p, flags: os.open(p, flags, 0o600)) as stream:
            json.dump({k: v for k, v in self.tokens.items() if k != "wrong"}, stream)
        self.env["OMNIGRAPH_SERVER_BEARER_TOKENS_FILE"] = str(token_file)
        self.disk("after-bootstrap")

    def request_native(self, epoch, name, body, *, route=None, pin=None, writer=False, seconds=15):
        if self.stage_deadline is not None:
            seconds = min(seconds, self.stage_deadline - time.monotonic())
        require(seconds > 0, "Stage deadline exhausted; no request dispatched")
        route = f"/graphs/workload-e{epoch}/" + (route or "queries/" + name)
        raw_body = encoded(body).encode()
        number = len(self.receipt["requests"]) + 1
        request_file = self.work / f"request-{number:06d}.json"
        request_file.write_bytes(raw_body)
        record = {"name": name, "assets": self.count, "epoch": epoch, "route": route,
                  "request_file": str(request_file), "request_sha256": sha(request_file),
                  "conditional_pin": pin, "actor": "act-writer" if writer else "act-reader", "outcome": "unknown"}
        self.receipt["requests"].append(record)
        headers = {"Content-Type": "application/json", "Omnigraph-Http-Api": "0.14",
                   "Authorization": "Bearer " + self.tokens[record["actor"]]}
        if pin is not None:
            headers["Omnigraph-If-Graph-Commit"] = pin
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=seconds)
        start = time.monotonic()
        signal.setitimer(signal.ITIMER_REAL, seconds)
        try:
            connection.request("POST", route, raw_body, headers)
            response = connection.getresponse()
            record["status"] = response.status
            record["http_contract"] = response.getheader("Omnigraph-Http-Api")
            raw = response.read(LIMIT + 1)
            record["duration_ms"] = (time.monotonic() - start) * 1000
            require(len(raw) <= LIMIT, "Response exceeded bound")
            for token in self.tokens.values():
                raw = raw.replace(token.encode(), b"[REDACTED]")
            path = self.work / f"response-{number:06d}.json"
            path.write_bytes(raw)
            record.update(response_file=str(path), response_sha256=sha(path), outcome="observed")
            require(record["http_contract"] == "0.14" and response.status == 200, "Native HTTP request failed")
            return json.loads(raw), record["duration_ms"]
        except BaseException as error:
            record["error"] = type(error).__name__
            raise
        finally:
            record.setdefault("duration_ms", (time.monotonic() - start) * 1000)
            signal.setitimer(signal.ITIMER_REAL, 0)
            connection.close()

    def start(self):
        require(self.server is None, "Previous owner remains")
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.port = sock.getsockname()[1]
        start = time.monotonic()
        self.server = self.launch("server", ["/native/omnigraph-server", "--cluster", self.native_root.as_uri(),
                                   "--bind", f"127.0.0.1:{self.port}", "--require-all-graphs",
                                   "--shutdown-grace-seconds", str(GRACE)])
        deadline = start + self.slo["server_ready_seconds"]
        while time.monotonic() < deadline:
            require(self.server.process.poll() is None, "Server died before readiness")
            try:
                # Missing-key readiness avoids priming inventory/discovery, but is still cache conditioning.
                result, _ = self.request_native(1, "point", {"params": {"key": "readiness-absent"}, "expect_mutation": False}, seconds=min(3, deadline-time.monotonic()))
                self.check(self.server.record["name"] + "/ready", exact(result.get("rows"), [])
                           and result.get("row_count") == 0 and result.get("query_name") == "point"
                           and time.monotonic() <= deadline)
                self.receipt.setdefault("readiness", []).append({"assets": self.count, "seconds": time.monotonic()-start})
                return
            except (ConnectionError, OSError):
                time.sleep(0.1)
        raise TimeoutError("Readiness budget exceeded")

    def stop(self):
        if self.server is None:
            return
        result = self.server.finish(GRACE + 5, terminate=True)
        with socket.socket() as sock:
            sock.settimeout(1)
            refused = sock.connect_ex(("127.0.0.1", self.port)) == errno.ECONNREFUSED
        self.check(result["name"] + "/quiescence", result["exit"] == 0 and result["group_absent"]
                   and not result["forced_kill"] and result["logs_drained"] and refused
                   and all(not x.get("error") and not x["truncated"] for x in result["logs"].values()))
        self.server = None

    def disk(self, stage):
        files = [p.stat() for p in self.native_root.rglob("*") if p.is_file()]
        apparent = sum(s.st_size for s in files)
        allocated = sum(s.st_blocks * 512 for s in files)
        self.receipt.setdefault("disk", []).append({"assets": self.count, "stage": stage,
                "apparent_bytes": apparent, "allocated_bytes": allocated, "baseline_bytes": 0})
        self.check(f"{self.count}/{stage}/disk-budget", max(apparent, allocated) <= self.slo["native_disk_growth_bytes"])

    def check_name(self, epoch, query):
        key = (epoch, query)
        self.query_sequence[key] = self.query_sequence.get(key, 0) + 1
        return f"{self.count}/e{epoch}/{query}/{self.query_sequence[key]}"

    def query(self, epoch, name, pin, params, expected):
        result, ms = self.request_native(epoch, name, {"params": params, "snapshot": pin, "expect_mutation": False})
        projected = rows(result, epoch, name)
        self.check(self.check_name(epoch, name),
                   result.get("query_name") == name and result.get("graph_commit_id") == pin
                   and result.get("target", {}).get("snapshot") == pin
                   and exact(projected, expected))
        return projected, ms

    def inventory(self, epoch, pin, label, mapping=None):
        start = time.monotonic()
        prior = self.stage_deadline
        self.stage_deadline = min(prior or float("inf"), start + self.slo["complete_inventory_seconds"])
        try:
            for name in ("inventory", "edges"):
                after = ""
                destination = ""
                for offset in range(0, self.count + 1, self.cfg["page_rows"]):
                    expected = [node(i, epoch, self.cfg["seed"]) if name == "inventory" else edge(i, self.count)
                                for i in range(offset, min(offset+self.cfg["page_rows"], self.count))]
                    params = {"after": after}
                    if name == "edges":
                        params["destination"] = destination
                    rows, _ = self.query(epoch, name, pin, params, expected)
                    if mapping is not None:
                        for row in rows:
                            mapped = dict(row)
                            if name == "inventory":
                                mapped["epoch"] = 2
                            mapping.write(encoded({"kind": name, "old_graph": "workload-e1", "old_pin": pin,
                                                  "old": row, "new_graph": "workload-e2", "new": mapped}) + "\n")
                    if rows:
                        after = rows[-1]["slug" if name == "inventory" else "source"]
                        if name == "edges":
                            destination = rows[-1]["destination"]
            elapsed = time.monotonic()-start
            self.receipt.setdefault("inventories", []).append({"assets": self.count, "epoch": epoch, "pin": pin,
                                                               "label": label, "seconds": elapsed, "nodes": self.count, "edges": self.count})
            self.check(f"{self.count}/{label}/inventory-budget", elapsed <= self.slo["complete_inventory_seconds"])
        finally:
            self.stage_deadline = prior

    def ingest(self, epoch, source):
        start = time.monotonic()
        prior = self.stage_deadline
        self.stage_deadline = min(prior or float("inf"), start + self.slo["ingest_each_size_seconds"])
        total = 0
        pin = None
        try:
            with source.open() as stream:
                batch = []
                for line in stream:
                    batch.append(line)
                    if len(batch) == self.cfg["page_rows"]:
                        result, _ = self.request_native(epoch, "load", {"data": "".join(batch), "mode": "merge"}, route="load", writer=True, seconds=600)
                        require(result["total_entities"] == len(batch), "Incomplete load acknowledgment")
                        pin = result["commit"]["graph_commit_id"]
                        total += len(batch)
                        batch.clear()
                require(not batch, "Generator must produce whole batches")
            self.check(f"{self.count}/e{epoch}/ingest", total == 2*self.count and isinstance(pin, str) and bool(pin)
                       and time.monotonic()-start <= self.slo["ingest_each_size_seconds"])
            self.receipt.setdefault("ingests", []).append({"assets": self.count, "epoch": epoch, "pin": pin,
                                                          "seconds": time.monotonic()-start, "entities": total, "input_sha256": sha(source)})
            self.disk(f"e{epoch}-ingested")
            return pin
        finally:
            self.stage_deadline = prior

    def samples(self, epoch, pin):
        for name in QUERIES:
            def sample(i):
                index = (self.cfg["seed"] + i * 7919) % self.count
                if name in ("inventory", "edges"):
                    offset = index // self.cfg["page_rows"] * self.cfg["page_rows"]
                    params = {"after": key(offset-1) if offset else ""}
                    if name == "edges":
                        params["destination"] = key(offset) if offset else ""
                    expected = [node(j, epoch, self.cfg["seed"]) if name == "inventory" else edge(j, self.count)
                                for j in range(offset, min(offset+self.cfg["page_rows"], self.count))]
                elif name == "point":
                    params, expected = {"key": key(index)}, [node(index, epoch, self.cfg["seed"])]
                else:
                    params = {"key": key(index)}
                    expected = [{"native_id": slug, "slug": slug} for slug in sorted([key((index+1)%self.count), key((index+2)%self.count)])]
                return self.query(epoch, name, pin, params, expected)[1]
            # An explicit unmeasured warmup, retained like every other request.
            sample(0)
            warm = [sample(i) for i in range(self.cfg["warm_samples_per_query"])]
            cold = []
            for i in range(self.cfg["cold_process_samples_per_query"]):
                self.stop()
                self.start()
                cold.append(sample(i))
            p95 = sorted(warm)[math.ceil(len(warm)*0.95)-1]
            self.receipt.setdefault("samples", []).append({"assets": self.count, "epoch": epoch, "query": name,
                       "warm_ms": warm, "warm_p95_ms": p95, "process_cold_ms": cold})
            self.check(f"{self.count}/e{epoch}/{name}/warm-budget", p95 <= self.slo["warm_p95_query_ms"])
            self.check(f"{self.count}/e{epoch}/{name}/cold-budget", max(cold) <= self.slo["cold_process_max_query_ms"])

    def retain(self, epoch, pin):
        branch = f"retained-original-e{epoch}"
        result, _ = self.request_native(epoch, "retain-original",
                                       {"name": branch, "from_commit": pin},
                                       route="branches", writer=True)
        binding = {"name": branch, "from_commit": pin, "create_receipt": result,
                   "create_request": len(self.receipt["requests"])}
        self.check(f"{self.count}/e{epoch}/retained-branch-create",
                   result.get("name") == branch and result.get("from") == "main"
                   and result.get("from_commit_id") == pin)
        self.retained_read(epoch, pin, branch)
        return binding

    def retained_read(self, epoch, pin, branch):
        result, _ = self.request_native(epoch, "point", {
            "params": {"key": key(0)}, "branch": branch, "expect_mutation": False})
        self.check(self.check_name(epoch, "retained-branch-read"),
                   result.get("query_name") == "point" and result.get("graph_commit_id") == pin
                   and result.get("target", {}).get("branch") == branch
                   and type(result.get("row_count")) is int and result["row_count"] == 1
                   and exact(rows(result, epoch, "point"), [node(0, epoch, self.cfg["seed"])]))

    def run(self):
        seed_file = self.work / "seed-e1.jsonl"
        with seed_file.open("x") as stream:
            for i in range(self.count):
                stream.write(encoded(load_node(node(i, 1, self.cfg["seed"]))) + "\n")
            for i in range(self.count):
                stream.write(encoded(load_edge(edge(i, self.count))) + "\n")
        self.receipt.setdefault("generated", []).append({"assets": self.count, "seed": self.cfg["seed"], "path": str(seed_file), "sha256": sha(seed_file)})
        restore = {"assets": self.count, "seed": self.cfg["seed"],
                   "canonical_root": self.native_root.as_uri(),
                   "bundle": str(self.work / "bundle"), "epochs": {},
                   "generator": str(self.work.parent / "generator.py"),
                   "advanced_change": {"type": "Asset", "key": key(0), "property": "name", "value": "advanced"},
                   "native_history_requirement": "Restore the complete quiesced root at this canonical URI; seed/mapping files are independent exact-read oracles, not native-history backups"}
        self.receipt.setdefault("restore_inputs", []).append(restore)
        self.setup()
        self.start()
        old = self.ingest(1, seed_file)
        restore["epochs"]["1"] = {"graph": "workload-e1", "original_pin": old,
                                  "seed_file": str(seed_file), "seed_sha256": sha(seed_file)}
        restore["epochs"]["1"]["retained_branch"] = self.retain(1, old)
        self.inventory(1, old, "original")
        mapping_file = self.work / "epoch-mapping.jsonl"
        mapped_file = self.work / "seed-e2.jsonl"
        start = time.monotonic()
        self.stage_deadline = start + self.slo["epoch_mapping_seconds"]
        with mapping_file.open("x") as mapping:
            self.inventory(1, old, "mapping-source", mapping)
        with mapping_file.open() as source, mapped_file.open("x") as target:
            for line in source:
                item = json.loads(line)
                target.write(encoded(load_node(item["new"]) if item["kind"] == "inventory" else load_edge(item["new"])) + "\n")
        new_epoch = self.ingest(2, mapped_file)
        restore["epochs"]["2"] = {"graph": "workload-e2", "original_pin": new_epoch,
                                  "seed_file": str(mapped_file), "seed_sha256": sha(mapped_file)}
        restore["epochs"]["2"]["retained_branch"] = self.retain(2, new_epoch)
        restore.update(mapping_file=str(mapping_file), mapping_sha256=sha(mapping_file))
        self.inventory(2, new_epoch, "mapped-target")
        self.inventory(1, old, "old-after-migration")
        elapsed = time.monotonic()-start
        self.stage_deadline = None
        self.receipt.setdefault("mappings", []).append({"assets": self.count, "old_pin": old, "new_pin": new_epoch,
                     "mapping_sha256": sha(mapping_file), "mapping_file": str(mapping_file), "mapped_input_sha256": sha(mapped_file),
                     "nodes": self.count, "edges": self.count, "seconds": elapsed})
        self.check(f"{self.count}/mapping-budget", elapsed <= self.slo["epoch_mapping_seconds"])
        for epoch, pin in ((1, old), (2, new_epoch)):
            self.samples(epoch, pin)
            self.query(epoch, "point", pin, {"key": "absent"}, [])
            result, _ = self.request_native(epoch, "advance", {"params": {"name": "advanced"}, "branch": "main", "expect_mutation": True},
                                           route="queries/advance/if-graph-commit", pin=pin, writer=True)
            advanced = result["commit"]["graph_commit_id"]
            restore["epochs"][str(epoch)]["advanced_pin"] = advanced
            self.check(f"{self.count}/e{epoch}/head-advanced", isinstance(advanced, str) and bool(advanced) and advanced != pin)
            changed = node(0, epoch, self.cfg["seed"])
            changed["name"] = "advanced"
            self.query(epoch, "point", advanced, {"key": key(0)}, [changed])
            self.inventory(epoch, pin, "old-after-head-advance")
            self.query(epoch, "discovery", pin, {"key": key(self.count-1)},
                       [{"native_id": key(i), "slug": key(i)} for i in (0, 1)])
            self.retained_read(epoch, pin, restore["epochs"][str(epoch)]["retained_branch"]["name"])
        self.disk("final-live")
        self.stop()
        self.disk("final-quiescent")
        final_records = [process.record for process, _ in self.processes]
        self.check(f"{self.count}/native-rss-observed", bool(final_records) and all(
            type(record.get("rusage", {}).get("peak_rss_bytes")) is int
            and record["rusage"]["peak_rss_bytes"] > 0
            and not record.get("rusage_unavailable") for record in final_records))
        self.check(f"{self.count}/native-rss-budget", max(
            record["rusage"]["peak_rss_bytes"] for record in final_records) <= self.slo["native_peak_rss_bytes"])
        root_stat = self.native_root.lstat()
        restore["root_identity"] = {"device": root_stat.st_dev, "inode": root_stat.st_ino}
        restore["quiescent"] = True
        self.receipt.setdefault("completed_scales", []).append(self.count)


def interrupted(signum, frame):
    raise TimeoutError(f"Interrupted by signal {signum}; uncertain writes are never retried")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    args = parser.parse_args()
    require(args.work.is_absolute(), "Work directory must be absolute")
    args.work.mkdir(mode=0o700, parents=False, exist_ok=False)
    receipt = {"scope": "bounded deterministic native 10k/100k typed graph workload qualification",
               "status": "fail", "wf24": "not_run", "checks": [], "commands": [], "requests": [],
               "unrun_scope": ["full WF24", "owner-service admission", "fuzzy/full-text/vector retrieval", "agent benefit", "hardware-cold cache", "power-loss durability"],
               "cache_conditioning": "Warmup then 20 warm requests per query; 5 fresh server processes per query. Readiness performs an absent point lookup. OS cache uncontrolled; no cache dropping.",
               "rss_method": "Final Linux wait4 kernel peak_rss_bytes required for every owned native process and checked against the frozen SLO; includes the final exit interval. 20ms /proc VmRSS/VmHWM observations are diagnostic only. No simultaneous process-tree RSS claim.",
               "timing_method": "monotonic HTTP dispatch through bounded complete body read; includes serialization transport, excludes oracle comparison; phase timings include validation",
               "disk_method": "native-root apparent bytes and st_blocks allocation at bootstrap/ingest/end, empty-root baseline; transient between-observation peak not claimed"}
    campaign = None
    for sig in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, interrupted)
    try:
        profile = validate_profile(args.profile, Path("/profile/source-build.json"))
        require(profile["source_commit"] == SOURCE, "Wrong source candidate")
        cfg = profile["development_contract"]["workloads"]
        require(cfg["asset_counts"] == [10000, 100000] and cfg["page_rows"] == 1000
                and cfg["warm_samples_per_query"] == 20 and cfg["cold_process_samples_per_query"] == 5
                and cfg["seed"] == 20261106, "Frozen generator/sample profile mismatch")
        identities = {name: sha(Path("/native") / name) for name in ("omnigraph", "omnigraph-server")}
        require(all(profile["members"][name] == digest for name, digest in identities.items()), "Executable mismatch")
        receipt.update(source_commit=SOURCE, profile_sha256=sha(args.profile), executables=identities,
                       workload_profile=cfg, generator_sha256=sha(Path(__file__)),
                       fixtures={p.name: sha(p) for p in sorted(FIXTURE.iterdir()) if p.is_file()},
                       source_build_sha256=sha(Path("/profile/source-build.json")))
        shutil.copyfile(Path(__file__), args.work / "generator.py")
        shutil.copyfile(args.profile, args.work / "profile.json")
        for count in cfg["asset_counts"]:
            work = args.work / str(count)
            work.mkdir(mode=0o700)
            campaign = Campaign(work, args.profile, count, receipt)
            try:
                campaign.run()
            finally:
                try:
                    campaign.stop()
                finally:
                    campaign.monitor_stop.set()
                    campaign.monitor.join(timeout=2)
            campaign = None
        require(receipt.get("completed_scales") == cfg["asset_counts"], "Incomplete scale coverage")
        require(len(receipt.get("samples", [])) == 16 and all(c["pass"] for c in receipt["checks"]), "Incomplete or failed query coverage")
        receipt["status"] = "pass"
    except BaseException as error:
        receipt["failure"] = {"class": type(error).__name__, "message": str(error)}
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        save(args.work / "receipt.json", receipt)
    return 0 if receipt["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
