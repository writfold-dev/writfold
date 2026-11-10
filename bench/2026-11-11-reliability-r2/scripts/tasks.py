"""R2 candidate tasks: text, family, kind, and the hidden answer spec.

Imported by expected.py (which resolves every spec against the pinned commit
and asserts it), prompts.py (task text) and grade.py (kinds).

Kinds:
  find     answer.txt rows `<path>:<line>: <text>`; graded against the pinned commit.
  edit     final checkout graded: each listed (path, line) has `old` replaced by
           `new` once; nothing else changes except a harness mutation, which
           must survive. `reply` REFUSED (C06) is required as the first word of
           the final reply when the mutation fired; if it never fired, C06 is
           graded as the unmutated task (`unmutated_edits`, value 30).
  session  answer.txt rows prefixed `S<n> ` for the find steps, plus an edit
           (as for `edit`), plus an `S5` row: the edited line as it reads in
           the final checkout.

Mutations (mutate.py): when the agent's own transcript first shows a tool
call or result containing `trigger`, the harness changes the checkout file
`path` in place, once: `insert_after` pinned line n inserts `lines`, or
`replace_line` n replaces that line's `old` with `new`. `renamed` (C08) is
the form the inserted lines must take in the final checkout.
"""

COMMIT = "b5addb64f0161ff6bfe94c124ef76f6a1fba5254"
URL = "https://github.com/encode/httpx.git"

FAMILIES = {
    "F1": "unfamiliar-repository structural find",
    "F2": "multi-file edit with look-alike decoys",
    "F3": "file mutated by the harness mid-task",
    "F4": "ambiguous scope (respect the boundary)",
    "F5": "long session (5 dependent steps)",
}

