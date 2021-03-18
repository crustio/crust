// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_im_online.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_im_online::WeightInfo for WeightInfo<T> {
    fn validate_unsigned_and_then_heartbeat(k: u32, e: u32, ) -> Weight {
        (112_814_000 as Weight)
            // Standard Error: 0
            .saturating_add((215_000 as Weight).saturating_mul(k as Weight))
            // Standard Error: 2_000
            .saturating_add((491_000 as Weight).saturating_mul(e as Weight))
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}
