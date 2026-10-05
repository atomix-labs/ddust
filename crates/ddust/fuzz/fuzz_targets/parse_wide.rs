//! The parser at 128 bits over any bytes, as `parse_narrow` checks the narrower widths.

#![no_main]

use ddust::{D128, UD128};
use libfuzzer_sys::fuzz_target;

#[path = "parse.rs"]
mod parse;

fuzz_target!(|data: &[u8]| {
    check!(data, D128<0>);
    check!(data, D128<18>);
    check!(data, D128<38>);
    check!(data, UD128<18>);
});
