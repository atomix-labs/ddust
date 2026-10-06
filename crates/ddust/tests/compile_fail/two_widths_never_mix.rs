//! Two integers are two types: values of two widths never add until one is converted.

use ddust::{D32, D64, dec};

fn main() {
    let (wide, narrow): (D64<2>, D32<2>) = (dec!(1.5), dec!(1.25));
    let _sum = wide + narrow;
}
