"""Strict admission of pinned cargo-verus reports, independent of execution."""
from copy import deepcopy
from dataclasses import dataclass
import json
from pathlib import Path
import re


ADMISSION_CONTROLS = (
    "absent", "truncated", "foreign-only", "partial", "zero-obligation",
    "missing-named-function", "missing-overload", "missing-failed-function",
    "unexpected-failed-function", "split-diagnostic", "successful-mutant-process",
)

class EvidenceError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise EvidenceError(message)


def objects(text):
    """Decode complete top-level JSON records amid Cargo's plain progress lines."""
    decoder = json.JSONDecoder()
    position = 0
    for match in re.finditer(r"(?m)^\s*\{", text):
        if match.start() < position:
            continue
        start = text.index("{", match.start())
        try:
            value, position = decoder.raw_decode(text, start)
        except json.JSONDecodeError as error:
            raise EvidenceError("truncated or malformed JSON record") from error
        require(isinstance(value, dict), "non-object report")
        yield value


def solver_functions(report):
    functions = {}
    for module in report.get("times-ms", {}).get("smt", {}).get("smt-run-module-times", []):
        for entry in module.get("function-breakdown", []):
            name = entry.get("function")
            require(isinstance(name, str) and type(entry.get("success")) is bool,
                    "missing solver name/verdict")
            functions[name] = functions.get(name, True) and entry["success"]
    return functions


def solver_counts(report):
    counts = {}
    for module in report.get("times-ms", {}).get("smt", {}).get("smt-run-module-times", []):
        for entry in module.get("function-breakdown", []):
            name = entry["function"]
            counts[name] = counts.get(name, 0) + 1
    return counts


def require_coverage(report, functions, spec):
    counts = solver_counts(report)
    for name, source in spec["functions"].items():
        require(name in functions, "missing named production solver entry")
        require(type(source["solver_entries"]) is int and source["solver_entries"] > 0
                and counts.get(name) == source["solver_entries"],
                "missing or unexpected overloaded production solver coverage")


def mutation_functions(profile, mutation):
    names = mutation.get("functions")
    require(isinstance(names, list) and names and all(isinstance(name, str) for name in names)
            and len(set(names)) == len(names), "invalid explicit mutant function family")
    available = profile["crates"][mutation["crate"]]["functions"]
    require(all(name in available for name in names), "mutant names unprofiled production function")
    return names


@dataclass
class Evidence:
    records: list

    @classmethod
    def parse(cls, text):
        return cls(list(objects(text)))

    def report(self, crate):
        # func-details contains imported specifications: it is NOT proof coverage.
        reports = [r for r in self.records if "verification-results" in r
                   and any(n.startswith(crate + "::") for n in solver_functions(r))]
        require(len(reports) == 1, f"expected one fresh solver report for {crate}")
        report = reports[0]
        functions = solver_functions(report)
        require(all(n.startswith(crate + "::") for n in functions), "foreign solver coverage")
        result = report["verification-results"]
        require(result.get("is-verifying-entire-crate") is True, "partial-crate verification")
        require(result.get("encountered-vir-error") is False, "VIR/front-end failure")
        for key in ("verified", "errors"):
            require(type(result.get(key)) is int and result[key] >= 0, "invalid obligation count")
        require(result["verified"] > 0, "zero verified obligations")
        return report, functions

    def positive(self, status, profile):
        require(status == 0, "verifier process failed")
        admitted = {}
        for crate, spec in profile["crates"].items():
            report, functions = self.report(crate)
            result = report["verification-results"]
            require(result.get("success") is True and result.get("encountered-error") is False
                    and result["errors"] == 0, "positive verification failed")
            require(result["verified"] >= spec["minimum_verified"], "obligation floor not met")
            require(all(functions.get(n) is True for n in spec["functions"]),
                    "missing named production solver success")
            require_coverage(report, functions, spec)
            require(all(functions.values()), "failed solver query in positive report")
            artifacts = [r for r in self.records if r.get("reason") == "compiler-artifact"
                         and r.get("target", {}).get("name") == crate]
            require(len(artifacts) == 1 and artifacts[0].get("fresh") is False,
                    "missing or cached production compiler artifact")
            admitted[crate] = {"results": result, "functions": functions,
                               "compiler_artifact": artifacts[0], "verus": report.get("verus")}
        require(not any(r.get("reason") == "compiler-message"
                        and r.get("message", {}).get("level") == "error" for r in self.records),
                "compiler error in positive run")
        require(any(r.get("reason") == "build-finished" and r.get("success") is True
                    for r in self.records), "missing successful Cargo completion")
        return admitted

    def mutant(self, status, profile, mutation, workspace):
        require(status > 0, "mutant process did not fail normally")
        crate = mutation["crate"]
        names = mutation_functions(profile, mutation)
        report, functions = self.report(crate)
        result = report["verification-results"]
        require(result.get("success") is False and result.get("encountered-error") is True
                and result["errors"] > 0, "mutant did not fail verification")
        require({n for n, ok in functions.items() if not ok} == set(names),
                "wrong production function family failed")
        require_coverage(report, functions, profile["crates"][crate])
        declarations = [declaration for name in names
                        for declaration in profile["crates"][crate]["functions"][name]["declarations"]]
        require(declarations, "missing production declaration mapping")
        diagnostics = []
        for record in self.records:
            if record.get("reason") != "compiler-message":
                continue
            message = record.get("message", {})
            if message.get("level") != "error":
                continue
            text = message.get("message", "")
            if text.startswith("aborting due to "):
                continue
            require(record.get("target", {}).get("name") == crate,
                    "foreign compiler error")
            require(text in {"postcondition not satisfied", "possible arithmetic underflow/overflow"},
                    f"not an intended contract/arithmetic failure: {text}")
            require(any(s.get("is_primary") is True
                        and any((workspace / s.get("file_name", "")).resolve()
                                == (workspace / declaration["source"]).resolve()
                                and type(s.get("line_start")) is int and type(s.get("line_end")) is int
                                and declaration["lines"][0] <= s["line_start"]
                                    <= s["line_end"] <= declaration["lines"][1]
                                for declaration in declarations)
                        for s in message.get("spans", [])),
                    "proof failure lacks primary span in the named production function")
            diagnostics.append(record)
        require(diagnostics, "missing attributable proof diagnostic")
        return {"results": result, "functions": functions, "diagnostics": diagnostics}


