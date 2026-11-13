# Tuft agent note

Use Tuft when a search needs coverage and copyable KB citations:

```sh
writfold tuft grep "exact bytes" --repo . --at HEAD --in 'crates/**/*.rs' --cite
```

Refs resolve once and the output shows the full commit. `--regex` uses the
bounded ASCII regex dialect. `--print-program` prints the actual pinned
program to stderr. `--limit N` bounds hits; `--json` emits the documented
[Tuft object](language.md#json-contract), not the KB envelope.

For a saved program, pin every `at` to a full commit:

```tuft
@h := find text "exact bytes" in "crates/" at "<full commit>" limit 100;
@e := retain @h around lines 0;
@v := project @e [path, line, excerpt(max_chars: 300)];
emit @v budget_tokens 4000;
```

`tuft run q.tuft --repo DIR` and `tuft check q.tuft --json` are strict.
One-shot `tuft run -` accepts refs; `--pinned` makes one-shot inputs strict.
Inside a KB, evidence state is automatic; otherwise use an initialized
`--state DIR`. An open KB run gets a retrieval receipt with full commit IDs.

Repeat `--repo NAME@REF` to search mapped repositories. Read each repository's
coverage and the overall line; resume one repository with its own cursor.
Only `coverage=complete`, `omitted=0`, no projection limit and `cursor=none`
mean every match is shown. Exit 0 also covers partial/empty results.

`--working-tree` includes uncommitted files. Dirty tokens and bindings are
explicitly non-pinned; do not present them as evidence at a commit.
`find def "NAME"` / `find refs "NAME"` with `--index FILE` imports ctags JSON
with a commit/digest sidecar. Symbol coverage is always partial: no result
does not prove no definition or caller. See the reference for index creation.
