//! Rounding modes: each a zero-sized type the compiler resolves, and [`Rounding`], one chosen at
//! run time. Every operation that can lose digits takes either, as its last argument.

/// Seals [`RoundingMode`]: the nine modes and [`Rounding`] are all there are.
mod sealed {
    /// Implemented by the nine modes and by [`Rounding`](super::Rounding).
    pub trait Sealed {}
}

/// How an operation settles the digits that do not fit: a mode type such as [`Floor`], compiled
/// in, or a [`Rounding`] chosen at run time.
///
/// Each mode is a table of sixteen bits, one for each way a division can end: the sign of the
/// result, whether its quotient is odd, and whether the remainder is zero, below half the
/// divisor, at half or above it. Rounding reads the one bit for how the division ended, so a mode
/// is data, whether it is a type, a constant or a value read at run time.
///
/// # Examples
/// ```
/// use ddust::round::{Floor, Rounding};
///
/// let modes: [Rounding; 2] = [Floor.into(), Rounding::HalfEven];
/// assert_eq!(modes[0], Rounding::Floor, "a type, and the same mode as a value");
/// ```
pub const trait RoundingMode: sealed::Sealed + Copy {
    /// The mode's table: bit `negative << 3 | odd << 2 | class` says whether a quotient moves one
    /// step away from zero, `class` being 0 for no remainder, 1 for one below half the divisor, 2
    /// for exactly half and 3 for more.
    #[doc(hidden)]
    fn table(self) -> u16;
}

/// Declares the modes, each a zero-sized type, and [`Rounding`], whose discriminants are their
/// tables.
macro_rules! modes {
    ($($(#[$doc:meta])* $name:ident = $table:literal;)*) => {
        $(
            $(#[$doc])*
            #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name;

            impl sealed::Sealed for $name {}

            const impl RoundingMode for $name {
                #[inline]
                fn table(self) -> u16 {
                    $table
                }
            }

            /// The mode as a value.
            impl From<$name> for Rounding {
                #[inline]
                fn from(_mode: $name) -> Self {
                    Self::$name
                }
            }
        )*

        /// A rounding mode chosen at run time, from a configuration or a protocol's rules: one of the
        /// nine mode types as a value.
        ///
        /// Its discriminant is the mode's table, so an operation reads it as it reads a type's,
        /// with no `match`.
        ///
        /// # Examples
        /// ```
        /// use ddust::round::{HalfEven, Rounding};
        ///
        /// let mode = Rounding::from(HalfEven);
        /// assert_eq!(mode, Rounding::HalfEven, "a type, as a value");
        /// assert_ne!(mode, Rounding::HalfExpand, "ties to even, not away from zero");
        /// ```
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum Rounding {
            $($(#[$doc])* $name = $table,)*
        }

        impl sealed::Sealed for Rounding {}

        const impl RoundingMode for Rounding {
            #[inline]
            #[expect(clippy::as_conversions, reason = "a field-less enum's discriminant, its table")]
            fn table(self) -> u16 {
                self as u16
            }
        }
    };
}

modes! {
    /// Toward negative infinity, as `f64::floor` does.
    Floor = 0xEE00;
    /// Toward positive infinity, as `f64::ceil` does: a charge up to the smallest coin.
    Ceil = 0x00EE;
    /// Toward zero, as the integers' `/` does.
    Trunc = 0x0000;
    /// Away from zero.
    Expand = 0xEEEE;
    /// To the nearest, ties toward negative infinity.
    HalfFloor = 0xCC88;
    /// To the nearest, ties toward positive infinity.
    HalfCeil = 0x88CC;
    /// To the nearest, ties toward zero.
    HalfTrunc = 0x8888;
    /// To the nearest, ties away from zero: commercial rounding, as `f64::round` does.
    HalfExpand = 0xCCCC;
    /// To the nearest, ties to the even neighbour: banker's rounding, as `f64::round_ties_even`
    /// does.
    HalfEven = 0xC8C8;
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{Rounding, RoundingMode};

    /// Whether `mode` moves a quotient away from zero, by its definition rather than its table.
    fn moves_away(mode: Rounding, negative: bool, odd: bool, class: u32) -> bool {
        let (exact, half) = (class == 0, class.cmp(&2));
        match mode {
            Rounding::Floor => negative && !exact,
            Rounding::Ceil => !negative && !exact,
            Rounding::Trunc => false,
            Rounding::Expand => !exact,
            Rounding::HalfFloor => !exact && (half.is_gt() || (half.is_eq() && negative)),
            Rounding::HalfCeil => !exact && (half.is_gt() || (half.is_eq() && !negative)),
            Rounding::HalfTrunc => !exact && half.is_gt(),
            Rounding::HalfExpand => !exact && half.is_ge(),
            Rounding::HalfEven => !exact && (half.is_gt() || (half.is_eq() && odd)),
        }
    }

    #[rstest]
    fn every_table_is_its_definition(
        #[values(
            Rounding::Floor,
            Rounding::Ceil,
            Rounding::Trunc,
            Rounding::Expand,
            Rounding::HalfFloor,
            Rounding::HalfCeil,
            Rounding::HalfTrunc,
            Rounding::HalfExpand,
            Rounding::HalfEven
        )]
        mode: Rounding,
    ) {
        for index in 0..16_u32 {
            let (negative, odd, class) = (index & 8 != 0, index & 4 != 0, index & 3);
            let bit = (mode.table() >> index) & 1 == 1;
            assert_eq!(bit, moves_away(mode, negative, odd, class), "{mode:?}, case {index}");
        }
    }
}
