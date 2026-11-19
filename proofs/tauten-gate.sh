#!/bin/sh
# `[gate] command` of proofs/claims.toml (docs/writfold/tauten-adoption.md):
# build the proofs gate image from this tree (as docs/writfold/aeneas.md does,
# without a Compose network) and run the unchanged gate through
# proofs/tauten_gate.py, which prints the TAUTEN-PROOFS result lines
# `cargo tauten proofs` reads. Progress goes to stderr; result lines to stdout.
# Run from the workspace root. WRITFOLD_PROOFS_IMAGE overrides the tag.
set -u
cd "$(dirname "$0")/.."
image=${WRITFOLD_PROOFS_IMAGE:-writfold-proofs:tauten}
missing() {
    printf 'TAUTEN-PROOFS {"detail": "%s", "stage": "tools", "status": "missing"}\n' "$1"
    exit 3
}
command -v docker >/dev/null 2>&1 || missing "docker is not installed"
docker info >/dev/null 2>&1 || missing "the docker daemon is not reachable"
docker build -f proofs/Dockerfile --target gate -t "$image" . >&2 \
    || missing "building the proofs image failed (see stderr)"
exec docker run --rm --network none "$image" python3 proofs/tauten_gate.py
