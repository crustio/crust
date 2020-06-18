use frame_support::{
    traits::{
        LockableCurrency
    }
};

/// A currency whose accounts can have liquidity restrictions.
pub trait TransferrableCurrency<AccountId>: LockableCurrency<AccountId> {
	fn transferrable_balance(who: &AccountId) -> Self::Balance;
}