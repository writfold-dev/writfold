"""Exact-byte local artifact regressions; no grounding or effect-service claims.

Each command is a fresh retained process managed by the smoke's existing Pipes.
The reversible A/B/A scenario concerns source bytes, never persistent effect keys.
"""
from __future__ import annotations

from pathlib import Path
import re
from writfold_evidence import document, file_record, sha


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(suite, case):
    """Run one complete workflow or the pair of denied preview operations."""
    require(case["action"] in ("artifact-workflow", "preview-refusal"),
            "Unknown artifact action")
    root = suite.work / case["id"]
    root.mkdir()
    source = root / "artifacts-source.weft"
    original = suite.paths["artifacts_source"].read_bytes()
    source.write_bytes(original)
    source.chmod(0o444)
    watched = {path: file_record(path) for key, path in suite.paths.items()
               if key.startswith("artifacts_")}
    watched[source] = file_record(source)
    observations = []

    def command(step, tool, arguments, expected_code, diagnostic=None):
        for path, identity in watched.items():
            require(file_record(path) == identity, f"Input changed before {step}: {path}")
        stdout, stderr, code = suite.pipes.run(
            f"{case['id']}-{step}", [str(suite.binaries[tool]), *map(str, arguments)],
            [("eof", None)], cwd=root, env=suite.env, timeout=suite.args.timeout,
            limit=2 * 1024 * 1024)
        for path, identity in watched.items():
            require(file_record(path) == identity, f"Command modified input: {path}")
        require(code == expected_code, f"{step}: exit {code}, expected {expected_code}")
        if diagnostic is None:
            require(not stderr, f"{step}: unexpected stderr")
        else:
            # The pinned CLI renders the typed CompileDiagnostic as Rust Debug.
            # Match its kind, not its prose or recipe-span formatting.
            match = re.search(rb"\bkind: ([A-Za-z]+)\b", stderr)
            require(match is not None and match[1].decode("ascii") == diagnostic,
                    f"{step}: wrong typed compile refusal")
            require(not stdout, f"{step}: denied preview emitted candidate bytes")
        captured = Path(suite.pipes.records[-1]["artifacts"]["stdout"]["path"])
        require(captured.read_bytes() == stdout, f"{step}: captured stdout differs")
        watched[captured] = file_record(captured)
        observations.append(dict(step=step, stdout=file_record(captured), exit_code=code))
        return captured, stdout

    def preview(step, recipe, expected):
        captured, data = command(step, "splicery", ["preview", "--root", root,
                                suite.paths[recipe]], 0)
        require(data == expected, f"{step}: exact candidate bytes differ")
        return captured

    def local_report(captured, schema):
        result = document(captured)
        require(result["schema"] == schema and result["writfold_grounded"] is False
                and result["effect_authority"] is False, "Unexpected report authority/schema")
        return result

    def prove(step, candidate, outcomes):
        captured, _ = command(step, "weft", ["prove", candidate, "--json"], 1)
        result = local_report(captured, "weft.report/1")
        require(result["kind"] == "assessment" and result["scope"] == "local-symbolic" and
                result["execution"] == "complete" and not result["diagnostics"],
                f"{step}: incomplete or nonlocal assessment")
        review = result["review"]
        require(review["profile"] == "weft.symbolic/1" and
                review["requested_context"] == "unresolved-sandbox" and
                review["requested_snapshot"] == "unresolved-release" and
                review["target"]["reference"] == "unresolved-plan" and
                review["target"]["resolution"] == "unresolved",
                f"{step}: review scope changed")
        require(result["source"]["sha256"] == sha(candidate) and
                result["source"]["content"] == {
                    "encoding": "utf-8", "text": candidate.read_bytes().decode("utf-8")},
                f"{step}: source identity or exact UTF-8/CRLF content differs")
        actual = [(check["name"], check["result"]) for check in result["checks"]]
        require(actual == outcomes, f"{step}: semantic check outcomes differ: {actual!r}")
        require(all(check["basis"] == "hypothetical" for check in result["checks"]),
                f"{step}: checks claim nonlocal grounding")
        observations[-1]["checks"] = actual
        return captured

    def reuse(step, report, candidate, prior_source):
        changed = sha(prior_source) != sha(candidate)
        captured, _ = command(step, "weft", ["reuse", report, candidate, "--json"],
                              1 if changed else 0)
        result = local_report(captured, "weft.reuse/1")
        require(result["scope"] == "local-report-scope-equality-only" and
                result["prior"]["sha256"] == sha(report) and
                result["candidate"]["sha256"] == sha(candidate) and
                result["prior"]["path"] == str(report) and
                result["candidate"]["path"] == str(candidate),
                f"{step}: reuse identity/scope differs")
        differences = ([dict(field="source.sha256", expected=sha(prior_source),
                             observed=sha(candidate))] if changed else [])
        require(result["decision"] == ("refused" if changed else "reusable") and
                result["differences"] == differences, f"{step}: wrong reuse decision")
        observations[-1]["decision"] = result["decision"]

    if case["action"] == "preview-refusal":
        start = len(suite.pipes.records)
        for recipe, diagnostic in (("artifacts_missing", "MissingSelector"),
                                   ("artifacts_ambiguous", "AmbiguousSelector")):
            command(recipe, "splicery", ["preview", "--root", root, suite.paths[recipe]],
                    2, diagnostic)
            observations[-1]["diagnostic_kind"] = diagnostic
        require(len(suite.pipes.records) == start + 2, "Denied candidate was handed onward")
        return dict(commands=observations, original_unchanged=True,
                    candidate_emitted=False, downstream_commands=0)

    expected = suite.paths["artifacts_candidate"].read_bytes()
    require(b"\r\n" in original and original.count(b"\n") == original.count(b"\r\n")
            and "café".encode() in original and "雪".encode() in original,
            "Artifact fixture must exercise UTF-8 and CRLF")
    old_outcomes = [("positive", "entailed"), ("negative", "refuted"),
                    ("independent", "undetermined")]
    new_outcomes = [("positive", "refuted"), ("negative", "refuted"),
                    ("independent", "undetermined")]
    old_report = prove("prove-a", source, old_outcomes)
    reuse("reuse-a", old_report, source, suite.paths["artifacts_source"])
    candidate = preview("preview-b", "artifacts_forward", expected)
    new_report = prove("prove-b", candidate, new_outcomes)
    reuse("reject-old-for-b", old_report, candidate, suite.paths["artifacts_source"])
    reuse("reuse-b", new_report, candidate, candidate)

    # Only the harness changes its disposable target, between processes. All
    # command boundaries independently assert that no CLI mutates any input.
    source.chmod(0o644)
    source.write_bytes(candidate.read_bytes())
    source.chmod(0o444)
    watched[source] = file_record(source)
    reuse("reject-old-at-same-path", old_report, source, suite.paths["artifacts_source"])
    restored = preview("preview-a", "artifacts_reverse", original)
    prove("prove-restored-a", restored, old_outcomes)
    reuse("reuse-restored-stdout", old_report, restored, suite.paths["artifacts_source"])
    source.chmod(0o644)
    source.write_bytes(restored.read_bytes())
    source.chmod(0o444)
    watched[source] = file_record(source)
    reuse("reuse-restored-same-path", old_report, source, suite.paths["artifacts_source"])
    reuse("reject-b-for-restored-a", new_report, source, candidate)

    malformed = suite.paths["artifacts_malformed"]
    captured, _ = command("malformed-prior", "weft", ["reuse", malformed, source, "--json"], 2)
    result = local_report(captured, "weft.reuse/1")
    require(result["execution"] == "failed" and "decision" not in result and
            result["prior"] == {"path": str(malformed)} and
            result["candidate"] == {"path": str(source)} and
            len(result["diagnostics"]) == 1 and
            result["diagnostics"][0]["code"] == "ReuseInputUnavailable" and
            result["diagnostics"][0]["phase"] == "input",
            "Malformed prior did not fail before making a reuse decision")
    require(source.read_bytes() == original, "A/B/A did not restore exact source bytes")
    return dict(commands=observations, exact_utf8_crlf=True, source_aba=True,
                scope="local-report-scope-equality-only", native_source_unchanged=True,
                writfold_grounded=False, effect_authority=False)
