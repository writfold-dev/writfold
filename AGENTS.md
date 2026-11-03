# Writfold implementation

Writfold is the Rust successor to kctx. The original kctx history is preserved in this repository. The current immutable authority is `handovers/revision-4/writfold/`; `handover/` remains the frozen historical baseline, not the current implementation contract. The Python and Lean source is the compatibility reference, not the Rust product.

## Authority and execution

- Follow `handovers/revision-4/writfold/AGENT_START_HERE.md`, all 19 stage contracts, and its `plan/08-execution-and-acceptance.md` in dependency order. Plans 11–16 are normative, including workflow 3, pinned playbook adoption, views and Omnigraph storage. Read the current adoption delta in `docs/writfold/handover-adoption.md` and identity in `qualification/handover-adoption.json`.
- Adopt private `private:playbook@e7d991e990e3dff9c5d70e4327c0e90298e2f67e` only at that pin. Do not publish private reference material or edit the playbook as part of consuming it.
- Preserve kctx `1c17b94be37af8f82f222baddf31435f14172e1a`, historical identifiers, signed bytes, schema namespaces, citations and `refs/kctx/*`. Do not repeat its completed Dolt warning or private-reader fixes.
- Keep current implementation status and exact evidence separate from immutable planning registries. Earlier P00/P01/P02 completions retain their earlier scope, not a renewed revision-4 pass. Inventory all WF00–WF24; P01O/WF24 blocks storage-dependent P04/P06 integration while independent domain/calculus work may proceed.
- Preserve failed attempts and bind evidence to exact source, tool, dependency, fixture and artifact identities. Never convert a skipped, blocked or unrun obligation into a pass.

## Engineering and builds

- Linux/WSL x86_64 only. Build and qualify locally through Compose/BuildKit. Reuse the operator's actual storage binding and admission floors; never guess drive letters, relocate storage, prune unrelated resources or install publishing secrets on build workers.
- Target a static musl Rust executable, qualifying the exact Verus executable-body/compiler arrangement first. Omnigraph is the intended primary structured backend, initially a supervised native companion, not embedded. Admit CLI/server/runtime separately; musl does not make the companion static. Git remains an explicit native dependency.
- Dolt is isolated historical reference/import tooling only; no second normal writer, dual writes or runtime fallback after uncertain writes. A fallback requires an explicit evidence-backed design revision. Physical schema epochs use separate immutable graphs; exact pins bind graph, commit, epoch, query and compatibility profile.
- Start with domain, kernel, wire and protocol crates. Keep kernel/protocol IO-free, nominal types checked, modules cohesive, and Rust dependencies rustls-only. Apply the size/dependency rules in `handovers/revision-4/writfold/plan/05-build-and-quality.md`.
- Preserve the Lean calculus and its assumptions. Verify selected shipping Rust bodies with Verus; qualify operational interleavings with TLC and inductive safety/bridges with TLAPS. Different proof systems and differential agreement are not an automatic refinement theorem.
- CLI and RPC share typed application decisions and immutable prepared views. Contexts never pool authority or evidence. Cancellation retains resource ownership until owned children are reaped.
- Keep handover files immutable. Update current documentation, diagrams, traceability and status alongside implemented changes. Public website copy may describe only exercised capabilities and source-bound assurance.
