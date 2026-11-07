"""Real concurrent Writfold sessions and independently launched Weft clients.

Direct servers rendezvous after validated initialization while stdin stays open;
shutdown is withheld until both are ready. Weft clients use a separate launch
barrier: their private frames and overlapping child lifetimes are not invented.
"""
from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import threading
import tarfile
import time

from writfold_conformance_process import write_json
from writfold_evidence import document, file_record
from writfold_stdio_process import Pipes


def require(condition, message):
    if not condition:
        raise ValueError(message)


def exact(value, expected, description):
    require(type(value) is type(expected) and value == expected,
            f"{description}: expected {expected!r}, observed {value!r}")
    # Python container equality otherwise admits True as 1 inside wire objects.
    if isinstance(expected, dict):
        for key, item in expected.items():
            exact(value[key], item, f"{description}.{key}")
    elif isinstance(expected, list):
        for index, item in enumerate(expected):
            exact(value[index], item, f"{description}[{index}]")


def loads(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, f"Duplicate response key: {key}")
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def frame(method, ident):
    value = dict(jsonrpc="2.0", method=method, id=ident)
    if method == "initialize":
        value["params"] = dict(protocol_version="2")
    return json.dumps(value, ensure_ascii=True, separators=(",", ":")).encode() + b"\n"


def responses(raw, identifiers):
    # The same LF-inclusive frame and strict envelope checks as writfold_stdio.
    lines = raw.splitlines(keepends=True)
    require(len(lines) == len(identifiers), "Missing or unsolicited direct-server response")
    values = []
    for line, ident in zip(lines, identifiers):
        require(line.endswith(b"\n") and len(line) <= 65536, "Invalid response framing/budget")
        value = loads(line.decode("utf-8"))
        require(type(value) is dict and set(value) == {"jsonrpc", "id", "result"},
                "Invalid direct-server success envelope")
        exact(value["jsonrpc"], "2.0", "Response JSON-RPC version")
        exact(value["id"], ident, "Response correlation identity")
        require(type(value["result"]) is dict, "Response result must be an object")
        values.append(value["result"])
    return values


def labels(session):
    require(type(session) is dict and set(session) == {"instance_id", "incarnation"},
            "Missing or additional session identity fields")
    for key, value in session.items():
        require(isinstance(value, str) and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]*", value),
                f"Invalid ephemeral transport {key}")


def initialize(data, contract_digest, methods, trust_boundary):
    require(set(data) == {"protocol_version", "profile", "implementation_scope", "server",
            "session", "generation", "capabilities", "batch", "notifications",
            "max_request_bytes", "max_response_bytes", "methods", "contract", "trust_boundary"},
            "Unexpected initialization fields")
    for key, expected in dict(protocol_version="2", profile="writfold.agent/1",
            implementation_scope="transport-control-only", generation=1,
            capabilities=["notifications"], batch=False, notifications=True,
            max_request_bytes=65536, max_response_bytes=65536,
            methods=methods, trust_boundary=trust_boundary).items():
        exact(data[key], expected, f"Direct initialization {key}")
    exact(data["contract"], dict(namespace="writfold-agent-contract", schema=1,
          sha256=contract_digest), "Direct-server shared contract identity")
    require(type(data["server"]) is dict and set(data["server"]) == {"name", "version"},
            "Unexpected direct-server identity fields")
    exact(data["server"]["name"], "writfold", "Direct-server name")
    require(isinstance(data["server"]["version"], str) and data["server"]["version"],
            "Missing direct-server version")
    labels(data["session"])


def shutdown(data, session):
    require(set(data) == {"exit_code", "complete", "generation", "session", "payload"},
            "Unexpected shutdown fields")
    for key, expected in dict(exit_code=0, complete=True, generation=2,
            session=session, payload={"shutdown": True}).items():
        exact(data[key], expected, f"Direct shutdown {key}")


