#!/usr/bin/env bash
# Make the template clone of the corpus: a fresh clone of the public
# repository, detached at the pinned commit, no remote, no refs, and every
# object not reachable from that commit pruned, so trials see only that
# commit's history.
# Usage: template.sh DEST            (clones URL below)
#        template.sh DEST SOURCE     (clones an existing local clone instead)
set -euo pipefail
URL=https://github.com/encode/httpx.git
C=b5addb64f0161ff6bfe94c124ef76f6a1fba5254
git clone -q --no-local --no-checkout "${2:-$URL}" "$1"
cd "$1"
git -c advice.detachedHead=false checkout -q --detach $C
git remote remove origin
git for-each-ref --format='%(refname)' | while read -r r; do git update-ref -d "$r"; done
git reflog expire --expire=now --all
git gc -q --prune=now
test "$(git rev-parse HEAD)" = $C
test -z "$(git for-each-ref)"
echo "template at $(git rev-parse HEAD), $(git rev-list --all | wc -l) commits reachable, $(git ls-files | wc -l) tracked files"
