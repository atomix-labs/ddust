//! The listings of the chapter Testing: decimals drawn for property tests, fuzzing and random runs.

#[cfg(feature = "proptest")]
// ANCHOR: proptest
mod properties {
    use ddust::D64;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn a_sum_is_the_same_either_way(a: D64<2>, b: D64<2>) {
            prop_assert_eq!(a.checked_add(b), b.checked_add(a));
        }
    }
}
// ANCHOR_END: proptest

#[cfg(feature = "arbitrary")]
#[test]
// ANCHOR: arbitrary
fn a_fuzzer_draws_any_decimal_from_its_bytes() {
    use arbitrary::Unstructured;
    use ddust::{D64, Decimal, Dynamic};

    let mut input = Unstructured::new(b"the bytes a fuzzer gives, any of them");
    let (a, b): (D64<2>, D64<2>) = input.arbitrary().expect("sixteen bytes");
    assert_eq!(a.checked_add(b), b.checked_add(a));
    let carried: Decimal<i64, Dynamic> = input.arbitrary().expect("steps, and a scale");
    assert!(carried.decimals() <= 38);
}
// ANCHOR_END: arbitrary

#[cfg(feature = "rand-09")]
#[test]
// ANCHOR: rand
fn a_seeded_draw_is_the_same_on_every_run() {
    use ddust::{D64, dec};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let draw = |seed| {
        let mut rng = StdRng::seed_from_u64(seed);
        let price: D64<2> = rng.random_range(dec!(99.00)..=dec!(101.00));
        price
    };
    assert_eq!(draw(7), draw(7), "one seed, one price");
    assert!((dec!(99)..=dec!(101)).contains(&draw(7)));
}
// ANCHOR_END: rand
