//! Python 3.12 / Unicode 15 whitespace for production metadata admission.
//! This is distinct from byte-nonempty nominal identity construction.
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::iter::IteratorSpec;

verus! {

pub open spec fn whitespace_spec(character: char) -> bool {
    let code = character as u32;
    (0x9 <= code <= 0xD) || (0x1C <= code <= 0x20) || code == 0x85 || code == 0xA0
        || code == 0x1680 || (0x2000 <= code <= 0x200A) || (0x2028 <= code <= 0x2029)
        || code == 0x202F || code == 0x205F || code == 0x3000
}

pub fn whitespace(character: char) -> (result: bool)
    ensures result == whitespace_spec(character),
{
    matches!(character, '\u{9}'..='\u{d}' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{a0}'
        | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'..='\u{2029}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

pub open spec fn nonblank_spec(text: Seq<char>) -> bool {
    exists|index: int| 0 <= index < text.len() && !whitespace_spec(text[index])
}

pub fn nonblank(text: &str) -> (result: bool)
    ensures result == nonblank_spec(text@),
{
    let mut characters = text.chars();
    let ghost mut consumed = Seq::<char>::empty();
    loop
        invariant
            characters.obeys_prophetic_iter_laws(),
            text@ == consumed + characters.remaining(),
            forall|i: int| 0 <= i < consumed.len() ==> whitespace_spec(consumed[i]),
        decreases text@.len() - consumed.len(),
    {
        match characters.next() {
            Some(character) => {
                if !whitespace(character) {
                    proof { assert(text@[consumed.len() as int] == character); }
                    return true;
                }
                proof { consumed = consumed.push(character); }
            }
            None => return false,
        }
    }
}

} // verus!
