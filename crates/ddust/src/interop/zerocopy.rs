//! zerocopy's `IntoBytes` for a decimal of a `Fixed` scale, which its derive cannot prove of a
//! generic `repr(C)` struct of two fields: the decimal is its integer's bytes.

use zerocopy::IntoBytes;

use crate::{Decimal, Fixed, Int};

// zerocopy asks that `IntoBytes` come from its derive, and the trait's one method, named for that
// and `#[doc(hidden)]`, is outside its documented API: a 0.8 release that renames the method, or
// adds another, fails this build.
//
// SAFETY: `Decimal` is `repr(C)`, its steps an `I` and its scale a `Fixed<D>`, a `repr(C)` struct
// of no field, so of size 0 and alignment 1: the decimal is `I`'s size and alignment, as the
// assertions beside `Decimal` check, with no padding, and its bytes are `I`'s, which
// `I: IntoBytes` says are all initialized.
#[expect(unsafe_code, reason = "zerocopy's derive cannot prove this struct free of padding")]
unsafe impl<I: Int + IntoBytes, const D: u8> IntoBytes for Decimal<I, Fixed<D>> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

#[cfg(test)]
mod tests {
    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

    use crate::{D64, Fixed, UD128, dec};

    /// A wire record of decimals, as a program of ddust's users writes one.
    #[derive(Debug, PartialEq, FromBytes, IntoBytes, KnownLayout, Immutable)]
    #[repr(C)]
    struct Trade {
        price: D64<8>,
        size: D64<8>,
        time: u64,
    }

    #[test]
    fn a_decimal_is_written_as_its_steps_bytes() {
        let price: D64<8> = dec!(60000.5);
        assert_eq!(price.as_bytes(), 6_000_050_000_000_i64.to_ne_bytes(), "its steps' bytes");
        let wide = UD128::<18>::from_steps(u128::MAX, Fixed);
        assert_eq!(wide.as_bytes(), u128::MAX.to_ne_bytes(), "and at sixteen bytes");
    }

    #[test]
    fn a_record_of_decimals_is_written_and_read_back_without_a_copy() {
        let trade = Trade { price: dec!(60000.5), size: dec!(0.25), time: 1_700_000_000 };
        let bytes = trade.as_bytes();
        assert_eq!(bytes.len(), 24, "three eight-byte fields");
        assert_eq!(Trade::ref_from_bytes(bytes), Ok(&trade), "read back in place");
    }
}
