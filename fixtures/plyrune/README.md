# Plyrune stand-in (tests only)

`bin/plyrune` is a stand-in for the Plyrune checker and `reports/` holds
stand-in report vectors. They exist so Writfold's optional Plyrune gate
recipe (`docs/guide/recipes/plyrune-check.sh`, #106) can be exercised
without the real binary. They are **not** Plyrune and **not** the exchange
profile being agreed in plyrune-dev/plyrune#39; replace them with that
profile's pinned conformance vectors when it is fixed (#122).
