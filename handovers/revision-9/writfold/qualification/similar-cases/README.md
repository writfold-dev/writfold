# Similar-case qualification packet

Read [the contract](../../plan/28-similar-incident-discovery.md) and [the engine decision table](../../plan/29-omnigraph-capability-fit.md). All data in this directory is synthetic.

| Account | Checked here | Open |
| --- | --- | --- |
| Local logical guards | Five proofs in [GuardLaws.mm](formal/GuardLaws.mm) verified by an existing, independent Metamath checker with standard propositional conjunction, implication composition and contraposition. A corrupted proof statement was rejected. | These propositions assume guard formulas as written. They do not validate production code or constitute the project's Lean qualification. |
| Erasing reads preserves authority | [TraceErasure.mm](formal/TraceErasure.mm) proves by structural induction that evaluating a trace has the same effect on authority as erasing every discovery/inspection and folding its checked witnesses. The checker verified the base, two constructor steps and general theorem. A mutation changing the read semantics was rejected. | Six model-defining equations assume reads preserve authority and witness transitions are separate. Qualify that mapping against actual Rust and Omnigraph. The Lean formalization remains the project gate. |
| Abstract reader/authority calculus | [CaseDiscovery.lean](formal/CaseDiscovery.lean) contains full candidate proof terms, concrete positive and negative witnesses, and axiom printouts. | No Lean 4.21 executable was available here. Run the pinned compiler and inspect its axiom report on WSL. The Rust adapter must refine these definitions. |
| Concurrent publication and reading | The finite Python transition model exhaustively checked two epoch values. Three guard mutants have counterexamples in [the receipt](../../evidence/similar-cases/bounded-reference.json). | The [TLA+ model](formal/IndexPublication.tla) and [TLC config](formal/IndexPublication.cfg) are supplied. Neither TLC nor TLAPS ran here. The Python run is a separate model, with no machine-checked translation to TLA+. |
| Agent surfacing | [Reader test](check_reader.py) injected every permutation of six synthetic candidates and several hostile responses. | Native Omnigraph integration, access isolation, retention, Verus mapping and held-out answer/token evaluation remain open. |

## Reproduce

From this directory, run `python3 check_protocol.py` and `python3 check_reader.py`. Use [the source/receipt manifest](../../evidence/similar-cases/receipts.json) for hashes. Product gates remain pending.

On the proof host, enter `formal`, confirm `lean --version` matches the exact checked-in `lean-toolchain`, and run `lean CaseDiscovery.lean`. Require exit zero, all expected declarations printed, no sorry/admit or extra project axioms, and archive the complete transcript, executable and source hashes. Review the standard Lean axiom dependencies in the output. Reproduce on a clean host before recording a Lean receipt.

To replay the supplemental propositional proof, obtain upstream [mmverify.py at the reviewed commit](https://github.com/david-a-wheeler/mmverify.py/blob/63a2aa3815cd5bf67892dbfc7f2048cbb177524c/mmverify.py), verify SHA-256 `a1586636b9f3b8378932e129a1a8299dab8ad39098eca22480583947d1f644b8`, and run `python3 mmverify.py formal/GuardLaws.mm -v 3` from this directory. Then run the same checker against `formal/TraceErasure.mm`. This small independent check is optional to the product toolchain.

For TLA+, pin the qualified tools jar and JVM from the proof platform. From `formal`, run `java -cp "$TLA2TOOLS_JAR" tlc2.TLC -config IndexPublication.cfg IndexPublication.tla`. Require TypeOK plus FreshPublish, CheckedOpen and EligibleIssue; check the counterexample mutants separately. For general epochs prove each action preserves the invariants, then discharge the inductive/temporal composition with TLAPS. Epochs stand for unique manifest pins, not reusable generation numbers. Production models must cover crash/fsync/CAS, retained refs/ABA, live read authorization and malformed provider results.

This packet does not prove that a nearest result is relevant, that Relevance(a,b) implies SameCause(a,b), or that an incident can be repaired by replaying a historical action. The held-out study in plan/28 decides whether the ranking is worth its cost.

[Two minimal GQ templates](fixtures/case-search.gq) and a [synthetic schema](fixtures/case-search.pg) give the native agent a concrete lexical/hybrid comparison to type-check. Run them **only on an isolated, synthetic visible partition** at an explicit immutable graph cut. They return case keys, not Writfold handles. The test dimension of four is only a fixture. These templates are unexecuted here. Production needs engine-owned binding resolution, governed per-scope statistics, missing-vector coverage and a qualified local embedding profile.

These documents and fixtures are the incident evaluation profile. [Plan 30](../../plan/30-corpus-discovery.md) defines the wider boundary. A second structural fixture checks mixed kinds with `python3 qualification/discovery/check_mixed.py` from the handover root.
