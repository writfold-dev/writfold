#!/usr/bin/env python3
"""Positive control, run before any trial: a reference solution for every
candidate task that grade.py accepts as clean, made with arm B's tools (the
Writfold trial flow for every edit, Tuft for every find), so each task is
fair, not impossible, in the more constrained arm. Mutations are applied as
the harness applies them, before the first prepare (after the agent's first
sight of the file). Each reference trial gets a one-call synthetic
transcript carrying its final reply.

Usage: reference.py SCRATCH_REF TEMPLATE EXPECTED_JSON WRITFOLD
Writes SCRATCH_REF/trials/R-<task>-B-r1 and SCRATCH_REF/transcripts; then
run grade.py EXPECTED_JSON SCRATCH_REF TEMPLATE and check clean for all."""
import json, pathlib, shutil, subprocess, sys

S, T, E, W = (pathlib.Path(a) for a in sys.argv[1:5])
here = pathlib.Path(__file__).resolve().parent
ex = json.load(open(E))
C = ex["commit"]


def sh(args, cwd=None, ok=True):
    r = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if ok and r.returncode:
        raise SystemExit(f"FAILED {args}\n{r.stdout}\n{r.stderr}")
    return r


def flow(d, path, recipe, apply_args=("git", "apply")):
    """recipe -> prepare -> approve (next: line) -> apply (next: line) -> git diff | git apply."""
    rp = d / "work" / (path.replace("/", "_") + ".spl")
    rp.write_text(recipe)
    out = sh([str(W), "trial", "prepare", "--repo", str(d / "repo"), "--file", path, "--recipe", str(rp),
              "--state", str(d / "state")]).stdout
    for _ in range(2):  # approve, then apply, each run exactly as its next: line prints it
        nxt = [l for l in out.splitlines() if l.startswith("next: ")][-1][6:].split()
        assert nxt[0] == "writfold" and "--state" in nxt, nxt
        out = sh([str(W)] + nxt[1:]).stdout
    run = nxt[3]
    patch = d / "work" / (path.replace("/", "_") + ".patch")
    patch.write_text(sh(["git", "diff"], cwd=d / "state" / "worktrees" / run).stdout)
    return sh(list(apply_args) + [str(patch)], cwd=d / "repo", ok=False)


def tuft(d, needle):
    q = d / "work" / "q.tuft"
    q.write_text(f'@h := find text "{needle}" in "httpx/" at "{C}" limit 100;\n'
                 "@e := retain @h around lines 0;\n@v := project @e [path, line, excerpt(max_chars: 300)];\n"
                 "emit @v budget_tokens 4000;\n")
    return sh([str(W), "tuft", "run", str(q), "--repo", str(d / "repo"), "--state", str(d / "state")]).stdout


