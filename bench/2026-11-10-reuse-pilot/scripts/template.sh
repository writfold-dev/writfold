#!/usr/bin/env bash
# Make the template clone: detached at the corpus commit, no remote, no refs,
# and every object not reachable from that commit pruned, so trials cannot see
# later commits (and therefore not this pilot's expected answers).
# Usage: template.sh SOURCE_REPO DEST
set -euo pipefail
C=c568505742d00859cad758924c98d8d0a6e98648
git clone -q --no-local --no-checkout "$1" "$2"
cd "$2"
git -c advice.detachedHead=false checkout -q --detach $C
git remote remove origin
git for-each-ref --format='%(refname)' | while read -r r; do git update-ref -d "$r"; done
git reflog expire --expire=now --all
git gc -q --prune=now
test "$(git rev-parse HEAD)" = $C
test -z "$(git for-each-ref)"
echo "template at $(git rev-parse HEAD), $(git rev-list --all | wc -l) commits reachable"
