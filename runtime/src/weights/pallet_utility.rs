#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl utility::WeightInfo for WeightInfo {
    fn batch(c: u32, ) -> Weight {
        (16461000 as Weight)
            .saturating_add((1982000 as Weight).saturating_mul(c as Weight))
    }
    // WARNING! Some components were not used: ["u"]
    fn as_derivative() -> Weight {
        (4086000 as Weight)
    }
}
