#!/usr/bin/env python3
"""Publish and inspect the standalone agent contract (stdlib only): transport control
plus the negotiated writfold.trial/1 capability.

export(Path) exclusively creates a deterministic gzip/USTAR artifact; inspect(Path)
reads an artifact without consulting the checkout or extracting any member. Both
return JSON-compatible metadata: path, sha256 (archive), bytes, contract_sha256,
namespace, schema, profile, implementation_scope and ordered members, each with
path/sha256/bytes. Digests establish integrity, not publisher authenticity. Invalid
input raises ContractError; filesystem errors remain explicit OSError exceptions.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import stat
import sys
import tarfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates/writfold-wire/contracts/agent-stdio/1"
PAYLOADS = frozenset({"methods.json", "trial-methods.json", "request.schema.json",
                      "response.schema.json", "citation-binding.schema.json", "vectors.json"})
CONTROL_METHODS = ["initialize", "shutdown"]
TRIAL_METHODS = ["trial/show", "trial/retain", "trial/prepare", "trial/run",
                 "trial/evidence-list", "trial/evidence-show"]
TRIAL_SCOPE = "transport-control+trial-worktree"
ROOT_KEYS = frozenset({"namespace", "schema", "profile", "implementation_scope",
                       "command", "protocol", "implemented_methods",
                       "advertised_capabilities", "capabilities", "identity", "lifecycle",
                       "validation", "blocked_requirements", "files"})
# Fixed tooling admission bounds for this small data-only contract artifact.
# These are not protocol frame limits or operator storage admission floors.
MAX_ARCHIVE_BYTES = 1024 * 1024
MAX_MEMBER_BYTES = 1024 * 1024
MAX_TAR_BYTES = 4 * 1024 * 1024


class ContractError(ValueError):
    """Contract bytes, scope, membership or archive representation are invalid."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def _pairs(pairs: list) -> dict:
    result = {}
    for key, value in pairs:
        _require(key not in result, f"duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _nonfinite(token: str) -> None:
    raise ContractError(f"nonfinite JSON number: {token}")


def _float(token: str) -> float:
    value = float(token)
    _require(math.isfinite(value), f"nonfinite JSON number: {token}")
    return value


def _json(data: bytes, name: str) -> dict:
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_pairs,
                           parse_constant=_nonfinite, parse_float=_float)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise ContractError(f"{name}: invalid strict UTF-8 JSON: {error}") from error
    _require(isinstance(value, dict), f"{name}: expected JSON object")
    return value


def _safe(name: object) -> bool:
    return isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", name) is not None


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _declarations(raw: bytes) -> tuple[dict, dict]:
    contract = _json(raw, "contract.json")
    _require(set(contract) == ROOT_KEYS, "contract.json: root field set is not closed schema 1")
    for key, expected in (("namespace", "writfold-agent-contract"), ("schema", 1),
                          ("profile", "writfold.agent/1"),
                          ("implementation_scope", "transport-control-only"),
                          ("command", ["writfold", "serve"]),
                          ("implemented_methods", CONTROL_METHODS + TRIAL_METHODS),
                          ("advertised_capabilities", ["notifications", "writfold.trial/1"])):
        _require(type(contract[key]) is type(expected) and contract[key] == expected,
                 f"contract.json: unsupported {key}")
    capabilities = contract["capabilities"]
    _require(isinstance(capabilities, dict) and set(capabilities) == {"writfold.trial/1"},
             "contract.json: unsupported negotiated capability set")
    trial = capabilities["writfold.trial/1"]
    _require(isinstance(trial, dict) and trial.get("implementation_scope") == TRIAL_SCOPE
             and trial.get("methods") == TRIAL_METHODS
             and trial.get("cli_only", {}).get("methods") == ["trial/approve", "trial/apply", "trial/recover"],
             "contract.json: writfold.trial/1 scope, methods or CLI-only set changed")
    _require(isinstance(contract["files"], list), "contract.json: files must be an array")
    declarations = {}
    for entry in contract["files"]:
        _require(isinstance(entry, dict) and set(entry) == {"path", "sha256", "bytes"},
                 "contract.json: file declaration must contain only path/sha256/bytes")
        name = entry["path"]
        _require(_safe(name), f"contract.json: unsafe flat member name {name!r}")
        _require(name not in declarations, f"contract.json: duplicate member {name}")
        _require(isinstance(entry["sha256"], str) and
                 re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]) is not None,
                 f"{name}: invalid SHA256 declaration")
        _require(type(entry["bytes"]) is int and entry["bytes"] >= 0,
                 f"{name}: invalid byte length declaration")
        declarations[name] = entry
    _require(set(declarations) == PAYLOADS, "contract.json: payload membership differs from schema 1")
    return contract, declarations


