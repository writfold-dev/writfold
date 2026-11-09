#!/bin/sh
# Regenerate proofs/Extracted/ from crates/writfold-core inside the pinned
# proofs image and copy the result out as host-owned files. Commit the result;
# the gate refuses any drift between the crate and the committed translation.
set -eu
cd "$(dirname "$0")/.."
docker compose -f compose.proofs.yaml build proofs
fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT
docker compose -f compose.proofs.yaml run --rm -T --no-deps proofs \
    sh -c 'sh proofs/extract.sh crates/writfold-core /tmp/fresh >&2 && tar -C /tmp/fresh -cf - .' > "$fresh"
rm -rf proofs/Extracted
mkdir -p proofs/Extracted
tar -C proofs/Extracted -xf "$fresh"
echo "refreshed: proofs/Extracted ($(ls proofs/Extracted | tr '\n' ' '))"
