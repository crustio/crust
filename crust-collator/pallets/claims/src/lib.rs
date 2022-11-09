// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to process claims from Ethereum addresses.
#![cfg_attr(not(feature = "std"), no_std)]
use sp_std::prelude::*;
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize, Serializer, Deserializer};

use sp_runtime::{RuntimeDebug};
use frame_support::ensure;
use sp_io::{hashing::keccak_256, crypto::secp256k1_ecdsa_recover};
pub use pallet::*;
use frame_support::traits::ExistenceRequirement::AllowDeath;
use sp_runtime::traits::Get;
use frame_support::traits::Currency;
use sp_runtime::DispatchResult;
use sp_runtime::traits::AccountIdConversion;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// An Ethereum address (i.e. 20 bytes, used to represent an Ethereum account).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug, scale_info::TypeInfo)]
pub struct EthereumAddress([u8; 20]);

#[cfg(feature = "std")]
impl Serialize for EthereumAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for EthereumAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let base_string = String::deserialize(deserializer)?;
        let offset = if base_string.starts_with("0x") { 2 } else { 0 };
        let s = &base_string[offset..];
        if s.len() != 40 {
            Err(serde::de::Error::custom("Bad length of Ethereum address (should be 42 including '0x')"))?;
        }
        let raw: Vec<u8> = rustc_hex::FromHex::from_hex(s)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        let mut r = Self::default();
        r.0.copy_from_slice(&raw);
        Ok(r)
    }
}

/// An Ethereum signature
#[derive(Encode, Decode, Clone, scale_info::TypeInfo)]
pub struct EcdsaSignature(pub [u8; 65]);

impl PartialEq for EcdsaSignature {
    fn eq(&self, other: &Self) -> bool {
        &self.0[..] == &other.0[..]
    }
}

impl sp_std::fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "EcdsaSignature({:?})", &self.0[..])
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug, scale_info::TypeInfo)]
pub struct EthereumTxHash([u8; 32]);

