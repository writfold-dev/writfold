# Writfold implementation

Writfold is the Rust successor to kctx. The original kctx history is preserved in this repository; the immutable implementation handover is under `handover/`. The current Python and Lean source is the compatibility reference, not the Rust product.

## Authority and execution

- Follow `handover/AGENT_START_HERE.md`, all 17 stage contracts, and `handover/plan/08-execution-and-acceptance.md` in dependency order. Workflow 3 and the playbook adoption in `handover/plan/11-workflow3-contract.md` and `handover/plan/12-playbook-adoption.md` are normative.
- Adopt private `private:playbook@e7d991e990e3dff9c5d70e4327c0e90298e2f67e` only at that pin. Do not publish private reference material or edit the playbook as part of consuming it.
- Preserve kctx `1c17b94be37af8f82f222baddf31435f14172e1a`, historical identifiers, signed bytes, schema namespaces, citations and `refs/kctx/*`. Do not repeat its completed Dolt warning or private-reader fixes.
- Keep current implementation status and exact evidence separate from the immutable planning registries. No Rust gate passes because the Python implementation or handover integrity check passes.
- Preserve failed attempts and bind evidence to exact source, tool, dependency, fixture and artifact identities. Never convert a skipped, blocked or unrun obligation into a pass.

## Engineering and builds

- Linux/WSL x86_64 only. Build and qualify locally through Compose/BuildKit. Reuse the operator's actual storage binding and admission floors; never guess drive letters, relocate storage, prune unrelated resources or install publishing secrets on build workers.
- Target a static musl Rust executable, qualifying the exact Verus executable-body/compiler arrangement first. Dolt and Git remain explicit native dependencies with separately qualified installation/offline closure.
- Start with domain, kernel, wire and protocol crates. Keep kernel/protocol IO-free, nominal types checked, modules cohesive, and Rust dependencies rustls-only. Apply the size/dependency rules in `handover/plan/05-build-and-quality.md`.
- Preserve the Lean calculus and its assumptions. Verify selected shipping Rust bodies with Verus; qualify operational interleavings with TLC and inductive safety/bridges with TLAPS. Different proof systems and differential agreement are not an automatic refinement theorem.
- CLI and RPC share typed application decisions and immutable prepared views. Contexts never pool authority or evidence. Cancellation retains resource ownership until owned children are reaped.
- Keep handover files immutable. Update current documentation, diagrams, traceability and status alongside implemented changes. Public website copy may describe only exercised capabilities and source-bound assurance.
