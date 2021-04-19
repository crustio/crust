// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use frame_support::traits::{OnUnbalanced, Currency, Imbalance, WithdrawReasons, Get, ExistenceRequirement};
use frame_support::unsigned::TransactionValidityError;
use frame_support::weights::{WeightToFeePolynomial, WeightToFeeCoefficients, WeightToFeeCoefficient};
use frame_support::dispatch::{GetCallMetadata, CallMetadata};
use crate::{Balances, Authorship, NegativeImbalance};
use sp_arithmetic::{Perbill, traits::{BaseArithmetic, Unsigned}};
use smallvec::smallvec;
use sp_std::marker::PhantomData;
use sp_runtime::{
    traits::{DispatchInfoOf, PostDispatchInfoOf, Zero, Saturating, Convert, SaturatedConversion},
    transaction_validity::InvalidTransaction,
};
use pallet_transaction_payment::{OnChargeTransaction};
use primitives::traits::BenefitInterface;

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
pub struct OneTenthFee<T>(sp_std::marker::PhantomData<T>);

impl<T> WeightToFeePolynomial for OneTenthFee<T> where
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
type NegativeImbalanceOf<C, T> = <C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

/// Implements the transaction payment for a module implementing the `Currency`
/// trait (eg. the pallet_balances) using an unbalance handler (implementing
/// `OnUnbalanced`).
pub struct CurrencyAdapter<C, R, OU>(PhantomData<(C, R, OU)>);

/// Default implementation for a Currency and an OnUnbalanced handler.
impl<T, C, R, OU> OnChargeTransaction<T> for CurrencyAdapter<C, R, OU>
    where
        T: pallet_transaction_payment::Config,
        T::TransactionByteFee: Get<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance>,
        C: Currency<<T as frame_system::Config>::AccountId>,
        R: BenefitInterface<<T as frame_system::Config>::AccountId, <C as Currency<<T as frame_system::Config>::AccountId>>::Balance, NegativeImbalanceOf<C, T>>,
        C::PositiveImbalance:
        Imbalance<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance, Opposite = C::NegativeImbalance>,
        C::NegativeImbalance:
        Imbalance<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance, Opposite = C::PositiveImbalance>,
        OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
        T::Call: GetCallMetadata,
{
    type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
    type Balance = <C as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Withdraw the predicted fee from the transaction origin.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn withdraw_fee(
        who: &T::AccountId,
        call: &T::Call,
        _info: &DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }

        let withdraw_reason = if tip.is_zero() {
            WithdrawReasons::TRANSACTION_PAYMENT
        } else {
            WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
        };

        let special_call = CallMetadata { function_name: "calculate_reward".into(), pallet_name: "Market".into() };
        if special_call == call.get_call_metadata()  {
            match R::maybe_reduce_fee(who, fee, withdraw_reason) {
                Ok(imbalance) => Ok(Some(imbalance)),
                Err(_) => Err(InvalidTransaction::Payment.into()),
            }
        } else {
            match C::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
                Ok(imbalance) => Ok(Some(imbalance)),
                Err(_) => Err(InvalidTransaction::Payment.into()),
            }
        }
    }

    /// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
    /// Since the predicted fee might have been too high, parts of the fee may
    /// be refunded.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn correct_and_deposit_fee(
        who: &T::AccountId,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        _post_info: &PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some(paid) = already_withdrawn {
            // Calculate how much refund we should return
            let refund_amount = paid.peek().saturating_sub(corrected_fee);
            // refund to the the account that paid the fees. If this fails, the
            // account might have dropped below the existential balance. In
            // that case we don't refund anything.
            let refund_imbalance =
                C::deposit_into_existing(&who, refund_amount).unwrap_or_else(|_| C::PositiveImbalance::zero());
            // merge the imbalance caused by paying the fees and refunding parts of it again.
            let adjusted_paid = paid
                .offset(refund_imbalance)
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
            // Call someone else to handle the imbalance (fee and tip separately)
            let imbalances = adjusted_paid.split(tip);
            OU::on_unbalanceds(Some(imbalances.0).into_iter().chain(Some(imbalances.1)));
        }
        Ok(())
    }
}