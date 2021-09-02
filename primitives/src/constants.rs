// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

/// Money matters.
pub mod currency {
    use crate::Balance;

    pub const CRUS: Balance = 1_000_000_000_000;
    pub const DOLLARS: Balance = CRUS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    // GPoS rewards in the first year
    pub const FIRST_YEAR_REWARDS: Balance = 5_000_000 * CRUS;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 1_000 * CENTS + (bytes as Balance) * 100 * MILLICENTS
	}
}

/// Time and blocks.
pub mod time {
    use crate::{BlockNumber, Moment};

    // Alpha & mainnet
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    // Testnet
    //	pub const MILLISECS_PER_BLOCK: Moment = 1000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Use different settings in the test
    #[cfg(feature = "test")]
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    #[cfg(not(feature = "test"))]
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 1 * HOURS;

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

pub mod staking {
    use crate::Balance;
    // The reward decrease ratio per year
    pub const REWARD_DECREASE_RATIO: (Balance, Balance) = (88, 100);
    // The minimal reward ratio
    pub const MIN_REWARD_RATIO: (Balance, Balance) = (28, 1000);
    // The start year for extra reward
    pub const EXTRA_REWARD_START_YEAR: u64 = 4;
}

pub mod swork {
    use super::time::*;

    // Use different settings in the test
    #[cfg(feature = "test")]
    pub const REPORT_SLOT: u64 = EPOCH_DURATION_IN_BLOCKS as u64 * 3;
    #[cfg(not(feature = "test"))]
    pub const REPORT_SLOT: u64 = EPOCH_DURATION_IN_BLOCKS as u64;

    pub const UPDATE_OFFSET: u32 = (REPORT_SLOT / 3) as u32;
    pub const END_OFFSET: u32 = 1;
}

pub mod market {
    pub const BASE_FEE_UPDATE_SLOT: u32 = 600;
    pub const BASE_FEE_UPDATE_OFFSET: u32 = 22;

    pub const PRICE_UPDATE_SLOT: u32 = 10;
    pub const PRICE_UPDATE_OFFSET: u32 = 3;
    pub const FILES_COUNT_REFERENCE: u32 = 20_000_000; // 20_000_000 / 50_000_000 = 40%

    pub const SPOWER_UPDATE_SLOT: u32 = 100;
    pub const SPOWER_UPDATE_OFFSET: u32 = 7;
    pub const MAX_PENDING_FILES: usize = 20;


    // Use different settings in the test
    #[cfg(feature = "test")]
    pub const COLLATERAL_RATIO: u32 = 10;
    #[cfg(not(feature = "test"))]
    pub const COLLATERAL_RATIO: u32 = 1;
}
