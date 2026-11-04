# Engineering/1 examples

These are synthetic planning fixtures, not installed skills, live repository
bindings, authority grants or native Omnigraph commits. `fingerprints.json` gives
canonical bytes and domain-separated hashes for the proposed engineering/1
profile. JSON is a readable fixture representation; authenticated bytes are the
restricted deterministic CBOR described in plan/18.

`tools/check_engineering_examples.py` checks hand-encoded CBOR vectors, unsupported
encoder input types, selected fingerprint properties and these exact fixtures.
There is no decoder here. Duplicate-key/noncanonical-byte rejection remains a
production parser obligation. These checks do not pass product gates.

The implementing agent must create the real three-Git-repository pagination
fixture in P17, preserve its failing old-consumer case, and qualify actual
Worktrunk/Git/forge operations in authorized profiles. Examples cannot stand in
for those experiments.
