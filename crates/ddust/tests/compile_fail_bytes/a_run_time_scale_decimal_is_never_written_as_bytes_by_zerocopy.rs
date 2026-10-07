//! A run-time scale's decimal has uninitialized padding after its scale's byte, so the decimal is
//! never written as bytes.

use ddust::{Decimal, Dynamic};
use zerocopy::IntoBytes;

fn main() {
    let price = Decimal::<i64, Dynamic>::default();
    let _bytes = price.as_bytes();
}
