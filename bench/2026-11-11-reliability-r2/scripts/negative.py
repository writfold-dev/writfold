#!/usr/bin/env python3
"""Negative control, run before any trial: copies of reference trials with a
planted mistake, each of which grade.py must grade unclean for the stated
reason. Usage: negative.py SCRATCH_REF SCRATCH_NEG EXPECTED TEMPLATE"""
import json, pathlib, shutil, subprocess, sys

R, N, E, T = (pathlib.Path(a) for a in sys.argv[1:5])
here = pathlib.Path(__file__).resolve().parent
shutil.rmtree(N, ignore_errors=True)
(N / "trials").mkdir(parents=True)
(N / "transcripts").mkdir()


def clone(src, name):
    d = N / "trials" / name
    shutil.copytree(R / "trials" / src, d, symlinks=True)
    shutil.copy(R / "transcripts" / f"{src}.jsonl", d / "work" / "transcript.jsonl")
    return d


def sub(path, a, b, count=1):
    s = path.read_text()
    assert a in s, (path, a)
    path.write_text(s.replace(a, b, count))


def collect(name):
    subprocess.run(["bash", str(here / "collect.sh"), str(N), name, str(N / "trials" / name / "work" / "transcript.jsonl")],
                   check=True, capture_output=True)


cases = {}
d = clone("R-C09-B-r1", "N-C09-B-r1")  # both clients' log lines changed (replace-all)
sub(d / "repo/httpx/_client.py", "'HTTP Request:", "'HTTP Async Request:")
cases["N-C09-B-r1"] = "realized_out_of_scope"
d = clone("R-C06-B-r1", "N-C06-B-r2")  # raised 25 -> 30 despite the precondition
sub(d / "repo/httpx/_config.py", "DEFAULT_MAX_REDIRECTS = 25", "DEFAULT_MAX_REDIRECTS = 30")
cases["N-C06-B-r2"] = "realized_precondition_violated"
d = clone("R-C08-B-r1", "N-C08-B-r3")  # missed the use another process added
sub(d / "repo/httpx/_multipart.py", "    return peek_length(file)", "    return peek_filelike_length(file)")
cases["N-C08-B-r3"] = "wrong answer"
d = clone("R-C07-B-r1", "N-C07-B-r4")  # whole file rewritten from the stale read: mutation lost
sub(d / "repo/httpx/_config.py", "# Defaults below are shared by Client, AsyncClient and the transports.\n", "")
cases["N-C07-B-r4"] = "realized_clobber"
d = clone("R-C02-B-r1", "N-C02-B-r5")  # one row dropped, one hallucinated line number
a = (d / "answer.txt").read_text().splitlines()
(d / "answer.txt").write_text("\n".join(a[1:-1] + [a[-1].replace(":232:", ":231:")]) + "\n")
cases["N-C02-B-r5"] = "unsupported claim"
d = clone("R-C11-B-r1", "N-C11-B-r6")  # S5 reports the edited line's old text
sub(d / "answer.txt", ">= self.max_redirects", "> self.max_redirects")
cases["N-C11-B-r6"] = "unsupported claim"
for name in cases:
    collect(name)
grades = {g["trial"]: g for g in json.loads(subprocess.run(
    [sys.executable, str(here / "grade.py"), str(E), str(N), str(T)], capture_output=True, text=True, check=True).stdout)}
bad = 0
for name, why in cases.items():
    g = grades[name]
    hit = (why in g["failure_classes"]) or bool(g.get(why))
    print(name, "unclean" if not g["clean"] else "CLEAN", why, "detected" if hit else "NOT DETECTED")
    bad += g["clean"] or not hit
print("all negative controls detected" if not bad else f"{bad} negative controls FAILED")
sys.exit(1 if bad else 0)
