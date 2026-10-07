//! A run-time scale's decimal has uninitialized padding after its scale's byte, so the decimal is
//! never written as bytes.

use bytemuck::bytes_of;
use ddust::{Decimal, Dynamic};

fn main() {
    let price = Decimal::<i64, Dynamic>::default();
    let _bytes = bytes_of(&price);
}
