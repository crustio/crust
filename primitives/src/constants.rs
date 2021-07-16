// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

/// Money matters.
pub mod currency {
    use crate::Balance;

    pub const CRUS: Balance = 1_000_000_000_000;
    pub const DOLLARS: Balance = CRUS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const INITIAL_ISSUANCE: Balance = 15_000_000 * CRUS;
    // Staking rewards in the first year
    pub const FIRST_YEAR_REWARDS: Balance = 5_000_000 * CRUS;
    // Block authoring rewards per year
    pub const BLOCK_AUTHORING_REWARDS: Balance = 100_000 * CRUS;
    // GPoS rewards in the first quarter
    pub const FIRST_QUARTER_TOTAL_REWARDS: Balance = 270_000 * CRUS;
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
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 5 * MINUTES;
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

pub mod swork {
    use super::time::*;

    pub const REPORT_SLOT: u64 = EPOCH_DURATION_IN_BLOCKS as u64 * 3;
}

pub mod market {
    pub const BASE_FEE_UPDATE_SLOT: u32 = 600;
    pub const BASE_FEE_UPDATE_OFFSET: u32 = 22;

    pub const PRICE_UPDATE_SLOT: u32 = 10;
    pub const PRICE_UPDATE_OFFSET: u32 = 3;
    pub const FILES_COUNT_REFERENCE: u32 = 20_000_000; // 20_000_000 / 50_000_000 = 40%

    pub const USED_UPDATE_SLOT: u32 = 100;
    pub const USED_UPDATE_OFFSET: u32 = 7;
    pub const MAX_PENDING_FILES: usize = 20;
}
