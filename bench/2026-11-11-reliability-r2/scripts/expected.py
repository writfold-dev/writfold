#!/usr/bin/env python3
"""Resolve every task spec in tasks.py against the pinned commit and write
the hidden expected answers. Asserts each spec (line text, substring present
exactly once, counts), so a wrong line number fails here, not in a trial.

Usage: expected.py TEMPLATE_REPO OUT_JSON"""
import ast, json, pathlib, subprocess, sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from tasks import COMMIT, TASKS, URL  # noqa: E402

repo, out = sys.argv[1], sys.argv[2]


def blob(path):
    return subprocess.run(["git", "-C", repo, "show", f"{COMMIT}:{path}"], capture_output=True, check=True).stdout.decode()


def row(path, line):
    return {"path": path, "line": line, "text": blob(path).split("\n")[line - 1]}


def mutate_text(text, m):
    lines = text.split("\n")
    if "insert_after" in m:
        n = m["insert_after"]
        lines = lines[:n] + list(m["lines"]) + lines[n:]
    else:
        n = m["replace_line"]
        assert lines[n - 1].count(m["old"]) == 1, (m, lines[n - 1])
        lines[n - 1] = lines[n - 1].replace(m["old"], m["new"])
    return "\n".join(lines)


def edit_files(spec):
    files = {}
    for path, line, old, new in spec["edits"]:
        files.setdefault(path, []).append((line, old, new))
    m = spec.get("mutation")
    if m:
        files.setdefault(m["path"], [])
    res = {}
    for path, eds in files.items():
        base = blob(path)
        lines = base.split("\n")
        changed = {}
        for line, old, new in eds:
            assert lines[line - 1].count(old) == 1, (path, line, old, lines[line - 1])
            changed[line] = lines[line - 1].replace(old, new)
        final_lines = [changed.get(i + 1, l) for i, l in enumerate(lines)]
        final = "\n".join(final_lines)
        r = {"changed_lines": {str(k): {"old": lines[k - 1], "new": v} for k, v in sorted(changed.items())},
             "final_without_mutation": final}
        for upath, line, old, new in spec.get("unmutated_edits", []):
            if upath == path:  # C06: the unmutated task's answer, used only if the mutation never fires
                assert lines[line - 1].count(old) == 1
                u = list(lines)
                u[line - 1] = lines[line - 1].replace(old, new)
                r["final_without_mutation"] = "\n".join(u)
                r["unmutated_changed_lines"] = {str(line): {"old": lines[line - 1], "new": u[line - 1]}}
        if m and m["path"] == path:
            mm = dict(m)
            if "renamed" in m:  # the inserted lines must also be renamed in the final checkout
                mm["lines"] = m["renamed"]
            r["mutated_base"] = mutate_text(base, m)
            r["final_with_mutation"] = mutate_text(final, mm) if "insert_after" in m else mutate_text(final, m)
        res[path] = r
    return res


expected = {"commit": COMMIT, "url": URL, "tasks": {}}
for tid, t in TASKS.items():
    s, e = t["spec"], {"family": t["family"], "kind": t["kind"]}
    if t["kind"] == "find":
        if "ast_methods_not_in" in s:
            path, cls, other = s["ast_methods_not_in"]
            tree = ast.parse(blob(path))
            meths = {c.name: {n.name: n.lineno for n in c.body if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))}
                     for c in tree.body if isinstance(c, ast.ClassDef)}
            rows = [row(path, ln) for name, ln in sorted(meths[cls].items(), key=lambda x: x[1]) if name not in meths[other]]
            assert len(rows) == 3, rows
        else:
            path, root = s["ast_subclasses"]
            # every class in the package, so a subclass defined elsewhere would be found too
            files = subprocess.run(["git", "-C", repo, "ls-tree", "-r", "--name-only", COMMIT, "httpx/"],
                                   capture_output=True, text=True, check=True).stdout.split()
            bases, where = {}, {}
            for f in files:
                if f.endswith(".py"):
                    for c in ast.walk(ast.parse(blob(f))):
                        if isinstance(c, ast.ClassDef):
                            bases[c.name] = [ast.unparse(b).split(".")[-1] for b in c.bases]
                            where[c.name] = (f, c.lineno)

            def sub(n, seen=()):
                return any(b == root or (b not in seen and sub(b, seen + (b,))) for b in bases.get(n, []))
            rows = sorted((row(*where[n]) for n in bases if sub(n)), key=lambda r: (r["path"], r["line"]))
            assert len(rows) == 15 and all(r["path"] == path for r in rows), rows
        e["rows"] = rows
    else:
        e["reply"] = t["reply"]
        e["files"] = edit_files(s)
        e["mutation"] = s.get("mutation")
        if t["kind"] == "session":
            e["steps"] = {k: [row(p, n) for p, n in v] for k, v in s["steps"].items()}
            p, n = s["s5"]
            e["s5"] = {"path": p, "line": n, "text": e["files"][p]["final_without_mutation"].split("\n")[n - 1]}
    expected["tasks"][tid] = e

# Spot checks of the intended design.
assert [r["line"] for r in expected["tasks"]["C01"]["rows"]] == [1978, 1990, 2008]
assert expected["tasks"]["C11"]["steps"]["S1"][0]["text"] == "DEFAULT_MAX_REDIRECTS = 20"
assert expected["tasks"]["C12"]["steps"]["S2"][0]["text"] == "class RequestError(HTTPError):"
assert "unquote(" in blob("httpx/_urls.py")
pathlib.Path(out).write_text(json.dumps(expected, indent=1, ensure_ascii=False) + "\n")
for tid, e in expected["tasks"].items():
    print(tid, e["kind"], len(e.get("rows", [])) or {p: len(f["changed_lines"]) for p, f in e.get("files", {}).items()})