R = {
 "C03": [("httpx/_config.py", 'edit "httpx/_config.py" {\n  @x := one text "Timeout(timeout=5.0)"\n  replace @x with "Timeout(timeout=8.0)"\n}\n'),
         ("httpx/_main.py", 'edit "httpx/_main.py" {\n  @a := one text "\\" connection, reading some data, etc... [Default: 5.0]"\n'
          '  replace @a with "\\" connection, reading some data, etc... [Default: 8.0]"\n'
          '  @b := one text "default=5.0,"\n  replace @b with "default=8.0,"\n'
          '  @c := one text "\\"connection, reading some data, etc... [Default: 5.0]"\n'
          '  replace @c with "\\"connection, reading some data, etc... [Default: 8.0]"\n}\n')],
 "C04": [("httpx/_utils.py", 'edit "httpx/_utils.py" {\n  @x := one text "def primitive_value_to_str("\n  replace @x with "def primitive_to_str("\n}\n')]
        + [(p, f'edit "{p}" {{\n  each @r in line_matches rx"^(?<pre>.*)primitive_value_to_str(?<post>.*)$" expect {n} {{\n'
            '    replace @r with [keep(@r.pre), "primitive_to_str", keep(@r.post)]\n  }\n}\n')
           for p, n in [("httpx/_urls.py", 4), ("httpx/_content.py", 3), ("httpx/_multipart.py", 2)]],
 "C05": [("httpx/_client.py", 'edit "httpx/_client.py" {\n  @s := one text "class Client(BaseClient):"\n'
          '  @a := one text "class AsyncClient(BaseClient):"\n  @z := one text "async def __aexit__("\n'
          '  @sync := from @s to @a\n  @async := from @a to @z\n'
          '  @x := one text "HTTP Request:" within @sync\n  @y := one text "HTTP Request:" within @async\n'
          '  replace @x with "HTTP Exchange:"\n  replace @y with "HTTP Exchange:"\n}\n'),
         ("tests/test_utils.py", 'edit "tests/test_utils.py" {\n  each @r in line_matches rx"^(?<pre>.*)HTTP Request:(?<post>.*)$" expect 3 {\n'
          '    replace @r with [keep(@r.pre), "HTTP Exchange:", keep(@r.post)]\n  }\n}\n')],
 "C06": [("httpx/_config.py", 'edit "httpx/_config.py" {\n  @x := one text "DEFAULT_MAX_REDIRECTS = 20"\n  replace @x with "DEFAULT_MAX_REDIRECTS = 30"\n}\n')],
 "C07": [("httpx/_config.py", 'edit "httpx/_config.py" {\n  @x := one text "max_keepalive_connections=20"\n  replace @x with "max_keepalive_connections=40"\n}\n')],
 "C08": [("httpx/_utils.py", 'edit "httpx/_utils.py" {\n  @x := one text "def peek_filelike_length("\n  replace @x with "def peek_length("\n}\n')]
        + [(p, f'edit "{p}" {{\n  each @r in line_matches rx"^(?<pre>.*)peek_filelike_length(?<post>.*)$" expect {n} {{\n'
            '    replace @r with [keep(@r.pre), "peek_length", keep(@r.post)]\n  }\n}\n')
           for p, n in [("httpx/_content.py", 2), ("httpx/_multipart.py", 3)]],
 "C09": [("httpx/_client.py", 'edit "httpx/_client.py" {\n  @a := one text "class AsyncClient(BaseClient):"\n  @z := one text "async def __aexit__("\n'
          '  @async := from @a to @z\n  @y := one text "HTTP Request:" within @async\n  replace @y with "HTTP Async Request:"\n}\n')],
 "C10": [("httpx/_utils.py", 'edit "httpx/_utils.py" {\n  @x := one text "def unquote("\n  replace @x with "def unquote_header_value("\n}\n'),
         ("httpx/_auth.py", 'edit "httpx/_auth.py" {\n  @a := one text "to_str, unquote"\n  replace @a with "to_str, unquote_header_value"\n'
          '  @b := one text "= unquote(value)"\n  replace @b with "= unquote_header_value(value)"\n}\n')],
 "C11": [("httpx/_client.py", 'edit "httpx/_client.py" {\n  @a := one text "class AsyncClient(BaseClient):"\n  @z := one text "async def __aexit__("\n'
          '  @async := from @a to @z\n  @y := one text "len(history) > self.max_redirects" within @async\n'
          '  replace @y with "len(history) >= self.max_redirects"\n}\n')],
 "C12": [("httpx/_client.py", 'edit "httpx/_client.py" {\n  @a := one text "class AsyncClient(BaseClient):"\n  @z := one text "async def __aexit__("\n'
          '  @async := from @a to @z\n  @y := one text "\\"Exceeded maximum allowed redirects.\\"" within @async\n'
          '  replace @y with "\\"Exceeded maximum allowed redirects (async client).\\""\n}\n')],
}

