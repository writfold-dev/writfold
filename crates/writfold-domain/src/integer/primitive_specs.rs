//! WF-TCB-001: trusted contracts of the pinned num-bigint dependency only.
//! No Writfold executable body is excluded here. These are assumptions about
//! external library primitives, not mechanically verified BigInt arithmetic.
use num_bigint::{BigInt, Sign, TryFromBigIntError};
use std::{cmp::Ordering, ops::{Add, Sub}};
use vstd::prelude::*;
use vstd::std_specs::ops::{AddSpec, SubSpec};

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

#[verifier::external_fn_specification]
pub fn equal(left: &BigInt, right: &BigInt) -> (result: bool)
    ensures result == (mathematical(*left) == mathematical(*right)),
{ left.eq(right) }

#[verifier::external_fn_specification]
pub fn compare(left: &BigInt, right: &BigInt) -> (result: Ordering)
    ensures
        (result is Less) <==> mathematical(*left) < mathematical(*right),
        (result is Equal) <==> mathematical(*left) == mathematical(*right),
        (result is Greater) <==> mathematical(*left) > mathematical(*right),
{ left.cmp(right) }

// WF-TCB-001 explicitly assumes only the domains of these two foreign
// operations are total. vstd/std_specs/ops.rs documents this orphan-rule
// limitation: this crate cannot implement its SpecImpl traits for BigInt.
// This is a trusted library fact, not a verified lemma or a Writfold decision.
#[verifier::external_body]
pub proof fn borrowed_arithmetic_domain(left: &BigInt, right: &BigInt)
    ensures
        <&BigInt as AddSpec<&BigInt>>::add_req(left, right),
        <&BigInt as SubSpec<&BigInt>>::sub_req(left, right),
{}

// These results are normalized by the library. Canonical identity connects
// Rust's trait result type to the same unbounded mathematical representation.
#[verifier::external_fn_specification]
pub fn add<'a, 'b>(left: &'b BigInt, right: &BigInt) -> (result: BigInt)
    ensures
        mathematical(result) == mathematical(*left) + mathematical(*right),
        result == from_primitive(mathematical(*left) + mathematical(*right)),
{ left.add(right) }

#[verifier::external_fn_specification]
pub fn subtract<'a, 'b>(left: &'b BigInt, right: &BigInt) -> (result: BigInt)
    ensures
        mathematical(result) == mathematical(*left) - mathematical(*right),
        result == from_primitive(mathematical(*left) - mathematical(*right)),
{ left.sub(right) }

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
