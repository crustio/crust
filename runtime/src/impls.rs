use sp_runtime::traits::{Convert, SaturatedConversion};
/*use frame_support::traits::{OnUnbalanced, Imbalance, Currency};
use crate::NegativeImbalance;*/

/// Logic for the author to get a portion of fees.
/// TODO: enable it when enable `treasury`
/*pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);

impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
    where
        R: balances::Trait + authorship::Trait,
        <R as system::Trait>::AccountId: From<primitives::AccountId>,
        <R as system::Trait>::AccountId: Into<primitives::AccountId>,
        <R as system::Trait>::Event: From<balances::RawEvent<
            <R as system::Trait>::AccountId,
            <R as balances::Trait>::Balance,
            balances::DefaultInstance>
        >,
{
    fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
        let numeric_amount = amount.peek();
        let author = <authorship::Module<R>>::author();
        <balances::Module<R>>::resolve_creating(&<authorship::Module<R>>::author(), amount);
        <system::Module<R>>::deposit_event(balances::RawEvent::Deposit(author, numeric_amount));
    }
}*/

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