#!/bin/sh
# Exit 0 when some docs/research/*.md has the three headings this team asks for.
for note in docs/research/*.md; do
    [ -f "$note" ] || continue
    if grep -q '^## Question' "$note" && grep -q '^## Found' "$note" && grep -q '^## Decision' "$note"; then
        exit 0
    fi
done
echo "no docs/research/*.md with Question, Found and Decision headings" >&2
exit 1
