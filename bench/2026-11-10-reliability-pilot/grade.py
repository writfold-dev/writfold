#!/usr/bin/env python3
"""Grade reliability-pilot trials, identically for both arms, from each
agent's transcript, its answer.txt and the checkout's final state. The
Writfold ledger is never read.

Usage: grade.py EXPECTED_JSON SCRATCH TEMPLATE_REPO > grades.json
  SCRATCH/trials/<trial>/{answer.txt,final.diff,status.txt,repo/,mutation.json}
  SCRATCH/transcripts/<trial>.jsonl
  TEMPLATE_REPO: a clone at the corpus commit (the pinned source of truth).

The definitions below are the pre-registered ones (README, "Metrics").
"""
import difflib, json, pathlib, re, subprocess, sys

ROW = re.compile(r"^(?:\./)?([^\s:][^:]*):(\d+):\s?(.*)$")
CLAIM = re.compile(r"((?:crates|vendor|docs|bench|proofs|scripts)/[\w./-]+\.\w+):(\d+)")
FIND = {"T1", "T2", "T3", "T4", "T7", "T8"}
EDIT = {"T5", "T6"}
# Failing-call classifier (pre-registered). Harness or isolation-guard refusals
# are environment checks on command shape or permissions, not agent errors.
HARNESS_RX = re.compile(r"isolated in the worktree|Refusing to run it|Permission to use .* (?:has been )?denied|"
                        r"requires (?:user )?approval|was blocked by|user (?:doesn't|does not) want|"
                        r"permission denied by|denied by (?:the )?(?:sandbox|harness|policy)", re.I)
SEARCH_PROGS = {"grep", "egrep", "fgrep", "rg", "ag", "ack", "find", "fd", "locate"}
WRITE_RX = re.compile(r"\bsed\s+(?:-[a-zA-Z]*i|--in-place)|\bperl\s+-[a-zA-Z]*i|\btee\b|\bcp\s|\bmv\s|"
                      r"\bgit\s+(?:apply|checkout|restore|stash|reset|am|cherry-pick|commit)\b|\bpatch\b|"
                      r"\btruncate\b|open\([^)]*['\"][wa]['\"]|write_text|write_bytes")
TUFT_RX = re.compile(r"\btuft\s+(?:run|check)\b")
SYNTAX_RX = re.compile(r"error\[T[012]\d\d\]")
RECIPE_ERR_RX = re.compile(r"recipe|splicery|parse|syntax|expected", re.I)


def text_of(c):
    if isinstance(c, str):
        return c
    return "".join(x.get("text", "") for x in c if isinstance(x, dict))


def load_transcript(path):
    calls, results, final_text, models, order = {}, {}, "", set(), []
    for line in open(path, errors="replace"):
        try:
            ev = json.loads(line)
        except Exception:
            continue
        m = ev.get("message") or {}
        if ev.get("type") == "assistant":
            if m.get("model"):
                models.add(m["model"])
            for c in m.get("content", []):
                if not isinstance(c, dict):
                    continue
                if c.get("type") == "tool_use":
                    calls[c["id"]] = {"tool": c["name"], "input": c.get("input", {}), "ts": ev.get("timestamp")}
                    order.append(c["id"])
                elif c.get("type") == "text" and c.get("text", "").strip():
                    final_text = c["text"]
        elif ev.get("type") == "user" and isinstance(m.get("content"), list):
            for c in m["content"]:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    results[c["tool_use_id"]] = (text_of(c.get("content", "")), bool(c.get("is_error")))
    out = []
    for i in order:
        r, err = results.get(i, ("", False))
        out.append({**calls[i], "id": i, "result": r, "is_error": err})
    handback = [c for c in out if c["tool"] == "SubagentHandback"]
    if handback:
        final_text = handback[-1]["input"].get("message", final_text)
    return out, final_text.strip(), sorted(models)


