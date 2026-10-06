//! A decimal counts its steps in an integer, never a float.

use ddust::{Decimal, Fixed};

fn main() {
    let _price = Decimal::<f64, Fixed<2>>::from_steps(1.5, Fixed).decimals();
}
