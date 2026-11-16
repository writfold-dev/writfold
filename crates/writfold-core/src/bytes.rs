//! Byte-exact identity comparison. `plan::contains` and the gate kernels
//! compare digests and keys through [`same`].

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
