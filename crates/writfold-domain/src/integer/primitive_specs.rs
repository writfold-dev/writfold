//! WF-TCB-001: trusted contracts of the pinned num-bigint dependency only.
//! No Writfold executable body is excluded here. These are assumptions about
//! external library primitives, not mechanically verified BigInt arithmetic.
use num_bigint::{BigInt, Sign, TryFromBigIntError};
use vstd::prelude::*;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExBigInt(BigInt);

#[verifier::external_type_specification]
pub struct ExSign(Sign);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
pub struct ExConversionError<T>(TryFromBigIntError<T>);

pub uninterp spec fn mathematical(value: BigInt) -> int;
pub uninterp spec fn from_primitive(value: int) -> BigInt;

// Match the library's separate impl-level and method-level reference binders.
#[verifier::external_fn_specification]
pub fn checked_u64<'a>(value: &BigInt)
    -> (result: Result<u64, TryFromBigIntError<()>>)
    ensures
        result.is_ok() <==> 0 <= mathematical(*value) <= u64::MAX,
        result.is_ok() ==> result.unwrap() as int == mathematical(*value),
{ u64::try_from(value) }

#[verifier::external_fn_specification]
pub fn checked_i64<'a>(value: &BigInt)
    -> (result: Result<i64, TryFromBigIntError<()>>)
    ensures
        result.is_ok() <==> i64::MIN <= mathematical(*value) <= i64::MAX,
        result.is_ok() ==> result.unwrap() as int == mathematical(*value),
{ i64::try_from(value) }

#[verifier::external_fn_specification]
pub fn sign(value: &BigInt) -> (result: Sign)
    ensures
        (result is Minus) <==> mathematical(*value) < 0,
        (result is NoSign) <==> mathematical(*value) == 0,
        (result is Plus) <==> mathematical(*value) > 0,
{ value.sign() }

#[verifier::external_fn_specification]
pub fn clone(value: &BigInt) -> (result: BigInt)
    ensures result == *value,
{ value.clone() }

#[verifier::external_fn_specification]
pub fn clone_from(target: &mut BigInt, source: &BigInt)
    ensures *final(target) == *source,
{ target.clone_from(source) }

} // verus!

macro_rules! primitive_constructor {
    ($name:ident, $primitive:ty) => {
        verus! {
        #[verifier::external_fn_specification]
        pub fn $name(value: $primitive) -> (result: BigInt)
            ensures mathematical(result) == value as int,
                result == from_primitive(value as int),
        { BigInt::from(value) }
        }
    };
}
primitive_constructor!(from_i8, i8);
primitive_constructor!(from_i16, i16);
primitive_constructor!(from_i32, i32);
primitive_constructor!(from_i64, i64);
primitive_constructor!(from_i128, i128);
primitive_constructor!(from_isize, isize);
primitive_constructor!(from_u8, u8);
primitive_constructor!(from_u16, u16);
primitive_constructor!(from_u32, u32);
primitive_constructor!(from_u64, u64);
primitive_constructor!(from_u128, u128);
primitive_constructor!(from_usize, usize);
