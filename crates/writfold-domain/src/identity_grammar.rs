//! Byte-exact ASCII grammars; opaque identifiers preserve arbitrary UTF-8.

pub(crate) fn token(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

pub(crate) fn slug(value: &str) -> bool {
    fn end(b: &u8) -> bool {
        b.is_ascii_lowercase() || b.is_ascii_digit()
    }
    value.as_bytes().first().is_some_and(end)
        && value.as_bytes().last().is_some_and(end)
        && value.bytes().all(|b| end(&b) || b == b'-')
}

pub(crate) fn relationship(value: &str) -> bool {
    value.split("__").all(slug)
}

pub(crate) fn proposal(value: &str) -> bool {
    token(value)
        || value
            .split_once('/')
            .is_some_and(|(a, b)| slug(a) && slug(b))
}

pub(crate) fn investigation(value: &str) -> bool {
    if token(value) {
        return true;
    }
    let Some((year, rest)) = value.split_once('/') else {
        return false;
    };
    let bytes = rest.as_bytes();
    year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit())
        && bytes.len() > 11
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && bytes[10] == b'-'
        && rest.get(11..).is_some_and(slug)
}

pub(crate) fn opaque(value: &str) -> bool {
    !value.is_empty()
}

pub(crate) fn lower_hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn digest(value: &str) -> bool {
    lower_hex(value, 64)
}

pub(crate) fn release(value: &str) -> bool {
    lower_hex(value, 40) || lower_hex(value, 64)
}

pub(crate) fn dolt(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'v').contains(&b))
}
