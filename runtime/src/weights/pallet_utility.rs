#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl pallet_utility::WeightInfo for WeightInfo {
    fn batch_all(c: u32, ) -> Weight {
        (19_735_000 as Weight)
            .saturating_add((1_990_000 as Weight).saturating_mul(c as Weight))
    }
    fn batch(c: u32, ) -> Weight {
        (16461000 as Weight)
            .saturating_add((1982000 as Weight).saturating_mul(c as Weight))
    }
    // WARNING! Some components were not used: ["u"]
    fn as_derivative() -> Weight {
        (4086000 as Weight)
    }
}
