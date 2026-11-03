//! Exact length-prefixed inventory identity without formatter assumptions.
use super::ContentId;
use vstd::prelude::*;

verus! {

pub open spec fn decimal_digit(value: nat) -> char { ((48 + value % 10) as u8) as char }
pub open spec fn decimal(value: nat) -> Seq<char>
    decreases value,
{
    if value < 10 { seq![decimal_digit(value)] }
    else { decimal(value / 10) + seq![decimal_digit(value % 10)] }
}

spec fn power10(exponent: nat) -> nat
    decreases exponent,
{
    if exponent == 0 { 1 } else { 10 * power10((exponent - 1) as nat) }
}

proof fn decimal_bound(value: nat, digits: nat)
    requires digits > 0, value < power10(digits),
    ensures 0 < decimal(value).len() <= digits,
    decreases digits,
{
    reveal_with_fuel(power10, 2);
    assert(power10(1) == 10);
    if value >= 10 {
        assert(digits > 1);
        assert(value / 10 < power10((digits - 1) as nat));
        decimal_bound(value / 10, (digits - 1) as nat);
    }
}

spec fn buffer_suffix(buffer: Seq<u8>, start: int) -> Seq<char> {
    buffer.subrange(start, buffer.len() as int).map_values(|byte: u8| byte as char)
}

fn decimal_bytes(value: usize, buffer: &mut [u8; 20]) -> (start: usize)
    ensures start < 20, buffer_suffix(final(buffer)@, start as int) == decimal(value as nat),
{
    let mut remaining = value;
    let mut index = 20usize;
    proof {
        reveal_with_fuel(power10, 21);
        assert((value as nat) < power10(20));
        decimal_bound(value as nat, 20);
    }
    while remaining >= 10
        invariant
            0 < index <= 20,
            0 < decimal(remaining as nat).len() <= index,
            decimal(value as nat) == decimal(remaining as nat) + buffer_suffix(buffer@, index as int),
        decreases remaining,
    {
        proof {
            assert(0 < decimal((remaining / 10) as nat).len());
            assert(index > 1);
        }
        index -= 1;
        buffer[index] = b'0' + (remaining % 10) as u8;
        remaining /= 10;
    }
    index -= 1;
    buffer[index] = b'0' + remaining as u8;
    index
}

fn append_decimal(output: &mut String, value: usize)
    ensures final(output)@ == old(output)@ + decimal(value as nat),
{
    let mut buffer = [0u8; 20];
    let start = decimal_bytes(value, &mut buffer);
    let mut index = start;
    let ghost before = output@;
    while index < 20
        invariant
            start <= index <= 20,
            buffer_suffix(buffer@, start as int) == decimal(value as nat),
            output@ == before + buffer@.subrange(start as int, index as int).map_values(|byte: u8| byte as char),
        decreases 20 - index,
    {
        output.push(buffer[index] as char);
        index += 1;
    }
}

pub open spec fn member_frames(members: Seq<Seq<char>>) -> Seq<char>
    decreases members.len(),
{
    if members.len() == 0 { Seq::empty() }
    else {
        let last = members.last();
        member_frames(members.drop_last())
            + decimal(vstd::utf8::encode_utf8(last).len()) + seq![':'] + last
    }
}

impl ContentId {
    pub open spec fn inventory_spec(members: Seq<Seq<char>>) -> Seq<char> {
        "inventory:"@ + member_frames(members)
    }

    /// Exact nonempty inventory framing; lengths count UTF-8 bytes, not characters.
    pub fn inventory(members: &[String]) -> (identity: Self)
        ensures identity@ == Self::inventory_spec(members.deep_view()), identity.valid(),
    {
        proof { broadcast use vstd::seq_lib::group_seq_properties; }
        let mut output = "inventory:".to_owned();
        let mut index = 0;
        proof { assert(members.deep_view().len() == members.len()); }
        while index < members.len()
            invariant
                index <= members.len(),
                members.deep_view().len() == members.len(),
                output@ == "inventory:"@ + member_frames(members.deep_view().take(index as int)),
            decreases members.len() - index,
        {
            let member = members[index].as_str();
            let bytes = member.as_bytes();
            proof {
                assert(member@ == members.deep_view()[index as int]);
                assert(bytes@ == vstd::utf8::encode_utf8(member@));
            }
            append_decimal(&mut output, bytes.len());
            output.push(':');
            output.push_str(member);
            index += 1;
            proof {
                let prefix = members.deep_view().take(index as int);
                assert(prefix.len() == index);
                assert(prefix.drop_last() == members.deep_view().take(index as int - 1));
                assert(prefix.last() == member@);
            }
        }
        proof {
            reveal_strlit("inventory:");
            assert(members.deep_view().take(members.len() as int) == members.deep_view());
            assert(output@ == Self::inventory_spec(members.deep_view()));
            assert(output@.len() > 0);
            vstd::utf8::lemma_encode_utf8_len_strictly_monotonic(output@, 0, output@.len() as int);
            assert(output@.subrange(0, output@.len() as int) == output@);
            assert(Self::accepts(output@));
        }
        Self(output)
    }
}

} // verus!