shutil.rmtree(S, ignore_errors=True)
(S / "transcripts").mkdir(parents=True)
for tid, e in ex["tasks"].items():
    d = S / "trials" / f"R-{tid}-B-r1"
    (d / "work").mkdir(parents=True)
    sh(["git", "-c", "advice.detachedHead=false", "clone", "-q", str(T), str(d / "repo")])
    sh(["git", "remote", "remove", "origin"], cwd=d / "repo")
    sh([str(W), "trial", "init", "--state", str(d / "state")])
    reply, notes = "DONE", []
    rows = e.get("rows", []) + [dict(r, step=s) for s, rs in e.get("steps", {}).items() for r in rs]
    for r in rows:  # every find row is shown by a Tuft run (the reference uses Tuft's rows, not the key)
        assert f'{r["path"]}:{r["line"]}:' in tuft(d, r["text"].strip().replace("\\", "\\\\").replace('"', '\\"')[:60]), r
    if e.get("mutation"):
        info = json.loads(sh([sys.executable, str(here / "mutate.py"), "apply", str(E), tid, str(d)]).stdout)
        info.update({"triggered": True, "tool_use_id": None, "mutated_at": "reference"})
        (d / "mutation.json").write_text(json.dumps(info))
        if tid == "C08":  # the worktree is made from HEAD: commit the other process's change first
            sh(["git", "-c", "user.name=ref", "-c", "user.email=ref@example.invalid", "commit", "-qam", "other process"], cwd=d / "repo")
            notes.append("committed the mutation so prepare's HEAD worktree contains it")
    for path, recipe in R.get(tid, []):
        r = flow(d, path, recipe)
        if r.returncode and tid == "C07":
            notes.append("git apply refused (context changed); patch -p1 applied with offset/fuzz")
            r = flow(d, path, recipe, ("patch", "-p1", "--no-backup-if-mismatch", "-i"))
        if r.returncode and tid == "C06":
            notes.append("git apply refused: the checkout no longer has 20 (another process set 25); no change made")
            reply = "REFUSED: DEFAULT_MAX_REDIRECTS is 25 in the checkout, not 20"
            continue
        if r.returncode:
            raise SystemExit(f"{tid} {path}: apply failed\n{r.stdout}\n{r.stderr}")
    if e["kind"] in ("find", "session"):
        lines = [f'{r["path"]}:{r["line"]}: {r["text"]}' for r in e.get("rows", [])]
        lines += [f'{s} {r["path"]}:{r["line"]}: {r["text"]}' for s, rs in e.get("steps", {}).items() for r in rs]
        if "s5" in e:
            r = e["s5"]
            lines.append(f'S5 {r["path"]}:{r["line"]}: {r["text"]}')
        (d / "answer.txt").write_text("\n".join(lines) + "\n")
    # The reference's evidence: it read each file it edits (pinned bytes) before its first write.
    evs = []
    for i, path in enumerate(e.get("files", {})):
        evs.append({"type": "assistant", "timestamp": "2026-11-11T00:00:00Z", "message": {"model": "reference", "content": [
            {"type": "tool_use", "id": f"read{i}", "name": "Read", "input": {"file_path": f"{d}/repo/{path}"}}]}})
        evs.append({"type": "user", "timestamp": "2026-11-11T00:00:00Z", "message": {"content": [
            {"type": "tool_result", "tool_use_id": f"read{i}", "content": sh(["git", "show", f"{C}:{path}"], cwd=d / "repo").stdout}]}})
    evs.append({"type": "assistant", "timestamp": "2026-11-11T00:00:01Z",
                "message": {"model": "reference", "content": [{"type": "tool_use", "id": "ref", "name": "SubagentHandback",
                                                               "input": {"message": reply}}]}})
    (d / "work" / "transcript.jsonl").write_text("".join(json.dumps(v) + "\n" for v in evs))
    sh(["bash", str(here / "collect.sh"), str(S), f"R-{tid}-B-r1", str(d / "work" / "transcript.jsonl")])
    print(tid, reply.split(":")[0], "; ".join(notes))
