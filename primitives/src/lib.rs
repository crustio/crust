#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};
use sp_std::vec::Vec;

pub mod constants;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// An index to a block.
pub type BlockNumber = u32;

/// An instant or duration in time.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Tee public key, little-endian-format, 64 bytes vec
pub type PubKey = Vec<u8>;

/// Tee signature, little-endian-format, 64 bytes vec
pub type TeeSignature = Vec<u8>;

/// Work report empty workload/storage merkle root
pub type MerkleRoot = Vec<u8>;

/// Report index, always be a multiple of era number
pub type ReportSlot = u64;