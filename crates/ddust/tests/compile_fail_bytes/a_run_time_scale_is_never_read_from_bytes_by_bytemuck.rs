//! A run-time scale's byte may be past 38 decimals, so neither it nor a decimal
//! of it reads from bytes.

use bytemuck::pod_read_unaligned;
use ddust::{Decimal, Dynamic};

fn main() {
    let _scale: Dynamic = pod_read_unaligned(&[4]);
    let _price: Decimal<i64, Dynamic> = pod_read_unaligned(&[0; 16]);
}
