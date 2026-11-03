//! Shared boilerplate only: these constructors enforce typed structure, not truth.
macro_rules! record {
    ($(#[$meta:meta])* $name:ident, $view:ident { $($field:ident: $ty:ty $(=> $equal:path)?),* $(,)? }) => {
        ::vstd::prelude::verus! {
        #[cfg(verus_keep_ghost)]
        pub ghost struct $view { $(pub $field: <$ty as ::vstd::prelude::DeepView>::V),* }
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name { $($field: $ty),* }
        #[cfg(verus_keep_ghost)]
        impl ::vstd::prelude::View for $name {
            type V = $view;
            closed spec fn view(&self) -> $view {
                $view { $($field: ::vstd::prelude::DeepView::deep_view(&self.$field)),* }
            }
        }
        #[cfg(verus_keep_ghost)]
        impl ::vstd::prelude::DeepView for $name {
            type V = $view;
            open spec fn deep_view(&self) -> $view {
                ::vstd::prelude::View::view(self)
            }
        }
        impl Clone for $name {
            #[inline]
            fn clone(&self) -> (cloned: Self)
                ensures ::vstd::prelude::View::view(&cloned) == ::vstd::prelude::View::view(self),
            {
                proof { broadcast use ::vstd::std_specs::vec::group_vec_axioms; }
                Self { $($field: self.$field.clone()),* }
            }
        }
        #[cfg(verus_keep_ghost)]
        impl ::vstd::std_specs::cmp::PartialEqSpecImpl for $name {
            open spec fn obeys_eq_spec() -> bool { true }
            open spec fn eq_spec(&self, other: &Self) -> bool {
                ::vstd::prelude::View::view(self) == ::vstd::prelude::View::view(other)
            }
        }
        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> (equal: bool)
                ensures equal == (::vstd::prelude::View::view(self) == ::vstd::prelude::View::view(other)),
            {
                proof { broadcast use crate::equality::vector_equality; }
                $(crate::records::field_equal!(&self.$field, &other.$field $(, $equal)?))&&*
            }
        }
        impl Eq for $name {}
        impl $name {
            /// Constructs a structurally typed candidate; semantic validity is a kernel judgment.
            #[allow(clippy::too_many_arguments, reason = "preserves the reference record field contract")]
            pub fn new($($field: $ty),*) -> (constructed: Self)
                ensures ::vstd::prelude::View::view(&constructed) ==
                    ($view { $($field: ::vstd::prelude::DeepView::deep_view(&$field)),* }),
            { Self { $($field),* } }
            $(pub fn $field(&self) -> (result: &$ty)
                ensures ::vstd::prelude::DeepView::deep_view(result) == ::vstd::prelude::View::view(self).$field,
            { &self.$field })*
        }
        }
    };
}

/// Fieldless enum values already are their mathematical representation.
macro_rules! scalar_views {
    ($($name:ty),+ $(,)?) => {$(
        ::vstd::prelude::verus! {
        impl ::vstd::prelude::DeepView for $name {
            type V = Self;
            open spec fn deep_view(&self) -> Self { *self }
        }
        #[cfg(verus_keep_ghost)]
        impl ::vstd::std_specs::cmp::PartialEqSpecImpl for $name {
            open spec fn obeys_eq_spec() -> bool { true }
            open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
        }
        }
    )+};
}

macro_rules! field_equal {
    ($left:expr, $right:expr) => {
        ($left).eq($right)
    };
    ($left:expr, $right:expr, $compare:path) => {
        $compare($left, $right)
    };
}

pub(crate) use {field_equal, record, scalar_views};
