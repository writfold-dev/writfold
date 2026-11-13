# External checkers

> **Status:** built in v0.5.0-dev.2 (phase 1: `check` and `make`, run by `stale` only). Not built: built-in kinds through the protocol, fetching, rendering or redaction, a server mode, an SDK, token migration, a checker together with a TTL, tracked check receipts, and a flag that fails when a checker is missing.

Writfold checks `code`, `gate`, `record` and `proposal` citations itself, from Git and the index. A citation of anything else (a file in another source-control system, a tracker item, a wiki page) is checked by time unless its kind names an external checker: a small program you install that Writfold asks whether each token still holds.

## Declaring a checked kind

One file per kind at the KB root, as for any [kind declaration](kb-layout.md#a-citation-token):

```toml
# .writfold/sources/src.toml
schema = "writfold-source/1"
kind = "src"
freshness = "checked"
checker = "acme-src"
fields = ["depot", "rev", "path", "L", "digest"]
required = ["depot", "path"]
description = "files in the acme-src depot"
```

| Key | Meaning |
|---|---|
| `freshness = "checked"` | tokens of this kind are asked of the checker; there is no lifetime, so `default_ttl` is refused |
| `checker` | a name, never a command: Writfold runs `writfold-check-<name>` found on `PATH` |
| `fields` | the fields the checker's `make` may write; `kind`, `seen`, `ttl`, `stale` and `v` belong to Writfold and cannot be listed |
| `required` | as for any kind: a token without one is `unverifiable` and is not sent |

The checker is found the way Git finds credential helpers: by name on `PATH`. The KB holds no command line, no URL and no secret; the checker brings its own configuration and credentials.

## Who runs it

Only `writfold kb stale` (without `--offline`) runs checkers: one process per kind per run, with every token of that kind in one batch. Every other reader, `gate status`, `gate cover`, guard rules, `why`, `brief`, the hooks and `proposals accept`, uses the verdict `stale` cached. A token `stale` has not checked yet reads `unverifiable: not-checked` there, so a gate cannot be covered by a citation nobody checked.

```text
$ writfold kb stale
cite src:lib/moved.c [moved] at checker:acme-src@1.4.0:r42 @notes/sources.md:5 !moved→lib/new.c:12-18
cite src:lib/gone.c [gone] at checker:acme-src@1.4.0:r42 said gone @notes/sources.md:9 !gone
cite src:lib/odd.c [unverifiable] at checker:acme-src@1.4.0 checker-failed (src): unknown status "stale" @notes/sources.md:13 !unverifiable
# coverage=complete records=4/4 shown=3 omitted=0 checked=4 stale=2 unverifiable=1 kb=… index=fresh cursor=none q=4b50c68a7933
```

`at checker:<name>@<version>:<at>` is what the checker reported checking against. Exit codes are those of `stale`: 0 when every citation is fresh, 2 when any is not (an unverifiable one included).

When the checker is not on `PATH`, its tokens are `unverifiable: checker-absent (<kind>)`, never fresh, and the trailer counts them:

```text
cite src:lib/fresh.c [unverifiable] at checker:acme-src checker-absent (src) @notes/sources.md:3 !unverifiable
# note: a checked kind's checker is not on PATH; `writfold kb checker list` names it
# coverage=complete records=4/4 shown=1 omitted=0 checked=1 stale=0 unverifiable=1 checker-absent=1 kb=… index=fresh cursor=none q=4b50c68a7933
```

The session-start hook names a missing checker in one line (a `PATH` lookup; nothing is run). `writfold kb checker list` shows one row per checked kind, running each checker's `describe`:

```text
$ writfold kb checker list
checker src [found] writfold-check-acme-src acme-src 1.4.0 ops check,make token versions [1]
```

## The protocol: `writfold-checker/1`

Newline-delimited JSON on stdin and stdout. Writfold writes a `describe` request, then one request per token, then closes stdin; the checker answers `describe` first, then one reply per request `id`, in any order, and exits 0. Only the token's own fields are sent: no clock, no KB path, no other token.

```text
→ {"v":1,"op":"describe"}
← {"v":1,"name":"acme-src","version":"1.4.0","kinds":["src"],"ops":["check","make"],"token_versions":[1]}
→ {"v":1,"op":"check","id":"0","kind":"src","fields":{"depot":"main","path":"lib/a.c","L":"3-9","rev":"r40","digest":"9f2c","v":"1","seen":"2026-11-12T09:00Z"}}
← {"v":1,"id":"0","status":"moved","at":"r42","moved":{"path":"lib/b.c","first":12,"last":18},"detail":"file renamed"}
```

`status` is one of `fresh`, `moved` (with `moved.first` and `moved.last`, and `moved.path` when the file moved), `changed`, `gone` or `unverifiable`; `detail` is shown with the verdict.

None of these failures can make a token fresh; each leaves the affected tokens `unverifiable` with `checker-failed (<kind>): <reason>`:

| What happened | Tokens affected |
|---|---|
| the deadline passed (60 s; `WRITFOLD_CHECKER_TIMEOUT` seconds overrides it), a non-zero exit, a signal | every token of the batch; the process group is stopped with TERM, then KILL, and nothing it started outlives it |
| a line that is not a JSON object, has no `"v":1` or no string `id`, or answers an id never sent | every token of the batch |
| `describe` speaks another protocol version, does not list the kind, or does not list `check` | every token of the batch |
| no reply, more than one reply, an unknown status, `moved` without a valid range | that token |
| a token whose `v=` is not in `token_versions` | that token, as `token-version (v=2; writfold-check-acme-src reads [1])` |

### Making a token: `cite make --kind`

```text
$ writfold kb cite make --kind src depot=main path=lib/a.c L=3-9
cite src:lib/a.c ⟦cite kind=src depot=main rev=r42 path=lib/a.c L=3-9 digest=9f2c v=1 seen=2026-11-13T10:00Z⟧
# note: made by writfold-check-acme-src
```

Writfold sends `{"v":1,"op":"make","id":"0","kind":"src","fields":{"L":"3-9","depot":"main","path":"lib/a.c"}}`; the reply carries `"fields":{…}`, string values only. A returned field the kind file does not list in `fields` (other than `v`) is refused, exit 3, as is a reply missing a `required` field or a `v` outside `token_versions`. Writfold adds `kind=` and `seen=` itself, orders the fields as `fields` lists them, and prints the token; nothing is written.

## Versions

Built-in kinds carry no `v=`: their format is pinned by Writfold's token codec and index schema. A checked kind's token may carry `v=<n>` (absent means 1), the format version of the checker's own fields. The checker's `make` sets it, and `describe` lists the `token_versions` it reads; a token outside that list is `unverifiable: token-version`. Writfold never rewrites a tracked token: migrating is running `cite make` again and committing the diff.

## A minimal checker

A checker for a kind `doc` that cites files on a shared documentation tree by content digest. Install it as `writfold-check-shared-docs` on `PATH` and declare the kind with `checker = "shared-docs"` and `fields = ["path", "digest"]`.

```python
#!/usr/bin/env python3
"""writfold-check-shared-docs: kind=doc tokens against files under $DOCS_ROOT."""
import hashlib, json, os, sys

ROOT = os.environ.get("DOCS_ROOT", "/srv/docs")

def digest(path):
    with open(os.path.join(ROOT, path), "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()[:16]

def answer(req):
    if req["op"] == "describe":
        return {"v": 1, "name": "shared-docs", "version": "0.1.0", "kinds": ["doc"],
                "ops": ["check", "make"], "token_versions": [1]}
    reply, fields = {"v": 1, "id": req["id"]}, req["fields"]
    try:
        now = digest(fields["path"])
    except (KeyError, FileNotFoundError):
        return {**reply, "status": "gone"} if req["op"] == "check" else reply
    if req["op"] == "make":
        return {**reply, "fields": {"path": fields["path"], "digest": now}}
    return {**reply, "status": "fresh" if fields.get("digest") == now else "changed", "at": now}

for line in sys.stdin:
    print(json.dumps(answer(json.loads(line))), flush=True)
```

A `make` for a missing file answers without `fields`, which Writfold refuses.