def changed_verdict(evidence, crate, name, success):
    changed = deepcopy(evidence)
    report, functions = changed.report(crate)
    require(name in functions and functions[name] is not success, "control must change an actual verdict")
    for module in report["times-ms"]["smt"]["smt-run-module-times"]:
        for entry in module["function-breakdown"]:
            if entry["function"] == name:
                entry["success"] = success
    return changed


def self_controls(profile, positive, negative, mutation, workspace):
    """Deliberately corrupt actual reports; never generate substitute proof logs."""
    cases = []
    crate = mutation["crate"]
    def rejected(name, action):
        try:
            action()
        except EvidenceError:
            cases.append({"name": name, "outcome": "rejected"})
        else:
            raise EvidenceError(f"admission control accepted {name}")
    rejected("absent", lambda: Evidence.parse("").positive(0, profile))
    rejected("truncated", lambda: Evidence.parse('{"verification-results":'))
    foreign = [r for r in positive.records if not any(
        n.startswith("writfold_") for n in solver_functions(r))]
    rejected("foreign-only", lambda: Evidence(foreign).positive(0, profile))
    for label, key, value in (("partial", "is-verifying-entire-crate", False),
                              ("zero-obligation", "verified", 0)):
        changed = deepcopy(positive)
        changed.report(crate)[0]["verification-results"][key] = value
        rejected(label, lambda: changed.positive(0, profile))
    changed = deepcopy(positive)
    target, _ = changed.report(crate)
    missing = next(iter(profile["crates"][crate]["functions"]))
    for module in target["times-ms"]["smt"]["smt-run-module-times"]:
        module["function-breakdown"] = [f for f in module["function-breakdown"]
                                        if f["function"] != missing]
    rejected("missing-named-function", lambda: changed.positive(0, profile))
    changed = deepcopy(positive)
    overloads = [(owner, name) for owner, spec in profile["crates"].items()
                 for name, source in spec["functions"].items() if source["solver_entries"] > 1]
    require(overloads, "profile lacks required overloaded-constructor calibration")
    owner, overloaded = overloads[0]
    target, _ = changed.report(owner)
    removed = False
    for module in target["times-ms"]["smt"]["smt-run-module-times"]:
        for index, entry in enumerate(module["function-breakdown"]):
            if entry["function"] == overloaded:
                del module["function-breakdown"][index]
                removed = True
                break
        if removed:
            break
    require(removed, "overloaded solver entry absent from positive evidence")
    rejected("missing-overload", lambda: changed.positive(0, profile))
    intended = mutation_functions(profile, mutation)
    hidden_failure = changed_verdict(negative, crate, intended[0], True)
    rejected("missing-failed-function", lambda: hidden_failure.mutant(1, profile, mutation, workspace))
    _, functions = negative.report(crate)
    unaffected = next(name for name, success in functions.items() if success and name not in intended)
    extra_failure = changed_verdict(negative, crate, unaffected, False)
    rejected("unexpected-failed-function", lambda: extra_failure.mutant(1, profile, mutation, workspace))
    split = deepcopy(negative)
    errors = [r for r in split.records if r.get("reason") == "compiler-message"
              and r.get("message", {}).get("level") == "error"
              and not r["message"].get("message", "").startswith("aborting due to ")]
    require(errors, "no actual negative diagnostics for self-controls")
    for record in errors:
        warning = deepcopy(record)
        warning["message"]["level"] = "warning"
        split.records.append(warning)
        for span in record["message"].get("spans", []):
            span["file_name"] = "foreign.rs"
    rejected("split-diagnostic", lambda: split.mutant(1, profile, mutation, workspace))
    rejected("successful-mutant-process", lambda: negative.mutant(0, profile, mutation, workspace))
    require(tuple(case["name"] for case in cases) == ADMISSION_CONTROLS,
            "incomplete independent admission calibration")
    return cases