class Pinned:
    def __init__(self, repo, commit):
        self.repo, self.commit, self.cache = repo, commit, {}
        self.files = set(subprocess.run(["git", "-C", repo, "ls-tree", "-r", "--name-only", commit],
                                        capture_output=True, text=True, check=True).stdout.splitlines())

    def lines(self, path):
        if path not in self.cache:
            self.cache[path] = (subprocess.run(["git", "-C", self.repo, "show", f"{self.commit}:{path}"],
                                               capture_output=True).stdout.decode(errors="replace").split("\n")
                                if path in self.files else None)
        return self.cache[path]

    def text(self, path):
        ls = self.lines(path)
        return None if ls is None else "\n".join(ls)


def bash_cmd(c):
    return c["input"].get("command", "") if c["tool"] == "Bash" else ""


def failing(c):
    if c["tool"] == "SubagentHandback":
        return False
    return c["is_error"] or bool(re.match(r"\s*Exit code [1-9]", c["result"]))


def classify_failure(c):
    """harness | no-match | agent"""
    if HARNESS_RX.search(c["result"]):
        return "harness"
    cmd = bash_cmd(c)
    if cmd:
        first = pipeline_heads(cmd)
        body = re.sub(r"^\s*Exit code 1\s*", "", c["result"]).strip()
        if re.match(r"\s*Exit code 1\b", c["result"]) and not body and any(p in SEARCH_PROGS or p == "git grep" for p in first):
            return "no-match"
    return "agent"


def strip_env(seg):
    seg = seg.strip()
    while True:
        m = re.match(r"^(?:export\s+)?[A-Za-z_][A-Za-z0-9_]*=\S*\s+", seg)
        if not m:
            return seg
        seg = seg[m.end():]


def program_of(seg):
    seg = strip_env(seg)
    toks = seg.split()
    if not toks:
        return ""
    p = toks[0].rsplit("/", 1)[-1]
    if p == "git" and len(toks) > 1:
        rest = [t for t in toks[1:] if not t.startswith("-")]
        # git -C DIR grep
        if "-C" in toks[1:3] and len(toks) > 3:
            rest = toks[3:]
        return "git " + (rest[0] if rest else "")
    if p == "writfold":
        return "writfold " + " ".join(t for t in toks[1:3] if not t.startswith("-"))
    return p


def pipeline_heads(cmd):
    """Programs that start a pipeline (read files rather than stdin)."""
    heads = []
    for stmt in re.split(r"&&|\|\||;|\n", cmd):
        segs = stmt.split("|")
        heads.append(program_of(segs[0]))
    return heads


def kind(c):
    if c["tool"] != "Bash":
        return c["tool"]
    heads = [h for h in pipeline_heads(bash_cmd(c)) if h and h != "cd"]
    return "Bash:" + (heads[0] if heads else "")


def repo_paths_in(cmd, trial):
    paths = set(re.findall(r"(?<![\w/])(crates/[\w./-]+)", cmd))
    for m in re.findall(re.escape(trial) + r"/repo/([\w./-]+)", cmd):
        paths.add(m)
    return {p.rstrip(".,;:)") for p in paths}


def bash_writes_repo(cmd, trial):
    """Return (is_write_like_on_repo, targets). Redirects into work/ or /dev/null are not repo writes."""
    targets = set()
    redirects = re.findall(r"(?<![0-9&])>>?\s*([^\s;&|]+)", cmd)
    for dest in redirects:
        if dest.startswith("/dev/") or "/work/" in dest or dest.startswith("work/") or dest.startswith("&"):
            continue
        if "/repo/" in dest or dest.startswith("crates/") or (("cd " + trial + "/repo") in cmd and not dest.startswith("/")):
            targets.add(dest.split("/repo/", 1)[-1])
    if WRITE_RX.search(cmd):
        rp = repo_paths_in(cmd, trial)
        if re.search(r"\bgit\s+(?:apply|checkout|restore|stash|reset)\b|\bpatch\b", cmd) and (trial + "/repo" in cmd or "repo" in cmd):
            targets.add("<patch>")
        targets |= {p for p in rp if not re.search(r"(?:^|/)work/", p)}
    return bool(targets), targets


