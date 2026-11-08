//! Borrowed traversal tokens retain the supplied representation, not a schema.

/// Exact CBOR integer representation without allocating an arbitrary integer.
/// Negative forms denote `-1 - argument`, not the negation of their argument.
/// Bignum arguments are minimal, big-endian byte strings longer than eight bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerRef<'a> {
    Unsigned(u64),
    Negative(u64),
    PositiveBig(&'a [u8]),
    NegativeBig(&'a [u8]),
}

impl IntegerRef<'_> {
    /// Native unsigned value, if this is the unsigned native representation.
    pub fn as_u64(self) -> Option<u64> {
        match self {
            Self::Unsigned(value) => Some(value),
            _ => None,
        }
    }

    /// Native signed value, including negative CBOR integers below `i64::MIN`.
    /// Bignums remain borrowed; this method deliberately does not convert them.
    pub fn as_native_i128(self) -> Option<i128> {
        match self {
            Self::Unsigned(value) => Some(i128::from(value)),
            Self::Negative(value) => Some(-1 - i128::from(value)),
            _ => None,
        }
    }
}

/// Token categories used in schema-level type errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Null,
    Bool,
    Integer,
    Bytes,
    Text,
    Array,
    Map,
    Key,
    EndArray,
    EndMap,
}

/// Containers emit an opening token, their children, then a closing token.
/// Map keys have a distinct token and are not counted as value nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenValue<'a> {
    Null,
    Bool(bool),
    Integer(IntegerRef<'a>),
    Bytes(&'a [u8]),
    Text(&'a str),
    Array(usize),
    Map(usize),
    Key(&'a str),
    EndArray,
    EndMap,
}

impl TokenValue<'_> {
    pub fn kind(self) -> Kind {
        match self {
            Self::Null => Kind::Null,
            Self::Bool(_) => Kind::Bool,
            Self::Integer(_) => Kind::Integer,
            Self::Bytes(_) => Kind::Bytes,
            Self::Text(_) => Kind::Text,
            Self::Array(_) => Kind::Array,
            Self::Map(_) => Kind::Map,
            Self::Key(_) => Kind::Key,
            Self::EndArray => Kind::EndArray,
            Self::EndMap => Kind::EndMap,
        }
    }
}

/// A borrowed token and its exact supplied bytes, with absolute input offset.
/// Opening containers cover only their header. Closing containers cover the
/// entire container (including its opening header), so no offsets need slicing.
/// Scalar tokens and map keys cover their complete encoded item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token<'a> {
    pub value: TokenValue<'a>,
    pub bytes: &'a [u8],
    pub offset: usize,
}
