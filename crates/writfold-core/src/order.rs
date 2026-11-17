//! The order receipts are judged in, oldest first, with no clock in it:
//! by `seq` (one more than the highest of the receipts it follows), then
//! by file name. Gate receipts, cover rows, assessed receipts and delivery
//! check receipts are all sorted with [`cmp`]; a receipt written before it
//! carried `seq` reads as 0.
//!
//! [`less`] and [`precedes`] are translated and proved
//! (`proofs/Writfold/Order.lean`): `precedes` is a strict total order, so
//! the sort has one answer whatever order the files were listed in.

use core::cmp::Ordering;

/// Whether byte string `a` sorts strictly before `b`: at the first place
/// they differ, the smaller byte; else the shorter.
pub fn less(a: &[u8], b: &[u8]) -> bool {
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
        i += 1;
    }
    a.len() < b.len()
}

/// Whether the receipt `(seq_a, name_a)` is judged before
/// `(seq_b, name_b)`: the lower `seq` first, then the file name.
pub fn precedes(seq_a: u64, name_a: &[u8], seq_b: u64, name_b: &[u8]) -> bool {
    if seq_a != seq_b {
        return seq_a < seq_b;
    }
    less(name_a, name_b)
}

/// [`precedes`] as the comparison `sort_by` takes.
pub fn cmp(a: (u64, &str), b: (u64, &str)) -> Ordering {
    if precedes(a.0, a.1.as_bytes(), b.0, b.1.as_bytes()) {
        Ordering::Less
    } else if precedes(b.0, b.1.as_bytes(), a.0, a.1.as_bytes()) {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::cmp;

    /// The comparison is the tuple order on `(seq, name)` it replaces.
    #[test]
    fn agrees_with_the_tuple_order() {
        let names = [
            "",
            "a",
            "ab",
            "b",
            "a-20261111T000100Z-1",
            "a-20261111T000100Z-2",
            "é",
            "z",
        ];
        for sa in [0u64, 1, 2, u64::MAX] {
            for sb in [0u64, 1, 2, u64::MAX] {
                for na in names {
                    for nb in names {
                        assert_eq!(
                            cmp((sa, na), (sb, nb)),
                            (sa, na).cmp(&(sb, nb)),
                            "{sa} {na} {sb} {nb}"
                        );
                    }
                }
            }
        }
    }
}
