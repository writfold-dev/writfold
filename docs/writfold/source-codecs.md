# Pure authored source codecs

`writfold-sources` implements document/frontmatter and citation syntax plus the exact authored material-identity projection, with no filesystem, Git, process, HTTP, renderer, source-resolution, clock or authority effects. It does not claim source acquisition, citation freshness or product release qualification. The immutable references are `src/kctx/codecs/documents.py`, `frontmatter.py`, `src/kctx/citations/syntax.py` and `src/kctx/services/proposals.py::material_revision`.

## Documents

`documents::parse_document` owns one UTF-8 `String`. `MarkdownDocument` has private fields, no mutable views and no unchecked deserializer. `text`/`as_bytes` return original bytes; `frontmatter`, `sections`, `field`, `section` and line iterators borrow content. `field_span` and `section_span` return UTF-8 **byte** ranges selecting the same content as Python's codepoint ranges. No newline or Unicode normalization happens on parsing. Non-UTF-8 byte decoding is the caller's separate responsibility.

A document starts with a Python-whitespace-stripped `---` line and ends frontmatter at the next such line. A non-comment line containing `:` contributes its stripped prefix as key and stripped suffix as value; all other material remains untouched. Empty keys are historically parseable. Duplicate stripped keys are refused. Unknown fields, comments, blank lines, preambles, unknown sections and fenced examples are retained.

Sections match the reference `^##\s+(.+?)\s*#*\s*$` on lines stripped of trailing CR/LF, outside fences. Fence openers have zero to three ASCII spaces and three or more identical backticks or tildes. A backtick opener's suffix cannot contain a backtick. A closer has the same marker, at least the opening width and only Python whitespace afterward. Fences and their contents do not create sections. Repeated decoded section names are refused, even in unfinished drafts. A section span starts after its heading and ends at the next recognized heading or EOF.

Parsing is not admission: `require` or `parse_document_required` explicitly requests nonempty required fields and present required sections. Empty required sections are allowed. The default parser accepts unfinished drafts.

`update_fields` takes an ordered slice of `(key, Option<value>)`; `None` clears a value, not its line. Existing values are replaced in their exact spans; new keys are appended before the closing delimiter in caller order. Duplicate input changes are refused because Rust slices, unlike Python mappings, can represent ambiguous repeated keys. Patches are applied to one allocated output in a single forward pass and reparsed. Tuple ordering retains the reference tie-break for coincident zero-width spans.

`update_section` replaces only a nonempty section's stripped content, retaining its surrounding whitespace. Empty sections retain their historical prefix/suffix behavior. Missing sections append with the reference blank-line separator. `append_section` inserts without removing previously authored bytes, including trailing spaces and unknown sections. Replacement/append content converts CRLF/CR/LF to the document's first CRLF, LF or CR sequence (default LF). Other Unicode line separators are not converted. `render_document` uses LF and caller field/section order, strips section content and reparses; it is a new rendering, not a whole-document rewrite operation.

Two historical distinctions are intentional: `body` finds the first CRLF/LF/CR after the closing frontmatter delimiter, while `MarkdownDocument::unfenced_lines` starts after the first **LF** after that delimiter. Therefore a CR-only document has a body but that particular line view is empty. The standalone `unfenced_lines` uses Python splitlines semantics directly.

### New single-line mutations and historical parsing

WF-TEXT-001 distinguishes new authoring from reading retained bytes. Field keys/values, inserted section names and rendered headings/titles reject CR, LF, U+000B, U+000C, U+001C–U+001E, U+0085, U+2028 and U+2029: every boundary recognized by Python `splitlines`. The former CR/LF-only guard allowed a value such as `a\u2028injected: b` to create a second metadata field. Existing documents retain those original separators and parsed fields, and unrelated edits do not normalize or rewrite them. Section bodies remain multiline. U+001F remains whitespace, not a line boundary.

## Flat frontmatter and lists

`frontmatter::parse_frontmatter` permits absent frontmatter, but routes a present block through duplicate-aware document parsing. It reproduces the reference's splitlines-and-LF-join **read-only projection**, stopping at the closing delimiter; it does not inspect duplicate headings in the rest of the document. `read_frontmatter` returns caller-owned ordered entries, while `FrontmatterBlock` offers immutable borrowed views.

`split_commas_respecting_brackets` returns trimmed borrowed list items, discarding empty items. Nested `⟦…⟧` and double-quoted spans protect commas. Quote tracking is disabled when the total double-quote count is odd, while bracket tracking remains active. There are no backslash escapes. `split_list_value` requires outer ASCII brackets after Python whitespace stripping.

## Authored material identity

`material::material_revision(&MarkdownDocument)` returns either
`MaterialIdentity::Legacy(ProposalMaterialRevision)` or
`MaterialIdentity::Design(DesignRevision)`. Absent workflow version means the
historical workflow-2 projection; explicit `"3"` selects `design-v3:<sha256>`.
Unsupported workflow versions are refused. This is a read-only identity
projection, not full proposal validation, acceptance or implementation binding.

The projection exactly preserves the reference's sorted frontmatter map,
preamble and ordered parsed sections. Lifecycle bookkeeping and `Decision log`
are excluded; target-revision and binding fields are excluded **only** for
workflow 3. Unknown workflow-2 metadata remains material, even if its key later
became a binding field. Preamble/section CRLF and CR are normalized to LF;
original document bytes are never rewritten. The preamble cutoff follows the
historical first unfenced `##`-plus-whitespace rule: this is not a whole-file
revision or a replacement for authored-record validation.

