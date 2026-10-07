//! bytemuck's `Pod` for a decimal of a `Fixed` scale, which no derive can prove of a generic
//! `repr(C)` struct: the decimal is its integer's bytes.

use bytemuck::Pod;

use crate::{Decimal, Fixed, Int};

// SAFETY: `Decimal` is `repr(C)`, its steps an `I` and its scale a `Fixed<D>`, a `repr(C)` struct
// of no field, so of size 0 and alignment 1: the decimal is `I`'s size and alignment, as the
// assertions beside `Decimal` check, with no padding, and it is inhabited, as `I` is. Its bytes
// are `I`'s, and every `I` is `Pod`: any bytes are a decimal, every byte of one is initialized,
// and neither field holds a pointer or a cell, so a shared decimal is only read. `Fixed<D>` is no
// `Pod`, since bytemuck's derive refuses its const parameter, but it has no bytes for `Pod` to ask
// anything of. The decimal is `Copy`, `'static` and `Zeroable`, as `Pod` requires.
#[expect(unsafe_code, reason = "no derive proves a generic `repr(C)` struct free of padding")]
unsafe impl<I: Int + Pod, const D: u8> Pod for Decimal<I, Fixed<D>> {}

#[cfg(test)]
mod tests {
    use bytemuck::{bytes_of, cast, cast_slice};

    use crate::{D64, dec};

    #[test]
    fn a_decimal_is_written_as_its_steps_bytes() {
        let price: D64<8> = dec!(60000.5);
        assert_eq!(bytes_of(&price), 6_000_050_000_000_i64.to_ne_bytes(), "its steps' bytes");
        assert_eq!(cast::<D64<8>, i64>(price), 6_000_050_000_000, "its steps, cast");
    }

    #[test]
    fn a_slice_of_decimals_is_viewed_as_bytes_without_a_copy() {
        let prices: [D64<2>; 2] = [dec!(1.5), dec!(-2.25)];
        let bytes: &[u8] = cast_slice(&prices);
        assert_eq!(bytes.len(), 16, "two eight-byte decimals");
        assert_eq!(cast_slice::<u8, D64<2>>(bytes), prices, "and back");
    }
}
