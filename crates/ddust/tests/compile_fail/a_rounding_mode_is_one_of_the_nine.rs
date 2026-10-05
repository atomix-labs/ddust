//! `RoundingMode` is sealed: a mode outside the nine would bypass their tables.

use ddust::round::RoundingMode;

#[derive(Clone, Copy)]
struct Nearest;

impl RoundingMode for Nearest {
    fn table(self) -> u16 {
        0
    }
}

fn main() {}
