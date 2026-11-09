//! Byte-exact identity comparison.
use alloc::vec::Vec;

/// Whether two byte strings are identical: same length, same bytes.
pub fn same(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut i = 0;
    while i < left.len() {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Optional identities are equal when both are absent or both name the same bytes.
pub fn same_option(left: &Option<Vec<u8>>, right: &Option<Vec<u8>>) -> bool {
    match left {
        None => match right {
            None => true,
            Some(_) => false,
        },
        Some(l) => match right {
            None => false,
            Some(r) => same(l, r),
        },
    }
}

/// Whether a 32-byte digest is in `set`.
pub fn contains_digest(set: &[[u8; 32]], digest: &[u8; 32]) -> bool {
    let mut i = 0;
    while i < set.len() {
        if same(&set[i], digest) {
            return true;
        }
        i += 1;
    }
    false
}
