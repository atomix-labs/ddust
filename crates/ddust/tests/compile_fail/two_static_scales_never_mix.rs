//! Two static scales are two types: their values never add.

use ddust::{D64, dec};

fn main() {
    let (tenths, hundredths): (D64<1>, D64<2>) = (dec!(1.5), dec!(1.25));
    let _sum = tenths + hundredths;
}
