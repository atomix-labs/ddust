//! Other crates' traits, each crate behind its own feature.

#[cfg(feature = "arbitrary")]
mod arbitrary;
#[cfg(feature = "defmt")]
mod defmt;
#[cfg(feature = "num-traits-02")]
mod num_traits;
#[cfg(feature = "proptest")]
mod proptest;

#[cfg(test)]
mod tests {
    #[cfg(feature = "zerocopy-08")]
    mod with_zerocopy {
        use zerocopy::{FromBytes as _, IntoBytes as _};

        use crate::{D64, Dynamic, dec};

        #[test]
        fn a_decimal_reads_from_its_steps_bytes() {
            let price = D64::<2>::read_from_bytes(&1_234_i64.to_ne_bytes()).expect("eight bytes");
            assert_eq!(price, dec!(12.34), "1,234 hundredths");
            let precision = Dynamic::new(4).expect("at most 38");
            assert_eq!(precision.as_bytes(), [4], "its decimals, one byte");
        }
    }

    #[cfg(feature = "bytemuck")]
    mod with_bytemuck {
        use bytemuck::{bytes_of, pod_read_unaligned};

        use crate::{D64, Dynamic, dec};

        #[test]
        fn a_decimal_reads_from_its_steps_bytes() {
            let price: D64<2> = pod_read_unaligned(&1_234_i64.to_ne_bytes());
            assert_eq!(price, dec!(12.34), "1,234 hundredths");
            assert_eq!(
                bytes_of(&Dynamic::new(4).expect("at most 38")),
                [4],
                "its decimals, one byte"
            );
        }
    }

    #[cfg(feature = "arbitrary")]
    mod with_arbitrary {
        use arbitrary::{Arbitrary as _, Unstructured};

        use crate::round::Rounding;
        use crate::{Dynamic, Scale as _};

        #[test]
        fn every_scale_is_drawn_and_no_other() {
            let mut seen = [false; 39];
            for byte in 0..=u8::MAX {
                let scale = Dynamic::arbitrary(&mut Unstructured::new(&[byte])).expect("one byte");
                *seen.get_mut(usize::from(scale.decimals())).expect("at most 38 decimals") = true;
            }
            assert!(seen.iter().all(|&drawn| drawn), "each of 0 to 38 decimals");
            assert!(Rounding::arbitrary(&mut Unstructured::new(&[8])).is_ok(), "a mode");
        }
    }

    #[cfg(feature = "proptest")]
    mod with_proptest {
        use alloc::format;

        use proptest::prelude::*;

        use crate::round::Rounding;
        use crate::{Decimal, Dynamic};

        proptest! {
            #[test]
            fn a_drawn_value_reads_back_from_its_text(value: Decimal<i64, Dynamic>, _mode: Rounding) {
                let read: Decimal<i64, Dynamic> = format!("{value:#}").parse().expect("its own text");
                prop_assert_eq!((read.steps(), read.decimals()), (value.steps(), value.decimals()));
            }
        }
    }
}
