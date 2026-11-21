# Project asset catalogs

> **Status:** built in v0.6.0-dev.15 ([#84](https://github.com/writfold-dev/writfold/issues/84), [#86](https://github.com/writfold-dev/writfold/issues/86), [RFC](../rfcs/2026-11-21-project-asset-catalogs.md)). What may be written is proved in Lean (`proofs/Writfold/Catalog.lean`); reading, comparing and patching are tested.

A team that already keeps its equipment, services or dashboards in a JSON file can declare **where that inventory is and how its fields map**, and Writfold turns it into a reviewable Git patch of ordinary asset records (`inventory/assets/<id>.json`). `find`, `context`, `record` and `propose` then use the reviewed records exactly as they use hand-written ones. Nothing is written for you, nothing is fetched, no producer code runs, and an import grants no evidence or freshness.

## Declaring a catalog

`.writfold/catalogs/brewery.toml` (the name is the file stem, a slug):

```toml
schema = "writfold-asset-catalog/1"
input = ".writfold/catalog-inputs/equipment.json"
items = "/equipment"

[fields]
id = "/tag"
name = "/label"
kind = "/classification"
aliases = "/search/names"
```

- **`input`** is one Git-tracked JSON file, or a directory of directly contained tracked `.json` files with one object each. Staged files count; an untracked `.json` in a declared directory is a fault, not skipped. The path must be inside the KB, with no `..`, no symbolic link, and not under `.git`, `inventory/` or `.writfold/catalogs/`.
- **`items`** (single file only) selects the array of objects with a [JSON Pointer](https://www.rfc-editor.org/rfc/rfc6901#section-4); `""` is the root array. Without it the file is one object.
- **`[fields]`** maps each record field to a pointer within one object: `id` (required, a slug), and optionally `name`, `kind`, `owner`, `lifecycle`, `locator`, `verified_at`, `ttl`, `notes` (strings), `aliases`, `projects` (arrays of strings). A mapped pointer that is missing or holds the wrong type is a fault; there are no defaults, transformations or inferred dates. `~1` escapes a `/` in a key.

`kb lint` checks declarations like every other definition file.

## Reviewing and applying

```text
$ writfold kb catalog check
asset mix-station [create] catalog brewery would create inventory/assets/mix-station.json @inventory/assets/mix-station.json
asset chiller [create] catalog brewery would create inventory/assets/chiller.json @inventory/assets/chiller.json
# note: catalog brewery: drift
$ writfold kb catalog diff brewery > brewery.patch
$ git apply --check brewery.patch && git apply brewery.patch
$ git add -A && git commit -m "assets: brewery catalog"
$ writfold kb catalog check     # exit 0: clean
```

- `catalog diff NAME` prints only a standard Git patch (each affected file whole, sorted by path, LF, no timestamps), exit 0, empty when there is nothing to change. `--json` gives one row per affected asset instead. Two clones of the same commit print the same bytes.
- `catalog check [NAME]` reports the same rows without a patch: exit **0** clean, **2** drift, **3** invalid or conflicting. `kb check` includes it when a catalog is declared or a managed record exists; a KB without catalogs reads as before.
- Each generated record carries `"writfold_catalog": {"name": …, "source": …}`. Reordering the input changes nothing; editing a managed record by hand shows as drift, and the next patch shows the exact lines it would undo.

## Ownership and failure

A catalog **creates** a record only where nothing is, **replaces or removes** only records it generated, and never writes anything else. Wanting an ID or path that something else holds (a hand-written asset, another catalog's record, any record or alias that already answers to that ID) is a **conflict**. Any of these makes the plan invalid, and an invalid plan prints **no patch at all**, not even the part that was fine:

- the input or a declaration is missing, unreadable, untracked, not UTF-8, invalid JSON, holds a duplicate key, or a field of the wrong type;
- a directory with no tracked `.json` file (an empty inventory is an explicit `[]` in one file, never an empty folder);
- the same ID twice, a conflict, or another catalog that is itself invalid (no catalog patches while another cannot be checked);
- a managed record whose catalog is no longer declared: empty the catalog's input and apply its removal patch before removing the declaration, or delete the `writfold_catalog` marker to keep the record by hand;
- an input or record that changed while it was read.

Removing an asset is deliberate: drop its row (or give `[]`), and the patch deletes the record. A missing or broken input never does.

## Other formats

Writfold reads committed JSON only. A team with CSV, YAML, a database or an API exports JSON on its own side and commits it. [`recipes/csv-to-catalog.py`](recipes/csv-to-catalog.py) is one such exporter (a `names[]` column becomes a list), run by `crates/writfold/tests/kb_catalog.rs`:

```sh
python3 docs/guide/recipes/csv-to-catalog.py equipment.csv > .writfold/catalog-inputs/equipment.json
git add .writfold/catalog-inputs/equipment.json && writfold kb catalog diff brewery
```

Writfold checks the committed input against the records; it does not know whether the export was complete or current.

## Older binaries

The records are ordinary asset files: a binary without catalog support (checked with v0.6.0-dev.14) lists and resolves them and ignores the marker, but cannot run `catalog check`; use a supporting binary in CI to catch drift.