TASKS = {
 "C01": {"family": "F1", "kind": "find",
  "text": "List the `def` line (the line containing `def`, not a decorator line) of every method defined directly in class `AsyncClient` in `httpx/_client.py` whose name is not also the name of a method defined directly in class `Client` in the same file.",
  "spec": {"ast_methods_not_in": ["httpx/_client.py", "AsyncClient", "Client"]}},
 "C02": {"family": "F1", "kind": "find",
  "text": "List the `class` definition line of every exception class in the package directory `httpx/` that inherits, directly or indirectly, from `TransportError`. Do not include `TransportError` itself.",
  "spec": {"ast_subclasses": ["httpx/_exceptions.py", "TransportError"]}},
 "C03": {"family": "F2", "kind": "edit", "reply": "DONE",
  "text": "Change httpx's default timeout from 5 seconds to `8.0` seconds in two places: the library-wide default timeout configuration in the package, and the command-line client, where both the `--timeout` option's default value and every help text that states that default must say `8.0`. Change nothing else, even where it also says `5.0` (for example docstring examples, the keep-alive expiry, tests or documentation).",
  "spec": {"edits": [["httpx/_config.py", 246, "timeout=5.0", "timeout=8.0"],
                     ["httpx/_main.py", 84, "[Default: 5.0]", "[Default: 8.0]"],
                     ["httpx/_main.py", 403, "default=5.0", "default=8.0"],
                     ["httpx/_main.py", 406, "[Default: 5.0]", "[Default: 8.0]"]]}},
 "C04": {"family": "F2", "kind": "edit", "reply": "DONE",
  "text": "Rename the helper function `primitive_value_to_str` (defined in `httpx/_utils.py`) to `primitive_to_str`: change its definition and every import and call of it in the package directory `httpx/`. Change nothing else.",
  "spec": {"edits": [[p, n, "primitive_value_to_str", "primitive_to_str"] for p, n in [
      ("httpx/_utils.py", 15), ("httpx/_urls.py", 10), ("httpx/_urls.py", 459), ("httpx/_urls.py", 549),
      ("httpx/_urls.py", 564), ("httpx/_content.py", 26), ("httpx/_content.py", 142), ("httpx/_content.py", 144),
      ("httpx/_multipart.py", 20), ("httpx/_multipart.py", 87)]]}},
 "C05": {"family": "F2", "kind": "edit", "reply": "DONE",
  "text": "Both httpx clients log each request with a message that begins `HTTP Request:`. Change that prefix to `HTTP Exchange:` in both clients, and update the tests under `tests/` that assert on that log text so they expect the new prefix. Do not change documentation or anything else.",
  "spec": {"edits": [["httpx/_client.py", 1026, "HTTP Request:", "HTTP Exchange:"],
                     ["httpx/_client.py", 1741, "HTTP Request:", "HTTP Exchange:"],
                     ["tests/test_utils.py", 63, "HTTP Request:", "HTTP Exchange:"],
                     ["tests/test_utils.py", 78, "HTTP Request:", "HTTP Exchange:"],
                     ["tests/test_utils.py", 84, "HTTP Request:", "HTTP Exchange:"]]}},
 "C06": {"family": "F3", "kind": "edit", "reply": "REFUSED",
  "text": "Raise httpx's default maximum number of redirects from 20 to `30` by changing the value of the constant that defines it. Precondition: make the change only if, at the moment you make it, that definition in the checkout still sets the value 20. If it holds any other value, make no change and reply `REFUSED: <reason>`.",
  "spec": {"edits": [], "unmutated_edits": [["httpx/_config.py", 248, "= 20", "= 30"]],
           "mutation": {"path": "httpx/_config.py", "trigger": "_config.py",
                        "replace_line": 248, "old": "DEFAULT_MAX_REDIRECTS = 20", "new": "DEFAULT_MAX_REDIRECTS = 25"}}},
 "C07": {"family": "F3", "kind": "edit", "reply": "DONE",
  "text": "Raise the default maximum number of keep-alive connections in httpx's default connection limits from 20 to `40`. Change only that value.",
  "spec": {"edits": [["httpx/_config.py", 247, "max_keepalive_connections=20", "max_keepalive_connections=40"]],
           "mutation": {"path": "httpx/_config.py", "trigger": "_config.py", "insert_after": 245,
                        "lines": ["# Defaults below are shared by Client, AsyncClient and the transports."]}}},
 "C08": {"family": "F3", "kind": "edit", "reply": "DONE",
  "text": "Rename the helper function `peek_filelike_length` (defined in `httpx/_utils.py`) to `peek_length`: change its definition and every import and use of it in the package directory `httpx/`, so that no use of the old name remains there. Change nothing else.",
  "spec": {"edits": [[p, n, "peek_filelike_length", "peek_length"] for p, n in [
      ("httpx/_utils.py", 95), ("httpx/_content.py", 26), ("httpx/_content.py", 121),
      ("httpx/_multipart.py", 19), ("httpx/_multipart.py", 177)]],
           "mutation": {"path": "httpx/_multipart.py", "trigger": "_multipart.py", "insert_after": 300,
                        "lines": ["", "", "def _file_length(file: typing.Any) -> int | None:",
                                  "    return peek_filelike_length(file)"],
                        "renamed": ["", "", "def _file_length(file: typing.Any) -> int | None:",
                                    "    return peek_length(file)"]}}},
 "C09": {"family": "F4", "kind": "edit", "reply": "DONE",
  "text": "In class `AsyncClient` only, change the log message format string that begins `'HTTP Request: ` so that it begins `'HTTP Async Request: ` instead. Leave every other occurrence (in class `Client`, tests and documentation) unchanged.",
  "spec": {"edits": [["httpx/_client.py", 1741, "HTTP Request:", "HTTP Async Request:"]]}},
 "C10": {"family": "F4", "kind": "edit", "reply": "DONE",
  "text": "httpx defines its own helper function `unquote` in `httpx/_utils.py`. Rename that helper to `unquote_header_value`, and update every use of that helper in the package directory `httpx/`. Other functions that are also called `unquote` (for example the one imported from `urllib.parse`) are not this helper: leave them and their uses unchanged.",
  "spec": {"edits": [["httpx/_utils.py", 91, "def unquote(", "def unquote_header_value("],
                     ["httpx/_auth.py", 13, "to_str, unquote", "to_str, unquote_header_value"],
                     ["httpx/_auth.py", 240, "= unquote(value)", "= unquote_header_value(value)"]]}},
 "C11": {"family": "F5", "kind": "session", "reply": "DONE",
  "text": ("This task has five steps; later steps depend on the exact results of earlier ones.\n"
           "Step 1. Find the line that defines the module-level constant holding httpx's default maximum number of redirects. Write it as one row `S1 <path>:<line>: <text>`.\n"
           "Step 2. Using that constant's name from step 1, list every other line in the package directory `httpx/` that mentions that name (imports included), one row each, as `S2 <path>:<line>: <text>`.\n"
           "Step 3. One of the step-2 lines gives the default value of the `max_redirects` parameter of class `AsyncClient`'s constructor. Write that line again as `S3 <path>:<line>: <text>`.\n"
           "Step 4. In class `AsyncClient`'s method `_send_handling_redirects` only, change the comparison `len(history) > self.max_redirects` to `len(history) >= self.max_redirects`. Leave the same comparison in class `Client` unchanged.\n"
           "Step 5. Write the line you changed in step 4, as it now reads in the checkout, as `S5 <path>:<line>: <text>`."),
  "spec": {"steps": {"S1": [["httpx/_config.py", 248]],
                     "S2": [["httpx/_client.py", 16], ["httpx/_client.py", 198], ["httpx/_client.py", 656], ["httpx/_client.py", 1369]],
                     "S3": [["httpx/_client.py", 1369]]},
           "edits": [["httpx/_client.py", 1686, "len(history) > self", "len(history) >= self"]],
           "s5": ["httpx/_client.py", 1686]}},
 "C12": {"family": "F5", "kind": "session", "reply": "DONE",
  "text": ("This task has five steps; later steps depend on the exact results of earlier ones.\n"
           "Step 1. Find the line that defines the exception class `TooManyRedirects`. Write it as one row `S1 <path>:<line>: <text>`.\n"
           "Step 2. Find the line that defines the class that `TooManyRedirects` directly inherits from. Write it as `S2 <path>:<line>: <text>`.\n"
           "Step 3. List the class definition line of every class in the same file that directly inherits from the step-2 class (`TooManyRedirects` included), one row each, as `S3 <path>:<line>: <text>`.\n"
           "Step 4. In class `AsyncClient` only, change the message `\"Exceeded maximum allowed redirects.\"` passed to `TooManyRedirects` to `\"Exceeded maximum allowed redirects (async client).\"`. Leave class `Client`'s message unchanged.\n"
           "Step 5. Write the line you changed in step 4, as it now reads in the checkout, as `S5 <path>:<line>: <text>`."),
  "spec": {"steps": {"S1": [["httpx/_exceptions.py", 249]],
                     "S2": [["httpx/_exceptions.py", 107]],
                     "S3": [["httpx/_exceptions.py", 123], ["httpx/_exceptions.py", 243], ["httpx/_exceptions.py", 249]]},
           "edits": [["httpx/_client.py", 1688, "allowed redirects.\"", "allowed redirects (async client).\""]],
           "s5": ["httpx/_client.py", 1688]}},
}
