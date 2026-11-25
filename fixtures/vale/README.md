# Vale stand-in

`bin/vale` is a stand-in for [Vale](https://vale.sh)'s command line, used
by Writfold's tests of `prose.vale` (#117, #133). **It is not Vale** and
does not lint anything. It prints hand-written reports in the shape of
`vale --output=JSON --counts`, as read from Vale 3.28.1's source:

```json
{"files": {"document.md": [{"Check": "House.Todo", "Severity": "error", "Line": 2, "Span": [1, 4], "Message": "No TODO.", ...}]},
 "counts": {"House.Hedges": 0, "House.Todo": 0, "House.Terms": 0}}
```

Writfold removes every `VALE_*` variable before it runs Vale, so the
stand-in takes its behaviour from the first line of the document it is
given: `<!-- standin: MODE [version=X.Y.Z] [log=FILE] -->`. The modes are
`clean`, `advice`, `errors`, `no-coverage`, `none-enabled`,
`config-error`, `garbage`, `cut`, `flood`, `exit-disagrees`,
`unknown-severity`, `stray-path`, `slow`, `crash`, `many` and `absent`
(its `--version` exits 127). With `log=FILE` it writes its arguments and
environment to FILE.

Whether real Vale prints these shapes is checked by a conformance test
against the pinned Vale, not by this stand-in.
