#!/usr/bin/env python3
"""Extract the hidden expected answers from a checkout at the corpus commit.

Find tasks (T1-T4, T7, T8) get answer rows; edit tasks (T5, T6) get the
target file's accepted final contents. T6's mid-session mutation is defined
here and read by scripts/mutate.py, so harness and grader share one source.
Usage: expected.py REPO > expected.json"""
import json, re, subprocess, sys

repo = sys.argv[1]
COMMIT = "e56621f7acf0d3392add5d814c35134b66c872a7"
assert subprocess.check_output(["git", "-C", repo, "rev-parse", "HEAD"], text=True).strip() == COMMIT


def grep(args):
    out = subprocess.run(["git", "-C", repo, "grep", "-n", "--no-color", *args],
                         capture_output=True).stdout.decode()
    rows = []
    for l in out.splitlines():
        p, n, t = l.split(":", 2)
        rows.append({"path": p, "line": int(n), "text": t})
    return rows


def source(path):
    return subprocess.run(["git", "-C", repo, "show", f"{COMMIT}:{path}"], capture_output=True,
                          check=True).stdout.decode()


def key(r):
    return (r["path"].encode(), r["line"])


def is_test_or_example(path):
    parts = path.split("/")
    name = parts[-1]
    return ("tests" in parts[:-1] or "examples" in parts[:-1] or name == "tests.rs"
            or name.endswith("_tests.rs"))


ans = {}
# T1: calls of a function or method named exactly `usable` (not reusable, gate_usable, ...).
ans["T1"] = [r for r in grep(["-E", r"usable\(", "--", "crates/"]) if re.search(r"(?<![A-Za-z0-9_])usable\(", r["text"])]
# T2: calls of `as_digest(` in non-test, non-example code, excluding comment lines and definitions.
ans["T2"] = [r for r in grep(["-F", "as_digest(", "--", "crates/"])
             if not r["text"].lstrip().startswith("//") and "fn as_digest" not in r["text"]
             and not is_test_or_example(r["path"])]
# T3: every line containing `.to_owned()`.
ans["T3"] = grep(["-F", ".to_owned()", "--", "crates/"])
# T4: calls of the `.expect(` method: none exist.
ans["T4"] = grep(["-F", ".expect(", "--", "crates/"])
assert ans["T4"] == []
# T7: `thread::sleep(` lines whose argument is not exactly `Duration::from_millis(1)`.
ans["T7"] = [r for r in grep(["-F", "thread::sleep(", "--", "crates/"]) if "thread::sleep(Duration::from_millis(1))" not in r["text"]]
# T8: the definition of WORKTRUNK_BYTES.
ans["T8"] = [r for r in grep(["-F", "WORKTRUNK_BYTES", "--", "crates/"]) if re.search(r"\bconst WORKTRUNK_BYTES\b", r["text"])]

# T5: the Tuft scope-prefix limit (crates/tuft/src/ir.rs), not the app's scanned-bytes limit.
t5_path = "crates/tuft/src/ir.rs"
t5_src = source(t5_path)
t5_old = "pub const MAX_SCOPE_BYTES: usize = 1_024;"
assert t5_src.count(t5_old) == 1
t5 = {"path": t5_path, "decoy": "crates/writfold-app/src/tuft/mod.rs",
      "accepted": [t5_src.replace(t5_old, f"pub const MAX_SCOPE_BYTES: usize = {v};") for v in ("2_048", "2048")]}

# T6: raise MAX_REQUEST_BYTES (crates/writfold-wire/src/rpc.rs), not MAX_RESPONSE_BYTES;
# the harness inserts two lines after line 5 once the trigger fires.
t6_path = "crates/writfold-wire/src/rpc.rs"
t6_src = source(t6_path)
t6_old = "pub const MAX_REQUEST_BYTES: usize = 65_536;"
assert t6_src.count(t6_old) == 1
mutation_lines = ["//!",
                  "//! The request ceiling below is shared with the response codec; keep them in step."]
lines = t6_src.split("\n")
mutated = "\n".join(lines[:5] + mutation_lines + lines[5:])
t6 = {"path": t6_path, "decoy": "crates/writfold-wire/src/rpc/response/mod.rs",
      "trigger": "writfold-wire/src/rpc.rs", "insert_after_line": 5, "mutation_lines": mutation_lines,
      "mutated": mutated,
      "accepted": [mutated.replace(t6_old, f"pub const MAX_REQUEST_BYTES: usize = {v};") for v in ("131_072", "131072")],
      "accepted_without_mutation": [t6_src.replace(t6_old, f"pub const MAX_REQUEST_BYTES: usize = {v};") for v in ("131_072", "131072")]}

print(json.dumps({"commit": COMMIT,
                  "answers": {k: sorted(v, key=key) for k, v in ans.items()},
                  "edits": {"T5": t5, "T6": t6}}, indent=1, ensure_ascii=False))
