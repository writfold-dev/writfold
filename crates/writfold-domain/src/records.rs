//! Shared boilerplate only: these constructors enforce typed structure, not truth.
macro_rules! record {
    ($(#[$meta:meta])* $name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $($field: $ty),* }
        impl $name {
            /// Constructs a structurally typed candidate; semantic validity is a kernel judgment.
            #[allow(clippy::too_many_arguments, reason = "preserves the reference record field contract")]
            pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
            $(pub fn $field(&self) -> &$ty { &self.$field })*
        }
    };
}

pub(crate) use record;
