use sp_runtime::{ MultiSignature, traits::{ Verify, IdentifyAccount}, generic };

// TODO: abstract all this custom type into `primitives` folder
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

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Money matters.
pub mod currency {
    use super::*;

    pub const CRUS: Balance = 1_000_000_000_000;
    pub const DOLLARS: Balance = CRUS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;
}

/// Time and blocks.
pub mod time {
    use super::*;

    // Alpha & mainnet
    pub const MILLISECS_PER_BLOCK: u64 = 6000;
    // Testnet
//	pub const MILLISECS_PER_BLOCK: Moment = 1000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
    // Alpha
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 2 * MINUTES;
    // Mainnet
//	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;
    // Testnet
//	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    // 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Fee-related.
pub mod fee {
    pub use sp_runtime::Perbill;

    /// The block saturation level. Fees will be updates based on this value.
    pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);
}
