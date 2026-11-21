//! Python 3.12 / Unicode 15 whitespace, which authored-material hashing
//! uses to find a heading (`##` followed by whitespace).

/// Whether `character` is whitespace in Python's `str.isspace` sense.
pub fn whitespace(character: char) -> bool {
    matches!(character, '\u{9}'..='\u{d}' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{a0}'
        | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'..='\u{2029}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}')
}
