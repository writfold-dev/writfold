# Scribe author experience: implementation sketch

This is the desired SDK experience, not an installed SDK or working Writfold release. P08S must implement and test the helpers and protocol; do not publish these imports as available today.

```python
from writfold_scribe import Scribe, ExactSource, ReadChunk

scribe = Scribe(id="example.team-notes", protocol=1)

@scribe.resolve(kind="example.note")
def resolve(request, host):
    # The host broker enforces the already granted root and returns retained identity.
    source = host.sources.resolve(request.locator)
    return ExactSource(source.id, source.revision)

@scribe.read()
def read(request, host):
    # No raw KB access, credentials, subprocess shell or knowledge admission here.
    chunk = host.sources.read(request.source, offset=request.offset,
                              max_bytes=request.max_bytes)
    return ReadChunk.from_broker(chunk)

scribe.run_stdio()
```

The SDK owns strict framing, IDs/session correlation, supported-version negotiation, clean stdout, callback bookkeeping, cancellation and bounded errors. Its convenience must not bypass host checks. The same protocol is usable from a small script without this SDK or from a compiled language. Pin the SDK/interpreter/dependencies alongside the script. A synthetic no-IO provider is the first authoring fixture; real local and HTTP providers qualify integration separately.

The catalog maps a locator kind to a namespaced admitted scribe capability. A locator can resolve to an immutable source; the source acquisition can produce a writ; a writ can support a proposed claim under the calculus and policy. None of those nouns is an interchangeable string ID.