def contract_inputs(suite):
    with tarfile.open(suite.contract_archive, "r:gz") as archive:
        raw = archive.extractfile("contract.json").read()
        exact(hashlib.sha256(raw).hexdigest(), suite.contract_digest, "Retained contract pin")
        contract = loads(raw)
        exact(contract["implemented_methods"], ["initialize", "shutdown"], "Implemented methods")
        payloads = {}
        for name in ("methods.json", "response.schema.json"):
            data = archive.extractfile(name).read()
            descriptor = next(item for item in contract["files"] if item["path"] == name)
            exact(hashlib.sha256(data).hexdigest(), descriptor["sha256"], f"Contract payload {name}")
            exact(len(data), descriptor["bytes"], f"Contract payload size {name}")
            payloads[name] = loads(data)
    boundary = payloads["response.schema.json"]["oneOf"][0]["properties"]["result"]["oneOf"][0]["properties"]["trust_boundary"]["const"]
    return payloads["methods.json"], boundary


def validate_control(suite, entry, correct):
    command = entry["command"]
    exact(command["exit_code"], 0 if correct else 2, "Weft control exit")
    require(not Path(command["artifacts"]["stderr"]["path"]).read_bytes(),
            "Unexpected control stderr (retained)")
    require(not command["timed_out"] and "error" not in command,
            "Control command did not complete within its owned process budget")
    result = document(Path(command["artifacts"]["stdout"]["path"]))
    require(isinstance(result, dict), "Control report must be an object")
    entry["observed"] = result
    for key, expected in dict(schema="weft.control/1", kind="control-session",
            scope="writfold.agent/1 transport-control-only",
            execution="complete" if correct else "failed",
            writfold_grounded=False, effect_authority=False).items():
        exact(result[key], expected, f"Control {key}")
    exact(result["peer"], dict(command=str(suite.binaries["writfold"]),
          sha256=suite.args.expect_writfold_sha256 if correct else "unverified"), "Control peer")
    require(isinstance(result["boundary"], str) and result["boundary"],
            "Control authority boundary is absent")
    if correct:
        require("diagnostics" not in result, "Completed control unexpectedly has diagnostics")
        session = result["session"]
        exact(session["contract_sha256"], suite.contract_digest, "Shared contract SHA-256")
        exact(session["initialize_generation"], 1, "Fresh invocation generation")
        exact(session["shutdown_generation"], 2, "Shutdown invocation generation")
        for key in ("instance_id", "incarnation"):
            require(isinstance(session[key], str) and
                    re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]*", session[key]),
                    f"Invalid ephemeral transport {key}")
        require(isinstance(session["server_version"], str) and session["server_version"],
                "Control server version is absent")
    else:
        require("session" not in result, "Wrong-pin client invented a session")
        diagnostics = result["diagnostics"]
        require(isinstance(diagnostics, list) and len(diagnostics) == 1,
                "Wrong-pin refusal must have one structured diagnostic")
        exact(diagnostics[0]["code"], "ControlFailure", "Wrong-pin refusal code")
        exact(diagnostics[0]["phase"], "peer", "Wrong-pin refusal phase")
        require(isinstance(diagnostics[0]["message"], str) and diagnostics[0]["message"],
                "Wrong-pin structured diagnostic is absent")
    return result