#[cfg(feature = "std")]
impl Serialize for EthereumTxHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for EthereumTxHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let base_string = String::deserialize(deserializer)?;
        let offset = if base_string.starts_with("0x") { 2 } else { 0 };
        let s = &base_string[offset..];
        if s.len() != 64 {
            Err(serde::de::Error::custom(
                "Bad length of Ethereum tx hash (should be 66 including '0x')",
            ))?;
        }
        let raw: Vec<u8> = rustc_hex::FromHex::from_hex(s)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        let mut r = Self::default();
        r.0.copy_from_slice(&raw);
        Ok(r)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use crate::{
        EthereumTxHash,
        EcdsaSignature, to_ascii_hex,
        EthereumAddress,
        ValidityError};
	use sp_std::prelude::*;
    use sp_std::convert::TryInto;
	use frame_support::{pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
    use sp_runtime::{
        transaction_validity::{
            TransactionLongevity, TransactionValidity, ValidTransaction, InvalidTransaction, TransactionSource,
        },
        traits::{
            StaticLookup, Saturating
        },
    };
    use frame_support::{
        traits::{Currency, Get}
    };

    /// The balance type of this module.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

    /// Configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
        /// The claim's module id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The payment balance.
        type Currency: Currency<Self::AccountId>;

        /// The constant used for ethereum signature.
        #[pallet::constant]
        type Prefix: Get<&'static [u8]>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// Init pot success
        InitPot(T::AccountId, BalanceOf<T>),
        /// Someone be the new superior
        SuperiorChanged(T::AccountId),
        /// Someone be the new miner
        MinerChanged(T::AccountId),
        /// Set limit successfully
        SetLimitSuccess(BalanceOf<T>),
        /// Mint claims successfully
        MintSuccess(EthereumTxHash, EthereumAddress, BalanceOf<T>),
        /// Someone claimed some CRUs. [who, ethereum_address, amount]
        Claimed(T::AccountId, EthereumAddress, BalanceOf<T>),
        /// Ethereum address was bonded to account. [who, ethereum_address]
        BondEthSuccess(T::AccountId, EthereumAddress),
    }

    #[pallet::error]
	pub enum Error<T> {
        /// Superior not exist, should set it first
        IllegalSuperior,
        /// Miner is not exist, should set it first
        MinerNotExist,
        /// Miner should be the registered
        IllegalMiner,
        /// Ethereum tx already be mint
        AlreadyBeMint,
        /// Ethereum tx already be claimed
        AlreadyBeClaimed,
        /// Invalid Ethereum signature.
        InvalidEthereumSignature,
        /// Ethereum address has no claims.
        SignerHasNoClaim,
        /// Sign not match
        SignatureNotMatch,
        /// Exceed claim limitation
        ExceedClaimLimit,
	}

    #[pallet::storage]
	#[pallet::getter(fn claim_limit)]
	pub(super) type ClaimLimit<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn claims)]
	pub(super) type Claims<T: Config> = StorageMap<_, Identity, EthereumTxHash, (EthereumAddress, BalanceOf<T>), OptionQuery>;

    #[pallet::storage]
	#[pallet::getter(fn claimed)]
	pub(super) type Claimed<T: Config> = StorageMap<_, Identity, EthereumTxHash, bool, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn superior)]
	pub(super) type Superior<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
	#[pallet::getter(fn miner)]
	pub(super) type Miner<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;


    #[pallet::call]
	impl<T: Config> Pallet<T> {
        /// Change superior
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// Parameter:
        /// - `new_superior`: The new superior's address
        #[pallet::weight(1000)]
        pub fn change_superior(origin: OriginFor<T>, new_superior: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
            ensure_root(origin)?;

            let new_superior = T::Lookup::lookup(new_superior)?;

            Superior::<T>::put(new_superior.clone());

            Self::deposit_event(Event::SuperiorChanged(new_superior));

            Ok(())
        }

        /// Change miner
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// Parameters:
        /// - `new_miner`: The new miner's address
        #[pallet::weight(1000)]
        pub fn change_miner(origin: OriginFor<T>, new_miner: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
            ensure_root(origin)?;

            let new_miner = T::Lookup::lookup(new_miner)?;

            Miner::<T>::put(new_miner.clone());

            Self::deposit_event(Event::MinerChanged(new_miner));
            Ok(())
        }

        /// Set claim limit
        ///
        /// The dispatch origin for this call must be _Superior_.
        ///
        /// Parameters:
        /// - `limit`: The claim CRUs limit
        #[pallet::weight(1000)]
        pub fn set_claim_limit(origin: OriginFor<T>, limit: BalanceOf<T>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let maybe_superior = Self::superior();

            // 1. Check if superior exist
            ensure!(maybe_superior.is_some(), Error::<T>::IllegalSuperior);

            // 2. Check if signer is superior
            ensure!(Some(&signer) == maybe_superior.as_ref(), Error::<T>::IllegalSuperior);

            // 3. Set claim limit
            ClaimLimit::<T>::put(limit);

            Self::deposit_event(Event::SetLimitSuccess(limit));
            Ok(())
        }

        /// Mint the claim
        ///
        /// This dispatch origin for this call must be _Miner_.
        ///
        /// Parameters:
        /// - `tx`: The claim ethereum tx hash
        /// - `who`: The claimer ethereum address
        /// - `value`: The amount of this tx, should be less than claim_limit
        #[pallet::weight(1000)]
        pub fn mint_claim(origin: OriginFor<T>, tx: EthereumTxHash, who: EthereumAddress, value: BalanceOf<T>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let maybe_miner = Self::miner();

            // 1. Check if miner exist
            ensure!(maybe_miner.is_some(), Error::<T>::MinerNotExist);

            // 2. Check if this tx already be mint
            ensure!(!Claims::<T>::contains_key(&tx), Error::<T>::AlreadyBeMint);

            // 3. Check if signer is miner
            ensure!(Some(&signer) == maybe_miner.as_ref(), Error::<T>::IllegalMiner);

            // 4. Check claim limit
            ensure!(Self::claim_limit() >= value, Error::<T>::ExceedClaimLimit);

            // 5. Save into claims
            Claims::<T>::insert(tx.clone(), (who.clone(), value.clone()));
            Claimed::<T>::insert(tx, false);

            // 6. Reduce claim limit
            ClaimLimit::<T>::mutate(|l| *l = l.saturating_sub(value));

            Self::deposit_event(Event::MintSuccess(tx, who, value));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn claim(origin: OriginFor<T>, dest: T::AccountId, tx: EthereumTxHash, sig: EcdsaSignature) -> DispatchResult {
            let _ = ensure_none(origin)?;

            // 1. Check the tx already be mint and not be claimed
            ensure!(Claims::<T>::contains_key(&tx), Error::<T>::SignerHasNoClaim);
            ensure!(!Self::claimed(&tx), Error::<T>::AlreadyBeClaimed);

            // 2. Sign data
            let data = dest.using_encoded(to_ascii_hex);
            let tx_data = tx.using_encoded(to_ascii_hex);
            let signer = Self::eth_recover(&sig, &data, &tx_data).ok_or(Error::<T>::InvalidEthereumSignature)?;

            // 3. Make sure signer is match with claimer
            Self::process_claim(tx, signer, dest)
        }
    }

    #[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            const PRIORITY: u64 = 100;
    
            let (maybe_signer, tx) = match call {
                Call::claim {dest, tx, sig} => {
                    let data = dest.using_encoded(to_ascii_hex);
                    let tx_data = tx.using_encoded(to_ascii_hex);
                    (Self::eth_recover(&sig, &data, &tx_data), tx)
                }
                _ => return Err(InvalidTransaction::Call.into()),
            };
    
            let signer = maybe_signer
                .ok_or(InvalidTransaction::Custom(ValidityError::InvalidEthereumSignature.into()))?;
    
            let e = InvalidTransaction::Custom(ValidityError::SignerHasNoClaim.into());
            ensure!(<Claims<T>>::contains_key(&tx), e);
    
            let e = InvalidTransaction::Custom(ValidityError::SignatureNotMatch.into());
            let (claimer, _) = Self::claims(&tx).unwrap();
            ensure!(claimer == signer, e);
    
            let e = InvalidTransaction::Custom(ValidityError::AlreadyBeClaimed.into());
            ensure!(!Self::claimed(&tx), e);
    
            Ok(ValidTransaction {
                priority: PRIORITY,
                requires: vec![],
                provides: vec![("claims", signer).encode()],
                longevity: TransactionLongevity::max_value(),
                propagate: true,
            })
        }

    }
}


