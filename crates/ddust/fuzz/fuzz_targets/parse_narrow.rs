//! The parser at 8, 32 and 64 bits over any bytes, which it refuses or reads and never panics on:
//! a read in place and at the front read alike, a value's spellings read back as it, and a
//! rounding read agrees with the exact one, or brackets it within a step.

#![no_main]

use ddust::{D8, D32, D64, UD64};
use libfuzzer_sys::fuzz_target;

#[path = "parse.rs"]
mod parse;

fuzz_target!(|data: &[u8]| {
    check!(data, D8<1>);
    check!(data, D32<4>);
    check!(data, D64<0>);
    check!(data, D64<7>);
    check!(data, D64<18>);
    check!(data, UD64<11>);
});
