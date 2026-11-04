# Local comparison harness

Run from the extracted handover with admitted exact binaries. The output directory must not exist; no existing database is mutated.

```sh
python3 qualification/storage/compare.py --omnigraph /absolute/path/to/omnigraph --dolt /absolute/path/to/dolt --out /new/attempt/directory
```

Omnigraph 0.14.0 Linux x86_64: https://github.com/ModernRelay/omnigraph/releases/download/v0.14.0/omnigraph-linux-x86_64.tar.gz

Archive SHA256: `02bde3e3a2c6c8cf50dc9739a8b1041df87cfeaa6ab59f620e1d8e32acffd4d9`. Verify before unpacking, then verify the CLI hash specified in the script. No downloaded binary is bundled. Dolt is the retained reference/import 2.6.1 profile; admit its executable digest through the existing provisioning gate, then compare with the retained run's digest.

The script executed 19 commands and checked 12 focused assertions here. The expected Omnigraph historical refusal is a passing observation, not a claim that the storage-authority contract passed. Logs and command argv are in `../../evidence/views-r3/storage-probe/`. The machine paths in those logs identify the actual executed attempt and need not exist on the PC. No benchmark, crash, server, formal proof or WSL qualification is implied.

See `../../plan/15-storage-decision.md` for the remaining comparison and admission rule. Local engines can create large data directories; the new attempt directory is disposable test state. The script never invokes cleanup on user data.


Revision 4 selects Omnigraph as the primary backend target. The original comparison remains unchanged reference evidence. Reproduce the follow-up with:

```sh
python3 qualification/storage/versioned_schema.py --omnigraph /absolute/path/to/omnigraph --out /new/versioned-attempt
```

It preserves ServiceV1, adds ServiceV2, compares the old pinned whole-node read and checks the explicit schema_changed branch refusal. Six assertions and seven commands passed in the retained local run. This is not the production two-epoch lifecycle or WF24. The latter is mandatory P01O under plans/15–16; do not choose the old retain-Dolt-only gate path.