def _validate(members: dict[str, bytes]) -> dict:
    _require("contract.json" in members, "missing contract.json")
    contract, declarations = _declarations(members["contract.json"])
    _require(set(members) == {"contract.json", "LICENSE"} | set(declarations),
             "bundle contains missing or unlisted members")
    _require(all(len(data) <= MAX_MEMBER_BYTES for data in members.values()),
             "contract member exceeds artifact tooling budget")
    _require(sum(map(len, members.values())) <= MAX_TAR_BYTES,
             "contract members exceed aggregate tooling budget")
    _require(bool(members["LICENSE"].strip()), "LICENSE must not be empty")
    parsed = {}
    for name, entry in declarations.items():
        data = members[name]
        _require(len(data) == entry["bytes"], f"{name}: declared byte length mismatch")
        _require(_sha(data) == entry["sha256"], f"{name}: declared SHA256 mismatch")
        parsed[name] = _json(data, name)
    methods = parsed["methods.json"]
    _require(set(methods) == {"initialize", "shutdown"}, "methods.json: non-control method scope")
    trial_methods = parsed["trial-methods.json"]
    _require(list(trial_methods) == CONTROL_METHODS + TRIAL_METHODS,
             "trial-methods.json: not exactly the control plus trial methods")
    _require(all(trial_methods[name] == methods[name] for name in CONTROL_METHODS),
             "trial-methods.json: control schemas differ from methods.json")
    _require(all(isinstance(trial_methods[name], dict) and trial_methods[name].get("type") == "object"
                 and trial_methods[name].get("additionalProperties") is False for name in TRIAL_METHODS),
             "trial-methods.json: trial parameter schemas are not closed objects")
    initialize = methods["initialize"]
    shutdown = methods["shutdown"]
    _require(isinstance(initialize, dict) and isinstance(shutdown, dict), "invalid method schemas")
    _require(initialize.get("type") == "object" and initialize.get("additionalProperties") is False
             and initialize.get("required") == ["protocol_version"], "initialize schema is not closed")
    properties = initialize.get("properties")
    _require(isinstance(properties, dict) and set(properties) == {"protocol_version", "capabilities"},
             "initialize schema has unsupported parameters")
    _require(isinstance(properties["protocol_version"], dict)
             and properties["protocol_version"].get("const") == "2", "unsupported protocol version")
    _require(shutdown.get("type") == "object" and shutdown.get("additionalProperties") is False
             and shutdown.get("properties") == {}, "shutdown schema is not empty and closed")
    # Validate the published initialization advertisement as well as the manifest.
    try:
        response = parsed["response.schema.json"]
        _require(len(response["oneOf"]) == 2, "response schema has extra envelope alternatives")
        results = response["oneOf"][0]["properties"]["result"]["oneOf"]
        # Control advertisement, shutdown, trial advertisement, trial result.
        _require(len(results) == 4, "response schema has extra success alternatives")
        for advertisement, scope, capabilities, names in (
                (results[0], contract["implementation_scope"], ["notifications"], CONTROL_METHODS),
                (results[2], TRIAL_SCOPE, ["notifications", "writfold.trial/1"],
                 CONTROL_METHODS + TRIAL_METHODS)):
            props = advertisement["properties"]
            _require(set(props) == {
                "protocol_version", "profile", "implementation_scope", "server", "session",
                "generation", "capabilities", "batch", "notifications", "max_request_bytes",
                "max_response_bytes", "methods", "contract", "trust_boundary",
            }, "response schema: unsupported initialization advertisement fields")
            _require(props["profile"] == {"const": contract["profile"]}, "response schema: unsupported profile")
            _require(props["implementation_scope"] == {"const": scope},
                     "response schema: unsupported implementation_scope")
            _require(props["capabilities"] == {"const": capabilities},
                     "response schema: capabilities differ from the negotiated set")
            method_schema = props["methods"]
            _require(set(method_schema["properties"]) == set(names)
                     and set(method_schema["required"]) == set(names)
                     and method_schema["additionalProperties"] is False,
                     "response schema: advertised methods differ from the negotiated set")
            _require(advertisement["additionalProperties"] is False,
                     "response schema: initialization advertisement must be closed")
    except (KeyError, TypeError, IndexError) as error:
        raise ContractError("response schema: missing control advertisement structure") from error
    return contract


def _read_regular(path: Path, maximum: int = MAX_MEMBER_BYTES) -> bytes:
    # O_NOFOLLOW closes the leaf-symlink race; count bytes as well as st_size so
    # concurrent file growth cannot bypass the bound after opening.
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        _require(stat.S_ISREG(info.st_mode), f"{path}: not a regular file")
        _require(info.st_size <= maximum, f"{path}: artifact byte budget exceeded")
        data = stream.read(maximum + 1)
        _require(len(data) <= maximum, f"{path}: artifact byte budget exceeded")
        return data


def _order(members: dict) -> list[str]:
    return ["contract.json", *sorted(PAYLOADS), "LICENSE"]


