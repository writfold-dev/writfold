"""Unverified reference encoder for proposed engineering/1 fixture bytes.

This is a planning/conformance aid, not the production parser or a proof.
It intentionally has no general CBOR decoder or effect/admission authority.
"""
import hashlib

KINDS = frozenset(('blob', 'package', 'dependency-lock', 'skill', 'procedure',
                  'design', 'repo-vector', 'read-set', 'environment',
                  'attempt-input', 'effect-intent', 'dispatch-stamp', 'receipt', 'ledger-event',
                  'learning', 'read-cut'))


def _head(major, n):
    if n < 0:
        raise ValueError('negative length')
    if n < 24:
        return bytes([(major << 5) | n])
    for ai, size in ((24, 1), (25, 2), (26, 4), (27, 8)):
        if n < 1 << (8 * size):
            return bytes([(major << 5) | ai]) + n.to_bytes(size, 'big')
    raise ValueError('length exceeds profile')


def canonical(value):
    if value is None:
        return b'\xf6'
    if type(value) is bool:
        return b'\xf5' if value else b'\xf4'
    if type(value) is int:
        magnitude = value if value >= 0 else -1 - value
        if magnitude < 1 << 64:
            return _head(0 if value >= 0 else 1, magnitude)
        raw = magnitude.to_bytes((magnitude.bit_length() + 7) // 8, 'big')
        return _head(6, 2 if value >= 0 else 3) + canonical(raw)
    if type(value) is bytes:
        return _head(2, len(value)) + value
    if type(value) is str:
        raw = value.encode('utf-8', errors='strict')
        return _head(3, len(raw)) + raw
    if type(value) is list:
        return _head(4, len(value)) + b''.join(map(canonical, value))
    if type(value) is dict:
        if any(type(k) is not str for k in value):
            raise ValueError('only unique text map keys')
        encoded = sorted((canonical(k), canonical(v)) for k, v in value.items())
        return _head(5, len(encoded)) + b''.join(k + v for k, v in encoded)
    raise ValueError('unsupported engineering/1 type')


def fingerprint(kind, payload):
    if kind not in KINDS:
        raise ValueError('unregistered kind')
    preimage = ('writfold-engineering/1/' + kind).encode() + b'\0' + canonical(payload)
    return 'wf1:' + kind + ':sha256:' + hashlib.sha256(preimage).hexdigest()
