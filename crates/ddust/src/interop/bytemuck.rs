//! bytemuck's `Pod` for a decimal of a static scale, which no derive can prove of a generic
//! `repr(C)` struct: the decimal is its integer's bytes.

use bytemuck::Pod;

use crate::{Decimal, Fixed, Int};

// `Fixed<D>` takes no room and no alignment, so a decimal of it has its integer's size and
// alignment, which the `Pod` below relies on; checked here for each of the ten integers.
const _: () = {
    macro_rules! same_layout {
        ($($integer:ty),*) => {$(
            assert!(size_of::<Decimal<$integer, Fixed<0>>>() == size_of::<$integer>());
            assert!(align_of::<Decimal<$integer, Fixed<0>>>() == align_of::<$integer>());
        )*};
    }
    same_layout!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);
};

#[expect(unsafe_code, reason = "no derive proves a generic `repr(C)` struct free of padding")]
// SAFETY: `Decimal` is `repr(C)`, its steps an `I` and its scale a `Fixed<D>`, a `repr(C)` struct
// of no field, so its size is 0 and its alignment 1: the decimal is `I`'s size and alignment, as
// the assertions above check, with no padding. Its bytes are `I`'s, and every `I` is `Pod`: any
// bytes are a decimal, and every byte of one is initialized. It is `Copy`, `'static` and
// `Zeroable`, as `Pod` requires.
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