Hashing streams Python-compatible `json.dumps(sort_keys=True,
ensure_ascii=False)` bytes, including its default comma/colon spacing. It does
not reuse compact signing JSON. The frozen legacy digest remains
`1947884be0e451a4959e78aef25b81ebf36cb09e4138b7d937166f8181d5e453`;
the adopted fixture's new `design-v3:` identity cannot equal its old unprefixed
hash. Six regressions and a 30-case Python/Rust static-consumer campaign exercise
these boundaries. Concrete bind/reopen/CAS authority remains P08D/P07 work.

## Citations

`parse_cite_body` follows Python `FIELD_RE.finditer`, not a stricter whole-body parser:

```text
(\w+)=(?:"([^"]*)"|([^\s⟧]+))
```

Keys use Python 3.12 Unicode `\w`. Quoted values retain every character between quotes, including whitespace; there is no escaping or Unicode normalization. Unknown extension fields retain insertion order. Malformed surrounding material is ignored as in the reference. An unmatched opening quote can be part of the unquoted alternative. Duplicate exact decoded keys are refused, not overwritten. `CiteFields` borrows keys/values and exposes exact whole-field byte spans. Canonically equivalent Unicode spellings remain distinct keys.

`iter_cite_tokens` scans every literal `⟦cite` opener, without requiring a word boundary. A token ends at the first closing `⟧`, next opener, or EOF, whichever applies first. Missing closers produce `complete=false` and never swallow the next opener. Raw/body text and occurrence ranges are borrowed. Physical line numbers are one plus the number of **LF bytes**, not Unicode splitlines counts. No scanner excludes illustrative tokens, unknown Markdown sections or fenced examples automatically.

`find_cite_lines` separately preserves the historical complete-token regex `⟦cite(?P<body>.*?)⟧` with DOTALL: its body can contain another opener before the next closer. The replacement/removal helper's historical `fullmatch` can backtrack through internal closing delimiters to a final closer; it is not a stricter single-occurrence validator. Both distinctions are behavioral regressions, not accidental undocumented differences.

`is_illustrative` recognizes `^\s*illus\b`, using Python word/whitespace boundaries. It is independent of field parsing; an ordinary field containing `illus` is not illustrative.

`format_cite` emits requested-order keys first, once each, then remaining keys in input order. Missing/empty values are omitted. Duplicate slice keys are refused. Keys must match `\w+`, and values cannot contain `⟧`. Whitespace-containing, empty or double-quote-prefixed values require double quoting; embedded double quotes then cause refusal. An unquoted value with an interior double quote remains historically representable. `replace_field` supports explicit empty values, preserves all unrelated bytes and appends missing fields after trimming trailing Python whitespace. `remove_field` also removes preceding Python whitespace, never the opener. Neither operation changes any other field.

`parse_range` accepts exactly two nonempty Unicode-decimal runs separated by ASCII `-`, with mathematical first ≥ 1 and last ≥ first. Signs, trailing line breaks, extra separators and non-decimal numeric characters are refused. `LineNumber` stores canonical arbitrary-length ASCII digits, so valid ranges are not narrowed to machine integers. `to_usize` is an explicit checked consumer conversion. Python's interpreter-configurable decimal conversion digit ceiling is not adopted as a line-range width limit; the requested unbounded syntax contract includes a 5,000-digit regression. This changes no authored raw token bytes.

`normalize_span` strips each line with Python whitespace semantics, discards blank lines, streams LF-separated remaining bytes into SHA-256 and returns its first sixteen lowercase hexadecimal characters. `find_unpinned` identifies only the historical `@(main|HEAD|master):` warning text; it does not resolve anything.

No parse, format, edit, list operation or hash operation creates a new `seen`, `checked_at`, source revision, blob identity, range or observation. Callers must explicitly request a specific field edit. Historical `rev`, `blob`, `span`, `L` and `seen` remain untouched when an unrelated extension/advisory field changes.

## Unicode provenance and execution status

The handwritten Rust algorithms use generated Python 3.12 / Unicode 15.0.0 tables for `\w`, `\d`, `str.isspace` and splitlines boundaries. In particular, Python includes U+001C–U+001F in whitespace, but only U+001C–U+001E in splitlines; combining marks are not generally word characters, while some numeric characters such as superscripts are. No dependency on the Rust compiler's changing Unicode version or regex crate classes supplies those predicates.

`crates/writfold-sources/tools/generate_unicode.py` is the retained generator; `unicode-provenance.json` records its SHA-256, output SHA-256, actual interpreter version and admitted image digest. Generation was performed with Python 3.12.3, Unicode 15.0.0, network disabled, using `writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6`. This was **data generation only**, not compatibility qualification. The generator is not a runtime dependency.

The behavioral tests in `crates/writfold-sources/tests/{documents,citations}.rs` cover exact retained slices, CRLF/Unicode/fences, ambiguous keys/headings, ordered extensions, mathematical ranges and preservation of historical citation fields. A locally built static musl consumer exercised safe targeted editing, retained historical Unicode/CRLF bytes and refusal of all eleven line-boundary spellings with `PATH=/nonexistent`; logs are retained under `~/.local/share/writfold/qualification/p02-integration-001/`. Full gate and product qualification remain separate; consult `qualification/status.json`.
