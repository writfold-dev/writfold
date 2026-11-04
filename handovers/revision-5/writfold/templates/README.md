# Schemas, not evidence

These schemas are proposed starting contracts. No receipt instance is marked pass. Implement semantic verification in the gate aggregator as well as JSON shape validation: source/tool/artifact hashes must match actual bytes; proof registries must be complete; no empty artifact list or fabricated count may pass by schema alone. Extend timestamps/clock metadata and trust-boundary fields as specified in the plan.

Revision 5 adds engineering-records.schema.json: proposed strict readable stream-head and dispatch-stamp shapes, plus repository/watermark definitions. They do not enforce authorization or canonical encoding. Plan/18 defines the digest domains; P14 freezes complete wire-to-canonical mappings before production writes.
