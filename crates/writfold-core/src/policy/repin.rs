//! Adopting a changed policy (`plan repin`, policy spec 2.5): which pin
//! entry is in force for a gate, a step or the required file, and whether
//! a run may write a repin at all. The caller lists the pin entries oldest
//! first and says which of them apply; nothing here reads a file.
//! Translated and proved with the rest of the module
//! (`proofs/Writfold/Policy.lean`).

use super::Pin;

/// The pin entry in force: the newest that applies. `applies` holds, for
/// each entry oldest first, whether it names what is asked about (a gate,
/// a gate for one step, or the required file); `None` when none does.
pub fn in_force(applies: &[bool]) -> Option<usize> {
    let mut index = applies.len();
    while index > 0 {
        index -= 1;
        if applies[index] {
            return Some(index);
        }
    }
    None
}

/// Why `plan repin` is refused, in the fixed order it is checked.
pub enum Refusal {
    /// The run is closed: it takes no plan write.
    Closed,
    /// A pin file no longer reads as written: a repin cannot adopt that.
    Invalid,
    /// The run pinned nothing: there is nothing to adopt a change from.
    Unpinned,
    /// Nothing moved: the policy is the pinned one.
    Same,
    /// A gate an open step owes does not load now: there is no definition
    /// to adopt.
    Undefined,
    /// The lock list as it is now locks a step the run already skipped:
    /// adopting it would reopen that step.
    Reopens,
}

/// Whether a run may write a repin: it is open (`writable`), every pin
/// file reads (`valid`), its policy against its pins ([`super::pin_all`])
/// is `Moved`, every gate an open step owes loads now (`loads`), and the
/// lock list as it is now locks no step the run skipped (`keeps`), so the
/// repin itself invalidates nothing.
pub fn repin_check(
    writable: bool,
    valid: bool,
    pin: &Pin,
    loads: bool,
    keeps: bool,
) -> Result<(), Refusal> {
    if !writable {
        return Err(Refusal::Closed);
    }
    if !valid {
        return Err(Refusal::Invalid);
    }
    match pin {
        Pin::Unpinned => Err(Refusal::Unpinned),
        Pin::Same => Err(Refusal::Same),
        Pin::Moved => {
            if !loads {
                return Err(Refusal::Undefined);
            }
            if !keeps {
                return Err(Refusal::Reopens);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Refusal, in_force, repin_check};
    use crate::policy::Pin;

    #[test]
    fn the_newest_applying_entry_is_in_force() {
        assert_eq!(in_force(&[]), None);
        assert_eq!(in_force(&[false, false]), None);
        assert_eq!(in_force(&[true, false]), Some(0));
        assert_eq!(in_force(&[true, false, true, false]), Some(2));
        // Exhaustive for up to six entries: the last `true`, or none.
        for len in 0..=6usize {
            for bits in 0..(1u32 << len) {
                let list: alloc::vec::Vec<bool> = (0..len).map(|i| bits & (1 << i) != 0).collect();
                assert_eq!(in_force(&list), list.iter().rposition(|b| *b));
            }
        }
    }

    #[test]
    fn a_repin_needs_an_open_run_whose_readable_pins_moved() {
        let pins = || [Pin::Unpinned, Pin::Same, Pin::Moved];
        for bits in 0..16u8 {
            let (writable, valid, loads) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
            let keeps = bits & 8 != 0;
            for pin in pins() {
                let moved = matches!(pin, Pin::Moved);
                let got = repin_check(writable, valid, &pin, loads, keeps);
                assert_eq!(got.is_ok(), writable && valid && moved && loads && keeps);
                match got {
                    Ok(()) => {}
                    Err(Refusal::Closed) => assert!(!writable),
                    Err(Refusal::Invalid) => assert!(writable && !valid),
                    Err(Refusal::Unpinned) => assert!(matches!(pin, Pin::Unpinned)),
                    Err(Refusal::Same) => assert!(matches!(pin, Pin::Same)),
                    Err(Refusal::Undefined) => assert!(moved && !loads),
                    Err(Refusal::Reopens) => assert!(moved && loads && !keeps),
                }
            }
        }
    }
}