def _tar(members: dict[str, bytes]) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for name in _order(members):
            info = tarfile.TarInfo(name)
            info.size = len(members[name])
            info.mode = 0o644
            info.uid = info.gid = info.mtime = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(members[name]))
    result = buffer.getvalue()
    _require(len(result) <= MAX_TAR_BYTES, "canonical tar exceeds artifact tooling budget")
    return result


def _metadata(path: Path, raw: bytes, members: dict[str, bytes], contract: dict) -> dict:
    return {"path": str(path), "sha256": _sha(raw), "bytes": len(raw),
            "contract_sha256": _sha(members["contract.json"]),
            **{key: contract[key] for key in ("namespace", "schema", "profile", "implementation_scope")},
            "members": [{"path": name, "sha256": _sha(members[name]), "bytes": len(members[name])}
                        for name in _order(members)]}


def export(destination: Path) -> dict:
    """Create destination exclusively; never repair or re-pin declared source data.

    The parent directory must exist. Partial writes are retained on IO failure so
    failed attempts are not silently erased. No existing path is overwritten.
    """
    destination = Path(destination)
    source = SOURCE.resolve()
    _require(not SOURCE.is_symlink(), "contract source directory is a symlink")
    _require(not destination.resolve().is_relative_to(source), "output is inside contract source directory")
    _require(not os.path.lexists(destination), f"output already exists: {destination}")
    for component in (SOURCE, *SOURCE.parents):
        _require(not component.is_symlink(), f"source ancestor is a symlink: {component}")
    raw_contract = _read_regular(SOURCE / "contract.json")
    _, declarations = _declarations(raw_contract)
    _require({entry.name for entry in SOURCE.iterdir()} == {"contract.json"} | set(declarations),
             "contract source directory contains missing or unlisted members")
    members = {"contract.json": raw_contract}
    members.update({name: _read_regular(SOURCE / name) for name in declarations})
    members["LICENSE"] = _read_regular(ROOT / "LICENSE")
    contract = _validate(members)
    buffer = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=buffer, compresslevel=9, mtime=0) as compressed:
        compressed.write(_tar(members))
    raw = buffer.getvalue()
    _require(len(raw) <= MAX_ARCHIVE_BYTES, "compressed artifact exceeds tooling budget")
    with destination.open("xb") as output:
        output.write(raw)
    return _metadata(destination, raw, members, contract)


def inspect(path: Path) -> dict:
    """Read a standalone canonical owner bundle, without extraction or checkout IO.

    Require the publisher's fixed USTAR representation, including padding: this
    rejects hidden data after tar EOF, extended headers and noncanonical metadata.
    Exactly one complete gzip stream is admitted, without concatenated trailers.
    """
    path = Path(path)
    raw = _read_regular(path, MAX_ARCHIVE_BYTES)
    try:
        decoder = zlib.decompressobj(wbits=31)
        unpacked = decoder.decompress(raw, MAX_TAR_BYTES + 1)
        _require(len(unpacked) <= MAX_TAR_BYTES, "expanded artifact exceeds tooling budget")
        _require(decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail,
                 "archive must contain exactly one complete gzip stream")
    except zlib.error as error:
        raise ContractError(f"invalid gzip archive: {error}") from error
    members = {}
    try:
        with tarfile.open(fileobj=io.BytesIO(unpacked), mode="r:") as archive:
            for member in archive:
                _require(_safe(member.name), f"unsafe archive member: {member.name!r}")
                _require(member.name not in members, f"duplicate archive member: {member.name}")
                _require(member.isreg() and not member.pax_headers,
                         f"non-regular or extended archive member: {member.name}")
                _require(member.name in PAYLOADS | {"contract.json", "LICENSE"},
                         f"unlisted archive member: {member.name}")
                _require(0 <= member.size <= MAX_MEMBER_BYTES,
                         f"member exceeds artifact tooling budget: {member.name}")
                stream = archive.extractfile(member)
                _require(stream is not None, f"unreadable archive member: {member.name}")
                with stream:
                    members[member.name] = stream.read(MAX_MEMBER_BYTES + 1)
                _require(len(members[member.name]) == member.size, f"truncated member: {member.name}")
    except (tarfile.TarError, EOFError) as error:
        raise ContractError(f"invalid tar archive: {error}") from error
    contract = _validate(members)
    _require(unpacked == _tar(members), "archive is not canonical fixed-order USTAR or has trailing data")
    return _metadata(path, raw, members, contract)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument("--output", type=Path, help="exclusively create a deterministic contract .tar.gz")
    operation.add_argument("--inspect", type=Path, help="read and validate a standalone contract .tar.gz")
    arguments = parser.parse_args()
    try:
        metadata = export(arguments.output) if arguments.output is not None else inspect(arguments.inspect)
    except (ContractError, OSError) as error:
        print(f"contract artifact error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(metadata, sort_keys=True, indent=2, allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
