// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weights for pallet_tips using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_tips::WeightInfo for WeightInfo<T> {
    fn report_awesome(r: u32, ) -> Weight {
        (70_338_000 as Weight)
            .saturating_add((2_000 as Weight).saturating_mul(r as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn retract_tip() -> Weight {
        (59_051_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn tip_new(r: u32, t: u32, ) -> Weight {
        (41_984_000 as Weight)
            .saturating_add((2_000 as Weight).saturating_mul(r as Weight))
            .saturating_add((180_000 as Weight).saturating_mul(t as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn tip(t: u32, ) -> Weight {
        (33_313_000 as Weight)
            .saturating_add((700_000 as Weight).saturating_mul(t as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn close_tip(t: u32, ) -> Weight {
        (110_781_000 as Weight)
            .saturating_add((364_000 as Weight).saturating_mul(t as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn slash_tip(t: u32, ) -> Weight {
        (37_184_000 as Weight)
            // Standard Error: 0
            .saturating_add((11_000 as Weight).saturating_mul(t as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
}
