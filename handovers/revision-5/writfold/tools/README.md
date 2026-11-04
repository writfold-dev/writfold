# Handover regeneration

Python 3.12, ReportLab 4.4.9 and Poppler were used for this atlas. Bundled DejaVu fonts and their license are in fonts/. `render_plan.py` generates stage Markdown/Mermaid from registries/stages.json. `render_assets.py` generates SVG/PNG diagrams, SVG icon variants, review-atlas.pdf and index.html without network access. These are authoring tools, not the future Rust product gate runner.

After a contract change, update prose, registries, meaningful diagram topology and atlas explanation together. Run both renderers, render the PDF with Poppler, inspect every page and update evidence/atlas-build.json and evidence/atlas-review.json against exact bytes. Regenerate MANIFEST.sha256 last, then run check_bundle.py. The checker detects stale/missing hashes and page coverage; it does not inspect visual fidelity or prove the written meaning.

The reference archive and historical documents retain their original internal paths. Their links resolve inside the original repo at the recorded commit.

Revision 3: `python3 tools/check_sample_vault.py` validates the sample manifest, links and scoped query templates. It does not execute Obsidian or DQL. `render_view_pages.py` now owns the four view pages and delegates the two storage pages to `render_storage_pages.py`. The storage probe is in `qualification/storage/compare.py`.

Revision 4: `render_storage_pages.py` owns backend decision/history pages; `versioned_schema.py` is the new native probe. Full epoch qualification is P01O, not a document-check pass.

Revision 5: render_engineering_pages.py adds ten vector atlas chapters; render_assets.py reads all 25 stages. check_engineering_examples.py validates the illustrative codec fixtures without writing bytecode. check_bundle.py validates the 48-page atlas, E01-E12 inventory, final-only WF31 and exact P11 mappings. These checks do not pass product/proof gates.
