# Tuft agent note

The tool note an agent prompt carries for Tuft, for the next projection pilot
([the first one](../../bench/2026-11-09-projection-pilot/README.md) found the
earlier, 1,290-byte note a third of Tuft's token gap). It is 594 bytes, and it
replaces the quick reference in the arm-B prompts; `<trial>`, `<commit>` and
`<writfold>` are filled in by the harness. Everything else is in
[the language reference](language.md), which the note does not cite, because
no pilot agent opened it.

```text
Search with Tuft. Write <trial>/work/q.tuft:
  @h := find text "exact bytes" in "crates/" at "<commit>" limit 100;
  @e := retain @h around lines 0;
  @v := project @e [path, line, excerpt(max_chars: 300)];
  emit @v budget_tokens 4000;
(`find rx"..."`: Rust regex over ASCII; omit `in` for the whole tree.)
Run <writfold> tuft run q.tuft --repo <trial>/repo --state <trial>/state
Rows are `path:line: text`; --out FILE writes them to a new file.
The last line `# coverage=complete ... omitted=0 cursor=none` means none
are missing; else raise limit or budget_tokens, or rerun with --cursor C.
```

The byte count is of the text inside the fence, with placeholders unfilled.
Rows in this form already match the pilot's answer format, so an agent can
hand the `--out` file over (or filter it) instead of retyping lines.
