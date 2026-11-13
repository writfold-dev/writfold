//! Repository-relative scopes. Without wildcards a scope is the historical
//! byte prefix. Globs match the whole path: `*` and `?` match non-slash bytes,
//! `**` crosses slashes, and `**/` also accepts zero directories. No escaping
//! or character classes are interpreted; all other bytes are literal.
use crate::ir::MAX_SCOPE_BYTES;

/// A conservative literal prefix for pruning before exact scope matching.
pub fn prefix(scope: &str) -> &str {
    let end = scope.find(['*', '?']).unwrap_or(scope.len());
    &scope[..end]
}

/// Match an admitted scope without allocations or exponential backtracking.
/// Scope admission caps the fixed workspace at `MAX_SCOPE_BYTES`.
pub fn matches(scope: &str, path: &str) -> bool {
    if prefix(scope).len() == scope.len() {
        return path.starts_with(scope);
    }
    if scope.len() > MAX_SCOPE_BYTES {
        return false;
    }
    let pattern = scope.as_bytes();
    let mut states = [false; MAX_SCOPE_BYTES + 1];
    states[0] = true;
    close(pattern, &mut states);
    for byte in path.bytes() {
        let mut next = [false; MAX_SCOPE_BYTES + 1];
        for at in 0..pattern.len() {
            if !states[at] {
                continue;
            }
            match pattern[at] {
                b'*' if pattern.get(at + 1..at + 3) == Some(b"*/") => {
                    next[at + 1] = true;
                    if byte == b'/' {
                        next[at + 3] = true;
                    }
                }
                b'*' if at > 0 && pattern[at - 1] == b'*' && pattern.get(at + 1) == Some(&b'/') => {
                    next[at] = true;
                    if byte == b'/' {
                        next[at + 2] = true;
                    }
                }
                b'*' if pattern.get(at + 1) == Some(&b'*') => next[at] = true,
                b'*' if byte != b'/' => next[at] = true,
                b'?' if byte != b'/' => next[at + 1] = true,
                literal if literal == byte && literal != b'*' => next[at + 1] = true,
                _ => {}
            }
        }
        close(pattern, &mut next);
        states = next;
    }
    states[pattern.len()]
}

fn close(pattern: &[u8], states: &mut [bool; MAX_SCOPE_BYTES + 1]) {
    for at in 0..pattern.len() {
        if !states[at] || pattern[at] != b'*' {
            continue;
        }
        if at > 0 && pattern[at - 1] == b'*' && pattern.get(at + 1) == Some(&b'/') {
            continue;
        }
        if pattern.get(at + 1) == Some(&b'*') {
            states[at + 2] = true;
            if pattern.get(at + 2) == Some(&b'/') {
                states[at + 3] = true;
            }
        } else {
            states[at + 1] = true;
        }
    }
}
