#!/usr/bin/env python3
"""Qualify the real static-musl transport-control executable, never corpus grounding."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import struct
import sys
import tarfile
import traceback

sys.dont_write_bytecode = True
from writfold_conformance_process import (
    ROOT, TARGET, manifest, observe_tools, record, retain_binary, sha, write_json,
)
from writfold_stdio_process import Pipes

SCOPE = ("transport-control-only: initialize/shutdown, notifications and bounded stdio; "
         "not child-tree, backend/native, corpus, authority, read/effect or Weft grounding "
         "qualification. WF24 and full transport remain blocked.")
BOUNDARIES = (
    "lf-inclusive-65536", "lf-inclusive-65537", "oversize-drain-recovery",
    "oversize-eof", "fragmented-unterminated-eof", "malformed-and-invalid-utf8",
    "escaped-and-large-integer-ids", "huge-id-output-budget",
    "shutdown-stops-reading", "sigterm-partial-frame", "broken-stdout",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def loads(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, f"Duplicate response key: {key}")
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def frame(method, ident=None, *, notification=False):
    value = {"jsonrpc": "2.0", "method": method}
    if not notification:
        value["id"] = ident
    if method == "initialize":
        value["params"] = {"protocol_version": "2"}
    return json.dumps(value, ensure_ascii=True, separators=(",", ":")).encode() + b"\n"


def expected(ident, kind, generation):
    return dict(id=ident, kind=kind, generation=generation)


def fault(code, generation=None, ident=None, category="malformed"):
    return dict(id=ident, code=code, category=category, generation=generation)


class BuildCommands:
    """Adapt the existing admitted-tool observer to bounded retained pipes."""
    def __init__(self, pipes, environment):
        self.pipes, self.env = pipes, environment
        self.directory, self.records = pipes.directory, pipes.records

    def run(self, name, argv, *, timeout=120, cwd=ROOT):
        _, _, status = self.pipes.run(name, argv, [("eof", None)], cwd=cwd,
                                      env=self.env, timeout=timeout, limit=32 * 1024 * 1024)
        require(status == 0, f"{name}: unexpected exit {status}")
        return Path(self.records[-1]["artifacts"]["stdout"]["path"])


class Producer:
    def __init__(self, output, cache):
        self.output, self.cache = output, cache
        self.preexisting = set(output.iterdir())
        self.pipes = Pipes(output)
        build_env = {key: value for key, value in os.environ.items()
                     if key in ("PATH", "HOME", "CC", "CXX", "AR", "LD", "CFLAGS", "LDFLAGS")
                     or key.startswith(("CARGO", "RUST", "LEAN", "LAKE", "ELAN"))}
        build_env.update(CARGO_NET_OFFLINE="true", CARGO_TARGET_DIR=str(cache),
                         PYTHONDONTWRITEBYTECODE="1")
        self.commands = BuildCommands(self.pipes, build_env)
        self.checks = {}
        for name in ("source-tool-binding", "contract-archive", "static-musl-build",
                     "contract-vectors", *BOUNDARIES, "evidence-integrity"):
            self.declare(name)
        self.report = dict(report_schema_version=1, result="fail", scope=SCOPE,
                           target=TARGET, profile="release", commands=self.pipes.records,
                           errors=[], skips=[], cases=[])
        self.baseline = None
        self.binary = None
        self.archive = None
        self.methods = None
        self.contract_digest = None
        write_json(output / "stdio-attempt.json", dict(argv=sys.argv, scope=SCOPE))

    def declare(self, name):
        require(name not in self.checks, f"Duplicate check {name}")
        self.checks[name] = dict(name=name, required=True, status="fail",
                                 errors=["Not exercised: prerequisite or producer failure"])

    def check(self, name, action):
        try:
            result = action()
            self.checks[name].update(status="pass", errors=[])
            return result
        except Exception as exc:
            message = f"{type(exc).__name__}: {exc}"
            self.checks[name]["errors"] = [message]
            self.report["errors"].append(dict(check=name, error=message))
            (self.output / f"{name}.failure.log").write_text(traceback.format_exc())
            return None

    def sources(self):
        paths = [ROOT / name for name in (
            "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "LICENSE", "compose.yaml",
            "toolchains.lock.json", "qualification/toolchain-observation.json",
            "docker/Dockerfile.tools", "scripts/writfold_stdio.py",
            "scripts/writfold_stdio_process.py", "scripts/writfold_conformance_process.py",
            "scripts/writfold_agent_contract.py")]
        for directory in ("crates", "vendor", ".cargo"):
            paths += [path for path in (ROOT / directory).rglob("*") if path.is_file()]
        return manifest(paths)

    def bind(self):
        require(platform.system() == "Linux" and platform.machine() == "x86_64",
                "Only admitted Linux x86_64 execution is supported")
        self.baseline = self.sources()
        self.report["sources"] = self.baseline
        write_json(self.output / "sources.json", self.baseline)
        self.report["tools"] = observe_tools(self.commands)
        for name in ("file", "readelf"):
            found = shutil.which(name)
            require(found is not None, f"Missing {name}")
            executable = Path(found).resolve(strict=True)
            output = self.commands.run(f"version-{name}", [str(executable), "--version"])
            self.report["tools"][name] = dict(**record(executable), version=output.read_text())
        return True

    def package(self):
        from writfold_agent_contract import export
        destination = self.output / "writfold-agent-contract.tar.gz"
        published = export(destination)
        require(destination.is_file(), "Publisher did not create the requested archive")
        self.archive = record(destination)
        require(published["sha256"] == self.archive["sha256"] and
                published["bytes"] == self.archive["bytes"], "Publisher/archive identity mismatch")
        self.report["contract_archive"] = dict(identity=self.archive, publisher=published)
        with tarfile.open(destination, "r:gz") as archive:
            members = archive.getmembers()
            names = [item.name for item in members]
            require(len(set(names)) == len(names) and all(item.isfile() for item in members),
                    "Archive contains duplicates or nonregular members")
            require(all(not name.startswith("/") and ".." not in name.split("/")
                        for name in names), "Unsafe archive name")
            blobs = {item.name: archive.extractfile(item).read() for item in members}
        contract_raw = blobs["contract.json"]
        contract = loads(contract_raw)
        require(contract["implementation_scope"] == "transport-control-only" and
                contract["implemented_methods"] == ["initialize", "shutdown"] and
                contract["advertised_capabilities"] == ["notifications"], "Unexpected contract scope")
        require(set(blobs) == {"contract.json", "LICENSE", *(item["path"] for item in contract["files"])},
                "Archive is not the exact self-contained contract")
        for item in contract["files"]:
            raw = blobs[item["path"]]
            require(len(raw) == item["bytes"] and hashlib.sha256(raw).hexdigest() == item["sha256"],
                    f"Archive declared member mismatch: {item['path']}")
        require(blobs["LICENSE"] == (ROOT / "LICENSE").read_bytes(), "Archive license mismatch")
        self.contract_digest = hashlib.sha256(contract_raw).hexdigest()
        self.methods = loads(blobs["methods.json"])
        require(set(self.methods) == {"initialize", "shutdown"}, "Unexpected method schemas")
        vectors = loads(blobs["vectors.json"])
        require(vectors["schema_version"] == 1 and vectors["profile"] == "writfold.agent/1"
                and vectors["cases"], "Missing contract vectors")
        self.report["contract_digest"] = self.contract_digest
        return vectors["cases"]

    def build(self):
        output = self.commands.run("build-writfold", ["cargo", "build", "--offline", "--locked",
            "--release", "--target", TARGET, "-p", "writfold", "--bin", "writfold",
            "--message-format=json"], timeout=900)
        messages = [loads(line) for line in output.read_bytes().splitlines()]
        require(any(m.get("reason") == "build-finished" and m.get("success") is True
                    for m in messages), "Cargo did not report a successful completed build")
        artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                     and m.get("target", {}).get("name") == "writfold"
                     and m["target"].get("kind") == ["bin"] and m.get("executable")]
        require(len(artifacts) == 1, "Cargo executable artifact is absent or ambiguous")
        artifact = artifacts[0]
        self.report["compiler_artifact"] = artifact
        source = Path(artifact["executable"])
        require(source.is_absolute() and source.resolve().is_relative_to(self.cache),
                "Cargo artifact is outside the admitted target directory")
        self.binary = self.output / "writfold"
        self.report["binary"] = retain_binary(source, self.binary)
        raw = self.binary.read_bytes()
        require(len(raw) >= 64 and raw[:7] == b"\x7fELF\x02\x01\x01", "Not ELF64 little-endian")
        kind, machine = struct.unpack_from("<HH", raw, 16)
        require(kind in (2, 3) and machine == 62, "Not an x86_64 executable ELF")
        offset = struct.unpack_from("<Q", raw, 32)[0]
        size, count = struct.unpack_from("<HH", raw, 54)
        require(size == 56 and count > 0 and offset + size * count <= len(raw),
                "Invalid ELF program-header table")
        headers = [struct.unpack_from("<IIQQQQQQ", raw, offset + size * i) for i in range(count)]
        require(any(h[0] == 1 for h in headers) and all(h[0] != 3 for h in headers),
                "ELF has an interpreter or no load segment")
        # Static PIE may carry relocation metadata, but never DT_NEEDED dependencies.
        for header in headers:
            require(header[2] + header[5] <= len(raw), "ELF segment extends beyond artifact")
            if header[0] == 2:
                require(header[5] % 16 == 0, "Malformed ELF dynamic segment")
                tags = [struct.unpack_from("<qQ", raw, pos)[0]
                        for pos in range(header[2], header[2] + header[5], 16)]
                require(0 in tags and 1 not in tags[:tags.index(0)],
                        "ELF requires shared libraries or has unterminated dynamic metadata")
        for name, args in (("file", ["-L"]), ("readelf", ["-h", "-l", "-d"])):
            self.commands.run(f"inspect-{name}", [self.report["tools"][name]["path"], *args, str(self.binary)])
        return True

    def validate(self, raw, expectations):
        require(not raw or raw.endswith(b"\n"), "Partial output frame")
        lines = raw.splitlines(keepends=True)
        require(len(lines) == len(expectations), "Missing or unsolicited response")
        session, previous = None, 0
        messages = {-32700: "Parse error", -32600: "Invalid Request", -32602: "Invalid params",
                    -32601: "Method not found", -32000: "Operation failed", -32002: "Initialization required"}
        for line, want in zip(lines, expectations):
            require(len(line) <= 65536 and line.endswith(b"\n"), "Output frame budget/framing")
            value = loads(line.decode("utf-8"))
            require(type(value) is dict and value.get("jsonrpc") == "2.0", "Invalid response envelope")
            require("id" in value and type(value["id"]) is type(want["id"])
                    and value["id"] == want["id"], "Response ID kind/value changed")
            if "code" in want:
                require(set(value) == {"jsonrpc", "id", "error"}, "Invalid error envelope")
                error = value["error"]
                require(set(error) == {"code", "message", "data"} and error["code"] == want["code"],
                        "Wrong fault code/shape")
                outer = want.get("message", messages[want["code"]])
                require(error["message"] == outer, "Wrong fault message")
                data = error["data"]
                require(data["schema_version"] == 1 and data["category"] == want["category"]
                        and data["exit_code"] == 1 and data["complete"] is False
                        and data["outcome"] == "rejected", "Wrong fault semantics")
                require(set(data) == {"schema_version", "category", "message", "diagnostic",
                        "exit_code", "complete", "generation", "session", "outcome"},
                        "Unexpected fault data fields")
                detail = {"Initialization required": "initialize the protocol before invoking methods"}.get(outer, outer)
                if want["code"] == -32000:
                    detail = ("engine is already initialized" if want["category"] == "conflict"
                              else "unsupported requested capability")
                require(data["message"] == detail, "Fault leaks or changes detail")
                diagnostic = data["diagnostic"]
                require(diagnostic == dict(code="E102" if want["category"] == "malformed" else "E104",
                        severity="error", message=detail, labels=[], help=None), "Wrong diagnostic")
            else:
                require(set(value) == {"jsonrpc", "id", "result"}, "Invalid success envelope")
                data = value["result"]
                if want["kind"] == "initialize":
                    require(set(data) == {"protocol_version", "profile", "implementation_scope",
                            "server", "session", "generation", "capabilities", "batch", "notifications",
                            "max_request_bytes", "max_response_bytes", "methods", "contract",
                            "trust_boundary"}, "Unexpected initialization fields")
                    require(data["protocol_version"] == "2" and data["profile"] == "writfold.agent/1"
                            and data["implementation_scope"] == "transport-control-only", "Wrong initialized scope")
                    require(data["capabilities"] == ["notifications"] and data["methods"] == self.methods
                            and data["batch"] is False and data["notifications"] is True,
                            "Fabricated capability or method")
                    require(data["max_request_bytes"] == data["max_response_bytes"] == 65536,
                            "Wrong framing budgets")
                    require(data["contract"] == dict(namespace="writfold-agent-contract", schema=1,
                            sha256=self.contract_digest), "Running binary/archive contract mismatch")
                    require(data["server"]["name"] == "writfold" and isinstance(data["server"]["version"], str)
                            and data["server"]["version"], "Missing server identity")
                    require(set(data["server"]) == {"name", "version"}, "Unexpected server fields")
                    require(data["trust_boundary"] == "Transport/session control only; no authorized corpus, retained release, grounding or effect service is available.", "Wrong trust boundary")
                else:
                    require(set(data) == {"exit_code", "complete", "generation", "session", "payload"},
                            "Unexpected shutdown fields")
                    require(want["kind"] == "shutdown" and data["exit_code"] == 0
                            and data["complete"] is True and data["payload"] == {"shutdown": True},
                            "Wrong shutdown result")
            generation = data["generation"]
            require(type(generation) is type(want["generation"]) and generation == want["generation"],
                    "Wrong generation allocation")
            if generation is not None:
                require(type(generation) is int and previous < generation <= 2**64 - 1,
                        "Nonmonotonic generation")
                previous = generation
            labels = data["session"]
            require(set(labels) == {"instance_id", "incarnation"} and
                    all(isinstance(v, str) and v for v in labels.values()), "Missing session labels")
            if session is None:
                session = labels
            require(session == labels, "Session changed within an invocation")

    def case(self, name, actions, wants, exit_code=0, stderr=None):
        write_json(self.output / f"{name}.expected.json", dict(responses=wants, exit_code=exit_code,
                   stderr=stderr.decode("utf-8") if stderr is not None else None, scope=SCOPE))
        raw, err, status = self.pipes.run(name, [str(self.binary), "serve"], actions,
            cwd=self.output, env={"PATH": "/nonexistent-writfold-tools", "HOME": "/nonexistent-writfold-home",
                                  "LANG": "C", "LC_ALL": "C"},
            ready=lambda data: self.validate(data, [expected("ready", "initialize", 1)]))
        require(status == exit_code, f"Unexpected exit {status}, expected {exit_code}")
        if stderr is None:
            require(not err, "Unexpected stderr")
        else:
            require(stderr in err and err.startswith(b"writfold: "), "Missing expected fatal diagnostic")
        self.validate(raw, wants)
        self.report["cases"].append(dict(name=name, responses=len(wants), exit_code=status))

    def boundaries(self):
        init, stop = frame("initialize", "ready"), frame("shutdown", "stop")
        hello, bye = expected("ready", "initialize", 1), expected("stop", "shutdown", 2)
        def ordinary(data):
            return [("write", data), ("eof", None)]
        def padded(size):
            return init[:-1] + b" " * (size - len(init)) + b"\n"
        oversized = fault(-32600)
        oversized["message"] = "Request frame exceeds 65536 bytes"
        cases = {
            "lf-inclusive-65536": (ordinary(padded(65536) + stop), [hello, bye], 0, None),
            "lf-inclusive-65537": (ordinary(padded(65537) + init + stop), [oversized, hello, bye], 0, None),
            "oversize-drain-recovery": (ordinary(b"x" * 262144 + b"\n" + init + stop), [oversized, hello, bye], 0, None),
            "oversize-eof": (ordinary(b"x" * 65537), [oversized], 0, None),
            "fragmented-unterminated-eof": ([("write", init), ("ready", 1),
                ("write", b'{"jsonrpc":"2.0",'), ("write", b'"method":"shutdown",'),
                ("write", b'"id":"stop"}'), ("eof", None)], [hello, bye], 0, None),
            "malformed-and-invalid-utf8": (ordinary(b'{\n\xff\n' + init + stop),
                [fault(-32700), fault(-32700), hello, bye], 0, None),
            "shutdown-stops-reading": ([("write", init), ("ready", 1),
                ("write", stop + b'{"jsonrpc":"2.0","method":"unknown","id":"unread"}\n')],
                [hello, bye], 0, None),
            "sigterm-partial-frame": ([("write", init), ("ready", 1),
                ("write", b'{"jsonrpc":"2.0","method":'), ("signal", signal.SIGTERM)],
                [hello], -signal.SIGTERM, None),
            "broken-stdout": ([("write", init), ("ready", 1), ("close-output", None),
                ("write", stop), ("eof", None)], [hello], 130, b"Broken pipe"),
        }
        escaped = 'quote" slash\\ newline\n tab\t snowman\u2603'
        big = 900719925474099312345678901234567890
        cases["escaped-and-large-integer-ids"] = (ordinary(frame("initialize", escaped)
            + frame("unknown", big) + frame("unknown", -big) + frame("shutdown", "stop")),
            [expected(escaped, "initialize", 1), fault(-32601, 2, big), fault(-32601, 3, -big),
             expected("stop", "shutdown", 4)], 0, None)
        # A near-budget valid string ID fits input but cannot fit initialization metadata.
        huge = frame("initialize", "x" * 65000)
        require(len(huge) <= 65536, "Huge-ID fixture is not an admitted request")
        cases["huge-id-output-budget"] = (ordinary(huge), [], 1,
                                           b"response exceeds the LF-inclusive output budget")
        for name in BOUNDARIES:
            actions, wants, status, err = cases[name]
            self.check(name, lambda n=name, a=actions, w=wants, s=status, e=err:
                       self.case(n, a, w, s, e))

    def integrity(self):
        require(self.baseline is not None and self.sources() == self.baseline, "Source inputs changed")
        for entry in self.pipes.records:
            require(not entry.get("error") and not entry["timed_out"], "Failed or unobserved command")
            for artifact in entry["artifacts"].values():
                require(record(Path(artifact["path"])) == artifact, "Process artifact changed")
        require(self.binary is not None and sha(self.binary) == self.report["binary"]["sha256"],
                "Retained executable changed")
        require(self.archive is not None and record(Path(self.archive["path"])) == self.archive,
                "Contract archive changed")
        for name, tool in self.report.get("tools", {}).items():
            require(sha(Path(tool["path"])) == tool["sha256"], f"Observed tool changed: {name}")

    def run(self):
        bound = self.check("source-tool-binding", self.bind)
        vectors = self.check("contract-archive", self.package)
        built = self.check("static-musl-build", self.build) if bound else None
        if vectors:
            for vector in vectors:
                name = "vector-" + vector["name"]
                require(all(c.isalnum() or c == "-" for c in name), "Unsafe vector name")
                self.declare(name)
                if built:
                    data = "\n".join(vector["frames"]).encode()
                    if vector.get("final_lf", True):
                        data += b"\n"
                    self.check(name, lambda n=name, d=data, v=vector:
                        self.case(n, [("write", d), ("eof", None)], v["expected"], v["exit_code"]))
            self.check("contract-vectors", lambda: require(all(
                self.checks["vector-" + v["name"]]["status"] == "pass" for v in vectors),
                "Not every contract sequence passed"))
        if built and vectors:
            self.boundaries()
        self.check("evidence-integrity", self.integrity)

    def finish(self):
        checks = list(self.checks.values())
        passed = all(check["status"] == "pass" for check in checks)
        self.report.update(result="pass" if passed else "fail", checks=checks,
            failed_obligations=sum(check["status"] != "pass" for check in checks),
            artifacts=[record(path) for path in sorted(self.output.iterdir())
                       if path.is_file() and path not in self.preexisting])
        report_path = self.output / "stdio-report.json"
        write_json(report_path, self.report)
        readable = self.output / "stdio-report.txt"
        readable.write_text("\n".join([f"Stdio control qualification: {self.report['result']}", SCOPE,
            *[f"{item['status']}: {item['name']} {'; '.join(item['errors'])}" for item in checks]]) + "\n")
        write_json(self.output / "obligations.json", dict(checks=checks, skips=[], scope=SCOPE,
                   report=record(report_path), readable_report=record(readable)))
        return 0 if passed else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=Path("/evidence"))
    parser.add_argument("--cache", type=Path, default=Path("/cache/target"))
    args = parser.parse_args()
    output, cache = args.evidence.resolve(), args.cache.resolve()
    if output != Path("/evidence") or cache != Path("/cache/target"):
        parser.error("Use the admitted Compose /evidence and /cache/target mounts")
    if output.is_relative_to(ROOT) or cache.is_relative_to(ROOT):
        parser.error("Evidence and target must be outside the source checkout")
    output.mkdir(parents=True, exist_ok=True)
    if (output / "stdio-attempt.json").exists() or (output / "obligations.json").exists():
        parser.error("Retain this attempt and use a fresh evidence directory")
    producer = Producer(output, cache)
    try:
        producer.run()
    except (Exception, KeyboardInterrupt) as exc:
        producer.report["errors"].append(dict(producer=f"{type(exc).__name__}: {exc}"))
        producer.checks["evidence-integrity"].update(status="fail", errors=["Producer interrupted or crashed"])
        (output / "stdio-producer.failure.log").write_text(traceback.format_exc())
    return producer.finish()


if __name__ == "__main__":
    raise SystemExit(main())
