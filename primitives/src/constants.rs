/// Money matters.
pub mod currency {
    use crate::Balance;

    pub const CRUS: Balance = 1_000_000_000_000;
    pub const DOLLARS: Balance = CRUS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const INITIAL_ISSUANCE: Balance = 15_000_000 * CRUS;
    pub const FIRST_YEAR_REWARDS: Balance = 5_000_000 * CRUS;
    pub const BLOCK_AUTHORING_REWARDS: Balance = 100_000 * CRUS / 365 / 24 / 2;
}

/// Time and blocks.
pub mod time {
    use crate::{BlockNumber, Moment};

    // Alpha & mainnet
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    // Testnet
    //	pub const MILLISECS_PER_BLOCK: Moment = 1000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
    // Alpha
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
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

pub mod tee {
    use super::time::*;

    pub const REPORT_SLOT: u64 = EPOCH_DURATION_IN_BLOCKS as u64 * 3;
}
