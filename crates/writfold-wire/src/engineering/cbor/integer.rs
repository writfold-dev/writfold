use super::{Error, encode::Writer};
use crate::Budget;
use writfold_domain::Integer;

pub(super) fn encode(writer: &mut Writer, value: &Integer) -> Result<(), Error> {
    let magnitude = value.as_bigint().magnitude();
    let negative = value.is_negative();
    let mut bits = magnitude.bits();
    let trailing = if negative {
        magnitude.trailing_zeros().unwrap_or(0)
    } else {
        0
    };
    // CBOR negative magnitude is abs(value)-1. Its bit width decreases exactly
    // for powers of two; no cloned BigInt or temporary byte buffer is needed.
    if negative && trailing == bits - 1 {
        bits -= 1;
    }
    if bits <= 64 {
        let low = magnitude.iter_u64_digits().next().unwrap_or(0);
        return writer.header(u8::from(negative), low.wrapping_sub(u64::from(negative)));
    }
    let length = bits.div_ceil(8);
    let length_usize = usize::try_from(length).map_err(|_| Error::Limit {
        budget: Budget::Bytes,
        offset: writer.bytes.len(),
    })?;
    writer.header(6, if negative { 3 } else { 2 })?;
    writer.header(2, length)?;
    writer.room(length_usize)?;
    writer.bytes.reserve(length_usize);
    let borrow_digit = trailing / 64;
    let mut first = true;
    for (index, digit) in magnitude.iter_u64_digits().enumerate().rev() {
        // Subtraction borrows through exactly the low zero digits. Visiting
        // high digits first streams big-endian output directly into its owner.
        let adjusted = if negative && index as u64 <= borrow_digit {
            digit.wrapping_sub(1)
        } else {
            digit
        };
        if first && adjusted == 0 {
            continue;
        }
        let skip = if first {
            (adjusted.leading_zeros() / 8) as usize
        } else {
            0
        };
        writer
            .bytes
            .extend_from_slice(&adjusted.to_be_bytes()[skip..]);
        first = false;
    }
    Ok(())
}
