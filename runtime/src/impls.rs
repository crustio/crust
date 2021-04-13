// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use sp_runtime::traits::{Convert, SaturatedConversion};
use frame_support::traits::{OnUnbalanced, Currency};
use frame_support::weights::{WeightToFeePolynomial, WeightToFeeCoefficients, WeightToFeeCoefficient};
use crate::{Balances, Authorship, NegativeImbalance};
use sp_arithmetic::{Perbill, traits::{BaseArithmetic, Unsigned}};
use smallvec::smallvec;

/// Logic for the author to get a portion of fees.
pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
    fn on_nonzero_unbalanced(amount: NegativeImbalance) {
        Balances::resolve_creating(&Authorship::author(), amount);
    }
}

/// Simple structure that exposes how u64 currency can be represented as... u64.
pub struct CurrencyToVoteHandler;

impl Convert<u64, u64> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u64 {
        x
    }
}
impl Convert<u128, u128> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u128 {
        x
    }
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u64 {
        x.saturated_into()
    }
}

impl Convert<u64, u128> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u128 {
        x as u128
    }
}

/// Implementor of `WeightToFeePolynomial` that maps one unit of weight to one unit of fee.
pub struct QuarterFee<T>(sp_std::marker::PhantomData<T>);

impl<T> WeightToFeePolynomial for QuarterFee<T> where
    T: BaseArithmetic + From<u32> + Copy + Unsigned
{
    type Balance = T;

    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        smallvec!(WeightToFeeCoefficient {
			coeff_integer: 0u32.into(),
			coeff_frac: Perbill::from_percent(10),
			negative: false,
			degree: 1,
		})
    }
}