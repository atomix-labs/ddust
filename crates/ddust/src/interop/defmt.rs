//! defmt's `Format`, for logging from a device: a decimal as its text, which the device writes
//! itself, eight bytes at a time, with no `core::fmt`.

use defmt::{Format, Formatter, write};

use crate::{Decimal, Fixed, Int, MAX_ASCII_LEN, Scale};

/// The shortest exact decimal, as `Display` writes it.
impl<I: Int, S: Scale> Format for Decimal<I, S> {
    fn format(&self, f: Formatter<'_>) {
        let mut out = [0; MAX_ASCII_LEN];
        let len = self.write_ascii(&mut out).unwrap_or_default();
        let text = out.get(..len).and_then(|text| str::from_utf8(text).ok()).unwrap_or_default();
        write!(f, "{=str}", text);
    }
}

/// `Fixed<D>`, with its decimals, as its `Debug` writes it.
impl<const D: u8> Format for Fixed<D> {
    fn format(&self, f: Formatter<'_>) {
        write!(f, "Fixed<{=u8}>", D);
    }
}