def run(suite, case):
    """Overlap two actual servers; independently launch Weft clients and reconnect."""
    exact(case["action"], "concurrent-control", "Session action")
    require(re.fullmatch(r"[a-z0-9-]+", case["id"]), "Unsafe session case ID")
    require(suite.args.timeout > 0, "Concurrent control requires a finite positive timeout")
    binaries = {name: file_record(suite.binaries[name]) for name in ("weft", "writfold")}
    for name, identity in binaries.items():
        exact(identity, suite.report["binaries"][name], f"Pinned {name} bytes")
    exact(binaries["writfold"]["sha256"], suite.args.expect_writfold_sha256,
          "Expected Writfold SHA-256")
    require(re.fullmatch(r"[0-9a-f]{64}", suite.contract_digest), "Invalid shared contract pin")
    methods, trust_boundary = contract_inputs(suite)
    original = file_record(suite.paths["original"])
    exact(original, suite.original_identity, "Unmodified original source")
    native_inputs = [item["original"] for item in suite.inputs]
    for identity in native_inputs:
        exact(file_record(Path(identity["path"])), identity, "Unmodified native input")

    root = suite.work / case["id"]
    root.mkdir()
    evidence_path = suite.attempt.directory / f"{case['id']}-sessions.json"
    names = ("valid-a", "valid-b", "wrong-pin")
    server_names = ("server-a", "server-b")
    workers = {}
    for name in (*server_names, *names):
        directory = root / name
        directory.mkdir()
        home, cwd, temporary = (directory / part for part in ("home", "cwd", "tmp"))
        for path in (home, cwd, temporary):
            path.mkdir()
        captures = suite.attempt.directory / f"{case['id']}-{name}"
        captures.mkdir()
        workers[name] = dict(pipes=Pipes(captures), cwd=cwd,
            env=dict(suite.env, HOME=str(home), TMPDIR=str(temporary)))
    require(len({worker["env"]["HOME"] for worker in workers.values()}) == len(workers) and
            len({str(worker["cwd"]) for worker in workers.values()}) == len(workers),
            "Concurrent workers do not have isolated HOME/cwd")
    wrong_pin = ("0" if suite.args.expect_writfold_sha256[0] != "0" else "1") + suite.args.expect_writfold_sha256[1:]
    observations = dict(schema="writfold-concurrent-control/1", action=case["action"],
        binaries=binaries, contract_sha256=suite.contract_digest, clients={}, servers={},
        max_concurrent_outer_processes=3, wrong_pin=wrong_pin,
        scope="Ephemeral initialize/shutdown control sessions only; no persistent resume.",
        weft_lifetime_overlap_claimed=False, weft_child_overlap_claimed=False,
        writfold_grounded=False, effect_authority=False, errors=[])

    def observe_overlap():
        entries = [observations["servers"][name] for name in server_names]
        for key in ("instance_id", "incarnation"):
            require(len({entry["initialize_response"]["session"][key] for entry in entries}) == 2,
                    f"Concurrent direct servers reused {key}")
        observations["overlap"] = dict(
            initialized_monotonic_ns={name: observations["servers"][name]["initialized_monotonic_ns"]
                                      for name in server_names},
            barrier_release_monotonic_ns=time.monotonic_ns(),
            actual_direct_servers=2, confirmed_by_shutdown=False,
            scope="Both real Writfold servers initialized before either receives shutdown. "
                  "Successful session-continuous shutdown confirms their common live interval. "
                  "This is not an observation of Weft-owned child overlap.")

    rendezvous = threading.Barrier(2, action=observe_overlap)

    def direct_server(name):
        worker = workers[name]
        entry = dict(argv=[str(suite.binaries["writfold"]), "serve"],
                     cwd=str(worker["cwd"]), environment=worker["env"])
        observations["servers"][name] = entry
        init_id, stop_id = f"{name}:initialize", f"{name}:shutdown"
        deadline = time.monotonic() + suite.args.timeout

        def ready(raw):
            data = responses(raw, [init_id])[0]
            initialize(data, suite.contract_digest, methods, trust_boundary)
            entry["initialize_response"] = data
            entry["initialized_monotonic_ns"] = time.monotonic_ns()
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"{name}: direct-server rendezvous deadline")
            rendezvous.wait(timeout=remaining)
            entry["shutdown_release_monotonic_ns"] = time.monotonic_ns()

        try:
            raw, stderr, code = worker["pipes"].run(f"{case['id']}-{name}", entry["argv"],
                [("write", frame("initialize", init_id)), ("ready", 1),
                 ("write", frame("shutdown", stop_id)), ("eof", None)],
                cwd=worker["cwd"], env=worker["env"], timeout=suite.args.timeout,
                limit=2 * 65536, ready=ready)
            exact(code, 0, "Direct Writfold exit")
            require(not stderr, "Unexpected direct Writfold stderr (retained)")
            hello, bye = responses(raw, [init_id, stop_id])
            exact(hello, entry["initialize_response"], "Retained initialization response")
            shutdown(bye, hello["session"])
            entry["shutdown_response"] = bye
            return hello["session"]
        except BaseException as error:
            rendezvous.abort()
            entry["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            if worker["pipes"].records:
                entry["command"] = worker["pipes"].records[-1]

    def launch_clients():
        observations["weft_launch"] = dict(clients=list(names),
            barrier_release_monotonic_ns=time.monotonic_ns(),
            scope="Synchronized launch attempts only; overlapping Weft/child lifetimes are not asserted.")

    launch = threading.Barrier(len(names), action=launch_clients)

    def execute(name, worker_name, correct, concurrent):
        worker = workers[worker_name]
        argv = [str(suite.binaries["weft"]), "control", str(suite.binaries["writfold"]),
                "--expect-sha256", suite.args.expect_writfold_sha256 if correct else wrong_pin,
                "--json"]
        entry = dict(argv=argv, cwd=str(worker["cwd"]), environment=worker["env"],
                     pin="correct" if correct else "incorrect")
        observations["clients"][name] = entry
        try:
            if concurrent:
                entry["launch_arrival_monotonic_ns"] = time.monotonic_ns()
                launch.wait(timeout=suite.args.timeout)
                entry["launch_ready_monotonic_ns"] = time.monotonic_ns()
            worker["pipes"].run(f"{case['id']}-{name}", argv, [("eof", None)],
                cwd=worker["cwd"], env=worker["env"], timeout=suite.args.timeout,
                limit=2 * 1024 * 1024)
            entry["command"] = worker["pipes"].records[-1]
            return validate_control(suite, entry, correct)
        except BaseException as error:
            launch.abort()
            entry["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            if worker["pipes"].records:
                entry["command"] = worker["pipes"].records[-1]

    def settle(calls):
        results = {}
        with ThreadPoolExecutor(max_workers=len(calls), thread_name_prefix="control-smoke") as pool:
            futures = {name: pool.submit(function, *args) for name, function, args in calls}
            for name, future in futures.items():
                try:
                    results[name] = future.result()
                except Exception as error:
                    observations["errors"].append(f"{name}: {type(error).__name__}: {error}")
        require(not observations["errors"], "; ".join(observations["errors"]))
        return results

    try:
        direct = settle([(name, direct_server, (name,)) for name in server_names])
        overlap = observations["overlap"]
        release = overlap["barrier_release_monotonic_ns"]
        require(max(overlap["initialized_monotonic_ns"].values()) <= release and
                all(release <= observations["servers"][name]["shutdown_release_monotonic_ns"]
                    for name in server_names),
                "A direct server was released before both initializations were validated")
        overlap["earliest_shutdown_release_monotonic_ns"] = min(
            observations["servers"][name]["shutdown_release_monotonic_ns"] for name in server_names)
        overlap["confirmed_by_shutdown"] = True
        results = settle([(name, execute, (name, name, name != "wrong-pin", True)) for name in names])
        # Reuse only the refused client's private HOME/cwd, not a server, session,
        # ledger or retained owner state. A new process must start generations anew.
        results["reconnect"] = execute("reconnect", "wrong-pin", True, False)
        positives = [*direct.values(),
                     *(results[name]["session"] for name in ("valid-a", "valid-b", "reconnect"))]
        for key in ("instance_id", "incarnation"):
            require(len({session[key] for session in positives}) == len(positives),
                    f"Fresh real control invocations reused ephemeral {key}")
        observations["fresh_session"] = dict(client="reconnect", after="wrong-pin",
            reused_home_and_cwd=True, generation_reset=[1, 2],
            distinct_instance_ids=[session["instance_id"] for session in positives],
            distinct_incarnations=[session["incarnation"] for session in positives],
            persistent_resume=False)
        observations["status"] = "PASS"
    except BaseException as error:
        observations["status"] = "FAIL"
        observations["errors"].append(f"{type(error).__name__}: {error}")
        raise
    finally:
        # ThreadPoolExecutor has settled every concurrent worker before records
        # enter the shared runner list, including timeout/refusal/failure paths.
        suite.pipes.records.extend(sorted(
            (record for worker in workers.values() for record in worker["pipes"].records),
            key=lambda record: (record["start_ns"], record["name"])))
        try:
            exact(file_record(suite.paths["original"]), original, "Unchanged source bytes")
            for name, identity in binaries.items():
                exact(file_record(suite.binaries[name]), identity, f"Unchanged {name} bytes")
            for identity in native_inputs:
                exact(file_record(Path(identity["path"])), identity, "Unchanged native input")
            observations["original_unchanged"] = True
            observations["native_inputs_unchanged"] = True
            observations["binaries_unchanged"] = True
        except BaseException as error:
            observations["status"] = "FAIL"
            observations["errors"].append(f"Integrity: {type(error).__name__}: {error}")
            raise
        finally:
            write_json(evidence_path, observations)
    return dict(evidence=file_record(evidence_path), **observations)