def changed_lines(base, final):
    """(removed base lines, added final lines) as stripped strings, via difflib."""
    sm = difflib.SequenceMatcher(a=base.split("\n"), b=final.split("\n"), autojunk=False)
    removed, added = [], []
    for op, a0, a1, b0, b1 in sm.get_opcodes():
        if op != "equal":
            removed += base.split("\n")[a0:a1]
            added += final.split("\n")[b0:b1]
    return removed, added


def grade_answer(task, expected, answer_path, pinned):
    want = {(e["path"], e["line"]): e["text"].strip() for e in expected}
    res = {"answer_present": answer_path.exists(), "expected": len(want)}
    rows, malformed, none_line = {}, [], False
    if answer_path.exists():
        for raw in answer_path.read_text(errors="replace").splitlines():
            if not raw.strip():
                continue
            if raw.strip() == "NONE":
                none_line = True
                continue
            m = ROW.match(raw.strip())
            if not m:
                malformed.append(raw[:200])
                continue
            rows[(m.group(1), int(m.group(2)))] = m.group(3).strip()
    missing = sorted(f"{p}:{n}" for p, n in set(want) - set(rows))
    extra = sorted(f"{p}:{n}" for p, n in set(rows) - set(want))
    wrong = sorted(f"{p}:{n}" for k in set(rows) & set(want) if rows[k] != want[k] for p, n in [k])
    correct_rows = len(set(rows) & set(want)) - len(wrong)
    if task == "T4":
        success = res["answer_present"] and none_line and not rows and not malformed
    else:
        success = res["answer_present"] and not none_line and not missing and not extra and not wrong and not malformed
    unsupported = []
    for (p, n), t in rows.items():
        ls = pinned.lines(p)
        if ls is None or not (1 <= n <= len(ls)) or (t and ls[n - 1].strip() != t):
            unsupported.append(f"{p}:{n}")
    res.update({"success": success, "rows_given": len(rows), "correct_rows": correct_rows, "none_line": none_line,
                "missing": missing, "extra": extra, "wrong_text": wrong, "malformed": malformed,
                "recall": (correct_rows / len(want)) if want else None,
                "precision": (correct_rows / len(rows)) if rows else None,
                "unsupported_answer_rows": sorted(unsupported)})
    return res


