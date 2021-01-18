// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_utility.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_utility::WeightInfo for WeightInfo<T> {
    fn batch(c: u32, ) -> Weight {
        (19_612_000 as Weight)
            // Standard Error: 0
            .saturating_add((1_988_000 as Weight).saturating_mul(c as Weight))
    }
    fn as_derivative() -> Weight {
        (5_849_000 as Weight)
    }
    fn batch_all(c: u32, ) -> Weight {
        (21_934_000 as Weight)
            // Standard Error: 0
            .saturating_add((1_503_000 as Weight).saturating_mul(c as Weight))
    }
}
