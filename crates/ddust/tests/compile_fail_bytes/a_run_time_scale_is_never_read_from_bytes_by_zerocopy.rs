//! A run-time scale's byte may be past 38 decimals, so neither it nor a decimal of it reads from
//! bytes.

use ddust::{Decimal, Dynamic};
use zerocopy::FromBytes;

fn main() {
    let _scale = Dynamic::read_from_bytes(&[4]);
    let _price = Decimal::<i64, Dynamic>::read_from_bytes(&[0; 16]);
}