impl<T: Config> Pallet<T> {
    // The claim pot account
    pub fn claim_pot() -> T::AccountId {
        // "modl" ++ "crclaims" ++ "clai" is 16 bytes
        T::PalletId::get().into_account_truncating()
    }

    // Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
    fn ethereum_signable_message(what: &[u8], extra: &[u8]) -> Vec<u8> {
        let prefix = T::Prefix::get();
        let mut l = prefix.len() + what.len() + extra.len();
        let mut rev = Vec::new();
        while l > 0 {
            rev.push(b'0' + (l % 10) as u8);
            l /= 10;
        }
        let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
        v.extend(rev.into_iter().rev());
        v.extend_from_slice(&prefix[..]);
        v.extend_from_slice(what);
        v.extend_from_slice(extra);
        v
    }

    // Attempts to recover the Ethereum address from a message signature signed by using
    // the Ethereum RPC's `personal_sign` and `eth_sign`.
    fn eth_recover(s: &EcdsaSignature, what: &[u8], extra: &[u8]) -> Option<EthereumAddress> {
        let msg = keccak_256(&Self::ethereum_signable_message(what, extra));
        let mut res = EthereumAddress::default();
        res.0.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
        Some(res)
    }

    fn process_claim(tx: EthereumTxHash, signer: EthereumAddress, dest: T::AccountId) -> DispatchResult {
        if let Some((claimer, amount)) = Self::claims(&tx) {
            // 1. Ensure signer matches claimer
            ensure!(claimer == signer, Error::<T>::SignatureNotMatch);

            // 2. Give money to signer
            T::Currency::transfer(&Self::claim_pot(), &dest, amount, AllowDeath)?;

            // 3. Mark it be claimed
            Claimed::<T>::insert(tx, true);

            // Let's deposit an event to let the outside world know who claimed money
            Self::deposit_event(Event::Claimed(dest, signer, amount));

            Ok(())
        } else {
            Err(Error::<T>::SignerHasNoClaim)?
        }
    }
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice the length.
fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
    let mut r = Vec::with_capacity(data.len() * 2);
    let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
    for &b in data.iter() {
        push_nibble(b / 16);
        push_nibble(b % 16);
    }
    r
}

/// Custom validity errors used in Polkadot while validating transactions.
#[repr(u8)]
pub enum ValidityError {
    /// The Ethereum signature is invalid.
    InvalidEthereumSignature = 0,
    /// The signer has no claim.
    SignerHasNoClaim = 1,
    /// No permission to execute the call.
    SignatureNotMatch = 2,
    /// This tx already be claimed.
    AlreadyBeClaimed = 3,
}

impl From<ValidityError> for u8 {
    fn from(err: ValidityError) -> Self {
        err as u8
    }
}