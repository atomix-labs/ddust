//! An unsigned decimal has no negation.

use ddust::{UD64, dec};

fn main() {
    let size: UD64<2> = dec!(1.5);
    let _negated = -size;
}