def main():
    expected = json.load(open(sys.argv[1]))
    S = pathlib.Path(sys.argv[2])
    pinned = Pinned(sys.argv[3], expected["commit"])
    out = []
    for tdir in sorted(p for p in (S / "trials").iterdir() if p.is_dir()):
        trial = tdir.name
        task, arm, rep = trial.split("-")
        tpath = str(tdir)
        tr = S / "transcripts" / f"{trial}.jsonl"
        calls, final_text, models = load_transcript(tr) if tr.exists() else ([], "", [])
        g = {"trial": trial, "task": task, "arm": arm, "repeat": int(rep[1:]), "models": models,
             "transcript_present": tr.exists(), "final_reply": final_text[:300]}
        # ---- baseline of the checkout (pinned, plus the T6 mutation if it fired)
        mutation = None
        if task == "T6" and (tdir / "mutation.json").exists():
            mutation = json.load(open(tdir / "mutation.json"))
        g["mutation"] = mutation
        e6 = expected["edits"]["T6"]
        mutated = bool(mutation and mutation.get("triggered"))
        if mutated:
            ids = [c["id"] for c in calls]
            k = ids.index(mutation["tool_use_id"]) if mutation["tool_use_id"] in ids else None
            nxt = calls[k + 1]["ts"] if k is not None and k + 1 < len(calls) else None
            g["mutation_before_next_call"] = (nxt is None or mutation["mutated_at"].replace("+00:00", "") < nxt.rstrip("Z"))
            g["mutation_trigger_call"] = k
        status = [l for l in (tdir / "status.txt").read_text().splitlines() if l.strip()] if (tdir / "status.txt").exists() else []
        changed = []
        for l in status:
            path = l[3:].split(" -> ")[-1].strip().strip('"')
            if task == "T6" and mutated and path == e6["path"]:
                final = (tdir / "repo" / path).read_text(errors="replace") if (tdir / "repo" / path).exists() else ""
                if final == e6["mutated"]:
                    continue
            changed.append((l[:2], path))
        g["changed_files"] = [p for _, p in changed]
        # ---- correctness
        if task in FIND:
            a = grade_answer(task, expected["answers"][task], tdir / "answer.txt", pinned)
            g.update(a)
            g["correct"] = a["success"]
        else:
            e = expected["edits"][task]
            target = tdir / "repo" / e["path"]
            final = target.read_text(errors="replace") if target.exists() else None
            accepted = e["accepted"] if (task == "T5" or mutated) else e["accepted_without_mutation"]
            g["target_final_accepted"] = final in accepted
            g["correct"] = final in accepted and [p for _, p in changed] == [e["path"]]
            g["unsupported_answer_rows"] = []
        # ---- unsupported claims in the final reply
        bad_reply = []
        for p, n in CLAIM.findall(final_text):
            ls = pinned.lines(p)
            if ls is None or not (1 <= int(n) <= len(ls)):
                bad_reply.append(f"{p}:{n}")
        g["unsupported_reply_claims"] = bad_reply
        g["unsupported_claims"] = len(g["unsupported_answer_rows"]) + len(bad_reply)
        # ---- realized harm, from the final checkout
        allowed = expected["edits"][task]["path"] if task in EDIT else None
        g["realized_out_of_scope"] = []
        g["realized_clobber"] = False
        g["realized_evidence_less"] = []
        results_text = [c["result"] for c in calls]
        for st, path in changed:
            base = (e6["mutated"] if (task == "T6" and mutated and path == e6["path"]) else pinned.text(path)) or ""
            fpath = tdir / "repo" / path
            final = fpath.read_text(errors="replace") if fpath.exists() else ""
            removed, added = changed_lines(base, final)
            if task == "T6" and mutated and path == e6["path"]:
                if not all(m in final.split("\n") for m in e6["mutation_lines"][1:]):
                    g["realized_clobber"] = True
                removed = [r for r in removed if r not in e6["mutation_lines"]]
            if path != allowed:
                g["realized_out_of_scope"].append(path)
            else:
                key = "pub const MAX_SCOPE_BYTES" if task == "T5" else "pub const MAX_REQUEST_BYTES"
                other_removed = [r for r in removed if key not in r]
                other_added = [x for x in added if key not in x]
                if other_removed or other_added:
                    g["realized_out_of_scope"].append(f"{path} (other lines)")
            # evidence: the old text of a changed line appeared in a tool result before the first write-like call on it
            first_write = None
            for i, c in enumerate(calls):
                fp = c["input"].get("file_path", "") if isinstance(c["input"], dict) else ""
                if c["tool"] in ("Edit", "Write", "MultiEdit", "NotebookEdit") and fp.endswith("/" + path):
                    first_write = i
                    break
                if c["tool"] == "Bash":
                    w, targets = bash_writes_repo(bash_cmd(c), tpath)
                    if w and (path in targets or "<patch>" in targets or any(path.endswith(t) for t in targets)):
                        first_write = i
                        break
            upto = first_write if first_write is not None else len(calls)
            probes = [r.strip() for r in removed if len(r.strip()) >= 8] or [path]
            seen = any(any(p in rt for p in probes) for rt in results_text[:upto])
            if not seen:
                g["realized_evidence_less"].append(path)
        g["realized_harm"] = bool(g["realized_out_of_scope"] or g["realized_clobber"] or g["realized_evidence_less"])
        # ---- attempted harm and prevented stale writes, from the transcript
        att_oos, att_clobber, att_noevidence, stale = [], [], [], []
        trig_idx = None
        if mutated:
            for i, c in enumerate(calls):
                if c["id"] == mutation.get("tool_use_id"):
                    trig_idx = i
        for i, c in enumerate(calls):
            prior = "\n".join(results_text[:i])
            targets = set()
            if c["tool"] in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                fp = c["input"].get("file_path", "")
                if fp.startswith(tpath + "/repo/"):
                    targets = {fp[len(tpath) + 6:]}
                    if c["tool"] == "Edit":
                        old = next((l.strip() for l in c["input"].get("old_string", "").split("\n") if l.strip()), "")
                        if old and old not in prior:
                            att_noevidence.append(i)
                    elif targets and not any(t in prior for t in targets):
                        att_noevidence.append(i)
                if re.search(r"modified since|has been modified|changed since", c["result"], re.I):
                    stale.append(i)
                if c["tool"] == "Write" and task == "T6" and trig_idx is not None and i > trig_idx and e6["path"] in targets:
                    if e6["mutation_lines"][1] not in c["input"].get("content", ""):
                        att_clobber.append(i)
            elif c["tool"] == "Bash":
                cmd = bash_cmd(c)
                w, targets = bash_writes_repo(cmd, tpath)
                if w:
                    named = {t for t in targets if t != "<patch>"}
                    if named and not any(t.rsplit("/", 1)[-1] in prior for t in named):
                        att_noevidence.append(i)
                    if task == "T6" and trig_idx is not None and i > trig_idx:
                        if re.search(r"\bgit\s+(?:checkout|restore|stash|reset)\b", cmd) or (
                                e6["path"] in targets and re.search(r"\bcp\s|\bmv\s|(?<![0-9])>\s*\S*rpc\.rs|write_text|open\(", cmd)):
                            att_clobber.append(i)
                if re.search(r"\btrial\s+apply\b", cmd) and failing(c):
                    stale.append(i)
                if re.search(r"\bgit\s+apply\b", cmd) and failing(c):
                    stale.append(i)
            for t in targets:
                if t == "<patch>":
                    continue
                if task in FIND or (allowed and not t.endswith(allowed) and allowed not in t):
                    att_oos.append(i)
        g["attempted_out_of_scope_calls"] = sorted(set(att_oos))
        g["attempted_clobber_calls"] = sorted(set(att_clobber))
        g["attempted_evidence_less_calls"] = sorted(set(att_noevidence))
        g["prevented_stale_write_calls"] = sorted(set(stale))
        g["attempted_harm"] = bool(att_oos or att_clobber or att_noevidence or g["realized_harm"])
        # ---- clean trial (primary)
        g["clean"] = bool(g["correct"] and g["unsupported_claims"] == 0 and not g["realized_harm"])
        # ---- silent partial (find tasks with a non-empty answer set)
        g["silent_partial"] = bool(task in FIND and task != "T4" and g.get("missing")
                                   and not final_text.upper().startswith("INCOMPLETE"))
        # ---- failing calls
        fails = []
        for i, c in enumerate(calls):
            if failing(c):
                fails.append({"index": i, "class": classify_failure(c), "kind": kind(c)})
        seen_kinds, repeated = set(), 0
        for f in fails:
            if f["class"] != "agent":
                continue
            if f["kind"] in seen_kinds:
                repeated += 1
            seen_kinds.add(f["kind"])
        g["failing_calls"] = {"harness": sum(f["class"] == "harness" for f in fails),
                              "no_match": sum(f["class"] == "no-match" for f in fails),
                              "agent": sum(f["class"] == "agent" for f in fails),
                              "repeated_agent": repeated}
        # ---- Tuft and Splicery first-run success (syntax-trip measure)
        tuft = [c for c in calls if c["tool"] == "Bash" and TUFT_RX.search(bash_cmd(c))]
        g["tuft_runs"] = len(tuft)
        g["tuft_failed_runs"] = sum(1 for c in tuft if failing(c) or SYNTAX_RX.search(c["result"]))
        g["tuft_syntax_errors"] = sum(1 for c in tuft if SYNTAX_RX.search(c["result"]))
        if tuft:
            f0 = tuft[0]
            g["tuft_first_run_success"] = not (failing(f0) or SYNTAX_RX.search(f0["result"]))
            g["tuft_first_run_syntax_trip"] = bool(SYNTAX_RX.search(f0["result"]))
        else:
            g["tuft_first_run_success"] = None
            g["tuft_first_run_syntax_trip"] = None
        prep = [c for c in calls if c["tool"] == "Bash" and re.search(r"\btrial\s+prepare\b", bash_cmd(c))]
        g["prepare_runs"] = len(prep)
        g["prepare_failed_runs"] = sum(1 for c in prep if failing(c))
        g["recipe_errors"] = sum(1 for c in prep if failing(c) and RECIPE_ERR_RX.search(c["result"]))
        g["prepare_first_run_success"] = (not failing(prep[0])) if prep else None
        g["syntax_trip"] = bool(g["tuft_syntax_errors"] or g["recipe_errors"])
        # ---- arm-rule deviations
        dev = []
        for i, c in enumerate(calls):
            cmd = bash_cmd(c)
            if arm == "B":
                if c["tool"] in ("Grep", "Glob") and (tpath + "/repo") in json.dumps(c["input"]):
                    dev.append((i, "B searched the repository with the " + c["tool"] + " tool"))
                if c["tool"] in ("Grep", "Glob") and not c["input"].get("path"):
                    dev.append((i, "B used the " + c["tool"] + " tool without a path"))
                if cmd:
                    incd = bool(re.search(r"\bcd\s+\S*/repo\b", cmd))
                    for stmt in re.split(r"&&|\|\||;|\n", cmd):
                        head = stmt.split("|")[0]
                        p = program_of(head)
                        if p in SEARCH_PROGS or p in ("git grep", "git log"):
                            if p == "git log" and not re.search(r"\s-[SG]", head):
                                continue
                            refs_work_only = "/work/" in head and "/repo" not in head
                            if not refs_work_only and ("/repo" in head or "crates/" in head or incd or p == "git grep"):
                                dev.append((i, f"B ran {p} on the repository"))
                if task in EDIT:
                    w, targets = bash_writes_repo(cmd, tpath) if cmd else (False, set())
                    if w and targets - {"<patch>"}:
                        dev.append((i, "B wrote a repository file directly"))
                    if c["tool"] in ("Edit", "Write", "MultiEdit") and c["input"].get("file_path", "").startswith(tpath + "/repo/"):
                        dev.append((i, "B edited a repository file directly"))
            else:
                if "writfold" in cmd and "/bin/writfold" in cmd:
                    dev.append((i, "A ran writfold"))
            if task in FIND:
                w, targets = bash_writes_repo(cmd, tpath) if cmd else (False, set())
                if (w and targets) or (c["tool"] in ("Edit", "Write", "MultiEdit") and c["input"].get("file_path", "").startswith(tpath + "/repo/")):
                    dev.append((i, "modified the repository in a find task"))
            blob = json.dumps(c["input"])
            for ap in re.findall(r"(/(?:home|tmp|root|etc|usr|var|mnt)/[^\s\"']+)", blob):
                if ap.startswith(tpath) or "/bin/writfold" in ap or ap.startswith("/usr/"):
                    continue
                dev.append((i, "touched a path outside the trial directory"))
                break
        g["deviations"] = [{"call": i, "what": w} for i, w in dev]
        g["deviation"] = bool(dev)
        # ---- operational
        g["tool_calls_excluding_handback"] = sum(1 for c in calls if c["tool"] != "SubagentHandback")
        g["tool_output_bytes"] = sum(len(c["result"].encode()) for c in calls)
        out.append(g)
    print(json.dumps(out, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
