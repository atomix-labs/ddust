//! The listings of the chapter Interop.

#[cfg(feature = "num-traits-02")]
#[test]
// ANCHOR: num_traits
fn generic_numeric_code_takes_a_decimal() {
    use ddust::{D64, dec};
    use num_traits::{CheckedAdd, FromPrimitive, ToPrimitive, Zero};

    fn total<T: Zero + CheckedAdd>(values: &[T]) -> Option<T> {
        values.iter().try_fold(T::zero(), |sum, value| sum.checked_add(value))
    }

    let fills: [D64<2>; 3] = [dec!(1.25), dec!(2.50), dec!(0.75)];
    assert_eq!(total(&fills), Some(dec!(4.5)));
    assert_eq!(total(&[D64::<2>::MAX, dec!(0.01)]), None);
    let rate: Option<D64<2>> = FromPrimitive::from_f64(0.125);
    assert_eq!(rate, Some(dec!(0.12)), "the nearest step, a tie to even");
    assert_eq!(dec!(-2.75: D64<2>).to_i64(), Some(-2), "the whole part, as a float's");
}
// ANCHOR_END: num_traits

#[cfg(feature = "bytemuck")]
#[test]
// ANCHOR: bytemuck
fn a_static_scale_decimal_is_its_steps_bytes() {
    use bytemuck::{cast, cast_slice};
    use ddust::{D64, dec};

    let prices: [D64<2>; 2] = [dec!(1.25), dec!(-0.01)];
    let bytes: &[u8] = cast_slice(&prices);
    assert_eq!(bytes.len(), 16, "eight bytes a price, and nothing else");
    assert_eq!(cast_slice::<u8, D64<2>>(bytes), prices);
    assert_eq!(cast::<D64<2>, i64>(prices[0]), 125, "the steps");
}
// ANCHOR_END: bytemuck

#[cfg(feature = "zerocopy-08")]
#[test]
// ANCHOR: zerocopy
fn a_static_scale_decimal_is_read_from_bytes_in_place() {
    use ddust::{D64, dec};
    use zerocopy::{FromBytes, IntoBytes};

    let price: D64<2> = dec!(64250.75);
    let bytes = price.as_bytes();
    assert_eq!(bytes, 6_425_075_i64.as_bytes());
    assert_eq!(D64::<2>::read_from_bytes(bytes), Ok(price));
}
// ANCHOR_END: zerocopy
