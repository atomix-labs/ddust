//! Text: a decimal read from what Rust's own numbers are written as, and written as its shortest
//! exact decimal.

mod format;
mod parse;

pub use self::format::MAX_ASCII_LEN;
